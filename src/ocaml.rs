use crate::infer::TypedExpression;
use crate::parser::Expression;
use crate::types::Type;

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
    if
        s
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
        "and",
        "as",
        "assert",
        "begin",
        "class",
        "constraint",
        "do",
        "done",
        "downto",
        "else",
        "end",
        "exception",
        "external",
        "false",
        "for",
        "fun",
        "function",
        "functor",
        "if",
        "in",
        "include",
        "inherit",
        "initializer",
        "lazy",
        "let",
        "match",
        "method",
        "module",
        "mutable",
        "new",
        "object",
        "of",
        "open",
        "or",
        "private",
        "rec",
        "sig",
        "struct",
        "then",
        "to",
        "true",
        "try",
        "type",
        "val",
        "virtual",
        "when",
        "while",
        "with",
    ];
    if keywords.contains(&s.as_str()) {
        s = format!("{}_", s);
    }

    // Keep generated temporaries readable and stable.
    if s.starts_with("__") {
        return s;
    }

    // Namespace translated language identifiers so they cannot collide with
    // OCaml/prelude names such as `int`.
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
        "and" | "and_" => Some("&&"),
        "or" | "or_" => Some("||"),
        "not" => Some("not"),
        _ => None,
    }
}

fn is_int_arith_op(name: &str) -> bool {
    matches!(name, "+" | "+#" | "-" | "-#" | "*" | "*#" | "/" | "/#" | "mod")
}

fn compile_call(children: &[TypedExpression]) -> String {
    if children.is_empty() {
        return "()".to_string();
    }
    let mut out = compile_expr(&children[0]);
    if children.len() == 1 {
        if let Some(Type::Function(param, _ret)) = children[0].typ.as_ref() {
            if matches!(**param, Type::Unit) {
                return format!("({} ())", out);
            }
        }
    }
    for arg in &children[1..] {
        let arg_src = compile_expr(arg);
        let needs_wrap = {
            let t = arg_src.trim();
            // In OCaml, `f -1` is parsed as subtraction, not application.
            // Emit `f (-1)` for negative numeric literals.
            t.starts_with('-') &&
                t.len() > 1 &&
                t[1..].chars().all(|c| (c.is_ascii_digit() || c == '.'))
        };
        let arg_src = if needs_wrap { format!("({})", arg_src) } else { arg_src };
        out = format!("({} {})", out, arg_src);
    }
    out
}

fn compile_do(items: &[Expression], children: &[TypedExpression]) -> String {
    if items.len() <= 1 {
        return "()".to_string();
    }
    let mut bindings = Vec::new();
    for i in 1..items.len() - 1 {
        if let Expression::Apply(let_items) = &items[i] {
            if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                if kw == "let" || kw == "let~" || kw == "let*" {
                    let val = children
                        .get(i)
                        .and_then(|n| n.children.get(2))
                        .map(compile_expr)
                        .unwrap_or_else(|| "()".to_string());
                    let binder = if kw == "let*" { "let rec" } else { "let" };
                    if let Expression::Apply(lambda_items) = &let_items[2] {
                        if let Some(Expression::Word(h)) = lambda_items.first() {
                            if h == "lambda" {
                                bindings.push(format!("{} {} = {}", binder, ident(name), val));
                                continue;
                            }
                        }
                    }
                    bindings.push(format!("{} {} = {}", binder, ident(name), val));
                    continue;
                }
            }
        }
        if let Some(n) = children.get(i) {
            bindings.push(format!("let __unused{} = {}", i, compile_expr(n)));
        }
    }
    let last = children
        .get(items.len() - 1)
        .map(compile_expr)
        .unwrap_or_else(|| "()".to_string());
    if bindings.is_empty() {
        format!("({})", last)
    } else {
        format!("({} in {})", bindings.join(" in "), last)
    }
}

pub fn compile_expr(node: &TypedExpression) -> String {
    match &node.expr {
        Expression::Int(n) => format!("{}", n),
        // Keep explicit float shape (e.g. 4.0) so OCaml uses float operators correctly.
        Expression::Float(n) => format!("{:?}", n),
        Expression::Word(w) => {
            match w.as_str() {
                "true" => "true".to_string(),
                "false" => "false".to_string(),
                "fst" => "fst".to_string(),
                "snd" => "snd".to_string(),
                _ => ident(w),
            }
        }
        Expression::Apply(items) => {
            if items.is_empty() {
                return "()".to_string();
            }
            match &items[0] {
                Expression::Word(op) => {
                    match op.as_str() {
                        "do" => compile_do(items, &node.children),
                        "vector" => {
                            let elems = &node.children[1..];
                            let args = elems
                                .iter()
                                .map(compile_expr)
                                .collect::<Vec<_>>()
                                .join("; ");
                            format!("(ref [|{}|])", args)
                        }
                        "string" => {
                            let args = node.children[1..]
                                .iter()
                                .map(compile_expr)
                                .collect::<Vec<_>>()
                                .join("; ");
                            format!("(ref [|{}|])", args)
                        }
                        "tuple" => {
                            let args = node.children[1..]
                                .iter()
                                .map(compile_expr)
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("({})", args)
                        }
                        "length" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "(ref [||])".to_string());
                            format!("(vec_length {})", a)
                        }
                        "get" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "(ref [||])".to_string());
                            let i = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            if
                                let Some(Type::Tuple(items)) = node.children
                                    .get(1)
                                    .and_then(|n| n.typ.as_ref())
                            {
                                if
                                    let Some(TypedExpression { expr: Expression::Int(idx), .. }) =
                                        node.children.get(2)
                                {
                                    let idx = *idx as usize;
                                    if idx < items.len() {
                                        let names = (0..items.len())
                                            .map(|n| format!("__t{}", n))
                                            .collect::<Vec<_>>();
                                        return format!(
                                            "(let ({}) = {} in {})",
                                            names.join(", "),
                                            a,
                                            names[idx]
                                        );
                                    }
                                }
                            }
                            format!("(vec_get {} {})", a, i)
                        }
                        "car" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "(ref [||])".to_string());
                            format!("(vec_get {} 0)", a)
                        }
                        "fst" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            format!("(fst {})", a)
                        }
                        "snd" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            format!("(snd {})", a)
                        }
                        "cdr" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "(ref [||])".to_string());
                            let i = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "1".to_string());
                            format!("(vec_rest {} {})", a, i)
                        }
                        "set!" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "(ref [||])".to_string());
                            let i = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            let v = node.children
                                .get(3)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("(vec_set {} {} {}; 0)", a, i, v)
                        }
                        "pop!" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "(ref [||])".to_string());
                            format!("(vec_pop {}; 0)", a)
                        }
                        "if" => {
                            let c = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            let mut t = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            let mut e = node.children
                                .get(3)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());

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
                        "loop" => {
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
                                .unwrap_or_else(|| "(fun _ -> ())".to_string());
                            format!(
                                "(for __i = {} to ({} - 1) do ignore (({}) __i) done; 0)",
                                start,
                                end,
                                f
                            )
                        }
                        "loop-finish" => {
                            let c = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "false".to_string());
                            let f = node.children
                                .get(2)
                                .map(compile_expr)
                                .unwrap_or_else(|| "(fun () -> ())".to_string());
                            format!("(while {} do ignore (({}) ()) done; 0)", c, f)
                        }
                        "lambda" => {
                            let body_idx = items.len() - 1;
                            let params = items[1..body_idx]
                                .iter()
                                .filter_map(|p| {
                                    if let Expression::Word(w) = p { Some(ident(w)) } else { None }
                                })
                                .collect::<Vec<_>>();
                            let body = node.children
                                .get(body_idx)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string());
                            if params.is_empty() {
                                format!("(fun () -> {})", body)
                            } else {
                                format!("(fun {} -> {})", params.join(" "), body)
                            }
                        }
                        "let" | "let~" | "let*" => {
                            if items.len() == 3 {
                                let name = if let Expression::Word(n) = &items[1] {
                                    ident(n)
                                } else {
                                    "_tmp".to_string()
                                };
                                let value = node.children
                                    .get(2)
                                    .map(compile_expr)
                                    .unwrap_or_else(|| "()".to_string());
                                let binder = if op == "let*" { "let rec" } else { "let" };
                                format!("({} {} = {} in 0)", binder, name, value)
                            } else {
                                "0".to_string()
                            }
                        }
                        "as" | "char" => {
                            node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "()".to_string())
                        }
                        "Int->Float" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("(float_of_int {})", a)
                        }
                        "Float->Int" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0.0".to_string());
                            format!("(int_of_float {})", a)
                        }
                        "~" => {
                            let a = node.children
                                .get(1)
                                .map(compile_expr)
                                .unwrap_or_else(|| "0".to_string());
                            format!("(lnot {})", a)
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
                            format!("({} lsl {})", a, b)
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
                            format!("({} asr {})", a, b)
                        }
                        _ => {
                            if let Some(opf) = op_call(op) {
                                if node.children.len() == 2 {
                                    let a = node.children
                                        .get(1)
                                        .map(compile_expr)
                                        .unwrap_or_else(|| "0".to_string());
                                    format!("({} {})", opf, a)
                                } else if node.children.len() >= 3 {
                                    let a = node.children
                                        .get(1)
                                        .map(compile_expr)
                                        .unwrap_or_else(|| "0".to_string());
                                    let b = node.children
                                        .get(2)
                                        .map(compile_expr)
                                        .unwrap_or_else(|| "0".to_string());
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
                                compile_call(&node.children)
                            }
                        }
                    }
                }
                _ => compile_call(&node.children),
            }
        }
    }
}

const OCAML_PRELUDE: &str =
    r#"[@@@warning "-26-27"]

type 'a vec = 'a array ref

let vec_length (v: 'a vec) : int = Array.length !v

let vec_get (v: 'a vec) (i: int) : 'a = (!v).(i)

let vec_rest (v: 'a vec) (i: int) : 'a vec =
  let a = !v in
  let n = Array.length a in
  if i <= 0 then ref a
  else if i >= n then ref [||]
  else ref (Array.sub a i (n - i))

let vec_set (v: 'a vec) (i: int) (x: 'a) : unit =
  let a = !v in
  let n = Array.length a in
  if i = n then v := Array.append a [|x|]
  else if i >= 0 && i < n then a.(i) <- x
  else failwith "set!: index out of bounds"

let vec_pop (v: 'a vec) : unit =
  let a = !v in
  let n = Array.length a in
  if n > 0 then v := Array.sub a 0 (n - 1)

let auto_int (x : int) : int = x

let v__gt_ a b = a > b
let v__lt_ a b = a < b
let v__eq_ a b = a = b
let v__gt__eq_ a b = a >= b
let v__lt__eq_ a b = a <= b
let v__gt__ a b = v__gt_ a b
let v__lt__ a b = v__lt_ a b
let v__eq__ a b = v__eq_ a b
let v__plus_ a b = a + b
let v__dash_ a b = a - b
let v__star_ a b = a * b
let v__slash_ a b = a / b
let v_mod a b = a mod b
let v__ a = lnot a
let v_u_float_to_int a = int_of_float a
let v_u_int_to_float a = float_of_int a

(* -------------------------------------------------
   Generic printer - works for any concrete value
   ------------------------------------------------- *)
let rec show_any (x : Obj.t) : string =
  match Obj.tag x with
  | tag when tag = Obj.int_tag -> string_of_int (Obj.obj x : int)
  | tag when tag = Obj.string_tag -> Printf.sprintf "%S" (Obj.obj x : string)
  | tag when tag = Obj.double_tag -> string_of_float (Obj.obj x : float)
  | tag when tag = Obj.closure_tag -> "<function>"
  | _ when Obj.is_block x ->
      let n = Obj.size x in
      (* vec runtime representation is: Obj.repr (ref [| ... |]) *)
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
        Type::Float => format!("(string_of_float {})", var),
        Type::Bool => format!("(if {} then \"true\" else \"false\")", var),
        Type::Char => format!("(string_of_int {})", var),
        Type::Unit => "\"()\"".to_string(),
        Type::Function(_, _) => "\"<function>\"".to_string(),
        Type::Var(_) => format!("(show_any (Obj.repr {}))", var),
        Type::List(inner) => {
            let inner_var = "__x";
            let inner_show = show_expr_for_type(inner, inner_var);
            format!(
                "(\"[\" ^ String.concat \" \" (List.map (fun {} -> {}) (Array.to_list !{})) ^ \"]\")",
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
    let mut out = format!(
        "{}\n\nlet result = log_last_with {} ({})\n",
        OCAML_PRELUDE,
        show_fn,
        body
    );
    for (from, to) in [
        ("( + )", "+"),
        ("( +# )", "+"),
        ("( - )", "-"),
        ("( -# )", "-"),
        ("( * )", "*"),
        ("( *# )", "*"),
        ("( / )", "/"),
        ("( /# )", "/"),
        ("( mod )", "mod"),
        ("( mod. )", "mod_float"),
        ("( = )", "="),
        ("( =# )", "="),
        ("( =? )", "="),
        ("( < )", "<"),
        ("( <# )", "<"),
        ("( <. )", "<"),
        ("( > )", ">"),
        ("( ># )", ">"),
        ("( >. )", ">"),
        ("( <= )", "<="),
        ("( <=# )", "<="),
        ("( <=. )", "<="),
        ("( >= )", ">="),
        ("( >=# )", ">="),
        ("( >=. )", ">="),
        ("( +. )", "+."),
        ("( -. )", "-."),
        ("( *. )", "*."),
        ("( /. )", "/."),
        ("(fun  ->", "(fun () ->"),
    ] {
        out = out.replace(from, to);
    }
    out
}

pub fn compile_program_to_ocaml(expr: &Expression) -> Result<String, String> {
    let (_typ, typed_ast) = crate::infer::infer_with_builtins_typed(
        expr,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
    )?;
    Ok(compile_program_to_ocaml_typed(&typed_ast))
}
