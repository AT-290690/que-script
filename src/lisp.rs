use core::panic;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

fn collect_idents(expr: &Expression, acc: &mut HashSet<String>) {
    match expr {
        Expression::Word(w) => {
            acc.insert(w.clone());
        }
        Expression::Apply(exprs) => {
            for e in exprs {
                collect_idents(e, acc);
            }
        }
        _ => {}
    }
}

fn tree_shake(std_defs: Vec<Expression>, used: &HashSet<String>) -> Vec<Expression> {
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

    let mut kept = Vec::new();
    let mut visited = HashSet::new();

    fn visit(
        name: &str,
        index: &HashMap<String, Expression>,
        kept: &mut Vec<Expression>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(name) {
            return;
        }
        if let Some(def) = index.get(name) {
            if let Expression::Apply(list) = def {
                if list.len() >= 3 {
                    let mut deps = HashSet::new();
                    collect_idents(&list[2], &mut deps);
                    for dep in deps {
                        visit(&dep, index, kept, visited);
                    }
                }
            }
            kept.push(def.clone());
        }
        visited.insert(name.to_string());
    }

    for name in used {
        visit(name, &index, &mut kept, &mut visited);
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
                    "get" => accessor_transform(exprs),
                    "lambda" => lambda_destructure_transform(exprs),
                    _ => Expression::Apply(exprs),
                }
            } else {
                Expression::Apply(exprs)
            }
        }
        other => other,
    }
}
fn lambda_destructure_transform(mut exprs: Vec<Expression>) -> Expression {
    // separate args and body
    if exprs.len() < 2 {
        panic!("lambda expects at least a body");
    }
    let args = &exprs[1..exprs.len() - 1];
    let body = exprs.last().unwrap().clone();

    // look for array args
    let mut new_bindings = vec![];
    let new_args: Vec<Expression> = args
        .iter()
        .map(|arg| {
            if let Expression::Apply(array_exprs) = arg {
                if let [Expression::Word(ref array_kw), ref elements @ ..] = &array_exprs[..] {
                    if array_kw == "array" {
                        // replace this arg with _args
                        for (i, elem) in elements.iter().enumerate() {
                            match elem {
                                Expression::Word(name) if name != "." => {
                                    new_bindings.push(Expression::Apply(vec![
                                        Expression::Word("let".to_string()),
                                        Expression::Word(name.clone()),
                                        Expression::Apply(vec![
                                            Expression::Word("get".to_string()),
                                            Expression::Word("_args".to_string()),
                                            Expression::Atom(i as i32),
                                        ]),
                                    ]));
                                }
                                Expression::Word(_) => { /* skip element */ }
                                _ => panic!("lambda array element must be a word or '.'"),
                            }
                        }
                        return Expression::Word("_args".to_string());
                    }
                }
            }
            arg.clone()
        })
        .collect();

    // wrap body with new bindings
    let new_body = if !new_bindings.is_empty() {
        let mut do_exprs = new_bindings;
        do_exprs.push(body);
        Expression::Apply(
            std::iter::once(Expression::Word("do".to_string()))
                .chain(do_exprs.into_iter())
                .collect(),
        )
    } else {
        body
    };

    // rebuild lambda with transformed args and body
    let mut lambda_exprs = vec![Expression::Word("lambda".to_string())];
    lambda_exprs.extend(new_args);
    lambda_exprs.push(new_body);
    Expression::Apply(lambda_exprs)
}
fn accessor_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    let len = exprs.len();
    let mut iter = exprs.into_iter();
    let first = iter.next().unwrap();

    if len == 1 {
        return Expression::Apply(vec![
            Expression::Word("get".to_string()),
            first,
            Expression::Atom(0),
        ]);
    }

    let mut acc = first;

    for e in iter {
        acc = Expression::Apply(vec![Expression::Word("get".to_string()), acc, e]);
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
            panic!("Undefined variable: {}", name)
        }
        Expression::Apply(exprs) => {
            if let Expression::Word(name) = &exprs[0] {
                let env_ref = env.borrow();
                if let Some(var) = env_ref.get(name) {
                    match var {
                        Evaluated::Function(func) => {
                            return func(exprs[1..].to_vec(), Rc::clone(&env), Rc::clone(&defs))
                        }
                        _ => panic!("Cannot apply a non-lambda value"),
                    }
                }
                let defs_ref = defs.borrow();
                if let Some(var) = defs_ref.get(name) {
                    match var {
                        Evaluated::Function(func) => {
                            return func(exprs[1..].to_vec(), Rc::clone(&env), Rc::clone(&defs))
                        }
                        _ => panic!("Cannot apply a non-lambda value"),
                    }
                }
                panic!("Function not found: {}", name)
            }
            panic!("Invalid lambda application")
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
                    if (args.len() == 2) {
                        while let Evaluated::Number(value) =
                            evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs))
                        {
                            if value == 1 {
                                evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                            } else {
                                break;
                            }
                        }
                    } else {
                        let start_val = evaluate(&args[0], Rc::clone(&env), Rc::clone(&defs));
                        let end_val = evaluate(&args[1], Rc::clone(&env), Rc::clone(&defs));
                        let func_val = evaluate(&args[2], Rc::clone(&env), Rc::clone(&defs));

                        let start = match start_val {
                            Evaluated::Number(n) => n,
                            _ => panic!("loop: start must be a number"),
                        };
                        let end = match end_val {
                            Evaluated::Number(n) => n,
                            _ => panic!("loop: end must be a number"),
                        };
                        let func = match func_val {
                            Evaluated::Function(f) => {
                                for i in start..end {
                                    f(vec![Expression::Atom(i)], Rc::clone(&env), Rc::clone(&defs));
                                }
                            }
                            _ => panic!("loop: third argument must be a lambda"),
                        };
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
                        panic!("First argument to 'let' must be a variable name")
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
                        panic!("First argument to 'apply' must be a word")
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

// COMPILATION TO BYTECODE

#[derive(Clone)]
pub struct CompilationEnv {
    vars: HashMap<String, Compiled>,
    parent: Option<Rc<RefCell<CompilationEnv>>>,
}

impl CompilationEnv {
    pub fn new() -> Self {
        CompilationEnv {
            vars: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Rc<RefCell<CompilationEnv>>) -> Self {
        CompilationEnv {
            vars: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<Compiled> {
        if let Some(var) = self.vars.get(name) {
            return Some(var.clone());
        }
        if let Some(ref parent) = self.parent {
            return parent.borrow().get(name);
        }
        None
    }

    pub fn set(&mut self, name: String, value: Compiled) {
        self.vars.insert(name, value);
    }
}

#[derive(Clone)]
enum Opcode {
    Let(Vec<(String, Vec<Opcode>)>, Vec<Opcode>), // bindings + body
    Do(Vec<Opcode>),
    Add,
    PushConst(Compiled), // push a literal Compiled onto the stack
    LoadVar(String),     // push value of variable
    StoreVar(String),    // pop and store in variable
    Call(usize),         // call function with N args (popped from stack)
    MakeClosure(Vec<Opcode>, Vec<String>), // body + parameter names
    Loop {
        start: usize,
        end: usize,
        body: Vec<Opcode>,
    }, // for-loop
    While {
        cond: Vec<Opcode>,
        body: Vec<Opcode>,
    }, // while-loop
    Return,
    // add more as needed: arithmetic, get/set, push!, etc.
}

#[derive(Clone)]
pub enum Compiled {
    Function(Rc<dyn Fn(Vec<Compiled>, Rc<RefCell<CompilationEnv>>) -> Compiled + 'static>),
    Number(i32),
    Vector(Rc<RefCell<Vec<Compiled>>>),
}

impl fmt::Debug for Compiled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Compiled::Number(value) => write!(f, "{}", value),
            Compiled::Function(_) => write!(f, "Function"),
            Compiled::Vector(arr) => {
                let arr_ref = arr.borrow();
                let elements: Vec<String> = arr_ref.iter().map(|x| format!("{:?}", x)).collect();
                write!(f, "[{}]", elements.join(" "))
            }
        }
    }
}
fn compile_expr_to_opcodes(expr: &Expression) -> Vec<Opcode> {
    match expr {
        Expression::Apply(items) if !items.is_empty() => {
            if let Expression::Word(ref name) = items[0] {
                match name.as_str() {
                    "do" => {
                        let mut code = vec![];
                        for expr in &items[1..] {
                            code.extend(compile_expr_to_opcodes(expr));
                        }
                        return code; // <- just inline all the statements
                    }
                    "let" => {
                        // Expect exactly 3 items: "let", variable name, value expression
                        if items.len() != 3 {
                            panic!("let must have exactly a variable name and a value");
                        }

                        let var_name = if let Expression::Word(n) = &items[1] {
                            n.clone()
                        } else {
                            panic!("let binding must be a variable name");
                        };

                        // Compile the value expression
                        let value_code = compile_expr_to_opcodes(&items[2]);

                        // Push value, then store it
                        let mut code = value_code;
                        code.push(Opcode::StoreVar(var_name.clone()));
                        code.push(Opcode::LoadVar(var_name)); // <- push it so Call can use it
                        return code;
                    }
                    "lambda" => {
                        let params_exprs = &items[1..items.len() - 1];
                        let body_expr = &items[items.len() - 1];
                        let params: Vec<String> = params_exprs
                            .iter()
                            .filter_map(|param| {
                                if let Expression::Word(name) = param {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        let body_code = compile_expr_to_opcodes(body_expr);
                        return vec![Opcode::MakeClosure(body_code, params)];
                    }
                    "+" => {
                        let mut code = vec![];
                        for arg in &items[1..] {
                            code.extend(compile_expr_to_opcodes(arg));
                        }
                        code.push(Opcode::Add);
                        return code;
                    }
                    _ => {}
                }
            }
            // General function call
            // Push arguments first, then the function itself
            let args = &items[1..];
            let mut code = vec![];
            for arg in args {
                code.extend(compile_expr_to_opcodes(arg));
            }
            // Then push the function
            code.extend(compile_expr_to_opcodes(&items[0]));
            // Call with number of arguments
            code.push(Opcode::Call(args.len()));
            code
        }
        Expression::Atom(n) => vec![Opcode::PushConst(Compiled::Number(*n))],
        Expression::Word(name) => vec![Opcode::LoadVar(name.clone())],
        _ => vec![],
    }
}

struct VM {
    stack: Vec<Compiled>,
    env: Rc<RefCell<CompilationEnv>>,
}
impl VM {
    pub fn run(&mut self, code: &[Opcode]) -> Compiled {
        let mut pc = 0;

        while pc < code.len() {
            match &code[pc] {
                Opcode::Do(stmts) => {
                    let mut last = Compiled::Number(0);
                    for stmt in stmts.iter().cloned() {
                        // clone each opcode
                        last = self.run(&[stmt]);
                    }
                    self.stack.push(last);
                }
                Opcode::PushConst(val) => self.stack.push(val.clone()),
                Opcode::Add => {
                    let b = self.stack.pop().expect("Stack underflow on Add");
                    let a = self.stack.pop().expect("Stack underflow on Add");
                    if let (Compiled::Number(x), Compiled::Number(y)) = (a, b) {
                        self.stack.push(Compiled::Number(x + y));
                    } else {
                        panic!("Add expects two numbers");
                    }
                }
                Opcode::StoreVar(name) => {
                    let val = self.stack.pop().expect("Stack underflow on StoreVar");
                    self.env.borrow_mut().set(name.clone(), val);
                }

                Opcode::LoadVar(name) => {
                    let val = self
                        .env
                        .borrow()
                        .get(name)
                        .unwrap_or_else(|| panic!("Variable {} not found", name));
                    self.stack.push(val);
                }

                Opcode::Let(bindings, body) => {
                    let local_env = Rc::new(RefCell::new(CompilationEnv::with_parent(Rc::clone(
                        &self.env,
                    ))));

                    for (name, expr_code) in bindings {
                        let val = VM {
                            stack: vec![],
                            env: Rc::clone(&local_env),
                        }
                        .run(expr_code);
                        local_env.borrow_mut().set(name.clone(), val);
                    }

                    let val = VM {
                        stack: vec![],
                        env: Rc::clone(&local_env),
                    }
                    .run(body);
                    self.stack.push(val);
                }
                Opcode::Call(n) => {
                    // Pop the last n values as arguments
                    let mut args = Vec::with_capacity(*n);
                    for _ in 0..*n {
                        args.push(self.stack.pop().expect("Stack underflow: missing argument"));
                    }
                    args.reverse(); // reverse because the first argument was pushed first

                    // Pop the function to call
                    let mut args = Vec::with_capacity(*n);
                    for _ in 0..*n {
                        args.push(self.stack.pop().expect("Stack underflow"));
                    }
                    args.reverse(); // because stack pops last argument first
                    let func = self
                        .stack
                        .pop()
                        .expect("Stack underflow: no function to call");

                    match func {
                        Compiled::Function(f) => {
                            let result = f(args, Rc::clone(&self.env));
                            self.stack.push(result);
                        }
                        _ => panic!("Call expects a function, got {:?}", func),
                    }
                }

                Opcode::MakeClosure(body, params) => {
                    let closure = Compiled::Function(Rc::new({
                        let body = body.clone();
                        let params = params.clone();

                        let env = Rc::clone(&self.env);

                        move |args: Vec<Compiled>, _env: Rc<RefCell<CompilationEnv>>| {
                            if args.len() != params.len() {
                                panic!("Expected {} arguments, got {}", params.len(), args.len());
                            }

                            // Bind parameters to compiled arguments
                            let local_env =
                                Rc::new(RefCell::new(CompilationEnv::with_parent(Rc::clone(&env))));
                            for (p, a) in params.iter().zip(args.iter()) {
                                local_env.borrow_mut().set(p.clone(), a.clone());
                            }

                            // Run the closure body
                            VM {
                                stack: vec![],
                                env: local_env,
                            }
                            .run(&body)
                        }
                    }));
                    self.stack.push(closure);
                }
                Opcode::Return => return self.stack.pop().expect("Return with empty stack"),
                _ => unimplemented!(),
            }

            pc += 1;
        }

        self.stack.pop().expect("Stack empty at end of run")
    }
}

pub fn virtual_machine(ast: &Expression) -> Compiled {
    let code = compile_expr_to_opcodes(ast);
    let mut vm = VM {
        stack: vec![],
        env: Rc::new(RefCell::new(CompilationEnv::new())),
    };
    vm.run(&code)
}
