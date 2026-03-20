use std::collections::HashSet;
use que::infer::TypedExpression;
use que::parser::Expression;
use que::types::Type;

fn ident(name: &str) -> String {
    let originally_upper = name
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false);
    let name = name.replace("->", "_to_");
    let mut s = String::new();
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
    for c in name.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => s.push(c.to_ascii_lowercase()),
            ':' | '-' | '*' | '/' | '?' | '!' | '.' | '+' | '<' | '>' | '=' | '|' | '&' | '^' => {
                push_encoded(&mut s, c);
            }
            _ => push_encoded(&mut s, c),
        }
    }
    if s.is_empty() {
        s = "_".to_string();
    }
    if s
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        s = format!("_{}", s);
    }
    if originally_upper {
        s = format!("u_{}", s);
    }
    let keywords = [
        "and", "as", "assert", "begin", "class", "constraint", "do", "done", "downto",
        "else", "end", "exception", "external", "false", "for", "fun", "function",
        "functor", "if", "in", "include", "inherit", "initializer", "lazy", "let",
        "match", "method", "module", "mutable", "new", "object", "of", "open", "or",
        "private", "rec", "sig", "struct", "then", "to", "true", "try", "type", "val",
        "virtual", "when", "while", "with",
    ];
    if keywords.contains(&s.as_str()) {
        s = format!("{}_", s);
    }
    if s.starts_with("__") {
        return s;
    }
    format!("v_{}", s)
}

fn op_call(name: &str) -> Option<&'static str> {
    match name {
        "+" | "+#" => Some("+"),
        "+." => Some("+."),
        "-" | "-#" => Some("-"),
        "-." => Some("-."),
        "*" | "*#" => Some("*"),
        "*." => Some("*."),
        "/" | "/#" => Some("/"),
        "/." => Some("/."),
        "mod" => Some("mod"),
        "mod." => Some("mod_float"),
        "=" | "=?" | "=#" | "=." => Some("="),
        "<" | "<#" | "<." => Some("<"),
        ">" | ">#" | ">." => Some(">"),
        "<=" | "<=#" | "<=." => Some("<="),
        ">=" | ">=#" | ">=." => Some(">="),
        "^" => Some("lxor"),
        "|" => Some("lor"),
        "&" => Some("land"),
        "and" => Some("&&"),
        "or" => Some("||"),
        "not" => Some("not"),
        _ => None,
    }
}

fn is_int_arith_op(name: &str) -> bool {
    matches!(name, "+" | "+#" | "-" | "-#" | "*" | "*#" | "/" | "/#" | "mod")
}

fn compile_call(children: &[TypedExpression], mut_vars: &HashSet<String>) -> String {
    if children.is_empty() {
        return "()".to_string();
    }
    let mut out = compile_expr_inner(&children[0], mut_vars);
    if children.len() == 1 {
        if let Some(Type::Function(param, _ret)) = children[0].typ.as_ref() {
            if matches!(**param, Type::Unit) {
                return format!("({} ())", out);
            }
        }
    }
    for arg in &children[1..] {
        let arg_src = compile_expr_inner(arg, mut_vars);
        let needs_wrap = {
            let t = arg_src.trim();
            t.starts_with('-') && t.len() > 1 && t[1..].chars().all(|c| c.is_ascii_digit() || c == '.')
        };
        let arg_src = if needs_wrap { format!("({})", arg_src) } else { arg_src };
        out = format!("({} {})", out, arg_src);
    }
    out
}

fn wrap_call_arg(arg_src: String) -> String {
    let t = arg_src.trim();
    let needs_wrap =
        t.starts_with('-') && t.len() > 1 && t[1..].chars().all(|c| c.is_ascii_digit() || c == '.');
    if needs_wrap {
        format!("({})", arg_src)
    } else {
        arg_src
    }
}

fn compile_do(items: &[Expression], children: &[TypedExpression], mut_vars: &HashSet<String>) -> String {
    if items.len() <= 1 {
        return "()".to_string();
    }
    let mut bindings = Vec::new();
    let mut scoped_mut_vars = mut_vars.clone();
    for i in 1..items.len() - 1 {
        if let Expression::Apply(let_items) = &items[i] {
            if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                if kw == "let" || kw == "letrec" || kw == "mut" {
                    let val = children
                        .get(i)
                        .and_then(|n| n.children.get(2))
                        .map(|n| compile_expr_inner(n, &scoped_mut_vars))
                        .unwrap_or_else(|| "()".to_string());
                    let ident_name = ident(name);
                    if kw == "mut" {
                        bindings.push(format!("let {} = ref {}", ident_name, wrap_call_arg(val)));
                        scoped_mut_vars.insert(name.clone());
                    } else {
                        let binder = if kw == "letrec" { "let rec" } else { "let" };
                        bindings.push(format!("{} {} = {}", binder, ident_name, val));
                    }
                    continue;
                }
            }
        }
        if let Some(n) = children.get(i) {
            bindings.push(format!("let __unused{} = {}", i, compile_expr_inner(n, &scoped_mut_vars)));
        }
    }
    let last = children
        .get(items.len() - 1)
        .map(|n| compile_expr_inner(n, &scoped_mut_vars))
        .unwrap_or_else(|| "()".to_string());
    if bindings.is_empty() {
        format!("({})", last)
    } else {
        format!("({} in {})", bindings.join(" in "), last)
    }
}

fn compile_expr_inner(node: &TypedExpression, mut_vars: &HashSet<String>) -> String {
    match &node.expr {
        Expression::Int(n) => format!("{}", n),
        Expression::Dec(n) => format!("{:?}", n),
        Expression::Word(w) => match w.as_str() {
            "nil" => "0".to_string(),
            "true" => "true".to_string(),
            "false" => "false".to_string(),
            "fst" => "fst".to_string(),
            "snd" => "snd".to_string(),
            _ => {
                let rendered = ident(w);
                if mut_vars.contains(w) {
                    format!("(!{})", rendered)
                } else {
                    rendered
                }
            }
        },
        Expression::Apply(items) => {
            if items.is_empty() {
                return "()".to_string();
            }
            match &items[0] {
                Expression::Word(op) => match op.as_str() {
                    "do" => compile_do(items, &node.children, mut_vars),
                    "vector" | "string" => {
                        let args = node.children[1..]
                            .iter()
                            .map(|n| compile_expr_inner(n, mut_vars))
                            .collect::<Vec<_>>()
                            .join("; ");
                        format!("(vec_of_array [|{}|])", args)
                    }
                    "tuple" => {
                        let args = node.children[1..]
                            .iter()
                            .map(|n| compile_expr_inner(n, mut_vars))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("({})", args)
                    }
                    "cons" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "(vec_of_array [||])".to_string());
                        let b = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "(vec_of_array [||])".to_string());
                        format!("(vec_cons {} {})", a, b)
                    }
                    "length" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "(vec_of_array [||])".to_string());
                        format!("(vec_length {})", a)
                    }
                    "get" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "(vec_of_array [||])".to_string());
                        let i = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        if let Some(Type::Tuple(items)) = node.children.get(1).and_then(|n| n.typ.as_ref()) {
                            if let Some(TypedExpression { expr: Expression::Int(idx), .. }) = node.children.get(2) {
                                let idx = *idx as usize;
                                if idx < items.len() {
                                    let names = (0..items.len()).map(|n| format!("__t{}", n)).collect::<Vec<_>>();
                                    return format!("(let ({}) = {} in {})", names.join(", "), a, names[idx]);
                                }
                            }
                        }
                        format!("(vec_get {} {})", a, i)
                    }
                    "car" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "(vec_of_array [||])".to_string());
                        format!("(vec_get {} 0)", a)
                    }
                    "fst" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "()".to_string());
                        format!("(fst {})", a)
                    }
                    "snd" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "()".to_string());
                        format!("(snd {})", a)
                    }
                    "cdr" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "(vec_of_array [||])".to_string());
                        let i = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "1".to_string());
                        format!("(vec_rest {} {})", a, i)
                    }
                    "set!" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "(vec_of_array [||])".to_string());
                        let i = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        let v = node.children.get(3).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        format!("(vec_set {} {} {}; 0)", a, i, v)
                    }
                    "pop!" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "(vec_of_array [||])".to_string());
                        format!("(vec_pop {}; 0)", a)
                    }
                    "alter!" => {
                        let name = match items.get(1) {
                            Some(Expression::Word(n)) => ident(n),
                            _ => "_tmp".to_string(),
                        };
                        let value = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        format!("({} := {}; 0)", name, value)
                    }
                    "if" => {
                        let c = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "false".to_string());
                        let mut t = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "()".to_string());
                        let mut e = node.children.get(3).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "()".to_string());
                        let t_ty = node.children.get(2).and_then(|n| n.typ.as_ref());
                        let e_ty = node.children.get(3).and_then(|n| n.typ.as_ref());
                        if let (Some(Type::Function(t_arg, t_ret)), Some(e_t)) = (t_ty, e_ty) {
                            if matches!(**t_arg, Type::Unit) && **t_ret == *e_t {
                                t = format!("({} ())", t);
                            }
                        }
                        if let (Some(t_t), Some(Type::Function(e_arg, e_ret))) = (t_ty, e_ty) {
                            if matches!(**e_arg, Type::Unit) && **e_ret == *t_t {
                                e = format!("({} ())", e);
                            }
                        }
                        format!("(if {} then {} else {})", c, t, e)
                    }
                    "while" => {
                        let c = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "false".to_string());
                        let b = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "()".to_string());
                        format!("(while {} do ignore ({}) done; 0)", c, b)
                    }
                    "lambda" => {
                        let body_idx = items.len() - 1;
                        let mut lambda_mut_vars = mut_vars.clone();
                        let params = items[1..body_idx]
                            .iter()
                            .filter_map(|p| {
                                if let Expression::Word(w) = p {
                                    lambda_mut_vars.remove(w);
                                    Some(ident(w))
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();
                        let body = node.children.get(body_idx).map(|n| compile_expr_inner(n, &lambda_mut_vars)).unwrap_or_else(|| "()".to_string());
                        if params.is_empty() {
                            format!("(fun () -> {})", body)
                        } else {
                            format!("(fun {} -> {})", params.join(" "), body)
                        }
                    }
                    "let" | "letrec" | "mut" => {
                        if items.len() == 3 {
                            let name = if let Expression::Word(n) = &items[1] { ident(n) } else { "_tmp".to_string() };
                            let value = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "()".to_string());
                            if op == "mut" {
                                format!("(let {} = ref {} in 0)", name, wrap_call_arg(value))
                            } else {
                                let binder = if op == "letrec" { "let rec" } else { "let" };
                                format!("({} {} = {} in 0)", binder, name, value)
                            }
                        } else {
                            "0".to_string()
                        }
                    }
                    "as" | "char" => node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "()".to_string()),
                    "Int->Dec" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        format!("(float_of_int {})", wrap_call_arg(a))
                    }
                    "Dec->Int" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0.0".to_string());
                        format!("(int_of_float {})", wrap_call_arg(a))
                    }
                    "~" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        format!("(lnot {})", wrap_call_arg(a))
                    }
                    "<<" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        let b = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        format!("({} lsl {})", a, b)
                    }
                    ">>" => {
                        let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        let b = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                        format!("({} asr {})", a, b)
                    }
                    _ => {
                        if let Some(opf) = op_call(op) {
                            if node.children.len() == 2 {
                                let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                                format!("({} {})", opf, a)
                            } else if node.children.len() >= 3 {
                                let a = node.children.get(1).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                                let b = node.children.get(2).map(|n| compile_expr_inner(n, mut_vars)).unwrap_or_else(|| "0".to_string());
                                if opf == "mod_float" {
                                    format!("({} {} {})", opf, a, b)
                                } else if is_int_arith_op(op) {
                                    format!("((auto_int ({})) {} (auto_int ({})))", a, opf, b)
                                } else {
                                    format!("({} {} {})", a, opf, b)
                                }
                            } else {
                                "()".to_string()
                            }
                        } else {
                            compile_call(&node.children, mut_vars)
                        }
                    }
                },
                _ => compile_call(&node.children, mut_vars),
            }
        }
    }
}

pub fn compile_expr(node: &TypedExpression) -> String {
    compile_expr_inner(node, &HashSet::new())
}

const OCAML_PRELUDE: &str =
r#"[@@@warning "-26-27"]

type 'a vec = { mutable data: 'a array }

let vec_of_array (a: 'a array) : 'a vec = { data = a }

let vec_length (v: 'a vec) : int = Array.length v.data

let vec_get (v: 'a vec) (i: int) : 'a = v.data.(i)

let vec_rest (v: 'a vec) (i: int) : 'a vec =
  let a = v.data in
  let n = Array.length a in
  if i <= 0 then vec_of_array a
  else if i >= n then vec_of_array [||]
  else vec_of_array (Array.sub a i (n - i))

let vec_cons (a: 'a vec) (b: 'a vec) : 'a vec =
  vec_of_array (Array.append a.data b.data)

let vec_set (v: 'a vec) (i: int) (x: 'a) : unit =
  let a = v.data in
  let n = Array.length a in
  if i = n then v.data <- Array.append a [|x|]
  else if i >= 0 && i < n then a.(i) <- x
  else failwith "set!: index out of bounds"

let vec_pop (v: 'a vec) : unit =
  let a = v.data in
  let n = Array.length a in
  if n > 0 then v.data <- Array.sub a 0 (n - 1)

let auto_int (x : int) : int = x

let rec show_any (x : Obj.t) : string =
  match Obj.tag x with
  | tag when tag = Obj.int_tag -> string_of_int (Obj.obj x : int)
  | tag when tag = Obj.string_tag -> Printf.sprintf "%S" (Obj.obj x : string)
  | tag when tag = Obj.double_tag -> string_of_float (Obj.obj x : float)
  | tag when tag = Obj.closure_tag -> "<function>"
  | _ when Obj.is_block x ->
      let n = Obj.size x in
      if n = 1 && Obj.is_block (Obj.field x 0) then
        let arr = Obj.field x 0 in
        let m = Obj.size arr in
        let elems = List.init m (fun i -> show_any (Obj.field arr i)) in
        Printf.sprintf "[%s]" (String.concat " " elems)
      else
        let elems = List.init n (fun i -> show_any (Obj.field x i)) in
        Printf.sprintf "[%s]" (String.concat " " elems)
  | _ -> "<unprintable>"

let log_last expr =
  Printf.printf "%s\n" (show_any (Obj.repr expr));
  expr

let log_last_with show expr =
  Printf.printf "%s\n" (show expr);
  expr
"#;

fn show_expr_for_type(ty: &Type, var: &str) -> String {
    match ty {
        Type::Int => format!("(string_of_int {})", var),
        Type::Dec => format!("(string_of_float {})", var),
        Type::Bool => format!("(if {} then \"true\" else \"false\")", var),
        Type::Char => format!("(string_of_int {})", var),
        Type::Unit => "\"()\"".to_string(),
        Type::Function(_, _) => "\"<function>\"".to_string(),
        Type::Var(_) => format!("(show_any (Obj.repr {}))", var),
        Type::List(inner) => {
            let inner_var = "__x";
            let inner_show = show_expr_for_type(inner, inner_var);
            format!(
                "(\"[\" ^ String.concat \" \" (List.map (fun {} -> {}) (Array.to_list {}.data)) ^ \"]\")",
                inner_var,
                inner_show,
                var
            )
        }
        Type::Tuple(items) => {
            let names = (0..items.len()).map(|i| format!("__t{}", i)).collect::<Vec<_>>();
            let rendered = items
                .iter()
                .enumerate()
                .map(|(i, t)| show_expr_for_type(t, &names[i]))
                .collect::<Vec<_>>();
            format!(
                "(let ({}) = {} in \"[\" ^ String.concat \" \" [{}] ^ \"]\")",
                names.join(", "),
                var,
                rendered.join("; ")
            )
        }
    }
}

fn show_fn_for_type(t: Option<&Type>) -> String {
    match t {
        Some(typ) => {
            let body = show_expr_for_type(typ, "v");
            format!("(fun v -> {})", body)
        }
        None => "(fun v -> show_any (Obj.repr v))".to_string(),
    }
}

pub fn compile_program_to_ocaml_typed(typed_ast: &TypedExpression) -> String {
    let body = compile_expr(typed_ast);
    let show_fn = show_fn_for_type(typed_ast.typ.as_ref());
    format!(
        "{}\n\nlet result = log_last_with {} ({})\n",
        OCAML_PRELUDE,
        show_fn,
        body
    )
}

pub fn compile_program_to_ocaml(expr: &Expression) -> Result<String, String> {
    let (_typ, typed_ast) = que::infer::infer_with_builtins_typed(
        expr,
        que::types::create_builtin_environment(que::types::TypeEnv::new())
    )?;
    Ok(compile_program_to_ocaml_typed(&typed_ast))
}
