use core::panic;
use std::cell::RefCell;
use std::collections::{HashSet, HashMap};
use std::fmt;
use std::rc::Rc;


fn collect_idents(expr: &Expression, acc: &mut HashSet<String>) {
    match expr {
        Expression::Word(w) => { acc.insert(w.clone()); }
        Expression::Apply(exprs) => {
            for e in exprs {
                collect_idents(e, acc);
            }
        }
        _ => {}
    }
}

fn tree_shake(std_defs: Vec<Expression>, used: &HashSet<String>) -> Vec<Expression> {
    let mut kept = Vec::new();
    let mut to_process = used.clone();
    let mut processed = HashSet::new();

    let mut index = HashMap::new();
    for expr in &std_defs {
        if let Expression::Apply(list) = expr {
            if let [Expression::Word(kw), Expression::Word(name), _rest @ ..] = &list[..] {
                if kw == "let" {
                    index.insert(name.clone(), expr.clone());
                }
            }
        }
    }

    while let Some(name) = to_process.iter().next().cloned() {
        to_process.remove(&name);
        if processed.contains(&name) {
            continue;
        }
        if let Some(def) = index.get(&name) {
            kept.push(def.clone());

            if let Expression::Apply(list) = def {
                if list.len() >= 3 {
                    let body = &list[2];
                    let mut acc = HashSet::new();
                    collect_idents(body, &mut acc);
                    for dep in acc {
                        if !processed.contains(&dep) {
                            to_process.insert(dep);
                        }
                    }
                }
            }
        }
        processed.insert(name);
    }

    kept
}



fn flush(buf: &mut String, out: &mut Vec<String>) {
    if !buf.is_empty() {
        out.push(std::mem::take(buf));
    }
}

fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ';' => {
                while let Some(nc) = chars.peek() {
                    if *nc == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            '(' | ')' => {
                flush(&mut buf, &mut out);
                out.push(ch.to_string());
            }
            c if c.is_whitespace() => {
                flush(&mut buf, &mut out);
            }
            _ => {
                buf.push(ch);
            }
        }
    }
    flush(&mut buf, &mut out);
    out
}

fn parse_expr(tokens: &[String], i: &mut usize) -> Result<Expression, String> {
    if *i >= tokens.len() {
        return Err("Unexpected end of input".into());
    }
    let tok = &tokens[*i];

    if tok == "(" {
        *i += 1;
        let mut exprs = Vec::new();
        while *i < tokens.len() && tokens[*i] != ")" {
            exprs.push(parse_expr(tokens, i)?);
        }
        if *i >= tokens.len() {
            return Err("Unclosed '('".into());
        }
        *i += 1;
        Ok(Expression::Apply(exprs))
    } else if tok == ")" {
        Err("Unexpected ')'".into())
    } else {
        *i += 1;
        if is_number(tok) {
            let n: i32 = tok
                .parse()
                .map_err(|e| format!("Bad number '{}': {}", tok, e))?;
            Ok(Expression::Atom(n))
        } else {
            Ok(Expression::Word(tok.clone()))
        }
    }
}
pub fn parse(src: &str) -> Result<Vec<Expression>, String> {
    let tokens = tokenize(src);
    let mut i = 0;
    let mut exprs = Vec::new();
    while i < tokens.len() {
        exprs.push(parse_expr(&tokens, &mut i)?);
    }
    Ok(exprs)
}
fn preprocess(source: &str) -> String {
    let mut out = String::new();
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '[' => {
                out.push('(');
                out.push_str("array ");
            }
            ']' => out.push(')'),
            '"' => {
                let mut s = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '"' {
                        break;
                    } else {
                        s.push(next);
                    }
                }
                out.push_str("(array ");
                for (i, c) in s.chars().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    out.push_str(&(c as u32).to_string());
                    out.push_str("");
                }
                out.push(')');
            }
            _ => out.push(ch),
        }
    }

    out
}
fn desugar(expr: Expression) -> Expression {
    match expr {
        Expression::Apply(exprs) if !exprs.is_empty() => {
            let exprs: Vec<Expression> = exprs.into_iter().map(desugar).collect::<Vec<_>>();

            if let Expression::Word(ref name) = exprs[0] {
                match name.as_str() {
                    "|>" => pipe_transform(exprs),
                    "cond" => cond_transform(exprs),
                    "if" => if_transform(exprs),
                    "unless" => unless_transform(exprs),
                    "-" => minus_transform(exprs),
                    "+" => plus_transform(exprs),
                    "*" => mult_transform(exprs),
                    "/" => div_transform(exprs),
                    "!=" => not_equal_transform(exprs),
                    "<>" => not_equal_transform(exprs),
                    "." => accessor_transform(exprs),
                    _ => Expression::Apply(exprs),
                }
            } else {
                Expression::Apply(exprs)
            }
        }
        other => other,
    }
}
fn accessor_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0); 
    let mut iter = exprs.into_iter();

    let first = iter.next().unwrap(); 
    let mut acc = first;

    for e in iter {
        acc = Expression::Apply(vec![
            Expression::Word("get".to_string()),
            acc,
            e,
        ]);
    }

    acc
}
fn not_equal_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    Expression::Apply(vec![
        Expression::Word("not".to_string()),
        Expression::Apply(vec![
            Expression::Word("=".to_string()),
            exprs[0].clone(),
            exprs[1].clone(),
        ]),
    ])
}
fn minus_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Atom(0),
        1 => Expression::Apply(vec![
            Expression::Word("*".to_string()),
            exprs.remove(0),
            Expression::Atom(-1),
        ]),
        _ => {
            let first = exprs.remove(0);
            exprs.into_iter().fold(first, |acc, next| {
                Expression::Apply(vec![Expression::Word("-".to_string()), acc, next])
            })
        }
    }
}
fn plus_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Atom(1),
        _ => {
            let first = exprs.remove(0);
            exprs.into_iter().fold(first, |acc, next| {
                Expression::Apply(vec![Expression::Word("+".to_string()), acc, next])
            })
        }
    }
}
fn mult_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Atom(1),
        _ => {
            let first = exprs.remove(0);
            exprs.into_iter().fold(first, |acc, next| {
                Expression::Apply(vec![Expression::Word("*".to_string()), acc, next])
            })
        }
    }
}
fn div_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Atom(1),
        _ => {
            let first = exprs.remove(0);
            exprs.into_iter().fold(first, |acc, next| {
                Expression::Apply(vec![Expression::Word("/".to_string()), acc, next])
            })
        }
    }
}
fn cond_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    if exprs.is_empty() {
        return Expression::Atom(0);
    }

    let mut pairs = Vec::new();
    let mut default = Expression::Atom(0);

    if exprs.len() % 2 == 1 {
        default = exprs.pop().unwrap();
    }

    for chunk in exprs.chunks(2) {
        let test = chunk[0].clone();
        let branch = chunk[1].clone();
        pairs.push((test, branch));
    }

    let mut result = default;
    for (test, branch) in pairs.into_iter().rev() {
        result = Expression::Apply(vec![
            Expression::Word("if".to_string()),
            test,
            branch,
            result,
        ]);
    }

    result
}
fn if_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    return Expression::Apply(vec![
        Expression::Word("if".to_string()),
        exprs[0].clone(),
        exprs[1].clone(),
        if exprs.len() == 2 {
            Expression::Atom(0)
        } else {
            exprs[2].clone()
        },
    ]);
}
fn unless_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    return Expression::Apply(vec![
        Expression::Word("if".to_string()),
        exprs[0].clone(),
        if exprs.len() == 2 {
            Expression::Atom(0)
        } else {
            exprs[2].clone()
        },
        exprs[1].clone(),
    ]);
}
fn pipe_transform(mut exprs: Vec<Expression>) -> Expression {
    let mut inp = exprs.remove(1);

    for stage in exprs.into_iter().skip(1) {
        if let Expression::Apply(mut inner) = stage {
            if inner.is_empty() {
                continue;
            }
            let func = inner.remove(0);
            let mut new_stage = vec![func, inp];
            new_stage.extend(inner);
            inp = Expression::Apply(new_stage);
        } else {
            inp = Expression::Apply(vec![stage, inp]);
        }
    }

    inp
}
fn is_number(s: &str) -> bool {
    if s == "-" || s == "+ " || s == "+" {
        return false;
    }
    if !s.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    let trimmed = if let Some(stripped) = s.strip_prefix('-') {
        stripped
    } else {
        s
    };
    for c in trimmed.chars() {
         if !c.is_ascii_digit() {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone)]
pub enum Expression {
    Atom(i32),
    Word(String),
    Apply(Vec<Expression>),
}
#[derive(Clone)]
pub enum Evaluated {
    Function(Rc<dyn Fn(Vec<Expression>, Rc<RefCell<Env>>, Rc<RefCell<Env>>) -> Evaluated>),
    Number(i32),
    Vector(Rc<RefCell<Vec<Evaluated>>>),
}

impl Expression {
    pub fn to_rust(&self) -> String {
        match self {
            Expression::Atom(n) => format!("Expression::Atom({})", n),
            Expression::Word(w) => format!("Expression::Word({:?}.to_string())", w),
            Expression::Apply(exprs) => {
                let inner: Vec<String> = exprs.iter().map(|e| e.to_rust()).collect();
                format!("Expression::Apply(vec![{}])", inner.join(", "))
            }
        }
    }
}

impl fmt::Debug for Evaluated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Evaluated::Number(value) => write!(f, "{}", value),
            Evaluated::Function(_) => write!(f, "Function"),
            Evaluated::Vector(arr) => {
                let arr_ref = arr.borrow();
                let elements: Vec<String> = arr_ref.iter().map(|x| format!("{:?}", x)).collect();
                write!(f, "[{}]", elements.join(" "))
            }
        }
    }
}

#[derive(Clone)]
pub struct Env {
    vars: HashMap<String, Evaluated>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    fn new() -> Self {
        Env {
            vars: HashMap::new(),
            parent: None,
        }
    }

    fn with_parent(parent: Rc<RefCell<Env>>) -> Self {
        Env {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    fn get(&self, name: &str) -> Option<Evaluated> {
        if let Some(var) = self.vars.get(name) {
            return Some(var.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().get(name);
        }
        None
    }

    fn set(&mut self, name: String, value: Evaluated) {
        self.vars.insert(name, value);
    }
}

fn evaluate(exp: &Expression, env: Rc<RefCell<Env>>, defs: Rc<RefCell<Env>>) -> Evaluated {
    match exp {
        Expression::Atom(value) => Evaluated::Number(*value),
        Expression::Word(name) => {
            let env_ref = env.borrow();
            if let Some(var) = env_ref.get(name) {
                return var;
            }
            let defs_ref = defs.borrow();
            if let Some(var) = defs_ref.get(name) {
                return var;
            }
            panic!("Undefined variable: {}", name);
        }
        Expression::Apply(exprs) => {
            if let Expression::Word(name) = &exprs[0] {
                let env_ref = env.borrow();
                if let Some(var) = env_ref.get(name) {
                    match var {
                        Evaluated::Function(func) => {
                            return func(exprs[1..].to_vec(), Rc::clone(&env), Rc::clone(&defs));
                        }
                        _ => panic!("Cannot apply a non-lambda value"),
                    }
                }
                let defs_ref = defs.borrow();
                if let Some(var) = defs_ref.get(name) {
                    match var {
                        Evaluated::Function(func) => {
                            return func(exprs[1..].to_vec(), Rc::clone(&env), Rc::clone(&defs));
                        }
                        _ => panic!("Cannot apply a non-lambda value"),
                    }
                }
                panic!("Function not found: {}", name);
            }
            panic!("Invalid lambda application");
        }
    }
}

pub fn run(expr: &Expression) -> Evaluated {
    let env = Rc::new(RefCell::new(Env::new()));
    let defs = Rc::new(RefCell::new(Env::new()));
    {
        let mut env_ref = env.borrow_mut();
        env_ref.vars.insert(
            "loop".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    while let Evaluated::Number(value) =
                        evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs))
                    {
                        if value == 1 {
                            evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        } else {
                            break;
                        }
                    }
                    return Evaluated::Number(-1);
                },
            )),
        );
        env_ref.vars.insert(
            "array".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let evaluated_args = args
                        .into_iter()
                        .map(|arg| evaluate(&arg, Rc::clone(&env), Rc::clone(&defs)))
                        .collect();
                    Evaluated::Vector(Rc::new(RefCell::new(evaluated_args)))
                },
            )),
        );
        env_ref.vars.insert(
            "length".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                        Evaluated::Vector(arr) => Evaluated::Number(arr.borrow().len() as i32),
                        _ => panic!("First argument must be an array"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "get".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                        Evaluated::Vector(arr) => {
                            let index = evaluate(&args[1], env, defs);
                            match index {
                                Evaluated::Number(index) => {
                                    let len: usize = arr.borrow().len();
                                    if index >= 0 && index < (len as i32) {
                                        arr.borrow_mut().get(index as usize).unwrap().clone()
                                    } else {
                                        panic!("Index is outside ofthe array bounds")
                                    }
                                }
                                _ => panic!("Second argument of get must be a number"),
                            }
                        }
                        _ => panic!("First argument must be an array"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "set!".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                        Evaluated::Vector(arr) => {
                            let index = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                            match index {
                                Evaluated::Number(index) => {
                                    {
                                        let len: usize = arr.borrow().len();
                                        if index >= 0 && index < (len as i32) {
                                            arr.borrow_mut()[index as usize] = evaluate(
                                                &args[2],
                                                Rc::clone(&env),
                                                Rc::clone(&defs),
                                            );
                                        } else if index == (len as i32) {
                                            arr.borrow_mut().push(evaluate(
                                                &args[2],
                                                Rc::clone(&env),
                                                Rc::clone(&defs),
                                            ));
                                        } else {
                                            panic!("Index is outside ofthe array bounds");
                                        }
                                    }
                                    Evaluated::Vector(arr)
                                }
                                _ => panic!("Second argument of get must be a number"),
                            }
                        }
                        _ => panic!("First argument must be an array"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "pop!".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                        Evaluated::Vector(arr) => {
                            arr.borrow_mut().pop();
                            Evaluated::Vector(arr)
                        }
                        _ => panic!("First argument must be an array"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "+".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a + b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "-".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a - b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "*".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a * b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "/".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a / b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "mod".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a % b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "&".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a & b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "|".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a | b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "^".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a ^ b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "~".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    match a {
                        Evaluated::Number(a) => Evaluated::Number(!a),
                        _ => panic!("Argument must be be number"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            ">>".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a >> b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "<<".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => Evaluated::Number(a << b),
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "if".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let condition = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    match condition {
                        Evaluated::Number(condition) => {
                            if condition == 1 {
                                evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs))
                            } else {
                                evaluate(&args[2], Rc::clone(&env), Rc::clone(&defs))
                            }
                        }
                        _ => panic!("First argument must be a 1 or 0"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            ">".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => {
                            Evaluated::Number(if a > b { 1 } else { 0 })
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "<".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => {
                            Evaluated::Number(if a < b { 1 } else { 0 })
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            ">=".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => {
                            Evaluated::Number(if a >= b { 1 } else { 0 })
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "<=".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => {
                            Evaluated::Number(if a <= b { 1 } else { 0 })
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "=".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let a = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    let b = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                    match (a, b) {
                        (Evaluated::Number(a), Evaluated::Number(b)) => {
                            Evaluated::Number(if a == b { 1 } else { 0 })
                        }
                        _ => panic!("Both arguments must be numbers"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "not".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let condition: Evaluated =
                        evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    match condition {
                        Evaluated::Number(condition) => {
                            if condition != 0 {
                                Evaluated::Number(0)
                            } else {
                                Evaluated::Number(1)
                            }
                        }
                        _ => panic!("Argument must be a 1 or 0"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "and".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                        Evaluated::Number(a) => {
                            if a == 1 {
                                match evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs)) {
                                    Evaluated::Number(b) => {
                                        if b == 1 {
                                            Evaluated::Number(1)
                                        } else {
                                            Evaluated::Number(0)
                                        }
                                    }
                                    _ => panic!("First argument must be a 1 or 0"),
                                }
                            } else {
                                Evaluated::Number(0)
                            }
                        }
                        _ => panic!("First argument must be a 1 or 0"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "or".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                        Evaluated::Number(a) => {
                            if a == 0 {
                                match evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs)) {
                                    Evaluated::Number(b) => {
                                        if b == 1 {
                                            Evaluated::Number(1)
                                        } else {
                                            Evaluated::Number(0)
                                        }
                                    }
                                    _ => panic!("First argument must be a 1 or 0"),
                                }
                            } else {
                                Evaluated::Number(1)
                            }
                        }
                        _ => panic!("First argument must be a 1 or 0"),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "do".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    let mut last_result: Evaluated = Evaluated::Number(0);
                    for expr in args {
                        last_result = evaluate(&expr, Rc::clone(&env), Rc::clone(&defs));
                    }
                    last_result
                },
            )),
        );
        env_ref.vars.insert(
            "lambda".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 scope: Rc<RefCell<Env>>|
                 -> Evaluated {
                    if args.len() < 1 {
                        panic!("lambda expects at least one argument: the body");
                    }
                    let params: Vec<String> = args[0..args.len() - 1]
                        .iter()
                        .filter_map(|param| match param {
                            Expression::Word(name) => Some(name.clone()),
                            _ => None,
                        })
                        .collect();
                    let body: Expression = args[args.len() - 1].clone();
                    Evaluated::Function(Rc::new(
                        move |lambda_args: Vec<Expression>,
                              _env: Rc<RefCell<Env>>,
                              defs: Rc<RefCell<Env>>| {
                            if lambda_args.len() != params.len() {
                                panic!(
                                    "Expected {} arguments, but got {}",
                                    params.len(),
                                    lambda_args.len()
                                );
                            }
                            let local_defs =
                                Rc::new(RefCell::new(Env::with_parent(Rc::clone(&scope))));
                            {
                                let mut local_defs_ref = local_defs.borrow_mut();
                                for (param, arg) in params.iter().zip(lambda_args.iter()) {
                                    let value = evaluate(arg, Rc::clone(&env), Rc::clone(&defs));
                                    local_defs_ref.set(param.clone(), value);
                                }
                            }
                            evaluate(&body, Rc::clone(&env), Rc::clone(&local_defs))
                        },
                    ))
                },
            )),
        );
        env_ref.vars.insert(
            "let".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    if args.len() != 2 {
                        panic!("let expects exactly two arguments: a variable name and a value");
                    }
                    if let Expression::Word(var_name) = &args[0] {
                        let value = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        defs.borrow_mut().set(var_name.clone(), value);
                        return evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                    } else {
                        panic!("First argument to 'let' must be a variable name");
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "apply".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    if args.len() < 1 {
                        panic!("apply expects exactly at least one argument");
                    }
                    if let Expression::Word(_) = &args[&args.len() - 1] {
                        let func = evaluate(
                            &args[((args.len() as i32) - 1) as usize],
                            Rc::clone(&env),
                            Rc::clone(&defs),
                        );
                        if let Evaluated::Function(func) = func {
                            func(
                                args[0..args.len() - 1].to_vec(),
                                Rc::clone(&env),
                                Rc::clone(&defs),
                            )
                        } else {
                            panic!("Last argument to 'apply' must be a lambda")
                        }
                    } else if let Expression::Apply(_) = &args[&args.len() - 1] {
                        let func = evaluate(
                            &args[((args.len() as i32) - 1) as usize],
                            Rc::clone(&env),
                            Rc::clone(&defs),
                        );
                        if let Evaluated::Function(func) = func {
                            func(
                                args[0..args.len() - 1].to_vec(),
                                Rc::clone(&env),
                                Rc::clone(&defs),
                            )
                        } else {
                            panic!("Last argument to 'apply' must be a lambda")
                        }
                    } else {
                        panic!("First argument to 'apply' must be a word");
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "atom?".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    if args.len() < 1 {
                        panic!("atom? expects at least one argument");
                    }
                    match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                        Evaluated::Number(_) => Evaluated::Number(1),
                        _ => Evaluated::Number(0),
                    }
                },
            )),
        );
        env_ref.vars.insert(
            "lambda?".to_string(),
            Evaluated::Function(Rc::new(
                |args: Vec<Expression>,
                 env: Rc<RefCell<Env>>,
                 defs: Rc<RefCell<Env>>|
                 -> Evaluated {
                    if args.len() < 1 {
                        panic!("lambda? expects at least one argument");
                    }
                    match evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs)) {
                        Evaluated::Function(_) => Evaluated::Number(1),
                        _ => Evaluated::Number(0),
                    }
                },
            )),
        );
    }
    return evaluate(&expr, Rc::clone(&env), Rc::clone(&defs));
}

#[allow(dead_code)]
pub fn with_std(program: &str, std: &str) -> Expression {
    let preprocessed = preprocess(&program);

    let exprs = parse(&preprocessed).unwrap();
    let desugared: Vec<Expression> = exprs.into_iter().map(desugar).collect();

    let preprocessed_std = preprocess(&std);
    let exprs_std = parse(&preprocessed_std).unwrap();
    let desugared_std: Vec<Expression> = exprs_std.into_iter().map(desugar).collect();

    let mut used = HashSet::new();
    for e in &desugared {
        collect_idents(e, &mut used);
    }

    let shaken_std = tree_shake(desugared_std, &used);

    let wrapped = Expression::Apply(
        std::iter::once(Expression::Word("do".to_string()))
            .chain(shaken_std.into_iter())
            .chain(desugared.into_iter())
            .collect(),
    );
    wrapped
}
#[allow(dead_code)]
pub fn eval_with_std(program: &str, std: &str) {
    let preprocessed = preprocess(&program);
    let exprs = parse(&preprocessed).unwrap();
    let desugared: Vec<Expression> = exprs.into_iter().map(desugar).collect();

    let preprocessed_std = preprocess(&std);
    let exprs_std = parse(&preprocessed_std).unwrap();
    let desugared_std: Vec<Expression> = exprs_std.into_iter().map(desugar).collect();

    let mut used = HashSet::new();
    for e in &desugared {
        collect_idents(e, &mut used);
    }

    let shaken_std = tree_shake(desugared_std, &used);

    let wrapped = Expression::Apply(
        std::iter::once(Expression::Word("do".to_string()))
            .chain(shaken_std.into_iter())
            .chain(desugared.into_iter())
            .collect(),
    );
    let result = run(&wrapped);
    println!("Result: {:?}", result);
}
#[allow(dead_code)]
pub fn eval(program: &str) {
    let preprocessed = preprocess(&program);
    let exprs = parse(&preprocessed).unwrap();
    let desugared: Vec<Expression> = exprs.into_iter().map(desugar).collect();
    let wrapped = Expression::Apply(
        std::iter::once(Expression::Word("do".to_string()))
            .chain(desugared.into_iter())
            .collect(),
    );
    let result = run(&wrapped);
    println!("Result: {:?}", result);
}