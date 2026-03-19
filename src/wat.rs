use crate::infer::{ EffectFlags, TypedExpression };
use crate::parser::Expression;
use crate::types::Type;
use std::collections::{ HashMap, HashSet };

#[derive(Clone)]
struct TopDef {
    expr: Expression,
    node: TypedExpression,
}

#[derive(Clone)]
struct PartialHelper {
    binding_name: String,
    helper_name: String,
    target_name: String,
    captured_nodes: Vec<TypedExpression>,
    remaining_params: Vec<Type>,
    ret: Type,
}

#[derive(Clone)]
struct DynamicPartialHelper {
    name: String,
    total_arity: usize,
}

#[derive(Clone, Debug)]
struct ClosureDef {
    key: String,
    name: String,
    captures: Vec<String>,
    user_arity: usize,
}

struct Ctx<'a> {
    fn_sigs: &'a HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &'a HashMap<String, i32>,
    lambda_ids: &'a HashMap<String, i32>,
    closure_defs: &'a HashMap<String, ClosureDef>,
    lambda_bindings: &'a HashMap<String, TypedExpression>,
    locals: HashMap<String, usize>,
    local_types: HashMap<String, Type>,
    tmp_i32: usize,
}
const EXTRA_I32_LOCALS: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DevirtualizeMode {
    Off,
    KnownHeads,
    Aggressive,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TailCallMode {
    Conservative,
    Aggressive,
}

#[derive(Clone, Copy)]
struct ArithmeticCheckConfig {
    int_overflow_check: bool,
    float_overflow_check: bool,
    div_zero_check: bool,
}

const DBG_GUARD_TRAP_INT_DIV_ZERO: i32 = 1;
const DBG_GUARD_TRAP_FLOAT_DIV_ZERO: i32 = 2;
const DBG_GUARD_TRAP_INT_OVERFLOW_ADD: i32 = 3;
const DBG_GUARD_TRAP_INT_OVERFLOW_SUB: i32 = 4;
const DBG_GUARD_TRAP_INT_OVERFLOW_MUL: i32 = 5;
fn decimal_scale_i32() -> i32 {
    match
        std::env
            ::var("QUE_DECIMAL_SCALE")
            .ok()
            .and_then(|v| v.trim().parse::<i32>().ok())
    {
        Some(scale) if scale > 0 && is_power_of_ten_i32(scale) && scale <= 1_000_000 => scale,
        _ => 1_000,
    }
}

fn decimal_scale_i64() -> i64 {
    decimal_scale_i32() as i64
}

fn is_power_of_ten_i32(n: i32) -> bool {
    if n < 1 {
        return false;
    }
    let mut cur = n;
    while cur % 10 == 0 {
        cur /= 10;
    }
    cur == 1
}

fn emit_guard_trap_wat(code: i32) -> String {
    format!("i32.const {code}\nglobal.set $dbg_guard_trap_code\nunreachable")
}

fn parse_env_bool_like(name: &str, default: bool) -> bool {
    std::env
        ::var(name)
        .ok()
        .map(|v| {
            !matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "false" | "off" | "no")
        })
        .unwrap_or(default)
}

fn arithmetic_check_config() -> ArithmeticCheckConfig {
    ArithmeticCheckConfig {
        int_overflow_check: parse_env_bool_like("QUE_INT_OVERFLOW_CHECK", false),
        float_overflow_check: parse_env_bool_like("QUE_FLOAT_OVERFLOW_CHECK", false),
        div_zero_check: parse_env_bool_like("QUE_DIV_ZERO_CHECK", false),
    }
}

fn devirtualize_mode_from_env() -> Result<DevirtualizeMode, String> {
    let raw = std::env::var("QUE_DEVIRTUALIZE").unwrap_or_else(|_| "aggressive".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" => Ok(DevirtualizeMode::Off),
        "known-heads" | "known_heads" | "known" => Ok(DevirtualizeMode::KnownHeads),
        "aggressive" => Ok(DevirtualizeMode::Aggressive),
        other =>
            Err(
                format!("invalid QUE_DEVIRTUALIZE='{}'. expected one of: off, known-heads, aggressive", other)
            ),
    }
}

fn tail_call_mode_from_env() -> Result<TailCallMode, String> {
    let raw = std::env::var("QUE_TCO").unwrap_or_else(|_| "conservative".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "conservative" | "safe" | "default" => Ok(TailCallMode::Conservative),
        "aggressive" => Ok(TailCallMode::Aggressive),
        other =>
            Err(format!("invalid QUE_TCO='{}'. expected one of: conservative, aggressive", other)),
    }
}

#[derive(Clone, Copy)]
enum VecElemKind {
    I32,
}

fn builtin_fn_tag(name: &str) -> Option<i32> {
    match name {
        "+" | "+#" => Some(1),
        "-" | "-#" => Some(2),
        "*" | "*#" => Some(3),
        "/" | "/#" => Some(4),
        "mod" => Some(5),
        "=" | "=?" | "=#" => Some(6),
        "<" | "<#" => Some(7),
        ">" | ">#" => Some(8),
        "<=" | "<=#" => Some(9),
        ">=" | ">=#" => Some(10),
        "and" => Some(11),
        "or" => Some(12),
        "^" => Some(13),
        "|" => Some(14),
        "&" => Some(15),
        "<<" => Some(16),
        ">>" => Some(17),
        "not" => Some(18),
        "~" => Some(19),
        "length" => Some(20),
        "set!" => Some(21),
        "pop!" => Some(22),
        "fst" => Some(23),
        "snd" => Some(24),
        "+." => Some(25),
        "-." => Some(26),
        "*." => Some(27),
        "/." => Some(28),
        "mod." => Some(29),
        "=." => Some(30),
        "<." => Some(31),
        ">." => Some(32),
        "<=." => Some(33),
        ">=." => Some(34),
        "Int->Dec" => Some(35),
        "Dec->Int" => Some(36),
        "cons" => Some(37),
        _ => None,
    }
}

fn builtin_tag_arity(tag: i32) -> Option<usize> {
    match tag {
        | 1
        | 2
        | 3
        | 4
        | 5
        | 6
        | 7
        | 8
        | 9
        | 10
        | 11
        | 12
        | 13
        | 14
        | 15
        | 16
        | 17
        | 25
        | 26
        | 27
        | 28
        | 29
        | 30
        | 31
        | 32
        | 33
        | 34
        | 37 => Some(2),
        21 => Some(3),
        18 | 19 | 20 | 22 | 23 | 24 | 35 | 36 => Some(1),
        _ => None,
    }
}

fn builtin_tag_first_param_is_ref(tag: i32) -> bool {
    matches!(tag, 21 | 37)
}

fn is_i32ish_type(t: &Type) -> bool {
    matches!(
        t,
        Type::Int |
            Type::Dec |
            Type::Bool |
            Type::Char |
            Type::Unit |
            Type::List(_) |
            Type::Tuple(_) |
            Type::Var(_) |
            Type::Function(_, _)
    )
}

fn is_ref_type(t: &Type) -> bool {
    matches!(t, Type::List(_) | Type::Function(_, _) | Type::Var(_))
}

fn is_managed_local_type(t: &Type) -> bool {
    matches!(t, Type::List(_) | Type::Function(_, _) | Type::Var(_))
}

fn closure_store_op_for_type(t: &Type) -> &'static str {
    match t {
        Type::Function(_, _) => "closure_set_fun",
        Type::List(_) | Type::Var(_) => "closure_set_ref",
        _ => "closure_set",
    }
}

fn closure_store_op_for_type_wat(t: &Type) -> &'static str {
    match t {
        Type::Function(_, _) => "$closure_set_fun",
        Type::List(_) | Type::Var(_) => "$closure_set_ref",
        _ => "$closure_set",
    }
}

impl VecElemKind {
    fn suffix(self) -> &'static str {
        match self {
            VecElemKind::I32 => "i32",
        }
    }
}

fn ident(name: &str) -> String {
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
            _ => s.push_str(&format!("_u{:x}_", c as u32)),
        }
    }
    let mut s = String::new();
    for c in name.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => s.push(c),
            _ => push_encoded(&mut s, c),
        }
    }
    if s.is_empty() {
        s.push_str("_ignored");
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
    format!("v_{}", s)
}

fn cache_init_global(name: &str) -> String {
    format!("g_init_{}", ident(name))
}

fn cache_value_global(name: &str) -> String {
    format!("g_val_{}", ident(name))
}

fn wasm_val_type(typ: &Type) -> Result<&'static str, String> {
    match typ {
        Type::Int | Type::Dec | Type::Bool | Type::Char | Type::Unit => Ok("i32"),
        Type::List(_) | Type::Tuple(_) => Ok("i32"),
        Type::Var(_) => Ok("i32"),
        Type::Function(_, _) => Ok("i32"),
    }
}

fn vec_elem_kind_from_type(typ: &Type) -> Result<VecElemKind, String> {
    match typ {
        | Type::Int
        | Type::Dec
        | Type::Bool
        | Type::Char
        | Type::Unit
        | Type::List(_)
        | Type::Tuple(_) => Ok(VecElemKind::I32),
        Type::Var(_) => Ok(VecElemKind::I32),
        Type::Function(_, _) => Ok(VecElemKind::I32),
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

fn is_special_word(w: &str) -> bool {
    matches!(
        w,
        "do" |
            "let" |
            "mut" |
            "letrec" |
            "lambda" |
            "if" |
            "vector" |
            "string" |
            "tuple" |
            "length" |
            "get" |
            "car" |
            "cdr" |
            "fst" |
            "snd" |
            "set!" |
            "alter!" |
            "pop!" |
            "while" |
            "read!" |
            "write!" |
            "delete!" |
            "move!" |
            "list-dir!" |
            "mkdir!" |
            "print!" |
            "sleep!" |
            "clear!" |
            "+" |
            "+#" |
            "+." |
            "-" |
            "-#" |
            "-." |
            "*" |
            "*#" |
            "*." |
            "/" |
            "/#" |
            "/." |
            "mod" |
            "mod." |
            "=" |
            "=?" |
            "=#" |
            "=." |
            "<" |
            "<#" |
            "<." |
            ">" |
            ">#" |
            ">." |
            "<=" |
            "<=#" |
            "<=." |
            ">=" |
            ">=#" |
            ">=." |
            "and" |
            "or" |
            "not" |
            "^" |
            "|" |
            "&" |
            "<<" |
            ">>" |
            "~" |
            "Int->Dec" |
            "Dec->Int" |
            "cons" |
            "true" |
            "false" |
            "nil"
    )
}

fn collect_pattern_words(expr: &Expression, out: &mut HashSet<String>) {
    match expr {
        Expression::Word(w) => {
            out.insert(w.clone());
        }
        Expression::Apply(items) => {
            for it in items {
                collect_pattern_words(it, out);
            }
        }
        _ => {}
    }
}

fn collect_refs(expr: &Expression, bound: &mut HashSet<String>, out: &mut HashSet<String>) {
    match expr {
        Expression::Word(w) => {
            if !bound.contains(w) && !is_special_word(w) {
                out.insert(w.clone());
            }
        }
        Expression::Apply(items) => {
            if items.is_empty() {
                return;
            }
            if let Expression::Word(op) = &items[0] {
                if op == "lambda" {
                    let mut scoped = bound.clone();
                    for p in &items[1..items.len().saturating_sub(1)] {
                        collect_pattern_words(p, &mut scoped);
                    }
                    if let Some(body) = items.last() {
                        collect_refs(body, &mut scoped, out);
                    }
                    return;
                }
                if op == "do" {
                    for it in &items[1..] {
                        if let Expression::Apply(let_items) = it {
                            if
                                let [Expression::Word(kw), Expression::Word(name), rhs] =
                                    &let_items[..]
                            {
                                if kw == "let" || kw == "letrec" || kw == "mut" {
                                    collect_refs(rhs, bound, out);
                                    bound.insert(name.clone());
                                    continue;
                                }
                            }
                        }
                        collect_refs(it, bound, out);
                    }
                    return;
                }
                if op == "let" || op == "letrec" || op == "mut" {
                    if let [_, Expression::Word(name), rhs] = &items[..] {
                        collect_refs(rhs, bound, out);
                        bound.insert(name.clone());
                        return;
                    }
                    if let Some(rhs) = items.get(2) {
                        collect_refs(rhs, bound, out);
                    }
                    return;
                }
                // Type/cast hints are compile-time-only in this backend.
                // Do not treat the hint operand as a runtime dependency.
                if op == "as" || op == "char" {
                    if let Some(v) = items.get(1) {
                        collect_refs(v, bound, out);
                    }
                    return;
                }
            }
            for it in items {
                collect_refs(it, bound, out);
            }
        }
        _ => {}
    }
}

fn collect_lambda_nodes(node: &TypedExpression, out: &mut Vec<TypedExpression>) {
    if let Expression::Apply(items) = &node.expr {
        if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
            out.push(node.clone());
        }
    }
    for ch in &node.children {
        collect_lambda_nodes(ch, out);
    }
}

fn collect_let_lambda_bindings(node: &TypedExpression, out: &mut HashMap<String, TypedExpression>) {
    if let Expression::Apply(items) = &node.expr {
        if let [Expression::Word(kw), Expression::Word(name), _] = &items[..] {
            if kw == "let" || kw == "letrec" {
                if let Some(rhs) = node.children.get(2) {
                    match &rhs.expr {
                        Expression::Apply(xs) if
                            matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                        => {
                            out.insert(name.clone(), rhs.clone());
                        }
                        Expression::Word(alias) => {
                            if let Some(target) = out.get(alias).cloned() {
                                out.insert(name.clone(), target);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    for ch in &node.children {
        collect_let_lambda_bindings(ch, out);
    }
}

fn lambda_is_hoistable(node: &TypedExpression, _top_defs: &HashMap<String, TopDef>) -> bool {
    let items = match &node.expr {
        Expression::Apply(xs) => xs,
        _ => {
            return false;
        }
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") || items.len() < 2 {
        return false;
    }
    let mut bound = HashSet::new();
    for p in &items[1..items.len() - 1] {
        collect_pattern_words(p, &mut bound);
    }
    let mut refs = HashSet::new();
    if let Some(body) = items.last() {
        collect_refs(body, &mut bound, &mut refs);
    }
    refs.is_empty()
}

fn lambda_capture_names(
    node: &TypedExpression,
    _top_defs: &HashMap<String, TopDef>
) -> Vec<String> {
    let items = match &node.expr {
        Expression::Apply(xs) => xs,
        _ => {
            return Vec::new();
        }
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") || items.len() < 2 {
        return Vec::new();
    }
    let mut bound = HashSet::new();
    for p in &items[1..items.len() - 1] {
        collect_pattern_words(p, &mut bound);
    }
    let mut refs = HashSet::new();
    if let Some(body) = items.last() {
        collect_refs(body, &mut bound, &mut refs);
    }
    let mut caps = refs.into_iter().collect::<Vec<_>>();
    caps.sort();
    caps
}

fn lambda_syntax_arity(expr: &Expression) -> usize {
    match expr {
        Expression::Apply(items) if
            matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") &&
            items.len() >= 2
        => {
            items.len().saturating_sub(2)
        }
        _ => 0,
    }
}

fn collect_apply_arities_from_code(code: &str, out: &mut HashSet<usize>) {
    let needle = "call $apply";
    let mut rest = code;
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        let digit_count = after
            .bytes()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if digit_count > 0 {
            let digits = &after[..digit_count];
            if after[digit_count..].starts_with("_i32") {
                if let Ok(n) = digits.parse::<usize>() {
                    out.insert(n);
                }
            }
        }
        rest = &after[digit_count..];
    }
}

fn emit_high_arity_apply_i32(
    arity: usize,
    fn_ids: &HashMap<String, i32>,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    closure_defs: &HashMap<String, ClosureDef>
) -> String {
    let mut out = String::new();
    out.push_str(&format!("  (func $apply{}_i32 (param $f i32)", arity));
    for i in 0..arity {
        out.push_str(&format!(" (param $a{} i32)", i));
    }
    out.push_str(" (result i32)\n");

    let closure_cases = closure_defs
        .values()
        .filter_map(|def| {
            let fid = *fn_ids.get(&def.name)?;
            let (ps, ret) = fn_sigs.get(&def.name)?;
            if
                def.user_arity != arity ||
                !is_i32ish_type(ret) ||
                ps.len() != def.captures.len() + arity
            {
                return None;
            }
            if !ps.iter().all(is_i32ish_type) {
                return None;
            }
            Some((fid, def.name.clone(), def.captures.len()))
        })
        .collect::<Vec<_>>();

    if !closure_cases.is_empty() {
        out.push_str(
            "    local.get $f\n    i32.const -2147483648\n    i32.and\n    i32.const -2147483648\n    i32.eq\n    if (result i32)\n"
        );
        for (fid, name, cap_len) in &closure_cases {
            out.push_str(
                &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
            );
            for i in 0..*cap_len {
                out.push_str(
                    &format!("        local.get $f\n        i32.const {}\n        call $closure_get\n", i)
                );
            }
            for i in 0..arity {
                out.push_str(&format!("        local.get $a{}\n", i));
            }
            out.push_str(&format!("        call ${}\n", ident(name)));
            out.push_str("      else\n");
        }
        out.push_str("        unreachable\n");
        for _ in 0..closure_cases.len() {
            out.push_str("      end\n");
        }
        out.push_str("    else\n");
    }

    let mut direct_cases = 0usize;
    for (name, tag) in fn_ids {
        if let Some((ps, ret)) = fn_sigs.get(name) {
            if ps.len() == arity && ps.iter().all(is_i32ish_type) && is_i32ish_type(ret) {
                direct_cases += 1;
                out.push_str(
                    &format!("    local.get $f\n    i32.const {}\n    i32.eq\n    if (result i32)\n", tag)
                );
                for i in 0..arity {
                    out.push_str(&format!("      local.get $a{}\n", i));
                }
                out.push_str(&format!("      call ${}\n    else\n", ident(name)));
            }
        }
    }

    out.push_str("      unreachable\n");
    for _ in 0..direct_cases {
        out.push_str("    end\n");
    }
    if !closure_cases.is_empty() {
        out.push_str("    end\n");
    }
    out.push_str("  )\n");
    out
}

fn emit_vector_runtime(
    fn_ids: &HashMap<String, i32>,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    closure_defs: &HashMap<String, ClosureDef>,
    apply_arities: &HashSet<usize>
) -> String {
    fn parse_env_i32(name: &str, default: i32, min: i32, max: i32) -> i32 {
        std::env
            ::var(name)
            .ok()
            .and_then(|v| v.trim().parse::<i32>().ok())
            .map(|v| v.clamp(min, max))
            .unwrap_or(default)
    }

    let vec_min_cap = parse_env_i32("QUE_VEC_MIN_CAP", 2, 1, 4096);
    let vec_growth_num = parse_env_i32("QUE_VEC_GROWTH_NUM", 2, 1, 64);
    let vec_growth_den = parse_env_i32("QUE_VEC_GROWTH_DEN", 1, 1, 64);
    let vec_bounds_check_enabled = parse_env_bool_like("QUE_BOUNDS_CHECK", true);

    let mut apply_arities = apply_arities.clone();
    // apply3 fallback chains through apply1, so ensure apply1 runtime exists.
    if apply_arities.contains(&3) {
        apply_arities.insert(1);
    }
    let mut out = String::new();
    out.push_str(
        r#"
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 65536))
  (global $free_head (mut i32) (i32.const 0))
  (global $free_small_16 (mut i32) (i32.const 0))
  (global $free_small_32 (mut i32) (i32.const 0))
  (global $free_small_64 (mut i32) (i32.const 0))
  (global $free_small_128 (mut i32) (i32.const 0))
  ;; Runtime ARGV storage (vector pointer). Lazily initialized to [].
  (global $argv_ptr (mut i32) (i32.const 0))
  ;; Debug-only guard trap code (0 means no guard trap).
  (global $dbg_guard_trap_code (mut i32) (i32.const 0))
  (export "dbg_guard_trap_code" (global $dbg_guard_trap_code))
  ;; __DBG_RC_GLOBALS__

  (func $alloc (param $n i32) (result i32)
    (local $prev i32)
    (local $cur i32)
    (local $next i32)
    (local $size i32)
    (local $rem_size i32)
    (local $rem_base i32)
    (local $base i32)
    (local $needed_end i32)
    (local $cur_bytes i32)
    (local $delta i32)
    (local $grow_pages i32)
    (local $grow_res i32)
    ;; __DBG_RC_ALLOC_INC__
    ;; Small-block fast path (segregated free lists).
    ;; Rounds small requests to class size and pops in O(1) when available.
    local.get $n
    i32.const 16
    i32.le_s
    if
      i32.const 16
      local.set $n
      global.get $free_small_16
      local.tee $cur
      i32.eqz
      if
      else
        local.get $cur
        i32.const 4
        i32.add
        i32.load
        global.set $free_small_16
        local.get $cur
        local.get $n
        i32.store
        local.get $cur
        i32.const 4
        i32.add
        i32.const 0
        i32.store
        local.get $cur
        i32.const 8
        i32.add
        return
      end
    else
      local.get $n
      i32.const 32
      i32.le_s
      if
        i32.const 32
        local.set $n
        global.get $free_small_32
        local.tee $cur
        i32.eqz
        if
        else
          local.get $cur
          i32.const 4
          i32.add
          i32.load
          global.set $free_small_32
          local.get $cur
          local.get $n
          i32.store
          local.get $cur
          i32.const 4
          i32.add
          i32.const 0
          i32.store
          local.get $cur
          i32.const 8
          i32.add
          return
        end
      else
        local.get $n
        i32.const 64
        i32.le_s
        if
          i32.const 64
          local.set $n
          global.get $free_small_64
          local.tee $cur
          i32.eqz
          if
          else
            local.get $cur
            i32.const 4
            i32.add
            i32.load
            global.set $free_small_64
            local.get $cur
            local.get $n
            i32.store
            local.get $cur
            i32.const 4
            i32.add
            i32.const 0
            i32.store
            local.get $cur
            i32.const 8
            i32.add
            return
          end
        else
          local.get $n
          i32.const 128
          i32.le_s
          if
            i32.const 128
            local.set $n
            global.get $free_small_128
            local.tee $cur
            i32.eqz
            if
            else
              local.get $cur
              i32.const 4
              i32.add
              i32.load
              global.set $free_small_128
              local.get $cur
              local.get $n
              i32.store
              local.get $cur
              i32.const 4
              i32.add
              i32.const 0
              i32.store
              local.get $cur
              i32.const 8
              i32.add
              return
            end
          end
        end
      end
    end
    global.get $free_head
    local.set $cur
    i32.const 0
    local.set $prev
    block $scan_done
      loop $scan
        local.get $cur
        i32.eqz
        br_if $scan_done
        local.get $cur
        i32.load
        local.set $size
        local.get $size
        local.get $n
        i32.ge_s
        if
          local.get $cur
          i32.const 4
          i32.add
          i32.load
          local.set $next
          local.get $size
          local.get $n
          i32.sub
          i32.const 16
          i32.ge_s
          if
            local.get $size
            local.get $n
            i32.sub
            i32.const 8
            i32.sub
            local.set $rem_size
            local.get $cur
            i32.const 8
            i32.add
            local.get $n
            i32.add
            local.set $rem_base
            local.get $rem_base
            local.get $rem_size
            i32.store
            local.get $rem_base
            i32.const 4
            i32.add
            local.get $next
            i32.store
            local.get $prev
            i32.eqz
            if
              local.get $rem_base
              global.set $free_head
            else
              local.get $prev
              i32.const 4
              i32.add
              local.get $rem_base
              i32.store
            end
            local.get $cur
            local.get $n
            i32.store
            local.get $cur
            i32.const 4
            i32.add
            i32.const 0
            i32.store
          else
            local.get $prev
            i32.eqz
            if
              local.get $next
              global.set $free_head
            else
              local.get $prev
              i32.const 4
              i32.add
              local.get $next
              i32.store
            end
          end
          local.get $cur
          i32.const 8
          i32.add
          return
        end
        local.get $cur
        local.set $prev
        local.get $cur
        i32.const 4
        i32.add
        i32.load
        local.set $cur
        br $scan
      end
    end
    global.get $heap
    local.set $base
    local.get $base
    local.get $n
    i32.const 8
    i32.add
    i32.add
    local.set $needed_end
    memory.size
    i32.const 16
    i32.shl
    local.set $cur_bytes
    local.get $needed_end
    local.get $cur_bytes
    i32.gt_u
    if
      local.get $needed_end
      local.get $cur_bytes
      i32.sub
      local.set $delta
      local.get $delta
      i32.const 65535
      i32.add
      i32.const 16
      i32.shr_u
      local.set $grow_pages
      local.get $grow_pages
      memory.grow
      local.set $grow_res
      local.get $grow_res
      i32.const -1
      i32.eq
      if
        unreachable
      end
    end
    local.get $base
    local.get $n
    i32.store
    local.get $base
    i32.const 4
    i32.add
    i32.const 0
    i32.store
    local.get $base
    local.get $n
    i32.const 8
    i32.add
    i32.add
    global.set $heap
    local.get $base
    i32.const 8
    i32.add
  )

  (func $free (param $ptr i32) (result i32)
    (local $base i32)
    (local $size i32)
    (local $prev i32)
    (local $cur i32)
    (local $next i32)
    (local $cur_size i32)
    (local $prev_size i32)
    local.get $ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    ;; __DBG_RC_FREE_INC__
    local.get $ptr
    i32.const 8
    i32.sub
    local.set $base
    local.get $base
    i32.load
    local.set $size
    ;; Small-block fast path: keep tiny blocks in size bins for O(1) reuse.
    local.get $size
    i32.const 16
    i32.eq
    if
      local.get $base
      i32.const 4
      i32.add
      global.get $free_small_16
      i32.store
      local.get $base
      global.set $free_small_16
      i32.const 0
      return
    end
    local.get $size
    i32.const 32
    i32.eq
    if
      local.get $base
      i32.const 4
      i32.add
      global.get $free_small_32
      i32.store
      local.get $base
      global.set $free_small_32
      i32.const 0
      return
    end
    local.get $size
    i32.const 64
    i32.eq
    if
      local.get $base
      i32.const 4
      i32.add
      global.get $free_small_64
      i32.store
      local.get $base
      global.set $free_small_64
      i32.const 0
      return
    end
    local.get $size
    i32.const 128
    i32.eq
    if
      local.get $base
      i32.const 4
      i32.add
      global.get $free_small_128
      i32.store
      local.get $base
      global.set $free_small_128
      i32.const 0
      return
    end
    i32.const 0
    local.set $prev
    global.get $free_head
    local.set $cur
    block $ins_done
      loop $ins
        local.get $cur
        i32.eqz
        br_if $ins_done
        local.get $cur
        local.get $base
        i32.ge_u
        br_if $ins_done
        local.get $cur
        local.set $prev
        local.get $cur
        i32.const 4
        i32.add
        i32.load
        local.set $cur
        br $ins
      end
    end
    local.get $base
    i32.const 4
    i32.add
    local.get $cur
    i32.store
    local.get $prev
    i32.eqz
    if
      local.get $base
      global.set $free_head
    else
      local.get $prev
      i32.const 4
      i32.add
      local.get $base
      i32.store
    end

    local.get $cur
    i32.eqz
    if
    else
      local.get $base
      i32.const 8
      i32.add
      local.get $size
      i32.add
      local.get $cur
      i32.eq
      if
        local.get $cur
        i32.load
        local.set $cur_size
        local.get $cur
        i32.const 4
        i32.add
        i32.load
        local.set $next
        local.get $size
        i32.const 8
        i32.add
        local.get $cur_size
        i32.add
        local.set $size
        local.get $base
        local.get $size
        i32.store
        local.get $base
        i32.const 4
        i32.add
        local.get $next
        i32.store
      end
    end

    local.get $prev
    i32.eqz
    if
    else
      local.get $prev
      i32.load
      local.set $prev_size
      local.get $prev
      i32.const 8
      i32.add
      local.get $prev_size
      i32.add
      local.get $base
      i32.eq
      if
        local.get $prev_size
        i32.const 8
        i32.add
        local.get $size
        i32.add
        local.set $prev_size
        local.get $prev
        local.get $prev_size
        i32.store
        local.get $base
        i32.const 4
        i32.add
        i32.load
        local.set $next
        local.get $prev
        i32.const 4
        i32.add
        local.get $next
        i32.store
      end
    end

    i32.const 0
  )

  (func $vec_len (param $ptr i32) (result i32)
    local.get $ptr
    i32.load
  )

  (func $rc_retain_vec (param $ptr i32) (result i32)
    local.get $ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 8
    i32.add
    local.get $ptr
    i32.const 8
    i32.add
    i32.load
    i32.const 1
    i32.add
    i32.store
    i32.const 0
  )

  (func $rc_release_vec (param $ptr i32) (result i32)
    (local $rc i32)
    (local $len i32)
    (local $i i32)
    (local $elem_ref i32)
    (local $data i32)
    (local $v i32)
    local.get $ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 8
    i32.add
    i32.load
    local.set $rc
    ;; __DBG_RC_RELEASE_VEC_RC_HIST__
    local.get $rc
    i32.const 1
    i32.sub
    local.set $rc
    local.get $ptr
    i32.const 8
    i32.add
    local.get $rc
    i32.store
    ;; __DBG_RC_RELEASE_VEC_DEC__
    local.get $rc
    i32.const 0
    i32.gt_s
    if
      ;; __DBG_RC_RELEASE_VEC_GT0__
      i32.const 0
      return
    end
    ;; __DBG_RC_RELEASE_VEC_FREE_PATH__
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $elem_ref
    local.get $ptr
    i32.const 16
    i32.add
    i32.load
    local.set $data
    local.get $elem_ref
    i32.const 0
    i32.eq
    if
      local.get $data
      call $free
      drop
      local.get $ptr
      call $free
      drop
      i32.const 0
      return
    end
    local.get $ptr
    i32.load
    local.set $len
    i32.const 0
    local.set $i
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_s
        br_if $done
        local.get $data
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $v
        local.get $v
        call $rc_release
        drop
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $data
    call $free
    drop
    local.get $ptr
    call $free
    drop
    i32.const 0
  )

  (func $tuple_new (param $a i32) (param $b i32) (result i32)
    (local $ptr i32)
    ;; Tuples are represented as 2-element reference vectors.
    ;; This gives tuple fields correct retain/release semantics.
    i32.const 0
    i32.const 1
    call $vec_new_i32
    local.set $ptr
    local.get $ptr
    local.get $a
    call $vec_push_i32
    drop
    local.get $ptr
    local.get $b
    call $vec_push_i32
    drop
    local.get $ptr
  )

  (func $tuple_fst (param $ptr i32) (result i32)
    local.get $ptr
    i32.const 0
    call $vec_get_i32
  )

  (func $tuple_snd (param $ptr i32) (result i32)
    local.get $ptr
    i32.const 1
    call $vec_get_i32
  )

  (func $__argv_get (result i32)
    global.get $argv_ptr
    i32.eqz
    if
      i32.const 0
      i32.const 1
      call $vec_new_i32
      global.set $argv_ptr
    end
    global.get $argv_ptr
  )

  (func (export "get_argv") (result i32)
    call $__argv_get
  )

  (func (export "set_argv") (param $ptr i32) (result i32)
    local.get $ptr
    call $rc_retain
    drop
    global.get $argv_ptr
    call $rc_release
    drop
    local.get $ptr
    global.set $argv_ptr
    i32.const 0
  )

  (func (export "argv_clear") (result i32)
    (local $v i32)
    i32.const 0
    i32.const 1
    call $vec_new_i32
    local.set $v
    global.get $argv_ptr
    call $rc_release
    drop
    local.get $v
    global.set $argv_ptr
    i32.const 0
  )

  (func (export "argv_push") (param $v i32) (result i32)
    call $__argv_get
    local.get $v
    call $vec_push_i32
  )

  (func (export "make_vec") (param $elem_ref i32) (result i32)
    i32.const 0
    local.get $elem_ref
    call $vec_new_i32
  )

  (func (export "vec_push") (param $ptr i32) (param $v i32) (result i32)
    local.get $ptr
    local.get $v
    call $vec_push_i32
  )

  (func (export "make_tuple") (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    call $tuple_new
  )

  (func (export "retain") (param $ptr i32) (result i32)
    local.get $ptr
    call $rc_retain
  )

  (func (export "release") (param $ptr i32) (result i32)
    local.get $ptr
    call $rc_release
  )

  (func $closure_new (param $fn i32) (param $n i32) (result i32)
    (local $ptr i32)
    (local $i i32)
    i32.const 12
    local.get $n
    i32.const 8
    i32.mul
    i32.add
    call $alloc
    local.set $ptr
    local.get $ptr
    local.get $fn
    i32.store
    local.get $ptr
    i32.const 4
    i32.add
    local.get $n
    i32.store
    local.get $ptr
    i32.const 8
    i32.add
    i32.const 1
    i32.store
    i32.const 0
    local.set $i
    block $done
      loop $init
        local.get $i
        local.get $n
        i32.ge_s
        br_if $done
        local.get $ptr
        i32.const 12
        i32.add
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.const 0
        i32.store
        local.get $ptr
        i32.const 12
        i32.add
        local.get $n
        i32.const 4
        i32.mul
        i32.add
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.const 0
        i32.store
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $init
      end
    end
    local.get $ptr
    i32.const -2147483648
    i32.or
  )

  (func $closure_set (param $ptr i32) (param $idx i32) (param $v i32) (result i32)
    (local $base i32)
    (local $n i32)
    local.get $ptr
    i32.const 2147483647
    i32.and
    local.tee $base
    i32.const 4
    i32.add
    i32.load
    local.set $n
    local.get $base
    i32.const 12
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.const 0
    i32.store
    local.get $base
    i32.const 12
    i32.add
    local.get $n
    i32.const 4
    i32.mul
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    local.get $v
    i32.store
    i32.const 0
  )

  (func $closure_set_ref (param $ptr i32) (param $idx i32) (param $v i32) (result i32)
    (local $base i32)
    (local $n i32)
    (local $old i32)
    (local $old_ref i32)
    local.get $ptr
    i32.const 2147483647
    i32.and
    local.tee $base
    i32.const 4
    i32.add
    i32.load
    local.set $n
    local.get $base
    i32.const 12
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.load
    local.set $old_ref
    local.get $old_ref
    i32.const 0
    i32.ne
    if
      local.get $base
      i32.const 12
      i32.add
      local.get $n
      i32.const 4
      i32.mul
      i32.add
      local.get $idx
      i32.const 4
      i32.mul
      i32.add
      i32.load
      local.set $old
      local.get $old
      call $rc_release
      drop
    end
    local.get $v
    call $rc_retain
    drop
    local.get $base
    i32.const 12
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.const 1
    i32.store
    local.get $base
    i32.const 12
    i32.add
    local.get $n
    i32.const 4
    i32.mul
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    local.get $v
    i32.store
    i32.const 0
  )

  (func $closure_set_fun (param $ptr i32) (param $idx i32) (param $v i32) (result i32)
    local.get $v
    i32.const -2147483648
    i32.and
    i32.const -2147483648
    i32.eq
    if (result i32)
      local.get $ptr
      local.get $idx
      local.get $v
      call $closure_set_ref
    else
      local.get $ptr
      local.get $idx
      local.get $v
      call $closure_set
    end
  )

  (func $closure_get (param $ptr i32) (param $idx i32) (result i32)
    (local $base i32)
    (local $n i32)
    local.get $ptr
    i32.const 2147483647
    i32.and
    local.tee $base
    i32.const 4
    i32.add
    i32.load
    local.set $n
    local.get $base
    i32.const 12
    i32.add
    local.get $n
    i32.const 4
    i32.mul
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.load
  )

  (func $closure_fn (param $ptr i32) (result i32)
    local.get $ptr
    i32.const 2147483647
    i32.and
    i32.load
  )

  (func $closure_retain (param $ptr i32) (result i32)
    (local $base i32)
    local.get $ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 2147483647
    i32.and
    local.tee $base
    i32.const 8
    i32.add
    local.get $base
    i32.const 8
    i32.add
    i32.load
    i32.const 1
    i32.add
    i32.store
    i32.const 0
  )

  (func $closure_release (param $ptr i32) (result i32)
    (local $base i32)
    (local $n i32)
    (local $rc i32)
    (local $i i32)
    (local $flag i32)
    (local $v i32)
    local.get $ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 2147483647
    i32.and
    local.set $base
    local.get $base
    i32.const 8
    i32.add
    i32.load
    local.set $rc
    local.get $rc
    i32.const 1
    i32.sub
    local.set $rc
    local.get $base
    i32.const 8
    i32.add
    local.get $rc
    i32.store
    local.get $rc
    i32.const 0
    i32.gt_s
    if
      i32.const 0
      return
    end
    local.get $base
    i32.const 4
    i32.add
    i32.load
    local.set $n
    i32.const 0
    local.set $i
    block $done
      loop $loop
        local.get $i
        local.get $n
        i32.ge_s
        br_if $done
        local.get $base
        i32.const 12
        i32.add
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $flag
        local.get $flag
        i32.const 0
        i32.ne
        if
          local.get $base
          i32.const 12
          i32.add
          local.get $n
          i32.const 4
          i32.mul
          i32.add
          local.get $i
          i32.const 4
          i32.mul
          i32.add
          i32.load
          local.set $v
          local.get $v
          call $rc_release
          drop
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $base
    call $free
    drop
    i32.const 0
  )

  (func $is_vec_ptr (param $ptr i32) (result i32)
    (local $mem_end i32)
    (local $len i32)
    (local $cap i32)
    (local $rc i32)
    (local $elem_ref i32)
    (local $data i32)
    (local $avail i32)
    (local $data_base i32)
    (local $data_block_size i32)
    local.get $ptr
    i32.const 65536
    i32.lt_u
    if
      i32.const 0
      return
    end
    memory.size
    i32.const 16
    i32.shl
    local.set $mem_end
    local.get $ptr
    i32.const 24
    i32.add
    local.get $mem_end
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    local.set $cap
    local.get $ptr
    i32.const 8
    i32.add
    i32.load
    local.set $rc
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $elem_ref
    local.get $ptr
    i32.const 16
    i32.add
    i32.load
    local.set $data
    local.get $ptr
    i32.const 20
    i32.add
    i32.load
    i32.const 1447380017
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $rc
    i32.const 0
    i32.le_s
    if
      i32.const 0
      return
    end
    local.get $len
    i32.const 0
    i32.lt_s
    if
      i32.const 0
      return
    end
    local.get $cap
    i32.const 0
    i32.lt_s
    if
      i32.const 0
      return
    end
    local.get $len
    local.get $cap
    i32.gt_s
    if
      i32.const 0
      return
    end
    local.get $elem_ref
    i32.const 0
    i32.ne
    if
      local.get $elem_ref
      i32.const 1
      i32.ne
      if
        i32.const 0
        return
      end
    end
    local.get $data
    i32.const 8
    i32.sub
    local.set $data_base
    local.get $data_base
    i32.const 65536
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $data_base
    i32.const 8
    i32.add
    local.get $mem_end
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $data_base
    i32.load
    local.set $data_block_size
    local.get $data_block_size
    i32.const 0
    i32.lt_s
    if
      i32.const 0
      return
    end
    local.get $cap
    i32.const 4
    i32.mul
    local.get $data_block_size
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $data
    local.get $mem_end
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $mem_end
    local.get $data
    i32.sub
    local.set $avail
    local.get $cap
    local.get $avail
    i32.const 2
    i32.shr_u
    i32.gt_u
    if
      i32.const 0
      return
    end
    i32.const 1
  )

  ;; __DBG_RC_HELPERS__

  (func $rc_retain (param $ptr i32) (result i32)
    local.get $ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    ;; __DBG_RC_RETAIN_INC__
    local.get $ptr
    i32.const -2147483648
    i32.and
    i32.const -2147483648
    i32.eq
    if
      local.get $ptr
      i32.const 2147483647
      i32.and
      i32.const 65536
      i32.lt_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      i32.const 2147483647
      i32.and
      memory.size
      i32.const 16
      i32.shl
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      call $closure_retain
      return
    end
    local.get $ptr
    i32.const 65536
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    call $is_vec_ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $ptr
    call $rc_retain_vec
  )

  (func $rc_release (param $ptr i32) (result i32)
    local.get $ptr
    i32.eqz
    if
      i32.const 0
      return
    end
    ;; __DBG_RC_RELEASE_INC__
    local.get $ptr
    i32.const -2147483648
    i32.and
    i32.const -2147483648
    i32.eq
    if
      local.get $ptr
      i32.const 2147483647
      i32.and
      i32.const 65536
      i32.lt_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      i32.const 2147483647
      i32.and
      memory.size
      i32.const 16
      i32.shl
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $ptr
      call $closure_release
      return
    end
    local.get $ptr
    i32.const 65536
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    call $is_vec_ptr
    i32.eqz
    if
      ;; __DBG_RC_RELEASE_REJECT_NOT_VEC__
      i32.const 0
      return
    end
    ;; __DBG_RC_RELEASE_TAKE_VEC_PATH__
    local.get $ptr
    call $rc_release_vec
  )

  (func $vec_new_i32 (param $len i32) (param $elem_ref i32) (result i32)
    (local $cap i32)
    (local $ptr i32)
    (local $data i32)
    (local $i i32)
    ;; __DBG_RC_VEC_NEW_INC__
    local.get $len
    i32.const __VEC_MIN_CAP__
    i32.lt_s
    if (result i32)
      i32.const __VEC_MIN_CAP__
    else
      local.get $len
    end
    local.set $cap
    i32.const 24
    call $alloc
    local.set $ptr
    local.get $cap
    i32.const 4
    i32.mul
    call $alloc
    local.set $data
    local.get $ptr
    local.get $len
    i32.store
    local.get $ptr
    i32.const 4
    i32.add
    local.get $cap
    i32.store
    local.get $ptr
    i32.const 8
    i32.add
    i32.const 1
    i32.store
    local.get $ptr
    i32.const 12
    i32.add
    local.get $elem_ref
    i32.store
    local.get $ptr
    i32.const 16
    i32.add
    local.get $data
    i32.store
    local.get $ptr
    i32.const 20
    i32.add
    i32.const 1447380017
    i32.store
    ;; Initialize backing storage so first write to an existing slot does not
    ;; read/release uninitialized garbage when elem_ref=1.
    i32.const 0
    local.set $i
    block $done
      loop $zero
        local.get $i
        local.get $cap
        i32.ge_s
        br_if $done
        local.get $data
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.const 0
        i32.store
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $zero
      end
    end
    local.get $ptr
  )

  (func $vec_get_i32 (param $ptr i32) (param $idx i32) (result i32)
    (local $len i32)
    ;; __VEC_GET_BOUNDS_CHECK__
    local.get $ptr
    i32.const 16
    i32.add
    i32.load
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.load
  )

  (func $vec_grow_i32 (param $ptr i32) (result i32)
    (local $cap i32)
    (local $new_cap i32)
    (local $len i32)
    (local $old_data i32)
    (local $new_data i32)
    (local $i i32)
    (local $v i32)
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    local.set $cap
    local.get $cap
    i32.const __VEC_GROWTH_NUM__
    i32.mul
    i32.const __VEC_GROWTH_DEN__
    i32.div_s
    local.set $new_cap
    local.get $new_cap
    local.get $cap
    i32.le_s
    if
      local.get $cap
      i32.const 1
      i32.add
      local.set $new_cap
    end
    local.get $new_cap
    i32.const 1
    i32.lt_s
    if
      i32.const 1
      local.set $new_cap
    end
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 16
    i32.add
    i32.load
    local.set $old_data
    local.get $new_cap
    i32.const 4
    i32.mul
    call $alloc
    local.set $new_data
    i32.const 0
    local.set $i
    block $done
      loop $copy
        local.get $i
        local.get $len
        i32.ge_s
        br_if $done
        local.get $old_data
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $v
        local.get $new_data
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        local.get $v
        i32.store
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy
      end
    end
    local.get $old_data
    call $free
    drop
    local.get $ptr
    i32.const 4
    i32.add
    local.get $new_cap
    i32.store
    local.get $ptr
    i32.const 16
    i32.add
    local.get $new_data
    i32.store
    i32.const 0
  )

  (func $vec_push_i32 (param $ptr i32) (param $v i32) (result i32)
    (local $len i32)
    (local $cap i32)
    (local $addr i32)
    (local $elem_ref i32)
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    local.set $cap
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $elem_ref
    local.get $len
    local.get $cap
    i32.lt_s
    i32.eqz
    if
      local.get $ptr
      call $vec_grow_i32
      drop
    end
    local.get $elem_ref
    i32.const 0
    i32.ne
    if
      local.get $v
      call $rc_retain
      drop
    end
    local.get $ptr
    i32.const 16
    i32.add
    i32.load
    local.get $len
    i32.const 4
    i32.mul
    i32.add
    local.set $addr
    local.get $addr
    local.get $v
    i32.store
    local.get $ptr
    local.get $len
    i32.const 1
    i32.add
    i32.store
    i32.const 0
  )

  (func $vec_concat_i32 (param $a i32) (param $b i32) (param $elem_ref i32) (result i32)
    (local $len_a i32)
    (local $len_b i32)
    (local $out i32)
    (local $i i32)
    (local $v i32)
    local.get $a
    i32.load
    local.set $len_a
    local.get $b
    i32.load
    local.set $len_b
    local.get $len_a
    local.get $len_b
    i32.add
    local.get $elem_ref
    call $vec_new_i32
    local.set $out
    local.get $out
    i32.const 0
    i32.store
    i32.const 0
    local.set $i
    block $done_a
      loop $copy_a
        local.get $i
        local.get $len_a
        i32.ge_s
        br_if $done_a
        local.get $a
        local.get $i
        call $vec_get_i32
        local.set $v
        local.get $out
        local.get $v
        call $vec_push_i32
        drop
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy_a
      end
    end
    i32.const 0
    local.set $i
    block $done_b
      loop $copy_b
        local.get $i
        local.get $len_b
        i32.ge_s
        br_if $done_b
        local.get $b
        local.get $i
        call $vec_get_i32
        local.set $v
        local.get $out
        local.get $v
        call $vec_push_i32
        drop
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy_b
      end
    end
    local.get $out
  )

  (func $dec_mul (param $a i32) (param $b i32) (result i32)
    local.get $a
    i64.extend_i32_s
    local.get $b
    i64.extend_i32_s
    i64.mul
    i64.const __DECIMAL_SCALE__
    i64.div_s
    i32.wrap_i64
  )

  (func $dec_div (param $a i32) (param $b i32) (result i32)
    local.get $a
    i64.extend_i32_s
    i64.const __DECIMAL_SCALE__
    i64.mul
    local.get $b
    i64.extend_i32_s
    i64.div_s
    i32.wrap_i64
  )

  (func $dec_mod (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $a
    local.get $b
    i32.div_s
    local.get $b
    i32.mul
    i32.sub
  )

  (func $dec_from_int (param $a i32) (result i32)
    local.get $a
    i64.extend_i32_s
    i64.const __DECIMAL_SCALE__
    i64.mul
    i32.wrap_i64
  )

  (func $dec_to_int (param $a i32) (result i32)
    local.get $a
    i32.const __DECIMAL_SCALE__
    i32.div_s
  )

  (func $vec_set_i32 (param $ptr i32) (param $idx i32) (param $v i32) (result i32)
    (local $len i32)
    (local $cap i32)
    (local $addr i32)
    (local $elem_ref i32)
    (local $old i32)
    ;; __DBG_RC_VEC_SET_INC__
    ;; __DBG_RC_VEC_SET_PTR_CHECK__
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    local.set $cap
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $elem_ref
    ;; __DBG_RC_VEC_SET_ELEM_REF__

    local.get $idx
    local.get $len
    i32.eq
    if
      ;; __DBG_RC_VEC_SET_APPEND_PATH__
      local.get $len
      local.get $cap
      i32.lt_s
      i32.eqz
      if
        local.get $ptr
        call $vec_grow_i32
        drop
      end
      local.get $elem_ref
      i32.const 0
      i32.ne
      if
        ;; __DBG_RC_VEC_SET_V_RC_BEFORE_RETAIN__
        ;; __DBG_RC_SET_VALUE_CHECK__
        local.get $v
        call $rc_retain
        drop
      end
      local.get $ptr
      i32.const 16
      i32.add
      i32.load
      local.get $len
      i32.const 4
      i32.mul
      i32.add
      local.set $addr
      local.get $addr
      local.get $v
      i32.store
      local.get $ptr
      local.get $len
      i32.const 1
      i32.add
      i32.store
      i32.const 0
      return
    end

    local.get $idx
    i32.const 0
    i32.ge_s
    local.get $idx
    local.get $len
    i32.lt_s
    i32.and
    if
      ;; __DBG_RC_VEC_SET_REPLACE_PATH__
      local.get $ptr
      i32.const 16
      i32.add
      i32.load
      local.get $idx
      i32.const 4
      i32.mul
      i32.add
      local.set $addr
      local.get $elem_ref
      i32.const 0
      i32.ne
      if
        ;; __DBG_RC_VEC_SET_V_RC_BEFORE_RETAIN__
        local.get $addr
        i32.load
        local.set $old
        ;; __DBG_RC_SET_OLD_CHECK__
        local.get $old
        local.get $v
        i32.ne
        if
          ;; __DBG_RC_VEC_SET_OLD_RC_HIST__
          local.get $v
          call $rc_retain
          drop
          local.get $old
          call $rc_release
          drop
        end
      end
      local.get $addr
      local.get $v
      i32.store
      i32.const 0
      return
    end

    unreachable
  )

  (func $vec_pop_i32 (param $ptr i32) (result i32)
    (local $len i32)
    (local $elem_ref i32)
    (local $addr i32)
    (local $v i32)
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $elem_ref
    local.get $len
    i32.const 0
    i32.gt_s
    if
      local.get $elem_ref
      i32.const 0
      i32.ne
      if
        local.get $ptr
        i32.const 16
        i32.add
        i32.load
        local.get $len
        i32.const 1
        i32.sub
        i32.const 4
        i32.mul
        i32.add
        local.set $addr
        local.get $addr
        i32.load
        local.set $v
        local.get $v
        call $rc_release
        drop
      end
      local.get $ptr
      local.get $len
      i32.const 1
      i32.sub
      i32.store
    end
    i32.const 0
  )

  (func $vec_slice_i32 (param $ptr i32) (param $start i32) (result i32)
    (local $len i32)
    (local $new_len i32)
    (local $i i32)
    (local $out i32)
    (local $elem_ref i32)
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $elem_ref

    local.get $start
    i32.const 0
    i32.le_s
    if (result i32)
      local.get $ptr
    else
      local.get $start
      local.get $len
      i32.ge_s
      if (result i32)
        i32.const 0
        local.get $elem_ref
        call $vec_new_i32
      else
        local.get $len
        local.get $start
        i32.sub
        local.set $new_len
        local.get $new_len
        local.get $elem_ref
        call $vec_new_i32
        local.set $out
        i32.const 0
        local.set $i
        block $done
          loop $copy
            local.get $i
            local.get $new_len
            i32.ge_s
            br_if $done
            local.get $out
            local.get $i
            local.get $ptr
            local.get $start
            local.get $i
            i32.add
            call $vec_get_i32
            call $vec_set_i32
            drop
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $copy
          end
        end
        local.get $out
      end
    end
  )
"#
    );
    out.push_str(
        r#"
  (export "$alloc" (func $alloc))
  (export "$rc_retain" (func $rc_retain))
  (export "$rc_release" (func $rc_release))
  (export "alloc" (func $alloc))
  (export "rc_retain" (func $rc_retain))
  (export "rc_release" (func $rc_release))
  ;; __DBG_RC_EXPORTS__
"#
    );
    if apply_arities.contains(&0) {
        out.push_str(&emit_high_arity_apply_i32(0, fn_ids, fn_sigs, closure_defs));
    }
    if apply_arities.contains(&1) {
        out.push_str(
            "  (func $apply1_i32 (param $f i32) (param $a i32) (result i32)\n    (local $clo i32)\n"
        );
        let apply1_closures = closure_defs
            .values()
            .filter_map(|def| {
                let fid = *fn_ids.get(&def.name)?;
                let (ps, ret) = fn_sigs.get(&def.name)?;
                if
                    def.user_arity != 1 ||
                    !is_i32ish_type(ret) ||
                    ps.len() != def.captures.len() + 1
                {
                    return None;
                }
                if !ps.iter().all(is_i32ish_type) {
                    return None;
                }
                Some((fid, def.name.clone(), def.captures.len()))
            })
            .collect::<Vec<_>>();
        let apply1_partial_closures = closure_defs
            .values()
            .filter_map(|def| {
                let fid = *fn_ids.get(&def.name)?;
                let (ps, ret) = fn_sigs.get(&def.name)?;
                if def.user_arity <= 1 || !is_i32ish_type(ret) {
                    return None;
                }
                if ps.len() != def.captures.len() + def.user_arity {
                    return None;
                }
                if !ps.iter().all(is_i32ish_type) {
                    return None;
                }
                let helper_name = format!("__partial_dyn_{}_1", def.user_arity);
                let helper_id = *fn_ids.get(&helper_name)?;
                let first_param_is_ref = ps
                    .get(def.captures.len())
                    .map(is_ref_type)
                    .unwrap_or(false);
                Some((fid, helper_id, first_param_is_ref))
            })
            .collect::<Vec<_>>();
        if !apply1_closures.is_empty() || !apply1_partial_closures.is_empty() {
            out.push_str(
                "    local.get $f\n    i32.const -2147483648\n    i32.and\n    i32.const -2147483648\n    i32.eq\n    if (result i32)\n"
            );
            for (fid, name, cap_len) in &apply1_closures {
                out.push_str(
                    &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
                );
                for i in 0..*cap_len {
                    out.push_str(
                        &format!("        local.get $f\n        i32.const {}\n        call $closure_get\n", i)
                    );
                }
                out.push_str(&format!("        local.get $a\n        call ${}\n", ident(name)));
                out.push_str("      else\n");
            }
            for (fid, helper_id, first_param_is_ref) in &apply1_partial_closures {
                out.push_str(
                    &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
                );
                out.push_str(
                    &format!("        i32.const {}\n        i32.const 2\n        call $closure_new\n        local.set $clo\n", helper_id)
                );
                out.push_str(
                    "        local.get $clo\n        i32.const 0\n        local.get $f\n        call $closure_set_fun\n        drop\n"
                );
                out.push_str("        local.get $clo\n        i32.const 1\n        local.get $a\n");
                if *first_param_is_ref {
                    out.push_str("        call $closure_set_ref\n");
                } else {
                    out.push_str("        call $closure_set\n");
                }
                out.push_str("        drop\n");
                out.push_str("        local.get $clo\n");
                out.push_str("      else\n");
            }
            out.push_str("        unreachable\n");
            for _ in 0..apply1_closures.len() + apply1_partial_closures.len() {
                out.push_str("      end\n");
            }
            out.push_str("    else\n");
        }
        let mut apply1_open_ends = 0usize;
        for (name, tag) in fn_ids {
            if let Some((ps, ret)) = fn_sigs.get(name) {
                if ps.len() == 1 && is_i32ish_type(&ps[0]) && is_i32ish_type(ret) {
                    apply1_open_ends += 1;
                    out.push_str(
                        &format!(
                            "    local.get $f\n    i32.const {}\n    i32.eq\n    if (result i32)\n      local.get $a\n      call ${}\n    else\n",
                            tag,
                            ident(name)
                        )
                    );
                }
            }
        }
        for (name, tag) in fn_ids {
            if let Some((ps, ret)) = fn_sigs.get(name) {
                if ps.len() > 1 && ps.iter().all(is_i32ish_type) && is_i32ish_type(ret) {
                    let helper_name = format!("__partial_dyn_{}_1", ps.len());
                    if let Some(helper_id) = fn_ids.get(&helper_name) {
                        let first_param_store = ps
                            .first()
                            .map(closure_store_op_for_type)
                            .unwrap_or("closure_set");
                        apply1_open_ends += 1;
                        out.push_str(
                            &format!(
                                "    local.get $f\n    i32.const {}\n    i32.eq\n    if (result i32)\n      i32.const {}\n      i32.const 2\n      call $closure_new\n      local.set $clo\n      local.get $clo\n      i32.const 0\n      i32.const {}\n      call $closure_set_fun\n      drop\n      local.get $clo\n      i32.const 1\n      local.get $a\n      call ${}\n      drop\n      local.get $clo\n    else\n",
                                tag,
                                helper_id,
                                tag,
                                first_param_store
                            )
                        );
                    }
                }
            }
        }
        let builtin_apply1_partial_tags = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 21, 25, 26, 27, 28, 29, 30,
            31, 32, 33, 34, 37,
        ];
        for tag in builtin_apply1_partial_tags {
            if let Some(arity) = builtin_tag_arity(tag) {
                if arity > 1 {
                    let helper_name = format!("__partial_dyn_{}_1", arity);
                    if let Some(helper_id) = fn_ids.get(&helper_name) {
                        let first_param_store = if builtin_tag_first_param_is_ref(tag) {
                            "closure_set_ref"
                        } else {
                            "closure_set"
                        };
                        apply1_open_ends += 1;
                        out.push_str(
                            &format!(
                                "    local.get $f\n    i32.const {}\n    i32.eq\n    if (result i32)\n      i32.const {}\n      i32.const 2\n      call $closure_new\n      local.set $clo\n      local.get $clo\n      i32.const 0\n      i32.const {}\n      call $closure_set_fun\n      drop\n      local.get $clo\n      i32.const 1\n      local.get $a\n      call ${}\n      drop\n      local.get $clo\n    else\n",
                                tag,
                                helper_id,
                                tag,
                                first_param_store
                            )
                        );
                    }
                }
            }
        }
        out.push_str(
            r#"
    local.get $f
    i32.const 20
    i32.eq
    if (result i32)
      local.get $a
      call $vec_len
    else
    local.get $f
    i32.const 22
    i32.eq
    if (result i32)
      local.get $a
      call $vec_pop_i32
    else
    local.get $f
    i32.const 23
    i32.eq
    if (result i32)
      local.get $a
      call $tuple_fst
    else
    local.get $f
    i32.const 24
    i32.eq
    if (result i32)
      local.get $a
      call $tuple_snd
    else
    local.get $f
    i32.const 18
    i32.eq
    if (result i32)
      local.get $a
      i32.eqz
    else
      local.get $f
      i32.const 19
      i32.eq
      if (result i32)
        local.get $a
        i32.const -1
        i32.xor
    else
        local.get $f
        i32.const 35
        i32.eq
        if (result i32)
          local.get $a
          call $dec_from_int
        else
          local.get $f
          i32.const 36
          i32.eq
          if (result i32)
            local.get $a
            call $dec_to_int
          else
        unreachable
          end
        end
      end
    end
    end
    end
    end
    end
    "#
        );
        for _ in 0..apply1_open_ends {
            out.push_str("    end\n");
        }
        if !apply1_closures.is_empty() || !apply1_partial_closures.is_empty() {
            out.push_str("    end\n");
        }
        out.push_str("  )\n");
    }
    if apply_arities.contains(&2) {
        out.push_str(
            "  (func $apply2_i32 (param $f i32) (param $a i32) (param $b i32) (result i32)\n"
        );
        let apply2_closures = closure_defs
            .values()
            .filter_map(|def| {
                let fid = *fn_ids.get(&def.name)?;
                let (ps, ret) = fn_sigs.get(&def.name)?;
                if
                    def.user_arity != 2 ||
                    !is_i32ish_type(ret) ||
                    ps.len() != def.captures.len() + 2
                {
                    return None;
                }
                if !ps.iter().all(is_i32ish_type) {
                    return None;
                }
                Some((fid, def.name.clone(), def.captures.len()))
            })
            .collect::<Vec<_>>();
        if !apply2_closures.is_empty() {
            out.push_str(
                "    local.get $f\n    i32.const -2147483648\n    i32.and\n    i32.const -2147483648\n    i32.eq\n    if (result i32)\n"
            );
            for (fid, name, cap_len) in &apply2_closures {
                out.push_str(
                    &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
                );
                for i in 0..*cap_len {
                    out.push_str(
                        &format!("        local.get $f\n        i32.const {}\n        call $closure_get\n", i)
                    );
                }
                out.push_str(
                    &format!(
                        "        local.get $a\n        local.get $b\n        call ${}\n",
                        ident(name)
                    )
                );
                out.push_str("      else\n");
            }
            out.push_str("        unreachable\n");
            for _ in 0..apply2_closures.len() {
                out.push_str("      end\n");
            }
            out.push_str("    else\n");
        }
        let mut apply2_open_ends = 0usize;
        for (name, tag) in fn_ids {
            if let Some((ps, ret)) = fn_sigs.get(name) {
                if
                    ps.len() == 2 &&
                    is_i32ish_type(&ps[0]) &&
                    is_i32ish_type(&ps[1]) &&
                    is_i32ish_type(ret)
                {
                    apply2_open_ends += 1;
                    out.push_str(
                        &format!(
                            "    local.get $f\n    i32.const {}\n    i32.eq\n    if (result i32)\n      local.get $a\n      local.get $b\n      call ${}\n    else\n",
                            tag,
                            ident(name)
                        )
                    );
                }
            }
        }
        out.push_str(
            r#"
    local.get $f
    i32.const 1
    i32.eq
    if (result i32)
      local.get $a
      local.get $b
      i32.add
    else
      local.get $f
      i32.const 2
      i32.eq
      if (result i32)
        local.get $a
        local.get $b
        i32.sub
      else
        local.get $f
        i32.const 3
        i32.eq
        if (result i32)
          local.get $a
          local.get $b
          i32.mul
        else
          local.get $f
          i32.const 4
          i32.eq
          if (result i32)
            local.get $a
            local.get $b
            i32.div_s
          else
            local.get $f
            i32.const 5
            i32.eq
            if (result i32)
              local.get $a
              local.get $b
              i32.rem_s
            else
              local.get $f
              i32.const 6
              i32.eq
              if (result i32)
                local.get $a
                local.get $b
                i32.eq
              else
                local.get $f
                i32.const 7
                i32.eq
                if (result i32)
                  local.get $a
                  local.get $b
                  i32.lt_s
                else
                  local.get $f
                  i32.const 8
                  i32.eq
                  if (result i32)
                    local.get $a
                    local.get $b
                    i32.gt_s
                  else
                    local.get $f
                    i32.const 9
                    i32.eq
                    if (result i32)
                      local.get $a
                      local.get $b
                      i32.le_s
                    else
                      local.get $f
                      i32.const 10
                      i32.eq
                      if (result i32)
                        local.get $a
                        local.get $b
                        i32.ge_s
                      else
                        local.get $f
                        i32.const 11
                        i32.eq
                        if (result i32)
                          local.get $a
                          local.get $b
                          i32.and
                        else
                          local.get $f
                          i32.const 12
                          i32.eq
                          if (result i32)
                            local.get $a
                            local.get $b
                            i32.or
                          else
                            local.get $f
                            i32.const 13
                            i32.eq
                            if (result i32)
                              local.get $a
                              local.get $b
                              i32.xor
                            else
                              local.get $f
                              i32.const 14
                              i32.eq
                              if (result i32)
                                local.get $a
                                local.get $b
                                i32.or
                              else
                                local.get $f
                                i32.const 15
                                i32.eq
                                if (result i32)
                                  local.get $a
                                  local.get $b
                                  i32.and
                                else
                                  local.get $f
                                  i32.const 16
                                  i32.eq
                                  if (result i32)
                                    local.get $a
                                    local.get $b
                                    i32.shl
                                  else
                                    local.get $f
                                    i32.const 17
                                    i32.eq
                                    if (result i32)
                                      local.get $a
                                      local.get $b
                                      i32.shr_s
                                    else
                                      local.get $f
                                      i32.const 25
                                      i32.eq
                                      if (result i32)
                                        local.get $a
                                        local.get $b
                                        i32.add
                                      else
                                        local.get $f
                                        i32.const 26
                                        i32.eq
                                        if (result i32)
                                          local.get $a
                                          local.get $b
                                          i32.sub
                                        else
                                          local.get $f
                                          i32.const 27
                                          i32.eq
                                          if (result i32)
                                            local.get $a
                                            local.get $b
                                            call $dec_mul
                                          else
                                            local.get $f
                                            i32.const 28
                                            i32.eq
                                            if (result i32)
                                              local.get $a
                                              local.get $b
                                              call $dec_div
                                            else
                                              local.get $f
                                              i32.const 29
                                              i32.eq
                                              if (result i32)
                                                local.get $a
                                                local.get $b
                                                call $dec_mod
                                              else
                                                local.get $f
                                                i32.const 30
                                                i32.eq
                                                if (result i32)
                                                  local.get $a
                                                  local.get $b
                                                  i32.eq
                                                else
                                                  local.get $f
                                                  i32.const 31
                                                  i32.eq
                                                  if (result i32)
                                                    local.get $a
                                                    local.get $b
                                                    i32.lt_s
                                                  else
                                                    local.get $f
                                                    i32.const 32
                                                    i32.eq
                                                    if (result i32)
                                                      local.get $a
                                                      local.get $b
                                                      i32.gt_s
                                                    else
                                                      local.get $f
                                                      i32.const 33
                                                      i32.eq
                                                      if (result i32)
                                                        local.get $a
                                                        local.get $b
                                                        i32.le_s
                                                      else
                                                        local.get $f
                                                        i32.const 34
                                                        i32.eq
                                                        if (result i32)
                                                          local.get $a
                                                          local.get $b
                                                          i32.ge_s
                                                        else
                                                          local.get $f
                                                          i32.const 37
                                                          i32.eq
                                                          if (result i32)
                                                            local.get $a
                                                            local.get $b
                                                            local.get $a
                                                            i32.const 12
                                                            i32.add
                                                            i32.load
                                                            call $vec_concat_i32
                                                          else
                                                            unreachable
                                                          end
                                                        end
                                                      end
                                                    end
                                                  end
                                                end
                                              end
                                            end
                                          end
                                        end
                                      end
                                    end
    "#
        );
        for _ in 0..apply2_open_ends {
            out.push_str("                                    end\n");
        }
        if !apply2_closures.is_empty() {
            out.push_str("    end\n");
        }
        out.push_str(
            r#"
                                  end
                                end
                              end
                            end
                          end
                        end
                      end
                    end
                  end
                end
              end
            end
          end
        end
      end
    end
  "#
        );
        out.push_str("  )\n");
    }

    if apply_arities.contains(&3) {
        out.push_str(
            "  (func $apply3_i32 (param $f i32) (param $a i32) (param $b i32) (param $c i32) (result i32)\n"
        );
        let apply3_closures = closure_defs
            .values()
            .filter_map(|def| {
                let fid = *fn_ids.get(&def.name)?;
                let (ps, ret) = fn_sigs.get(&def.name)?;
                if
                    def.user_arity != 3 ||
                    !is_i32ish_type(ret) ||
                    ps.len() != def.captures.len() + 3
                {
                    return None;
                }
                if !ps.iter().all(is_i32ish_type) {
                    return None;
                }
                Some((fid, def.name.clone(), def.captures.len()))
            })
            .collect::<Vec<_>>();
        if !apply3_closures.is_empty() {
            out.push_str(
                "    local.get $f\n    i32.const -2147483648\n    i32.and\n    i32.const -2147483648\n    i32.eq\n    if (result i32)\n"
            );
            for (fid, name, cap_len) in &apply3_closures {
                out.push_str(
                    &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
                );
                for i in 0..*cap_len {
                    out.push_str(
                        &format!("        local.get $f\n        i32.const {}\n        call $closure_get\n", i)
                    );
                }
                out.push_str(
                    &format!(
                        "        local.get $a\n        local.get $b\n        local.get $c\n        call ${}\n",
                        ident(name)
                    )
                );
                out.push_str("      else\n");
            }
            out.push_str("        unreachable\n");
            for _ in 0..apply3_closures.len() {
                out.push_str("      end\n");
            }
            out.push_str("    else\n");
        }
        out.push_str(
            "    local.get $f\n    i32.const 21\n    i32.eq\n    if (result i32)\n      local.get $a\n      local.get $b\n      local.get $c\n      call $vec_set_i32\n    else\n"
        );
        for (name, tag) in fn_ids {
            if let Some((ps, ret)) = fn_sigs.get(name) {
                if
                    ps.len() == 3 &&
                    is_i32ish_type(&ps[0]) &&
                    is_i32ish_type(&ps[1]) &&
                    is_i32ish_type(&ps[2]) &&
                    is_i32ish_type(ret)
                {
                    out.push_str(
                        &format!(
                            "    local.get $f\n    i32.const {}\n    i32.eq\n    if (result i32)\n      local.get $a\n      local.get $b\n      local.get $c\n      call ${}\n    else\n",
                            tag,
                            ident(name)
                        )
                    );
                }
            }
        }
        out.push_str(
            "      local.get $f\n      local.get $a\n      call $apply1_i32\n      local.get $b\n      call $apply1_i32\n      local.get $c\n      call $apply1_i32\n"
        );
        for (name, _tag) in fn_ids {
            if let Some((ps, ret)) = fn_sigs.get(name) {
                if
                    ps.len() == 3 &&
                    is_i32ish_type(&ps[0]) &&
                    is_i32ish_type(&ps[1]) &&
                    is_i32ish_type(&ps[2]) &&
                    is_i32ish_type(ret)
                {
                    out.push_str("    end\n");
                }
            }
        }
        out.push_str("    end\n");
        if !apply3_closures.is_empty() {
            out.push_str("    end\n");
        }
        out.push_str("  )\n");
    }

    let mut extra_apply_arities = apply_arities
        .iter()
        .copied()
        .filter(|n| *n > 3)
        .collect::<Vec<_>>();
    extra_apply_arities.sort_unstable();
    for arity in extra_apply_arities {
        out.push_str(&emit_high_arity_apply_i32(arity, fn_ids, fn_sigs, closure_defs));
    }
    let debug_rc_enabled = cfg!(feature = "debug-rc");
    out = out.replace("__VEC_MIN_CAP__", &vec_min_cap.to_string());
    out = out.replace("__VEC_GROWTH_NUM__", &vec_growth_num.to_string());
    out = out.replace("__VEC_GROWTH_DEN__", &vec_growth_den.to_string());
    out = out.replace("__DECIMAL_SCALE__", &decimal_scale_i64().to_string());
    out = out.replace(";; __VEC_GET_BOUNDS_CHECK__", if vec_bounds_check_enabled {
        r#"local.get $idx
    i32.const 0
    i32.lt_s
    if
      unreachable
    end
    local.get $ptr
    i32.load
    local.set $len
    local.get $idx
    local.get $len
    i32.ge_s
    if
      unreachable
    end"#
    } else {
        ""
    });

    let replacements = [
        (
            ";; __DBG_RC_GLOBALS__",
            if debug_rc_enabled {
                r#"
  (global $dbg_alloc_count (mut i64) (i64.const 0))
  (global $dbg_free_count (mut i64) (i64.const 0))
  (global $dbg_retain_count (mut i64) (i64.const 0))
  (global $dbg_release_count (mut i64) (i64.const 0))
  (global $dbg_vec_new_count (mut i64) (i64.const 0))
  (global $dbg_vec_set_count (mut i64) (i64.const 0))
  (global $dbg_bad_vec_set_ptr (mut i64) (i64.const 0))
  (global $dbg_bad_ref_value (mut i64) (i64.const 0))
  (global $dbg_bad_ref_old (mut i64) (i64.const 0))
  (global $dbg_vec_set_elem_ref_0 (mut i64) (i64.const 0))
  (global $dbg_vec_set_elem_ref_1 (mut i64) (i64.const 0))
  (global $dbg_rc_release_vec_gt0 (mut i64) (i64.const 0))
  (global $dbg_rc_release_vec_free (mut i64) (i64.const 0))
  (global $dbg_vec_set_append_path (mut i64) (i64.const 0))
  (global $dbg_vec_set_replace_path (mut i64) (i64.const 0))
  (global $dbg_rc_release_vec_rc_eq_1 (mut i64) (i64.const 0))
  (global $dbg_rc_release_vec_rc_ge_2 (mut i64) (i64.const 0))
  (global $dbg_vec_set_old_rc_eq_1 (mut i64) (i64.const 0))
  (global $dbg_vec_set_old_rc_ge_2 (mut i64) (i64.const 0))
  (global $dbg_vec_set_old_not_vec (mut i64) (i64.const 0))
  (global $dbg_tmp_release_exec (mut i64) (i64.const 0))
  (global $dbg_tmp_release_skip (mut i64) (i64.const 0))
  (global $dbg_vec_set_v_rc_eq_1 (mut i64) (i64.const 0))
  (global $dbg_vec_set_v_rc_ge_2 (mut i64) (i64.const 0))
  (global $dbg_vec_set_v_not_vec (mut i64) (i64.const 0))
  (global $dbg_tmp_release_post_rc_eq_1 (mut i64) (i64.const 0))
  (global $dbg_tmp_release_post_rc_other (mut i64) (i64.const 0))
  (global $dbg_tmp_release_post_not_vec (mut i64) (i64.const 0))
  (global $dbg_rc_release_reject_not_vec (mut i64) (i64.const 0))
  (global $dbg_rc_release_take_vec_path (mut i64) (i64.const 0))"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_HELPERS__",
            if debug_rc_enabled {
                r#"
  (func $dbg_is_managed_ptr (param $p i32) (result i32)
    local.get $p
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $p
      i32.const 65536
      i32.lt_u
      if (result i32)
        i32.const 1
      else
      local.get $p
      i32.const -2147483648
      i32.and
      i32.const -2147483648
      i32.eq
      if (result i32)
        local.get $p
        i32.const 2147483647
        i32.and
        i32.const 65536
        i32.ge_u
        local.get $p
        i32.const 2147483647
        i32.and
        memory.size
        i32.const 16
        i32.shl
        i32.lt_u
        i32.and
      else
        local.get $p
        call $is_vec_ptr
      end
      end
    end
  )"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_ALLOC_INC__",
            if debug_rc_enabled {
                r#"global.get $dbg_alloc_count
    i64.const 1
    i64.add
    global.set $dbg_alloc_count"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_FREE_INC__",
            if debug_rc_enabled {
                r#"global.get $dbg_free_count
    i64.const 1
    i64.add
    global.set $dbg_free_count"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_RETAIN_INC__",
            if debug_rc_enabled {
                r#"global.get $dbg_retain_count
    i64.const 1
    i64.add
    global.set $dbg_retain_count"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_RELEASE_INC__",
            if debug_rc_enabled {
                r#"global.get $dbg_release_count
    i64.const 1
    i64.add
    global.set $dbg_release_count"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_VEC_NEW_INC__",
            if debug_rc_enabled {
                r#"global.get $dbg_vec_new_count
    i64.const 1
    i64.add
    global.set $dbg_vec_new_count"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_VEC_SET_INC__",
            if debug_rc_enabled {
                r#"global.get $dbg_vec_set_count
    i64.const 1
    i64.add
    global.set $dbg_vec_set_count"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_VEC_SET_ELEM_REF__",
            if debug_rc_enabled {
                r#"local.get $elem_ref
    i32.eqz
    if
      global.get $dbg_vec_set_elem_ref_0
      i64.const 1
      i64.add
      global.set $dbg_vec_set_elem_ref_0
    else
      global.get $dbg_vec_set_elem_ref_1
      i64.const 1
      i64.add
      global.set $dbg_vec_set_elem_ref_1
    end"#
            } else {
                ""
            },
        ),
        (";; __DBG_RC_RELEASE_VEC_DEC__", if debug_rc_enabled { "" } else { "" }),
        (
            ";; __DBG_RC_RELEASE_VEC_RC_HIST__",
            if debug_rc_enabled {
                r#"local.get $rc
    i32.const 1
    i32.eq
    if
      global.get $dbg_rc_release_vec_rc_eq_1
      i64.const 1
      i64.add
      global.set $dbg_rc_release_vec_rc_eq_1
    else
      global.get $dbg_rc_release_vec_rc_ge_2
      i64.const 1
      i64.add
      global.set $dbg_rc_release_vec_rc_ge_2
    end"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_RELEASE_VEC_GT0__",
            if debug_rc_enabled {
                r#"global.get $dbg_rc_release_vec_gt0
      i64.const 1
      i64.add
      global.set $dbg_rc_release_vec_gt0"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_RELEASE_VEC_FREE_PATH__",
            if debug_rc_enabled {
                r#"global.get $dbg_rc_release_vec_free
    i64.const 1
    i64.add
    global.set $dbg_rc_release_vec_free"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_VEC_SET_APPEND_PATH__",
            if debug_rc_enabled {
                r#"global.get $dbg_vec_set_append_path
      i64.const 1
      i64.add
      global.set $dbg_vec_set_append_path"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_VEC_SET_REPLACE_PATH__",
            if debug_rc_enabled {
                r#"global.get $dbg_vec_set_replace_path
      i64.const 1
      i64.add
      global.set $dbg_vec_set_replace_path"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_VEC_SET_OLD_RC_HIST__",
            if debug_rc_enabled {
                r#"local.get $old
          call $is_vec_ptr
          if
            local.get $old
            i32.const 8
            i32.add
            i32.load
            i32.const 1
            i32.eq
            if
              global.get $dbg_vec_set_old_rc_eq_1
              i64.const 1
              i64.add
              global.set $dbg_vec_set_old_rc_eq_1
            else
              global.get $dbg_vec_set_old_rc_ge_2
              i64.const 1
              i64.add
              global.set $dbg_vec_set_old_rc_ge_2
            end
          else
            global.get $dbg_vec_set_old_not_vec
            i64.const 1
            i64.add
            global.set $dbg_vec_set_old_not_vec
          end"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_VEC_SET_V_RC_BEFORE_RETAIN__",
            if debug_rc_enabled {
                r#"local.get $v
        call $is_vec_ptr
        if
          local.get $v
          i32.const 8
          i32.add
          i32.load
          i32.const 1
          i32.eq
          if
            global.get $dbg_vec_set_v_rc_eq_1
            i64.const 1
            i64.add
            global.set $dbg_vec_set_v_rc_eq_1
          else
            global.get $dbg_vec_set_v_rc_ge_2
            i64.const 1
            i64.add
            global.set $dbg_vec_set_v_rc_ge_2
          end
        else
          global.get $dbg_vec_set_v_not_vec
          i64.const 1
          i64.add
          global.set $dbg_vec_set_v_not_vec
        end"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_VEC_SET_PTR_CHECK__",
            if debug_rc_enabled {
                r#"local.get $ptr
    call $is_vec_ptr
    i32.eqz
    if
      global.get $dbg_bad_vec_set_ptr
      i64.const 1
      i64.add
      global.set $dbg_bad_vec_set_ptr
    end"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_SET_VALUE_CHECK__",
            if debug_rc_enabled {
                r#"local.get $v
        call $dbg_is_managed_ptr
        i32.eqz
        if
          global.get $dbg_bad_ref_value
          i64.const 1
          i64.add
          global.set $dbg_bad_ref_value
        end"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_SET_OLD_CHECK__",
            if debug_rc_enabled {
                r#"local.get $old
        call $dbg_is_managed_ptr
        i32.eqz
        if
          global.get $dbg_bad_ref_old
          i64.const 1
          i64.add
          global.set $dbg_bad_ref_old
        end"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_RELEASE_REJECT_NOT_VEC__",
            if debug_rc_enabled {
                r#"global.get $dbg_rc_release_reject_not_vec
      i64.const 1
      i64.add
      global.set $dbg_rc_release_reject_not_vec"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_RELEASE_TAKE_VEC_PATH__",
            if debug_rc_enabled {
                r#"global.get $dbg_rc_release_take_vec_path
    i64.const 1
    i64.add
    global.set $dbg_rc_release_take_vec_path"#
            } else {
                ""
            },
        ),
        (
            ";; __DBG_RC_EXPORTS__",
            if debug_rc_enabled {
                r#"
  (export "dbg_alloc_count" (global $dbg_alloc_count))
  (export "dbg_free_count" (global $dbg_free_count))
  (export "dbg_retain_count" (global $dbg_retain_count))
  (export "dbg_release_count" (global $dbg_release_count))
  (export "dbg_vec_new_count" (global $dbg_vec_new_count))
  (export "dbg_vec_set_count" (global $dbg_vec_set_count))
  (export "dbg_bad_vec_set_ptr" (global $dbg_bad_vec_set_ptr))
  (export "dbg_bad_ref_value" (global $dbg_bad_ref_value))
  (export "dbg_bad_ref_old" (global $dbg_bad_ref_old))
  (export "dbg_vec_set_elem_ref_0" (global $dbg_vec_set_elem_ref_0))
  (export "dbg_vec_set_elem_ref_1" (global $dbg_vec_set_elem_ref_1))
  (export "dbg_rc_release_vec_gt0" (global $dbg_rc_release_vec_gt0))
  (export "dbg_rc_release_vec_free" (global $dbg_rc_release_vec_free))
  (export "dbg_vec_set_append_path" (global $dbg_vec_set_append_path))
  (export "dbg_vec_set_replace_path" (global $dbg_vec_set_replace_path))
  (export "dbg_rc_release_vec_rc_eq_1" (global $dbg_rc_release_vec_rc_eq_1))
  (export "dbg_rc_release_vec_rc_ge_2" (global $dbg_rc_release_vec_rc_ge_2))
  (export "dbg_vec_set_old_rc_eq_1" (global $dbg_vec_set_old_rc_eq_1))
  (export "dbg_vec_set_old_rc_ge_2" (global $dbg_vec_set_old_rc_ge_2))
  (export "dbg_vec_set_old_not_vec" (global $dbg_vec_set_old_not_vec))
  (export "dbg_tmp_release_exec" (global $dbg_tmp_release_exec))
  (export "dbg_tmp_release_skip" (global $dbg_tmp_release_skip))
  (export "dbg_vec_set_v_rc_eq_1" (global $dbg_vec_set_v_rc_eq_1))
  (export "dbg_vec_set_v_rc_ge_2" (global $dbg_vec_set_v_rc_ge_2))
  (export "dbg_vec_set_v_not_vec" (global $dbg_vec_set_v_not_vec))
  (export "dbg_tmp_release_post_rc_eq_1" (global $dbg_tmp_release_post_rc_eq_1))
  (export "dbg_tmp_release_post_rc_other" (global $dbg_tmp_release_post_rc_other))
  (export "dbg_tmp_release_post_not_vec" (global $dbg_tmp_release_post_not_vec))
  (export "dbg_rc_release_reject_not_vec" (global $dbg_rc_release_reject_not_vec))
  (export "dbg_rc_release_take_vec_path" (global $dbg_rc_release_take_vec_path))"#
            } else {
                ""
            },
        ),
    ];
    for (needle, replacement) in replacements {
        out = out.replace(needle, replacement);
    }
    out
}

fn emit_builtin(op: &str, node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    fn emit_int_div_zero_check(rhs_local: usize) -> String {
        format!(
            "local.get {rhs_local}\ni32.eqz\nif\n{}\nend",
            emit_guard_trap_wat(DBG_GUARD_TRAP_INT_DIV_ZERO)
        )
    }

    fn emit_float_div_zero_check(rhs_local: usize) -> String {
        format!(
            "local.get {rhs_local}\ni32.eqz\nif\n{}\nend",
            emit_guard_trap_wat(DBG_GUARD_TRAP_FLOAT_DIV_ZERO)
        )
    }

    fn emit_float_overflow_or_nan_check(result_local: usize) -> String {
        let _ = result_local;
        String::new()
    }

    fn emit_int_add_overflow_check(lhs_local: usize, rhs_local: usize, res_local: usize) -> String {
        format!(
            "local.get {lhs_local}\nlocal.get {res_local}\ni32.xor\nlocal.get {rhs_local}\nlocal.get {res_local}\ni32.xor\ni32.and\ni32.const 0\ni32.lt_s\nif\n{}\nend",
            emit_guard_trap_wat(DBG_GUARD_TRAP_INT_OVERFLOW_ADD)
        )
    }

    fn emit_int_sub_overflow_check(lhs_local: usize, rhs_local: usize, res_local: usize) -> String {
        format!(
            "local.get {lhs_local}\nlocal.get {rhs_local}\ni32.xor\nlocal.get {lhs_local}\nlocal.get {res_local}\ni32.xor\ni32.and\ni32.const 0\ni32.lt_s\nif\n{}\nend",
            emit_guard_trap_wat(DBG_GUARD_TRAP_INT_OVERFLOW_SUB)
        )
    }

    fn emit_int_mul_overflow_check(lhs_local: usize, rhs_local: usize, res_local: usize) -> String {
        format!(
            "local.get {rhs_local}\ni32.const 0\ni32.ne\nif\n  local.get {res_local}\n  local.get {rhs_local}\n  i32.div_s\n  local.get {lhs_local}\n  i32.ne\n  if\n{}\n  end\nend",
            emit_guard_trap_wat(DBG_GUARD_TRAP_INT_OVERFLOW_MUL)
        )
    }

    let checks = arithmetic_check_config();
    let lhs_local = ctx.tmp_i32;
    let rhs_local = ctx.tmp_i32 + 1;
    let res_local = ctx.tmp_i32 + 2;
    let a = node.children
        .get(1)
        .ok_or_else(|| format!("Missing lhs for {}", op))
        .and_then(|n| compile_expr(n, ctx))?;
    let b = node.children
        .get(2)
        .ok_or_else(|| format!("Missing rhs for {}", op))
        .and_then(|n| compile_expr(n, ctx))?;
    let code = match op {
        "+" | "+#" => {
            if checks.int_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.add\nlocal.set {res_local}\n{}\nlocal.get {res_local}",
                        emit_int_add_overflow_check(lhs_local, rhs_local, res_local)
                    )
                );
            }
            "i32.add"
        }
        "-" | "-#" => {
            if checks.int_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.sub\nlocal.set {res_local}\n{}\nlocal.get {res_local}",
                        emit_int_sub_overflow_check(lhs_local, rhs_local, res_local)
                    )
                );
            }
            "i32.sub"
        }
        "*" | "*#" => {
            if checks.int_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.mul\nlocal.set {res_local}\n{}\nlocal.get {res_local}",
                        emit_int_mul_overflow_check(lhs_local, rhs_local, res_local)
                    )
                );
            }
            "i32.mul"
        }
        "/" | "/#" => {
            if checks.div_zero_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\n{}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.div_s",
                        emit_int_div_zero_check(rhs_local)
                    )
                );
            }
            "i32.div_s"
        }
        "mod" => {
            if checks.div_zero_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\n{}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.rem_s",
                        emit_int_div_zero_check(rhs_local)
                    )
                );
            }
            "i32.rem_s"
        }
        "=" | "=?" | "=#" => "i32.eq",
        "<" | "<#" => "i32.lt_s",
        ">" | ">#" => "i32.gt_s",
        "<=" | "<=#" => "i32.le_s",
        ">=" | ">=#" => "i32.ge_s",
        "and" => {
            return Ok(
                format!(
                    "{a}\n(if (result i32)\n  (then\n    {b}\n  )\n  (else\n    i32.const 0\n  )\n)"
                )
            );
        }
        "or" => {
            return Ok(
                format!(
                    "{a}\n(if (result i32)\n  (then\n    i32.const 1\n  )\n  (else\n    {b}\n  )\n)"
                )
            );
        }
        "^" => "i32.xor",
        "|" => "i32.or",
        "&" => "i32.and",
        "<<" => "i32.shl",
        ">>" => "i32.shr_s",
        "+." => {
            if checks.float_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.add\nlocal.set {res_local}\n{}\nlocal.get {res_local}",
                        emit_float_overflow_or_nan_check(res_local)
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ni32.add"));
        }
        "-." => {
            if checks.float_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.sub\nlocal.set {res_local}\n{}\nlocal.get {res_local}",
                        emit_float_overflow_or_nan_check(res_local)
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ni32.sub"));
        }
        "*." => {
            if checks.float_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ncall $dec_mul\nlocal.set {res_local}\n{}\nlocal.get {res_local}",
                        emit_float_overflow_or_nan_check(res_local)
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ncall $dec_mul"));
        }
        "/." => {
            if checks.div_zero_check || checks.float_overflow_check {
                let div_zero_check = if checks.div_zero_check {
                    format!("{}\n", emit_float_div_zero_check(rhs_local))
                } else {
                    String::new()
                };
                let overflow_check = if checks.float_overflow_check {
                    format!("{}\n", emit_float_overflow_or_nan_check(res_local))
                } else {
                    String::new()
                };
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\n{div_zero_check}local.get {lhs_local}\nlocal.get {rhs_local}\ncall $dec_div\nlocal.set {res_local}\n{overflow_check}local.get {res_local}"
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ncall $dec_div"));
        }
        "mod." => {
            if checks.div_zero_check || checks.float_overflow_check {
                let div_zero_check = if checks.div_zero_check {
                    format!("{}\n", emit_float_div_zero_check(rhs_local))
                } else {
                    String::new()
                };
                let overflow_check = if checks.float_overflow_check {
                    format!("{}\n", emit_float_overflow_or_nan_check(res_local))
                } else {
                    String::new()
                };
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\n{div_zero_check}local.get {lhs_local}\nlocal.get {rhs_local}\ncall $dec_mod\nlocal.set {res_local}\n{overflow_check}local.get {res_local}"
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ncall $dec_mod"));
        }
        "=." => {
            return Ok(format!("{a}\n{b}\ni32.eq"));
        }
        "<." => {
            return Ok(format!("{a}\n{b}\ni32.lt_s"));
        }
        ">." => {
            return Ok(format!("{a}\n{b}\ni32.gt_s"));
        }
        "<=." => {
            return Ok(format!("{a}\n{b}\ni32.le_s"));
        }
        ">=." => {
            return Ok(format!("{a}\n{b}\ni32.ge_s"));
        }
        "cons" => {
            let elem_ref = match node.typ.as_ref() {
                Some(Type::List(inner)) if is_ref_type(inner) => 1,
                Some(Type::List(_)) => 0,
                _ => {
                    return Err("cons result must be a vector".to_string());
                }
            };
            return Ok(format!("{a}\n{b}\ni32.const {elem_ref}\ncall $vec_concat_i32"));
        }
        "let" | "letrec" | "mut" | "while" => {
            return Err(format!("Unsupported return of builtin {}", op));
        }
        _ => {
            return Err(format!("Unsupported builtin {}", op));
        }
    };
    Ok(format!("{a}\n{b}\n{code}"))
}

fn compile_if(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let cond = compile_expr(
        node.children.get(1).ok_or_else(|| "if missing condition".to_string())?,
        ctx
    )?;
    let t = compile_expr(node.children.get(2).ok_or_else(|| "if missing then".to_string())?, ctx)?;
    let e = compile_expr(node.children.get(3).ok_or_else(|| "if missing else".to_string())?, ctx)?;
    let result_ty = node.typ
        .as_ref()
        .ok_or_else(|| "if missing type".to_string())
        .and_then(wasm_val_type)?;
    Ok(format!("{cond}\n(if (result {result_ty})\n  (then\n    {t}\n  )\n  (else\n    {e}\n  )\n)"))
}

fn is_borrowing_accessor_expr(node: &TypedExpression) -> bool {
    match &node.expr {
        Expression::Apply(items) if !items.is_empty() =>
            matches!(
                &items[0],
                Expression::Word(op)
                    if op == "get"
                        || op == "fst"
                        || op == "snd"
                        || op == "car"
            ),
        _ => false,
    }
}

const MAX_BORROW_ANALYSIS_DEPTH: usize = 64;

#[derive(Clone)]
enum CallableBinding {
    Named(String),
    Lambda(TypedExpression),
}

fn apply_child_at<'a>(node: &'a TypedExpression, item_idx: usize) -> Option<&'a TypedExpression> {
    let items = match &node.expr {
        Expression::Apply(items) => items,
        _ => {
            return None;
        }
    };
    let child_offset = if node.children.len() + 1 == items.len() { 1 } else { 0 };
    if item_idx < child_offset {
        None
    } else {
        node.children.get(item_idx - child_offset)
    }
}

fn lambda_params_and_body<'a>(
    lambda_node: &'a TypedExpression
) -> Option<(Vec<String>, &'a TypedExpression)> {
    let items = match &lambda_node.expr {
        Expression::Apply(items) => items,
        _ => {
            return None;
        }
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") || items.len() < 2 {
        return None;
    }
    let body_idx = items.len() - 1;
    let mut params = Vec::new();
    for p in &items[1..body_idx] {
        if let Expression::Word(name) = p {
            params.push(name.clone());
        } else {
            return None;
        }
    }
    let body = apply_child_at(lambda_node, body_idx).or_else(|| lambda_node.children.last())?;
    Some((params, body))
}

fn resolve_callable_binding_from_arg(
    arg: &TypedExpression,
    callable_env: &HashMap<String, CallableBinding>,
    lambda_bindings: &HashMap<String, TypedExpression>
) -> Option<CallableBinding> {
    match &arg.expr {
        Expression::Word(name) => {
            if let Some(binding) = callable_env.get(name) {
                Some(binding.clone())
            } else if lambda_bindings.contains_key(name) {
                Some(CallableBinding::Named(name.clone()))
            } else {
                None
            }
        }
        Expression::Apply(items) if !items.is_empty() => {
            if let Expression::Word(op) = &items[0] {
                if op == "lambda" {
                    return Some(CallableBinding::Lambda(arg.clone()));
                }
                if op == "as" || op == "char" {
                    return apply_child_at(arg, 1).and_then(|inner|
                        resolve_callable_binding_from_arg(inner, callable_env, lambda_bindings)
                    );
                }
            }
            None
        }
        _ => None,
    }
}

fn analyze_borrow_for_lambda_invocation(
    lambda_node: &TypedExpression,
    call_node: &TypedExpression,
    env: &HashMap<String, bool>,
    callable_env: &HashMap<String, CallableBinding>,
    lambda_bindings: &HashMap<String, TypedExpression>,
    call_stack: &mut Vec<String>,
    depth: usize
) -> Option<bool> {
    let (params, body) = lambda_params_and_body(lambda_node)?;
    let mut lambda_env: HashMap<String, bool> = HashMap::new();
    let mut lambda_callable_env: HashMap<String, CallableBinding> = HashMap::new();
    for (idx, param_name) in params.iter().enumerate() {
        let arg_node = apply_child_at(call_node, idx + 1);
        let arg_borrowed = arg_node
            .map(|arg|
                is_borrowed_managed_rhs_with_env(
                    arg,
                    env,
                    callable_env,
                    lambda_bindings,
                    call_stack,
                    depth + 1
                )
            )
            .unwrap_or(false);
        lambda_env.insert(param_name.clone(), arg_borrowed);

        if let Some(arg_node) = arg_node {
            if
                let Some(callable_binding) = resolve_callable_binding_from_arg(
                    arg_node,
                    callable_env,
                    lambda_bindings
                )
            {
                lambda_callable_env.insert(param_name.clone(), callable_binding);
            }
        }
    }

    Some(
        is_borrowed_managed_rhs_with_env(
            body,
            &lambda_env,
            &lambda_callable_env,
            lambda_bindings,
            call_stack,
            depth + 1
        )
    )
}

fn is_borrowed_managed_rhs_with_env(
    node: &TypedExpression,
    env: &HashMap<String, bool>,
    callable_env: &HashMap<String, CallableBinding>,
    lambda_bindings: &HashMap<String, TypedExpression>,
    call_stack: &mut Vec<String>,
    depth: usize
) -> bool {
    if depth > MAX_BORROW_ANALYSIS_DEPTH {
        // Conservative fallback: keep values alive rather than risk releasing
        // a borrowed alias when wrapper chains are unexpectedly deep.
        return true;
    }
    match &node.expr {
        Expression::Word(name) => *env.get(name).unwrap_or(&true),
        Expression::Apply(items) if !items.is_empty() => {
            let op = match &items[0] {
                Expression::Word(w) => w.as_str(),
                _ => {
                    return false;
                }
            };
            if op == "as" || op == "char" {
                return apply_child_at(node, 1)
                    .map(|n|
                        is_borrowed_managed_rhs_with_env(
                            n,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1
                        )
                    )
                    .unwrap_or(false);
            }
            if is_borrowing_accessor_expr(node) {
                return apply_child_at(node, 1)
                    .map(|n|
                        is_borrowed_managed_rhs_with_env(
                            n,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1
                        )
                    )
                    .unwrap_or(false);
            }
            if op == "if" {
                let then_borrowed = apply_child_at(node, 2)
                    .map(|n|
                        is_borrowed_managed_rhs_with_env(
                            n,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1
                        )
                    )
                    .unwrap_or(false);
                let else_borrowed = apply_child_at(node, 3)
                    .map(|n|
                        is_borrowed_managed_rhs_with_env(
                            n,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1
                        )
                    )
                    .unwrap_or(false);
                return then_borrowed && else_borrowed;
            }
            if op == "do" {
                let mut scoped_env = env.clone();
                let mut scoped_callable_env = callable_env.clone();
                if items.len() > 1 {
                    for i in 1..items.len() - 1 {
                        if let Expression::Apply(let_items) = &items[i] {
                            if
                                let [Expression::Word(kw), Expression::Word(name), _] =
                                    &let_items[..]
                            {
                                if kw == "let" || kw == "letrec" || kw == "mut" {
                                    let rhs_borrowed = apply_child_at(node, i)
                                        .and_then(|let_node| let_node.children.get(2))
                                        .map(|rhs|
                                            is_borrowed_managed_rhs_with_env(
                                                rhs,
                                                &scoped_env,
                                                &scoped_callable_env,
                                                lambda_bindings,
                                                call_stack,
                                                depth + 1
                                            )
                                        )
                                        .unwrap_or(false);
                                    scoped_env.insert(name.clone(), rhs_borrowed);
                                    if
                                        let Some(rhs_node) = apply_child_at(node, i).and_then(|n|
                                            n.children.get(2)
                                        )
                                    {
                                        if
                                            let Some(callable_binding) =
                                                resolve_callable_binding_from_arg(
                                                    rhs_node,
                                                    &scoped_callable_env,
                                                    lambda_bindings
                                                )
                                        {
                                            scoped_callable_env.insert(
                                                name.clone(),
                                                callable_binding
                                            );
                                        } else {
                                            scoped_callable_env.remove(name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                return apply_child_at(node, items.len() - 1)
                    .map(|last|
                        is_borrowed_managed_rhs_with_env(
                            last,
                            &scoped_env,
                            &scoped_callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1
                        )
                    )
                    .unwrap_or(false);
            }
            if let Some(binding) = callable_env.get(op) {
                match binding {
                    CallableBinding::Named(name) => {
                        if call_stack.iter().any(|item| item == name) {
                            return true;
                        }
                        if let Some(lambda_node) = lambda_bindings.get(name) {
                            call_stack.push(name.clone());
                            let result = analyze_borrow_for_lambda_invocation(
                                lambda_node,
                                node,
                                env,
                                callable_env,
                                lambda_bindings,
                                call_stack,
                                depth + 1
                            ).unwrap_or(false);
                            call_stack.pop();
                            return result;
                        }
                    }
                    CallableBinding::Lambda(lambda_node) => {
                        return analyze_borrow_for_lambda_invocation(
                            lambda_node,
                            node,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1
                        ).unwrap_or(false);
                    }
                }
            }
            if let Some(lambda_node) = lambda_bindings.get(op) {
                if call_stack.iter().any(|name| name == op) {
                    // Recursive/cyclic wrapper chain: stay conservative.
                    return true;
                }
                call_stack.push(op.to_string());
                let result = analyze_borrow_for_lambda_invocation(
                    lambda_node,
                    node,
                    env,
                    callable_env,
                    lambda_bindings,
                    call_stack,
                    depth + 1
                ).unwrap_or(false);
                call_stack.pop();
                return result;
            }
            if matches!(op, "vector" | "tuple" | "lambda" | "box" | "int" | "dec" | "bool") {
                // Fresh constructors return owned values.
                return false;
            }
            // Unknown call heads (e.g. higher-order callback params like `cb`)
            // are ambiguous: they may return borrowed aliases. Be conservative
            // to avoid use-after-free from auto-releasing discarded `do` values.
            true
        }
        _ => false,
    }
}

fn is_borrowed_managed_rhs_expr(
    node: &TypedExpression,
    lambda_bindings: &HashMap<String, TypedExpression>
) -> bool {
    let env = HashMap::new();
    let callable_env = HashMap::new();
    let mut call_stack = Vec::new();
    is_borrowed_managed_rhs_with_env(node, &env, &callable_env, lambda_bindings, &mut call_stack, 0)
}

fn is_fresh_owned_managed_expr(node: &TypedExpression) -> bool {
    match &node.expr {
        Expression::Apply(items) if !items.is_empty() => {
            if let Expression::Word(op) = &items[0] {
                if op == "as" || op == "char" {
                    return node.children.get(1).map(is_fresh_owned_managed_expr).unwrap_or(false);
                }
                return matches!(
                    op.as_str(),
                    "lambda" | "vector" | "tuple" | "box" | "int" | "dec" | "bool"
                );
            }
            false
        }
        _ => false,
    }
}

fn should_release_set_rhs(node: &TypedExpression) -> bool {
    if !node.typ.as_ref().map(is_managed_local_type).unwrap_or(false) {
        return false;
    }
    is_fresh_owned_managed_expr(node)
}

fn compile_do(
    items: &[Expression],
    node: &TypedExpression,
    ctx: &Ctx<'_>
) -> Result<String, String> {
    if items.len() <= 1 {
        return Ok("i32.const 0".to_string());
    }
    let child_offset = if node.children.len() + 1 == items.len() { 1 } else { 0 };
    let child_at = |item_idx: usize| -> Option<&TypedExpression> {
        if item_idx < child_offset { None } else { node.children.get(item_idx - child_offset) }
    };
    // Name-based local maps lose shadowed bindings, which can make alias checks
    // miss live refs and incorrectly release them. Be conservative: compare a
    // managed temporary against every non-temp slot in this function.
    let managed_local_slots: Vec<usize> = (0..ctx.tmp_i32).collect();
    let mut parts = Vec::new();
    let mut scoped_lambda_bindings = ctx.lambda_bindings.clone();
    for i in 1..items.len() - 1 {
        if let Expression::Apply(let_items) = &items[i] {
            if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                if kw == "let" || kw == "letrec" || kw == "mut" {
                    let val_node = child_at(i).and_then(|n| n.children.get(2));
                    let self_capture_idx = val_node.and_then(|n| {
                        if
                            kw != "mut" &&
                            matches!(&n.expr, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                        {
                            let key = n.expr.to_lisp();
                            ctx.closure_defs
                                .get(&key)
                                .and_then(|d| { d.captures.iter().position(|c| c == name) })
                        } else {
                            None
                        }
                    });
                    if let Some(n) = val_node {
                        match &n.expr {
                            Expression::Apply(xs) if
                                kw != "mut" &&
                                matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                            => {
                                scoped_lambda_bindings.insert(name.clone(), n.clone());
                            }
                            Expression::Word(alias) => {
                                if kw != "mut" {
                                    if
                                        let Some(target) = scoped_lambda_bindings
                                            .get(alias)
                                            .cloned()
                                    {
                                        scoped_lambda_bindings.insert(name.clone(), target);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    let value = val_node
                        .ok_or_else(|| format!("Missing let value for {}", name))
                        .and_then(|n| {
                            let scoped_ctx = Ctx {
                                fn_sigs: ctx.fn_sigs,
                                fn_ids: ctx.fn_ids,
                                lambda_ids: ctx.lambda_ids,
                                closure_defs: ctx.closure_defs,
                                lambda_bindings: &scoped_lambda_bindings,
                                locals: ctx.locals.clone(),
                                local_types: ctx.local_types.clone(),
                                tmp_i32: ctx.tmp_i32,
                            };
                            if
                                matches!(&n.expr, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                            {
                                compile_expr(n, &scoped_ctx)
                            } else {
                                compile_expr(n, &scoped_ctx)
                            }
                        })?;
                    if let Some(local_idx) = ctx.locals.get(name) {
                        let managed_local = ctx.local_types
                            .get(name)
                            .map(is_managed_local_type)
                            .unwrap_or(false);
                        let borrowed_rhs = val_node
                            .map(|n| is_borrowed_managed_rhs_expr(n, &scoped_lambda_bindings))
                            .unwrap_or(false);
                        let value = if managed_local && borrowed_rhs {
                            let tmp_owned = ctx.tmp_i32 + 2;
                            format!(
                                "{value}\nlocal.tee {}\ncall $rc_retain\ndrop\nlocal.get {}",
                                tmp_owned,
                                tmp_owned
                            )
                        } else {
                            value
                        };
                        if managed_local {
                            parts.push(
                                format!(
                                    "{value}\nlocal.get {}\ncall $rc_release\ndrop\nlocal.set {}",
                                    local_idx,
                                    local_idx
                                )
                            );
                        } else {
                            parts.push(format!("{value}\nlocal.set {}", local_idx));
                        }
                        if let Some(cap_idx) = self_capture_idx {
                            // Recursive local lambda: fill self-capture after binding is assigned.
                            // Use non-ref capture to avoid RC self-cycles.
                            parts.push(
                                format!(
                                    "local.get {}\ni32.const {}\nlocal.get {}\ncall $closure_set\ndrop",
                                    local_idx,
                                    cap_idx,
                                    local_idx
                                )
                            );
                        }
                    } else {
                        return Err(format!("Unknown local '{}'", name));
                    }
                    continue;
                }
            }
        }
        if let Some(n) = child_at(i) {
            let scoped_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: &scoped_lambda_bindings,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                tmp_i32: ctx.tmp_i32,
            };
            let c = compile_expr(n, &scoped_ctx)?;
            let managed = n.typ.as_ref().map(is_managed_local_type).unwrap_or(false);
            // Non-last managed expressions in `do` are usually temporaries and should be
            // released, but borrowed aliases (e.g. push! returning the same vector) must not
            // be released here.
            let borrowed = if managed {
                is_borrowed_managed_rhs_expr(n, &scoped_lambda_bindings)
            } else {
                false
            };
            if managed && !borrowed {
                let tmp_val = ctx.tmp_i32;
                let tmp_keep = ctx.tmp_i32 + 1;
                let mut blk = Vec::new();
                blk.push(format!("{c}\nlocal.set {}", tmp_val));
                if managed_local_slots.is_empty() {
                    blk.push(format!("local.get {}\ncall $rc_release\ndrop", tmp_val));
                } else {
                    blk.push(format!("i32.const 0\nlocal.set {}", tmp_keep));
                    for slot in &managed_local_slots {
                        blk.push(
                            format!(
                                "local.get {}\nlocal.get {}\ni32.eq\nif\n  i32.const 1\n  local.set {}\nend",
                                tmp_val,
                                slot,
                                tmp_keep
                            )
                        );
                    }
                    blk.push(
                        format!(
                            "local.get {}\ni32.eqz\nif\n  local.get {}\n  call $rc_release\n  drop\nend",
                            tmp_keep,
                            tmp_val
                        )
                    );
                }
                parts.push(blk.join("\n"));
            } else {
                parts.push(format!("{c}\ndrop"));
            }
        }
    }
    let last = child_at(items.len() - 1)
        .ok_or_else(|| "Missing final do expression".to_string())
        .and_then(|n| {
            let scoped_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: &scoped_lambda_bindings,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                tmp_i32: ctx.tmp_i32,
            };
            compile_expr(n, &scoped_ctx)
        })?;
    parts.push(last);
    Ok(parts.join("\n"))
}

fn compile_vector_literal(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let elem_kind = match node.typ.as_ref() {
        Some(Type::List(inner)) => vec_elem_kind_from_type(inner)?,
        Some(other) => {
            return Err(format!("vector literal expected list type, got {}", other));
        }
        None => {
            return Err("vector literal missing type".to_string());
        }
    };
    let args = &node.children[1..];
    let elem_ref_flag = match node.typ.as_ref() {
        Some(Type::List(inner)) if is_ref_type(inner) => 1,
        // Polymorphic vectors may carry reference elements at runtime
        // (e.g. hash-table buckets of key/value vectors). Default to
        // reference semantics for unknown element types.
        Some(Type::List(inner)) if matches!(inner.as_ref(), Type::Var(_)) => 1,
        _ => 0,
    };
    let mut out = Vec::new();
    out.push(
        format!(
            "i32.const {}\ni32.const {}\ncall $vec_new_{}\nlocal.set {}",
            0,
            elem_ref_flag,
            elem_kind.suffix(),
            ctx.tmp_i32
        )
    );
    for a in args {
        let nested_ctx = Ctx {
            fn_sigs: ctx.fn_sigs,
            fn_ids: ctx.fn_ids,
            lambda_ids: ctx.lambda_ids,
            closure_defs: ctx.closure_defs,
            lambda_bindings: ctx.lambda_bindings,
            locals: ctx.locals.clone(),
            local_types: ctx.local_types.clone(),
            tmp_i32: ctx.tmp_i32 + 1,
        };
        let v = compile_expr(a, &nested_ctx)?;
        let is_lambda_literal =
            matches!(
            &a.expr,
            Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
        );
        let arg_is_managed = a.typ.as_ref().map(is_managed_local_type).unwrap_or(false);
        if is_lambda_literal && arg_is_managed {
            // Fresh lambda values are retained by vector push; release the temporary owner.
            out.push(
                format!(
                    "local.get {}\n{}\nlocal.tee {}\ncall $vec_push_{}\ndrop\nlocal.get {}\ncall $rc_release\ndrop",
                    ctx.tmp_i32,
                    v,
                    ctx.tmp_i32 + 1,
                    elem_kind.suffix(),
                    ctx.tmp_i32 + 1
                )
            );
        } else {
            out.push(
                format!(
                    "local.get {}\n{}\ncall $vec_push_{}\ndrop",
                    ctx.tmp_i32,
                    v,
                    elem_kind.suffix()
                )
            );
        }
    }
    out.push(format!("local.get {}", ctx.tmp_i32));
    Ok(out.join("\n"))
}

fn compile_tuple(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let a = compile_expr(
        node.children.get(1).ok_or_else(|| "tuple missing first element".to_string())?,
        ctx
    )?;
    let b = compile_expr(
        node.children.get(2).ok_or_else(|| "tuple missing second element".to_string())?,
        ctx
    )?;
    Ok(format!("{a}\n{b}\ncall $tuple_new"))
}

fn compile_fst(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let p = compile_expr(
        node.children.get(1).ok_or_else(|| "fst missing tuple arg".to_string())?,
        ctx
    )?;
    Ok(format!("{p}\ncall $tuple_fst"))
}

fn compile_snd(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let p = compile_expr(
        node.children.get(1).ok_or_else(|| "snd missing tuple arg".to_string())?,
        ctx
    )?;
    Ok(format!("{p}\ncall $tuple_snd"))
}

fn compile_get(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs = compile_expr(
        node.children.get(1).ok_or_else(|| "get missing vector".to_string())?,
        ctx
    )?;
    let idx = compile_expr(
        node.children.get(2).ok_or_else(|| "get missing index".to_string())?,
        ctx
    )?;
    let elem = match node.typ.as_ref() {
        Some(t) => vec_elem_kind_from_type(t)?,
        None => {
            return Err("get missing return type".to_string());
        }
    };
    Ok(format!("{xs}\n{idx}\ncall $vec_get_{}", elem.suffix()))
}

fn compile_set(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs_node = node.children.get(1).ok_or_else(|| "set! missing vector".to_string())?;
    let xs = compile_expr(xs_node, ctx)?;
    let idx = compile_expr(
        node.children.get(2).ok_or_else(|| "set! missing index".to_string())?,
        ctx
    )?;
    let val_node = node.children.get(3).ok_or_else(|| "set! missing value".to_string())?;
    let v = compile_expr(val_node, ctx)?;
    let elem = val_node.typ
        .as_ref()
        .ok_or_else(|| "set! value missing type".to_string())
        .and_then(vec_elem_kind_from_type)?;
    let release_rhs = should_release_set_rhs(val_node);
    let release_target = false;
    let managed_slots = managed_local_slots(ctx);
    let target_tmp = ctx.tmp_i32 + 3;
    let target_keep = ctx.tmp_i32 + 4;
    let target_prefix = if release_target { format!("{xs}\nlocal.tee {target_tmp}") } else { xs };
    let target_release = if release_target {
        emit_release_managed_temp_if_not_local_alias(target_tmp, target_keep, &managed_slots)
    } else {
        String::new()
    };
    if release_rhs {
        let tmp_val = ctx.tmp_i32 + 1;
        let keep_tmp = ctx.tmp_i32 + 2;
        let release = emit_release_managed_temp_if_not_local_alias(
            tmp_val,
            keep_tmp,
            &managed_slots
        );
        let mut tail = Vec::new();
        tail.push(release);
        if !target_release.is_empty() {
            tail.push(target_release);
        }
        Ok(
            format!(
                "{target_prefix}\n{idx}\n{v}\nlocal.tee {tmp_val}\ncall $vec_set_{}\n{}",
                elem.suffix(),
                tail.join("\n")
            )
        )
    } else {
        if target_release.is_empty() {
            Ok(format!("{target_prefix}\n{idx}\n{v}\ncall $vec_set_{}", elem.suffix()))
        } else {
            Ok(
                format!(
                    "{target_prefix}\n{idx}\n{v}\ncall $vec_set_{}\n{}",
                    elem.suffix(),
                    target_release
                )
            )
        }
    }
}

fn compile_alter(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let target_name = match &node.expr {
        Expression::Apply(items) => {
            if items.len() != 3 {
                return Err("alter! requires exactly 2 arguments".to_string());
            }
            match &items[1] {
                Expression::Word(name) => name.clone(),
                _ => {
                    return Err("alter! first argument must be a mutable variable name".to_string());
                }
            }
        }
        _ => {
            return Err("alter! invalid form".to_string());
        }
    };
    let local_idx = *ctx.locals
        .get(&target_name)
        .ok_or_else(|| format!("alter! unknown local '{}'", target_name))?;
    let value = compile_expr(
        node.children.get(2).ok_or_else(|| "alter! missing value".to_string())?,
        ctx
    )?;
    Ok(format!("{value}\nlocal.set {local_idx}\ni32.const 0"))
}

fn compile_pop(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs = compile_expr(
        node.children.get(1).ok_or_else(|| "pop! missing vector".to_string())?,
        ctx
    )?;
    Ok(format!("{xs}\ncall $vec_pop_i32"))
}

fn compile_cdr(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs_node = node.children.get(1).ok_or_else(|| "cdr missing vector".to_string())?;
    let xs = compile_expr(xs_node, ctx)?;
    let start = if let Some(n) = node.children.get(2) {
        compile_expr(n, ctx)?
    } else {
        "i32.const 1".to_string()
    };
    let elem = match xs_node.typ.as_ref() {
        Some(Type::List(inner)) => vec_elem_kind_from_type(inner)?,
        Some(other) => {
            return Err(format!("cdr expected list, got {}", other));
        }
        None => {
            return Err("cdr missing argument type".to_string());
        }
    };
    Ok(format!("{xs}\n{start}\ncall $vec_slice_{}", elem.suffix()))
}

fn compile_loop_while(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let cond = compile_expr(
        node.children.get(1).ok_or_else(|| "while missing condition".to_string())?,
        ctx
    )?;
    let body_node = node.children.get(2).ok_or_else(|| "while missing body".to_string())?;
    let body_and_drop = format!("{}\ndrop", compile_expr(body_node, ctx)?);

    Ok(
        format!(
            "block\n  loop\n    {cond}\n    i32.eqz\n    br_if 1\n    {body_and_drop}\n    br 0\n  end\nend\ni32.const 0"
        )
    )
}

fn compile_fast_box_ctor(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>
) -> Result<String, String> {
    let value_node = node.children
        .get(1)
        .ok_or_else(|| format!("{} requires exactly 1 argument", op))?;
    if node.children.len() != 2 {
        return Err(format!("{} requires exactly 1 argument", op));
    }
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        tmp_i32: ctx.tmp_i32 + 2,
    };
    let value = compile_expr(value_node, &nested_ctx)?;
    let value_is_managed = value_node.typ.as_ref().map(is_managed_local_type).unwrap_or(false);
    let is_lambda_literal =
        matches!(
        &value_node.expr,
        Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
    );
    // Keep polymorphic `box` as ref-cell like generic lowering; typed scalar ctors stay scalar cells.
    let elem_ref = if op == "box" { 1 } else { 0 };
    let normalized_value = if op == "bool" {
        format!("{value}\ni32.const 0\ni32.ne")
    } else {
        value
    };
    let vec_local = ctx.tmp_i32;
    let tmp_val = ctx.tmp_i32 + 1;
    if is_lambda_literal && value_is_managed {
        Ok(
            format!(
                "i32.const 0\ni32.const {elem_ref}\ncall $vec_new_i32\nlocal.set {vec_local}\nlocal.get {vec_local}\ni32.const 0\n{normalized_value}\nlocal.tee {tmp_val}\ncall $vec_set_i32\ndrop\nlocal.get {tmp_val}\ncall $rc_release\ndrop\nlocal.get {vec_local}"
            )
        )
    } else {
        Ok(
            format!(
                "i32.const 0\ni32.const {elem_ref}\ncall $vec_new_i32\nlocal.set {vec_local}\nlocal.get {vec_local}\ni32.const 0\n{normalized_value}\ncall $vec_set_i32\ndrop\nlocal.get {vec_local}"
            )
        )
    }
}

fn compile_fast_cell_set(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    normalize_bool: bool
) -> Result<String, String> {
    if node.children.len() != 3 {
        return Err(format!("{} requires exactly 2 arguments", op));
    }
    let cell_node = node.children.get(1).ok_or_else(|| format!("{} missing cell", op))?;
    let value_node = node.children.get(2).ok_or_else(|| format!("{} missing value", op))?;
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        tmp_i32: ctx.tmp_i32 + 2,
    };
    let cell = compile_expr(cell_node, &nested_ctx)?;
    let value_raw = compile_expr(value_node, &nested_ctx)?;
    let value = if normalize_bool {
        format!("{value_raw}\ni32.const 0\ni32.ne")
    } else {
        value_raw
    };
    let release_rhs = should_release_set_rhs(value_node);
    let managed_slots = managed_local_slots(ctx);
    let cell_prefix = cell;
    if release_rhs {
        let tmp_val = ctx.tmp_i32 + 1;
        let keep_tmp = ctx.tmp_i32 + 2;
        let release = emit_release_managed_temp_if_not_local_alias(
            tmp_val,
            keep_tmp,
            &managed_slots
        );
        Ok(
            format!("{cell_prefix}\ni32.const 0\n{value}\nlocal.tee {tmp_val}\ncall $vec_set_i32\n{}", release)
        )
    } else {
        Ok(format!("{cell_prefix}\ni32.const 0\n{value}\ncall $vec_set_i32"))
    }
}

fn compile_fast_truthy(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    negate: bool
) -> Result<String, String> {
    if node.children.len() != 2 {
        return Err(format!("{} requires exactly 1 argument", op));
    }
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        tmp_i32: ctx.tmp_i32 + 2,
    };
    let cell = compile_expr(
        node.children.get(1).ok_or_else(|| format!("{} missing cell", op))?,
        &nested_ctx
    )?;
    if negate {
        Ok(format!("{cell}\ni32.const 0\ncall $vec_get_i32\ni32.eqz"))
    } else {
        Ok(format!("{cell}\ni32.const 0\ncall $vec_get_i32\ni32.const 0\ni32.ne"))
    }
}

fn compile_fast_cell_helper(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>
) -> Option<Result<String, String>> {
    match op {
        "box" | "int" | "dec" | "bool" => Some(compile_fast_box_ctor(op, node, ctx)),
        "&alter!" | "set" | "=!" => Some(compile_fast_cell_set(op, node, ctx, false)),
        "true?" => Some(compile_fast_truthy(op, node, ctx, false)),
        "false?" => Some(compile_fast_truthy(op, node, ctx, true)),
        _ => None,
    }
}

fn managed_local_slots(ctx: &Ctx<'_>) -> Vec<usize> {
    // Name-based local maps drop shadowed bindings, which can make alias checks
    // miss live refs and incorrectly release them. Be conservative: scan all
    // non-temp slots in the current function frame.
    (0..ctx.tmp_i32).collect()
}

fn emit_release_managed_temp_if_not_local_alias(
    tmp_val: usize,
    tmp_keep: usize,
    managed_local_slots: &[usize]
) -> String {
    let debug_rc = cfg!(feature = "debug-rc");
    if managed_local_slots.is_empty() {
        if debug_rc {
            return format!(
                "global.get $dbg_tmp_release_exec\ni64.const 1\ni64.add\nglobal.set $dbg_tmp_release_exec\nlocal.get {tmp_val}\ncall $rc_release\ndrop\nlocal.get {tmp_val}\ncall $is_vec_ptr\nif\n  local.get {tmp_val}\n  i32.const 8\n  i32.add\n  i32.load\n  i32.const 1\n  i32.eq\n  if\n    global.get $dbg_tmp_release_post_rc_eq_1\n    i64.const 1\n    i64.add\n    global.set $dbg_tmp_release_post_rc_eq_1\n  else\n    global.get $dbg_tmp_release_post_rc_other\n    i64.const 1\n    i64.add\n    global.set $dbg_tmp_release_post_rc_other\n  end\nelse\n  global.get $dbg_tmp_release_post_not_vec\n  i64.const 1\n  i64.add\n  global.set $dbg_tmp_release_post_not_vec\nend"
            );
        }
        return format!("local.get {tmp_val}\ncall $rc_release\ndrop");
    }
    let mut out = Vec::new();
    out.push(format!("i32.const 0\nlocal.set {}", tmp_keep));
    for slot in managed_local_slots {
        out.push(
            format!(
                "local.get {}\nlocal.get {}\ni32.eq\nif\n  i32.const 1\n  local.set {}\nend",
                tmp_val,
                slot,
                tmp_keep
            )
        );
    }
    if debug_rc {
        out.push(
            format!(
                "local.get {}\ni32.eqz\nif\n  global.get $dbg_tmp_release_exec\n  i64.const 1\n  i64.add\n  global.set $dbg_tmp_release_exec\n  local.get {}\n  call $rc_release\n  drop\n  local.get {}\n  call $is_vec_ptr\n  if\n    local.get {}\n    i32.const 8\n    i32.add\n    i32.load\n    i32.const 1\n    i32.eq\n    if\n      global.get $dbg_tmp_release_post_rc_eq_1\n      i64.const 1\n      i64.add\n      global.set $dbg_tmp_release_post_rc_eq_1\n    else\n      global.get $dbg_tmp_release_post_rc_other\n      i64.const 1\n      i64.add\n      global.set $dbg_tmp_release_post_rc_other\n    end\n  else\n    global.get $dbg_tmp_release_post_not_vec\n    i64.const 1\n    i64.add\n    global.set $dbg_tmp_release_post_not_vec\n  end\nelse\n  global.get $dbg_tmp_release_skip\n  i64.const 1\n  i64.add\n  global.set $dbg_tmp_release_skip\nend",
                tmp_keep,
                tmp_val,
                tmp_val,
                tmp_val
            )
        );
    } else {
        out.push(
            format!(
                "local.get {}\ni32.eqz\nif\n  local.get {}\n  call $rc_release\n  drop\nend",
                tmp_keep,
                tmp_val
            )
        );
    }
    out.join("\n")
}

#[cfg(feature = "io")]
fn compile_host_unary_string_call(
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    op_name: &str,
    host_symbol: &str
) -> Result<String, String> {
    if node.children.len() != 2 {
        return Err(format!("{op_name} expects exactly one [Char] argument"));
    }
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        tmp_i32: ctx.tmp_i32 + 3,
    };
    let arg = compile_expr(
        node.children.get(1).ok_or_else(|| format!("{op_name} missing argument"))?,
        &nested_ctx
    )?;
    let arg_tmp = ctx.tmp_i32;
    let keep_tmp = ctx.tmp_i32 + 1;
    let ret_tmp = ctx.tmp_i32 + 2;
    let managed_slots = managed_local_slots(ctx);
    let release = emit_release_managed_temp_if_not_local_alias(arg_tmp, keep_tmp, &managed_slots);
    Ok(
        format!(
            "{arg}\nlocal.tee {arg_tmp}\ncall ${host_symbol}\nlocal.set {ret_tmp}\n{release}\nlocal.get {ret_tmp}"
        )
    )
}

#[cfg(feature = "io")]
fn compile_host_unary_int_call(
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    op_name: &str,
    host_symbol: &str
) -> Result<String, String> {
    if node.children.len() != 2 {
        return Err(format!("{op_name} expects exactly one Int argument"));
    }
    let arg = compile_expr(
        node.children.get(1).ok_or_else(|| format!("{op_name} missing argument"))?,
        ctx
    )?;
    Ok(format!("{arg}\ncall ${host_symbol}"))
}

#[cfg(feature = "io")]
fn compile_host_unit_call(
    node: &TypedExpression,
    _ctx: &Ctx<'_>,
    op_name: &str,
    host_symbol: &str
) -> Result<String, String> {
    if node.children.len() != 1 {
        return Err(format!("{op_name} expects no arguments"));
    }
    Ok(format!("call ${host_symbol}"))
}

#[cfg(feature = "io")]
fn compile_host_write_call(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    if node.children.len() != 3 {
        return Err("write! expects exactly two [Char] arguments".to_string());
    }
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        tmp_i32: ctx.tmp_i32 + 5,
    };
    let path = compile_expr(
        node.children.get(1).ok_or_else(|| "write! missing path".to_string())?,
        &nested_ctx
    )?;
    let data = compile_expr(
        node.children.get(2).ok_or_else(|| "write! missing content".to_string())?,
        &nested_ctx
    )?;
    let path_tmp = ctx.tmp_i32;
    let data_tmp = ctx.tmp_i32 + 1;
    let keep_tmp = ctx.tmp_i32 + 2;
    let ret_tmp = ctx.tmp_i32 + 3;
    let managed_slots = managed_local_slots(ctx);
    let release_path = emit_release_managed_temp_if_not_local_alias(
        path_tmp,
        keep_tmp,
        &managed_slots
    );
    let release_data = emit_release_managed_temp_if_not_local_alias(
        data_tmp,
        keep_tmp,
        &managed_slots
    );
    Ok(
        format!(
            "{path}\nlocal.set {path_tmp}\n{data}\nlocal.set {data_tmp}\nlocal.get {path_tmp}\nlocal.get {data_tmp}\ncall $host_write_file\nlocal.set {ret_tmp}\n{release_path}\n{release_data}\nlocal.get {ret_tmp}"
        )
    )
}

#[cfg(feature = "io")]
fn compile_host_binary_string_call(
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    op_name: &str,
    host_symbol: &str
) -> Result<String, String> {
    if node.children.len() != 3 {
        return Err(format!("{op_name} expects exactly two [Char] arguments"));
    }
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        tmp_i32: ctx.tmp_i32 + 5,
    };
    let left = compile_expr(
        node.children.get(1).ok_or_else(|| format!("{op_name} missing first argument"))?,
        &nested_ctx
    )?;
    let right = compile_expr(
        node.children.get(2).ok_or_else(|| format!("{op_name} missing second argument"))?,
        &nested_ctx
    )?;
    let left_tmp = ctx.tmp_i32;
    let right_tmp = ctx.tmp_i32 + 1;
    let keep_tmp = ctx.tmp_i32 + 2;
    let ret_tmp = ctx.tmp_i32 + 3;
    let managed_slots = managed_local_slots(ctx);
    let release_left = emit_release_managed_temp_if_not_local_alias(
        left_tmp,
        keep_tmp,
        &managed_slots
    );
    let release_right = emit_release_managed_temp_if_not_local_alias(
        right_tmp,
        keep_tmp,
        &managed_slots
    );
    Ok(
        format!(
            "{left}\nlocal.set {left_tmp}\n{right}\nlocal.set {right_tmp}\nlocal.get {left_tmp}\nlocal.get {right_tmp}\ncall ${host_symbol}\nlocal.set {ret_tmp}\n{release_left}\n{release_right}\nlocal.get {ret_tmp}"
        )
    )
}

#[cfg(not(feature = "io"))]
fn compile_host_unary_string_call(
    _node: &TypedExpression,
    _ctx: &Ctx<'_>,
    op_name: &str,
    _host_symbol: &str
) -> Result<String, String> {
    Err(format!("{op_name} requires enabling the 'io' feature"))
}

#[cfg(not(feature = "io"))]
fn compile_host_unary_int_call(
    _node: &TypedExpression,
    _ctx: &Ctx<'_>,
    op_name: &str,
    _host_symbol: &str
) -> Result<String, String> {
    Err(format!("{op_name} requires enabling the 'io' feature"))
}

#[cfg(not(feature = "io"))]
fn compile_host_unit_call(
    _node: &TypedExpression,
    _ctx: &Ctx<'_>,
    op_name: &str,
    _host_symbol: &str
) -> Result<String, String> {
    Err(format!("{op_name} requires enabling the 'io' feature"))
}

#[cfg(not(feature = "io"))]
fn compile_host_write_call(_node: &TypedExpression, _ctx: &Ctx<'_>) -> Result<String, String> {
    Err("write! requires enabling the 'io' feature".to_string())
}

#[cfg(not(feature = "io"))]
fn compile_host_binary_string_call(
    _node: &TypedExpression,
    _ctx: &Ctx<'_>,
    op_name: &str,
    _host_symbol: &str
) -> Result<String, String> {
    Err(format!("{op_name} requires enabling the 'io' feature"))
}

fn compile_call(node: &TypedExpression, op: &str, ctx: &Ctx<'_>) -> Result<String, String> {
    if let Some(fast) = compile_fast_cell_helper(op, node, ctx) {
        return fast;
    }
    let (params, ret_ty) = if let Some(sig) = ctx.fn_sigs.get(op) {
        sig.clone()
    } else if builtin_fn_tag(op).is_some() {
        // Builtin used via namespaced value (e.g. std/vector/set!) without explicit top def.
        (Vec::new(), Type::Int)
    } else {
        return Err(format!("Unknown function '{}'", op));
    };
    let args = &node.children[1..];
    if params.is_empty() && !args.is_empty() {
        let (ret_params, _ret_final) = function_parts(&ret_ty);
        if !ret_params.is_empty() && args.len() < ret_params.len() {
            let total = ret_params.len();
            let provided = args.len();
            let helper_name = format!("__partial_dyn_{}_{}", total, provided);
            let helper_id = *ctx.fn_ids
                .get(&helper_name)
                .ok_or_else(|| format!("Missing dynamic partial helper '{}'", helper_name))?;
            let clo_local = ctx.tmp_i32;
            let tmp_local = ctx.tmp_i32 + 1;
            let mut out = Vec::new();
            out.push(
                format!(
                    "i32.const {}\ni32.const {}\ncall $closure_new\nlocal.set {}",
                    helper_id,
                    1 + provided,
                    clo_local
                )
            );
            out.push(
                format!(
                    "local.get {}\ni32.const 0\ncall ${}\ncall $closure_set_fun\ndrop",
                    clo_local,
                    ident(op)
                )
            );
            for (i, arg) in args.iter().enumerate() {
                let nested_ctx = Ctx {
                    fn_sigs: ctx.fn_sigs,
                    fn_ids: ctx.fn_ids,
                    lambda_ids: ctx.lambda_ids,
                    closure_defs: ctx.closure_defs,
                    lambda_bindings: ctx.lambda_bindings,
                    locals: ctx.locals.clone(),
                    local_types: ctx.local_types.clone(),
                    tmp_i32: ctx.tmp_i32 + 2,
                };
                let av = compile_expr(arg, &nested_ctx)?;
                let idx = i + 1;
                let store_op = closure_store_op_for_type_wat(&ret_params[i]);
                let is_lambda_literal =
                    matches!(
                    &arg.expr,
                    Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                );
                if store_op != "$closure_set" {
                    if is_lambda_literal {
                        out.push(
                            format!(
                                "local.get {}\ni32.const {}\n{}\nlocal.tee {}\ncall {}\ndrop\nlocal.get {}\ncall $rc_release\ndrop",
                                clo_local,
                                idx,
                                av,
                                tmp_local,
                                store_op,
                                tmp_local
                            )
                        );
                    } else {
                        out.push(
                            format!(
                                "local.get {}\ni32.const {}\n{}\ncall {}\ndrop",
                                clo_local,
                                idx,
                                av,
                                store_op
                            )
                        );
                    }
                } else {
                    out.push(
                        format!(
                            "local.get {}\ni32.const {}\n{}\ncall $closure_set\ndrop",
                            clo_local,
                            idx,
                            av
                        )
                    );
                }
            }
            out.push(format!("local.get {}", clo_local));
            return Ok(out.join("\n"));
        }
        if !ret_params.is_empty() && args.len() > ret_params.len() {
            let (initial_args, rest_args) = args.split_at(ret_params.len());
            let mut out = vec![format!("call ${}", ident(op))];
            for arg in initial_args {
                out.push(compile_expr(arg, ctx)?);
            }
            out.push(format!("call $apply{}_i32", initial_args.len()));
            for arg in rest_args {
                out.push(compile_expr(arg, ctx)?);
                out.push("call $apply1_i32".to_string());
            }
            return Ok(out.join("\n"));
        }
        let mut out = vec![format!("call ${}", ident(op))];
        for arg in args {
            out.push(compile_expr(arg, ctx)?);
        }
        out.push(format!("call $apply{}_i32", args.len()));
        return Ok(out.join("\n"));
    }
    let unit_arity_elided = params.len() == 1 && matches!(params[0], Type::Unit) && args.is_empty();
    if !unit_arity_elided && args.len() < params.len() {
        let total = params.len();
        let provided = args.len();
        let helper_name = format!("__partial_dyn_{}_{}", total, provided);
        let helper_id = *ctx.fn_ids
            .get(&helper_name)
            .ok_or_else(|| format!("Missing dynamic partial helper '{}'", helper_name))?;
        let fn_ptr = if let Some(fid) = ctx.fn_ids.get(op) {
            format!("i32.const {}", fid)
        } else if let Some(tag) = builtin_fn_tag(op) {
            format!("i32.const {}", tag)
        } else {
            return Err(
                format!("Partial application requires function id/tag for '{}', but none was found", op)
            );
        };
        let clo_local = ctx.tmp_i32;
        let tmp_local = ctx.tmp_i32 + 1;
        let mut out = Vec::new();
        out.push(
            format!(
                "i32.const {}\ni32.const {}\ncall $closure_new\nlocal.set {}",
                helper_id,
                1 + provided,
                clo_local
            )
        );
        out.push(
            format!("local.get {}\ni32.const 0\n{}\ncall $closure_set_fun\ndrop", clo_local, fn_ptr)
        );
        for (i, arg) in args.iter().enumerate() {
            let nested_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: ctx.lambda_bindings,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                tmp_i32: ctx.tmp_i32 + 2,
            };
            let av = compile_expr(arg, &nested_ctx)?;
            let idx = i + 1;
            let store_op = closure_store_op_for_type_wat(&params[i]);
            let is_lambda_literal =
                matches!(
                &arg.expr,
                Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
            );
            if store_op != "$closure_set" {
                if is_lambda_literal {
                    out.push(
                        format!(
                            "local.get {}\ni32.const {}\n{}\nlocal.tee {}\ncall {}\ndrop\nlocal.get {}\ncall $rc_release\ndrop",
                            clo_local,
                            idx,
                            av,
                            tmp_local,
                            store_op,
                            tmp_local
                        )
                    );
                } else {
                    out.push(
                        format!(
                            "local.get {}\ni32.const {}\n{}\ncall {}\ndrop",
                            clo_local,
                            idx,
                            av,
                            store_op
                        )
                    );
                }
            } else {
                out.push(
                    format!(
                        "local.get {}\ni32.const {}\n{}\ncall $closure_set\ndrop",
                        clo_local,
                        idx,
                        av
                    )
                );
            }
        }
        out.push(format!("local.get {}", clo_local));
        return Ok(out.join("\n"));
    }
    if args.len() > params.len() && !unit_arity_elided {
        let (initial_args, rest_args) = args.split_at(params.len());
        let mut out = Vec::new();
        for arg in initial_args {
            out.push(compile_expr(arg, ctx)?);
        }
        out.push(format!("call ${}", ident(op)));
        for arg in rest_args {
            out.push(compile_expr(arg, ctx)?);
            out.push("call $apply1_i32".to_string());
        }
        return Ok(out.join("\n"));
    }
    if args.len() != params.len() && !unit_arity_elided {
        return Err(
            format!(
                "Unassigned function with partial application/extra args not yet supported in wasm backend: '{}' expected {} args, got {}",
                op,
                params.len(),
                args.len()
            )
        );
    }
    let mut out = Vec::new();
    if !unit_arity_elided {
        for arg in args {
            out.push(compile_expr(arg, ctx)?);
        }
    }
    out.push(format!("call ${}", ident(op)));
    Ok(out.join("\n"))
}

fn compile_dynamic_call(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let f_node = node.children.first().ok_or_else(|| "call missing function".to_string())?;
    let f = compile_expr(f_node, ctx)?;
    let args = &node.children[1..];
    let head_ty = f_node.typ.as_ref().ok_or_else(|| "dynamic call head missing type".to_string())?;
    let (head_params, _head_ret) = function_parts(head_ty);
    if args.is_empty() {
        // Zero-arg invocation of a function value (e.g. local thunk).
        return Ok(format!("{f}\ncall $apply0_i32"));
    }
    if !head_params.is_empty() && args.len() < head_params.len() {
        let total = head_params.len();
        let provided = args.len();
        let helper_name = format!("__partial_dyn_{}_{}", total, provided);
        let helper_id = *ctx.fn_ids
            .get(&helper_name)
            .ok_or_else(|| format!("Missing dynamic partial helper '{}'", helper_name))?;
        let clo_local = ctx.tmp_i32;
        let tmp_local = ctx.tmp_i32 + 1;
        let mut out = Vec::new();
        out.push(
            format!(
                "i32.const {}\ni32.const {}\ncall $closure_new\nlocal.set {}",
                helper_id,
                1 + provided,
                clo_local
            )
        );
        out.push(
            format!("local.get {}\ni32.const 0\n{}\ncall $closure_set_fun\ndrop", clo_local, f)
        );
        for (i, arg) in args.iter().enumerate() {
            let nested_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: ctx.lambda_bindings,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                tmp_i32: ctx.tmp_i32 + 2,
            };
            let av = compile_expr(arg, &nested_ctx)?;
            let idx = i + 1;
            let store_op = closure_store_op_for_type_wat(&head_params[i]);
            let is_lambda_literal =
                matches!(
                &arg.expr,
                Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
            );
            if store_op != "$closure_set" {
                if is_lambda_literal {
                    out.push(
                        format!(
                            "local.get {}\ni32.const {}\n{}\nlocal.tee {}\ncall {}\ndrop\nlocal.get {}\ncall $rc_release\ndrop",
                            clo_local,
                            idx,
                            av,
                            tmp_local,
                            store_op,
                            tmp_local
                        )
                    );
                } else {
                    out.push(
                        format!(
                            "local.get {}\ni32.const {}\n{}\ncall {}\ndrop",
                            clo_local,
                            idx,
                            av,
                            store_op
                        )
                    );
                }
            } else {
                out.push(
                    format!(
                        "local.get {}\ni32.const {}\n{}\ncall $closure_set\ndrop",
                        clo_local,
                        idx,
                        av
                    )
                );
            }
        }
        out.push(format!("local.get {}", clo_local));
        return Ok(out.join("\n"));
    }
    if !head_params.is_empty() && args.len() > head_params.len() {
        let (initial_args, rest_args) = args.split_at(head_params.len());
        let mut out = vec![f];
        for arg in initial_args {
            out.push(compile_expr(arg, ctx)?);
        }
        out.push(format!("call $apply{}_i32", initial_args.len()));
        for arg in rest_args {
            out.push(compile_expr(arg, ctx)?);
            out.push("call $apply1_i32".to_string());
        }
        return Ok(out.join("\n"));
    }
    let mut out = vec![f];
    for arg in args {
        out.push(compile_expr(arg, ctx)?);
    }
    out.push(format!("call $apply{}_i32", args.len()));
    Ok(out.join("\n"))
}

fn resolve_local_devirtualized_head(
    local_head: &str,
    ctx: &Ctx<'_>
) -> Result<Option<String>, String> {
    let mode = devirtualize_mode_from_env()?;
    if mode == DevirtualizeMode::Off {
        return Ok(None);
    }
    let Some(lambda_node) = ctx.lambda_bindings.get(local_head) else {
        return Ok(None);
    };
    let key = lambda_node.expr.to_lisp();
    if ctx.closure_defs.contains_key(&key) {
        return Ok(None);
    }
    let Some(target_id) = ctx.lambda_ids.get(&key).copied() else {
        return Ok(None);
    };
    Ok(
        ctx.fn_ids
            .iter()
            .find_map(|(name, id)| if *id == target_id { Some(name.clone()) } else { None })
    )
}

fn compile_lambda_literal(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let key = node.expr.to_lisp();
    if let Some(id) = ctx.lambda_ids.get(&key) {
        Ok(format!("i32.const {}", id))
    } else if let Some(def) = ctx.closure_defs.get(&key) {
        let fn_id = ctx.fn_ids
            .get(&def.name)
            .ok_or_else(|| format!("Missing function id for closure '{}'", def.name))?;
        let clo_local = ctx.tmp_i32;
        let mut out = Vec::new();
        out.push(
            format!(
                "i32.const {}\ni32.const {}\ncall $closure_new\nlocal.set {}",
                fn_id,
                def.captures.len(),
                clo_local
            )
        );
        for (i, cap) in def.captures.iter().enumerate() {
            let (cap_v, set_fn) = if let Some(local_idx) = ctx.locals.get(cap) {
                let local_ty = ctx.local_types.get(cap);
                (
                    format!("local.get {}", local_idx),
                    if matches!(local_ty, Some(Type::Function(_, _))) {
                        "$closure_set_fun"
                    } else if local_ty.map(is_managed_local_type).unwrap_or(false) {
                        "$closure_set_ref"
                    } else {
                        "$closure_set"
                    },
                )
            } else if cap == "ARGV" {
                ("call $__argv_get".to_string(), "$closure_set_ref")
            } else if let Some((ps, ret)) = ctx.fn_sigs.get(cap) {
                if ps.is_empty() {
                    (
                        format!("call ${}", ident(cap)),
                        if is_managed_local_type(ret) {
                            "$closure_set_ref"
                        } else {
                            "$closure_set"
                        },
                    )
                } else if let Some(id) = ctx.fn_ids.get(cap) {
                    // Function-valued global capture: store function pointer id.
                    (format!("i32.const {}", id), "$closure_set_fun")
                } else if let Some(tag) = builtin_fn_tag(cap) {
                    // Builtin function captured as value.
                    (format!("i32.const {}", tag), "$closure_set_fun")
                } else {
                    return Err(
                        format!("Unsupported closure capture '{}' in wasm backend (no function id/tag)", cap)
                    );
                }
            } else if let Some(tag) = builtin_fn_tag(cap) {
                (format!("i32.const {}", tag), "$closure_set_fun")
            } else {
                return Err(format!("Unsupported closure capture '{}' in wasm backend", cap));
            };
            out.push(
                format!(
                    "local.get {}\ni32.const {}\n{}\ncall {}\ndrop",
                    clo_local,
                    i,
                    cap_v,
                    set_fn
                )
            );
        }
        out.push(format!("local.get {}", clo_local));
        Ok(out.join("\n"))
    } else {
        Err(format!("Unsupported lambda literal in wasm backend (missing lowering id): {}", key))
    }
}

fn compile_expr(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    match &node.expr {
        Expression::Int(n) => Ok(format!("i32.const {}", n)),
        Expression::Dec(n) => {
            let scaled = ((*n as f64) * (decimal_scale_i64() as f64)).round() as i64;
            Ok(format!("i32.const {}", scaled as i32))
        }
        Expression::Word(w) =>
            match w.as_str() {
                "true" => Ok("i32.const 1".to_string()),
                "false" => Ok("i32.const 0".to_string()),
                "nil" => Ok("i32.const 0".to_string()),
                _ => {
                    if let Some(local_idx) = ctx.locals.get(w) {
                        Ok(format!("local.get {}", local_idx))
                    } else if w == "ARGV" {
                        Ok("call $__argv_get".to_string())
                    } else if let Some((params, _ret)) = ctx.fn_sigs.get(w) {
                        if params.is_empty() {
                            Ok(format!("call ${}", ident(w)))
                        } else if let Some(id) = ctx.fn_ids.get(w) {
                            Ok(format!("i32.const {}", id))
                        } else {
                            Err(
                                format!("Unsupported function-valued word in wasm backend: '{}'", w)
                            )
                        }
                    } else if let Some(tag) = builtin_fn_tag(w) {
                        Ok(format!("i32.const {}", tag))
                    } else {
                        Err(format!("Unsupported free word in wasm backend: '{}'", w))
                    }
                }
            }
        Expression::Apply(items) => {
            if items.is_empty() {
                return Ok("i32.const 0".to_string());
            }
            match &items[0] {
                Expression::Word(op) => {
                    let op_full = op.as_str();
                    match op_full {
                        _ if ctx.locals.contains_key(op_full) => {
                            if
                                let Some(target_name) = resolve_local_devirtualized_head(
                                    op_full,
                                    ctx
                                )?
                            {
                                compile_call(node, &target_name, ctx)
                            } else {
                                compile_dynamic_call(node, ctx)
                            }
                        }
                        "lambda" => compile_lambda_literal(node, ctx),
                        "do" => compile_do(items, node, ctx),
                        "if" => compile_if(node, ctx),
                        "tuple" => compile_tuple(node, ctx),
                        "vector" | "string" => compile_vector_literal(node, ctx),
                        "length" => {
                            let a = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "length missing arg".to_string())?,
                                ctx
                            )?;
                            Ok(format!("{a}\ncall $vec_len"))
                        }
                        "get" => compile_get(node, ctx),
                        "fst" => compile_fst(node, ctx),
                        "snd" => compile_snd(node, ctx),
                        "car" => {
                            let xs = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "car missing vector".to_string())?,
                                ctx
                            )?;
                            let elem = match node.typ.as_ref() {
                                Some(t) => vec_elem_kind_from_type(t)?,
                                None => {
                                    return Err("car missing return type".to_string());
                                }
                            };
                            Ok(format!("{xs}\ni32.const 0\ncall $vec_get_{}", elem.suffix()))
                        }
                        "cdr" => compile_cdr(node, ctx),
                        "set!" => compile_set(node, ctx),
                        "alter!" => compile_alter(node, ctx),
                        "pop!" => compile_pop(node, ctx),
                        "while" => compile_loop_while(node, ctx),
                        "list-dir!" =>
                            compile_host_unary_string_call(node, ctx, "list-dir!", "host_list_dir"),
                        "read!" =>
                            compile_host_unary_string_call(node, ctx, "read!", "host_read_file"),
                        "mkdir!" =>
                            compile_host_unary_string_call(node, ctx, "mkdir!", "host_mkdir_p"),
                        "delete!" =>
                            compile_host_unary_string_call(node, ctx, "delete!", "host_delete"),
                        "print!" =>
                            compile_host_unary_string_call(node, ctx, "print!", "host_print"),
                        "sleep!" => compile_host_unary_int_call(node, ctx, "sleep!", "host_sleep"),
                        "clear!" => compile_host_unit_call(node, ctx, "clear!", "host_clear"),
                        "write!" => compile_host_write_call(node, ctx),
                        "move!" => compile_host_binary_string_call(node, ctx, "move!", "host_move"),
                        "not" => {
                            let a = compile_expr(
                                node.children.get(1).ok_or_else(|| "not missing arg".to_string())?,
                                ctx
                            )?;
                            Ok(format!("{a}\ni32.eqz"))
                        }
                        "~" => {
                            let a = compile_expr(
                                node.children.get(1).ok_or_else(|| "~ missing arg".to_string())?,
                                ctx
                            )?;
                            // Bitwise NOT for i32.
                            Ok(format!("{a}\ni32.const -1\ni32.xor"))
                        }
                        "Int->Dec" => {
                            let a = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "Int->Dec missing arg".to_string())?,
                                ctx
                            )?;
                            Ok(format!("{a}\ncall $dec_from_int"))
                        }
                        "Dec->Int" => {
                            let a = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "Dec->Int missing arg".to_string())?,
                                ctx
                            )?;
                            Ok(format!("{a}\ncall $dec_to_int"))
                        }
                        "as" | "char" =>
                            node.children
                                .get(1)
                                .map(|n| compile_expr(n, ctx))
                                .unwrap_or_else(|| Ok("i32.const 0".to_string())),
                        op if
                            builtin_fn_tag(op)
                                .and_then(builtin_tag_arity)
                                .map(|arity| node.children.len().saturating_sub(1) != arity)
                                .unwrap_or(false)
                        => compile_dynamic_call(node, ctx),
                        op if is_special_word(op) => emit_builtin(op, node, ctx),
                        _ => compile_call(node, op_full, ctx),
                    }
                }
                _ => compile_dynamic_call(node, ctx),
            }
        }
    }
}

fn collect_let_locals(node: &TypedExpression, out: &mut Vec<(String, Type)>) {
    if let Expression::Apply(items) = &node.expr {
        if let [Expression::Word(kw), Expression::Word(name), _] = &items[..] {
            if kw == "let" || kw == "letrec" || kw == "mut" {
                if let Some(t) = node.children.get(2).and_then(|n| n.typ.as_ref()) {
                    if !out.iter().any(|(n, _)| n == name) {
                        out.push((name.clone(), t.clone()));
                    }
                }
            }
        }
    }
    for ch in &node.children {
        collect_let_locals(ch, out);
    }
}

fn typed_expr_uses_host_io(node: &TypedExpression) -> bool {
    match &node.expr {
        Expression::Apply(items) if !items.is_empty() => {
            let uses_here = if let Some(Expression::Word(op)) = items.first() {
                op == "read!" ||
                    op == "write!" ||
                    op == "list-dir!" ||
                    op == "mkdir!" ||
                    op == "delete!" ||
                    op == "move!" ||
                    op == "print!" ||
                    op == "sleep!" ||
                    op == "clear!"
            } else {
                false
            };
            uses_here || node.children.iter().any(typed_expr_uses_host_io)
        }
        _ => node.children.iter().any(typed_expr_uses_host_io),
    }
}

fn collect_call_specializations(
    node: &TypedExpression,
    top_def_names: &HashSet<String>,
    out: &mut HashMap<String, (Vec<Type>, Type)>
) {
    if let Expression::Apply(items) = &node.expr {
        if let Some(Expression::Word(name)) = items.first() {
            if top_def_names.contains(name) {
                let params = node.children[1..]
                    .iter()
                    .map(|n| n.typ.clone().unwrap_or(Type::Int))
                    .collect::<Vec<_>>();
                let ret = node.typ.clone().unwrap_or(Type::Int);
                match out.get(name) {
                    Some((prev_params, _)) if prev_params.len() >= params.len() => {}
                    _ => {
                        out.insert(name.clone(), (params, ret));
                    }
                }
            }
        }
    }
    for ch in &node.children {
        collect_call_specializations(ch, top_def_names, out);
    }
}

fn collect_dynamic_partial_specs(
    node: &TypedExpression,
    top_def_names: &HashSet<String>,
    out: &mut HashSet<(usize, usize)>
) {
    if let Expression::Apply(items) = &node.expr {
        if !items.is_empty() && node.children.len() >= 2 {
            let dynamic_word_head = match &items[0] {
                Expression::Word(w) => !is_special_word(w),
                _ => false,
            };
            if dynamic_word_head {
                if let Some(head_ty) = node.children.first().and_then(|n| n.typ.as_ref()) {
                    let (head_params, _head_ret) = function_parts(head_ty);
                    let provided = node.children.len().saturating_sub(1);
                    if provided > 0 && provided < head_params.len() {
                        out.insert((head_params.len(), provided));
                    }
                }
            }
        }
    }
    for ch in &node.children {
        collect_dynamic_partial_specs(ch, top_def_names, out);
    }
}

fn collect_type_subst(pattern: &Type, concrete: &Type, out: &mut HashMap<u64, Type>) {
    match pattern {
        Type::Var(v) => {
            out.entry(v.id).or_insert_with(|| concrete.clone());
        }
        Type::List(a) => {
            if let Type::List(b) = concrete {
                collect_type_subst(a, b, out);
            }
        }
        Type::Tuple(as_) => {
            if let Type::Tuple(bs) = concrete {
                if as_.len() == bs.len() {
                    for (a, b) in as_.iter().zip(bs.iter()) {
                        collect_type_subst(a, b, out);
                    }
                }
            }
        }
        Type::Function(a1, a2) => {
            if let Type::Function(b1, b2) = concrete {
                collect_type_subst(a1, b1, out);
                collect_type_subst(a2, b2, out);
            }
        }
        _ => {}
    }
}

fn apply_type_subst(t: &Type, subst: &HashMap<u64, Type>) -> Type {
    match t {
        Type::Var(v) =>
            subst
                .get(&v.id)
                .cloned()
                .unwrap_or_else(|| Type::Var(v.clone())),
        Type::List(a) => Type::List(Box::new(apply_type_subst(a, subst))),
        Type::Tuple(xs) =>
            Type::Tuple(
                xs
                    .iter()
                    .map(|x| apply_type_subst(x, subst))
                    .collect()
            ),
        Type::Function(a, b) =>
            Type::Function(
                Box::new(apply_type_subst(a, subst)),
                Box::new(apply_type_subst(b, subst))
            ),
        _ => t.clone(),
    }
}

fn specialize_typed_expr(node: &TypedExpression, subst: &HashMap<u64, Type>) -> TypedExpression {
    TypedExpression {
        expr: node.expr.clone(),
        typ: node.typ.as_ref().map(|t| apply_type_subst(t, subst)),
        effect: node.effect,
        children: node.children
            .iter()
            .map(|c| specialize_typed_expr(c, subst))
            .collect(),
    }
}

fn indent_block(code: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    code.lines()
        .map(|l| format!("{pad}{l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn compile_tail_expr(
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    self_name: &str,
    arity: usize
) -> Result<Option<String>, String> {
    match &node.expr {
        Expression::Apply(items) if !items.is_empty() =>
            match &items[0] {
                Expression::Word(op) if op == self_name => {
                    let args = &node.children[1..];
                    if args.len() != arity {
                        return Ok(None);
                    }
                    let mut out = Vec::new();
                    for a in args {
                        out.push(compile_expr(a, ctx)?);
                    }
                    out.push(format!("return_call ${}", ident(self_name)));
                    Ok(Some(out.join("\n")))
                }
                Expression::Word(op) if op == "if" => {
                    let cond_node = node.children
                        .get(1)
                        .ok_or_else(|| "if missing condition".to_string())?;
                    let then_node = node.children
                        .get(2)
                        .ok_or_else(|| "if missing then".to_string())?;
                    let else_node = node.children
                        .get(3)
                        .ok_or_else(|| "if missing else".to_string())?;
                    let cond = compile_expr(cond_node, ctx)?;
                    let result_ty = node.typ
                        .as_ref()
                        .ok_or_else(|| "if missing type".to_string())
                        .and_then(wasm_val_type)?;
                    let then_code = if
                        let Some(tc) = compile_tail_expr(then_node, ctx, self_name, arity)?
                    {
                        tc
                    } else {
                        compile_expr(then_node, ctx)?
                    };
                    let else_code = if
                        let Some(tc) = compile_tail_expr(else_node, ctx, self_name, arity)?
                    {
                        tc
                    } else {
                        compile_expr(else_node, ctx)?
                    };
                    Ok(
                        Some(
                            format!(
                                "{cond}\n(if (result {result_ty})\n  (then\n{}\n  )\n  (else\n{}\n  )\n)\nreturn",
                                indent_block(&then_code, 2),
                                indent_block(&else_code, 2)
                            )
                        )
                    )
                }
                _ => Ok(None),
            }
        _ => Ok(None),
    }
}

fn compile_lambda_func(
    name: &str,
    lambda_expr: &Expression,
    lambda_node: &TypedExpression,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &HashMap<String, i32>,
    lambda_ids: &HashMap<String, i32>,
    closure_defs: &HashMap<String, ClosureDef>,
    lambda_bindings: &HashMap<String, TypedExpression>,
    tail_call_mode: TailCallMode
) -> Result<String, String> {
    let items = match lambda_expr {
        Expression::Apply(xs) => xs,
        _ => {
            return Err(format!("Top def '{}' is not lambda apply", name));
        }
    };
    if items.len() < 2 {
        return Err(format!("lambda '{}' missing body", name));
    }
    let body_idx = items.len() - 1;
    let body_node_raw = lambda_node.children
        .get(body_idx)
        .ok_or_else(|| format!("Missing typed body for '{}'", name))?;
    let sig = fn_sigs.get(name).cloned();
    let mut params = Vec::new();
    for (i, p) in items[1..body_idx].iter().enumerate() {
        if let Expression::Word(w) = p {
            let ty = if let Some((ps, _ret)) = &sig {
                ps
                    .get(i)
                    .cloned()
                    .ok_or_else(|| {
                        format!("Missing specialized param type for '{}' arg {}", name, i)
                    })?
            } else {
                lambda_node.typ
                    .as_ref()
                    .map(function_parts)
                    .and_then(|(ps, _)| ps.get(i).cloned())
                    .ok_or_else(|| format!("Missing param type for '{}' arg {}", name, i))?
            };
            params.push((w.clone(), ty));
        } else {
            return Err(format!("Non-word lambda parameter in '{}'", name));
        }
    }
    let ret_ty = if let Some((_ps, ret)) = sig {
        ret
    } else {
        lambda_node.typ
            .as_ref()
            .map(function_parts)
            .map(|(_, ret)| ret)
            .ok_or_else(|| format!("Missing lambda return type for '{}'", name))?
    };
    let mut subst = HashMap::new();
    if let Some(decl_fn_ty) = lambda_node.typ.as_ref() {
        let (decl_ps, decl_ret) = function_parts(decl_fn_ty);
        for ((_, spec_t), decl_t) in params.iter().zip(decl_ps.iter()) {
            collect_type_subst(decl_t, spec_t, &mut subst);
        }
        collect_type_subst(&decl_ret, &ret_ty, &mut subst);
    }
    let body_node_owned = specialize_typed_expr(body_node_raw, &subst);
    let body_node = &body_node_owned;

    let mut local_defs = Vec::new();
    collect_let_locals(body_node, &mut local_defs);
    local_defs.retain(|(n, _)| !params.iter().any(|(p, _)| p == n));

    let mut locals = HashMap::new();
    for (i, (p, _)) in params.iter().enumerate() {
        locals.insert(p.clone(), i);
    }
    for (i, (n, _)) in local_defs.iter().enumerate() {
        locals.insert(n.clone(), params.len() + i);
    }

    let tmp_i32 = params.len() + local_defs.len();
    let mut scoped_lambda_bindings = lambda_bindings.clone();
    // Function params shadow outer lambda bindings with the same name.
    for (pname, _) in &params {
        scoped_lambda_bindings.remove(pname);
    }
    let mut local_lambda_bindings = HashMap::new();
    collect_let_lambda_bindings(body_node, &mut local_lambda_bindings);
    for (k, v) in local_lambda_bindings {
        scoped_lambda_bindings.insert(k, v);
    }

    let mut local_types = HashMap::new();
    for (p, t) in &params {
        local_types.insert(p.clone(), t.clone());
    }
    for (n, t) in &local_defs {
        local_types.insert(n.clone(), t.clone());
    }
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        lambda_ids,
        closure_defs,
        lambda_bindings: &scoped_lambda_bindings,
        locals,
        local_types,
        tmp_i32,
    };
    let body_code = compile_expr(body_node, &ctx).map_err(|e|
        format!("in lambda '{}': {}", name, e)
    )?;
    let ret_is_ref = is_managed_local_type(&ret_ty);
    let mut ref_slots: Vec<usize> = Vec::new();
    for (i, (_n, t)) in local_defs.iter().enumerate() {
        if is_managed_local_type(t) {
            ref_slots.push(params.len() + i);
        }
    }
    let has_managed_locals = local_defs.iter().any(|(_, t)| is_managed_local_type(t));
    let tco_safe = match tail_call_mode {
        TailCallMode::Conservative => !is_managed_local_type(&ret_ty) && !has_managed_locals,
        TailCallMode::Aggressive => !has_managed_locals,
    };
    let tail_body_code = if tco_safe {
        compile_tail_expr(body_node, &ctx, name, params.len())?
    } else {
        None
    };
    let mut out = String::new();
    out.push_str(&format!("  (func ${}", ident(name)));
    for (_pname, pty) in &params {
        out.push_str(&format!(" (param {})", wasm_val_type(pty)?));
    }
    out.push_str(&format!(" (result {})\n", wasm_val_type(&ret_ty)?));
    for (_n, t) in &local_defs {
        out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    for _ in 0..EXTRA_I32_LOCALS {
        out.push_str("    (local i32)\n");
    }
    if let Some(tail_code) = tail_body_code {
        out.push_str(&format!("    {}\n", tail_code.replace('\n', "\n    ")));
        out.push_str("    unreachable\n");
        out.push_str("  )\n");
        return Ok(out);
    }
    out.push_str(&format!("    (local {})\n", wasm_val_type(&ret_ty)?));
    let ret_slot = params.len() + local_defs.len() + EXTRA_I32_LOCALS;
    out.push_str(&format!("    {}\n", body_code.replace('\n', "\n    ")));
    out.push_str(&format!("    local.set {}\n", ret_slot));
    let scratch_slot = params.len() + local_defs.len();
    out.push_str(&emit_release_unique_refs(&ref_slots, ret_slot, ret_is_ref, scratch_slot));
    out.push_str(&format!("    local.get {}\n", ret_slot));
    out.push_str("  )\n");
    Ok(out)
}

fn compile_closure_func(
    name: &str,
    lambda_node: &TypedExpression,
    captures: &[String],
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &HashMap<String, i32>,
    lambda_ids: &HashMap<String, i32>,
    closure_defs: &HashMap<String, ClosureDef>,
    lambda_bindings: &HashMap<String, TypedExpression>
) -> Result<String, String> {
    let items = match &lambda_node.expr {
        Expression::Apply(xs) => xs,
        _ => {
            return Err(format!("Closure '{}' is not lambda apply", name));
        }
    };
    if items.len() < 2 {
        return Err(format!("Closure '{}' missing body", name));
    }
    let body_idx = items.len() - 1;
    let body_node = lambda_node.children
        .get(body_idx)
        .ok_or_else(|| format!("Missing typed body for closure '{}'", name))?;
    let (all_ps, ret_ty) = fn_sigs
        .get(name)
        .cloned()
        .ok_or_else(|| format!("Missing signature for closure '{}'", name))?;
    if all_ps.len() < captures.len() {
        return Err(format!("Invalid closure signature for '{}'", name));
    }

    let mut params = Vec::new();
    for (i, cap) in captures.iter().enumerate() {
        params.push((cap.clone(), all_ps[i].clone()));
    }
    for (i, p) in items[1..body_idx].iter().enumerate() {
        if let Expression::Word(w) = p {
            let ty = all_ps
                .get(captures.len() + i)
                .cloned()
                .ok_or_else(|| format!("Missing closure param type for '{}' arg {}", name, i))?;
            params.push((w.clone(), ty));
        } else {
            return Err(format!("Non-word lambda parameter in closure '{}'", name));
        }
    }

    let mut local_defs = Vec::new();
    collect_let_locals(body_node, &mut local_defs);
    local_defs.retain(|(n, _)| !params.iter().any(|(p, _)| p == n));

    let mut locals = HashMap::new();
    for (i, (p, _)) in params.iter().enumerate() {
        locals.insert(p.clone(), i);
    }
    for (i, (n, _)) in local_defs.iter().enumerate() {
        locals.insert(n.clone(), params.len() + i);
    }

    let tmp_i32 = params.len() + local_defs.len();
    let mut scoped_lambda_bindings = lambda_bindings.clone();
    // Function params (captures + user params) shadow outer lambda bindings.
    for (pname, _) in &params {
        scoped_lambda_bindings.remove(pname);
    }
    let mut local_lambda_bindings = HashMap::new();
    collect_let_lambda_bindings(body_node, &mut local_lambda_bindings);
    for (k, v) in local_lambda_bindings {
        scoped_lambda_bindings.insert(k, v);
    }

    let mut local_types = HashMap::new();
    for (p, t) in &params {
        local_types.insert(p.clone(), t.clone());
    }
    for (n, t) in &local_defs {
        local_types.insert(n.clone(), t.clone());
    }
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        lambda_ids,
        closure_defs,
        lambda_bindings: &scoped_lambda_bindings,
        locals,
        local_types,
        tmp_i32,
    };
    let body_code = compile_expr(body_node, &ctx).map_err(|e|
        format!("in closure '{}': {}", name, e)
    )?;
    let ret_is_ref = is_managed_local_type(&ret_ty);
    let mut ref_slots: Vec<usize> = Vec::new();
    for (i, (_n, t)) in local_defs.iter().enumerate() {
        if is_managed_local_type(t) {
            ref_slots.push(params.len() + i);
        }
    }

    let mut out = String::new();
    out.push_str(&format!("  (func ${}", ident(name)));
    for (_pname, pty) in &params {
        out.push_str(&format!(" (param {})", wasm_val_type(pty)?));
    }
    out.push_str(&format!(" (result {})\n", wasm_val_type(&ret_ty)?));
    for (_n, t) in &local_defs {
        out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    for _ in 0..EXTRA_I32_LOCALS {
        out.push_str("    (local i32)\n");
    }
    out.push_str(&format!("    (local {})\n", wasm_val_type(&ret_ty)?));
    let ret_slot = params.len() + local_defs.len() + EXTRA_I32_LOCALS;
    out.push_str(&format!("    {}\n", body_code.replace('\n', "\n    ")));
    out.push_str(&format!("    local.set {}\n", ret_slot));
    let scratch_slot = params.len() + local_defs.len();
    out.push_str(&emit_release_unique_refs(&ref_slots, ret_slot, ret_is_ref, scratch_slot));
    out.push_str(&format!("    local.get {}\n", ret_slot));
    out.push_str("  )\n");
    Ok(out)
}

fn emit_release_unique_refs(
    ref_slots: &[usize],
    ret_slot: usize,
    ret_is_ref: bool,
    scratch_slot: usize
) -> String {
    let mut out = String::new();
    for (i, slot) in ref_slots.iter().enumerate() {
        out.push_str("    i32.const 1\n");
        out.push_str(&format!("    local.set {}\n", scratch_slot));
        if ret_is_ref {
            out.push_str(&format!("    local.get {}\n", slot));
            out.push_str(&format!("    local.get {}\n", ret_slot));
            out.push_str("    i32.eq\n");
            out.push_str("    if\n");
            out.push_str("      i32.const 0\n");
            out.push_str(&format!("      local.set {}\n", scratch_slot));
            out.push_str("    end\n");
        }
        for prev in ref_slots.iter().take(i) {
            out.push_str(&format!("    local.get {}\n", scratch_slot));
            out.push_str("    if\n");
            out.push_str(&format!("      local.get {}\n", slot));
            out.push_str(&format!("      local.get {}\n", prev));
            out.push_str("      i32.eq\n");
            out.push_str("      if\n");
            out.push_str("        i32.const 0\n");
            out.push_str(&format!("        local.set {}\n", scratch_slot));
            out.push_str("      end\n");
            out.push_str("    end\n");
        }
        out.push_str(&format!("    local.get {}\n", scratch_slot));
        out.push_str("    if\n");
        out.push_str(&format!("      local.get {}\n", slot));
        out.push_str("      call $rc_release\n");
        out.push_str("      drop\n");
        out.push_str("    end\n");
    }
    out
}

fn compile_value_func(
    name: &str,
    value_node: &TypedExpression,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &HashMap<String, i32>,
    lambda_ids: &HashMap<String, i32>,
    closure_defs: &HashMap<String, ClosureDef>,
    lambda_bindings: &HashMap<String, TypedExpression>
) -> Result<String, String> {
    let ret_ty = value_node.typ
        .as_ref()
        .ok_or_else(|| format!("Missing value type for '{}'", name))?;

    let mut local_defs = Vec::new();
    collect_let_locals(value_node, &mut local_defs);
    let mut locals = HashMap::new();
    for (i, (n, _)) in local_defs.iter().enumerate() {
        locals.insert(n.clone(), i);
    }
    let tmp_i32 = local_defs.len();
    let mut scoped_lambda_bindings = lambda_bindings.clone();
    let mut local_lambda_bindings = HashMap::new();
    collect_let_lambda_bindings(value_node, &mut local_lambda_bindings);
    for (k, v) in local_lambda_bindings {
        scoped_lambda_bindings.insert(k, v);
    }

    let mut local_types = HashMap::new();
    for (n, t) in &local_defs {
        local_types.insert(n.clone(), t.clone());
    }
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        lambda_ids,
        closure_defs,
        lambda_bindings: &scoped_lambda_bindings,
        locals,
        local_types,
        tmp_i32,
    };
    let body_code = compile_expr(value_node, &ctx).map_err(|e|
        format!("in value '{}': {}", name, e)
    )?;
    let ret_is_ref = is_managed_local_type(ret_ty);
    let ref_slots: Vec<usize> = local_defs
        .iter()
        .enumerate()
        .filter_map(|(i, (_n, t))| {
            if is_managed_local_type(t) { Some(i) } else { None }
        })
        .collect();

    let mut out = String::new();
    out.push_str(&format!("  (func ${} (result {})\n", ident(name), wasm_val_type(ret_ty)?));
    for (_n, t) in &local_defs {
        out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    for _ in 0..EXTRA_I32_LOCALS {
        out.push_str("    (local i32)\n");
    }
    out.push_str(&format!("    (local {})\n", wasm_val_type(ret_ty)?));
    let ret_slot = local_defs.len() + EXTRA_I32_LOCALS;
    let scratch_slot = local_defs.len();
    let g_init = cache_init_global(name);
    let g_val = cache_value_global(name);

    out.push_str(&format!("    global.get ${}\n", g_init));
    out.push_str("    if\n");
    out.push_str(&format!("      global.get ${}\n", g_val));
    out.push_str(&format!("      local.set {}\n", ret_slot));
    if ret_is_ref {
        out.push_str(&format!("      local.get {}\n", ret_slot));
        out.push_str("      call $rc_retain\n");
        out.push_str("      drop\n");
    }
    out.push_str("    else\n");
    out.push_str(&format!("      {}\n", body_code.replace('\n', "\n      ")));
    out.push_str(&format!("      local.set {}\n", ret_slot));
    out.push_str(
        &indent_block(&emit_release_unique_refs(&ref_slots, ret_slot, ret_is_ref, scratch_slot), 6)
    );
    out.push('\n');
    if ret_is_ref {
        // Keep one root reference in the global cache while returning one to caller.
        out.push_str(&format!("      local.get {}\n", ret_slot));
        out.push_str("      call $rc_retain\n");
        out.push_str("      drop\n");
    }
    out.push_str(&format!("      local.get {}\n", ret_slot));
    out.push_str(&format!("      global.set ${}\n", g_val));
    out.push_str("      i32.const 1\n");
    out.push_str(&format!("      global.set ${}\n", g_init));
    out.push_str("    end\n");
    out.push_str(&format!("    local.get {}\n", ret_slot));
    out.push_str("  )\n");
    Ok(out)
}

fn compile_value_func_fn_ptr(name: &str, fn_id: i32) -> String {
    format!("  (func ${} (result i32)\n    i32.const {}\n  )\n", ident(name), fn_id)
}

fn compile_partial_helper_func(
    h: &PartialHelper,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &HashMap<String, i32>,
    lambda_ids: &HashMap<String, i32>,
    closure_defs: &HashMap<String, ClosureDef>,
    lambda_bindings: &HashMap<String, TypedExpression>
) -> Result<String, String> {
    let mut locals = HashMap::new();
    for i in 0..h.remaining_params.len() {
        locals.insert(format!("__p{}", i), i);
    }
    let mut local_types = HashMap::new();
    for (i, t) in h.remaining_params.iter().enumerate() {
        local_types.insert(format!("__p{}", i), t.clone());
    }
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        lambda_ids,
        closure_defs,
        lambda_bindings,
        locals,
        local_types,
        tmp_i32: h.remaining_params.len(),
    };

    let mut body_parts = Vec::new();
    for c in &h.captured_nodes {
        body_parts.push(compile_expr(c, &ctx)?);
    }
    for i in 0..h.remaining_params.len() {
        body_parts.push(format!("local.get {}", i));
    }
    body_parts.push(format!("call ${}", ident(&h.target_name)));

    let mut out = String::new();
    out.push_str(&format!("  (func ${}", ident(&h.helper_name)));
    for p in &h.remaining_params {
        out.push_str(&format!(" (param {})", wasm_val_type(p)?));
    }
    out.push_str(&format!(" (result {})\n", wasm_val_type(&h.ret)?));
    for _ in 0..EXTRA_I32_LOCALS {
        out.push_str("    (local i32)\n");
    }
    out.push_str(&format!("    {}\n", body_parts.join("\n    ")));
    out.push_str("  )\n");
    Ok(out)
}

fn compile_dynamic_partial_helper_func(h: &DynamicPartialHelper) -> String {
    let mut out = String::new();
    out.push_str(&format!("  (func ${}", ident(&h.name)));
    for _ in 0..1 + h.total_arity {
        out.push_str(" (param i32)");
    }
    out.push_str(" (result i32)\n");
    for _ in 0..EXTRA_I32_LOCALS {
        out.push_str("    (local i32)\n");
    }
    out.push_str("    local.get 0\n");
    for i in 1..=h.total_arity {
        out.push_str(&format!("    local.get {}\n", i));
    }
    out.push_str(&format!("    call $apply{}_i32\n", h.total_arity));
    out.push_str("  )\n");
    out
}

pub fn compile_program_to_wat_typed_with_opts(
    typed_ast: &TypedExpression,
    enable_optimizer: bool
) -> Result<String, String> {
    // Validate devirtualization mode early so invalid env values fail deterministically.
    let _ = devirtualize_mode_from_env()?;
    let tail_call_mode = tail_call_mode_from_env()?;
    let optimized_typed_ast = if enable_optimizer {
        Some(crate::op::optimize_typed_ast(typed_ast))
    } else {
        None
    };
    let typed_ast = optimized_typed_ast.as_ref().unwrap_or(typed_ast);

    let (top_defs, main_expr, main_node) = match &typed_ast.expr {
        Expression::Apply(items) if
            matches!(items.first(), Some(Expression::Word(w)) if w == "do")
        => {
            let child_offset = if typed_ast.children.len() + 1 == items.len() { 1 } else { 0 };
            let child_at = |item_idx: usize| -> Option<&TypedExpression> {
                if item_idx < child_offset {
                    None
                } else {
                    typed_ast.children.get(item_idx - child_offset)
                }
            };
            let mut defs = HashMap::new();
            let mut main_items_expr = vec![Expression::Word("do".to_string())];
            let mut main_items_nodes: Vec<TypedExpression> = Vec::new();
            for i in 1..items.len() {
                if let Expression::Apply(let_items) = &items[i] {
                    if let [Expression::Word(kw), Expression::Word(name), rhs] = &let_items[..] {
                        if kw == "let" || kw == "letrec" {
                            if
                                let Some(node) = child_at(i)
                                    .and_then(|n| n.children.get(2))
                                    .cloned()
                            {
                                defs.insert(name.clone(), TopDef {
                                    expr: rhs.clone(),
                                    node,
                                });
                                // Top-level bindings are canonicalized as defs and referenced by name.
                                // Do not also keep duplicate let expressions in main.
                                continue;
                            }
                        }
                    }
                }
                main_items_expr.push(items[i].clone());
                let node = child_at(i)
                    .cloned()
                    .ok_or_else(|| {
                        "Missing typed top-level expression while building wasm main".to_string()
                    })?;
                main_items_nodes.push(node);
            }
            if main_items_nodes.is_empty() {
                main_items_expr.push(Expression::Int(0));
                main_items_nodes.push(TypedExpression {
                    expr: Expression::Int(0),
                    typ: Some(Type::Int),
                    effect: EffectFlags::PURE,
                    children: Vec::new(),
                });
            }
            let main_expr = Expression::Apply(main_items_expr);
            let main_typ = main_items_nodes.last().and_then(|n| n.typ.clone());
            let main_effect = main_items_nodes
                .iter()
                .fold(EffectFlags::PURE, |acc, n| acc | n.effect);
            let main_node = TypedExpression {
                expr: main_expr.clone(),
                typ: main_typ,
                effect: main_effect,
                children: main_items_nodes,
            };
            (defs, main_expr, main_node)
        }
        _ => (HashMap::new(), typed_ast.expr.clone(), typed_ast.clone()),
    };
    let _needs_host_io = typed_expr_uses_host_io(typed_ast);

    let mut needed = HashSet::new();
    let mut bound = HashSet::new();
    collect_refs(&main_expr, &mut bound, &mut needed);

    let mut stack: Vec<String> = needed.iter().cloned().collect();
    while let Some(name) = stack.pop() {
        if let Some(def) = top_defs.get(&name) {
            let mut refs = HashSet::new();
            let mut b = HashSet::new();
            if let Expression::Apply(items) = &def.expr {
                if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
                    for p in &items[1..items.len().saturating_sub(1)] {
                        if let Expression::Word(n) = p {
                            b.insert(n.clone());
                        }
                    }
                    if let Some(body) = items.last() {
                        collect_refs(body, &mut b, &mut refs);
                    }
                } else {
                    collect_refs(&def.expr, &mut b, &mut refs);
                }
            } else {
                collect_refs(&def.expr, &mut b, &mut refs);
            }
            for r in refs {
                if !needed.contains(&r) {
                    needed.insert(r.clone());
                    stack.push(r);
                }
            }
        }
    }
    // Keep all top-level std/user defs available to avoid lookup misses for scoped aliases
    // (e.g. `(let =! (lambda ...))`) under higher-order/transformed call shapes.
    for name in top_defs.keys() {
        needed.insert(name.clone());
    }

    let mut fn_sigs: HashMap<String, (Vec<Type>, Type)> = HashMap::new();
    let mut top_level_lambda_key_to_name: HashMap<String, String> = HashMap::new();
    let top_def_names: HashSet<String> = top_defs.keys().cloned().collect();
    let mut dynamic_partial_specs: HashSet<(usize, usize)> = HashSet::new();
    collect_dynamic_partial_specs(typed_ast, &top_def_names, &mut dynamic_partial_specs);
    let mut call_specs: HashMap<String, (Vec<Type>, Type)> = HashMap::new();
    collect_call_specializations(typed_ast, &top_def_names, &mut call_specs);
    for (name, def) in &top_defs {
        let is_lambda_def =
            matches!(
            &def.expr,
            Expression::Apply(items)
                if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
        );
        let (ps, ret) = if is_lambda_def {
            let t = def.node.typ
                .as_ref()
                .ok_or_else(|| format!("Missing type for def '{}'", name))?;
            let (mut decl_ps, decl_ret) = function_parts(t);
            let syn_arity = lambda_syntax_arity(&def.expr);
            if syn_arity == 0 && decl_ps.len() == 1 && matches!(decl_ps[0], Type::Unit) {
                decl_ps.clear();
            } else if decl_ps.len() >= syn_arity {
                decl_ps.truncate(syn_arity);
            }
            (decl_ps, decl_ret)
        } else {
            let t = def.node.typ
                .as_ref()
                .ok_or_else(|| format!("Missing type for def '{}'", name))?;
            (Vec::new(), t.clone())
        };
        for p in &ps {
            wasm_val_type(p)?;
        }
        wasm_val_type(&ret)?;
        fn_sigs.insert(name.clone(), (ps, ret));
        if is_lambda_def {
            top_level_lambda_key_to_name.insert(def.expr.to_lisp(), name.clone());
            top_level_lambda_key_to_name.insert(def.node.expr.to_lisp(), name.clone());
        }
    }
    let mut lambda_nodes = Vec::new();
    collect_lambda_nodes(typed_ast, &mut lambda_nodes);
    let mut lambda_bindings: HashMap<String, TypedExpression> = HashMap::new();
    collect_let_lambda_bindings(typed_ast, &mut lambda_bindings);
    let mut lambda_names: HashMap<String, String> = HashMap::new();
    let mut closure_defs: HashMap<String, ClosureDef> = HashMap::new();
    let mut dynamic_partial_helpers: Vec<DynamicPartialHelper> = Vec::new();
    let mut lambda_ids: HashMap<String, i32> = HashMap::new();
    let mut next_lambda_idx = 0i32;
    let mut next_closure_idx = 0i32;
    for node in &lambda_nodes {
        let key = node.expr.to_lisp();
        if top_level_lambda_key_to_name.contains_key(&key) {
            continue;
        }
        if lambda_names.contains_key(&key) || closure_defs.contains_key(&key) {
            continue;
        }
        if lambda_is_hoistable(node, &top_defs) {
            let name = format!("__lambda{}", next_lambda_idx);
            next_lambda_idx += 1;
            lambda_names.insert(key.clone(), name.clone());
            if let Some(t) = node.typ.as_ref() {
                let (mut ps, ret) = function_parts(t);
                let syn_arity = lambda_syntax_arity(&node.expr);
                if syn_arity == 0 && ps.len() == 1 && matches!(ps[0], Type::Unit) {
                    ps.clear();
                } else if ps.len() >= syn_arity {
                    ps.truncate(syn_arity);
                }
                fn_sigs.insert(name.clone(), (ps, ret));
            }
        } else {
            let name = format!("__closure_lambda{}", next_closure_idx);
            next_closure_idx += 1;
            if let Some(t) = node.typ.as_ref() {
                let (mut ps, ret) = function_parts(t);
                let syn_arity = lambda_syntax_arity(&node.expr);
                if syn_arity == 0 && ps.len() == 1 && matches!(ps[0], Type::Unit) {
                    ps.clear();
                } else if ps.len() >= syn_arity {
                    ps.truncate(syn_arity);
                }
                let captures = lambda_capture_names(node, &top_defs);
                let mut all_ps = vec![Type::Int; captures.len()];
                all_ps.extend(ps.clone());
                fn_sigs.insert(name.clone(), (all_ps, ret));
                closure_defs.insert(key.clone(), ClosureDef {
                    key,
                    name,
                    captures,
                    user_arity: ps.len(),
                });
            }
        }
    }

    // Runtime apply1 fallback can synthesize partial closures for callable arities > 1.
    // Ensure those dynamic helper functions are always available.
    for (_name, (ps, ret)) in &fn_sigs {
        if ps.len() > 1 && ps.iter().all(is_i32ish_type) && is_i32ish_type(ret) {
            dynamic_partial_specs.insert((ps.len(), 1));
        }
    }
    for tag in [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 21, 25, 26, 27, 28, 29, 30, 31, 32,
        33, 34,
    ] {
        if let Some(arity) = builtin_tag_arity(tag) {
            if arity > 1 {
                dynamic_partial_specs.insert((arity, 1));
            }
        }
    }

    let mut dynamic_partial_specs_sorted = dynamic_partial_specs.into_iter().collect::<Vec<_>>();
    dynamic_partial_specs_sorted.sort_unstable();
    for (total, provided) in dynamic_partial_specs_sorted {
        let name = format!("__partial_dyn_{}_{}", total, provided);
        if fn_sigs.contains_key(&name) {
            continue;
        }
        // __partial_dyn_N_K signature is:
        //   (fn_ptr, arg0, arg1, ..., argN-1) -> i32
        // The first param is always a function value and must be treated as
        // a managed reference so closure captures retain/release correctly.
        let mut helper_params = Vec::with_capacity(1 + total);
        helper_params.push(Type::Function(Box::new(Type::Int), Box::new(Type::Int)));
        helper_params.extend(std::iter::repeat(Type::Int).take(total));
        fn_sigs.insert(name.clone(), (helper_params, Type::Int));
        let cap_count = 1 + provided;
        let captures = (0..cap_count).map(|i| format!("__cap{}", i)).collect::<Vec<_>>();
        let key = format!("__partial_dyn_key_{}_{}", total, provided);
        closure_defs.insert(key.clone(), ClosureDef {
            key,
            name: name.clone(),
            captures,
            user_arity: total - provided,
        });
        dynamic_partial_helpers.push(DynamicPartialHelper {
            name,
            total_arity: total,
        });
    }

    // Compile-time partial application lowering for top-level value bindings:
    // (let mod2 (k-mod 2)) => helper function equivalent to (lambda x (k-mod 2 x))
    let mut partial_helpers: Vec<PartialHelper> = Vec::new();
    for (name, def) in &top_defs {
        let rhs_items = match &def.expr {
            Expression::Apply(xs) => xs,
            _ => {
                continue;
            }
        };
        let target_name = match rhs_items.first() {
            Some(Expression::Word(w)) => w.clone(),
            _ => {
                continue;
            }
        };
        let (target_params, target_ret) = match fn_sigs.get(&target_name) {
            Some((ps, ret)) if !ps.is_empty() => (ps.clone(), ret.clone()),
            _ => {
                continue;
            }
        };
        let provided = rhs_items.len().saturating_sub(1);
        if provided >= target_params.len() {
            continue;
        }
        let captured_nodes = if def.node.children.len() > 1 {
            def.node.children[1..].to_vec()
        } else {
            Vec::new()
        };
        if captured_nodes.len() != provided {
            continue;
        }
        let helper_name = format!("__partial_top_{}", name);
        let remaining_params = target_params[provided..].to_vec();
        partial_helpers.push(PartialHelper {
            binding_name: name.clone(),
            helper_name: helper_name.clone(),
            target_name: target_name.clone(),
            captured_nodes,
            remaining_params: remaining_params.clone(),
            ret: target_ret.clone(),
        });
        fn_sigs.insert(helper_name, (remaining_params, target_ret));
    }
    let mut fn_ids: HashMap<String, i32> = HashMap::new();
    let mut next_fn_id = 100i32;
    for (name, (ps, _ret)) in &fn_sigs {
        if !ps.is_empty() {
            fn_ids.insert(name.clone(), next_fn_id);
            next_fn_id += 1;
        }
    }
    for (_k, name) in &lambda_names {
        if fn_ids.contains_key(name) {
            continue;
        }
        if fn_sigs.contains_key(name) {
            fn_ids.insert(name.clone(), next_fn_id);
            next_fn_id += 1;
        }
    }
    for (key, name) in &top_level_lambda_key_to_name {
        if let Some(id) = fn_ids.get(name) {
            lambda_ids.insert(key.clone(), *id);
        }
    }
    for (key, name) in &lambda_names {
        if let Some(id) = fn_ids.get(name) {
            lambda_ids.insert(key.clone(), *id);
        }
    }
    let main_ret_ty = main_node.typ
        .as_ref()
        .ok_or_else(|| "Missing main expression type".to_string())?;
    let mut emitted_funcs: Vec<String> = Vec::new();
    let mut cached_value_defs: Vec<String> = Vec::new();

    for (name, def) in &top_defs {
        if partial_helpers.iter().any(|h| h.binding_name == *name) {
            continue;
        }
        match &def.expr {
            Expression::Apply(items) if
                matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
            => {
                emitted_funcs.push(
                    compile_lambda_func(
                        name,
                        &def.expr,
                        &def.node,
                        &fn_sigs,
                        &fn_ids,
                        &lambda_ids,
                        &closure_defs,
                        &lambda_bindings,
                        tail_call_mode
                    )?
                );
            }
            _ => {
                cached_value_defs.push(name.clone());
                emitted_funcs.push(
                    compile_value_func(
                        name,
                        &def.node,
                        &fn_sigs,
                        &fn_ids,
                        &lambda_ids,
                        &closure_defs,
                        &lambda_bindings
                    )?
                );
            }
        }
    }
    for h in &partial_helpers {
        emitted_funcs.push(
            compile_partial_helper_func(
                h,
                &fn_sigs,
                &fn_ids,
                &lambda_ids,
                &closure_defs,
                &lambda_bindings
            )?
        );
    }
    for h in &dynamic_partial_helpers {
        emitted_funcs.push(compile_dynamic_partial_helper_func(h));
    }
    for h in &partial_helpers {
        let helper_id = fn_ids
            .get(&h.helper_name)
            .copied()
            .ok_or_else(|| format!("Missing function id for helper '{}'", h.helper_name))?;
        emitted_funcs.push(compile_value_func_fn_ptr(&h.binding_name, helper_id));
    }
    let mut emitted_hoisted_lambda_names: HashSet<String> = HashSet::new();
    for node in &lambda_nodes {
        let key = node.expr.to_lisp();
        if let Some(name) = lambda_names.get(&key) {
            if !emitted_hoisted_lambda_names.insert(name.clone()) {
                continue;
            }
            emitted_funcs.push(
                compile_lambda_func(
                    name,
                    &node.expr,
                    node,
                    &fn_sigs,
                    &fn_ids,
                    &lambda_ids,
                    &closure_defs,
                    &lambda_bindings,
                    tail_call_mode
                )?
            );
        }
    }
    for def in closure_defs.values() {
        if let Some(node) = lambda_nodes.iter().find(|n| n.expr.to_lisp() == def.key) {
            emitted_funcs.push(
                compile_closure_func(
                    &def.name,
                    node,
                    &def.captures,
                    &fn_sigs,
                    &fn_ids,
                    &lambda_ids,
                    &closure_defs,
                    &lambda_bindings
                )?
            );
        }
    }

    let main_wasm_ty = wasm_val_type(main_ret_ty)?;

    let mut main_local_defs = Vec::new();
    collect_let_locals(&main_node, &mut main_local_defs);
    let mut main_locals = HashMap::new();
    for (i, (n, _)) in main_local_defs.iter().enumerate() {
        main_locals.insert(n.clone(), i);
    }

    let mut scoped_lambda_bindings = lambda_bindings.clone();
    let mut local_lambda_bindings = HashMap::new();
    collect_let_lambda_bindings(&main_node, &mut local_lambda_bindings);
    for (k, v) in local_lambda_bindings {
        scoped_lambda_bindings.insert(k, v);
    }

    let mut main_local_types = HashMap::new();
    for (n, t) in &main_local_defs {
        main_local_types.insert(n.clone(), t.clone());
    }
    let main_ctx = Ctx {
        fn_sigs: &fn_sigs,
        fn_ids: &fn_ids,
        lambda_ids: &lambda_ids,
        closure_defs: &closure_defs,
        lambda_bindings: &scoped_lambda_bindings,
        locals: main_locals,
        local_types: main_local_types,
        tmp_i32: main_local_defs.len(),
    };
    let main_code = compile_expr(&main_node, &main_ctx)?;
    let mut apply_arities: HashSet<usize> = HashSet::new();
    for func in &emitted_funcs {
        collect_apply_arities_from_code(func, &mut apply_arities);
    }
    collect_apply_arities_from_code(&main_code, &mut apply_arities);

    let mut main_func = String::new();
    main_func.push_str(&format!("  ;; Type: {}\n", main_ret_ty));
    main_func.push_str(&format!("  (func (export \"main\") (result {main_wasm_ty})\n"));
    for (_n, t) in &main_local_defs {
        main_func.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    for _ in 0..EXTRA_I32_LOCALS {
        main_func.push_str("    (local i32)\n");
    }
    main_func.push_str(&format!("    {}\n", main_code.replace('\n', "\n    ")));
    main_func.push_str("  )\n");

    let mut wat = String::new();
    wat.push_str(&format!(";; Type: {}\n", main_ret_ty));
    wat.push_str("(module\n");
    if _needs_host_io {
        wat.push_str(
            "  (import \"host\" \"list_dir\" (func $host_list_dir (param i32) (result i32)))\n"
        );
        wat.push_str(
            "  (import \"host\" \"read_file\" (func $host_read_file (param i32) (result i32)))\n"
        );
        wat.push_str(
            "  (import \"host\" \"write_file\" (func $host_write_file (param i32 i32) (result i32)))\n"
        );
        wat.push_str(
            "  (import \"host\" \"mkdir_p\" (func $host_mkdir_p (param i32) (result i32)))\n"
        );
        wat.push_str(
            "  (import \"host\" \"delete\" (func $host_delete (param i32) (result i32)))\n"
        );
        wat.push_str(
            "  (import \"host\" \"move\" (func $host_move (param i32 i32) (result i32)))\n"
        );
        wat.push_str("  (import \"host\" \"print\" (func $host_print (param i32) (result i32)))\n");
        wat.push_str("  (import \"host\" \"sleep\" (func $host_sleep (param i32) (result i32)))\n");
        wat.push_str("  (import \"host\" \"clear\" (func $host_clear (result i32)))\n");
    }
    for name in &cached_value_defs {
        wat.push_str(&format!("  (global ${} (mut i32) (i32.const 0))\n", cache_init_global(name)));
        wat.push_str(
            &format!("  (global ${} (mut i32) (i32.const 0))\n", cache_value_global(name))
        );
    }
    wat.push_str(&emit_vector_runtime(&fn_ids, &fn_sigs, &closure_defs, &apply_arities));
    for func in emitted_funcs {
        wat.push_str(&func);
    }
    wat.push_str(&main_func);
    wat.push_str(")\n");

    Ok(wat)
}

pub fn compile_program_to_wat_typed(typed_ast: &TypedExpression) -> Result<String, String> {
    compile_program_to_wat_typed_with_opts(typed_ast, true)
}

pub fn compile_program_to_wat_with_opts(
    expr: &Expression,
    enable_optimizer: bool
) -> Result<String, String> {
    let (_typ, typed_ast) = crate::infer::infer_with_builtins_typed(
        expr,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
    )?;
    compile_program_to_wat_typed_with_opts(&typed_ast, enable_optimizer)
}

pub fn compile_program_to_wat(expr: &Expression) -> Result<String, String> {
    compile_program_to_wat_with_opts(expr, true)
}
