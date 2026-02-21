use crate::infer::TypedExpression;
use crate::parser::Expression;
use crate::types::Type;

fn ident(name: &str, idx: usize) -> String {
    match name {
        "true" => "true".to_string(),
        "false" => "false".to_string(),
        _ => {
            fn push_encoded(s: &mut String, c: char) {
                match c {
                    ':' => s.push_str("_colon_"),
                    '-' => s.push_str("_dash_"),
                    '*' => s.push_str("_star_"),
                    '/' => s.push_str("_slash_"),
                    '?' => s.push_str("_q_"),
                    '!' => s.push_str("_bang_"),
                    '.' => s.push_str("_dot_"),
                    '+' => s.push_str("_plus_"),
                    '<' => s.push_str("_lt_"),
                    '>' => s.push_str("_gt_"),
                    '=' => s.push_str("_eq_"),
                    '|' => s.push_str("_pipe_"),
                    '&' => s.push_str("_amp_"),
                    '^' => s.push_str("_xor_"),
                    _ => s.push('_'),
                }
            }

            let mut s = String::new();
            for c in name.chars() {
                match c {
                    '.' if name == "." => {
                        s.push_str("_hole_");
                        s.push_str(&idx.to_string());
                    }
                    'a'..='z' | 'A'..='Z' | '0'..='9' => s.push(c.to_ascii_lowercase()),
                    | ':'
                    | '-'
                    | '*'
                    | '/'
                    | '?'
                    | '!'
                    | '.'
                    | '+'
                    | '<'
                    | '>'
                    | '='
                    | '|'
                    | '&'
                    | '^' => push_encoded(&mut s, c),
                    _ => push_encoded(&mut s, c),
                }
            }
            if s.is_empty() {
                s = "_ignored".to_string();
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
                "as",
                "break",
                "class",
                "continue",
                "do",
                "else",
                "false",
                "for",
                "fun",
                "if",
                "in",
                "interface",
                "is",
                "null",
                "object",
                "package",
                "return",
                "super",
                "this",
                "throw",
                "true",
                "try",
                "typealias",
                "typeof",
                "val",
                "var",
                "when",
                "while",
            ];
            if keywords.contains(&s.as_str()) {
                s.push('_');
            }
            format!("v_{}", s)
        }
    }
}

fn ident_typed(name: &str, idx: usize, typ: Option<&Type>) -> String {
    let base = ident(name, idx);
    if base == "true" || base == "false" {
        return base;
    }
    match typ {
        Some(Type::Function(_, _)) => format!("{}_fn", base),
        _ => base,
    }
}

fn ident_fn(name: &str, idx: usize) -> String {
    let base = ident(name, idx);
    if base == "true" || base == "false" {
        base
    } else {
        format!("{}_fn", base)
    }
}

fn kotlin_type(t: &Type) -> String {
    match t {
        Type::Int => "Int".to_string(),
        Type::Float => "Double".to_string(),
        Type::Bool => "Boolean".to_string(),
        Type::Char => "Int".to_string(),
        Type::Unit => "Int".to_string(),
        Type::Var(_) => "Any?".to_string(),
        // Kotlin mutable lists are invariant; keep runtime list shape unified to avoid
        // pervasive MutableList<Int> vs MutableList<Any?> mismatches in higher-order std fns.
        Type::List(_) => "MutableList<Any?>".to_string(),
        Type::Tuple(_) => "MutableList<Any?>".to_string(),
        Type::Function(from, to) => {
            if matches!(&**from, Type::Unit) {
                format!("() -> {}", kotlin_type(to))
            } else {
                format!("({}) -> {}", kotlin_type(from), kotlin_type(to))
            }
        }
    }
}

fn kotlin_param_type(t: &Type) -> String {
    match t {
        Type::Function(_, _) => format!("({})", kotlin_type(t)),
        _ => kotlin_type(t),
    }
}

fn cast_binding_value(expr: String, typ: Option<&Type>) -> String {
    match typ {
        Some(Type::Function(_, _)) | Some(Type::Var(_)) | None => expr,
        Some(t) => format!("({}) as {}", expr, kotlin_type(t)),
    }
}

fn function_parts(typ: &Type) -> (Vec<Type>, Type) {
    let mut params = Vec::new();
    let mut current = typ.clone();
    loop {
        match current {
            Type::Function(a, b) => {
                params.push(*a);
                current = *b;
            }
            other => {
                return (params, other);
            }
        }
    }
}

fn op_word(name: &str) -> Option<&'static str> {
    match name {
        "+" | "+#" => Some("{ a: Int -> { b: Int -> a + b } }"),
        "+." => Some("{ a: Double -> { b: Double -> a + b } }"),
        "-" | "-#" => Some("{ a: Int -> { b: Int -> a - b } }"),
        "-." => Some("{ a: Double -> { b: Double -> a - b } }"),
        "*" | "*#" => Some("{ a: Int -> { b: Int -> a * b } }"),
        "*." => Some("{ a: Double -> { b: Double -> a * b } }"),
        "/" | "/#" => Some("{ a: Int -> { b: Int -> a / b } }"),
        "/." => Some("{ a: Double -> { b: Double -> a / b } }"),
        "mod" => Some("{ a: Int -> { b: Int -> a % b } }"),
        "mod." => Some("{ a: Double -> { b: Double -> a % b } }"),
        "=" | "=?" | "=#" | "=." => Some("{ a: Any? -> { b: Any? -> a == b } }"),
        "<" | "<#" => Some("{ a: Int -> { b: Int -> a < b } }"),
        "<." => Some("{ a: Double -> { b: Double -> a < b } }"),
        ">" | ">#" => Some("{ a: Int -> { b: Int -> a > b } }"),
        ">." => Some("{ a: Double -> { b: Double -> a > b } }"),
        "<=" | "<=#" => Some("{ a: Int -> { b: Int -> a <= b } }"),
        "<=." => Some("{ a: Double -> { b: Double -> a <= b } }"),
        ">=" | ">=#" => Some("{ a: Int -> { b: Int -> a >= b } }"),
        ">=." => Some("{ a: Double -> { b: Double -> a >= b } }"),
        "and" => Some("{ a: Boolean -> { b: Boolean -> a && b } }"),
        "or" => Some("{ a: Boolean -> { b: Boolean -> a || b } }"),
        "not" => Some("{ a: Boolean -> !a }"),
        "^" => Some("{ a: Int -> { b: Int -> a xor b } }"),
        "|" => Some("{ a: Int -> { b: Int -> a or b } }"),
        "&" => Some("{ a: Int -> { b: Int -> a and b } }"),
        "<<" => Some("{ a: Int -> { b: Int -> a shl b } }"),
        ">>" => Some("{ a: Int -> { b: Int -> a shr b } }"),
        "~" => Some("{ a: Int -> a.inv() }"),
        "fst" => Some("{ p: MutableList<Any?> -> p[0] }"),
        "snd" => Some("{ p: MutableList<Any?> -> p[1] }"),
        "car" => Some("{ xs: MutableList<Any?> -> xs[0] }"),
        "cdr" => Some("{ xs: MutableList<Any?> -> { i: Int -> xs.drop(i).toMutableList() } }"),
        "Int->Float" => Some("{ a: Int -> a.toDouble() }"),
        "Float->Int" => Some("{ a: Double -> a.toInt() }"),
        _ => None,
    }
}

fn compile_call(children: &[TypedExpression]) -> String {
    if children.is_empty() {
        return "0".to_string();
    }
    if children.len() == 1 {
        return match &children[0].expr {
            Expression::Word(w) if op_word(w).is_none() => {
                format!("(({} as () -> Any?))()", ident_fn(w, 0))
            }
            _ => {
                let out = compile_expr(&children[0]);
                format!("(({} as () -> Any?))()", out)
            }
        };
    }
    fn cast_to(value: &str, typ: &Type) -> String {
        match typ {
            Type::Int => format!("({} as Int)", value),
            Type::Float => format!("({} as Double)", value),
            Type::Bool => format!("({} as Boolean)", value),
            Type::Char => format!("({} as Int)", value),
            Type::List(_) | Type::Tuple(_) => format!("({} as MutableList<Any?>)", value),
            Type::Var(_) => format!("({} as Any?)", value),
            Type::Function(_, _) | Type::Unit => value.to_string(),
        }
    }
    fn adapt_expr(expr: String, from: &Type, to: &Type, depth: usize) -> String {
        match (from, to) {
            (Type::Function(f_in, f_out), Type::Function(_, t_out)) => {
                let p = format!("v__adapt_{}", depth);
                let callee = format!("({} as {})", expr, kotlin_type(from));
                let applied = format!("({})({})", callee, cast_to(&p, f_in));
                let body = adapt_expr(applied, f_out, t_out, depth + 1);
                format!("{{ {}: Any? -> {} }}", p, body)
            }
            _ => cast_to(&expr, to),
        }
    }

    let mut out = match &children[0].expr {
        Expression::Word(w) if op_word(w).is_none() => {
            let id = ident_fn(w, 0);
            match children[0].typ.as_ref() {
                Some(t @ Type::Function(_, _)) => format!("({} as {})", id, kotlin_type(t)),
                _ => id,
            }
        }
        _ => compile_expr(&children[0]),
    };
    let mut current_type = children[0].typ.as_ref().cloned();
    for arg in &children[1..] {
        let mut arg_expr = compile_expr(arg);
        if
            let (Some(Type::Function(expected_arg, ret)), Some(actual_arg)) = (
                current_type.as_ref(),
                arg.typ.as_ref(),
            )
        {
            if
                matches!(
                    (&**expected_arg, actual_arg),
                    (Type::Function(_, _), Type::Function(_, _))
                )
            {
                arg_expr = adapt_expr(arg_expr, actual_arg, expected_arg, 0);
            } else if !matches!(&**expected_arg, Type::Function(_, _)) {
                arg_expr = cast_to(&arg_expr, expected_arg);
            }
            current_type = Some((**ret).clone());
        } else {
            current_type = None;
        }
        out = format!("({})({})", out, arg_expr);
    }
    out
}

fn cast_result_to_node_type(expr: String, typ: Option<&Type>) -> String {
    match typ {
        Some(Type::Int) => format!("({}) as Int", expr),
        Some(Type::Float) => format!("({}) as Double", expr),
        Some(Type::Bool) => format!("({}) as Boolean", expr),
        Some(Type::Char) => format!("({}) as Int", expr),
        _ => expr,
    }
}

fn compile_do(items: &[Expression], node: &TypedExpression) -> String {
    if items.len() <= 1 {
        return "0".to_string();
    }
    // Some typed trees include the operator as child[0], others only include operands.
    let child_offset = if node.children.len() + 1 == items.len() { 1 } else { 0 };
    let child_at = |item_idx: usize| -> Option<&TypedExpression> {
        if item_idx < child_offset { None } else { node.children.get(item_idx - child_offset) }
    };
    let mut lines = Vec::new();
    for i in 1..items.len() - 1 {
        if let Expression::Apply(let_items) = &items[i] {
            if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                if kw == "let" || kw == "let~" || kw == "let*" {
                    let val_node = child_at(i).and_then(|n| n.children.get(2));
                    let value = val_node.map(compile_expr).unwrap_or_else(|| "0".to_string());
                    let binding = if kw == "let*" { "var" } else { "val" };
                    let id = ident_typed(
                        name,
                        0,
                        val_node.and_then(|n| n.typ.as_ref())
                    );
                    let id = match let_items.get(2) {
                        Some(Expression::Apply(xs)) if
                            matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                        => {
                            ident_fn(name, 0)
                        }
                        _ => id,
                    };
                    let t = val_node
                        .and_then(|n| n.typ.as_ref())
                        .map(kotlin_type)
                        .unwrap_or_else(|| "Any?".to_string());
                    let value = cast_binding_value(
                        value,
                        val_node.and_then(|n| n.typ.as_ref())
                    );
                    let is_lambda_rhs =
                        matches!(
                        let_items.get(2),
                        Some(Expression::Apply(xs))
                            if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                    );
                    if kw == "let*" && is_lambda_rhs {
                        lines.push(format!("lateinit var {}: {};", id, t));
                        lines.push(format!("{} = {};", id, value));
                    } else {
                        lines.push(format!("{} {} = {};", binding, id, value));
                    }
                    continue;
                }
            }
        }
        if let Some(n) = child_at(i) {
            lines.push(format!("{};", compile_expr(n)));
        }
    }
    let last = child_at(items.len() - 1)
        .map(compile_expr)
        .unwrap_or_else(|| "0".to_string());
    if lines.is_empty() {
        format!("run {{ {} }}", last)
    } else {
        format!("run {{ {} {} }}", lines.join(" "), last)
    }
}

fn compile_lambda(items: &[Expression], node: &TypedExpression) -> String {
    if items.len() < 2 {
        return "{ _: Any? -> 0 }".to_string();
    }
    let body_idx = items.len() - 1;
    let body = node.children
        .get(body_idx)
        .map(compile_expr)
        .unwrap_or_else(|| "0".to_string());
    let param_types = node.typ
        .as_ref()
        .map(|t| function_parts(t).0)
        .unwrap_or_default();
    let params = items[1..body_idx]
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = match p {
                Expression::Word(w) => ident_typed(w, i, param_types.get(i)),
                _ => format!("v__p{}", i),
            };
            match param_types.get(i) {
                Some(Type::Function(_, _)) => format!("{}: Any?", name),
                _ => {
                    let ty = param_types
                        .get(i)
                        .map(kotlin_param_type)
                        .unwrap_or_else(|| "Any?".to_string());
                    format!("{}: {}", name, ty)
                }
            }
        })
        .collect::<Vec<_>>();

    if params.is_empty() {
        format!("{{ -> {} }}", body)
    } else {
        params
            .iter()
            .rev()
            .fold(body, |acc, p| format!("{{ {} -> {} }}", p, acc))
    }
}

pub fn compile_expr(node: &TypedExpression) -> String {
    match &node.expr {
        Expression::Int(n) => format!("{}", n),
        Expression::Float(n) => format!("{:?}", n),
        Expression::Word(w) => {
            if let Some(op) = op_word(w) {
                op.to_string()
            } else {
                let id = ident_typed(w, 0, node.typ.as_ref());
                match node.typ.as_ref() {
                    Some(t @ Type::Function(_, _)) => format!("({} as {})", id, kotlin_type(t)),
                    _ => id,
                }
            }
        }
        Expression::Apply(items) => {
            if items.is_empty() {
                return "0".to_string();
            }
            match &items[0] {
                Expression::Word(op) =>
                    match op.as_str() {
                        "do" => compile_do(items, node),
                        "let" | "let~" | "let*" => {
                            if items.len() != 3 {
                                return "0".to_string();
                            }
                            let value_node = node.children.get(2);
                            let name = match &items[1] {
                                Expression::Word(n) =>
                                    ident_typed(
                                        n,
                                        0,
                                        value_node.and_then(|n| n.typ.as_ref())
                                    ),
                                _ => "_tmp".to_string(),
                            };
                            let name = match items.get(2) {
                                Some(Expression::Apply(xs)) if
                                    matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                                => {
                                    match &items[1] {
                                        Expression::Word(n) => ident_fn(n, 0),
                                        _ => name,
                                    }
                                }
                                _ => name,
                            };
                            let v = value_node.map(compile_expr).unwrap_or_else(|| "0".to_string());
                            let kw = if op == "let*" { "var" } else { "val" };
                            let t = value_node
                                .and_then(|n| n.typ.as_ref())
                                .map(kotlin_type)
                                .unwrap_or_else(|| "Any?".to_string());
                            let v = cast_binding_value(
                                v,
                                value_node.and_then(|n| n.typ.as_ref())
                            );
                            let is_lambda_rhs =
                                matches!(
                            items.get(2),
                            Some(Expression::Apply(xs))
                                if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                        );
                            if op == "let*" && is_lambda_rhs {
                                format!(
                                    "run {{ lateinit var {}: {}; {} = {}; 0 }}",
                                    name,
                                    t,
                                    name,
                                    v
                                )
                            } else {
                                format!("run {{ {} {} = {}; 0 }}", kw, name, v)
                            }
                        }
                        "vector" | "string" => {
                            let elems = node.children[1..]
                                .iter()
                                .map(compile_expr)
                                .collect::<Vec<_>>();
                            format!("mutableListOf<Any?>({})", elems.join(", "))
                        }
                        "tuple" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("mutableListOf<Any?>({}, {})", a, b)
                        }
                        "length" =>
                            format!(
                                "(({}) as MutableList<Any?>).size",
                                compile_expr(&node.children[1])
                            ),
                        "get" =>
                            cast_result_to_node_type(
                                format!(
                                    "(({}) as MutableList<Any?>)[({}) as Int]",
                                    compile_expr(&node.children[1]),
                                    compile_expr(&node.children[2])
                                ),
                                node.typ.as_ref()
                            ),
                        "car" =>
                            cast_result_to_node_type(
                                format!(
                                    "(({}) as MutableList<Any?>)[0]",
                                    compile_expr(&node.children[1])
                                ),
                                node.typ.as_ref()
                            ),
                        "cdr" =>
                            format!(
                                "({}).drop(({}) as Int).toMutableList()",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "fst" =>
                            cast_result_to_node_type(
                                format!(
                                    "(({}) as MutableList<Any?>)[0]",
                                    compile_expr(&node.children[1])
                                ),
                                node.typ.as_ref()
                            ),
                        "snd" =>
                            cast_result_to_node_type(
                                format!(
                                    "(({}) as MutableList<Any?>)[1]",
                                    compile_expr(&node.children[1])
                                ),
                                node.typ.as_ref()
                            ),
                        "set!" =>
                            format!(
                                "run {{ vecSet({}, {}, {}); 0 }}",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2]),
                                compile_expr(&node.children[3])
                            ),
                        "pop!" =>
                            format!("run {{ vecPop({}); 0 }}", compile_expr(&node.children[1])),
                        "+" | "+#" =>
                            format!(
                                "(({}) as Int + (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "+." =>
                            format!(
                                "(({}) as Double + (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "-" | "-#" =>
                            format!(
                                "(({}) as Int - (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "-." =>
                            format!(
                                "(({}) as Double - (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "*" | "*#" =>
                            format!(
                                "(({}) as Int * (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "*." =>
                            format!(
                                "(({}) as Double * (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "/" | "/#" =>
                            format!(
                                "(({}) as Int / (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "/." =>
                            format!(
                                "(({}) as Double / (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "mod" =>
                            format!(
                                "(({}) as Int % (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "mod." =>
                            format!(
                                "(({}) as Double % (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "=" | "=?" | "=#" | "=." =>
                            format!(
                                "({} == {})",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "<" | "<#" =>
                            format!(
                                "((({}) as Int) < (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "<." =>
                            format!(
                                "((({}) as Double) < (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        ">" | ">#" =>
                            format!(
                                "((({}) as Int) > (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        ">." =>
                            format!(
                                "((({}) as Double) > (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "<=" | "<=#" =>
                            format!(
                                "((({}) as Int) <= (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "<=." =>
                            format!(
                                "((({}) as Double) <= (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        ">=" | ">=#" =>
                            format!(
                                "((({}) as Int) >= (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        ">=." =>
                            format!(
                                "((({}) as Double) >= (({}) as Double))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "and" =>
                            format!(
                                "(({}) as Boolean && (({}) as Boolean))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "or" =>
                            format!(
                                "(({}) as Boolean || (({}) as Boolean))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "not" => format!("(!(({}) as Boolean))", compile_expr(&node.children[1])),
                        "^" =>
                            format!(
                                "(({}) as Int xor (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "|" =>
                            format!(
                                "(({}) as Int or (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "&" =>
                            format!(
                                "(({}) as Int and (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "<<" =>
                            format!(
                                "(({}) as Int shl (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        ">>" =>
                            format!(
                                "(({}) as Int shr (({}) as Int))",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2])
                            ),
                        "~" => format!("(({}) as Int).inv()", compile_expr(&node.children[1])),
                        "if" =>
                            format!(
                                "(if (({}) as Boolean) {} else {})",
                                compile_expr(&node.children[1]),
                                compile_expr(&node.children[2]),
                                compile_expr(&node.children[3])
                            ),
                        "loop" => {
                            let start = compile_expr(&node.children[1]);
                            let end = compile_expr(&node.children[2]);
                            let f = compile_expr(&node.children[3]);
                            format!("loop({}, {}, {})", start, end, f)
                        }
                        "loop-finish" => {
                            let c = compile_expr(&node.children[1]);
                            let f_wrapped = match &node.children[2].expr {
                                Expression::Apply(items) if
                                    matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
                                => {
                                    compile_expr(&node.children[2])
                                }
                                Expression::Word(w) => {
                                    // Avoid reusing compile_expr() here because it may pre-cast to a wrong
                                    // function arity; loopFinish always requires a zero-arg thunk.
                                    let fn_ref = ident_typed(w, 0, node.children[2].typ.as_ref());
                                    format!("{{ (({} as () -> Any?))() }}", fn_ref)
                                }
                                _ => {
                                    let f = compile_expr(&node.children[2]);
                                    format!("{{ (({} as () -> Any?))() }}", f)
                                }
                            };
                            format!("loopFinish({{ {} }}, {})", c, f_wrapped)
                        }
                        "lambda" => compile_lambda(items, node),
                        "as" | "char" =>
                            node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string()),
                        "Int->Float" =>
                            format!("(({}) as Int).toDouble()", compile_expr(&node.children[1])),
                        "Float->Int" =>
                            format!("(({}) as Double).toInt()", compile_expr(&node.children[1])),
                        _ =>
                            cast_result_to_node_type(
                                compile_call(&node.children),
                                node.typ.as_ref()
                            ),
                    }
                _ => cast_result_to_node_type(compile_call(&node.children), node.typ.as_ref()),
            }
        }
    }
}

fn annotate_untyped_lambda_params(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len() + 64);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            out.push('{');
            i += 1;
            let ws_start = i;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                out.push(bytes[i] as char);
                i += 1;
            }
            let name_start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if i > name_start {
                let name = &src[name_start..i];
                let mut k = i;
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if
                    k + 1 < bytes.len() &&
                    bytes[k] == b'-' &&
                    bytes[k + 1] == b'>' &&
                    !name.is_empty()
                {
                    out.push_str(name);
                    out.push_str(": Any?");
                    while i < k {
                        out.push(bytes[i] as char);
                        i += 1;
                    }
                    continue;
                }
            } else if ws_start == i {
                // nothing special
            }
            // fallback: copy consumed token bytes as-is
            if i > name_start {
                out.push_str(&src[name_start..i]);
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

pub fn compile_program_to_kotlin_typed(typed_ast: &TypedExpression) -> String {
    let body = annotate_untyped_lambda_params(&compile_expr(typed_ast));
    let ret_t = typed_ast.typ
        .as_ref()
        .map(kotlin_type)
        .unwrap_or_else(|| "Any?".to_string());
    format!(
        "@file:Suppress(\"USELESS_CAST\", \"UNCHECKED_CAST\")\n\n\
import kotlin.math.*\n\n\
fun <T> vecSet(xs: MutableList<T>, i: Int, x: T): Int {{\n\
  if (i == xs.size) xs.add(x) else xs[i] = x\n\
  return 0\n\
}}\n\n\
fun <T> vecPop(xs: MutableList<T>): Int {{\n\
  if (xs.isNotEmpty()) xs.removeAt(xs.size - 1)\n\
  return 0\n\
}}\n\n\
fun loop(start: Int, end: Int, fn: (Int) -> Any?): Int {{\n\
  var i = start\n\
  val e = end\n\
  while (i < e) {{ fn(i); i += 1 }}\n\
  return 0\n\
}}\n\n\
fun loopFinish(cond: () -> Boolean, fn: () -> Any?): Int {{\n\
  while (cond()) {{ fn() }}\n\
  return 0\n\
}}\n\n\
fun pretty(v: Any?): String = when (v) {{\n\
  null -> \"()\"\n\
  is Boolean -> if (v) \"true\" else \"false\"\n\
  is MutableList<*> -> \"[\" + v.joinToString(\" \") {{ pretty(it) }} + \"]\"\n\
  is List<*> -> \"[\" + v.joinToString(\" \") {{ pretty(it) }} + \"]\"\n\
  is Pair<*, *> -> \"[\" + pretty(v.first) + \" \" + pretty(v.second) + \"]\"\n\
  else -> v.toString()\n\
}}\n\n\
fun _main(): {} = (({}) as {})\n\n\
fun main() {{\n\
  println(pretty(_main()))\n\
}}\n",
        ret_t,
        body,
        ret_t
    )
}

pub fn compile_program_to_kotlin(expr: &Expression) -> Result<String, String> {
    let (_typ, typed_ast) = crate::infer::infer_with_builtins_typed(
        expr,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
    )?;
    Ok(compile_program_to_kotlin_typed(&typed_ast))
}
