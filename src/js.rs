use crate::parser::Expression;
fn ident(name: &str) -> String {
    // make a JS-safe identifier from Lisp name:
    // simple rules: replace ":" "*" "?" "!" "-" "." with underscores, remove other unsafe chars

    match name {
        "+" | "+#" => "(a,b)=>a+b".to_string(),
        "-" | "-#" => "(a,b)=>a-b".to_string(),
        "/" | "/#" => "(a,b)=>(a/b)|0".to_string(),
        // float division
        "/." => "(a,b)=>a/b".to_string(),
        "*" | "*#" => "(a,b)=>a*b".to_string(),
        "mod" => "(a,b)=>a%b".to_string(),
        "=" | "=?" | "=#" => "(a,b)=>a==b".to_string(),
        "<" | "<#" => "(a,b)=>a<b".to_string(),
        ">" | ">#" => "(a,b)=>a>b".to_string(),
        "<=" | "<=#" => "(a,b)=>a<=b".to_string(),
        ">=" | ">=#" => "(a,b)=>a>=b".to_string(),
        "not" => "(a)=>!a".to_string(),
        "and" => "(a,b)=>a&&b".to_string(),
        "or" => "(a,b)=>a||b".to_string(),

        "^" => "(a,b)=>a^b".to_string(),
        ">>" => "(a,b)=>a>>b".to_string(),
        "<<" => "(a,b)=>a<<b".to_string(),
        "|" => "(a,b)=>a|b".to_string(),
        "&" => "(a,b)=>a&b".to_string(),
        "~" => "(a)=>~a".to_string(),

        "fst" => "(a)=>a[0]".to_string(),
        "snd" => "(a)=>a[1]".to_string(),

        "var" => "__var".to_string(),
        "const" => "__const".to_string(),

        "++" => "__inc".to_string(),
        "--" => "__dec".to_string(),
        "+=" => "__inc_by".to_string(),
        "-=" => "__dec_by".to_string(),
        "*=" => "__mult_by".to_string(),
        "/=" => "__div_by".to_string(),
        "**" => "__exp".to_string(),
        "=!" => "__set".to_string(),
        "=!" => "__set".to_string(),
        "true" => "true".to_string(),
        "false" => "false".to_string(),
        "null" => "__null".to_string(),

        "function" => "__function".to_string(),
        _ => {
            let mut s = String::new();
            for c in name.chars() {
                match c {
                    'a'..='z' | 'A'..='Z' | '0'..='9' => s.push(c),
                    ':' | '-' | '.' | '*' | '/' => s.push('_'),
                    '?' => {
                        s.push('_');
                        s.push('p')
                    }
                    '!' => {
                        s.push('_');
                        s.push('m');
                    }
                    _ => s.push('_'),
                }
            }
            // if it begins with a digit, prefix with _
            if s.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                format!("_{}", s)
            } else {
                s
            }
        }
    }
}
pub fn compile_expr_to_js(expr: &Expression) -> String {
    compile_expr_to_js_inner(expr, false)
}

/// Compile a single Expression into a JavaScript expression / snippet.
/// This returns a JS fragment; callers are responsible for combining/terminating top-level statements as needed.
fn compile_expr_to_js_inner(expr: &Expression, in_lambda_body: bool) -> String {
    match expr {
        Expression::Atom(n) => format!("{}", n),
        Expression::Word(w) => ident(w),
        Expression::Apply(items) => {
            // head must be an expression (usually Word)
            match &items[0] {
                Expression::Word(op) => match op.as_str() {
                    "do" => {
                        if items.len() < 2 {
                            panic!("do requires at least one body expression");
                        }
                        let mut compiled: Vec<String> = items[1..]
                            .iter()
                            .map(|e| compile_expr_to_js_inner(e, false))
                            .collect();

                        let last = compiled.pop().unwrap();
                        let mut stmts: Vec<String> =
                            compiled.into_iter().map(|s| format!("{};", s)).collect();

                        if in_lambda_body {
                            stmts.push(format!("return {};", last));
                            format!("{{ {} }}", stmts.join(" "))
                        } else {
                            stmts.push(format!("return {};", last));
                            format!("(()=>{{ {} }})()", stmts.join(" "))
                        }
                    }
                    // let: (let name value) -> `var name = <value>;` as statement. Here we return an expression that
                    // is valid when used in top-level statement context; caller may wrap it as statement.
                    "let" => {
                        if items.len() != 3 {
                            panic!("let requires exactly 2 arguments: name and value");
                        }
                        let name = match &items[1] {
                            Expression::Word(n) => ident(n),
                            _ => panic!("let: first arg must be a Word"),
                        };
                        let val = compile_expr_to_js(&items[2]);
                        format!("var {} = ({})", name, val)
                    }
                    // vector literal - variadic: (vector a b c) -> `[a, b, c]`
                    "vector" | "string" | "tuple" => {
                        let elems: Vec<String> =
                            items[1..].iter().map(|e| compile_expr_to_js(e)).collect();
                        format!("[{}]", elems.join(", "))
                    }
                    "length" => format!("{}.length", compile_expr_to_js(&items[1])),
                    "get" => format!(
                        "({}[{}])",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "fst" => format!("({}[{}])", compile_expr_to_js(&items[1]), 0),
                    "snd" => format!("({}[{}])", compile_expr_to_js(&items[1]), 1),
                    "set!" => format!(
                        "({}[{}]={},0)",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2]),
                        compile_expr_to_js(&items[3])
                    ),
                    "pop!" => format!("({}.pop(),0)", compile_expr_to_js(&items[1])),
                    "*" | "*#" => format!(
                        "({} * {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "+" | "+#" => format!(
                        "({} + {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "-" | "-#" => format!(
                        "({} - {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "/" | "/#" => format!(
                        "(({} / {}) | 0)",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    // float division
                    "/." => format!(
                        "({} / {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "mod" => format!(
                        "({} % {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    ">" | ">#" => format!(
                        "({} > {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "<" | "<#" => format!(
                        "({} < {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    ">=" | ">=#" => format!(
                        "({} >= {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "<=" | "<=#" => format!(
                        "({} <= {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "=" | "=?" | "=#" => format!(
                        "({} == {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "or" => format!(
                        "({} || {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "and" => format!(
                        "({} && {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "not" => format!("!{}", compile_expr_to_js(&items[1])),
                    "^" => format!(
                        "({} ^ {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "|" => format!(
                        "({} | {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "&" => format!(
                        "({} & {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "~" => format!("~{}", compile_expr_to_js(&items[1])),
                    "<<" => format!(
                        "({} << {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    ">>" => format!(
                        "({} >> {})",
                        compile_expr_to_js(&items[1]),
                        compile_expr_to_js(&items[2])
                    ),
                    "if" => {
                        if items.len() != 4 {
                            panic!("if expects exactly 3 arguments: (if cond then else)");
                        }
                        let cond_js = compile_expr_to_js(&items[1]);
                        let then_js = compile_expr_to_js(&items[2]);
                        let else_js = compile_expr_to_js(&items[3]);
                        format!("({} ? {} : {})", cond_js, then_js, else_js)
                    }
                    "loop-finish" => {
                        if items.len() != 3 {
                            panic!("loop-finish expects 2 arguments: condition, lambda");
                        }
                        let cond_js = compile_expr_to_js(&items[1]);
                        let func_js = compile_expr_to_js(&items[2]);
                        format!("(()=>{{while({}){{({})();}}return 0}})()", cond_js, func_js)
                    }
                    "loop" => {
                        if items.len() != 4 {
                            panic!("loop expects 3 arguments: start, end, lambda");
                        }
                        let start_js = compile_expr_to_js(&items[1]);
                        let end_js = compile_expr_to_js(&items[2]);
                        let func_js = compile_expr_to_js(&items[3]);

                        // we assume func_js looks like `(i)=>{ ... }`
                        // so we apply it inside the loop.
                        format!(
                            "(()=>{{for(let __i={}; __i<{}; ++__i){{({})(__i);}}return 0}})()",
                            start_js, end_js, func_js
                        )
                    }
                    // lambda: (lambda p1 p2 ... body) -> JS arrow function
                    "lambda" => {
                        if items.len() < 2 {
                            panic!("lambda expects at least a body");
                        }
                        let params_exprs = &items[1..items.len() - 1];
                        let params: Vec<String> = params_exprs
                            .iter()
                            .map(|p| match p {
                                Expression::Word(n) => ident(n),
                                _ => panic!("lambda parameters must be words"),
                            })
                            .collect();

                        let body_expr = &items[items.len() - 1];
                        let body_js = compile_expr_to_js_inner(body_expr, true);

                        if params.len() == 0 {
                            format!("() => {}", body_js)
                        } else if params.len() == 1 {
                            format!("({}) => {}", params[0], body_js)
                        } else {
                            format!("_curry(({}) => {})", params.join(", "), body_js)
                        }
                    }
                    "char" => format!("{}", compile_expr_to_js(&items[1]),),
                    "as" => format!("{}", compile_expr_to_js(&items[1]),),
                    // call a named function/operator: default: compile args then `fn(args...)`
                    _ => {
                        // compile operator expression (could be a word or more complex expr)
                        let func_js = compile_expr_to_js(&items[0]);
                        let args_js: Vec<String> =
                            items[1..].iter().map(|e| compile_expr_to_js(e)).collect();
                        format!("{}({})", func_js, args_js.join(", "))
                    }
                },

                // If head is not a Word, compile it and call it: (<expr> arg1 arg2)
                other_head => {
                    let func_js = compile_expr_to_js(other_head);
                    let args_js: Vec<String> =
                        items[1..].iter().map(|e| compile_expr_to_js(e)).collect();
                    format!("({})({})", func_js, args_js.join(", "))
                }
            }
        }
    }
}

/// Compile a sequence of top-level expressions into a JS program string.
///
/// The returned string is an IIFE that runs the program and returns the value of the last expression.
/// We assume the top-level `Expression` is typically a `Apply` with `do` or many forms; to be safe we accept a single Expression.
pub fn compile_program_to_js(top: &Expression) -> String {
    // small helper that turns a `do` style (Apply with head "do") into sequence of statements
    fn top_level_to_statements(expr: &Expression) -> Vec<String> {
        match expr {
            Expression::Apply(items) if !items.is_empty() => {
                if let Expression::Word(w) = &items[0] {
                    if (w == "do") {
                        // compile each sub-expression as a statement; last one as returned expression
                        let mut stmts = Vec::new();
                        for (i, e) in items[1..].iter().enumerate() {
                            let js = compile_expr_to_js(e);
                            if i == items.len() - 2 {
                                // last expr -> produce `return <expr>;`
                                stmts.push(format!("return {};", js));
                            } else {
                                // other exprs -> expression statement (if it's a let it already emits var; otherwise add `;`)
                                // If `js` starts with "var " we keep it as a statement, otherwise force a semicolon.
                                if js.starts_with("var ") {
                                    stmts.push(format!("{};", js));
                                } else {
                                    stmts.push(format!("{};", js));
                                }
                            }
                        }
                        stmts
                    } else {
                        panic!("Unreachable!")
                    }
                } else {
                    // not a `do` block: compile as single `return expr;`
                    vec![format!("return {};", compile_expr_to_js(expr))]
                }
            }
            _ => vec![format!("return {};", compile_expr_to_js(expr))],
        }
    }
    let stmts = top_level_to_statements(top);
    let body = stmts.join("\n");
    format!(
        "(()=>{{\n{}\n{}\n}})()",
        "let _mconsole_log_m = console.log; function _curry(func) {return function curried(...args) {if (args.length === func.length){return func(...args);}else{return function (...next) {return curried(...args, ...next);};}};}",
        body
    )
}
