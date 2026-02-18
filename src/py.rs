use crate::parser::Expression;

fn ident(name: &str, idx: usize) -> String {
    match name {
        "+" | "+#" | "+." => "(lambda a: (lambda b: a + b))".to_string(),
        "-" | "-#" | "-." => "(lambda a: (lambda b: a - b))".to_string(),
        "/" | "/#" => "(lambda a: (lambda b: int(a / b)))".to_string(),
        "/." => "(lambda a: (lambda b: a / b))".to_string(),
        "*" | "*#" | "*." => "(lambda a: (lambda b: a * b))".to_string(),
        "mod" | "mod." => "(lambda a: (lambda b: a % b))".to_string(),
        "=" | "=?" | "=#" | "=." => "(lambda a: (lambda b: a == b))".to_string(),
        "<" | "<#" | "<." => "(lambda a: (lambda b: a < b))".to_string(),
        ">" | ">#" | ">." => "(lambda a: (lambda b: a > b))".to_string(),
        "<=" | "<=#" | "<=." => "(lambda a: (lambda b: a <= b))".to_string(),
        ">=" | ">=#" | ">=." => "(lambda a: (lambda b: a >= b))".to_string(),
        "not" => "(lambda a: (not a))".to_string(),
        "and" => "(lambda a: (lambda b: a and b))".to_string(),
        "or" => "(lambda a: (lambda b: a or b))".to_string(),
        "^" => "(lambda a: (lambda b: a ^ b))".to_string(),
        ">>" => "(lambda a: (lambda b: a >> b))".to_string(),
        "<<" => "(lambda a: (lambda b: a << b))".to_string(),
        "|" => "(lambda a: (lambda b: a | b))".to_string(),
        "&" => "(lambda a: (lambda b: a & b))".to_string(),
        "~" => "(lambda a: (~a))".to_string(),
        "fst" => "(lambda a: a[0])".to_string(),
        "snd" => "(lambda a: a[1])".to_string(),
        "car" => "(lambda a: a[0])".to_string(),
        "cdr" => "(lambda a: a[1:])".to_string(),
        "Float->Int" => "(lambda a: int(a))".to_string(),
        "Int->Float" => "(lambda a: float(a))".to_string(),
        "true" => "True".to_string(),
        "false" => "False".to_string(),
        "null" => "None".to_string(),
        _ => {
            let mut s = String::new();
            for c in name.chars() {
                match c {
                    '.' => {
                        s.push('_');
                        for c in idx.to_string().chars() {
                            s.push(c);
                        }
                    }
                    'a'..='z' | 'A'..='Z' | '0'..='9' => s.push(c.to_ascii_lowercase()),
                    ':' | '-' | '*' | '/' => s.push('_'),
                    '?' | '!' | '<' | '>' | '=' | '+' | '^' | '|' | '&' => s.push('_'),
                    _ => s.push('_'),
                }
            }
            if
                s
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
            {
                s = format!("_{}", s);
            }
            let keywords = [
                "false",
                "none",
                "true",
                "and",
                "as",
                "assert",
                "async",
                "await",
                "break",
                "class",
                "continue",
                "def",
                "del",
                "elif",
                "else",
                "except",
                "finally",
                "for",
                "from",
                "global",
                "if",
                "import",
                "in",
                "is",
                "lambda",
                "nonlocal",
                "not",
                "or",
                "pass",
                "raise",
                "return",
                "try",
                "while",
                "with",
                "yield",
                "sum",
                "range",
                "map",
                "filter",
                "len",
            ];
            if keywords.contains(&s.as_str()) {
                s.push('_');
            }
            format!("v_{}", s)
        }
    }
}

fn compile_curried_call(func_py: String, args_py: &[String]) -> String {
    if args_py.is_empty() {
        return format!("({})()", func_py);
    }
    args_py
        .iter()
        .fold(func_py, |acc, arg| format!("({})({})", acc, arg))
}

fn compile_do_expr(items: &[Expression]) -> String {
    if items.len() < 2 {
        return "None".to_string();
    }
    let mut exprs = Vec::new();
    for expr in &items[1..] {
        match expr {
            Expression::Apply(parts) if parts.len() == 3 => {
                if let Expression::Word(op) = &parts[0] {
                    if op == "let" || op == "let*" || op == "let~" {
                        if let Expression::Word(name) = &parts[1] {
                            let v = compile_expr_to_py(&parts[2]);
                            exprs.push(format!("({} := ({}))", ident(name, 0), v));
                            continue;
                        }
                    }
                }
                exprs.push(compile_expr_to_py(expr));
            }
            _ => exprs.push(compile_expr_to_py(expr)),
        }
    }
    format!("((lambda: [{}][-1])())", exprs.join(", "))
}

fn compile_expr_to_py_inner(expr: &Expression) -> String {
    match expr {
        Expression::Int(n) => format!("{}", n),
        Expression::Float(n) => format!("{:?}", n),
        Expression::Word(w) => ident(w, 0),
        Expression::Apply(items) => {
            if items.is_empty() {
                return "None".to_string();
            }
            match &items[0] {
                Expression::Word(op) =>
                    match op.as_str() {
                        "do" => compile_do_expr(items),
                        "let" | "let*" | "let~" => {
                            if items.len() != 3 {
                                return "None".to_string();
                            }
                            let name = match &items[1] {
                                Expression::Word(n) => ident(n, 0),
                                _ => "__tmp".to_string(),
                            };
                            let val = compile_expr_to_py(&items[2]);
                            format!("({} := ({}))", name, val)
                        }
                        "vector" | "string" | "tuple" => {
                            let elems: Vec<String> = items[1..]
                                .iter()
                                .map(compile_expr_to_py)
                                .collect();
                            format!("[{}]", elems.join(", "))
                        }
                        "length" => format!("len({})", compile_expr_to_py(&items[1])),
                        "get" =>
                            format!(
                                "({}[{}])",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "car" => format!("({}[0])", compile_expr_to_py(&items[1])),
                        "cdr" =>
                            format!(
                                "({}[{}:])",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "fst" => format!("({}[0])", compile_expr_to_py(&items[1])),
                        "snd" => format!("({}[1])", compile_expr_to_py(&items[1])),
                        "set!" =>
                            format!(
                                "_set({}, {}, {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2]),
                                compile_expr_to_py(&items[3])
                            ),
                        "pop!" => format!("_pop({})", compile_expr_to_py(&items[1])),
                        "*" | "*#" | "*." =>
                            format!(
                                "({} * {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "+" | "+#" | "+." =>
                            format!(
                                "({} + {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "-" | "-#" | "-." =>
                            format!(
                                "({} - {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "/" | "/#" =>
                            format!(
                                "int({} / {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "/." =>
                            format!(
                                "({} / {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "mod" | "mod." =>
                            format!(
                                "({} % {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        ">" | ">#" | ">." =>
                            format!(
                                "({} > {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "<" | "<#" | "<." =>
                            format!(
                                "({} < {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        ">=" | ">=#" | ">=." =>
                            format!(
                                "({} >= {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "<=" | "<=#" | "<=." =>
                            format!(
                                "({} <= {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "=" | "=?" | "=#" | "=." =>
                            format!(
                                "({} == {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "or" =>
                            format!(
                                "({} or {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "and" =>
                            format!(
                                "({} and {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "not" => format!("(not {})", compile_expr_to_py(&items[1])),
                        "^" =>
                            format!(
                                "({} ^ {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "|" =>
                            format!(
                                "({} | {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "&" =>
                            format!(
                                "({} & {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "~" => format!("(~{})", compile_expr_to_py(&items[1])),
                        "<<" =>
                            format!(
                                "({} << {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        ">>" =>
                            format!(
                                "({} >> {})",
                                compile_expr_to_py(&items[1]),
                                compile_expr_to_py(&items[2])
                            ),
                        "if" => {
                            if items.len() != 4 {
                                return "None".to_string();
                            }
                            let c = compile_expr_to_py(&items[1]);
                            let t = compile_expr_to_py(&items[2]);
                            let e = compile_expr_to_py(&items[3]);
                            format!("({} if {} else {})", t, c, e)
                        }
                        "loop-finish" => {
                            if items.len() != 3 {
                                return "0".to_string();
                            }
                            let cond = compile_expr_to_py(&items[1]);
                            let f = compile_expr_to_py(&items[2]);
                            format!("_loop_finish((lambda: {}), {})", cond, f)
                        }
                        "loop" => {
                            if items.len() != 4 {
                                return "0".to_string();
                            }
                            let start = compile_expr_to_py(&items[1]);
                            let end = compile_expr_to_py(&items[2]);
                            let f = compile_expr_to_py(&items[3]);
                            format!("_loop({}, {}, {})", start, end, f)
                        }
                        "lambda" => {
                            if items.len() < 2 {
                                return "(lambda: None)".to_string();
                            }
                            let params_exprs = &items[1..items.len() - 1];
                            let params: Vec<String> = params_exprs
                                .iter()
                                .enumerate()
                                .map(|(i, p)| {
                                    match p {
                                        Expression::Word(n) => ident(n, i),
                                        _ => "__arg".to_string(),
                                    }
                                })
                                .collect();
                            let body = compile_expr_to_py(&items[items.len() - 1]);
                            if params.is_empty() {
                                format!("(lambda: {})", body)
                            } else {
                                params
                                    .iter()
                                    .rev()
                                    .fold(body, |acc, p| format!("(lambda {}: {})", p, acc))
                            }
                        }
                        "as" | "char" | "Int->Float" => compile_expr_to_py(&items[1]),
                        "Float->Int" => format!("int({})", compile_expr_to_py(&items[1])),
                        _ => {
                            let f = compile_expr_to_py(&items[0]);
                            let args: Vec<String> = items[1..]
                                .iter()
                                .map(compile_expr_to_py)
                                .collect();
                            compile_curried_call(f, &args)
                        }
                    }
                other_head => {
                    let f = compile_expr_to_py(other_head);
                    let args: Vec<String> = items[1..]
                        .iter()
                        .map(compile_expr_to_py)
                        .collect();
                    compile_curried_call(format!("({})", f), &args)
                }
            }
        }
    }
}

pub fn compile_expr_to_py(expr: &Expression) -> String {
    compile_expr_to_py_inner(expr)
}

pub fn compile_program_to_python(top: &Expression) -> String {
    fn top_level_to_statements(expr: &Expression) -> Vec<String> {
        match expr {
            Expression::Apply(items) if !items.is_empty() => {
                if let Expression::Word(w) = &items[0] {
                    if w == "do" {
                        let mut out = Vec::new();
                        for (i, e) in items[1..].iter().enumerate() {
                            let is_last = i == items.len() - 2;
                            match e {
                                Expression::Apply(let_items) if let_items.len() == 3 => {
                                    if let Expression::Word(kw) = &let_items[0] {
                                        if kw == "let" || kw == "let*" || kw == "let~" {
                                            if let Expression::Word(name) = &let_items[1] {
                                                let v = compile_expr_to_py(&let_items[2]);
                                                out.push(format!("{} = ({})", ident(name, 0), v));
                                                if is_last {
                                                    out.push(format!("return {}", ident(name, 0)));
                                                }
                                                continue;
                                            }
                                        }
                                    }
                                    let src = compile_expr_to_py(e);
                                    if is_last {
                                        out.push(format!("return {}", src));
                                    } else {
                                        out.push(src);
                                    }
                                }
                                _ => {
                                    let src = compile_expr_to_py(e);
                                    if is_last {
                                        out.push(format!("return {}", src));
                                    } else {
                                        out.push(src);
                                    }
                                }
                            }
                        }
                        out
                    } else {
                        vec![format!("return {}", compile_expr_to_py(expr))]
                    }
                } else {
                    vec![format!("return {}", compile_expr_to_py(expr))]
                }
            }
            _ => vec![format!("return {}", compile_expr_to_py(expr))],
        }
    }

    let stmts = top_level_to_statements(top)
        .into_iter()
        .map(|s| format!("    {}", s))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "def _set(xs, i, v):\n    i = int(i)\n    if i == len(xs):\n        xs.append(v)\n    else:\n        xs[i] = v\n    return 0\n\n\
def _pop(xs):\n    if len(xs) > 0:\n        xs.pop()\n    return 0\n\n\
def _loop(start, end, fn_):\n    for i in range(int(start), int(end)):\n        fn_(i)\n    return 0\n\n\
def _loop_finish(cond, fn_):\n    while cond():\n        fn_()\n    return 0\n\n\
def _main():\n{}\n\n\
if __name__ == '__main__':\n    print(_main())\n",
        stmts
    )
}
