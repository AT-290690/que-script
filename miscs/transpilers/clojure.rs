use std::collections::HashSet;
use que::parser::Expression;

fn ident(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => out.push(c.to_ascii_lowercase()),
            _ => out.push('_'),
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    if out.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        out.insert(0, '_');
    }
    if out == "_" {
        "_ignored".to_string()
    } else {
        format!("q_{}", out)
    }
}

fn compile_curried_call(func_src: String, args_src: &[String]) -> String {
    if args_src.is_empty() {
        return format!("({})", func_src);
    }
    args_src
        .iter()
        .fold(func_src, |acc, arg| format!("({} {})", acc, arg))
}

fn compile_lambda(items: &[Expression], mut_vars: &HashSet<String>) -> String {
    if items.len() < 2 {
        return "(fn [] 0)".to_string();
    }
    let body_idx = items.len() - 1;
    let mut scoped_mut = mut_vars.clone();
    let params = items[1..body_idx]
        .iter()
        .filter_map(|p| {
            if let Expression::Word(w) = p {
                scoped_mut.remove(w);
                Some(ident(w))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let body = compile_expr_inner(&items[body_idx], &scoped_mut, true);
    if params.is_empty() {
        format!("(fn [] {})", body)
    } else {
        params
            .iter()
            .rev()
            .fold(body, |acc, p| format!("(fn [{}] {})", p, acc))
    }
}

fn compile_letrec_binding(name: &str, lambda_items: &[Expression], mut_vars: &HashSet<String>, body: String) -> String {
    let name_ident = ident(name);
    let lambda_src = compile_lambda(lambda_items, mut_vars);
    format!(
        "(letfn [({} [& __args] (reduce (fn [f a] (f a)) {} __args))] {})",
        name_ident, lambda_src, body
    )
}

fn compile_do_tail(items: &[Expression], mut_vars: &HashSet<String>, in_fn_body: bool) -> String {
    if items.len() <= 1 {
        return "0".to_string();
    }

    fn build(
        exprs: &[Expression],
        mut_vars: &HashSet<String>,
        in_fn_body: bool,
    ) -> String {
        if exprs.is_empty() {
            return "0".to_string();
        }
        if exprs.len() == 1 {
            return compile_expr_inner(&exprs[0], mut_vars, in_fn_body);
        }

        match &exprs[0] {
            Expression::Apply(let_items)
                if let_items.len() == 3
                    && matches!(let_items.first(), Some(Expression::Word(w)) if w == "let" || w == "letrec" || w == "mut") =>
            {
                let kw = match &let_items[0] {
                    Expression::Word(w) => w.as_str(),
                    _ => unreachable!(),
                };
                let name = match &let_items[1] {
                    Expression::Word(w) => w,
                    _ => return format!("(do {} {})", compile_expr_inner(&exprs[0], mut_vars, false), build(&exprs[1..], mut_vars, in_fn_body)),
                };
                let rhs = &let_items[2];
                let name_ident = ident(name);
                let next = if kw == "mut" {
                    let mut next_mut = mut_vars.clone();
                    next_mut.insert(name.clone());
                    build(&exprs[1..], &next_mut, in_fn_body)
                } else {
                    build(&exprs[1..], mut_vars, in_fn_body)
                };

                if kw == "mut" {
                    let rhs_src = compile_expr_inner(rhs, mut_vars, false);
                    format!("(let [{} (atom {})] {})", name_ident, rhs_src, next)
                } else if kw == "letrec" {
                    if let Expression::Apply(lambda_items) = rhs {
                        if matches!(lambda_items.first(), Some(Expression::Word(w)) if w == "lambda") {
                            return compile_letrec_binding(name, lambda_items, mut_vars, next);
                        }
                    }
                    let rhs_src = compile_expr_inner(rhs, mut_vars, false);
                    format!("(let [{} {}] {})", name_ident, rhs_src, next)
                } else {
                    let rhs_src = compile_expr_inner(rhs, mut_vars, false);
                    format!("(let [{} {}] {})", name_ident, rhs_src, next)
                }
            }
            first => {
                let first_src = compile_expr_inner(first, mut_vars, false);
                let rest_src = build(&exprs[1..], mut_vars, in_fn_body);
                format!("(do {} {})", first_src, rest_src)
            }
        }
    }

    build(&items[1..], mut_vars, in_fn_body)
}

fn compile_expr_inner(expr: &Expression, mut_vars: &HashSet<String>, in_fn_body: bool) -> String {
    match expr {
        Expression::Int(n) => n.to_string(),
        Expression::Dec(n) => format!("{:?}", n),
        Expression::Word(w) => match w.as_str() {
            "nil" => "0".to_string(),
            "true" => "true".to_string(),
            "false" => "false".to_string(),
            _ => {
                let rendered = ident(w);
                if mut_vars.contains(w) {
                    format!("@{}", rendered)
                } else {
                    rendered
                }
            }
        },
        Expression::Apply(items) => {
            if items.is_empty() {
                return "0".to_string();
            }
            match &items[0] {
                Expression::Word(op) => match op.as_str() {
                    "do" => compile_do_tail(items, mut_vars, in_fn_body),
                    "let" => {
                        let name = match items.get(1) {
                            Some(Expression::Word(w)) => ident(w),
                            _ => "_tmp".to_string(),
                        };
                        let rhs = items
                            .get(2)
                            .map(|e| compile_expr_inner(e, mut_vars, false))
                            .unwrap_or_else(|| "0".to_string());
                        format!("(let [{} {}] 0)", name, rhs)
                    }
                    "letrec" => {
                        let name = match items.get(1) {
                            Some(Expression::Word(w)) => w.as_str(),
                            _ => "_tmp",
                        };
                        if let Some(Expression::Apply(lambda_items)) = items.get(2) {
                            if matches!(lambda_items.first(), Some(Expression::Word(w)) if w == "lambda") {
                                return compile_letrec_binding(name, lambda_items, mut_vars, "0".to_string());
                            }
                        }
                        let rhs = items
                            .get(2)
                            .map(|e| compile_expr_inner(e, mut_vars, false))
                            .unwrap_or_else(|| "0".to_string());
                        format!("(let [{} {}] 0)", ident(name), rhs)
                    }
                    "mut" => {
                        let name = match items.get(1) {
                            Some(Expression::Word(w)) => ident(w),
                            _ => "_tmp".to_string(),
                        };
                        let rhs = items
                            .get(2)
                            .map(|e| compile_expr_inner(e, mut_vars, false))
                            .unwrap_or_else(|| "0".to_string());
                        format!("(let [{} (atom {})] 0)", name, rhs)
                    }
                    "vector" | "string" | "tuple" => {
                        let elems = items[1..]
                            .iter()
                            .map(|e| compile_expr_inner(e, mut_vars, false))
                            .collect::<Vec<_>>()
                            .join(" ");
                        format!("(q-vec {})", elems)
                    }
                    "cons" => format!(
                        "(q-cons {} {})",
                        compile_expr_inner(&items[1], mut_vars, false),
                        compile_expr_inner(&items[2], mut_vars, false)
                    ),
                    "length" => format!("(q-len {})", compile_expr_inner(&items[1], mut_vars, false)),
                    "get" => format!(
                        "(q-get {} {})",
                        compile_expr_inner(&items[1], mut_vars, false),
                        compile_expr_inner(&items[2], mut_vars, false)
                    ),
                    "car" => format!("(q-get {} 0)", compile_expr_inner(&items[1], mut_vars, false)),
                    "cdr" => format!(
                        "(q-cdr {} {})",
                        compile_expr_inner(&items[1], mut_vars, false),
                        compile_expr_inner(&items[2], mut_vars, false)
                    ),
                    "fst" => format!("(q-get {} 0)", compile_expr_inner(&items[1], mut_vars, false)),
                    "snd" => format!("(q-get {} 1)", compile_expr_inner(&items[1], mut_vars, false)),
                    "set!" => format!(
                        "(do (q-set! {} {} {}) 0)",
                        compile_expr_inner(&items[1], mut_vars, false),
                        compile_expr_inner(&items[2], mut_vars, false),
                        compile_expr_inner(&items[3], mut_vars, false)
                    ),
                    "pop!" => format!(
                        "(do (q-pop! {}) 0)",
                        compile_expr_inner(&items[1], mut_vars, false)
                    ),
                    "alter!" => {
                        let name = match items.get(1) {
                            Some(Expression::Word(w)) => ident(w),
                            _ => "_tmp".to_string(),
                        };
                        let rhs = compile_expr_inner(&items[2], mut_vars, false);
                        format!("(do (reset! {} {}) 0)", name, rhs)
                    }
                    "+" | "+#" => format!("(+ {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "+." => format!("(+ {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "-" | "-#" => format!("(- {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "-." => format!("(- {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "*" | "*#" => format!("(* {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "*." => format!("(* {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "/" | "/#" => format!("(quot {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "/." => format!("(/ {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "mod" | "mod." => format!("(mod {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "=" | "=?" | "=#" | "=." => format!("(= {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "<" | "<#" | "<." => format!("(< {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    ">" | ">#" | ">." => format!("(> {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "<=" | "<=#" | "<=." => format!("(<= {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    ">=" | ">=#" | ">=." => format!("(>= {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "and" => format!("(and {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "or" => format!("(or {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "not" => format!("(not {})", compile_expr_inner(&items[1], mut_vars, false)),
                    "^" => format!("(bit-xor {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "|" => format!("(bit-or {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "&" => format!("(bit-and {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "~" => format!("(bit-not {})", compile_expr_inner(&items[1], mut_vars, false)),
                    "<<" => format!("(bit-shift-left {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    ">>" => format!("(bit-shift-right {} {})", compile_expr_inner(&items[1], mut_vars, false), compile_expr_inner(&items[2], mut_vars, false)),
                    "if" => format!(
                        "(if {} {} {})",
                        compile_expr_inner(&items[1], mut_vars, false),
                        compile_expr_inner(&items[2], mut_vars, false),
                        compile_expr_inner(&items[3], mut_vars, false)
                    ),
                    "while" => format!(
                        "(do (while {} {}) 0)",
                        compile_expr_inner(&items[1], mut_vars, false),
                        compile_expr_inner(&items[2], mut_vars, false)
                    ),
                    "lambda" => compile_lambda(items, mut_vars),
                    "as" | "char" | "Int->Dec" => compile_expr_inner(&items[1], mut_vars, false),
                    "Dec->Int" => format!("(int {})", compile_expr_inner(&items[1], mut_vars, false)),
                    _ => {
                        let func = compile_expr_inner(&items[0], mut_vars, false);
                        let args = items[1..]
                            .iter()
                            .map(|e| compile_expr_inner(e, mut_vars, false))
                            .collect::<Vec<_>>();
                        compile_curried_call(func, &args)
                    }
                },
                other_head => {
                    let func = compile_expr_inner(other_head, mut_vars, false);
                    let args = items[1..]
                        .iter()
                        .map(|e| compile_expr_inner(e, mut_vars, false))
                        .collect::<Vec<_>>();
                    compile_curried_call(func, &args)
                }
            }
        }
    }
}

const CLOJURE_PRELUDE: &str = r#"(defn q-vec
  [& xs]
  (let [out (java.util.ArrayList.)]
    (doseq [x xs]
      (.add ^java.util.ArrayList out x))
    out))

(defn q-len [xs]
  (.size ^java.util.List xs))

(defn q-get [xs i]
  (.get ^java.util.List xs i))

(defn q-cdr [xs n]
  (let [cnt (.size ^java.util.List xs)]
    (cond
      (<= n 0) (let [out (java.util.ArrayList.)]
                 (doseq [x xs]
                   (.add ^java.util.ArrayList out x))
                 out)
      (>= n cnt) (java.util.ArrayList.)
      :else (let [out (java.util.ArrayList.)]
              (doseq [x (.subList ^java.util.List xs n cnt)]
                (.add ^java.util.ArrayList out x))
              out))))

(defn q-cons [a b]
  (let [out (java.util.ArrayList.)]
    (doseq [x a]
      (.add ^java.util.ArrayList out x))
    (doseq [x b]
      (.add ^java.util.ArrayList out x))
    out))

(defn q-set! [xs i v]
  (let [cnt (.size ^java.util.List xs)]
    (cond
      (= i cnt) (.add ^java.util.List xs v)
      (and (>= i 0) (< i cnt)) (.set ^java.util.List xs i v)
      :else (throw (ex-info "set!: index out of bounds" {:index i :count cnt})))))

(defn q-pop! [xs]
  (let [cnt (.size ^java.util.List xs)]
    (when (> cnt 0)
      (.remove ^java.util.List xs (dec cnt)))))

(defn q-show [x]
  (cond
    (instance? java.util.List x) (vec (map q-show x))
    :else x))
"#;

pub fn compile_program_to_clj(top: &Expression) -> String {
    let body = compile_expr_inner(top, &HashSet::new(), false);
    format!(
        "{}\n\n(let [result {}]\n  (q-show result))\n",
        CLOJURE_PRELUDE, body
    )
}
