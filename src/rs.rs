use crate::infer::TypedExpression;
use crate::parser::Expression;
use crate::types::Type;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static DECL_ARITY: RefCell<HashMap<String, usize>> = RefCell::new(HashMap::new());
}

fn ident(name: &str) -> String {
    match name {
        "true" => "true".to_string(),
        "false" => "false".to_string(),
        "+" => "(|a, b| a + b)".to_string(),
        "+#" => "(|a, b| a + b)".to_string(),
        "+." => "(|a, b| a + b)".to_string(),
        "-" => "(|a, b| a - b)".to_string(),
        "-#" => "(|a, b| a - b)".to_string(),
        "-." => "(|a, b| a - b)".to_string(),
        "*" => "(|a, b| a * b)".to_string(),
        "*#" => "(|a, b| a * b)".to_string(),
        "*." => "(|a, b| a * b)".to_string(),
        "/" => "(|a, b| a / b)".to_string(),
        "/#" => "(|a, b| a / b)".to_string(),
        "/." => "(|a, b| a / b)".to_string(),
        "mod" => "(|a, b| a % b)".to_string(),
        "mod." => "(|a, b| a % b)".to_string(),
        "=" => "(|a, b| a == b)".to_string(),
        "=?" => "(|a, b| a == b)".to_string(),
        "=#" => "(|a, b| a == b)".to_string(),
        "=." => "(|a, b| a == b)".to_string(),
        "<" => "(|a, b| a < b)".to_string(),
        "<#" => "(|a, b| a < b)".to_string(),
        "<." => "(|a, b| a < b)".to_string(),
        ">" => "(|a, b| a > b)".to_string(),
        ">#" => "(|a, b| a > b)".to_string(),
        ">." => "(|a, b| a > b)".to_string(),
        "<=" => "(|a, b| a <= b)".to_string(),
        "<=#" => "(|a, b| a <= b)".to_string(),
        "<=." => "(|a, b| a <= b)".to_string(),
        ">=" => "(|a, b| a >= b)".to_string(),
        ">=#" => "(|a, b| a >= b)".to_string(),
        ">=." => "(|a, b| a >= b)".to_string(),
        "not" => "(|a| !a)".to_string(),
        "and" => "(|a, b| a && b)".to_string(),
        "or" => "(|a, b| a || b)".to_string(),
        "^" => "(|a, b| a ^ b)".to_string(),
        "|" => "(|a, b| a | b)".to_string(),
        "&" => "(|a, b| a & b)".to_string(),
        "~" => "(|a| !a)".to_string(),
        "<<" => "(|a, b| a << b)".to_string(),
        ">>" => "(|a, b| a >> b)".to_string(),
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
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => {
                        s.push(c);
                    }
                    ':' | '-' | '*' | '/' | '?' | '!' | '.' | '+' | '<' | '>' | '=' | '|' | '&' | '^' => push_encoded(&mut s, c),
                    _ => push_encoded(&mut s, c),
                }
            }
            let mut out = if s.is_empty() {
                "__ignored".to_string()
            } else if
                s
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
            {
                format!("_{}", s)
            } else {
                s
            };

            const RUST_KEYWORDS: &[&str] = &[
                "as",
                "break",
                "const",
                "continue",
                "crate",
                "else",
                "enum",
                "extern",
                "false",
                "fn",
                "for",
                "if",
                "impl",
                "in",
                "let",
                "loop",
                "match",
                "mod",
                "move",
                "mut",
                "pub",
                "ref",
                "return",
                "self",
                "Self",
                "static",
                "struct",
                "super",
                "trait",
                "true",
                "type",
                "unsafe",
                "use",
                "where",
                "while",
                "async",
                "await",
                "dyn",
                "abstract",
                "become",
                "box",
                "do",
                "final",
                "macro",
                "override",
                "priv",
                "typeof",
                "unsized",
                "virtual",
                "yield",
                "try",
            ];
            if out == "_" {
                out = "__ignored".to_string();
            } else if RUST_KEYWORDS.contains(&out.as_str()) {
                out.push('_');
            }
            out
        }
    }
}

fn rust_type(typ: &Type) -> String {
    match typ {
        Type::Int => "i32".to_string(),
        Type::Float => "f32".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Char => "i32".to_string(),
        Type::Unit => "i32".to_string(),
        Type::List(inner) => format!("std::rc::Rc<std::cell::RefCell<Vec<{}>>>", rust_type(inner)),
        Type::Tuple(items) => {
            let elems = items.iter().map(rust_type).collect::<Vec<_>>();
            match elems.len() {
                0 => "()".to_string(),
                1 => format!("({},)", elems[0]),
                _ => format!("({})", elems.join(", ")),
            }
        }
        Type::Function(_, _) => {
            let (params, ret) = function_parts(typ);
            let ps = if params.is_empty() {
                String::new()
            } else {
                params.iter().map(rust_type).collect::<Vec<_>>().join(", ")
            };
            format!("fn({}) -> {}", ps, rust_type(&ret))
        }
        Type::Var(_) => "_".to_string(),
    }
}

fn rust_type_with_vars(typ: &Type, vars: &std::collections::HashMap<u64, String>) -> String {
    match typ {
        Type::Int => "i32".to_string(),
        Type::Float => "f32".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Char => "i32".to_string(),
        Type::Unit => "i32".to_string(),
        Type::List(inner) => {
            format!("std::rc::Rc<std::cell::RefCell<Vec<{}>>>", rust_type_with_vars(inner, vars))
        }
        Type::Tuple(items) => {
            let elems = items
                .iter()
                .map(|t| rust_type_with_vars(t, vars))
                .collect::<Vec<_>>();
            match elems.len() {
                0 => "()".to_string(),
                1 => format!("({},)", elems[0]),
                _ => format!("({})", elems.join(", ")),
            }
        }
        Type::Function(_, _) => {
            let (params, ret) = function_parts(typ);
            let ps = params
                .iter()
                .map(|p| rust_type_with_vars(p, vars))
                .collect::<Vec<_>>()
                .join(", ");
            format!("fn({}) -> {}", ps, rust_type_with_vars(&ret, vars))
        }
        Type::Var(v) =>
            vars
                .get(&v.id)
                .cloned()
                .unwrap_or_else(|| "i32".to_string()),
    }
}

fn rust_type_with_infer_vars(typ: &Type) -> String {
    match typ {
        Type::Int => "i32".to_string(),
        Type::Float => "f32".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Char => "i32".to_string(),
        Type::Unit => "i32".to_string(),
        Type::List(inner) => {
            format!("std::rc::Rc<std::cell::RefCell<Vec<{}>>>", rust_type_with_infer_vars(inner))
        }
        Type::Tuple(items) => {
            let elems = items.iter().map(rust_type_with_infer_vars).collect::<Vec<_>>();
            match elems.len() {
                0 => "()".to_string(),
                1 => format!("({},)", elems[0]),
                _ => format!("({})", elems.join(", ")),
            }
        }
        Type::Function(_, _) => {
            let (params, ret) = function_parts(typ);
            let ps = if params.is_empty() {
                String::new()
            } else {
                params.iter().map(rust_type_with_infer_vars).collect::<Vec<_>>().join(", ")
            };
            format!("fn({}) -> {}", ps, rust_type_with_infer_vars(&ret))
        }
        Type::Var(_) => "_".to_string(),
    }
}

fn collect_type_vars(typ: &Type, out: &mut std::collections::BTreeSet<u64>) {
    match typ {
        Type::Var(v) => {
            out.insert(v.id);
        }
        Type::List(inner) => collect_type_vars(inner, out),
        Type::Function(a, b) => {
            collect_type_vars(a, out);
            collect_type_vars(b, out);
        }
        Type::Tuple(items) => {
            for item in items {
                collect_type_vars(item, out);
            }
        }
        _ => {}
    }
}

fn function_parts(typ: &Type) -> (Vec<Type>, Type) {
    let mut params = Vec::new();
    let mut current = typ.clone();
    loop {
        match current {
            Type::Function(a, b) => {
                if matches!(*a, Type::Unit) {
                    current = *b;
                    continue;
                }
                params.push(*a);
                current = *b;
            }
            other => {
                return (params, other);
            }
        }
    }
}

fn compile_named_function(
    name: &str,
    lambda_items: &[Expression],
    lambda_node: &TypedExpression
) -> String {
    let body_idx = lambda_items.len() - 1;
    let body = lambda_node.children
        .get(body_idx)
        .map(|n| compile_expr_with_mode(n, false))
        .unwrap_or_else(|| "()".to_string());

    let lambda_type = lambda_node.typ.clone().unwrap_or(Type::Unit);
    let (param_types, ret_type) = function_parts(&lambda_type);

    fn collect_as_var_narrowings(
        node: &TypedExpression,
        out: &mut std::collections::HashMap<u64, Type>
    ) {
        if let Expression::Apply(items) = &node.expr {
            if
                !items.is_empty() &&
                matches!(&items[0], Expression::Word(w) if w == "as") &&
                node.children.len() >= 2
            {
                let src = node.children[1].typ.as_ref();
                let dst = node.typ.as_ref();
                if let (Some(Type::Var(v)), Some(t)) = (src, dst) {
                    if !matches!(t, Type::Var(_)) {
                        out.insert(v.id, t.clone());
                    }
                }
            }
        }
        for ch in &node.children {
            collect_as_var_narrowings(ch, out);
        }
    }

    let mut tvs = std::collections::BTreeSet::new();
    collect_type_vars(&lambda_type, &mut tvs);
    let mut narrowed: std::collections::HashMap<u64, Type> = std::collections::HashMap::new();
    collect_as_var_narrowings(lambda_node, &mut narrowed);

    let mut type_var_map: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    let mut generic_names: Vec<String> = Vec::new();
    let mut next_generic = 0usize;
    for id in &tvs {
        if let Some(t) = narrowed.get(id) {
            type_var_map.insert(*id, rust_type(t));
        } else {
            let g = format!("T{}", next_generic);
            next_generic += 1;
            type_var_map.insert(*id, g.clone());
            generic_names.push(g);
        }
    }

    let mut params = Vec::new();
    let mut fn_generic_bounds = Vec::new();
    for (i, expr) in lambda_items[1..body_idx].iter().enumerate() {
        if let Expression::Word(param_name) = expr {
            let ty = param_types.get(i).map_or_else(
                || "i32".to_string(),
                |t| {
                    if let Type::Function(_, _) = t {
                        let g = format!("F{}", i);
                        let (fn_params, fn_ret) = function_parts(t);
                        let sig = fn_params
                            .iter()
                            .map(|p| rust_type_with_vars(p, &type_var_map))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let ret = rust_type_with_vars(&fn_ret, &type_var_map);
                        fn_generic_bounds.push(format!("{}: Fn({}) -> {}", g, sig, ret));
                        g
                    } else {
                        rust_type_with_vars(t, &type_var_map)
                    }
                }
            );
            params.push(format!("{}: {}", ident(param_name), ty));
        }
    }

    generic_names.sort();
    generic_names.dedup();
    let mut generic_bounds = generic_names
        .into_iter()
        .map(|p| format!("{}: Clone", p))
        .collect::<Vec<_>>();
    generic_bounds.extend(fn_generic_bounds);
    let generics = if generic_bounds.is_empty() {
        "".to_string()
    } else {
        format!("<{}>", generic_bounds.join(", "))
    };

    let ret = rust_type_with_vars(&ret_type, &type_var_map);
    let body = if matches!(ret_type, Type::Unit) {
        format!("{{ let _ = {}; 0i32 }}", body)
    } else {
        body
    };
    format!("fn {}{}({}) -> {} {{ {} }}", ident(name), generics, params.join(", "), ret, body)
}

fn rust_type_mono(typ: &Type) -> String {
    let t = rust_type(typ);
    if t == "_" {
        "i32".to_string()
    } else {
        t
    }
}

fn compile_recursive_let_lambda(
    name: &str,
    lambda_items: &[Expression],
    lambda_node: &TypedExpression
) -> String {
    let body_idx = lambda_items.len().saturating_sub(1);
    let body = lambda_node.children
        .get(body_idx)
        .map(|n| compile_expr_with_mode(n, false))
        .unwrap_or_else(|| "()".to_string());

    let lambda_type = lambda_node.typ.clone().unwrap_or(Type::Unit);
    let (param_types, ret_type) = function_parts(&lambda_type);
    let ret = rust_type_mono(&ret_type);
    let name_id = ident(name);

    let mut params_decl = Vec::new();
    let mut arg_names = Vec::new();
    for (i, expr) in lambda_items[1..body_idx].iter().enumerate() {
        if let Expression::Word(param_name) = expr {
            let ty = param_types
                .get(i)
                .map(rust_type_mono)
                .unwrap_or_else(|| "i32".to_string());
            let pn = ident(param_name);
            arg_names.push(pn.clone());
            params_decl.push(format!("{}: {}", pn, ty));
        }
    }
    let fn_sig_args = params_decl.join(", ");
    let call_args = arg_names.join(", ");
    let dyn_fn_type = format!(
        "dyn Fn({}) -> {}",
        param_types.iter().map(rust_type_mono).collect::<Vec<_>>().join(", "),
        ret
    );

    format!(
        "let mut {name_id} = {{ \
            let __self: std::rc::Rc<std::cell::RefCell<Option<Box<{dyn_fn_type}>>>> = std::rc::Rc::new(std::cell::RefCell::new(None)); \
            let __self_for_call = __self.clone(); \
            let __self_call = {{ \
                let __self2 = __self.clone(); \
                move |{fn_sig_args}| -> {ret} {{ (__self2.borrow().as_ref().unwrap())({call_args}) }} \
            }}; \
            let __f = {{ \
                let {name_id} = __self_call; \
                move |{fn_sig_args}| -> {ret} {{ {body} }} \
            }}; \
            *__self.borrow_mut() = Some(Box::new(__f)); \
            move |{fn_sig_args}| -> {ret} {{ (__self_for_call.borrow().as_ref().unwrap())({call_args}) }} \
        }};"
    )
}

fn type_contains_var(t: &Type) -> bool {
    match t {
        Type::Var(_) => true,
        Type::Function(from, to) => type_contains_var(from) || type_contains_var(to),
        Type::List(inner) => type_contains_var(inner),
        Type::Tuple(inner) => inner.iter().any(type_contains_var),
        _ => false,
    }
}

fn compile_function_alias(alias: &str, target: &str, fn_type: &Type) -> String {
    let (param_types, ret_type) = function_parts(fn_type);
    let mut tvs = std::collections::BTreeSet::new();
    collect_type_vars(fn_type, &mut tvs);
    let mut type_var_map: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    for (i, id) in tvs.iter().enumerate() {
        type_var_map.insert(*id, format!("T{}", i));
    }
    let mut generic_bounds = tvs
        .iter()
        .enumerate()
        .map(|(i, _)| format!("T{}: Clone", i))
        .collect::<Vec<_>>();
    generic_bounds.sort();
    generic_bounds.dedup();
    let generics = if generic_bounds.is_empty() {
        String::new()
    } else {
        format!("<{}>", generic_bounds.join(", "))
    };

    let mut params_decl = Vec::new();
    let mut call_args = Vec::new();
    for (i, p) in param_types.iter().enumerate() {
        let name = format!("__p{}", i);
        params_decl.push(format!("{}: {}", name, rust_type_with_vars(p, &type_var_map)));
        call_args.push(format!("std_clone(&({}))", name));
    }
    let ret = rust_type_with_vars(&ret_type, &type_var_map);
    format!(
        "fn {}{}({}) -> {} {{ {}({}) }}",
        ident(alias),
        generics,
        params_decl.join(", "),
        ret,
        ident(target),
        call_args.join(", ")
    )
}

fn lambda_captures_from_set(
    lambda_items: &[Expression],
    captured_names: &std::collections::HashSet<String>
) -> bool {
    if lambda_items.is_empty() {
        return false;
    }

    fn walk(
        expr: &Expression,
        scope: &mut Vec<String>,
        captured_names: &std::collections::HashSet<String>
    ) -> bool {
        match expr {
            Expression::Word(w) => {
                let w_id = ident(w);
                if w == "nil" || w == "true" || w == "false" {
                    return false;
                }
                if w_id == "_" {
                    return false;
                }
                captured_names.contains(&w_id) && !scope.iter().any(|s| s == &w_id)
            }
            Expression::Apply(items) => {
                if items.is_empty() {
                    return false;
                }
                if let Expression::Word(head) = &items[0] {
                    if head == "as" {
                        // (as value Type) : second argument is type-only metadata.
                        if items.len() >= 2 {
                            return walk(&items[1], scope, captured_names);
                        }
                        return false;
                    }
                    if head == "lambda" {
                        let mut nested_scope = scope.clone();
                        if items.len() >= 2 {
                            for p in &items[1..items.len() - 1] {
                                if let Expression::Word(param) = p {
                                    nested_scope.push(ident(param));
                                }
                            }
                            return walk(&items[items.len() - 1], &mut nested_scope, captured_names);
                        }
                    }
                    if (head == "let" || head == "let*" || head == "let~") && items.len() == 3 {
                        if walk(&items[2], scope, captured_names) {
                            return true;
                        }
                        if let Expression::Word(local_name) = &items[1] {
                            scope.push(ident(local_name));
                        }
                        return false;
                    }
                }
                items.iter().any(|e| walk(e, scope, captured_names))
            }
            _ => false,
        }
    }

    let mut scope = Vec::new();
    if lambda_items.len() >= 2 {
        for p in &lambda_items[1..lambda_items.len() - 1] {
            if let Expression::Word(param) = p {
                scope.push(ident(param));
            }
        }
    }
    walk(&lambda_items[lambda_items.len() - 1], &mut scope, captured_names)
}

fn compile_do(items: &[Expression], node: &TypedExpression, lift_named_fns: bool) -> String {
    if items.len() <= 1 {
        return "0i32".to_string();
    }
    let mut lines = Vec::new();
    let mut prior_top_level_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut prior_function_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for i in 1..items.len() - 1 {
        let sub_node = node.children.get(i);
        if let Expression::Apply(let_items) = &items[i] {
            if !let_items.is_empty() {
                if let Expression::Word(kw) = &let_items[0] {
                    if (kw == "let" || kw == "let*" || kw == "let~") && let_items.len() == 3 {
                        if let Expression::Word(name) = &let_items[1] {
                            let value_node = sub_node.and_then(|n| n.children.get(2)).or(sub_node);
                            if kw == "let*" {
                                if let Some(val_node) = value_node {
                                    if let Expression::Apply(lambda_items) = &let_items[2] {
                                        if
                                            !lambda_items.is_empty() &&
                                            matches!(&lambda_items[0], Expression::Word(h) if h == "lambda")
                                        {
                                            lines.push(
                                                compile_recursive_let_lambda(
                                                    name,
                                                    lambda_items,
                                                    val_node
                                                )
                                            );
                                            continue;
                                        }
                                    }
                                }
                            }
                            let val_expr = sub_node
                                .and_then(|n| n.children.get(2))
                                .map(|n| compile_expr_with_mode(n, false))
                                .unwrap_or_else(|| "()".to_string());
                            if
                                lift_named_fns &&
                                matches!(&let_items[2], Expression::Word(_)) &&
                                value_node.and_then(|n| n.typ.as_ref()).is_some_and(|t|
                                    matches!(t, Type::Function(_, _)) && type_contains_var(t)
                                )
                            {
                                if let Expression::Word(target_word) = &let_items[2] {
                                    lines.push(
                                        compile_function_alias(
                                            name,
                                            target_word,
                                            value_node.and_then(|n| n.typ.as_ref()).unwrap()
                                        )
                                    );
                                    let name_id = ident(name);
                                    prior_function_names.insert(name_id.clone());
                                    prior_top_level_names.insert(name_id);
                                    continue;
                                }
                            }
                            if let Some(val_node) = value_node {
                                if let Expression::Apply(lambda_items) = &let_items[2] {
                                    if !lambda_items.is_empty() {
                                        if let Expression::Word(head) = &lambda_items[0] {
                                            let prior_value_names = prior_top_level_names
                                                .difference(&prior_function_names)
                                                .cloned()
                                                .collect::<std::collections::HashSet<_>>();
                                            let captures_outer = lambda_captures_from_set(
                                                lambda_items,
                                                &prior_value_names
                                            );
                                            if
                                                lift_named_fns &&
                                                head == "lambda" &&
                                                !captures_outer
                                            {
                                                lines.push(
                                                    compile_named_function(
                                                        name,
                                                        lambda_items,
                                                        val_node
                                                    )
                                                );
                                                let name_id = ident(name);
                                                prior_function_names.insert(name_id.clone());
                                                prior_top_level_names.insert(name_id);
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                            lines.push(format!("let mut {} = {};", ident(name), val_expr));
                            prior_function_names.remove(&ident(name));
                            prior_top_level_names.insert(ident(name));
                            continue;
                        }
                    }
                }
            }
        }
        if let Some(n) = sub_node {
            lines.push(format!("let _ = {};", compile_expr_with_mode(n, false)));
        }
    }
    let last = node.children
        .get(items.len() - 1)
        .map(|n| compile_expr_with_mode(n, false))
        .unwrap_or_else(|| "0i32".to_string());
    format!("{{ {} {} }}", lines.join(" "), last)
}

fn compile_lambda(items: &[Expression], node: &TypedExpression) -> String {
    if items.len() < 2 {
        return "| | { 0i32 }".to_string();
    }
    let body_idx = items.len() - 1;
    let body = node.children
        .get(body_idx)
        .map(compile_expr)
        .unwrap_or_else(|| "()".to_string());
    let param_types = node.typ
        .as_ref()
        .map(|t| function_parts(t).0)
        .unwrap_or_default();

    let mut params = Vec::new();
    let empty_vars: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    for (i, expr) in items[1..body_idx].iter().enumerate() {
        if let Expression::Word(name) = expr {
            if let Some(param_ty) = param_types.get(i) {
                match param_ty {
                    // Let function-valued params infer from usage (supports closures and fn pointers).
                    Type::Function(_, _) => params.push(ident(name)),
                    _ => {
                        let ty = if type_contains_var(param_ty) {
                            rust_type_with_infer_vars(param_ty)
                        } else {
                            rust_type_with_vars(param_ty, &empty_vars)
                        };
                        if ty == "_" {
                            // Avoid `|x: _|` in closures.
                            params.push(ident(name));
                        } else {
                            params.push(format!("{}: {}", ident(name), ty));
                        }
                    }
                }
            } else {
                params.push(ident(name));
            }
        }
    }
    format!("|{}| {{ {} }}", params.join(", "), body)
}

fn compile_loop(node: &TypedExpression) -> String {
    let start = node.children
        .get(1)
        .map(compile_expr)
        .unwrap_or_else(|| "0".to_string());
    let end = node.children
        .get(2)
        .map(compile_expr)
        .unwrap_or_else(|| "0".to_string());
    let f = node.children
        .get(3)
        .map(compile_expr)
        .unwrap_or_else(|| "|_i: i32| { 0i32 }".to_string());
    format!(
        "{{ let __f = {}; for __i in ({} as i32)..({} as i32) {{ __f(__i); }} 0i32 }}",
        f,
        start,
        end
    )
}

fn compile_call_parts(children: &[TypedExpression]) -> String {
    fn declared_arity_for_head(head: &TypedExpression) -> Option<usize> {
        if let Expression::Word(w) = &head.expr {
            let key = ident(w);
            return DECL_ARITY.with(|m| m.borrow().get(&key).copied());
        }
        None
    }

    let f = children
        .first()
        .map(compile_expr)
        .unwrap_or_else(|| "()".to_string());
    let mut arg_exprs: Vec<Option<String>> = Vec::new();
    for arg in &children[1..] {
        if let Expression::Word(w) = &arg.expr {
            if w == "_" || w == "__ignored" {
                arg_exprs.push(None);
                continue;
            }
        }
        let expr = compile_expr(arg);
        if expr == "__ignored" {
            arg_exprs.push(None);
            continue;
        }
        let clone_by_type = match arg.typ.as_ref() {
            Some(Type::List(_)) | Some(Type::Tuple(_)) => true,
            Some(Type::Var(_)) => true,
            Some(Type::Function(_, _)) => false,
            Some(Type::Int)
            | Some(Type::Float)
            | Some(Type::Bool)
            | Some(Type::Char)
            | Some(Type::Unit) => false,
            None => false,
        };
        let clone_by_word_fallback =
            matches!(arg.expr, Expression::Word(_)) &&
            matches!(arg.typ.as_ref(), Some(_)) &&
            !matches!(arg.typ.as_ref(), Some(Type::Function(_, _)) | Some(Type::Var(_)));
        if clone_by_type || clone_by_word_fallback {
            arg_exprs.push(Some(format!("std_clone(&({}))", expr)));
        } else {
            arg_exprs.push(Some(expr));
        }
    }

    if arg_exprs.is_empty() {
        return format!("{}()", f);
    }

    // Partial application: explicit holes or fewer supplied args than inferred arity.
    if let Some(f_ty) = children.first().and_then(|n| n.typ.as_ref()) {
        let (param_types, _ret_ty) = function_parts(f_ty);
        let direct_arity_opt = children
            .first()
            .and_then(declared_arity_for_head)
            .or_else(|| if param_types.is_empty() { None } else { Some(param_types.len()) });
        let direct_arity = direct_arity_opt.unwrap_or(0);
        let has_holes = arg_exprs.iter().any(|a| a.is_none());
        if direct_arity > 0 && (has_holes || arg_exprs.len() < direct_arity) {
            let mut binds = Vec::new();
            binds.push(format!("let __fun = {};", f));

            let mut rem_params = Vec::new();
            let mut all_args = Vec::new();
            let mut p_idx = 0usize;
            let mut a_idx = 0usize;

            for (i, ty) in param_types.iter().take(direct_arity).enumerate() {
                if a_idx < arg_exprs.len() {
                    if let Some(expr) = &arg_exprs[a_idx] {
                        let name = format!("__arg{}", a_idx);
                        binds.push(format!("let {} = {};", name, expr));
                        all_args.push(name);
                    } else {
                        let name = format!("__p{}", p_idx);
                        p_idx += 1;
                        rem_params.push(format!("{}: {}", name, rust_type_with_infer_vars(ty)));
                        all_args.push(name);
                    }
                    a_idx += 1;
                } else {
                    let name = format!("__p{}", p_idx);
                    p_idx += 1;
                    rem_params.push(format!("{}: {}", name, rust_type_with_infer_vars(&param_types[i])));
                    all_args.push(name);
                }
            }
            return format!(
                "{{ {} move |{}| {{ __fun({}) }} }}",
                binds.join(" "),
                rem_params.join(", "),
                all_args.join(", ")
            );
        }

        // Over-application: apply first `arity` args normally, then apply the
        // remaining arguments one-by-one to the returned function value.
        if direct_arity > 0 && arg_exprs.len() > direct_arity {
            let head_n = direct_arity;
            let mut binds = Vec::new();
            binds.push(format!("let __fun = {};", f));

            let mut head_names = Vec::new();
            for (i, arg_expr_opt) in arg_exprs.iter().take(head_n).enumerate() {
                let arg_expr = arg_expr_opt
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| "0i32".to_string());
                let name = format!("__arg{}", i);
                binds.push(format!("let {} = {};", name, arg_expr));
                head_names.push(name);
            }
            binds.push(format!("let mut __ap = __fun({});", head_names.join(", ")));

            for (j, arg_expr_opt) in arg_exprs.iter().skip(head_n).enumerate() {
                let arg_expr = arg_expr_opt
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| "0i32".to_string());
                let name = format!("__arg_over{}", j);
                binds.push(format!("let {} = {};", name, arg_expr));
                binds.push(format!("__ap = __ap({});", name));
            }

            return format!("{{ {} __ap }}", binds.join(" "));
        }
    }

    let mut binds = Vec::new();
    let mut names = Vec::new();

    for (i, arg_expr_opt) in arg_exprs.iter().enumerate() {
        let arg_expr = arg_expr_opt
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "0i32".to_string());
        let name = format!("__arg{}", i);
        binds.push(format!("let {} = {};", name, arg_expr));
        names.push(name);
    }

    format!("{{ {} {}({}) }}", binds.join(" "), f, names.join(", "))
}

fn compile_call(node: &TypedExpression) -> String {
    compile_call_parts(&node.children)
}

fn tuple_index_projection(tuple_src: String, idx: usize, arity: usize) -> String {
    let names = (0..arity).map(|n| format!("__t{}", n)).collect::<Vec<_>>();
    format!("{{ let ({}) = {}; {}; }}", names.join(", "), tuple_src, names[idx])
}

fn rust_cast_target_from_as_hint(
    hint_expr: Option<&Expression>,
    hint_type: Option<&Type>
) -> Option<&'static str> {
    if let Some(Expression::Word(w)) = hint_expr {
        return match w.as_str() {
            "Int" => Some("i32"),
            "Float" => Some("f32"),
            "Char" => Some("i32"),
            _ => None,
        };
    }
    match hint_type {
        Some(Type::Int) => Some("i32"),
        Some(Type::Float) => Some("f32"),
        Some(Type::Char) => Some("i32"),
        _ => None,
    }
}

fn compile_expr_with_mode(node: &TypedExpression, lift_named_fns: bool) -> String {
    match &node.expr {
        Expression::Int(n) => format!("{}", n),
        Expression::Float(n) => format!("{}f32", n),
        Expression::Word(w) => {
            if w == "nil" {
                "0i32".to_string()
            } else {
                let id = ident(w);
                match node.typ.as_ref() {
                    Some(Type::List(_)) | Some(Type::Tuple(_)) => format!("({}).clone()", id),
                    _ => id,
                }
            }
        }
        Expression::Apply(items) => {
            if items.is_empty() {
                return "()".to_string();
            }
            match &items[0] {
                Expression::Word(op) =>
                    match op.as_str() {
                        _ if op.starts_with("std/fn/apply/first/") => {
                            if node.children.len() >= 2 {
                                compile_call_parts(&node.children[1..])
                            } else {
                                "0i32".to_string()
                            }
                        }
                        "do" => compile_do(items, node, lift_named_fns),
                        "vector" | "string" | "tuple" => {
                            let args = node.children[1..]
                                .iter()
                                .map(compile_expr)
                                .collect::<Vec<_>>();
                            match node.typ.as_ref() {
                                Some(Type::Tuple(_)) => {
                                    match args.len() {
                                        0 => "()".to_string(),
                                        1 => format!("({},)", args[0]),
                                        _ => format!("({})", args.join(", ")),
                                    }
                                }
                                Some(Type::List(_)) =>
                                    format!(
                                        "std::rc::Rc::new(std::cell::RefCell::new(vec![{}]))",
                                        args.join(", ")
                                    ),
                                _ => format!("vec![{}]", args.join(", ")),
                            }
                        }
                        "length" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(||
                                    "std::rc::Rc::new(std::cell::RefCell::new(Vec::<i32>::new()))".to_string()
                                );
                            format!("{{ let __arr = ({}).clone(); let __r = __arr.borrow(); let __n = __r.len() as i32; __n }}", a)
                        }
                        "get" | "car" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(||
                                    "std::rc::Rc::new(std::cell::RefCell::new(Vec::<i32>::new()))".to_string()
                                );
                            let i = if op == "car" {
                                "0".to_string()
                            } else {
                                node.children
                                    .get(2)
                                    .map(compile_expr)
                                    .unwrap_or_else(|| "0".to_string())
                            };
                            if
                                op == "get" &&
                                matches!(
                                    node.children.get(1).and_then(|n| n.typ.as_ref()),
                                    Some(Type::Tuple(_))
                                )
                            {
                                if
                                    let Some(Type::Tuple(items)) = node.children
                                        .get(1)
                                        .and_then(|n| n.typ.as_ref())
                                {
                                    if
                                        let Some(
                                            TypedExpression { expr: Expression::Int(idx), .. },
                                        ) = node.children.get(2)
                                    {
                                        let idx = *idx as usize;
                                        if idx < items.len() {
                                            return tuple_index_projection(a, idx, items.len());
                                        }
                                    }
                                }
                            }
                            format!(
                                "{{ let __arr = ({}).clone(); let __r = __arr.borrow(); __r[({}) as usize].clone() }}",
                                a,
                                i
                            )
                        }
                        "fst" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            format!("({}).0", a)
                        }
                        "snd" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            format!("({}).1", a)
                        }
                        "cdr" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(||
                                    "std::rc::Rc::new(std::cell::RefCell::new(Vec::<i32>::new()))".to_string()
                                );
                            let i = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "1".to_string());
                            format!(
                                "{{ let __arr = {}; let __r = __arr.borrow(); std::rc::Rc::new(std::cell::RefCell::new(__r[({}) as usize..].to_vec())) }}",
                                a,
                                i
                            )
                        }
                        "set!" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(||
                                    "std::rc::Rc::new(std::cell::RefCell::new(Vec::<i32>::new()))".to_string()
                                );
                            let i = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let v = node.children
                                .get(3)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!(
                                "{{ let __arr_rc = ({}).clone(); let __idx = ({}) as usize; let __val = {}; let mut __arr = __arr_rc.borrow_mut(); if __idx == __arr.len() {{ __arr.push(__val); }} else {{ __arr[__idx] = __val; }} 0i32 }}",
                                a,
                                i,
                                v
                            )
                        }
                        "pop!" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(||
                                    "std::rc::Rc::new(std::cell::RefCell::new(Vec::<i32>::new()))".to_string()
                                );
                            format!("{{ ({}).borrow_mut().pop(); 0i32 }}", a)
                        }
                        "if" => {
                            let c = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            let t = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            let e = node.children
                                .get(3)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            format!("(if {} {{ {} }} else {{ {} }})", c, t, e)
                        }
                        "loop" => compile_loop(node),
                        "loop-finish" => {
                            let cond = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            let fun = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "|| { 0i32 }".to_string());
                            format!("{{ let __f = {}; while {} {{ __f(); }} 0i32 }}", fun, cond)
                        }
                        "lambda" => compile_lambda(items, node),
                        "let" | "let*" | "let~" => {
                            if items.len() == 3 {
                                let name = if let Expression::Word(n) = &items[1] {
                                    ident(n)
                                } else {
                                    "_tmp".to_string()
                                };
                                let v = node.children
                                    .get(2)
                                    .map(compile_expr)
                                    .unwrap_or_else(|| "0i32".to_string());
                                format!("{{ let mut {} = {}; 0i32 }}", name, v)
                            } else {
                                "0i32".to_string()
                            }
                        }
                        "as" => {
                            let v = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            let cast_target = rust_cast_target_from_as_hint(
                                items.get(2),
                                node.typ.as_ref()
                            );
                            match cast_target {
                                Some(target) => format!("({}) as {}", v, target),
                                None => v,
                            }
                        }
                        "char" | "Int->Float" => {
                            if op == "char" {
                                node.children
                                    .get(1)
                                    .map(compile_expr)
                                    .unwrap_or_else(|| "0i32".to_string())
                            } else {
                                let a = node.children
                                    .get(1)
                                    .map(compile_expr)
                                    .unwrap_or_else(|| "0".to_string());
                                format!("({}) as f32", a)
                            }
                        }
                        "Float->Int" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0f32".to_string());
                            format!("({}) as i32", a)
                        }
                        "not" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            format!("(!{})", a)
                        }
                        "and" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            format!("({} && {})", a, b)
                        }
                        "or" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            format!("({} || {})", a, b)
                        }
                        "+" | "+#" | "+." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} + {})", a, b)
                        }
                        "-" | "-#" | "-." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} - {})", a, b)
                        }
                        "*" | "*#" | "*." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} * {})", a, b)
                        }
                        "/" | "/#" | "/." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "1".to_string());
                            format!("({} / {})", a, b)
                        }
                        "mod" | "mod." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "1".to_string());
                            format!("({} % {})", a, b)
                        }
                        "=" | "=?" | "=#" | "=." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} == {})", a, b)
                        }
                        "<" | "<#" | "<." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} < {})", a, b)
                        }
                        ">" | ">#" | ">." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} > {})", a, b)
                        }
                        "<=" | "<=#" | "<=." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} <= {})", a, b)
                        }
                        ">=" | ">=#" | ">=." => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} >= {})", a, b)
                        }
                        "^" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} ^ {})", a, b)
                        }
                        "|" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} | {})", a, b)
                        }
                        "&" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} & {})", a, b)
                        }
                        "~" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("(!{})", a)
                        }
                        "<<" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} << {})", a, b)
                        }
                        ">>" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let b = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("({} >> {})", a, b)
                        }
                        _ => compile_call(node),
                    }
                _ => compile_call(node),
            }
        }
    }
}

pub fn compile_expr(node: &TypedExpression) -> String {
    compile_expr_with_mode(node, false)
}

pub fn compile_program_to_rust_typed(typed_ast: &TypedExpression) -> String {
    fn collect_decl_arity(node: &TypedExpression, out: &mut HashMap<String, usize>) {
        if let Expression::Apply(items) = &node.expr {
            if let Some(Expression::Word(op)) = items.first() {
                if (op == "let" || op == "let*" || op == "let~") && items.len() == 3 {
                    if let Expression::Word(name) = &items[1] {
                        if let Expression::Apply(lambda_items) = &items[2] {
                            if let Some(Expression::Word(h)) = lambda_items.first() {
                                if h == "lambda" {
                                    let arity = lambda_items.len().saturating_sub(2);
                                    out.insert(ident(name), arity);
                                }
                            }
                        }
                    }
                }
            }
        }
        for c in &node.children {
            collect_decl_arity(c, out);
        }
    }

    let mut arities = HashMap::new();
    collect_decl_arity(typed_ast, &mut arities);
    DECL_ARITY.with(|m| *m.borrow_mut() = arities);

    // Lift only non-capturing lambdas to Rust `fn` items; capturing lambdas stay closures.
    let body = compile_expr_with_mode(typed_ast, true);
    let int_to_float_name = ident("Int->Float");
    let float_to_int_name = ident("Float->Int");
    let helpers = r#"
fn v_plus(a: i32, b: i32) -> i32 { a + b }
fn v_plus_char(a: i32, b: i32) -> i32 { a + b }
fn v_plus_float(a: f32, b: f32) -> f32 { a + b }
fn v_minus(a: i32, b: i32) -> i32 { a - b }
fn v_minus_char(a: i32, b: i32) -> i32 { a - b }
fn v_minus_float(a: f32, b: f32) -> f32 { a - b }
fn v_mul(a: i32, b: i32) -> i32 { a * b }
fn v_mul_char(a: i32, b: i32) -> i32 { a * b }
fn v_mul_float(a: f32, b: f32) -> f32 { a * b }
fn v_div(a: i32, b: i32) -> i32 { a / b }
fn v_div_char(a: i32, b: i32) -> i32 { a / b }
fn v_div_float(a: f32, b: f32) -> f32 { a / b }
fn v_mod(a: i32, b: i32) -> i32 { a % b }
fn v_mod_float(a: f32, b: f32) -> f32 { a % b }
fn v_eq(a: i32, b: i32) -> bool { a == b }
fn v_eq_bool(a: bool, b: bool) -> bool { a == b }
fn v_eq_char(a: i32, b: i32) -> bool { a == b }
fn v_eq_float(a: f32, b: f32) -> bool { a == b }
fn v_lt(a: i32, b: i32) -> bool { a < b }
fn v_lt_char(a: i32, b: i32) -> bool { a < b }
fn v_lt_float(a: f32, b: f32) -> bool { a < b }
fn v_gt(a: i32, b: i32) -> bool { a > b }
fn v_gt_char(a: i32, b: i32) -> bool { a > b }
fn v_gt_float(a: f32, b: f32) -> bool { a > b }
fn v_lte(a: i32, b: i32) -> bool { a <= b }
fn v_lte_char(a: i32, b: i32) -> bool { a <= b }
fn v_lte_float(a: f32, b: f32) -> bool { a <= b }
fn v_gte(a: i32, b: i32) -> bool { a >= b }
fn v_gte_char(a: i32, b: i32) -> bool { a >= b }
fn v_gte_float(a: f32, b: f32) -> bool { a >= b }
fn v_not(a: bool) -> bool { !a }
fn v_and(a: bool, b: bool) -> bool { a && b }
fn v_or(a: bool, b: bool) -> bool { a || b }
fn v_xor(a: i32, b: i32) -> i32 { a ^ b }
fn v_bor(a: i32, b: i32) -> i32 { a | b }
fn v_band(a: i32, b: i32) -> i32 { a & b }
fn v_bnot(a: i32) -> i32 { !a }
fn v_shl(a: i32, b: i32) -> i32 { a << b }
fn v_shr(a: i32, b: i32) -> i32 { a >> b }
"#;
    format!(
        "#![allow(warnings)]\n\nfn std_clone<T: Clone>(x: &T) -> T {{ x.clone() }}\n\nfn normalize_ws(s: &str) -> String {{\n    let mut out = String::new();\n    let mut prev_space = false;\n    for ch in s.chars() {{\n        if ch.is_whitespace() {{\n            if !prev_space {{\n                out.push(' ');\n                prev_space = true;\n            }}\n        }} else {{\n            out.push(ch);\n            prev_space = false;\n        }}\n    }}\n    out.trim().to_string()\n}}\n\nfn pretty_result<T: std::fmt::Debug>(value: &T) -> String {{\n    let mut s = format!(\"{{:?}}\", value);\n    s = s.replace(\"RefCell {{ value: \", \"\");\n    s = s.replace(\"}}\", \"\");\n    s = s.replace(',', \"\");\n    s = s.replace('(', \"[\");\n    s = s.replace(')', \"]\");\n    s = normalize_ws(&s);\n    s = s.replace(\"[ \", \"[\");\n    s = s.replace(\" ]\", \"]\");\n    s\n}}\n{}\n#[allow(non_snake_case)]\nfn {}(x: i32) -> f32 {{ x as f32 }}\n\n#[allow(non_snake_case)]\nfn {}(x: f32) -> i32 {{ x as i32 }}\n\nfn main() {{\n    let result = {};\n    println!(\"{{}}\", pretty_result(&result));\n}}\n",
        helpers,
        int_to_float_name,
        float_to_int_name,
        body
    )
}

pub fn compile_program_to_rust(expr: &Expression) -> Result<String, String> {
    let (_typ, typed_ast) = crate::infer::infer_with_builtins_typed(
        expr,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
    )?;
    Ok(compile_program_to_rust_typed(&typed_ast))
}
