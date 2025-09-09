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
                    "variable" => variable_transform(exprs),
                    "integer" => integer_transform(exprs),
                    "boolean" => boolean_transform(exprs),
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
                        for (i, elem) in elements.iter().rev().enumerate() {
                            match elem {
                                Expression::Word(name) => {
                                    if i == elements.len() - 1 {
                                        if name != "." {
                                            new_bindings.push(Expression::Apply(vec![
                                                Expression::Word("let".to_string()),
                                                Expression::Word(name.clone()),
                                                Expression::Word("_args".to_string()),
                                            ]))
                                        }
                                    } else {
                                        if name != "." {
                                            new_bindings.push(Expression::Apply(vec![
                                                Expression::Word("let".to_string()),
                                                Expression::Word(name.clone()),
                                                Expression::Apply(vec![
                                                    Expression::Word("pop-and-get!".to_string()),
                                                    Expression::Word("_args".to_string()),
                                                ]),
                                            ]))
                                        } else {
                                            new_bindings.push(Expression::Apply(vec![
                                                Expression::Word("pop-and-get!".to_string()),
                                                Expression::Word("_args".to_string()),
                                            ]))
                                        }
                                    }
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
fn variable_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    Expression::Apply(vec![
        Expression::Word("let".to_string()),
        exprs[0].clone(),
        Expression::Apply(vec![Expression::Word("box".to_string()), exprs[1].clone()]),
    ])
}
fn integer_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    Expression::Apply(vec![
        Expression::Word("let".to_string()),
        exprs[0].clone(),
        Expression::Apply(vec![Expression::Word("int".to_string()), exprs[1].clone()]),
    ])
}
fn boolean_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    match &exprs[1] {
        Expression::Word(x) => {
            if x != "true" && x != "false" {
                panic!(
                    "Booleans variables only be assigned to true or false but got: {}",
                    x
                )
            }
        }
        Expression::Apply(x) => match &x[0] {
            Expression::Word(y) => {
                if y != "="
                    && y != ">"
                    && y != "<"
                    && y != "<="
                    && y != ">="
                    && y != "not"
                    && y != "or"
                    && y != "and"
                {
                    panic!(
                        "Booleans variables only be assigned to results of boolean expressions but got: {}",
                        y
                    )
                }
            }
            _ => panic!(
                "Booleans variables only be assigned to true or false but got: {:?}",
                x[0]
            ),
        },
        x => panic!(
            "Booleans variables only be assigned to true or false but got : {:?}",
            x
        ),
    }
    Expression::Apply(vec![
        Expression::Word("let".to_string()),
        exprs[0].clone(),
        Expression::Apply(vec![
            Expression::Word("array".to_string()),
            exprs[1].clone(),
        ]),
    ])
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
