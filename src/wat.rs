use crate::infer::{EffectFlags, TypedExpression};
use crate::parser::{DecimalLiteral, Expression};
use crate::types::Type;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Default)]
struct WasiOverrides {
    host: Option<bool>,
    no_result: Option<bool>,
    allow: Option<String>,
}

thread_local! {
    static WASI_OVERRIDES: RefCell<WasiOverrides> = RefCell::new(WasiOverrides::default());
    static STATIC_PROOF_FACTS: RefCell<Option<crate::static_analysis::ProofFacts>> =
        const { RefCell::new(None) };
}

struct StaticProofFactsGuard(Option<crate::static_analysis::ProofFacts>);

impl StaticProofFactsGuard {
    fn install(typed_ast: &TypedExpression) -> Self {
        STATIC_PROOF_FACTS.with(|cell| {
            let previous = cell.replace(Some(crate::static_analysis::ProofFacts::from_typed_ast(
                typed_ast,
            )));
            Self(previous)
        })
    }
}

impl Drop for StaticProofFactsGuard {
    fn drop(&mut self) {
        STATIC_PROOF_FACTS.with(|cell| {
            cell.replace(self.0.take());
        });
    }
}

fn static_proof_is_safe(kind: crate::static_analysis::ProofKind, expr: &Expression) -> bool {
    STATIC_PROOF_FACTS.with(|cell| {
        cell.borrow()
            .as_ref()
            .is_some_and(|facts| facts.is_proven_safe(kind, expr))
    })
}

pub struct WasiOverrideGuard(WasiOverrides);

pub fn scoped_wasi_override(key: &str, value: &str) -> Option<WasiOverrideGuard> {
    WASI_OVERRIDES.with(|cell| {
        let previous = cell.borrow().clone();
        let mut current = previous.clone();
        match key {
            "QUE_WASI_HOST" => current.host = Some(matches!(value, "1" | "true" | "on" | "yes")),
            "QUE_WASI_NO_RESULT" => {
                current.no_result = Some(matches!(value, "1" | "true" | "on" | "yes"))
            }
            "QUE_WASI_ALLOW" => current.allow = Some(value.to_string()),
            _ => return None,
        }
        *cell.borrow_mut() = current;
        Some(WasiOverrideGuard(previous))
    })
}

impl Drop for WasiOverrideGuard {
    fn drop(&mut self) {
        WASI_OVERRIDES.with(|cell| *cell.borrow_mut() = self.0.clone());
    }
}

fn wasi_bool(key: &str) -> bool {
    let overridden = WASI_OVERRIDES.with(|cell| match key {
        "QUE_WASI_HOST" => cell.borrow().host,
        "QUE_WASI_NO_RESULT" => cell.borrow().no_result,
        _ => None,
    });
    overridden.unwrap_or_else(|| {
        std::env::var(key)
            .map(|value| matches!(value.as_str(), "1" | "true" | "on" | "yes"))
            .unwrap_or(false)
    })
}

fn wasi_allow() -> Option<String> {
    WASI_OVERRIDES
        .with(|cell| cell.borrow().allow.clone())
        .or_else(|| std::env::var("QUE_WASI_ALLOW").ok())
}

#[derive(Clone)]
struct TopDef {
    expr: Expression,
    node: TypedExpression,
}

pub struct SplitWatModules {
    pub runtime_wat: String,
    pub user_wat: String,
}

struct WatBuildOutput {
    monolithic_wat: String,
    split: SplitWatModules,
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

#[derive(Clone, Default)]
struct RcCycleCheckEnv {
    managed_roots: HashMap<String, String>,
    closure_captures: HashMap<String, HashSet<String>>,
    storage_summaries: HashMap<String, StorageSummary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StorageSummary {
    target_param: usize,
    value_param: usize,
}

struct Ctx<'a> {
    fn_sigs: &'a HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &'a HashMap<String, i32>,
    extern_names: &'a HashSet<String>,
    lambda_ids: &'a HashMap<String, i32>,
    closure_defs: &'a HashMap<String, ClosureDef>,
    lambda_bindings: &'a HashMap<String, TypedExpression>,
    current_function: Option<&'a str>,
    locals: HashMap<String, usize>,
    local_types: HashMap<String, Type>,
    materialized_scalar_local_slots: HashSet<usize>,
    hoisted_scalar_vec_data_slots: HashMap<usize, usize>,
    proven_scalar_vec_min_lengths: HashMap<usize, i32>,
    definitely_materialized_top_level_scalar_names: &'a HashSet<String>,
    proven_scalar_index_loads: &'a HashSet<(String, String)>,
    nonnegative_int_locals: &'a HashSet<String>,
    tmp_i32: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DevirtualizeMode {
    Off,
    KnownHeads,
    Aggressive,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TailCallMode {
    Off,
    Conservative,
    Aggressive,
}

fn max_local_index_in_code(code: &str) -> Option<usize> {
    let tokens: Vec<&str> = code.split_whitespace().collect();
    let mut max_idx = None;
    let mut i = 0usize;
    while i + 1 < tokens.len() {
        match tokens[i] {
            "local.get" | "local.set" | "local.tee" => {
                if let Ok(idx) = tokens[i + 1].parse::<usize>() {
                    max_idx = Some(max_idx.map_or(idx, |m: usize| m.max(idx)));
                }
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    max_idx
}

fn scratch_i32_locals_needed(
    base_local_count: usize,
    codes: &[&str],
    needs_release_scratch: bool,
) -> usize {
    let body_needed = codes
        .iter()
        .filter_map(|code| max_local_index_in_code(code))
        .filter(|idx| *idx >= base_local_count)
        .map(|idx| idx - base_local_count + 1)
        .max()
        .unwrap_or(0);
    if needs_release_scratch {
        body_needed.max(1)
    } else {
        body_needed
    }
}

fn emit_i32_locals(out: &mut String, count: usize) {
    for _ in 0..count {
        out.push_str("    (local i32)\n");
    }
}

#[derive(Clone, Copy)]
struct ArithmeticCheckConfig {
    int_overflow_check: bool,
    dec_overflow_check: bool,
    div_zero_check: bool,
}

const DBG_GUARD_TRAP_INT_DIV_ZERO: i32 = 1;
const DBG_GUARD_TRAP_DEC_DIV_ZERO: i32 = 2;
const DBG_GUARD_TRAP_INT_OVERFLOW_ADD: i32 = 3;
const DBG_GUARD_TRAP_INT_OVERFLOW_SUB: i32 = 4;
const DBG_GUARD_TRAP_INT_OVERFLOW_MUL: i32 = 5;
const DBG_GUARD_TRAP_DEC_OVERFLOW: i32 = 6;
fn decimal_scale_i32() -> i32 {
    match std::env::var("QUE_DECIMAL_SCALE")
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

fn decimal_literal_i32(value: &DecimalLiteral) -> Result<i32, String> {
    let scaled = value
        .scaled_i64(decimal_scale_i64())
        .ok_or_else(|| format!("Dec literal `{value}` is too large"))?;
    i32::try_from(scaled).map_err(|_| {
        format!(
            "Dec literal `{value}` is outside the range supported at scale {}",
            decimal_scale_i64()
        )
    })
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
    let report = if wasi_bool("QUE_WASI_HOST") {
        format!("\ni32.const {code}\ncall $__wasi_guard_trap")
    } else {
        String::new()
    };
    format!("i32.const {code}\nglobal.set $dbg_guard_trap_code{report}\nunreachable")
}

fn emit_wasi_guard_trap_runtime() -> String {
    let messages = [
        "debug.guard_trap: unknown guard trap\n",
        "debug.guard_trap: integer divide/modulo by zero (QUE_DIV_ZERO_CHECK)\n",
        "debug.guard_trap: dec divide by zero (QUE_DIV_ZERO_CHECK)\n",
        "debug.guard_trap: integer overflow on add/inc (QUE_INT_OVERFLOW_CHECK)\n",
        "debug.guard_trap: integer overflow on sub/dec (QUE_INT_OVERFLOW_CHECK)\n",
        "debug.guard_trap: integer overflow on mul/square (QUE_INT_OVERFLOW_CHECK)\n",
        "debug.guard_trap: dec overflow (QUE_DEC_OVERFLOW_CHECK)\n",
    ];
    let mut out = String::new();
    let mut offsets = Vec::new();
    let mut address = 62_000usize;
    for message in messages {
        offsets.push((address, message.len()));
        let escaped = message.replace('\n', "\\0a");
        out.push_str(&format!("  (data (i32.const {address}) \"{escaped}\")\n"));
        address += message.len();
    }
    out.push_str(
        "  (func $__wasi_guard_trap (param $code i32)\n    (local $ptr i32) (local $len i32)\n",
    );
    out.push_str(&format!(
        "    i32.const {}\n    local.set $ptr\n    i32.const {}\n    local.set $len\n",
        offsets[0].0, offsets[0].1
    ));
    for (code, (ptr, len)) in offsets.iter().enumerate().skip(1) {
        out.push_str(&format!(
            "    local.get $code\n    i32.const {code}\n    i32.eq\n    if\n      i32.const {ptr}\n      local.set $ptr\n      i32.const {len}\n      local.set $len\n    end\n"
        ));
    }
    out.push_str(
        "    i32.const 0\n    local.get $ptr\n    i32.store\n    i32.const 4\n    local.get $len\n    i32.store\n    i32.const 2\n    i32.const 0\n    i32.const 1\n    i32.const 16\n    call $__wasi_fd_write\n    drop\n    i32.const 70\n    call $__wasi_proc_exit\n    unreachable)\n",
    );
    out
}

fn parse_env_bool_like(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| {
            !matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off" | "no"
            )
        })
        .unwrap_or(default)
}

fn arithmetic_check_config() -> ArithmeticCheckConfig {
    ArithmeticCheckConfig {
        int_overflow_check: parse_env_bool_like("QUE_INT_OVERFLOW_CHECK", false),
        dec_overflow_check: parse_env_bool_like("QUE_DEC_OVERFLOW_CHECK", false),
        div_zero_check: parse_env_bool_like("QUE_DIV_ZERO_CHECK", false),
    }
}

fn devirtualize_mode_from_env() -> Result<DevirtualizeMode, String> {
    let raw = std::env::var("QUE_DEVIRTUALIZE").unwrap_or_else(|_| "aggressive".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" => Ok(DevirtualizeMode::Off),
        "known-heads" | "known_heads" | "known" => Ok(DevirtualizeMode::KnownHeads),
        "aggressive" => Ok(DevirtualizeMode::Aggressive),
        other => Err(format!(
            "invalid QUE_DEVIRTUALIZE='{}'. expected one of: off, known-heads, aggressive",
            other
        )),
    }
}

fn tail_call_mode_from_env() -> Result<TailCallMode, String> {
    let raw = std::env::var("QUE_TCO").unwrap_or_else(|_| "conservative".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" | "none" | "0" | "false" => Ok(TailCallMode::Off),
        "conservative" | "safe" | "default" => Ok(TailCallMode::Conservative),
        "aggressive" => Ok(TailCallMode::Aggressive),
        other => Err(format!(
            "invalid QUE_TCO='{}'. expected one of: off, conservative, aggressive",
            other
        )),
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
        "%" => Some(5),
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
        "pop-val!" => Some(38),
        "push!" => Some(39),
        "fst" => Some(23),
        "snd" => Some(24),
        "+." => Some(25),
        "-." => Some(26),
        "*." => Some(27),
        "/." => Some(28),
        "%." => Some(29),
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
        1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 25 | 26
        | 27 | 28 | 29 | 30 | 31 | 32 | 33 | 34 | 37 | 39 => Some(2),
        21 => Some(3),
        18 | 19 | 20 | 22 | 23 | 24 | 35 | 36 | 38 => Some(1),
        _ => None,
    }
}

fn builtin_tag_first_param_is_ref(tag: i32) -> bool {
    matches!(tag, 21 | 37 | 39)
}

fn is_i32ish_type(t: &Type) -> bool {
    matches!(
        t,
        Type::Int
            | Type::Dec
            | Type::Bool
            | Type::Char
            | Type::Unit
            | Type::List(_)
            | Type::Tuple(_)
            | Type::Var(_)
            | Type::Function(_, _)
    )
}

fn is_ref_type(t: &Type) -> bool {
    matches!(
        t,
        Type::List(_) | Type::Tuple(_) | Type::Function(_, _) | Type::Var(_)
    )
}

fn is_managed_local_type(t: &Type) -> bool {
    matches!(
        t,
        Type::List(_) | Type::Tuple(_) | Type::Function(_, _) | Type::Var(_)
    )
}

fn rc_retain_for_type(t: &Type) -> &'static str {
    match t {
        Type::List(_) | Type::Tuple(_) => "$rc_retain_vec",
        Type::Function(_, _) => "$closure_retain",
        Type::Var(_) => "$rc_retain",
        _ => "$rc_retain",
    }
}

fn rc_release_for_type(t: &Type) -> &'static str {
    match t {
        Type::List(_) | Type::Tuple(_) => "$rc_release_vec",
        Type::Function(_, _) => "$closure_release",
        Type::Var(_) => "$rc_release",
        _ => "$rc_release",
    }
}

fn rc_release_for_opt_type(t: Option<&Type>) -> &'static str {
    t.map(rc_release_for_type).unwrap_or("$rc_release")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RcKind {
    Vector,
    Dynamic,
}

impl RcKind {
    fn for_type(t: &Type) -> Self {
        match t {
            Type::List(_) | Type::Tuple(_) => Self::Vector,
            Type::Function(_, _) | Type::Var(_) => Self::Dynamic,
            _ => Self::Dynamic,
        }
    }

    fn release(self) -> &'static str {
        match self {
            Self::Vector => "$rc_release_vec",
            Self::Dynamic => "$rc_release",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ManagedRefSlot {
    slot: usize,
    kind: RcKind,
}

impl ManagedRefSlot {
    fn new(slot: usize, typ: &Type) -> Self {
        Self {
            slot,
            kind: RcKind::for_type(typ),
        }
    }
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

fn vec_push_runtime_for_elem_ref(elem_ref: i32) -> &'static str {
    if elem_ref == 0 {
        "$vec_push_scalar_i32"
    } else {
        "$vec_push_i32"
    }
}

fn vec_set_runtime_for_scalar(is_scalar: bool) -> &'static str {
    if is_scalar {
        "$vec_set_scalar_i32"
    } else {
        "$vec_set_i32"
    }
}

fn vec_set_runtime_for_materialized_scalar(is_scalar: bool) -> &'static str {
    if is_scalar {
        "$vec_set_scalar_materialized_i32"
    } else {
        "$vec_set_i32"
    }
}

fn is_scalar_vector_type(t: &Type) -> bool {
    matches!(t, Type::List(inner) if !is_ref_type(inner))
}

fn expr_is_definitely_materialized_scalar_vector(
    expr: &TypedExpression,
    materialized_scalar_local_slots: &HashSet<usize>,
    locals: &HashMap<String, usize>,
    definitely_materialized_top_level_scalar_names: &HashSet<String>,
) -> bool {
    if !expr
        .typ
        .as_ref()
        .map(is_scalar_vector_type)
        .unwrap_or(false)
    {
        return false;
    }
    match &expr.expr {
        Expression::Word(name) => locals
            .get(name)
            .map(|slot| materialized_scalar_local_slots.contains(slot))
            .unwrap_or_else(|| definitely_materialized_top_level_scalar_names.contains(name)),
        Expression::Apply(items) => matches!(
            items.first(),
                Some(Expression::Word(op))
                if matches!(
                    op.as_str(),
                    "vector" | "string" | "__vec_new_zeroed_i32" | "__vec_new_uninit_i32" | "integers" | "bools" | "decimals"
                )
        ),
        _ => false,
    }
}

impl VecElemKind {
    fn suffix(self) -> &'static str {
        match self {
            VecElemKind::I32 => "i32",
        }
    }
}

fn top_level_expr_is_definitely_materialized_scalar_vector(
    expr: &TypedExpression,
    top_defs: &HashMap<String, TopDef>,
    visiting: &mut HashSet<String>,
) -> bool {
    if !expr
        .typ
        .as_ref()
        .map(is_scalar_vector_type)
        .unwrap_or(false)
    {
        return false;
    }
    match &expr.expr {
        Expression::Word(name) => {
            if !visiting.insert(name.clone()) {
                return false;
            }
            let out = top_defs
                .get(name)
                .map(|def| {
                    top_level_expr_is_definitely_materialized_scalar_vector(
                        &def.node, top_defs, visiting,
                    )
                })
                .unwrap_or(false);
            visiting.remove(name);
            out
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "do") =>
        {
            let child_offset = if expr.children.len() + 1 == items.len() {
                1
            } else {
                0
            };
            let child_at = |item_idx: usize| -> Option<&TypedExpression> {
                if item_idx < child_offset {
                    None
                } else {
                    expr.children.get(item_idx - child_offset)
                }
            };
            let mut local_slots: HashMap<String, usize> = HashMap::new();
            let mut local_materialized: HashSet<usize> = HashSet::new();
            for i in 1..items.len().saturating_sub(1) {
                if let Expression::Apply(let_items) = &items[i] {
                    if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                        let slot = local_slots.len();
                        local_slots.entry(name.clone()).or_insert(slot);
                        if (kw == "let" || kw == "letrec" || kw == "mut")
                            && child_at(i)
                                .and_then(|n| n.children.get(2))
                                .map(|rhs| {
                                    expr_is_definitely_materialized_scalar_vector(
                                        rhs,
                                        &local_materialized,
                                        &local_slots,
                                        &HashSet::new(),
                                    ) || top_level_expr_is_definitely_materialized_scalar_vector(
                                        rhs, top_defs, visiting,
                                    )
                                })
                                .unwrap_or(false)
                        {
                            local_materialized.insert(*local_slots.get(name).unwrap());
                        }
                    }
                }
            }
            if let Some(last) = expr.children.last() {
                return expr_is_definitely_materialized_scalar_vector(
                    last,
                    &local_materialized,
                    &local_slots,
                    &HashSet::new(),
                ) || top_level_expr_is_definitely_materialized_scalar_vector(
                    last, top_defs, visiting,
                );
            }
            false
        }
        Expression::Apply(items) => {
            if matches!(
                items.first(),
                Some(Expression::Word(op))
                    if matches!(
                        op.as_str(),
                        "vector" | "string" | "__vec_new_zeroed_i32" | "__vec_new_uninit_i32" | "integers" | "bools" | "decimals"
                    )
            ) {
                return true;
            }
            if let Some(Expression::Word(op)) = items.first() {
                if let Some(def) = top_defs.get(op) {
                    if matches!(&def.expr, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                    {
                        if let Some(body) = def.node.children.last() {
                            return top_level_expr_is_definitely_materialized_scalar_vector(
                                body, top_defs, visiting,
                            );
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}

fn collect_definitely_materialized_top_level_scalar_names(
    top_defs: &HashMap<String, TopDef>,
) -> HashSet<String> {
    let mut out = HashSet::new();
    for (name, def) in top_defs {
        if top_level_expr_is_definitely_materialized_scalar_vector(
            &def.node,
            top_defs,
            &mut HashSet::new(),
        ) {
            out.insert(name.clone());
        }
    }
    out
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
    if s.chars()
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

fn compile_borrowed_top_level_cached_ref(
    name: &str,
    ctx: &Ctx<'_>,
    scratch_slot: usize,
) -> Option<String> {
    if ctx.locals.contains_key(name) || name == "ARGV" {
        return None;
    }
    if let Some(slot) = ctx.locals.get(&format!("__borrowed_top_level::{name}")) {
        return Some(format!("local.get {slot}"));
    }
    let (params, ret_ty) = ctx.fn_sigs.get(name)?;
    if !params.is_empty()
        || !is_managed_local_type(ret_ty)
        || matches!(ret_ty, Type::Function(_, _))
    {
        return None;
    }
    let g_init = cache_init_global(name);
    let g_val = cache_value_global(name);
    Some(
        format!(
            "global.get ${g_init}\nif (result i32)\n  global.get ${g_val}\nelse\n  call ${}\n  local.set {scratch_slot}\n  local.get {scratch_slot}\n  call {}\n  drop\n  global.get ${g_val}\nend",
            ident(name),
            rc_release_for_type(ret_ty)
        )
    )
}

fn collect_top_level_managed_refs(
    expr: &Expression,
    locals: &HashMap<String, usize>,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    cached_top_level_names: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    match expr {
        Expression::Word(name) => {
            if locals.contains_key(name) || name == "ARGV" {
                return;
            }
            if cached_top_level_names.contains(name)
                && fn_sigs.get(name).is_some_and(|(params, ret)| {
                    params.is_empty()
                        && is_managed_local_type(ret)
                        && !matches!(ret, Type::Function(_, _))
                })
            {
                out.insert(name.clone());
            }
        }
        Expression::Apply(items) => {
            for item in items {
                collect_top_level_managed_refs(item, locals, fn_sigs, cached_top_level_names, out);
            }
        }
        _ => {}
    }
}

fn top_level_borrow_plan(
    expr: &Expression,
    locals: &mut HashMap<String, usize>,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    cached_top_level_names: &HashSet<String>,
    first_slot: usize,
) -> (usize, String) {
    let mut names = HashSet::new();
    collect_top_level_managed_refs(expr, locals, fn_sigs, cached_top_level_names, &mut names);
    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort();
    let scratch_slot = first_slot + names.len();
    let mut prelude = Vec::new();
    for (offset, name) in names.iter().enumerate() {
        let slot = first_slot + offset;
        locals.insert(format!("__borrowed_top_level::{name}"), slot);
        let g_init = cache_init_global(name);
        let g_val = cache_value_global(name);
        let release = fn_sigs
            .get(name)
            .map(|(_, ret)| rc_release_for_type(ret))
            .unwrap_or("$rc_release");
        prelude.push(format!(
            "global.get ${g_init}\n\
             if (result i32)\n\
               global.get ${g_val}\n\
             else\n\
               call ${}\n\
               local.set {scratch_slot}\n\
               local.get {scratch_slot}\n\
               call {release}\n\
               drop\n\
               global.get ${g_val}\n\
             end\n\
             local.set {slot}",
            ident(name)
        ));
    }
    (names.len(), prelude.join("\n"))
}

fn wasm_val_type(typ: &Type) -> Result<&'static str, String> {
    match typ {
        Type::Int | Type::Dec | Type::Bool | Type::Char | Type::Unit => Ok("i32"),
        Type::List(_) | Type::Tuple(_) => Ok("i32"),
        Type::Var(_) => Ok("i32"),
        Type::Function(_, _) => Ok("i32"),
    }
}

fn wasm_param_types_for_signature(params: &[Type]) -> Result<Vec<&'static str>, String> {
    if params.len() == 1 && matches!(params[0], Type::Unit) {
        return Ok(Vec::new());
    }
    params.iter().map(wasm_val_type).collect()
}

fn vec_elem_kind_from_type(typ: &Type) -> Result<VecElemKind, String> {
    match typ {
        Type::Int
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
        "extern"
            | "do"
            | "let"
            | "mut"
            | "letrec"
            | "lambda"
            | "if"
            | "vector"
            | "string"
            | "integers"
            | "bools"
            | "decimals"
            | "strings"
            | "__vec_new_zeroed_i32"
            | "__vec_new_uninit_i32"
            | "tuple"
            | "length"
            | "get"
            | "car"
            | "cdr"
            | "fst"
            | "snd"
            | "set!"
            | "alter!"
            | "pop!"
            | "pop-val!"
            | "while"
            | "+"
            | "+#"
            | "+."
            | "-"
            | "-#"
            | "-."
            | "*"
            | "*#"
            | "*."
            | "/"
            | "/#"
            | "/."
            | "%"
            | "%."
            | "="
            | "=?"
            | "=#"
            | "=."
            | "<"
            | "<#"
            | "<."
            | ">"
            | ">#"
            | ">."
            | "<="
            | "<=#"
            | "<=."
            | ">="
            | ">=#"
            | ">=."
            | "and"
            | "or"
            | "not"
            | "^"
            | "|"
            | "&"
            | "<<"
            | ">>"
            | "~"
            | "Int->Dec"
            | "Dec->Int"
            | "cons"
            | "true"
            | "false"
            | "nil"
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
                            if let [Expression::Word(kw), Expression::Word(name), rhs] =
                                &let_items[..]
                            {
                                if kw == "let" || kw == "letrec" || kw == "mut" {
                                    if kw == "letrec" {
                                        let mut scoped = bound.clone();
                                        scoped.insert(name.clone());
                                        collect_refs(rhs, &mut scoped, out);
                                    } else {
                                        collect_refs(rhs, bound, out);
                                    }
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
                        if op == "letrec" {
                            let mut scoped = bound.clone();
                            scoped.insert(name.clone());
                            collect_refs(rhs, &mut scoped, out);
                        } else {
                            collect_refs(rhs, bound, out);
                        }
                        bound.insert(name.clone());
                        return;
                    }
                    if let Some(rhs) = items.get(2) {
                        collect_refs(rhs, bound, out);
                    }
                    return;
                }
                if op == "extern" {
                    if let Some(Expression::Word(name)) = items.get(3) {
                        bound.insert(name.clone());
                    }
                    return;
                }
                if op == "letype" {
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

fn top_level_binding_rhs_refs_main_only_names(
    kw: &str,
    name: &str,
    rhs: &Expression,
    main_only_names: &HashSet<String>,
) -> bool {
    let mut bound = HashSet::new();
    if kw == "letrec" {
        bound.insert(name.to_string());
    }
    let mut refs = HashSet::new();
    collect_refs(rhs, &mut bound, &mut refs);
    refs.iter().any(|r| main_only_names.contains(r))
}

fn collect_top_level_let_names(items: &[Expression]) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in items.iter().skip(1) {
        if let Expression::Apply(let_items) = item {
            if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                if kw == "let" {
                    names.insert(name.clone());
                }
            }
        }
    }
    names
}

fn collect_main_mutated_top_level_let_names(items: &[Expression]) -> HashSet<String> {
    let top_level_let_names = collect_top_level_let_names(items);
    let mut out = HashSet::new();
    for item in items.iter().skip(1) {
        if is_top_level_binding_form(item) || is_extern_decl_form(item) {
            continue;
        }
        collect_main_mutated_top_level_let_names_in_expr(item, &top_level_let_names, &mut out);
    }
    out
}

fn is_top_level_binding_form(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Apply(items)
            if matches!(
                &items[..],
                [Expression::Word(kw), Expression::Word(_), _] if kw == "let" || kw == "letrec" || kw == "mut"
            )
    )
}

fn is_extern_decl_form(expr: &Expression) -> bool {
    matches!(expr, Expression::Apply(_))
        && crate::externals::parse_extern_decl(expr)
            .ok()
            .flatten()
            .is_some()
}

fn collect_main_mutated_top_level_let_names_in_expr(
    expr: &Expression,
    top_level_let_names: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
        if top_level_let_names.contains(target)
            && (matches!(
                op.as_str(),
                "set!" | "push!" | "pop!" | "pop-val!" | "pull!" | "alter!" | "&alter!"
            ) || op.ends_with('!'))
        {
            out.insert(target.clone());
        }
    }
    for item in items {
        collect_main_mutated_top_level_let_names_in_expr(item, top_level_let_names, out);
    }
}

fn collect_builtin_host_extern_call_heads(expr: &Expression, out: &mut HashSet<String>) {
    match expr {
        Expression::Apply(items) => {
            if let Some(Expression::Word(op)) = items.first() {
                if crate::externals::is_builtin_host_extern_symbol(op) {
                    out.insert(op.clone());
                }
            }
            for item in items {
                collect_builtin_host_extern_call_heads(item, out);
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

fn collect_top_level_lambda_bindings(
    top_defs: &HashMap<String, TopDef>,
    out: &mut HashMap<String, TypedExpression>,
) {
    let mut aliases = Vec::new();
    for (name, def) in top_defs {
        match &def.node.expr {
            Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda") =>
            {
                out.insert(name.clone(), def.node.clone());
            }
            Expression::Word(alias) => {
                aliases.push((name.clone(), alias.clone()));
            }
            _ => {}
        }
    }

    // Resolve aliases after collecting all lambdas. `top_defs` is a HashMap, so
    // resolving aliases during the first pass made ownership analysis depend on
    // nondeterministic iteration order and missed chains such as
    // Integer->String -> std/convert/chars->integer -> lambda.
    while !aliases.is_empty() {
        let before = aliases.len();
        aliases.retain(|(name, alias)| {
            if let Some(target) = out.get(alias).cloned() {
                out.insert(name.clone(), target);
                false
            } else {
                true
            }
        });
        if aliases.len() == before {
            break;
        }
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
        let mut direct_host_calls = HashSet::new();
        collect_builtin_host_extern_call_heads(body, &mut direct_host_calls);
        refs.retain(|name| !direct_host_calls.contains(name));
    }
    refs.is_empty()
}

fn lambda_capture_names(
    node: &TypedExpression,
    _top_defs: &HashMap<String, TopDef>,
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
        let mut direct_host_calls = HashSet::new();
        collect_builtin_host_extern_call_heads(body, &mut direct_host_calls);
        refs.retain(|name| !direct_host_calls.contains(name));
    }
    let mut caps = refs.into_iter().collect::<Vec<_>>();
    caps.sort();
    caps
}

fn builtin_storage_summary(name: &str) -> Option<StorageSummary> {
    match name {
        "push!"
        | "std/vector/push!"
        | "std/vector/append!"
        | "std/vector/push-and-get!"
        | "Vector/push!"
        | "Vector/append!"
        | "Que/push!"
        | "Que/enque!"
        | "Que/append!"
        | "Que/prepend!"
        | "Set/add!"
        | "Heap/push!" => Some(StorageSummary {
            target_param: 0,
            value_param: 1,
        }),
        "set!" | "std/vector/set!" | "std/vector/update!" | "Vector/set!" => Some(StorageSummary {
            target_param: 0,
            value_param: 2,
        }),
        "Table/set!" | "Table/push-or!" => Some(StorageSummary {
            target_param: 0,
            value_param: 2,
        }),
        _ => None,
    }
}

fn storage_summary_for_name(name: &str, env: &RcCycleCheckEnv) -> Option<StorageSummary> {
    env.storage_summaries
        .get(name)
        .copied()
        .or_else(|| builtin_storage_summary(name))
}

fn expr_managed_root(node: &TypedExpression, env: &RcCycleCheckEnv) -> Option<String> {
    match &node.expr {
        Expression::Word(name) => env.managed_roots.get(name).cloned(),
        _ => None,
    }
}

fn closure_capture_roots(node: &TypedExpression, env: &RcCycleCheckEnv) -> Option<HashSet<String>> {
    match &node.expr {
        Expression::Word(name) => env.closure_captures.get(name).cloned(),
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") =>
        {
            let mut roots = HashSet::new();
            for cap in lambda_capture_names(node, &HashMap::new()) {
                if let Some(root) = env.managed_roots.get(&cap) {
                    roots.insert(root.clone());
                }
                if let Some(captured) = env.closure_captures.get(&cap) {
                    roots.extend(captured.iter().cloned());
                }
            }
            Some(roots)
        }
        _ => None,
    }
}

fn typed_binding_name(node: &TypedExpression) -> Option<String> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), _] = &items[..] else {
        return None;
    };
    if kw == "let" || kw == "letrec" || kw == "mut" {
        Some(name.clone())
    } else {
        None
    }
}

fn resolve_param_index(
    node: &TypedExpression,
    aliases: &HashMap<String, usize>,
    params: &HashMap<String, usize>,
) -> Option<usize> {
    match &node.expr {
        Expression::Word(name) => aliases
            .get(name)
            .copied()
            .or_else(|| params.get(name).copied()),
        _ => None,
    }
}

fn lambda_storage_summary(node: &TypedExpression, env: &RcCycleCheckEnv) -> Option<StorageSummary> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") || items.len() < 3 {
        return None;
    }
    let body = node.children.last()?;
    let mut params = HashMap::new();
    for (idx, param) in items[1..items.len() - 1].iter().enumerate() {
        let Expression::Word(name) = param else {
            return None;
        };
        params.insert(name.clone(), idx);
    }
    find_lambda_storage_summary(body, env, &params, &mut HashMap::new())
}

fn find_lambda_storage_summary(
    node: &TypedExpression,
    env: &RcCycleCheckEnv,
    params: &HashMap<String, usize>,
    aliases: &mut HashMap<String, usize>,
) -> Option<StorageSummary> {
    match &node.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") => {
            None
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "do") =>
        {
            let mut scoped_aliases = aliases.clone();
            for child in &node.children {
                if let Some(summary) =
                    find_lambda_storage_summary(child, env, params, &mut scoped_aliases)
                {
                    return Some(summary);
                }
                if let Some(name) = typed_binding_name(child) {
                    if let Some(rhs) = child.children.get(2) {
                        if let Some(param_idx) = resolve_param_index(rhs, &scoped_aliases, params) {
                            scoped_aliases.insert(name, param_idx);
                        }
                    }
                }
            }
            None
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "let" || w == "letrec" || w == "mut") => {
            node.children
                .get(2)
                .and_then(|rhs| find_lambda_storage_summary(rhs, env, params, aliases))
        }
        Expression::Apply(items) => {
            if let Some(Expression::Word(name)) = items.first() {
                if let Some(summary) = storage_summary_for_name(name, env) {
                    let target = node
                        .children
                        .get(summary.target_param + 1)
                        .and_then(|child| resolve_param_index(child, aliases, params));
                    let value = node
                        .children
                        .get(summary.value_param + 1)
                        .and_then(|child| resolve_param_index(child, aliases, params));
                    if let (Some(target_param), Some(value_param)) = (target, value) {
                        return Some(StorageSummary {
                            target_param,
                            value_param,
                        });
                    }
                }
            }
            for child in node.children.iter().skip(1) {
                if let Some(summary) = find_lambda_storage_summary(child, env, params, aliases) {
                    return Some(summary);
                }
            }
            None
        }
        _ => None,
    }
}

fn bind_rc_cycle_let(name: &str, rhs: &TypedExpression, env: &mut RcCycleCheckEnv) {
    if let Some(captures) = closure_capture_roots(rhs, env) {
        env.closure_captures.insert(name.to_string(), captures);
    } else {
        env.closure_captures.remove(name);
    }

    match &rhs.expr {
        Expression::Word(alias) => {
            if let Some(summary) = storage_summary_for_name(alias, env) {
                env.storage_summaries.insert(name.to_string(), summary);
            } else {
                env.storage_summaries.remove(name);
            }
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") => {
            if let Some(summary) = lambda_storage_summary(rhs, env) {
                env.storage_summaries.insert(name.to_string(), summary);
            } else {
                env.storage_summaries.remove(name);
            }
        }
        _ => {
            env.storage_summaries.remove(name);
        }
    }

    let Some(typ) = rhs.typ.as_ref() else {
        env.managed_roots.remove(name);
        return;
    };
    if !is_managed_local_type(typ) || matches!(typ, Type::Function(_, _)) {
        env.managed_roots.remove(name);
        return;
    }

    let root = expr_managed_root(rhs, env).unwrap_or_else(|| name.to_string());
    env.managed_roots.insert(name.to_string(), root);
}

fn rc_cycle_error(op: &str, target_root: &str, value: &TypedExpression) -> String {
    format!(
        "compile-time RC cycle check: '{}' would store a closure that captures '{}' back into the same managed value: {}",
        op,
        target_root,
        value.expr.to_lisp()
    )
}

fn validate_no_rc_cycles(node: &TypedExpression) -> Result<(), String> {
    validate_no_rc_cycles_with_env(node, &mut RcCycleCheckEnv::default())
}

fn validate_no_rc_cycles_with_env(
    node: &TypedExpression,
    env: &mut RcCycleCheckEnv,
) -> Result<(), String> {
    match &node.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") =>
        {
            if let Some(body) = node.children.last() {
                let mut scoped = env.clone();
                for param in &items[1..items.len().saturating_sub(1)] {
                    let mut bound = HashSet::new();
                    collect_pattern_words(param, &mut bound);
                    for name in bound {
                        scoped.managed_roots.remove(&name);
                        scoped.closure_captures.remove(&name);
                        scoped.storage_summaries.remove(&name);
                    }
                }
                validate_no_rc_cycles_with_env(body, &mut scoped)?;
            }
            Ok(())
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "do") =>
        {
            let mut scoped = env.clone();
            for child in &node.children {
                validate_no_rc_cycles_with_env(child, &mut scoped)?;
                if let Some(name) = typed_binding_name(child) {
                    if let Some(rhs) = child.children.get(2) {
                        bind_rc_cycle_let(&name, rhs, &mut scoped);
                    }
                }
            }
            Ok(())
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "let" || w == "letrec" || w == "mut") =>
        {
            for child in &node.children {
                validate_no_rc_cycles_with_env(child, env)?;
            }
            Ok(())
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "set!") =>
        {
            for child in &node.children {
                validate_no_rc_cycles_with_env(child, env)?;
            }
            let Some(target) = node.children.get(1) else {
                return Ok(());
            };
            let Some(value) = node.children.get(3) else {
                return Ok(());
            };
            if let (Some(target_root), Some(captures)) = (
                expr_managed_root(target, env),
                closure_capture_roots(value, env),
            ) {
                if captures.contains(&target_root) {
                    return Err(rc_cycle_error("set!", &target_root, value));
                }
            }
            Ok(())
        }
        Expression::Apply(items) => {
            for child in &node.children {
                validate_no_rc_cycles_with_env(child, env)?;
            }
            if let Some(Expression::Word(name)) = items.first() {
                if let Some(summary) = storage_summary_for_name(name, env) {
                    if let (Some(target), Some(value)) = (
                        node.children.get(summary.target_param + 1),
                        node.children.get(summary.value_param + 1),
                    ) {
                        if let (Some(target_root), Some(captures)) = (
                            expr_managed_root(target, env),
                            closure_capture_roots(value, env),
                        ) {
                            if captures.contains(&target_root) {
                                return Err(rc_cycle_error(name, &target_root, value));
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn lambda_syntax_arity(expr: &Expression) -> usize {
    match expr {
        Expression::Apply(items)
            if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
                && items.len() >= 2 =>
        {
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
        let digit_count = after.bytes().take_while(|b| b.is_ascii_digit()).count();
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

fn split_generic_runtime_and_apply_runtime(runtime_body: &str) -> (&str, &str) {
    if let Some(pos) = runtime_body.find("\n  (func $apply") {
        runtime_body.split_at(pos)
    } else {
        (runtime_body, "")
    }
}

fn extract_wat_func_signatures(module_body: &str) -> BTreeMap<String, String> {
    let mut signatures = BTreeMap::new();
    for line in module_body.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("(func $") else {
            continue;
        };
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == ')')
            .unwrap_or(rest.len());
        if name_end == 0 {
            continue;
        }
        let name = rest[..name_end].to_string();
        let sig = rest[name_end..].trim_end().to_string();
        signatures.insert(name, sig);
    }
    signatures
}

fn emit_runtime_func_exports(runtime_body: &str) -> String {
    let signatures = extract_wat_func_signatures(runtime_body);
    let mut out = String::new();
    for name in signatures.keys() {
        let export_plain = format!("(export \"{}\"", name);
        if runtime_body.contains(&export_plain) {
            continue;
        }
        out.push_str(&format!("  (export \"{}\" (func ${}))\n", name, name));
    }
    out
}

fn emit_runtime_imports(runtime_body: &str, user_body: &str) -> String {
    let signatures = extract_wat_func_signatures(runtime_body);
    let mut out = String::new();
    out.push_str("  (import \"que_runtime\" \"memory\" (memory 1))\n");
    out.push_str(
        "  (import \"que_runtime\" \"dbg_guard_trap_code\" (global $dbg_guard_trap_code (mut i32)))\n",
    );
    for (name, sig) in signatures {
        if !user_body.contains(&format!("call ${}", name)) {
            continue;
        }
        out.push_str(&format!(
            "  (import \"que_runtime\" \"{}\" (func ${}{}))\n",
            name, name, sig
        ));
    }
    out
}

fn emit_high_arity_apply_i32(
    arity: usize,
    fn_ids: &HashMap<String, i32>,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    closure_defs: &HashMap<String, ClosureDef>,
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
            if def.user_arity != arity
                || !is_i32ish_type(ret)
                || ps.len() != def.captures.len() + arity
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
        out.push_str("    local.get $f\n    call $is_closure_ptr\n    if (result i32)\n");
        for (fid, name, cap_len) in &closure_cases {
            out.push_str(
                &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
            );
            for i in 0..*cap_len {
                out.push_str(&format!(
                    "        local.get $f\n        i32.const {}\n        call $closure_get\n",
                    i
                ));
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
                out.push_str(&format!(
                    "    local.get $f\n    i32.const {}\n    i32.eq\n    if (result i32)\n",
                    tag
                ));
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
    apply_arities: &HashSet<usize>,
) -> String {
    fn parse_env_i32(name: &str, default: i32, min: i32, max: i32) -> i32 {
        std::env::var(name)
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
    (local $magic i32)
    (local $backing i32)
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
    i32.const 20
    i32.add
    i32.load
    local.set $magic
    local.get $magic
    i32.const 1447380018
    i32.eq
    if
      local.get $ptr
      i32.const 24
      i32.add
      i32.load
      local.set $backing
      local.get $backing
      call $rc_release
      drop
      local.get $ptr
      call $free
      drop
      i32.const 0
      return
    end
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
    i32.const 16
    local.get $n
    i32.const 8
    i32.mul
    i32.add
    call $alloc
    local.set $ptr
    local.get $ptr
    i32.const 1131176307
    i32.store
    local.get $ptr
    i32.const 4
    i32.add
    local.get $fn
    i32.store
    local.get $ptr
    i32.const 8
    i32.add
    local.get $n
    i32.store
    local.get $ptr
    i32.const 12
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
        i32.const 16
        i32.add
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.const 0
        i32.store
        local.get $ptr
        i32.const 16
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
  )

  (func $closure_set (param $ptr i32) (param $idx i32) (param $v i32) (result i32)
    (local $base i32)
    (local $n i32)
    local.get $ptr
    local.tee $base
    i32.const 8
    i32.add
    i32.load
    local.set $n
    local.get $base
    i32.const 16
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.const 0
    i32.store
    local.get $base
    i32.const 16
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
    local.tee $base
    i32.const 8
    i32.add
    i32.load
    local.set $n
    local.get $base
    i32.const 16
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
      i32.const 16
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
    i32.const 16
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.const 1
    i32.store
    local.get $base
    i32.const 16
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
    call $is_closure_ptr
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
    local.tee $base
    i32.const 8
    i32.add
    i32.load
    local.set $n
    local.get $base
    i32.const 16
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
    i32.const 4
    i32.add
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
    local.tee $base
    i32.const 12
    i32.add
    local.get $base
    i32.const 12
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
    local.set $base
    local.get $base
    i32.const 12
    i32.add
    i32.load
    local.set $rc
    local.get $rc
    i32.const 1
    i32.sub
    local.set $rc
    local.get $base
    i32.const 12
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
    i32.const 8
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
        i32.const 16
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
          i32.const 16
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

  (func $is_closure_ptr (param $ptr i32) (result i32)
    (local $mem_end i32)
    (local $n i32)
    (local $rc i32)
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
    local.get $mem_end
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $mem_end
    i32.const 16
    i32.sub
    i32.gt_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.load
    i32.const 1131176307
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 8
    i32.add
    i32.load
    local.set $n
    local.get $n
    i32.const 0
    i32.lt_s
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $rc
    local.get $rc
    i32.const 0
    i32.le_s
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 16
    i32.add
    local.get $n
    i32.const 8
    i32.mul
    i32.add
    local.get $mem_end
    i32.gt_u
    if
      i32.const 0
      return
    end
    i32.const 1
  )

  (func $is_vec_ptr (param $ptr i32) (result i32)
    (local $mem_end i32)
    (local $len i32)
    (local $cap i32)
    (local $rc i32)
    (local $elem_ref i32)
    (local $data i32)
    (local $magic i32)
    (local $backing i32)
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
    local.get $mem_end
    i32.ge_u
    if
      i32.const 0
      return
    end
    local.get $ptr
    local.get $mem_end
    i32.const 28
    i32.sub
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
    local.set $magic
    local.get $magic
    i32.const 1447380018
    i32.eq
    if
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
      i32.const 65536
      i32.lt_u
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
      local.get $ptr
      i32.const 24
      i32.add
      i32.load
      local.set $backing
      local.get $backing
      i32.const 0
      i32.eq
      if
        i32.const 0
        return
      end
      i32.const 1
      return
    end
    local.get $magic
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
    call $is_closure_ptr
    if
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
    call $is_closure_ptr
    if
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
    ;; Initialize only the live prefix [0, len). Appends write before exposing
    ;; new slots via len, so zeroing spare capacity is wasted work.
    i32.const 0
    local.set $i
    block $done
      loop $zero
        local.get $i
        local.get $len
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

  (func $vec_new_zeroed_i32 (param $len i32) (result i32)
    (local $cap i32)
    (local $ptr i32)
    (local $data i32)
    local.get $len
    i32.const 1
    i32.lt_s
    if (result i32)
      i32.const 1
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
    i32.const 0
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
    local.get $len
    i32.const 0
    i32.gt_s
    if
      local.get $data
      i32.const 0
      local.get $len
      i32.const 4
      i32.mul
      memory.fill
    end
    local.get $ptr
  )

  (func $vec_new_filled_i32 (param $len i32) (param $value i32) (result i32)
    (local $cap i32)
    (local $ptr i32)
    (local $data i32)
    (local $i i32)
    (local $remaining i32)
    (local $copy i32)
    local.get $len
    i32.const 1
    i32.lt_s
    if (result i32)
      i32.const 1
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
    i32.const 0
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
    local.get $len
    i32.const 0
    i32.gt_s
    if
      local.get $value
      i32.eqz
      if
        local.get $data
        i32.const 0
        local.get $len
        i32.const 4
        i32.mul
        memory.fill
      else
        local.get $data
        local.get $value
        i32.store
        i32.const 1
        local.set $i
        block $done
          loop $fill
            local.get $i
            local.get $len
            i32.ge_s
            br_if $done
            local.get $len
            local.get $i
            i32.sub
            local.set $remaining
            local.get $i
            local.get $remaining
            i32.lt_s
            if (result i32)
              local.get $i
            else
              local.get $remaining
            end
            local.set $copy
            local.get $data
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            local.get $data
            local.get $copy
            i32.const 4
            i32.mul
            memory.copy
            local.get $i
            local.get $copy
            i32.add
            local.set $i
            br $fill
          end
        end
      end
    end
    local.get $ptr
  )

  (func $vec_new_uninit_i32 (param $len i32) (result i32)
    (local $cap i32)
    (local $ptr i32)
    (local $data i32)
    local.get $len
    i32.const 1
    i32.lt_s
    if (result i32)
      i32.const 1
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
    i32.const 0
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

  (func $vec_materialize_i32 (param $ptr i32) (result i32)
    (local $magic i32)
    (local $len i32)
    (local $cap i32)
    (local $elem_ref i32)
    (local $old_data i32)
    (local $new_data i32)
    (local $backing i32)
    (local $i i32)
    (local $v i32)
    local.get $ptr
    i32.const 20
    i32.add
    i32.load
    local.set $magic
    local.get $magic
    i32.const 1447380018
    i32.ne
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.load
    local.set $len
    local.get $len
    i32.const __VEC_MIN_CAP__
    i32.lt_s
    if (result i32)
      i32.const __VEC_MIN_CAP__
    else
      local.get $len
    end
    local.set $cap
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $elem_ref
    local.get $ptr
    i32.const 16
    i32.add
    i32.load
    local.set $old_data
    local.get $cap
    i32.const 4
    i32.mul
    call $alloc
    local.set $new_data
    i32.const 0
    local.set $i
    block $zero_done
      loop $zero
        local.get $i
        local.get $cap
        i32.ge_s
        br_if $zero_done
        local.get $new_data
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
    i32.const 0
    local.set $i
    block $copy_done
      loop $copy
        local.get $i
        local.get $len
        i32.ge_s
        br_if $copy_done
        local.get $old_data
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $v
        local.get $elem_ref
        i32.const 0
        i32.ne
        if
          local.get $v
          call $rc_retain
          drop
        end
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
    local.get $ptr
    i32.const 24
    i32.add
    i32.load
    local.set $backing
    local.get $backing
    call $rc_release
    drop
    local.get $ptr
    i32.const 4
    i32.add
    local.get $cap
    i32.store
    local.get $ptr
    i32.const 16
    i32.add
    local.get $new_data
    i32.store
    local.get $ptr
    i32.const 20
    i32.add
    i32.const 1447380017
    i32.store
    local.get $ptr
    i32.const 24
    i32.add
    i32.const 0
    i32.store
    i32.const 0
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
    call $vec_materialize_i32
    drop
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
    call $vec_materialize_i32
    drop
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

  (func $vec_push_scalar_i32 (param $ptr i32) (param $v i32) (result i32)
    (local $len i32)
    (local $cap i32)
    (local $addr i32)
    local.get $ptr
    call $vec_materialize_i32
    drop
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    local.set $cap
    local.get $len
    local.get $cap
    i32.lt_s
    i32.eqz
    if
      local.get $ptr
      call $vec_grow_i32
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
    (local $r i64)
    local.get $a
    i64.extend_i32_s
    local.get $b
    i64.extend_i32_s
    i64.mul
    i64.const __DECIMAL_SCALE__
    i64.div_s
    local.set $r
    ;; __DEC_OVERFLOW_CHECK_R__
    local.get $r
    i32.wrap_i64
  )

  (func $dec_div (param $a i32) (param $b i32) (result i32)
    (local $r i64)
    local.get $a
    i64.extend_i32_s
    i64.const __DECIMAL_SCALE__
    i64.mul
    local.get $b
    i64.extend_i32_s
    i64.div_s
    local.set $r
    ;; __DEC_OVERFLOW_CHECK_R__
    local.get $r
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
    (local $r i64)
    local.get $a
    i64.extend_i32_s
    i64.const __DECIMAL_SCALE__
    i64.mul
    local.set $r
    ;; __DEC_OVERFLOW_CHECK_R__
    local.get $r
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
    call $vec_materialize_i32
    drop
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

  (func $vec_set_scalar_i32 (param $ptr i32) (param $idx i32) (param $v i32) (result i32)
    (local $len i32)
    (local $cap i32)
    (local $addr i32)
    local.get $ptr
    call $vec_materialize_i32
    drop
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    local.set $cap

    local.get $idx
    local.get $len
    i32.eq
    if
      local.get $len
      local.get $cap
      i32.lt_s
      i32.eqz
      if
        local.get $ptr
        call $vec_grow_i32
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
      local.get $ptr
      i32.const 16
      i32.add
      i32.load
      local.get $idx
      i32.const 4
      i32.mul
      i32.add
      local.set $addr
      local.get $addr
      local.get $v
      i32.store
      i32.const 0
      return
    end

    unreachable
  )

  (func $vec_set_scalar_materialized_i32 (param $ptr i32) (param $idx i32) (param $v i32) (result i32)
    (local $len i32)
    (local $cap i32)
    (local $addr i32)
    local.get $ptr
    i32.load
    local.set $len
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
    local.set $cap

    local.get $idx
    local.get $len
    i32.eq
    if
      local.get $len
      local.get $cap
      i32.lt_s
      i32.eqz
      if
        local.get $ptr
        call $vec_grow_i32
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
      local.get $ptr
      i32.const 16
      i32.add
      i32.load
      local.get $idx
      i32.const 4
      i32.mul
      i32.add
      local.set $addr
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
    call $vec_materialize_i32
    drop
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

  (func $vec_pop_val_i32 (param $ptr i32) (result i32)
    (local $len i32)
    (local $addr i32)
    (local $v i32)
    local.get $ptr
    call $vec_materialize_i32
    drop
    local.get $ptr
    i32.load
    local.set $len
    local.get $len
    i32.const 0
    i32.le_s
    if
      unreachable
    end
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
    local.get $ptr
    local.get $len
    i32.const 1
    i32.sub
    i32.store
    local.get $v
  )

  (func $vec_slice_i32 (param $ptr i32) (param $start i32) (result i32)
    (local $len i32)
    (local $new_len i32)
    (local $out i32)
    (local $elem_ref i32)
    (local $data i32)
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
        i32.const 28
        call $alloc
        local.set $out
        local.get $ptr
        i32.const 16
        i32.add
        i32.load
        local.set $data
        local.get $ptr
        call $rc_retain
        drop
        local.get $out
        local.get $new_len
        i32.store
        local.get $out
        i32.const 4
        i32.add
        local.get $new_len
        i32.store
        local.get $out
        i32.const 8
        i32.add
        i32.const 1
        i32.store
        local.get $out
        i32.const 12
        i32.add
        local.get $elem_ref
        i32.store
        local.get $out
        i32.const 16
        i32.add
        local.get $data
        local.get $start
        i32.const 4
        i32.mul
        i32.add
        i32.store
        local.get $out
        i32.const 20
        i32.add
        i32.const 1447380018
        i32.store
        local.get $out
        i32.const 24
        i32.add
        local.get $ptr
        i32.store
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
"#,
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
                if def.user_arity != 1 || !is_i32ish_type(ret) || ps.len() != def.captures.len() + 1
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
                let first_param_is_ref =
                    ps.get(def.captures.len()).map(is_ref_type).unwrap_or(false);
                Some((fid, helper_id, first_param_is_ref))
            })
            .collect::<Vec<_>>();
        if !apply1_closures.is_empty() || !apply1_partial_closures.is_empty() {
            out.push_str("    local.get $f\n    call $is_closure_ptr\n    if (result i32)\n");
            for (fid, name, cap_len) in &apply1_closures {
                out.push_str(
                    &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
                );
                for i in 0..*cap_len {
                    out.push_str(&format!(
                        "        local.get $f\n        i32.const {}\n        call $closure_get\n",
                        i
                    ));
                }
                out.push_str(&format!(
                    "        local.get $a\n        call ${}\n",
                    ident(name)
                ));
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
    i32.const 38
    i32.eq
    if (result i32)
      local.get $a
      call $vec_pop_val_i32
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
    end
    "#,
        );
        for _ in 0..apply1_open_ends {
            out.push_str("    end\n");
        }
        if !apply1_closures.is_empty() || !apply1_partial_closures.is_empty() {
            out.push_str("    end\n");
        }
        out.push_str("  )\n");
        out.push_str("  (export \"$apply1_i32\" (func $apply1_i32))\n");
        out.push_str("  (export \"apply1_i32\" (func $apply1_i32))\n");
    }
    if apply_arities.contains(&2) {
        out.push_str(
            "  (func $apply2_i32 (param $f i32) (param $a i32) (param $b i32) (result i32)\n",
        );
        let apply2_closures = closure_defs
            .values()
            .filter_map(|def| {
                let fid = *fn_ids.get(&def.name)?;
                let (ps, ret) = fn_sigs.get(&def.name)?;
                if def.user_arity != 2 || !is_i32ish_type(ret) || ps.len() != def.captures.len() + 2
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
            out.push_str("    local.get $f\n    call $is_closure_ptr\n    if (result i32)\n");
            for (fid, name, cap_len) in &apply2_closures {
                out.push_str(
                    &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
                );
                for i in 0..*cap_len {
                    out.push_str(&format!(
                        "        local.get $f\n        i32.const {}\n        call $closure_get\n",
                        i
                    ));
                }
                out.push_str(&format!(
                    "        local.get $a\n        local.get $b\n        call ${}\n",
                    ident(name)
                ));
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
                if ps.len() == 2
                    && is_i32ish_type(&ps[0])
                    && is_i32ish_type(&ps[1])
                    && is_i32ish_type(ret)
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
    i32.const 39
    i32.eq
    if (result i32)
      local.get $a
      local.get $a
      call $vec_len
      local.get $b
      call $vec_set_i32
    else
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
    "#,
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
    end
  "#,
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
                if def.user_arity != 3 || !is_i32ish_type(ret) || ps.len() != def.captures.len() + 3
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
            out.push_str("    local.get $f\n    call $is_closure_ptr\n    if (result i32)\n");
            for (fid, name, cap_len) in &apply3_closures {
                out.push_str(
                    &format!("      local.get $f\n      call $closure_fn\n      i32.const {}\n      i32.eq\n      if (result i32)\n", fid)
                );
                for i in 0..*cap_len {
                    out.push_str(&format!(
                        "        local.get $f\n        i32.const {}\n        call $closure_get\n",
                        i
                    ));
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
                if ps.len() == 3
                    && is_i32ish_type(&ps[0])
                    && is_i32ish_type(&ps[1])
                    && is_i32ish_type(&ps[2])
                    && is_i32ish_type(ret)
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
                if ps.len() == 3
                    && is_i32ish_type(&ps[0])
                    && is_i32ish_type(&ps[1])
                    && is_i32ish_type(&ps[2])
                    && is_i32ish_type(ret)
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
        out.push_str(&emit_high_arity_apply_i32(
            arity,
            fn_ids,
            fn_sigs,
            closure_defs,
        ));
    }
    let debug_rc_enabled = cfg!(feature = "debug-rc");
    out = out.replace("__VEC_MIN_CAP__", &vec_min_cap.to_string());
    out = out.replace("__VEC_GROWTH_NUM__", &vec_growth_num.to_string());
    out = out.replace("__VEC_GROWTH_DEN__", &vec_growth_den.to_string());
    out = out.replace("__DECIMAL_SCALE__", &decimal_scale_i64().to_string());
    let dec_overflow_check = if arithmetic_check_config().dec_overflow_check {
        format!(
            r#"local.get $r
    i64.const {}
    i64.gt_s
    if
      {}
    end
    local.get $r
    i64.const {}
    i64.lt_s
    if
      {}
    end"#,
            i32::MAX,
            indent_block(&emit_guard_trap_wat(DBG_GUARD_TRAP_DEC_OVERFLOW), 3),
            i32::MIN,
            indent_block(&emit_guard_trap_wat(DBG_GUARD_TRAP_DEC_OVERFLOW), 3)
        )
    } else {
        String::new()
    };
    out = out.replace(";; __DEC_OVERFLOW_CHECK_R__", &dec_overflow_check);
    out = out.replace(
        ";; __VEC_GET_BOUNDS_CHECK__",
        if vec_bounds_check_enabled {
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
        },
    );

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
      call $is_closure_ptr
      if (result i32)
        i32.const 1
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
        (
            ";; __DBG_RC_RELEASE_VEC_DEC__",
            if debug_rc_enabled { "" } else { "" },
        ),
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

fn emit_wasi_print_runtime() -> &'static str {
    r#"
  ;; Que [Char] -> UTF-8 stdout. Scratch bytes live below the heap base.
  (func $__wasi_write_text (param $fd i32) (param $text i32) (result i32)
    (local $len i32) (local $data i32) (local $i i32)
    (local $c i32) (local $n i32)
    local.get $text
    i32.load
    local.set $len
    local.get $text
    i32.const 16
    i32.add
    i32.load
    local.set $data
    block $done
      loop $chars
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $data
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $c
        local.get $c
        i32.const 128
        i32.lt_u
        if
          i32.const 32
          local.get $c
          i32.store8
          i32.const 1
          local.set $n
        else
          local.get $c
          i32.const 2048
          i32.lt_u
          if
            i32.const 32
            local.get $c
            i32.const 6
            i32.shr_u
            i32.const 192
            i32.or
            i32.store8
            i32.const 33
            local.get $c
            i32.const 63
            i32.and
            i32.const 128
            i32.or
            i32.store8
            i32.const 2
            local.set $n
          else
            local.get $c
            i32.const 65536
            i32.lt_u
            if
              i32.const 32
              local.get $c
              i32.const 12
              i32.shr_u
              i32.const 224
              i32.or
              i32.store8
              i32.const 33
              local.get $c
              i32.const 6
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              i32.const 34
              local.get $c
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              i32.const 3
              local.set $n
            else
              i32.const 32
              local.get $c
              i32.const 18
              i32.shr_u
              i32.const 240
              i32.or
              i32.store8
              i32.const 33
              local.get $c
              i32.const 12
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              i32.const 34
              local.get $c
              i32.const 6
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              i32.const 35
              local.get $c
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              i32.const 4
              local.set $n
            end
          end
        end
        i32.const 0
        i32.const 32
        i32.store
        i32.const 4
        local.get $n
        i32.store
        local.get $fd
        i32.const 0
        i32.const 1
        i32.const 16
        call $__wasi_fd_write
        drop
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $chars
      end
    end
    i32.const 0)
  (func $v_print_bang_ (param $text i32) (result i32)
    i32.const 1
    local.get $text
    call $__wasi_write_text)
"#
}

fn emit_wasi_clock_runtime() -> &'static str {
    r#"
  (func $v_time_bang_ (result i32)
    i32.const 0
    i64.const 1000000
    i32.const 40
    call $__wasi_clock_time_get
    if
      unreachable
    end
    i32.const 40
    i64.load
    i64.const 1000000000
    i64.div_u
    i32.wrap_i64)
"#
}

fn emit_wasi_random_runtime() -> &'static str {
    r#"
  (func $v_random_bang_ (result i32)
    i32.const 48
    i32.const 4
    call $__wasi_random_get
    if
      unreachable
    end
    i32.const 48
    i32.load)
"#
}

fn emit_wasi_sleep_runtime() -> &'static str {
    r#"
  (func $v_sleep_bang_ (param $millis i32) (result i32)
    local.get $millis
    i32.const 0
    i32.lt_s
    if
      unreachable
    end
    ;; A relative monotonic-clock subscription. Timeout and precision are ns.
    i32.const 64
    i64.const 0
    i64.store
    i32.const 72
    i32.const 0
    i32.store8
    i32.const 80
    i32.const 1
    i32.store
    i32.const 88
    local.get $millis
    i64.extend_i32_u
    i64.const 1000000
    i64.mul
    i64.store
    i32.const 96
    i64.const 1000000
    i64.store
    i32.const 104
    i32.const 0
    i32.store16
    i32.const 64
    i32.const 112
    i32.const 1
    i32.const 144
    call $__wasi_poll_oneoff
    if
      unreachable
    end
    i32.const 0)
"#
}

fn emit_wasi_clear_runtime() -> &'static str {
    r#"
  ;; ANSI clear-screen sequence written to stdout.
  (func $v_clear_bang_ (result i32)
    i32.const 32
    i64.const 20366371090225947
    i64.store
    i32.const 0
    i32.const 32
    i32.store
    i32.const 4
    i32.const 7
    i32.store
    i32.const 1
    i32.const 0
    i32.const 1
    i32.const 16
    call $__wasi_fd_write
    drop
    i32.const 0)
"#
}

fn emit_wasi_stdin_runtime() -> &'static str {
    r#"
  ;; Read stdin and decode UTF-8 into Que's [Char] representation.
  (func $__wasi_read_fd (param $fd i32) (result i32)
    (local $out i32) (local $n i32) (local $i i32)
    (local $b i32) (local $c i32) (local $needed i32)
    i32.const 0
    i32.const 0
    call $vec_new_i32
    local.set $out
    block $done
      loop $read
        i32.const 0
        i32.const 1024
        i32.store
        i32.const 4
        i32.const 60000
        i32.store
        local.get $fd
        i32.const 0
        i32.const 1
        i32.const 16
        call $__wasi_fd_read
        if unreachable end
        i32.const 16
        i32.load
        local.tee $n
        i32.eqz
        br_if $done
        i32.const 0
        local.set $i
        block $chunk_done
          loop $decode
            local.get $i
            local.get $n
            i32.ge_u
            br_if $chunk_done
            i32.const 1024
            local.get $i
            i32.add
            i32.load8_u
            local.set $b
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            local.get $needed
            i32.eqz
            if
              local.get $b
              i32.const 128
              i32.lt_u
              if
                local.get $b
                local.set $c
              else
                local.get $b
                i32.const 224
                i32.and
                i32.const 192
                i32.eq
                if
                  local.get $b
                  i32.const 31
                  i32.and
                  local.set $c
                  i32.const 1
                  local.set $needed
                else
                  local.get $b
                  i32.const 240
                  i32.and
                  i32.const 224
                  i32.eq
                  if
                    local.get $b
                    i32.const 15
                    i32.and
                    local.set $c
                    i32.const 2
                    local.set $needed
                  else
                    local.get $b
                    i32.const 7
                    i32.and
                    local.set $c
                    i32.const 3
                    local.set $needed
                  end
                end
              end
            else
              local.get $c
              i32.const 6
              i32.shl
              local.get $b
              i32.const 63
              i32.and
              i32.or
              local.set $c
              local.get $needed
              i32.const 1
              i32.sub
              local.set $needed
            end
            local.get $needed
            i32.eqz
            if
              local.get $out
              local.get $c
              call $vec_push_i32
              drop
            end
            br $decode
          end
        end
        br $read
      end
    end
    local.get $out)
  (func $v_stdin_bang_ (result i32)
    i32.const 0
    call $__wasi_read_fd)
"#
}

fn emit_wasi_argv_runtime() -> &'static str {
    r#"
  (func $__wasi_init_argv (result i32)
    (local $argc i32) (local $size i32) (local $table i32) (local $bytes i32)
    (local $outer i32) (local $inner i32) (local $arg i32) (local $p i32)
    (local $b i32) (local $c i32) (local $needed i32)
    i32.const 0
    i32.const 4
    call $__wasi_args_sizes_get
    if unreachable end
    i32.const 0
    i32.load
    local.set $argc
    i32.const 4
    i32.load
    local.set $size
    local.get $argc
    i32.const 4
    i32.mul
    call $alloc
    local.set $table
    local.get $size
    call $alloc
    local.set $bytes
    local.get $table
    local.get $bytes
    call $__wasi_args_get
    if unreachable end
    i32.const 0
    i32.const 1
    call $vec_new_i32
    local.set $outer
    i32.const 1
    local.set $arg
    block $done
      loop $args
        local.get $arg
        local.get $argc
        i32.ge_u
        br_if $done
        i32.const 0
        i32.const 0
        call $vec_new_i32
        local.set $inner
        local.get $table
        local.get $arg
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $p
        i32.const 0
        local.set $needed
        block $string_done
          loop $chars
            local.get $p
            i32.load8_u
            local.tee $b
            i32.eqz
            br_if $string_done
            local.get $p
            i32.const 1
            i32.add
            local.set $p
            local.get $needed
            i32.eqz
            if
              local.get $b
              i32.const 128
              i32.lt_u
              if
                local.get $b
                local.set $c
              else
                local.get $b
                i32.const 224
                i32.and
                i32.const 192
                i32.eq
                if
                  local.get $b
                  i32.const 31
                  i32.and
                  local.set $c
                  i32.const 1
                  local.set $needed
                else
                  local.get $b
                  i32.const 240
                  i32.and
                  i32.const 224
                  i32.eq
                  if
                    local.get $b
                    i32.const 15
                    i32.and
                    local.set $c
                    i32.const 2
                    local.set $needed
                  else
                    local.get $b
                    i32.const 7
                    i32.and
                    local.set $c
                    i32.const 3
                    local.set $needed
                  end
                end
              end
            else
              local.get $c
              i32.const 6
              i32.shl
              local.get $b
              i32.const 63
              i32.and
              i32.or
              local.set $c
              local.get $needed
              i32.const 1
              i32.sub
              local.set $needed
            end
            local.get $needed
            i32.eqz
            if
              local.get $inner
              local.get $c
              call $vec_push_i32
              drop
            end
            br $chars
          end
        end
        local.get $outer
        local.get $inner
        call $vec_push_i32
        drop
        local.get $inner
        call $rc_release_vec
        drop
        local.get $arg
        i32.const 1
        i32.add
        local.set $arg
        br $args
      end
    end
    local.get $outer
    global.set $argv_ptr
    i32.const 0)
"#
}

fn emit_wasi_file_runtime(has_read: bool, has_write: bool) -> String {
    let mut out = String::from(
        r#"
  ;; Encode a Que [Char] path as UTF-8 at scratch address 1024.
  (func $__wasi_encode_path (param $path i32) (result i32)
    (local $len i32) (local $data i32) (local $i i32)
    (local $p i32) (local $c i32)
    local.get $path
    i32.load
    local.set $len
    local.get $path
    i32.const 16
    i32.add
    i32.load
    local.set $data
    i32.const 1024
    local.set $p
    block $done
      loop $chars
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $data
        local.get $i
        i32.const 4
        i32.mul
        i32.add
        i32.load
        local.set $c
        local.get $c
        i32.const 128
        i32.lt_u
        if
          local.get $p
          local.get $c
          i32.store8
          local.get $p
          i32.const 1
          i32.add
          local.set $p
        else
          local.get $c
          i32.const 2048
          i32.lt_u
          if
            local.get $p
            local.get $c
            i32.const 6
            i32.shr_u
            i32.const 192
            i32.or
            i32.store8
            local.get $p
            i32.const 1
            i32.add
            local.get $c
            i32.const 63
            i32.and
            i32.const 128
            i32.or
            i32.store8
            local.get $p
            i32.const 2
            i32.add
            local.set $p
          else
            local.get $c
            i32.const 65536
            i32.lt_u
            if
              local.get $p
              local.get $c
              i32.const 12
              i32.shr_u
              i32.const 224
              i32.or
              i32.store8
              local.get $p
              i32.const 1
              i32.add
              local.get $c
              i32.const 6
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              local.get $p
              i32.const 2
              i32.add
              local.get $c
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              local.get $p
              i32.const 3
              i32.add
              local.set $p
            else
              local.get $p
              local.get $c
              i32.const 18
              i32.shr_u
              i32.const 240
              i32.or
              i32.store8
              local.get $p
              i32.const 1
              i32.add
              local.get $c
              i32.const 12
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              local.get $p
              i32.const 2
              i32.add
              local.get $c
              i32.const 6
              i32.shr_u
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              local.get $p
              i32.const 3
              i32.add
              local.get $c
              i32.const 63
              i32.and
              i32.const 128
              i32.or
              i32.store8
              local.get $p
              i32.const 4
              i32.add
              local.set $p
            end
          end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $chars
      end
    end
    local.get $p
    i32.const 1024
    i32.sub)
"#,
    );
    if has_read {
        out.push_str(
            r#"
  (func $v_read_bang_ (param $path i32) (result i32)
    (local $len i32) (local $fd i32) (local $result i32)
    local.get $path
    call $__wasi_encode_path
    local.set $len
    i32.const 3
    i32.const 0
    i32.const 1024
    local.get $len
    i32.const 0
    i64.const 2
    i64.const 0
    i32.const 0
    i32.const 16
    call $__wasi_path_open
    if unreachable end
    i32.const 16
    i32.load
    local.tee $fd
    call $__wasi_read_fd
    local.set $result
    local.get $fd
    call $__wasi_fd_close
    drop
    local.get $result)
"#,
        );
    }
    if has_write {
        out.push_str(
            r#"
  (func $v_write_bang_ (param $path i32) (param $text i32) (result i32)
    (local $len i32) (local $fd i32)
    local.get $path
    call $__wasi_encode_path
    local.set $len
    i32.const 3
    i32.const 0
    i32.const 1024
    local.get $len
    i32.const 9
    i64.const 64
    i64.const 0
    i32.const 0
    i32.const 16
    call $__wasi_path_open
    if unreachable end
    i32.const 16
    i32.load
    local.tee $fd
    local.get $text
    call $__wasi_write_text
    drop
    local.get $fd
    call $__wasi_fd_close
    drop
    i32.const 0)
"#,
        );
    }
    out
}

fn emit_wasi_path_mutation_runtime(has_mkdir: bool, has_delete: bool, has_move: bool) -> String {
    let mut out = String::new();
    if has_mkdir {
        out.push_str(
            r#"
  (func $v_mkdir_bang_ (param $path i32) (result i32)
    (local $len i32) (local $i i32) (local $result i32)
    local.get $path
    call $__wasi_encode_path
    local.set $len
    i32.const 1
    local.set $i
    block $parents_done
      loop $parents
        local.get $i
        local.get $len
        i32.ge_u
        br_if $parents_done
        i32.const 1024
        local.get $i
        i32.add
        i32.load8_u
        i32.const 47
        i32.eq
        if
          i32.const 3
          i32.const 1024
          local.get $i
          call $__wasi_path_create_directory
          local.tee $result
          i32.const 0
          i32.ne
          local.get $result
          i32.const 20
          i32.ne
          i32.and
          if unreachable end
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $parents
      end
    end
    i32.const 3
    i32.const 1024
    local.get $len
    call $__wasi_path_create_directory
    local.tee $result
    i32.const 20
    i32.ne
    local.get $result
    i32.const 0
    i32.ne
    i32.and
    if unreachable end
    i32.const 0)
"#,
        );
    }
    if has_delete {
        out.push_str(
            r#"
  (func $v_delete_bang_ (param $path i32) (result i32)
    (local $len i32) (local $entries i32) (local $count i32) (local $edata i32)
    (local $i i32) (local $name i32) (local $child i32)
    (local $plen i32) (local $pdata i32) (local $nlen i32) (local $ndata i32) (local $j i32)
    local.get $path
    call $__wasi_encode_path
    local.set $len
    i32.const 3
    i32.const 1024
    local.get $len
    call $__wasi_path_unlink_file
    i32.eqz
    if i32.const 0 return end
    local.get $path call $v_list_dash_dir_bang_ local.tee $entries
    i32.load local.set $count
    local.get $entries i32.const 16 i32.add i32.load local.set $edata
    local.get $path i32.load local.set $plen
    local.get $path i32.const 16 i32.add i32.load local.set $pdata
    block $children_done loop $children
      local.get $i local.get $count i32.ge_u br_if $children_done
      local.get $edata local.get $i i32.const 4 i32.mul i32.add i32.load local.tee $name
      i32.load local.set $nlen
      local.get $name i32.const 16 i32.add i32.load local.set $ndata
      i32.const 0 i32.const 0 call $vec_new_i32 local.set $child
      i32.const 0 local.set $j
      block $path_done loop $copy_path
        local.get $j local.get $plen i32.ge_u br_if $path_done
        local.get $child local.get $pdata local.get $j i32.const 4 i32.mul i32.add i32.load
        call $vec_push_i32 drop
        local.get $j i32.const 1 i32.add local.set $j br $copy_path
      end end
      local.get $plen i32.eqz
      if local.get $child i32.const 47 call $vec_push_i32 drop
      else
        local.get $pdata local.get $plen i32.const 1 i32.sub i32.const 4 i32.mul i32.add i32.load
        i32.const 47 i32.ne
        if local.get $child i32.const 47 call $vec_push_i32 drop end
      end
      i32.const 0 local.set $j
      block $name_done loop $copy_name
        local.get $j local.get $nlen i32.ge_u br_if $name_done
        local.get $child local.get $ndata local.get $j i32.const 4 i32.mul i32.add i32.load
        call $vec_push_i32 drop
        local.get $j i32.const 1 i32.add local.set $j br $copy_name
      end end
      local.get $child call $v_delete_bang_ drop
      local.get $child call $rc_release_vec drop
      local.get $i i32.const 1 i32.add local.set $i br $children
    end end
    local.get $entries call $rc_release_vec drop
    local.get $path call $__wasi_encode_path local.set $len
    i32.const 3 i32.const 1024 local.get $len call $__wasi_path_remove_directory
    if unreachable end
    i32.const 0)
"#,
        );
    }
    if has_move {
        out.push_str(
            r#"
  (func $v_move_bang_ (param $src i32) (param $dst i32) (result i32)
    (local $src_len i32) (local $dst_len i32)
    local.get $src
    call $__wasi_encode_path
    local.set $src_len
    i32.const 32768
    i32.const 1024
    local.get $src_len
    memory.copy
    local.get $dst
    call $__wasi_encode_path
    local.set $dst_len
    i32.const 3
    i32.const 32768
    local.get $src_len
    i32.const 3
    i32.const 1024
    local.get $dst_len
    call $__wasi_path_rename
    if unreachable end
    i32.const 0)
"#,
        );
    }
    out
}

fn emit_wasi_chunk_runtime(
    has_file_chunks: bool,
    has_stdin_chunks: bool,
    has_lines: bool,
) -> String {
    let mut out = String::from(
        r#"
  (func $__wasi_chunks (param $text i32) (param $size i32) (param $callback i32) (result i32)
    (local $len i32) (local $data i32) (local $i i32) (local $j i32)
    (local $chunk i32) (local $stop i32)
    local.get $size
    i32.const 0
    i32.le_s
    if unreachable end
    local.get $text
    i32.load
    local.set $len
    local.get $text
    i32.const 16
    i32.add
    i32.load
    local.set $data
    block $done
      loop $outer
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        i32.const 0
        i32.const 0
        call $vec_new_i32
        local.set $chunk
        i32.const 0
        local.set $j
        block $chunk_done
          loop $copy
            local.get $j
            local.get $size
            i32.ge_u
            br_if $chunk_done
            local.get $i
            local.get $len
            i32.ge_u
            br_if $chunk_done
            local.get $chunk
            local.get $data
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.load
            call $vec_push_i32
            drop
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            local.get $j
            i32.const 1
            i32.add
            local.set $j
            br $copy
          end
        end
        local.get $callback
        local.get $chunk
        call $apply1_i32
        local.set $stop
        local.get $chunk
        call $rc_release_vec
        drop
        local.get $stop
        if
          i32.const 1
          return
        end
        br $outer
      end
    end
    i32.const 0)
"#,
    );
    if has_file_chunks {
        out.push_str(
            r#"
  (func $v_read_slash_chunks_bang_ (param $path i32) (param $size i32) (param $callback i32) (result i32)
    (local $text i32) (local $result i32)
    local.get $path
    call $v_read_bang_
    local.set $text
    local.get $text
    local.get $size
    local.get $callback
    call $__wasi_chunks
    local.set $result
    local.get $text
    call $rc_release_vec
    drop
    local.get $result)
"#,
        );
    }
    if has_stdin_chunks {
        out.push_str(
            r#"
  (func $v_stdin_slash_chunks_bang_ (param $size i32) (param $callback i32) (result i32)
    (local $text i32) (local $result i32)
    call $v_stdin_bang_
    local.set $text
    local.get $text
    local.get $size
    local.get $callback
    call $__wasi_chunks
    local.set $result
    local.get $text
    call $rc_release_vec
    drop
    local.get $result)
"#,
        );
    }
    if has_lines {
        out.push_str(
            r#"
  (func $v_read_slash_lines_bang_ (param $path i32) (param $callback i32) (result i32)
    (local $text i32) (local $len i32) (local $data i32) (local $i i32)
    (local $line i32) (local $c i32) (local $stop i32)
    local.get $path
    call $v_read_bang_
    local.tee $text
    i32.load
    local.set $len
    local.get $text
    i32.const 16
    i32.add
    i32.load
    local.set $data
    block $done
      loop $lines
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        i32.const 0
        i32.const 0
        call $vec_new_i32
        local.set $line
        block $line_done
          loop $chars
            local.get $i
            local.get $len
            i32.ge_u
            br_if $line_done
            local.get $data
            local.get $i
            i32.const 4
            i32.mul
            i32.add
            i32.load
            local.set $c
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            local.get $c
            i32.const 10
            i32.eq
            br_if $line_done
            local.get $line
            local.get $c
            call $vec_push_i32
            drop
            br $chars
          end
        end
        local.get $line
        i32.load
        i32.const 0
        i32.gt_u
        if
          local.get $line
          i32.const 16
          i32.add
          i32.load
          local.get $line
          i32.load
          i32.const 1
          i32.sub
          i32.const 4
          i32.mul
          i32.add
          i32.load
          i32.const 13
          i32.eq
          if
            local.get $line
            call $vec_pop_i32
            drop
          end
        end
        local.get $callback
        local.get $line
        call $apply1_i32
        local.set $stop
        local.get $line
        call $rc_release_vec
        drop
        local.get $stop
        if
          local.get $text
          call $rc_release_vec
          drop
          i32.const 1
          return
        end
        br $lines
      end
    end
    local.get $text
    call $rc_release_vec
    drop
    i32.const 0)
"#,
        );
    }
    out
}

fn emit_wasi_list_dir_runtime() -> &'static str {
    include_str!("wasi_list_dir.wat")
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
            emit_guard_trap_wat(DBG_GUARD_TRAP_DEC_DIV_ZERO)
        )
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
    let integer_arithmetic_is_proven_safe = static_proof_is_safe(
        crate::static_analysis::ProofKind::IntegerArithmetic,
        &node.expr,
    );
    let divisor_is_proven_nonzero = static_proof_is_safe(
        crate::static_analysis::ProofKind::NonZeroDivisor,
        &node.expr,
    );
    let lhs_local = ctx.tmp_i32;
    let rhs_local = ctx.tmp_i32 + 1;
    let res_local = ctx.tmp_i32 + 2;
    let a = node
        .children
        .get(1)
        .ok_or_else(|| format!("Missing lhs for {}", op))
        .and_then(|n| compile_expr(n, ctx))?;
    let b = node
        .children
        .get(2)
        .ok_or_else(|| format!("Missing rhs for {}", op))
        .and_then(|n| compile_expr(n, ctx))?;
    let code = match op {
        "+" | "+#" => {
            if checks.int_overflow_check && !integer_arithmetic_is_proven_safe {
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
            if checks.int_overflow_check && !integer_arithmetic_is_proven_safe {
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
            if checks.int_overflow_check && !integer_arithmetic_is_proven_safe {
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
            if checks.div_zero_check && !divisor_is_proven_nonzero {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\n{}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.div_s",
                        emit_int_div_zero_check(rhs_local)
                    )
                );
            }
            "i32.div_s"
        }
        "%" => {
            if checks.div_zero_check && !divisor_is_proven_nonzero {
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
            return Ok(format!(
                "{a}\n(if (result i32)\n  (then\n    {b}\n  )\n  (else\n    i32.const 0\n  )\n)"
            ));
        }
        "or" => {
            return Ok(format!(
                "{a}\n(if (result i32)\n  (then\n    i32.const 1\n  )\n  (else\n    {b}\n  )\n)"
            ));
        }
        "^" => "i32.xor",
        "|" => "i32.or",
        "&" => "i32.and",
        "<<" => "i32.shl",
        ">>" => "i32.shr_s",
        "+." => {
            if checks.dec_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.add\nlocal.set {res_local}\n{}\nlocal.get {res_local}",
                        emit_int_add_overflow_check(lhs_local, rhs_local, res_local)
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ni32.add"));
        }
        "-." => {
            if checks.dec_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ni32.sub\nlocal.set {res_local}\n{}\nlocal.get {res_local}",
                        emit_int_sub_overflow_check(lhs_local, rhs_local, res_local)
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ni32.sub"));
        }
        "*." => {
            if checks.dec_overflow_check {
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\nlocal.get {lhs_local}\nlocal.get {rhs_local}\ncall $dec_mul"
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ncall $dec_mul"));
        }
        "/." => {
            if checks.div_zero_check || checks.dec_overflow_check {
                let div_zero_check = if checks.div_zero_check {
                    format!("{}\n", emit_float_div_zero_check(rhs_local))
                } else {
                    String::new()
                };
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\n{div_zero_check}local.get {lhs_local}\nlocal.get {rhs_local}\ncall $dec_div"
                    )
                );
            }
            return Ok(format!("{a}\n{b}\ncall $dec_div"));
        }
        "%." => {
            if checks.div_zero_check || checks.dec_overflow_check {
                let div_zero_check = if checks.div_zero_check {
                    format!("{}\n", emit_float_div_zero_check(rhs_local))
                } else {
                    String::new()
                };
                return Ok(
                    format!(
                        "{a}\n{b}\nlocal.set {rhs_local}\nlocal.set {lhs_local}\n{div_zero_check}local.get {lhs_local}\nlocal.get {rhs_local}\ncall $dec_mod"
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
            return Ok(format!(
                "{a}\n{b}\ni32.const {elem_ref}\ncall $vec_concat_i32"
            ));
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
        node.children
            .get(1)
            .ok_or_else(|| "if missing condition".to_string())?,
        ctx,
    )?;
    let t = compile_expr(
        node.children
            .get(2)
            .ok_or_else(|| "if missing then".to_string())?,
        ctx,
    )?;
    let e = compile_expr(
        node.children
            .get(3)
            .ok_or_else(|| "if missing else".to_string())?,
        ctx,
    )?;
    let result_ty = node
        .typ
        .as_ref()
        .ok_or_else(|| "if missing type".to_string())
        .and_then(wasm_val_type)?;
    Ok(format!(
        "{cond}\n(if (result {result_ty})\n  (then\n    {t}\n  )\n  (else\n    {e}\n  )\n)"
    ))
}

fn compile_if_discarding_result(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let cond = compile_expr(
        node.children
            .get(1)
            .ok_or_else(|| "if missing condition".to_string())?,
        ctx,
    )?;
    let then_code = compile_expr_discarding_result(
        node.children
            .get(2)
            .ok_or_else(|| "if missing then".to_string())?,
        ctx,
    )?;
    let else_code = compile_expr_discarding_result(
        node.children
            .get(3)
            .ok_or_else(|| "if missing else".to_string())?,
        ctx,
    )?;
    Ok(format!(
        "{cond}\nif\n  {then_code}\nelse\n  {else_code}\nend"
    ))
}

fn compile_and_discarding_result(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let left = compile_expr(
        node.children
            .get(1)
            .ok_or_else(|| "and missing left operand".to_string())?,
        ctx,
    )?;
    let right = compile_expr_discarding_result(
        node.children
            .get(2)
            .ok_or_else(|| "and missing right operand".to_string())?,
        ctx,
    )?;
    Ok(format!("{left}\nif\n  {right}\nend"))
}

fn compile_or_discarding_result(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let left = compile_expr(
        node.children
            .get(1)
            .ok_or_else(|| "or missing left operand".to_string())?,
        ctx,
    )?;
    let right = compile_expr_discarding_result(
        node.children
            .get(2)
            .ok_or_else(|| "or missing right operand".to_string())?,
        ctx,
    )?;
    Ok(format!("{left}\ni32.eqz\nif\n  {right}\nend"))
}

fn is_borrowing_accessor_expr(node: &TypedExpression) -> bool {
    match &node.expr {
        Expression::Apply(items) if !items.is_empty() => matches!(
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
    let child_offset = if node.children.len() + 1 == items.len() {
        1
    } else {
        0
    };
    if item_idx < child_offset {
        None
    } else {
        node.children.get(item_idx - child_offset)
    }
}

fn lambda_params_and_body<'a>(
    lambda_node: &'a TypedExpression,
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

fn recursive_passthrough_param_origins(
    expr: &Expression,
    self_name: &str,
    params: &[String],
) -> Option<HashSet<usize>> {
    match expr {
        Expression::Word(name) => params
            .iter()
            .position(|param| param == name)
            .map(|idx| HashSet::from([idx])),
        Expression::Apply(items) if !items.is_empty() => {
            let Expression::Word(op) = &items[0] else {
                return None;
            };
            match op.as_str() {
                "as" | "char" => items.get(1).and_then(|inner| {
                    recursive_passthrough_param_origins(inner, self_name, params)
                }),
                "do" | "block" => items
                    .last()
                    .and_then(|last| recursive_passthrough_param_origins(last, self_name, params)),
                "if" => {
                    let mut origins =
                        recursive_passthrough_param_origins(items.get(2)?, self_name, params)?;
                    origins.extend(recursive_passthrough_param_origins(
                        items.get(3)?,
                        self_name,
                        params,
                    )?);
                    Some(origins)
                }
                name if name == self_name => {
                    let mut origins = HashSet::new();
                    for arg in items.iter().skip(1) {
                        origins
                            .extend(recursive_passthrough_param_origins(arg, self_name, params)?);
                    }
                    Some(origins)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn resolve_callable_binding_from_arg(
    arg: &TypedExpression,
    callable_env: &HashMap<String, CallableBinding>,
    lambda_bindings: &HashMap<String, TypedExpression>,
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
                    return apply_child_at(arg, 1).and_then(|inner| {
                        resolve_callable_binding_from_arg(inner, callable_env, lambda_bindings)
                    });
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
    invoked_name: Option<&str>,
    env: &HashMap<String, bool>,
    callable_env: &HashMap<String, CallableBinding>,
    lambda_bindings: &HashMap<String, TypedExpression>,
    call_stack: &mut Vec<String>,
    depth: usize,
) -> Option<bool> {
    let (params, body) = lambda_params_and_body(lambda_node)?;
    if let Some(name) = invoked_name {
        if let Some(origins) = recursive_passthrough_param_origins(&body.expr, name, &params) {
            return Some(origins.into_iter().any(|idx| {
                apply_child_at(call_node, idx + 1)
                    .map(|arg| {
                        is_borrowed_managed_rhs_with_env(
                            arg,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1,
                        )
                    })
                    .unwrap_or(true)
            }));
        }
    }
    let mut lambda_env: HashMap<String, bool> = HashMap::new();
    let mut lambda_callable_env: HashMap<String, CallableBinding> = HashMap::new();
    for (idx, param_name) in params.iter().enumerate() {
        let arg_node = apply_child_at(call_node, idx + 1);
        let arg_borrowed = arg_node
            .map(|arg| {
                is_borrowed_managed_rhs_with_env(
                    arg,
                    env,
                    callable_env,
                    lambda_bindings,
                    call_stack,
                    depth + 1,
                )
            })
            .unwrap_or(false);
        lambda_env.insert(param_name.clone(), arg_borrowed);

        if let Some(arg_node) = arg_node {
            if let Some(callable_binding) =
                resolve_callable_binding_from_arg(arg_node, callable_env, lambda_bindings)
            {
                lambda_callable_env.insert(param_name.clone(), callable_binding);
            }
        }
    }

    Some(is_borrowed_managed_rhs_with_env(
        body,
        &lambda_env,
        &lambda_callable_env,
        lambda_bindings,
        call_stack,
        depth + 1,
    ))
}

fn is_borrowed_managed_rhs_with_env(
    node: &TypedExpression,
    env: &HashMap<String, bool>,
    callable_env: &HashMap<String, CallableBinding>,
    lambda_bindings: &HashMap<String, TypedExpression>,
    call_stack: &mut Vec<String>,
    depth: usize,
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
                    .map(|n| {
                        is_borrowed_managed_rhs_with_env(
                            n,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1,
                        )
                    })
                    .unwrap_or(false);
            }
            if is_borrowing_accessor_expr(node) {
                return apply_child_at(node, 1)
                    .map(|n| {
                        is_borrowed_managed_rhs_with_env(
                            n,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1,
                        )
                    })
                    .unwrap_or(false);
            }
            if op == "if" {
                let then_borrowed = apply_child_at(node, 2)
                    .map(|n| {
                        is_borrowed_managed_rhs_with_env(
                            n,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1,
                        )
                    })
                    .unwrap_or(false);
                let else_borrowed = apply_child_at(node, 3)
                    .map(|n| {
                        is_borrowed_managed_rhs_with_env(
                            n,
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1,
                        )
                    })
                    .unwrap_or(false);
                // A call result is safe to release only when every possible
                // branch returns an owned value. If either branch may return
                // a borrowed alias, classify the whole result as borrowed.
                return then_borrowed || else_borrowed;
            }
            if op == "do" {
                let mut scoped_env = env.clone();
                let mut scoped_callable_env = callable_env.clone();
                if items.len() > 1 {
                    for i in 1..items.len() - 1 {
                        if let Expression::Apply(let_items) = &items[i] {
                            if let [Expression::Word(kw), Expression::Word(name), _] =
                                &let_items[..]
                            {
                                if kw == "let" || kw == "letrec" || kw == "mut" {
                                    let rhs_borrowed = apply_child_at(node, i)
                                        .and_then(|let_node| let_node.children.get(2))
                                        .map(|rhs| {
                                            is_borrowed_managed_rhs_with_env(
                                                rhs,
                                                &scoped_env,
                                                &scoped_callable_env,
                                                lambda_bindings,
                                                call_stack,
                                                depth + 1,
                                            )
                                        })
                                        .unwrap_or(false);
                                    scoped_env.insert(name.clone(), rhs_borrowed);
                                    if let Some(rhs_node) =
                                        apply_child_at(node, i).and_then(|n| n.children.get(2))
                                    {
                                        if let Some(callable_binding) =
                                            resolve_callable_binding_from_arg(
                                                rhs_node,
                                                &scoped_callable_env,
                                                lambda_bindings,
                                            )
                                        {
                                            scoped_callable_env
                                                .insert(name.clone(), callable_binding);
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
                    .map(|last| {
                        is_borrowed_managed_rhs_with_env(
                            last,
                            &scoped_env,
                            &scoped_callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1,
                        )
                    })
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
                                Some(name),
                                env,
                                callable_env,
                                lambda_bindings,
                                call_stack,
                                depth + 1,
                            )
                            .unwrap_or(false);
                            call_stack.pop();
                            return result;
                        }
                    }
                    CallableBinding::Lambda(lambda_node) => {
                        return analyze_borrow_for_lambda_invocation(
                            lambda_node,
                            node,
                            Some(op),
                            env,
                            callable_env,
                            lambda_bindings,
                            call_stack,
                            depth + 1,
                        )
                        .unwrap_or(false);
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
                    Some(op),
                    env,
                    callable_env,
                    lambda_bindings,
                    call_stack,
                    depth + 1,
                )
                .unwrap_or(false);
                call_stack.pop();
                return result;
            }
            if matches!(
                op,
                "vector"
                    | "tuple"
                    | "lambda"
                    | "box"
                    | "int"
                    | "dec"
                    | "bool"
                    | "string"
                    | "integers"
                    | "bools"
                    | "decimals"
                    | "strings"
                    | "__vec_new_zeroed_i32"
                    | "__vec_new_uninit_i32"
                    | "serialize"
                    | "deserialize"
            ) {
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
    lambda_bindings: &HashMap<String, TypedExpression>,
) -> bool {
    let env = HashMap::new();
    let callable_env = HashMap::new();
    let mut call_stack = Vec::new();
    is_borrowed_managed_rhs_with_env(
        node,
        &env,
        &callable_env,
        lambda_bindings,
        &mut call_stack,
        0,
    )
}

fn should_release_set_rhs(
    node: &TypedExpression,
    lambda_bindings: &HashMap<String, TypedExpression>,
) -> bool {
    if !node
        .typ
        .as_ref()
        .map(is_managed_local_type)
        .unwrap_or(false)
    {
        return false;
    }
    !is_borrowed_managed_rhs_expr(node, lambda_bindings)
}

fn emit_release_fresh_owned_temp(tmp_val: usize, ty: Option<&Type>) -> String {
    format!(
        "local.get {}\ncall {}\ndrop",
        tmp_val,
        rc_release_for_opt_type(ty)
    )
}

fn emit_direct_builder_scalar_store_i32(
    ptr: &str,
    idx: &str,
    value: &str,
    ctx: &Ctx<'_>,
) -> String {
    let ptr_tmp = ctx.tmp_i32 + 1;
    let idx_tmp = ctx.tmp_i32 + 2;
    let val_tmp = ctx.tmp_i32 + 3;
    let len_tmp = ctx.tmp_i32 + 4;
    let addr_tmp = ctx.tmp_i32 + 5;
    format!(
        "{ptr}\nlocal.set {ptr_tmp}\n{idx}\nlocal.set {idx_tmp}\n{value}\nlocal.set {val_tmp}\n\
local.get {ptr_tmp}\ni32.load\nlocal.set {len_tmp}\n\
local.get {idx_tmp}\ni32.const 0\ni32.ge_s\n\
local.get {idx_tmp}\nlocal.get {len_tmp}\ni32.lt_s\ni32.and\n\
if (result i32)\n\
  local.get {ptr_tmp}\n  i32.const 16\n  i32.add\n  i32.load\n\
  local.get {idx_tmp}\n  i32.const 4\n  i32.mul\n  i32.add\n\
  local.set {addr_tmp}\n\
  local.get {addr_tmp}\n  local.get {val_tmp}\n  i32.store\n\
  i32.const 0\n\
else\n\
  unreachable\n\
end"
    )
}

fn local_lambda_binding_needs_runtime_value(name: &str, following_items: &[Expression]) -> bool {
    for expr in following_items {
        if let Expression::Apply(items) = expr {
            if let [Expression::Word(kw), Expression::Word(bound_name), _] = &items[..] {
                if (kw == "let" || kw == "letrec" || kw == "mut") && bound_name == name {
                    return false;
                }
            }
        }
        if expr_uses_name_as_value(name, expr, false) {
            return true;
        }
    }
    false
}

fn items_bind_name(name: &str, items: &[Expression]) -> bool {
    items.iter().any(|expr| {
        matches!(
            expr,
            Expression::Apply(xs)
                if matches!(
                    &xs[..],
                    [Expression::Word(kw), Expression::Word(bound_name), _]
                        if (kw == "let" || kw == "letrec" || kw == "mut") && bound_name == name
                )
        )
    })
}

fn expr_uses_name_as_value(name: &str, expr: &Expression, inside_lambda: bool) -> bool {
    match expr {
        Expression::Word(w) => w == name,
        Expression::Apply(items) => {
            if items.is_empty() {
                return false;
            }
            if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
                let mut bound = HashSet::new();
                if items.len() >= 2 {
                    for p in &items[1..items.len() - 1] {
                        collect_pattern_words(p, &mut bound);
                    }
                    if bound.contains(name) {
                        return false;
                    }
                    return items
                        .last()
                        .map(|body| expr_uses_name_as_value(name, body, true))
                        .unwrap_or(false);
                }
                return false;
            }
            if let [Expression::Word(kw), Expression::Word(bound_name), rhs] = &items[..] {
                if kw == "let" || kw == "letrec" || kw == "mut" {
                    return expr_uses_name_as_value(name, rhs, inside_lambda)
                        || (bound_name != name
                            && items[2..]
                                .iter()
                                .any(|item| expr_uses_name_as_value(name, item, inside_lambda)));
                }
            }
            if !inside_lambda && matches!(items.first(), Some(Expression::Word(w)) if w == name) {
                return items[1..]
                    .iter()
                    .any(|item| expr_uses_name_as_value(name, item, inside_lambda));
            }
            items
                .iter()
                .any(|item| expr_uses_name_as_value(name, item, inside_lambda))
        }
        _ => false,
    }
}

fn expr_uses_name_via_local_lambda(
    name: &str,
    expr: &Expression,
    lambda_bindings: &HashMap<String, TypedExpression>,
) -> bool {
    match expr {
        Expression::Word(w) => lambda_bindings
            .get(w)
            .map(|lambda| expr_uses_name_as_value(name, &lambda.expr, false))
            .unwrap_or(false),
        Expression::Apply(items) => {
            if items.is_empty() {
                return false;
            }
            if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
                let mut bound = HashSet::new();
                if items.len() >= 2 {
                    for p in &items[1..items.len() - 1] {
                        collect_pattern_words(p, &mut bound);
                    }
                    if bound.contains(name) {
                        return false;
                    }
                }
            }
            if let Some(Expression::Word(op)) = items.first() {
                if let Some(lambda) = lambda_bindings.get(op) {
                    if expr_uses_name_as_value(name, &lambda.expr, false) {
                        return true;
                    }
                }
            }
            items
                .iter()
                .any(|item| expr_uses_name_via_local_lambda(name, item, lambda_bindings))
        }
        _ => false,
    }
}

fn append_last_use_releases_for_do_expr(
    parts: &mut Vec<String>,
    managed_do_locals: &[(String, ManagedRefSlot)],
    current_expr: &Expression,
    later_exprs: &[Expression],
    lambda_bindings: &HashMap<String, TypedExpression>,
    moved_from_slot: Option<usize>,
) {
    for (name, reference) in managed_do_locals {
        if moved_from_slot == Some(reference.slot) {
            continue;
        }
        if (expr_uses_name_as_value(name, current_expr, false)
            || expr_uses_name_via_local_lambda(name, current_expr, lambda_bindings))
            && !later_exprs.iter().any(|expr| {
                expr_uses_name_as_value(name, expr, false)
                    || expr_uses_name_via_local_lambda(name, expr, lambda_bindings)
            })
        {
            parts.push(format!(
                "local.get {}\ncall {}\ndrop\ni32.const 0\nlocal.set {}",
                reference.slot,
                reference.kind.release(),
                reference.slot
            ));
        }
    }
}

fn direct_last_use_managed_move_source(
    value_node: Option<&TypedExpression>,
    destination_name: &str,
    managed_do_locals: &[(String, ManagedRefSlot)],
    later_exprs: &[Expression],
    lambda_bindings: &HashMap<String, TypedExpression>,
) -> Option<ManagedRefSlot> {
    let Expression::Word(source_name) = &value_node?.expr else {
        return None;
    };
    if source_name == destination_name {
        return None;
    }
    let reference = managed_do_locals
        .iter()
        .find_map(|(name, reference)| (name == source_name).then_some(*reference))?;
    if later_exprs.iter().any(|expr| {
        expr_uses_name_as_value(source_name, expr, false)
            || expr_uses_name_via_local_lambda(source_name, expr, lambda_bindings)
    }) {
        return None;
    }
    Some(reference)
}

fn compile_last_use_managed_alter_move(
    node: &TypedExpression,
    later_exprs: &[Expression],
    managed_do_locals: &[(String, ManagedRefSlot)],
    lambda_bindings: &HashMap<String, TypedExpression>,
    ctx: &Ctx<'_>,
) -> Option<(String, usize)> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    let [Expression::Word(op), Expression::Word(target), Expression::Word(source)] =
        items.as_slice()
    else {
        return None;
    };
    if op != "alter!" || target == source {
        return None;
    }
    let source_ref = managed_do_locals
        .iter()
        .find_map(|(name, reference)| (name == source).then_some(*reference))?;
    if later_exprs.iter().any(|expr| {
        expr_uses_name_as_value(source, expr, false)
            || expr_uses_name_via_local_lambda(source, expr, lambda_bindings)
    }) {
        return None;
    }
    let target_slot = *ctx.locals.get(target)?;
    if target_slot == source_ref.slot {
        return None;
    }
    let target_type = ctx.local_types.get(target)?;
    if !is_managed_local_type(target_type) {
        return None;
    }
    let value_tmp = ctx.tmp_i32;
    Some((
        format!(
            "local.get {}\n\
             local.set {value_tmp}\n\
             local.get {target_slot}\n\
             call {}\n\
             drop\n\
             local.get {value_tmp}\n\
             local.set {target_slot}\n\
             i32.const 0\n\
             local.set {}\n\
             i32.const 0",
            source_ref.slot,
            rc_release_for_type(target_type),
            source_ref.slot,
        ),
        source_ref.slot,
    ))
}

fn cached_length_vector_before_loop(
    items: &[Expression],
    loop_index: usize,
    bound_name: &str,
    current_function: Option<&str>,
) -> Option<String> {
    for binding_index in (1..loop_index).rev() {
        let Expression::Apply(binding) = &items[binding_index] else {
            continue;
        };
        if matches!(binding.as_slice(), [Expression::Word(op), Expression::Word(name), ..]
            if name == bound_name && op != "let")
        {
            return None;
        }
        let [Expression::Word(op), Expression::Word(name), Expression::Apply(rhs)] =
            binding.as_slice()
        else {
            continue;
        };
        if name != bound_name {
            continue;
        }
        if op != "let" {
            return None;
        }
        let [Expression::Word(length), Expression::Word(vector_name)] = rhs.as_slice() else {
            return None;
        };
        if length != "length" {
            return None;
        }
        let stable = items[binding_index + 1..loop_index].iter().all(|expr| {
            !expr_mutates_scalar_name(expr, bound_name)
                && !expr_mutates_vector_name_except_self(expr, vector_name, current_function)
        });
        return stable.then(|| vector_name.clone());
    }
    None
}

fn rewrite_cached_length_while(
    node: &TypedExpression,
    vector_name: &str,
    ctx: &Ctx<'_>,
) -> Option<TypedExpression> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    let [Expression::Word(while_op), condition, body] = items.as_slice() else {
        return None;
    };
    if while_op != "while" {
        return None;
    }
    let Expression::Apply(condition_items) = condition else {
        return None;
    };
    let [Expression::Word(compare), Expression::Word(index), Expression::Word(_bound)] =
        condition_items.as_slice()
    else {
        return None;
    };
    if compare != "<" {
        return None;
    }
    let body_node = node.children.get(2)?;
    if !body_has_only_final_positive_increment(body_node, index, ctx)
        || !vector_mutations_are_loop_replacement_sets(
            &body_node.expr,
            vector_name,
            index,
            ctx.current_function,
        )
    {
        return None;
    }
    let vector_type = ctx.local_types.get(vector_name)?.clone();
    let length_expr = Expression::Apply(vec![
        Expression::Word("length".to_string()),
        Expression::Word(vector_name.to_string()),
    ]);
    let length_node = TypedExpression {
        expr: length_expr.clone(),
        typ: Some(Type::Int),
        effect: EffectFlags::PURE,
        children: vec![
            TypedExpression {
                expr: Expression::Word("length".to_string()),
                typ: None,
                effect: EffectFlags::PURE,
                children: Vec::new(),
            },
            TypedExpression {
                expr: Expression::Word(vector_name.to_string()),
                typ: Some(vector_type),
                effect: EffectFlags::PURE,
                children: Vec::new(),
            },
        ],
    };
    let mut condition_node = node.children.get(1)?.clone();
    condition_node.expr = Expression::Apply(vec![
        Expression::Word(compare.clone()),
        Expression::Word(index.clone()),
        length_expr,
    ]);
    *condition_node.children.get_mut(2)? = length_node;
    let mut rewritten = node.clone();
    rewritten.expr = Expression::Apply(vec![
        Expression::Word("while".to_string()),
        condition_node.expr.clone(),
        body.clone(),
    ]);
    *rewritten.children.get_mut(1)? = condition_node;
    Some(rewritten)
}

fn compile_do(
    items: &[Expression],
    node: &TypedExpression,
    ctx: &Ctx<'_>,
) -> Result<String, String> {
    if items.len() <= 1 {
        return Ok("i32.const 0".to_string());
    }
    let child_offset = if node.children.len() + 1 == items.len() {
        1
    } else {
        0
    };
    let child_at = |item_idx: usize| -> Option<&TypedExpression> {
        if item_idx < child_offset {
            None
        } else {
            node.children.get(item_idx - child_offset)
        }
    };
    // Name-based local maps lose shadowed bindings, which can make alias checks
    // miss live refs and incorrectly release them. Be conservative: compare a
    // managed temporary against every non-temp slot in this function.
    let managed_local_slots: Vec<usize> = (0..ctx.tmp_i32).collect();
    let mut parts = Vec::new();
    let mut scoped_lambda_bindings = ctx.lambda_bindings.clone();
    let mut scoped_materialized_scalar_local_slots = ctx.materialized_scalar_local_slots.clone();
    let mut scoped_nonnegative_int_locals = ctx.nonnegative_int_locals.clone();
    let mut scoped_proven_scalar_vec_min_lengths = ctx.proven_scalar_vec_min_lengths.clone();
    let mut scoped_exact_int_locals: HashMap<String, i32> = HashMap::new();
    let managed_do_locals: Vec<(String, ManagedRefSlot)> = items
        .iter()
        .filter_map(|expr| {
            let Expression::Apply(let_items) = expr else {
                return None;
            };
            let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] else {
                return None;
            };
            if kw != "let" && kw != "letrec" && kw != "mut" {
                return None;
            }
            let slot = *ctx.locals.get(name)?;
            let typ = ctx.local_types.get(name)?;
            if is_managed_local_type(typ) && !is_borrowed_projection_local(name, ctx) {
                Some((name.clone(), ManagedRefSlot::new(slot, typ)))
            } else {
                None
            }
        })
        .collect();
    let mut skip_until = 0usize;
    for i in 1..items.len() - 1 {
        if i < skip_until {
            continue;
        }
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
                    let can_elide_lambda_value =
                        devirtualize_mode_from_env()? != DevirtualizeMode::Off &&
                        self_capture_idx.is_none() &&
                        !items_bind_name(name, &items[1..i]) &&
                        val_node
                            .map(|n| {
                                matches!(
                                    &n.expr,
                                    Expression::Apply(xs)
                                        if kw != "mut"
                                            && matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                                )
                            })
                            .unwrap_or(false) &&
                        !local_lambda_binding_needs_runtime_value(name, &items[i + 1..]);
                    if let Some(n) = val_node {
                        match &n.expr {
                            Expression::Apply(xs)
                                if kw != "mut"
                                    && matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda") =>
                            {
                                scoped_lambda_bindings.insert(name.clone(), n.clone());
                            }
                            Expression::Word(alias) => {
                                if kw != "mut" {
                                    if let Some(target) = scoped_lambda_bindings.get(alias).cloned()
                                    {
                                        scoped_lambda_bindings.insert(name.clone(), target);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if can_elide_lambda_value {
                        continue;
                    }
                    if kw == "let"
                        && val_node.and_then(scalar_vector_literal_len) == Some(0)
                        && i + 2 < items.len()
                    {
                        if let (
                            Some(Expression::Apply(next_mut_items)),
                            Some(fill_node),
                            Some(local_idx),
                        ) = (items.get(i + 1), child_at(i + 2), ctx.locals.get(name))
                        {
                            if let [Expression::Word(next_kw), Expression::Word(idx_name), Expression::Int(start)] =
                                &next_mut_items[..]
                            {
                                if next_kw == "mut" && *start >= 0 {
                                    let mut fill_nonnegative =
                                        scoped_nonnegative_int_locals.clone();
                                    fill_nonnegative.insert(idx_name.clone());
                                    let mut fill_exact = scoped_exact_int_locals.clone();
                                    fill_exact.insert(idx_name.clone(), *start);
                                    let fill_ctx = Ctx {
                                        fn_sigs: ctx.fn_sigs,
                                        fn_ids: ctx.fn_ids,
                                        extern_names: ctx.extern_names,
                                        lambda_ids: ctx.lambda_ids,
                                        closure_defs: ctx.closure_defs,
                                        lambda_bindings: &scoped_lambda_bindings,
                                        current_function: ctx.current_function,
                                        locals: ctx.locals.clone(),
                                        local_types: ctx.local_types.clone(),
                                        materialized_scalar_local_slots:
                                            scoped_materialized_scalar_local_slots.clone(),
                                        hoisted_scalar_vec_data_slots: ctx
                                            .hoisted_scalar_vec_data_slots
                                            .clone(),
                                        proven_scalar_vec_min_lengths:
                                            scoped_proven_scalar_vec_min_lengths.clone(),
                                        definitely_materialized_top_level_scalar_names: ctx
                                            .definitely_materialized_top_level_scalar_names,
                                        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                                        nonnegative_int_locals: &fill_nonnegative,
                                        tmp_i32: ctx.tmp_i32,
                                    };
                                    if let Some((slot, len, value, idx_name)) =
                                        append_fill_loop_const_i32(
                                            fill_node,
                                            &fill_ctx,
                                            &fill_exact,
                                        )
                                    {
                                        if slot == *local_idx {
                                            parts.push(format!(
                                                "i32.const {len}\n\
                                                 i32.const {value}\n\
                                                 call $vec_new_filled_i32\n\
                                                 local.set {local_idx}"
                                            ));
                                            scoped_materialized_scalar_local_slots
                                                .insert(*local_idx);
                                            scoped_proven_scalar_vec_min_lengths
                                                .insert(*local_idx, len);
                                            if let Some(idx_slot) = ctx.locals.get(&idx_name) {
                                                let final_idx = start.saturating_add(len);
                                                parts.push(format!(
                                                    "i32.const {final_idx}\nlocal.set {idx_slot}"
                                                ));
                                                scoped_nonnegative_int_locals
                                                    .insert(idx_name.clone());
                                                scoped_exact_int_locals.insert(idx_name, final_idx);
                                            }
                                            skip_until = i + 3;
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let value = val_node
                        .ok_or_else(|| format!("Missing let value for {}", name))
                        .and_then(|n| {
                            let scoped_ctx = Ctx {
                                fn_sigs: ctx.fn_sigs,
                                fn_ids: ctx.fn_ids,
                    extern_names: ctx.extern_names,
                                lambda_ids: ctx.lambda_ids,
                                closure_defs: ctx.closure_defs,
                                lambda_bindings: &scoped_lambda_bindings,
                                current_function: ctx.current_function,
                                locals: ctx.locals.clone(),
                                local_types: ctx.local_types.clone(),
                                materialized_scalar_local_slots: scoped_materialized_scalar_local_slots.clone(),
                                hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                                proven_scalar_vec_min_lengths: scoped_proven_scalar_vec_min_lengths.clone(),
                                definitely_materialized_top_level_scalar_names: ctx.definitely_materialized_top_level_scalar_names,
                                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                                nonnegative_int_locals: &scoped_nonnegative_int_locals,
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
                        let managed_local = ctx
                            .local_types
                            .get(name)
                            .map(is_managed_local_type)
                            .unwrap_or(false);
                        let borrowed_rhs = val_node
                            .map(|n| is_borrowed_managed_rhs_expr(n, &scoped_lambda_bindings))
                            .unwrap_or(false);
                        let move_source = managed_local
                            .then(|| {
                                direct_last_use_managed_move_source(
                                    val_node,
                                    name,
                                    &managed_do_locals,
                                    &items[i + 1..],
                                    &scoped_lambda_bindings,
                                )
                            })
                            .flatten();
                        let value = if managed_local
                            && borrowed_rhs
                            && move_source.is_none()
                            && !is_borrowed_projection_local(name, ctx)
                        {
                            let tmp_owned = ctx.tmp_i32 + 2;
                            let retain = ctx
                                .local_types
                                .get(name)
                                .map(rc_retain_for_type)
                                .unwrap_or("$rc_retain");
                            format!(
                                "{value}\nlocal.tee {}\ncall {retain}\ndrop\nlocal.get {}",
                                tmp_owned, tmp_owned,
                            )
                        } else {
                            value
                        };
                        parts.push(format!("{value}\nlocal.set {}", local_idx));
                        if let Some(source) = move_source {
                            parts.push(format!("i32.const 0\nlocal.set {}", source.slot));
                        }
                        if val_node
                            .map(|n| {
                                expr_is_definitely_materialized_scalar_vector(
                                    n,
                                    &scoped_materialized_scalar_local_slots,
                                    &ctx.locals,
                                    ctx.definitely_materialized_top_level_scalar_names,
                                )
                            })
                            .unwrap_or(false)
                        {
                            scoped_materialized_scalar_local_slots.insert(*local_idx);
                        }
                        if let Some(len) = val_node.and_then(scalar_vector_literal_len) {
                            scoped_proven_scalar_vec_min_lengths.insert(*local_idx, len);
                        }
                        if let Some(cap_idx) = self_capture_idx {
                            // Recursive local lambda: fill self-capture after binding is assigned.
                            // Use non-ref capture to avoid RC self-cycles.
                            parts.push(format!(
                                "local.get {}\ni32.const {}\nlocal.get {}\ncall $closure_set\ndrop",
                                local_idx, cap_idx, local_idx
                            ));
                        }
                        if val_node
                            .map(|n| {
                                typed_expr_is_nonnegative_int(n, &scoped_nonnegative_int_locals)
                            })
                            .unwrap_or(false)
                        {
                            scoped_nonnegative_int_locals.insert(name.clone());
                        } else if kw == "mut" {
                            scoped_nonnegative_int_locals.remove(name);
                        }
                        if let Some(Expression::Int(value)) = val_node.map(|n| &n.expr) {
                            scoped_exact_int_locals.insert(name.clone(), *value);
                        } else {
                            scoped_exact_int_locals.remove(name);
                        }
                    } else {
                        return Err(format!("Unknown local '{}'", name));
                    }
                    if let Some(value_node) = val_node {
                        update_scalar_vec_min_lengths_after_expr(
                            value_node,
                            ctx,
                            &mut scoped_proven_scalar_vec_min_lengths,
                        );
                    }
                    append_last_use_releases_for_do_expr(
                        &mut parts,
                        &managed_do_locals,
                        &items[i],
                        &items[i + 1..],
                        &scoped_lambda_bindings,
                        direct_last_use_managed_move_source(
                            val_node,
                            name,
                            &managed_do_locals,
                            &items[i + 1..],
                            &scoped_lambda_bindings,
                        )
                        .map(|reference| reference.slot),
                    );
                    continue;
                }
            }
        }
        if let Some(n) = child_at(i) {
            if let Some((code, moved_from_slot)) = compile_last_use_managed_alter_move(
                n,
                &items[i + 1..],
                &managed_do_locals,
                &scoped_lambda_bindings,
                ctx,
            ) {
                parts.push(code);
                update_scalar_vec_min_lengths_after_expr(
                    n,
                    ctx,
                    &mut scoped_proven_scalar_vec_min_lengths,
                );
                collect_altered_int_locals(&n.expr, &mut scoped_exact_int_locals);
                append_last_use_releases_for_do_expr(
                    &mut parts,
                    &managed_do_locals,
                    &items[i],
                    &items[i + 1..],
                    &scoped_lambda_bindings,
                    Some(moved_from_slot),
                );
                continue;
            }
            let rewritten_loop = match &n.expr {
                Expression::Apply(loop_items)
                    if matches!(loop_items.as_slice(), [Expression::Word(op), Expression::Apply(condition), _]
                        if op == "while" && matches!(condition.as_slice(), [Expression::Word(compare), _, Expression::Word(_)] if compare == "<")) =>
                {
                    let bound_name = match &loop_items[1] {
                        Expression::Apply(condition) => match condition.get(2) {
                            Some(Expression::Word(name)) => Some(name.as_str()),
                            _ => None,
                        },
                        _ => None,
                    };
                    bound_name
                        .and_then(|bound| {
                            cached_length_vector_before_loop(items, i, bound, ctx.current_function)
                        })
                        .and_then(|vector| rewrite_cached_length_while(n, &vector, ctx))
                }
                _ => None,
            };
            let n = rewritten_loop.as_ref().unwrap_or(n);
            let scoped_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                extern_names: ctx.extern_names,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: &scoped_lambda_bindings,
                current_function: ctx.current_function,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                materialized_scalar_local_slots: scoped_materialized_scalar_local_slots.clone(),
                hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                proven_scalar_vec_min_lengths: scoped_proven_scalar_vec_min_lengths.clone(),
                definitely_materialized_top_level_scalar_names: ctx
                    .definitely_materialized_top_level_scalar_names,
                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                nonnegative_int_locals: &scoped_nonnegative_int_locals,
                tmp_i32: ctx.tmp_i32,
            };
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
                let c = compile_expr(n, &scoped_ctx)?;
                let release = n
                    .typ
                    .as_ref()
                    .map(rc_release_for_type)
                    .unwrap_or("$rc_release");
                let tmp_val = ctx.tmp_i32;
                let tmp_keep = ctx.tmp_i32 + 1;
                let mut blk = Vec::new();
                if managed_local_slots.is_empty() {
                    blk.push(format!("{c}\ncall {release}\ndrop"));
                } else {
                    blk.push(format!("{c}\nlocal.set {}", tmp_val));
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
                    blk.push(format!(
                        "local.get {}\ni32.eqz\nif\n  local.get {}\n  call {release}\n  drop\nend",
                        tmp_keep, tmp_val
                    ));
                }
                parts.push(blk.join("\n"));
            } else {
                let c = compile_expr_discarding_result(n, &scoped_ctx)?;
                if !c.is_empty() {
                    parts.push(c);
                }
            }
            if let Some(slot) = scalar_vector_set_target_slot(&n.expr, ctx) {
                scoped_materialized_scalar_local_slots.insert(slot);
            }
            update_scalar_vec_min_lengths_after_expr(
                n,
                ctx,
                &mut scoped_proven_scalar_vec_min_lengths,
            );
            if let Some((slot, added_len)) =
                append_fill_loop_min_length(n, &scoped_ctx, &scoped_exact_int_locals)
            {
                scoped_proven_scalar_vec_min_lengths
                    .entry(slot)
                    .and_modify(|len| *len = len.saturating_add(added_len))
                    .or_insert(added_len);
            }
            collect_altered_int_locals(&n.expr, &mut scoped_exact_int_locals);
        }
        append_last_use_releases_for_do_expr(
            &mut parts,
            &managed_do_locals,
            &items[i],
            &items[i + 1..],
            &scoped_lambda_bindings,
            None,
        );
    }
    let last_node =
        child_at(items.len() - 1).ok_or_else(|| "Missing final do expression".to_string())?;
    let last = Some(last_node)
        .ok_or_else(|| "Missing final do expression".to_string())
        .and_then(|n| {
            let scoped_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                extern_names: ctx.extern_names,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: &scoped_lambda_bindings,
                current_function: ctx.current_function,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                materialized_scalar_local_slots: scoped_materialized_scalar_local_slots.clone(),
                hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                proven_scalar_vec_min_lengths: scoped_proven_scalar_vec_min_lengths.clone(),
                definitely_materialized_top_level_scalar_names: ctx
                    .definitely_materialized_top_level_scalar_names,
                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                nonnegative_int_locals: &scoped_nonnegative_int_locals,
                tmp_i32: ctx.tmp_i32,
            };
            compile_expr(n, &scoped_ctx)
        })?;
    parts.push(last);
    Ok(parts.join("\n"))
}

fn compile_tail_do(
    items: &[Expression],
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    self_name: &str,
    arity: usize,
    releasable_ref_slots: &[ManagedRefSlot],
) -> Result<Option<String>, String> {
    if items.len() <= 1 {
        return Ok(None);
    }
    let child_offset = if node.children.len() + 1 == items.len() {
        1
    } else {
        0
    };
    let child_at = |item_idx: usize| -> Option<&TypedExpression> {
        if item_idx < child_offset {
            None
        } else {
            node.children.get(item_idx - child_offset)
        }
    };
    let managed_local_slots: Vec<usize> = (0..ctx.tmp_i32).collect();
    let mut parts = Vec::new();
    let mut scoped_lambda_bindings = ctx.lambda_bindings.clone();
    let mut scoped_materialized_scalar_local_slots = ctx.materialized_scalar_local_slots.clone();
    let mut scoped_nonnegative_int_locals = ctx.nonnegative_int_locals.clone();
    let mut scoped_proven_scalar_vec_min_lengths = ctx.proven_scalar_vec_min_lengths.clone();
    let mut scoped_exact_int_locals: HashMap<String, i32> = HashMap::new();
    let managed_do_locals: Vec<(String, ManagedRefSlot)> = items
        .iter()
        .filter_map(|expr| {
            let Expression::Apply(let_items) = expr else {
                return None;
            };
            let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] else {
                return None;
            };
            if kw != "let" && kw != "letrec" && kw != "mut" {
                return None;
            }
            let slot = *ctx.locals.get(name)?;
            let typ = ctx.local_types.get(name)?;
            if is_managed_local_type(typ) && !is_borrowed_projection_local(name, ctx) {
                Some((name.clone(), ManagedRefSlot::new(slot, typ)))
            } else {
                None
            }
        })
        .collect();
    let mut skip_until = 0usize;
    for i in 1..items.len() - 1 {
        if i < skip_until {
            continue;
        }
        if let Expression::Apply(let_items) = &items[i] {
            if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                if kw == "let" || kw == "letrec" || kw == "mut" {
                    let val_node = child_at(i).and_then(|n| n.children.get(2));
                    let self_capture_idx = val_node.and_then(|n| {
                        if kw != "mut"
                            && matches!(&n.expr, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                        {
                            let key = n.expr.to_lisp();
                            ctx.closure_defs
                                .get(&key)
                                .and_then(|d| d.captures.iter().position(|c| c == name))
                        } else {
                            None
                        }
                    });
                    let can_elide_lambda_value =
                        devirtualize_mode_from_env()? != DevirtualizeMode::Off &&
                        self_capture_idx.is_none() &&
                        !items_bind_name(name, &items[1..i]) &&
                        val_node
                            .map(|n| {
                                matches!(
                                    &n.expr,
                                    Expression::Apply(xs)
                                        if kw != "mut"
                                            && matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                                )
                            })
                            .unwrap_or(false) &&
                        !local_lambda_binding_needs_runtime_value(name, &items[i + 1..]);
                    if let Some(n) = val_node {
                        match &n.expr {
                            Expression::Apply(xs)
                                if kw != "mut"
                                    && matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda") =>
                            {
                                scoped_lambda_bindings.insert(name.clone(), n.clone());
                            }
                            Expression::Word(alias) => {
                                if kw != "mut" {
                                    if let Some(target) = scoped_lambda_bindings.get(alias).cloned()
                                    {
                                        scoped_lambda_bindings.insert(name.clone(), target);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if can_elide_lambda_value {
                        continue;
                    }
                    if kw == "let"
                        && val_node.and_then(scalar_vector_literal_len) == Some(0)
                        && i + 2 < items.len()
                    {
                        if let (
                            Some(Expression::Apply(next_mut_items)),
                            Some(fill_node),
                            Some(local_idx),
                        ) = (items.get(i + 1), child_at(i + 2), ctx.locals.get(name))
                        {
                            if let [Expression::Word(next_kw), Expression::Word(idx_name), Expression::Int(start)] =
                                &next_mut_items[..]
                            {
                                if next_kw == "mut" && *start >= 0 {
                                    let mut fill_nonnegative =
                                        scoped_nonnegative_int_locals.clone();
                                    fill_nonnegative.insert(idx_name.clone());
                                    let mut fill_exact = scoped_exact_int_locals.clone();
                                    fill_exact.insert(idx_name.clone(), *start);
                                    let fill_ctx = Ctx {
                                        fn_sigs: ctx.fn_sigs,
                                        fn_ids: ctx.fn_ids,
                                        extern_names: ctx.extern_names,
                                        lambda_ids: ctx.lambda_ids,
                                        closure_defs: ctx.closure_defs,
                                        lambda_bindings: &scoped_lambda_bindings,
                                        current_function: ctx.current_function,
                                        locals: ctx.locals.clone(),
                                        local_types: ctx.local_types.clone(),
                                        materialized_scalar_local_slots:
                                            scoped_materialized_scalar_local_slots.clone(),
                                        hoisted_scalar_vec_data_slots: ctx
                                            .hoisted_scalar_vec_data_slots
                                            .clone(),
                                        proven_scalar_vec_min_lengths:
                                            scoped_proven_scalar_vec_min_lengths.clone(),
                                        definitely_materialized_top_level_scalar_names: ctx
                                            .definitely_materialized_top_level_scalar_names,
                                        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                                        nonnegative_int_locals: &fill_nonnegative,
                                        tmp_i32: ctx.tmp_i32,
                                    };
                                    if let Some((slot, len, value, idx_name)) =
                                        append_fill_loop_const_i32(
                                            fill_node,
                                            &fill_ctx,
                                            &fill_exact,
                                        )
                                    {
                                        if slot == *local_idx {
                                            parts.push(format!(
                                                "i32.const {len}\n\
                                                 i32.const {value}\n\
                                                 call $vec_new_filled_i32\n\
                                                 local.set {local_idx}"
                                            ));
                                            scoped_materialized_scalar_local_slots
                                                .insert(*local_idx);
                                            scoped_proven_scalar_vec_min_lengths
                                                .insert(*local_idx, len);
                                            if let Some(idx_slot) = ctx.locals.get(&idx_name) {
                                                let final_idx = start.saturating_add(len);
                                                parts.push(format!(
                                                    "i32.const {final_idx}\nlocal.set {idx_slot}"
                                                ));
                                                scoped_nonnegative_int_locals
                                                    .insert(idx_name.clone());
                                                scoped_exact_int_locals.insert(idx_name, final_idx);
                                            }
                                            skip_until = i + 3;
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let value = val_node
                        .ok_or_else(|| format!("Missing let value for {}", name))
                        .and_then(|n| {
                            let scoped_ctx = Ctx {
                                fn_sigs: ctx.fn_sigs,
                                fn_ids: ctx.fn_ids,
                                extern_names: ctx.extern_names,
                                lambda_ids: ctx.lambda_ids,
                                closure_defs: ctx.closure_defs,
                                lambda_bindings: &scoped_lambda_bindings,
                                current_function: ctx.current_function,
                                locals: ctx.locals.clone(),
                                local_types: ctx.local_types.clone(),
                                materialized_scalar_local_slots:
                                    scoped_materialized_scalar_local_slots.clone(),
                                hoisted_scalar_vec_data_slots: ctx
                                    .hoisted_scalar_vec_data_slots
                                    .clone(),
                                proven_scalar_vec_min_lengths: scoped_proven_scalar_vec_min_lengths
                                    .clone(),
                                definitely_materialized_top_level_scalar_names: ctx
                                    .definitely_materialized_top_level_scalar_names,
                                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                                nonnegative_int_locals: &scoped_nonnegative_int_locals,
                                tmp_i32: ctx.tmp_i32,
                            };
                            compile_expr(n, &scoped_ctx)
                        })?;
                    if let Some(local_idx) = ctx.locals.get(name) {
                        let managed_local = ctx
                            .local_types
                            .get(name)
                            .map(is_managed_local_type)
                            .unwrap_or(false);
                        let borrowed_rhs = val_node
                            .map(|n| is_borrowed_managed_rhs_expr(n, &scoped_lambda_bindings))
                            .unwrap_or(false);
                        let move_source = managed_local
                            .then(|| {
                                direct_last_use_managed_move_source(
                                    val_node,
                                    name,
                                    &managed_do_locals,
                                    &items[i + 1..],
                                    &scoped_lambda_bindings,
                                )
                            })
                            .flatten();
                        let value = if managed_local
                            && borrowed_rhs
                            && move_source.is_none()
                            && !is_borrowed_projection_local(name, ctx)
                        {
                            let tmp_owned = ctx.tmp_i32 + 2;
                            let retain = ctx
                                .local_types
                                .get(name)
                                .map(rc_retain_for_type)
                                .unwrap_or("$rc_retain");
                            format!(
                                "{value}\nlocal.tee {}\ncall {retain}\ndrop\nlocal.get {}",
                                tmp_owned, tmp_owned,
                            )
                        } else {
                            value
                        };
                        parts.push(format!("{value}\nlocal.set {}", local_idx));
                        if let Some(source) = move_source {
                            parts.push(format!("i32.const 0\nlocal.set {}", source.slot));
                        }
                        if val_node
                            .map(|n| {
                                expr_is_definitely_materialized_scalar_vector(
                                    n,
                                    &scoped_materialized_scalar_local_slots,
                                    &ctx.locals,
                                    ctx.definitely_materialized_top_level_scalar_names,
                                )
                            })
                            .unwrap_or(false)
                        {
                            scoped_materialized_scalar_local_slots.insert(*local_idx);
                        }
                        if let Some(len) = val_node.and_then(scalar_vector_literal_len) {
                            scoped_proven_scalar_vec_min_lengths.insert(*local_idx, len);
                        }
                        if let Some(cap_idx) = self_capture_idx {
                            parts.push(format!(
                                "local.get {}\ni32.const {}\nlocal.get {}\ncall $closure_set\ndrop",
                                local_idx, cap_idx, local_idx
                            ));
                        }
                        if val_node
                            .map(|n| {
                                typed_expr_is_nonnegative_int(n, &scoped_nonnegative_int_locals)
                            })
                            .unwrap_or(false)
                        {
                            scoped_nonnegative_int_locals.insert(name.clone());
                        } else if kw == "mut" {
                            scoped_nonnegative_int_locals.remove(name);
                        }
                        if let Some(Expression::Int(value)) = val_node.map(|n| &n.expr) {
                            scoped_exact_int_locals.insert(name.clone(), *value);
                        } else {
                            scoped_exact_int_locals.remove(name);
                        }
                    } else {
                        return Err(format!("Unknown local '{}'", name));
                    }
                    if let Some(value_node) = val_node {
                        update_scalar_vec_min_lengths_after_expr(
                            value_node,
                            ctx,
                            &mut scoped_proven_scalar_vec_min_lengths,
                        );
                    }
                    append_last_use_releases_for_do_expr(
                        &mut parts,
                        &managed_do_locals,
                        &items[i],
                        &items[i + 1..],
                        &scoped_lambda_bindings,
                        direct_last_use_managed_move_source(
                            val_node,
                            name,
                            &managed_do_locals,
                            &items[i + 1..],
                            &scoped_lambda_bindings,
                        )
                        .map(|reference| reference.slot),
                    );
                    continue;
                }
            }
        }
        if let Some(n) = child_at(i) {
            if let Some((code, moved_from_slot)) = compile_last_use_managed_alter_move(
                n,
                &items[i + 1..],
                &managed_do_locals,
                &scoped_lambda_bindings,
                ctx,
            ) {
                parts.push(code);
                update_scalar_vec_min_lengths_after_expr(
                    n,
                    ctx,
                    &mut scoped_proven_scalar_vec_min_lengths,
                );
                collect_altered_int_locals(&n.expr, &mut scoped_exact_int_locals);
                append_last_use_releases_for_do_expr(
                    &mut parts,
                    &managed_do_locals,
                    &items[i],
                    &items[i + 1..],
                    &scoped_lambda_bindings,
                    Some(moved_from_slot),
                );
                continue;
            }
            let scoped_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                extern_names: ctx.extern_names,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: &scoped_lambda_bindings,
                current_function: ctx.current_function,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                materialized_scalar_local_slots: scoped_materialized_scalar_local_slots.clone(),
                hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                proven_scalar_vec_min_lengths: scoped_proven_scalar_vec_min_lengths.clone(),
                definitely_materialized_top_level_scalar_names: ctx
                    .definitely_materialized_top_level_scalar_names,
                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                nonnegative_int_locals: &scoped_nonnegative_int_locals,
                tmp_i32: ctx.tmp_i32,
            };
            let managed = n.typ.as_ref().map(is_managed_local_type).unwrap_or(false);
            let borrowed = if managed {
                is_borrowed_managed_rhs_expr(n, &scoped_lambda_bindings)
            } else {
                false
            };
            if managed && !borrowed {
                let c = compile_expr(n, &scoped_ctx)?;
                let release = n
                    .typ
                    .as_ref()
                    .map(rc_release_for_type)
                    .unwrap_or("$rc_release");
                let tmp_val = ctx.tmp_i32;
                let tmp_keep = ctx.tmp_i32 + 1;
                let mut blk = Vec::new();
                if managed_local_slots.is_empty() {
                    blk.push(format!("{c}\ncall {release}\ndrop"));
                } else {
                    blk.push(format!("{c}\nlocal.set {}", tmp_val));
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
                    blk.push(format!(
                        "local.get {}\ni32.eqz\nif\n  local.get {}\n  call {release}\n  drop\nend",
                        tmp_keep, tmp_val
                    ));
                }
                parts.push(blk.join("\n"));
            } else {
                let c = compile_expr_discarding_result(n, &scoped_ctx)?;
                if !c.is_empty() {
                    parts.push(c);
                }
            }
            if let Some(slot) = scalar_vector_set_target_slot(&n.expr, ctx) {
                scoped_materialized_scalar_local_slots.insert(slot);
            }
            update_scalar_vec_min_lengths_after_expr(
                n,
                ctx,
                &mut scoped_proven_scalar_vec_min_lengths,
            );
            if let Some((slot, added_len)) =
                append_fill_loop_min_length(n, &scoped_ctx, &scoped_exact_int_locals)
            {
                scoped_proven_scalar_vec_min_lengths
                    .entry(slot)
                    .and_modify(|len| *len = len.saturating_add(added_len))
                    .or_insert(added_len);
            }
            collect_altered_int_locals(&n.expr, &mut scoped_exact_int_locals);
        }
        append_last_use_releases_for_do_expr(
            &mut parts,
            &managed_do_locals,
            &items[i],
            &items[i + 1..],
            &scoped_lambda_bindings,
            None,
        );
    }
    let last_node =
        child_at(items.len() - 1).ok_or_else(|| "Missing final do expression".to_string())?;
    let scoped_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: &scoped_lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: scoped_materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: scoped_proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: &scoped_nonnegative_int_locals,
        tmp_i32: ctx.tmp_i32,
    };
    let Some(last) = compile_tail_expr(
        last_node,
        &scoped_ctx,
        self_name,
        arity,
        releasable_ref_slots,
    )?
    else {
        return Ok(None);
    };
    parts.push(last);
    Ok(Some(parts.join("\n")))
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
    if elem_ref_flag == 0 && !args.is_empty() {
        return compile_scalar_vector_literal_direct(args, elem_kind, ctx);
    }
    if elem_ref_flag == 1 && !args.is_empty() {
        return compile_managed_vector_literal_direct(args, ctx);
    }
    let mut out = Vec::new();
    let push_op = vec_push_runtime_for_elem_ref(elem_ref_flag);
    out.push(format!(
        "i32.const {}\ni32.const {}\ncall $vec_new_{}\nlocal.set {}",
        0,
        elem_ref_flag,
        elem_kind.suffix(),
        ctx.tmp_i32
    ));
    for a in args {
        let nested_ctx = Ctx {
            fn_sigs: ctx.fn_sigs,
            fn_ids: ctx.fn_ids,
            extern_names: ctx.extern_names,
            lambda_ids: ctx.lambda_ids,
            closure_defs: ctx.closure_defs,
            lambda_bindings: ctx.lambda_bindings,
            current_function: ctx.current_function,
            locals: ctx.locals.clone(),
            local_types: ctx.local_types.clone(),
            materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
            hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
            proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
            definitely_materialized_top_level_scalar_names: ctx
                .definitely_materialized_top_level_scalar_names,
            proven_scalar_index_loads: ctx.proven_scalar_index_loads,
            nonnegative_int_locals: ctx.nonnegative_int_locals,
            tmp_i32: ctx.tmp_i32 + 1,
        };
        let v = compile_expr(a, &nested_ctx)?;
        let release_arg = should_release_set_rhs(a, ctx.lambda_bindings);
        if release_arg {
            // Fresh managed values are retained by vector push; release the temporary owner.
            out.push(format!(
                "local.get {}\n{}\nlocal.tee {}\ncall {}\ndrop\nlocal.get {}\ncall {}\ndrop",
                ctx.tmp_i32,
                v,
                ctx.tmp_i32 + 1,
                push_op,
                ctx.tmp_i32 + 1,
                rc_release_for_opt_type(a.typ.as_ref())
            ));
        } else {
            out.push(format!(
                "local.get {}\n{}\ncall {}\ndrop",
                ctx.tmp_i32, v, push_op
            ));
        }
    }
    out.push(format!("local.get {}", ctx.tmp_i32));
    Ok(out.join("\n"))
}

fn compile_managed_vector_literal_direct(
    args: &[TypedExpression],
    ctx: &Ctx<'_>,
) -> Result<String, String> {
    let vec_tmp = ctx.tmp_i32;
    let val_tmp = ctx.tmp_i32 + 1;
    let data_tmp = ctx.tmp_i32 + 2;
    let mut out = Vec::new();
    out.push(format!(
        "i32.const {}\n\
         call $vec_new_uninit_i32\n\
         local.set {vec_tmp}\n\
         local.get {vec_tmp}\n\
         i32.const 12\n\
         i32.add\n\
         i32.const 1\n\
         i32.store\n\
         local.get {vec_tmp}\n\
         i32.const 16\n\
         i32.add\n\
         i32.load\n\
         local.set {data_tmp}",
        args.len()
    ));

    for (idx, arg) in args.iter().enumerate() {
        let nested_ctx = Ctx {
            fn_sigs: ctx.fn_sigs,
            fn_ids: ctx.fn_ids,
            extern_names: ctx.extern_names,
            lambda_ids: ctx.lambda_ids,
            closure_defs: ctx.closure_defs,
            lambda_bindings: ctx.lambda_bindings,
            current_function: ctx.current_function,
            locals: ctx.locals.clone(),
            local_types: ctx.local_types.clone(),
            materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
            hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
            proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
            definitely_materialized_top_level_scalar_names: ctx
                .definitely_materialized_top_level_scalar_names,
            proven_scalar_index_loads: ctx.proven_scalar_index_loads,
            nonnegative_int_locals: ctx.nonnegative_int_locals,
            tmp_i32: ctx.tmp_i32 + 3,
        };
        let value = compile_expr(arg, &nested_ctx)?;
        out.push(format!("{value}\nlocal.set {val_tmp}"));

        // A fresh managed result already owns one reference, so storing it in
        // the new vector transfers that ownership directly. Borrowed values
        // need one retain because the vector becomes an additional owner.
        if !should_release_set_rhs(arg, ctx.lambda_bindings) {
            let retain = arg
                .typ
                .as_ref()
                .map(rc_retain_for_type)
                .unwrap_or("$rc_retain");
            out.push(format!("local.get {val_tmp}\ncall {retain}\ndrop"));
        }

        let offset = idx.saturating_mul(4);
        let offset_code = if offset == 0 {
            String::new()
        } else {
            format!("\ni32.const {offset}\ni32.add")
        };
        out.push(format!(
            "local.get {data_tmp}{offset_code}\nlocal.get {val_tmp}\ni32.store"
        ));
    }

    out.push(format!("local.get {vec_tmp}"));
    Ok(out.join("\n"))
}

fn compile_scalar_vector_literal_direct(
    args: &[TypedExpression],
    _elem_kind: VecElemKind,
    ctx: &Ctx<'_>,
) -> Result<String, String> {
    let vec_tmp = ctx.tmp_i32;
    let val_tmp = ctx.tmp_i32 + 1;
    let data_tmp = ctx.tmp_i32 + 2;
    let mut out = Vec::new();
    out.push(format!(
        "i32.const {}\n\
         call $vec_new_uninit_i32\n\
         local.set {}\n\
         local.get {}\n\
         i32.const 16\n\
         i32.add\n\
         i32.load\n\
         local.set {}",
        args.len(),
        vec_tmp,
        vec_tmp,
        data_tmp
    ));
    if let Some(source_slot) = contiguous_scalar_clone_source_slot(args, ctx) {
        out.push(format!(
            "local.get {data_tmp}\n\
             local.get {source_slot}\n\
             i32.const 16\n\
             i32.add\n\
             i32.load\n\
             i32.const {}\n\
             memory.copy",
            args.len().saturating_mul(4)
        ));
        out.push(format!("local.get {vec_tmp}"));
        return Ok(out.join("\n"));
    }
    for (idx, arg) in args.iter().enumerate() {
        let nested_ctx = Ctx {
            fn_sigs: ctx.fn_sigs,
            fn_ids: ctx.fn_ids,
            extern_names: ctx.extern_names,
            lambda_ids: ctx.lambda_ids,
            closure_defs: ctx.closure_defs,
            lambda_bindings: ctx.lambda_bindings,
            current_function: ctx.current_function,
            locals: ctx.locals.clone(),
            local_types: ctx.local_types.clone(),
            materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
            hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
            proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
            definitely_materialized_top_level_scalar_names: ctx
                .definitely_materialized_top_level_scalar_names,
            proven_scalar_index_loads: ctx.proven_scalar_index_loads,
            nonnegative_int_locals: ctx.nonnegative_int_locals,
            tmp_i32: ctx.tmp_i32 + 3,
        };
        let value = compile_expr(arg, &nested_ctx)?;
        let offset = idx.saturating_mul(4);
        let offset_code = if offset == 0 {
            String::new()
        } else {
            format!("\n             i32.const {offset}\n             i32.add")
        };
        out.push(format!(
            "{value}\nlocal.set {val_tmp}\n\
             local.get {data_tmp}{offset_code}\n\
             local.get {val_tmp}\n\
             i32.store"
        ));
    }
    out.push(format!("local.get {vec_tmp}"));
    Ok(out.join("\n"))
}

fn contiguous_scalar_clone_source_slot(args: &[TypedExpression], ctx: &Ctx<'_>) -> Option<usize> {
    if parse_env_bool_like("QUE_BOUNDS_CHECK", true) || args.is_empty() {
        return None;
    }
    let mut source_name: Option<&str> = None;
    for (index, arg) in args.iter().enumerate() {
        let Expression::Apply(items) = &arg.expr else {
            return None;
        };
        let [Expression::Word(op), Expression::Word(name), Expression::Int(source_index)] =
            &items[..]
        else {
            return None;
        };
        if op != "get" || usize::try_from(*source_index).ok()? != index {
            return None;
        }
        if source_name.is_some_and(|source| source != name) {
            return None;
        }
        source_name = Some(name);
    }
    let source_slot = *ctx.locals.get(source_name?)?;
    let required_len = i32::try_from(args.len()).ok()?;
    (ctx.proven_scalar_vec_min_lengths
        .get(&source_slot)
        .copied()
        .unwrap_or(0)
        >= required_len)
        .then_some(source_slot)
}

fn compile_trusted_string_literal_expr(expr: &Expression, ctx: &Ctx<'_>) -> Result<String, String> {
    let Expression::Apply(items) = expr else {
        return Err("strings expects string literal elements".to_string());
    };
    let Some(Expression::Word(head)) = items.first() else {
        return Err("strings expects string literal elements".to_string());
    };
    if head != "string" {
        return Err("strings expects string literal elements".to_string());
    }

    let mut out = Vec::new();
    out.push(format!(
        "i32.const 0\ni32.const 0\ncall $vec_new_i32\nlocal.set {}",
        ctx.tmp_i32
    ));
    for item in &items[1..] {
        let nested_ctx = Ctx {
            fn_sigs: ctx.fn_sigs,
            fn_ids: ctx.fn_ids,
            extern_names: ctx.extern_names,
            lambda_ids: ctx.lambda_ids,
            closure_defs: ctx.closure_defs,
            lambda_bindings: ctx.lambda_bindings,
            current_function: ctx.current_function,
            locals: ctx.locals.clone(),
            local_types: ctx.local_types.clone(),
            materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
            hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
            proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
            definitely_materialized_top_level_scalar_names: ctx
                .definitely_materialized_top_level_scalar_names,
            proven_scalar_index_loads: ctx.proven_scalar_index_loads,
            nonnegative_int_locals: ctx.nonnegative_int_locals,
            tmp_i32: ctx.tmp_i32 + 1,
        };
        let v = match item {
            Expression::Int(n) => format!("i32.const {}", n),
            Expression::Word(w) if w == "true" => "i32.const 1".to_string(),
            Expression::Word(w) if w == "false" || w == "nil" => "i32.const 0".to_string(),
            Expression::Dec(n) => format!("i32.const {}", decimal_literal_i32(n)?),
            Expression::Apply(_) => {
                let fake_node = TypedExpression {
                    expr: item.clone(),
                    typ: None,
                    effect: EffectFlags::PURE,
                    children: Vec::new(),
                };
                compile_expr(&fake_node, &nested_ctx)?
            }
            _ => {
                return Err("strings expects string literal elements".to_string());
            }
        };
        out.push(format!(
            "local.get {}\n{}\ncall $vec_push_scalar_i32\ndrop",
            ctx.tmp_i32, v
        ));
    }
    out.push(format!("local.get {}", ctx.tmp_i32));
    Ok(out.join("\n"))
}

fn compile_trusted_typed_vector_literal(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>,
) -> Result<String, String> {
    let items = match &node.expr {
        Expression::Apply(items) => items,
        _ => {
            return Err(format!("{} literal expected apply expression", op));
        }
    };
    let (elem_ref_flag, compile_item): (i32, fn(&Expression, &Ctx<'_>) -> Result<String, String>) =
        match op {
            "integers" => (0, |expr, _ctx| match expr {
                Expression::Int(n) => Ok(format!("i32.const {}", n)),
                _ => Err("integers expects integer literal elements".to_string()),
            }),
            "bools" => (0, |expr, _ctx| match expr {
                Expression::Word(w) if w == "true" => Ok("i32.const 1".to_string()),
                Expression::Word(w) if w == "false" => Ok("i32.const 0".to_string()),
                _ => Err("bools expects boolean literal elements".to_string()),
            }),
            "decimals" => (0, |expr, _ctx| match expr {
                Expression::Dec(n) => Ok(format!("i32.const {}", decimal_literal_i32(n)?)),
                _ => Err("decimals expects decimal literal elements".to_string()),
            }),
            "strings" => (1, compile_trusted_string_literal_expr),
            _ => {
                return Err(format!("Unsupported trusted typed vector literal '{}'", op));
            }
        };

    let mut out = Vec::new();
    let push_op = vec_push_runtime_for_elem_ref(elem_ref_flag);
    out.push(format!(
        "i32.const 0\ni32.const {}\ncall $vec_new_i32\nlocal.set {}",
        elem_ref_flag, ctx.tmp_i32
    ));
    for item in &items[1..] {
        let nested_ctx = Ctx {
            fn_sigs: ctx.fn_sigs,
            fn_ids: ctx.fn_ids,
            extern_names: ctx.extern_names,
            lambda_ids: ctx.lambda_ids,
            closure_defs: ctx.closure_defs,
            lambda_bindings: ctx.lambda_bindings,
            current_function: ctx.current_function,
            locals: ctx.locals.clone(),
            local_types: ctx.local_types.clone(),
            materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
            hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
            proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
            definitely_materialized_top_level_scalar_names: ctx
                .definitely_materialized_top_level_scalar_names,
            proven_scalar_index_loads: ctx.proven_scalar_index_loads,
            nonnegative_int_locals: ctx.nonnegative_int_locals,
            tmp_i32: ctx.tmp_i32 + 1,
        };
        let v = compile_item(item, &nested_ctx)?;
        let fake_node = TypedExpression {
            expr: item.clone(),
            typ: node.typ.as_ref().and_then(|t| match t {
                Type::List(inner) => Some((**inner).clone()),
                _ => None,
            }),
            effect: EffectFlags::PURE,
            children: Vec::new(),
        };
        if should_release_set_rhs(&fake_node, ctx.lambda_bindings) {
            out.push(format!(
                "local.get {}\n{}\nlocal.tee {}\ncall {}\ndrop\n{}",
                ctx.tmp_i32,
                v,
                ctx.tmp_i32 + 1,
                push_op,
                emit_release_fresh_owned_temp(ctx.tmp_i32 + 1, fake_node.typ.as_ref())
            ));
        } else {
            out.push(format!(
                "local.get {}\n{}\ncall {}\ndrop",
                ctx.tmp_i32, v, push_op
            ));
        }
    }
    out.push(format!("local.get {}", ctx.tmp_i32));
    Ok(out.join("\n"))
}

fn compile_tuple(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: ctx.tmp_i32 + 3,
    };
    let a_node = node
        .children
        .get(1)
        .ok_or_else(|| "tuple missing first element".to_string())?;
    let b_node = node
        .children
        .get(2)
        .ok_or_else(|| "tuple missing second element".to_string())?;
    let a = compile_expr(a_node, &nested_ctx)?;
    let b = compile_expr(b_node, &nested_ctx)?;
    let release_a = should_release_set_rhs(a_node, ctx.lambda_bindings);
    let release_b = should_release_set_rhs(b_node, ctx.lambda_bindings);
    let a_tmp = ctx.tmp_i32;
    let b_tmp = ctx.tmp_i32 + 1;
    let out_tmp = ctx.tmp_i32 + 2;
    let data_tmp = ctx.tmp_i32 + 3;
    let mut out = Vec::new();
    out.push(format!("{a}\nlocal.set {}", a_tmp));
    out.push(format!("{b}\nlocal.set {}", b_tmp));
    out.push(format!(
        "i32.const 2\n\
         call $vec_new_uninit_i32\n\
         local.set {out_tmp}\n\
         local.get {out_tmp}\n\
         i32.const 12\n\
         i32.add\n\
         i32.const 1\n\
         i32.store\n\
         local.get {out_tmp}\n\
         i32.const 16\n\
         i32.add\n\
         i32.load\n\
         local.set {data_tmp}"
    ));
    if a_node.typ.as_ref().is_some_and(is_managed_local_type) && !release_a {
        out.push(format!(
            "local.get {a_tmp}\ncall {}\ndrop",
            rc_retain_for_type(a_node.typ.as_ref().expect("managed tuple field type"))
        ));
    }
    if b_node.typ.as_ref().is_some_and(is_managed_local_type) && !release_b {
        out.push(format!(
            "local.get {b_tmp}\ncall {}\ndrop",
            rc_retain_for_type(b_node.typ.as_ref().expect("managed tuple field type"))
        ));
    }
    out.push(format!(
        "local.get {data_tmp}\n\
         local.get {a_tmp}\n\
         i32.store\n\
         local.get {data_tmp}\n\
         i32.const 4\n\
         i32.add\n\
         local.get {b_tmp}\n\
         i32.store\n\
         local.get {out_tmp}"
    ));
    Ok(out.join("\n"))
}

fn compile_fst(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    if let Some(tuple_node) = node.children.get(1) {
        if matches!(&tuple_node.expr, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "tuple"))
        {
            let a_node = tuple_node
                .children
                .get(1)
                .ok_or_else(|| "tuple missing first element".to_string())?;
            let b_node = tuple_node
                .children
                .get(2)
                .ok_or_else(|| "tuple missing second element".to_string())?;
            let nested_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                extern_names: ctx.extern_names,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: ctx.lambda_bindings,
                current_function: ctx.current_function,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
                hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
                definitely_materialized_top_level_scalar_names: ctx
                    .definitely_materialized_top_level_scalar_names,
                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                nonnegative_int_locals: ctx.nonnegative_int_locals,
                tmp_i32: ctx.tmp_i32 + 3,
            };
            let a = compile_expr(a_node, &nested_ctx)?;
            let b = compile_expr(b_node, &nested_ctx)?;
            let keep_a = a_node
                .typ
                .as_ref()
                .map(is_managed_local_type)
                .unwrap_or(false);
            let release_b = should_release_set_rhs(b_node, ctx.lambda_bindings);
            let a_tmp = ctx.tmp_i32;
            let b_tmp = ctx.tmp_i32 + 1;
            let mut out = Vec::new();
            out.push(format!("{a}\nlocal.set {}", a_tmp));
            if release_b {
                out.push(format!("{b}\nlocal.set {}", b_tmp));
                out.push(emit_release_fresh_owned_temp(b_tmp, b_node.typ.as_ref()));
            } else {
                out.push(format!("{b}\ndrop"));
            }
            if keep_a {
                out.push(format!("local.get {}", a_tmp));
            } else {
                out.push(format!("local.get {}", a_tmp));
            }
            return Ok(out.join("\n"));
        }
    }
    let p = compile_expr(
        node.children
            .get(1)
            .ok_or_else(|| "fst missing tuple arg".to_string())?,
        ctx,
    )?;
    if node
        .children
        .get(1)
        .and_then(|tuple| tuple.typ.as_ref())
        .is_some_and(|typ| matches!(typ, Type::Tuple(_)))
    {
        return Ok(format!(
            "{p}\n\
             i32.const 16\n\
             i32.add\n\
             i32.load\n\
             i32.load"
        ));
    }
    Ok(format!("{p}\ncall $tuple_fst"))
}

fn compile_snd(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    if let Some(tuple_node) = node.children.get(1) {
        if matches!(&tuple_node.expr, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "tuple"))
        {
            let a_node = tuple_node
                .children
                .get(1)
                .ok_or_else(|| "tuple missing first element".to_string())?;
            let b_node = tuple_node
                .children
                .get(2)
                .ok_or_else(|| "tuple missing second element".to_string())?;
            let nested_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                extern_names: ctx.extern_names,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: ctx.lambda_bindings,
                current_function: ctx.current_function,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
                hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
                definitely_materialized_top_level_scalar_names: ctx
                    .definitely_materialized_top_level_scalar_names,
                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                nonnegative_int_locals: ctx.nonnegative_int_locals,
                tmp_i32: ctx.tmp_i32 + 3,
            };
            let a = compile_expr(a_node, &nested_ctx)?;
            let b = compile_expr(b_node, &nested_ctx)?;
            let release_a = should_release_set_rhs(a_node, ctx.lambda_bindings);
            let a_tmp = ctx.tmp_i32;
            let mut out = Vec::new();
            if release_a {
                out.push(format!("{a}\nlocal.set {}", a_tmp));
                out.push(emit_release_fresh_owned_temp(a_tmp, a_node.typ.as_ref()));
            } else {
                out.push(format!("{a}\ndrop"));
            }
            out.push(b);
            return Ok(out.join("\n"));
        }
    }
    let p = compile_expr(
        node.children
            .get(1)
            .ok_or_else(|| "snd missing tuple arg".to_string())?,
        ctx,
    )?;
    if node
        .children
        .get(1)
        .and_then(|tuple| tuple.typ.as_ref())
        .is_some_and(|typ| matches!(typ, Type::Tuple(_)))
    {
        return Ok(format!(
            "{p}\n\
             i32.const 16\n\
             i32.add\n\
             i32.load\n\
             i32.const 4\n\
             i32.add\n\
             i32.load"
        ));
    }
    Ok(format!("{p}\ncall $tuple_snd"))
}

fn typed_expr_is_nonnegative_int(
    node: &TypedExpression,
    nonnegative_locals: &HashSet<String>,
) -> bool {
    if !matches!(node.typ.as_ref(), Some(Type::Int)) {
        return false;
    }
    match &node.expr {
        Expression::Int(n) => *n >= 0,
        Expression::Word(name) => nonnegative_locals.contains(name),
        Expression::Apply(items) => {
            let Some(Expression::Word(op)) = items.first() else {
                return false;
            };
            match op.as_str() {
                "+" | "*" => node
                    .children
                    .iter()
                    .skip(1)
                    .all(|child| typed_expr_is_nonnegative_int(child, nonnegative_locals)),
                "/" => {
                    node.children
                        .get(1)
                        .map(|child| typed_expr_is_nonnegative_int(child, nonnegative_locals))
                        .unwrap_or(false)
                        && node
                            .children
                            .get(2)
                            .map(|child| typed_expr_is_nonnegative_int(child, nonnegative_locals))
                            .unwrap_or(false)
                }
                "if" | "cond" => node
                    .children
                    .iter()
                    .skip(2)
                    .all(|child| typed_expr_is_nonnegative_int(child, nonnegative_locals)),
                _ => false,
            }
        }
        _ => false,
    }
}

fn scalar_get_is_proven_in_bounds(node: &TypedExpression, ctx: &Ctx<'_>) -> Option<(usize, usize)> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    let [Expression::Word(op), Expression::Word(xs), Expression::Word(idx)] = &items[..] else {
        return None;
    };
    if op != "get" {
        return None;
    }
    if !ctx
        .proven_scalar_index_loads
        .contains(&(xs.clone(), idx.clone()))
    {
        return None;
    }
    let xs_slot = *ctx.locals.get(xs)?;
    let idx_slot = *ctx.locals.get(idx)?;
    Some((xs_slot, idx_slot))
}

fn hoisted_scalar_vec_data_slot(node: &TypedExpression, ctx: &Ctx<'_>) -> Option<usize> {
    let Expression::Word(name) = &node.expr else {
        return None;
    };
    let slot = *ctx.locals.get(name)?;
    ctx.hoisted_scalar_vec_data_slots.get(&slot).copied()
}

fn emit_unchecked_scalar_get_from_slots(xs_slot: usize, idx_slot: usize) -> String {
    format!(
        "local.get {xs_slot}\n\
         i32.const 16\n\
         i32.add\n\
         i32.load\n\
         local.get {idx_slot}\n\
         i32.const 4\n\
         i32.mul\n\
         i32.add\n\
         i32.load"
    )
}

fn emit_unchecked_scalar_get_from_data_slot(data_slot: usize, idx_slot: usize) -> String {
    format!(
        "local.get {data_slot}\n\
         local.get {idx_slot}\n\
         i32.const 4\n\
         i32.mul\n\
         i32.add\n\
         i32.load"
    )
}

fn emit_dynamic_scalar_get_from_data_slot(index: &str, data_slot: usize, idx_tmp: usize) -> String {
    format!(
        "{index}\n\
         local.set {idx_tmp}\n\
         local.get {data_slot}\n\
         local.get {idx_tmp}\n\
         i32.const 4\n\
         i32.mul\n\
         i32.add\n\
         i32.load"
    )
}

fn emit_constant_scalar_get_from_data_slot(data_slot: usize, index: i32) -> String {
    let offset = index.saturating_mul(4);
    let offset_code = if offset == 0 {
        String::new()
    } else {
        format!("\ni32.const {offset}\ni32.add")
    };
    format!("local.get {data_slot}{offset_code}\ni32.load")
}

fn emit_constant_scalar_set_from_data_slot(
    value: &str,
    data_slot: usize,
    index: i32,
    value_tmp: usize,
) -> String {
    let offset = index.saturating_mul(4);
    let offset_code = if offset == 0 {
        String::new()
    } else {
        format!("\ni32.const {offset}\ni32.add")
    };
    format!(
        "{value}\n\
         local.set {value_tmp}\n\
         local.get {data_slot}{offset_code}\n\
         local.get {value_tmp}\n\
         i32.store\n\
         i32.const 0"
    )
}

fn emit_constant_scalar_get(
    xs: &str,
    index: i32,
    tmp_ptr: usize,
    release_xs_after: bool,
    check_bounds: bool,
) -> String {
    let offset = index.saturating_mul(4);
    let upper_check = if check_bounds {
        if index < 0 {
            "unreachable".to_string()
        } else {
            format!(
                "local.get {tmp_ptr}\n\
                 i32.load\n\
                 i32.const {index}\n\
                 i32.le_s\n\
                 if\n\
                   unreachable\n\
                 end"
            )
        }
    } else {
        String::new()
    };
    let offset_code = if offset == 0 {
        String::new()
    } else {
        format!("\ni32.const {offset}\ni32.add")
    };
    let load = format!(
        "{xs}\n\
         local.set {tmp_ptr}\n\
         {upper_check}\n\
         local.get {tmp_ptr}\n\
         i32.const 16\n\
         i32.add\n\
         i32.load{offset_code}\n\
         i32.load"
    );
    if release_xs_after {
        format!("{load}\nlocal.get {tmp_ptr}\ncall $rc_release_vec\ndrop")
    } else {
        load
    }
}

fn emit_constant_scalar_set(
    target_prefix: &str,
    value: &str,
    index: i32,
    target_tmp: usize,
    value_tmp: usize,
    release_target_code: &str,
    target_already_materialized: bool,
) -> String {
    let offset = index.saturating_mul(4);
    let offset_code = if offset == 0 {
        String::new()
    } else {
        format!("\ni32.const {offset}\ni32.add")
    };
    let replacement = format!(
        "local.get {target_tmp}\n\
         i32.const 16\n\
         i32.add\n\
         i32.load{offset_code}\n\
         local.get {value_tmp}\n\
         i32.store\n\
         i32.const 0"
    );
    let fallback_op = if target_already_materialized {
        "$vec_set_scalar_materialized_i32"
    } else {
        "$vec_set_scalar_i32"
    };
    let fallback = format!(
        "local.get {target_tmp}\n\
         i32.const {index}\n\
         local.get {value_tmp}\n\
         call {fallback_op}"
    );
    let body = if index < 0 {
        format!("unreachable\n{fallback}")
    } else {
        let materialized_guard = if target_already_materialized {
            String::new()
        } else {
            format!(
                "local.get {target_tmp}\n\
                 i32.const 20\n\
                 i32.add\n\
                 i32.load\n\
                 i32.const 1447380017\n\
                 i32.eq\n"
            )
        };
        let combine_guard = if target_already_materialized {
            String::new()
        } else {
            "i32.and\n".to_string()
        };
        format!(
            "{materialized_guard}local.get {target_tmp}\n\
             i32.load\n\
             i32.const {index}\n\
             i32.gt_s\n\
             {combine_guard}\
             if (result i32)\n\
               {replacement}\n\
             else\n\
               {fallback}\n\
             end"
        )
    };
    let mut out = format!(
        "{target_prefix}\n\
         local.set {target_tmp}\n\
         {value}\n\
         local.set {value_tmp}\n\
         {body}"
    );
    if !release_target_code.is_empty() {
        out.push('\n');
        out.push_str(release_target_code);
    }
    out
}

fn emit_constant_scalar_set_unchecked_replacement(
    target_prefix: &str,
    value: &str,
    index: i32,
    target_tmp: usize,
    value_tmp: usize,
    release_target_code: &str,
    target_already_materialized: bool,
) -> String {
    let offset = index.saturating_mul(4);
    let offset_code = if offset == 0 {
        String::new()
    } else {
        format!("\ni32.const {offset}\ni32.add")
    };
    let body = if target_already_materialized {
        format!(
            "local.get {target_tmp}\n\
             i32.const 16\n\
             i32.add\n\
             i32.load{offset_code}\n\
             local.get {value_tmp}\n\
             i32.store\n\
             i32.const 0"
        )
    } else {
        format!(
            "local.get {target_tmp}\n\
             i32.const 20\n\
             i32.add\n\
             i32.load\n\
             i32.const 1447380017\n\
             i32.eq\n\
             if (result i32)\n\
               local.get {target_tmp}\n\
               i32.const 16\n\
               i32.add\n\
               i32.load{offset_code}\n\
               local.get {value_tmp}\n\
               i32.store\n\
               i32.const 0\n\
             else\n\
               local.get {target_tmp}\n\
               i32.const {index}\n\
               local.get {value_tmp}\n\
               call $vec_set_scalar_i32\n\
             end"
        )
    };
    let mut out = format!(
        "{target_prefix}\n\
         local.set {target_tmp}\n\
         {value}\n\
         local.set {value_tmp}\n\
         {body}"
    );
    if !release_target_code.is_empty() {
        out.push('\n');
        out.push_str(release_target_code);
    }
    out
}

fn emit_dynamic_scalar_set(
    target_prefix: &str,
    index: &str,
    value: &str,
    target_tmp: usize,
    index_tmp: usize,
    value_tmp: usize,
    release_target_code: &str,
    target_already_materialized: bool,
) -> String {
    let replacement = format!(
        "local.get {target_tmp}\n\
         i32.const 16\n\
         i32.add\n\
         i32.load\n\
         local.get {index_tmp}\n\
         i32.const 4\n\
         i32.mul\n\
         i32.add\n\
         local.get {value_tmp}\n\
         i32.store\n\
         i32.const 0"
    );
    let fallback_op = if target_already_materialized {
        "$vec_set_scalar_materialized_i32"
    } else {
        "$vec_set_scalar_i32"
    };
    let fallback = format!(
        "local.get {target_tmp}\n\
         local.get {index_tmp}\n\
         local.get {value_tmp}\n\
         call {fallback_op}"
    );
    let materialized_guard = if target_already_materialized {
        String::new()
    } else {
        format!(
            "local.get {target_tmp}\n\
             i32.const 20\n\
             i32.add\n\
             i32.load\n\
             i32.const 1447380017\n\
             i32.eq\n"
        )
    };
    let combine_guard = if target_already_materialized {
        String::new()
    } else {
        "i32.and\n".to_string()
    };
    let body = if parse_env_bool_like("QUE_BOUNDS_CHECK", true) {
        format!(
            "{materialized_guard}local.get {index_tmp}\n\
             i32.const 0\n\
             i32.ge_s\n\
             local.get {index_tmp}\n\
             local.get {target_tmp}\n\
             i32.load\n\
             i32.lt_s\n\
             i32.and\n\
             {combine_guard}\
             if (result i32)\n\
               {replacement}\n\
             else\n\
               {fallback}\n\
             end"
        )
    } else {
        // set! also supports appending at idx == length. Keep that case on the
        // runtime helper, but let trusted replacement stores go straight to memory.
        format!(
            "{materialized_guard}local.get {index_tmp}\n\
             local.get {target_tmp}\n\
             i32.load\n\
             i32.ne\n\
             {combine_guard}\
             if (result i32)\n\
               {replacement}\n\
             else\n\
               {fallback}\n\
             end"
        )
    };
    let mut out = format!(
        "{target_prefix}\n\
         local.set {target_tmp}\n\
         {index}\n\
         local.set {index_tmp}\n\
         {value}\n\
         local.set {value_tmp}\n\
         {body}"
    );
    if !release_target_code.is_empty() {
        out.push('\n');
        out.push_str(release_target_code);
    }
    out
}

fn emit_dynamic_scalar_set_proven_replacement(
    target_prefix: &str,
    index: &str,
    value: &str,
    target_tmp: usize,
    index_tmp: usize,
    value_tmp: usize,
    release_target_code: &str,
    target_already_materialized: bool,
) -> String {
    let replacement = format!(
        "local.get {target_tmp}\n\
         i32.const 16\n\
         i32.add\n\
         i32.load\n\
         local.get {index_tmp}\n\
         i32.const 4\n\
         i32.mul\n\
         i32.add\n\
         local.get {value_tmp}\n\
         i32.store\n\
         i32.const 0"
    );
    let body = if target_already_materialized {
        replacement
    } else {
        format!(
            "local.get {target_tmp}\n\
             i32.const 20\n\
             i32.add\n\
             i32.load\n\
             i32.const 1447380017\n\
             i32.eq\n\
             if (result i32)\n\
               {replacement}\n\
             else\n\
               local.get {target_tmp}\n\
               local.get {index_tmp}\n\
               local.get {value_tmp}\n\
               call $vec_set_scalar_i32\n\
             end"
        )
    };
    let mut out = format!(
        "{target_prefix}\n\
         local.set {target_tmp}\n\
         {index}\n\
         local.set {index_tmp}\n\
         {value}\n\
         local.set {value_tmp}\n\
         {body}"
    );
    if !release_target_code.is_empty() {
        out.push('\n');
        out.push_str(release_target_code);
    }
    out
}

fn emit_dynamic_scalar_set_from_data_slot(
    index: &str,
    value: &str,
    data_slot: usize,
    index_tmp: usize,
    value_tmp: usize,
) -> String {
    format!(
        "{index}\n\
         local.set {index_tmp}\n\
         {value}\n\
         local.set {value_tmp}\n\
         local.get {data_slot}\n\
         local.get {index_tmp}\n\
         i32.const 4\n\
         i32.mul\n\
         i32.add\n\
         local.get {value_tmp}\n\
         i32.store\n\
         i32.const 0"
    )
}

fn compile_get(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs_node = node
        .children
        .get(1)
        .ok_or_else(|| "get missing vector".to_string())?;
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: ctx.tmp_i32 + 3,
    };
    let (xs, release_xs_after) = match &xs_node.expr {
        Expression::Word(name) => {
            if let Some(borrowed) =
                compile_borrowed_top_level_cached_ref(name, ctx, ctx.tmp_i32 + 3)
            {
                (borrowed, false)
            } else if !ctx.locals.contains_key(name) && name != "ARGV" {
                (format!("call ${}", ident(name)), true)
            } else {
                (compile_expr(xs_node, &nested_ctx)?, false)
            }
        }
        _ => (compile_expr(xs_node, &nested_ctx)?, false),
    };
    let idx = compile_expr(
        node.children
            .get(2)
            .ok_or_else(|| "get missing index".to_string())?,
        &nested_ctx,
    )?;
    let elem = match node.typ.as_ref() {
        Some(t) => vec_elem_kind_from_type(t)?,
        None => {
            return Err("get missing return type".to_string());
        }
    };
    let statically_in_bounds =
        static_proof_is_safe(crate::static_analysis::ProofKind::BoundsRead, &node.expr);
    let runtime_bounds_check =
        parse_env_bool_like("QUE_BOUNDS_CHECK", true) && !statically_in_bounds;
    if node.typ.as_ref().map(|t| !is_ref_type(t)).unwrap_or(false) {
        if let Some((xs_slot, idx_slot)) = scalar_get_is_proven_in_bounds(node, ctx) {
            if let Some(data_slot) = ctx.hoisted_scalar_vec_data_slots.get(&xs_slot).copied() {
                return Ok(emit_unchecked_scalar_get_from_data_slot(
                    data_slot, idx_slot,
                ));
            }
            return Ok(emit_unchecked_scalar_get_from_slots(xs_slot, idx_slot));
        }
        if let Some(Expression::Int(index)) = node.children.get(2).map(|n| &n.expr) {
            if let Some(data_slot) = hoisted_scalar_vec_data_slot(xs_node, ctx) {
                return Ok(emit_constant_scalar_get_from_data_slot(data_slot, *index));
            }
            return Ok(emit_constant_scalar_get(
                &xs,
                *index,
                ctx.tmp_i32,
                release_xs_after,
                runtime_bounds_check,
            ));
        }
        let bounds = if runtime_bounds_check {
            format!(
                "{xs}\n\
                 local.set {}\n\
                 {idx}\n\
                 local.set {}\n\
                 local.get {}\n\
                 i32.const 0\n\
                 i32.lt_s\n\
                 if\n\
                   unreachable\n\
                 end\n\
                 local.get {}\n\
                 i32.load\n\
                 local.set {}\n\
                 local.get {}\n\
                 local.get {}\n\
                 i32.ge_s\n\
                 if\n\
                   unreachable\n\
                 end\n\
                 local.get {}\n\
                 i32.const 16\n\
                 i32.add\n\
                 i32.load\n\
                 local.get {}\n\
                 i32.const 4\n\
                 i32.mul\n\
                 i32.add\n\
                 i32.load",
                ctx.tmp_i32,
                ctx.tmp_i32 + 1,
                ctx.tmp_i32 + 1,
                ctx.tmp_i32,
                ctx.tmp_i32 + 2,
                ctx.tmp_i32 + 1,
                ctx.tmp_i32 + 2,
                ctx.tmp_i32,
                ctx.tmp_i32 + 1
            )
        } else {
            format!(
                "{xs}\n\
                 local.set {}\n\
                 {idx}\n\
                 local.set {}\n\
                 local.get {}\n\
                 i32.const 16\n\
                 i32.add\n\
                 i32.load\n\
                 local.get {}\n\
                 i32.const 4\n\
                 i32.mul\n\
                 i32.add\n\
                 i32.load",
                ctx.tmp_i32,
                ctx.tmp_i32 + 1,
                ctx.tmp_i32,
                ctx.tmp_i32 + 1
            )
        };
        if !runtime_bounds_check {
            if let Some(data_slot) = hoisted_scalar_vec_data_slot(xs_node, ctx) {
                return Ok(emit_dynamic_scalar_get_from_data_slot(
                    &idx,
                    data_slot,
                    ctx.tmp_i32 + 1,
                ));
            }
        }
        if release_xs_after {
            return Ok(format!(
                "{bounds}\nlocal.get {}\ncall $rc_release_vec\ndrop",
                ctx.tmp_i32
            ));
        }
        return Ok(bounds);
    }
    if !runtime_bounds_check && !release_xs_after {
        if let Some(data_slot) = hoisted_scalar_vec_data_slot(xs_node, ctx) {
            if let Some(Expression::Int(index)) = node.children.get(2).map(|n| &n.expr) {
                return Ok(emit_constant_scalar_get_from_data_slot(data_slot, *index));
            }
            return Ok(emit_dynamic_scalar_get_from_data_slot(
                &idx,
                data_slot,
                ctx.tmp_i32 + 1,
            ));
        }
    }
    if !runtime_bounds_check {
        let load = format!(
            "{xs}\nlocal.set {}\n{idx}\nlocal.set {}\nlocal.get {}\ni32.const 16\ni32.add\ni32.load\nlocal.get {}\ni32.const 4\ni32.mul\ni32.add\ni32.load",
            ctx.tmp_i32,
            ctx.tmp_i32 + 1,
            ctx.tmp_i32,
            ctx.tmp_i32 + 1,
        );
        if release_xs_after {
            return Ok(format!(
                "{load}\nlocal.set {}\nlocal.get {}\ncall $rc_release_vec\ndrop\nlocal.get {}",
                ctx.tmp_i32 + 2,
                ctx.tmp_i32,
                ctx.tmp_i32 + 2
            ));
        }
        return Ok(load);
    }
    if release_xs_after {
        Ok(format!(
            "{xs}\nlocal.set {}\n{idx}\nlocal.get {}\ncall $vec_get_{}\nlocal.set {}\nlocal.get {}\ncall $rc_release_vec\ndrop\nlocal.get {}",
            ctx.tmp_i32,
            ctx.tmp_i32,
            elem.suffix(),
            ctx.tmp_i32 + 1,
            ctx.tmp_i32,
            ctx.tmp_i32 + 1
        ))
    } else {
        Ok(format!("{xs}\n{idx}\ncall $vec_get_{}", elem.suffix()))
    }
}

fn compile_set(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs_node = node
        .children
        .get(1)
        .ok_or_else(|| "set! missing vector".to_string())?;
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: ctx.tmp_i32 + 5,
    };
    let (xs, release_target) = match &xs_node.expr {
        Expression::Word(name) => {
            if let Some(borrowed) =
                compile_borrowed_top_level_cached_ref(name, ctx, ctx.tmp_i32 + 5)
            {
                (borrowed, false)
            } else if !ctx.locals.contains_key(name) && name != "ARGV" {
                (format!("call ${}", ident(name)), true)
            } else {
                (compile_expr(xs_node, &nested_ctx)?, false)
            }
        }
        _ => (compile_expr(xs_node, &nested_ctx)?, false),
    };
    let idx = compile_expr(
        node.children
            .get(2)
            .ok_or_else(|| "set! missing index".to_string())?,
        &nested_ctx,
    )?;
    let val_node = node
        .children
        .get(3)
        .ok_or_else(|| "set! missing value".to_string())?;
    let v = compile_expr(val_node, &nested_ctx)?;
    val_node
        .typ
        .as_ref()
        .ok_or_else(|| "set! value missing type".to_string())
        .and_then(vec_elem_kind_from_type)?;
    let is_scalar_value = val_node
        .typ
        .as_ref()
        .map(|t| !is_ref_type(t))
        .unwrap_or(false);
    let definitely_materialized_scalar_target = if is_scalar_value {
        match &xs_node.expr {
            Expression::Word(name) => ctx
                .locals
                .get(name)
                .map(|slot| ctx.materialized_scalar_local_slots.contains(slot))
                .unwrap_or_else(|| {
                    ctx.definitely_materialized_top_level_scalar_names
                        .contains(name)
                }),
            _ => false,
        }
    } else {
        false
    };
    let scalar_set_op = if definitely_materialized_scalar_target {
        vec_set_runtime_for_materialized_scalar(is_scalar_value)
    } else {
        vec_set_runtime_for_scalar(is_scalar_value)
    };
    let release_rhs = should_release_set_rhs(val_node, ctx.lambda_bindings);
    let managed_slots = managed_local_slots(ctx);
    let target_tmp = ctx.tmp_i32 + 3;
    let target_keep = ctx.tmp_i32 + 4;
    let target_prefix = if release_target {
        format!("{xs}\nlocal.tee {target_tmp}")
    } else {
        xs
    };
    let target_release = if release_target {
        emit_release_managed_temp_if_not_local_alias(
            target_tmp,
            target_keep,
            &managed_slots,
            xs_node.typ.as_ref(),
        )
    } else {
        String::new()
    };
    let proven_replacement = static_proof_is_safe(
        crate::static_analysis::ProofKind::BoundsWriteReplacement,
        &node.expr,
    );
    if is_scalar_value {
        if let Some(Expression::Int(index)) = node.children.get(2).map(|n| &n.expr) {
            if *index >= 0 {
                if let Expression::Word(xs_name) = &xs_node.expr {
                    if let Some(xs_slot) = ctx.locals.get(xs_name) {
                        let proven_len = ctx
                            .proven_scalar_vec_min_lengths
                            .get(xs_slot)
                            .copied()
                            .unwrap_or(0);
                        if proven_len > *index {
                            if let Some(data_slot) =
                                ctx.hoisted_scalar_vec_data_slots.get(xs_slot).copied()
                            {
                                return Ok(emit_constant_scalar_set_from_data_slot(
                                    &v,
                                    data_slot,
                                    *index,
                                    ctx.tmp_i32 + 1,
                                ));
                            }
                            return Ok(emit_constant_scalar_set_unchecked_replacement(
                                &target_prefix,
                                &v,
                                *index,
                                target_tmp,
                                ctx.tmp_i32 + 1,
                                &target_release,
                                definitely_materialized_scalar_target,
                            ));
                        }
                    }
                }
            }
            if proven_replacement && *index >= 0 {
                return Ok(emit_constant_scalar_set_unchecked_replacement(
                    &target_prefix,
                    &v,
                    *index,
                    target_tmp,
                    ctx.tmp_i32 + 1,
                    &target_release,
                    definitely_materialized_scalar_target,
                ));
            }
            return Ok(emit_constant_scalar_set(
                &target_prefix,
                &v,
                *index,
                target_tmp,
                ctx.tmp_i32 + 1,
                &target_release,
                definitely_materialized_scalar_target,
            ));
        }
        if let (Expression::Word(xs_name), Some(Expression::Word(idx_name))) =
            (&xs_node.expr, node.children.get(2).map(|n| &n.expr))
        {
            if ctx
                .proven_scalar_index_loads
                .contains(&(xs_name.clone(), idx_name.clone()))
            {
                if let Some(xs_slot) = ctx.locals.get(xs_name) {
                    if let Some(data_slot) = ctx.hoisted_scalar_vec_data_slots.get(xs_slot).copied()
                    {
                        return Ok(emit_dynamic_scalar_set_from_data_slot(
                            &idx,
                            &v,
                            data_slot,
                            ctx.tmp_i32 + 2,
                            ctx.tmp_i32 + 1,
                        ));
                    }
                }
            }
        }
        if proven_replacement {
            if let Expression::Word(xs_name) = &xs_node.expr {
                if let Some(data_slot) = ctx
                    .locals
                    .get(xs_name)
                    .and_then(|slot| ctx.hoisted_scalar_vec_data_slots.get(slot))
                    .copied()
                {
                    return Ok(emit_dynamic_scalar_set_from_data_slot(
                        &idx,
                        &v,
                        data_slot,
                        ctx.tmp_i32 + 2,
                        ctx.tmp_i32 + 1,
                    ));
                }
            }
            return Ok(emit_dynamic_scalar_set_proven_replacement(
                &target_prefix,
                &idx,
                &v,
                target_tmp,
                ctx.tmp_i32 + 2,
                ctx.tmp_i32 + 1,
                &target_release,
                definitely_materialized_scalar_target,
            ));
        }
        return Ok(emit_dynamic_scalar_set(
            &target_prefix,
            &idx,
            &v,
            target_tmp,
            ctx.tmp_i32 + 2,
            ctx.tmp_i32 + 1,
            &target_release,
            definitely_materialized_scalar_target,
        ));
    }
    if release_rhs {
        let tmp_val = ctx.tmp_i32 + 1;
        let keep_tmp = ctx.tmp_i32 + 2;
        let release = emit_release_managed_temp_if_not_local_alias(
            tmp_val,
            keep_tmp,
            &managed_slots,
            val_node.typ.as_ref(),
        );
        let mut tail = Vec::new();
        tail.push(release);
        if !target_release.is_empty() {
            tail.push(target_release);
        }
        let set_body = format!(
            "{target_prefix}\n{idx}\n{v}\nlocal.tee {tmp_val}\ncall {}",
            scalar_set_op
        );
        Ok(format!("{}\n{}", set_body, tail.join("\n")))
    } else {
        if target_release.is_empty() {
            Ok(format!(
                "{target_prefix}\n{idx}\n{v}\ncall {}",
                scalar_set_op
            ))
        } else {
            Ok(format!(
                "{target_prefix}\n{idx}\n{v}\ncall {}\n{}",
                scalar_set_op, target_release
            ))
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
    let local_idx = *ctx
        .locals
        .get(&target_name)
        .ok_or_else(|| format!("alter! unknown local '{}'", target_name))?;
    let value_node = node
        .children
        .get(2)
        .ok_or_else(|| "alter! missing value".to_string())?;
    let value = compile_expr(value_node, ctx)?;
    let Some(target_type) = ctx.local_types.get(&target_name) else {
        return Ok(format!("{value}\nlocal.set {local_idx}\ni32.const 0"));
    };
    if !is_managed_local_type(target_type) {
        return Ok(format!("{value}\nlocal.set {local_idx}\ni32.const 0"));
    }

    // A managed mutable local owns its current reference. Replacing it must
    // release that ownership instead of simply overwriting the pointer. Keep
    // the RHS in a temporary so self-assignment remains safe: borrowed values
    // are retained before the old target is released, while fresh/owned values
    // transfer their existing ownership directly into the local.
    let value_tmp = ctx.tmp_i32;
    let retain_borrowed = if is_borrowed_managed_rhs_expr(value_node, ctx.lambda_bindings) {
        format!(
            "local.get {value_tmp}\ncall {}\ndrop\n",
            rc_retain_for_type(target_type)
        )
    } else {
        String::new()
    };
    Ok(format!(
        "{value}\n\
         local.set {value_tmp}\n\
         {retain_borrowed}\
         local.get {local_idx}\n\
         call {}\n\
         drop\n\
         local.get {value_tmp}\n\
         local.set {local_idx}\n\
         i32.const 0",
        rc_release_for_type(target_type)
    ))
}

fn compile_expr_discarding_result(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    if matches!(&node.expr, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "if"))
    {
        return compile_if_discarding_result(node, ctx);
    }
    if matches!(&node.expr, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "and"))
    {
        return compile_and_discarding_result(node, ctx);
    }
    if matches!(&node.expr, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "or"))
    {
        return compile_or_discarding_result(node, ctx);
    }
    let code = compile_expr(node, ctx)?;
    if node
        .typ
        .as_ref()
        .map(is_managed_local_type)
        .unwrap_or(false)
        && should_release_set_rhs(node, ctx.lambda_bindings)
    {
        return Ok(format!(
            "{code}\ncall {}\ndrop",
            rc_release_for_opt_type(node.typ.as_ref())
        ));
    }
    if !matches!(node.typ.as_ref(), Some(Type::Unit)) {
        return Ok(format!("{code}\ndrop"));
    }
    let trimmed = code.trim_end();
    if trimmed == "i32.const 0" {
        return Ok(String::new());
    }
    if let Some(prefix) = trimmed.strip_suffix("\ni32.const 0") {
        return Ok(prefix.to_string());
    }
    Ok(format!("{code}\ndrop"))
}

fn extract_less_than_length_loop_bound(
    cond_node: &TypedExpression,
    ctx: &Ctx<'_>,
) -> Option<(String, String, usize, usize)> {
    let Expression::Apply(cond_items) = &cond_node.expr else {
        return None;
    };
    if !matches!(cond_items.first(), Some(Expression::Word(w)) if w == "<") {
        return None;
    }
    let (Some(Expression::Word(idx_name)), Some(Expression::Apply(len_items))) =
        (cond_items.get(1), cond_items.get(2))
    else {
        return None;
    };
    let [Expression::Word(len_op), Expression::Word(xs_name)] = &len_items[..] else {
        return None;
    };
    if len_op != "length" || !ctx.nonnegative_int_locals.contains(idx_name) {
        return None;
    }
    let idx_slot = *ctx.locals.get(idx_name)?;
    let xs_slot = *ctx.locals.get(xs_name)?;
    Some((idx_name.clone(), xs_name.clone(), idx_slot, xs_slot))
}

fn extract_less_than_const_loop_bound(
    cond_node: &TypedExpression,
    ctx: &Ctx<'_>,
) -> Option<(String, i32, usize)> {
    let Expression::Apply(cond_items) = &cond_node.expr else {
        return None;
    };
    if !matches!(cond_items.first(), Some(Expression::Word(w)) if w == "<") {
        return None;
    }
    let (Some(Expression::Word(idx_name)), Some(Expression::Int(bound))) =
        (cond_items.get(1), cond_items.get(2))
    else {
        return None;
    };
    if *bound < 0 || !ctx.nonnegative_int_locals.contains(idx_name) {
        return None;
    }
    let idx_slot = *ctx.locals.get(idx_name)?;
    Some((idx_name.clone(), *bound, idx_slot))
}

fn extract_const_exclusive_loop_bound(
    cond_node: &TypedExpression,
    ctx: &Ctx<'_>,
) -> Option<(String, i32, usize)> {
    if let Some(bound) = extract_less_than_const_loop_bound(cond_node, ctx) {
        return Some(bound);
    }
    let Expression::Apply(cond_items) = &cond_node.expr else {
        return None;
    };
    if !matches!(cond_items.first(), Some(Expression::Word(w)) if w == "<=") {
        return None;
    }
    let (Some(Expression::Word(idx_name)), Some(Expression::Int(bound))) =
        (cond_items.get(1), cond_items.get(2))
    else {
        return None;
    };
    if *bound < 0 || !ctx.nonnegative_int_locals.contains(idx_name) {
        return None;
    }
    let exclusive = bound.checked_add(1)?;
    let idx_slot = *ctx.locals.get(idx_name)?;
    Some((idx_name.clone(), exclusive, idx_slot))
}

fn body_has_only_final_positive_increment(
    body_node: &TypedExpression,
    idx_name: &str,
    ctx: &Ctx<'_>,
) -> bool {
    let Expression::Apply(items) = &body_node.expr else {
        return false;
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") || items.len() < 2 {
        return false;
    }
    let increment_idx = if matches!(items.last(), Some(Expression::Word(w)) if w == "nil") {
        items.len().saturating_sub(2)
    } else {
        items.len().saturating_sub(1)
    };
    if increment_idx == 0 || !is_positive_index_increment_expr(&items[increment_idx], idx_name, ctx)
    {
        return false;
    }
    items[1..increment_idx]
        .iter()
        .all(|expr| !expr_mutates_scalar_name(expr, idx_name))
}

fn is_positive_index_increment_expr(expr: &Expression, idx_name: &str, ctx: &Ctx<'_>) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    let [Expression::Word(op), Expression::Word(target), Expression::Apply(add_items)] = &items[..]
    else {
        return false;
    };
    if op != "alter!" || target != idx_name {
        return false;
    }
    let [Expression::Word(add_op), Expression::Word(lhs), step] = &add_items[..] else {
        return false;
    };
    if add_op != "+" || lhs != idx_name {
        return false;
    }
    match step {
        Expression::Int(step) => *step > 0,
        Expression::Word(step_name) => ctx.nonnegative_int_locals.contains(step_name),
        _ => false,
    }
}

fn scalar_vector_literal_len(node: &TypedExpression) -> Option<i32> {
    if !node
        .typ
        .as_ref()
        .map(is_scalar_vector_type)
        .unwrap_or(false)
    {
        return None;
    }
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    if !matches!(items.first(), Some(Expression::Word(op)) if op == "vector") {
        return None;
    }
    i32::try_from(items.len().saturating_sub(1)).ok()
}

fn append_fill_loop_min_length(
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    exact_int_locals: &HashMap<String, i32>,
) -> Option<(usize, i32)> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    if !matches!(items.first(), Some(Expression::Word(op)) if op == "while") {
        return None;
    }
    let cond_node = node.children.get(1)?;
    let body_node = node.children.get(2)?;
    let (idx_name, exclusive_bound, _) = extract_const_exclusive_loop_bound(cond_node, ctx)?;
    let start = *exact_int_locals.get(&idx_name)?;
    if start < 0 || exclusive_bound <= start {
        return None;
    }
    if !body_has_only_final_positive_increment(body_node, &idx_name, ctx) {
        return None;
    }
    let mut append_slots = HashMap::new();
    if !collect_loop_append_by_length_set_vector_slots(&body_node.expr, ctx, &mut append_slots)
        || append_slots.len() != 1
    {
        return None;
    }
    let (_, slot) = append_slots.into_iter().next()?;
    Some((slot, exclusive_bound.saturating_sub(start)))
}

fn append_fill_loop_const_i32(
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    exact_int_locals: &HashMap<String, i32>,
) -> Option<(usize, i32, i32, String)> {
    let cond_node = node.children.get(1)?;
    let body_node = node.children.get(2)?;
    let (idx_name, _, idx_slot) = extract_const_exclusive_loop_bound(cond_node, ctx)?;
    let (slot, len) = append_fill_loop_min_length(node, ctx, exact_int_locals)?;
    if !body_has_final_literal_one_increment(body_node, &idx_name) {
        return None;
    }
    let value = append_by_length_const_i32_value(&body_node.expr)?;
    Some((slot, len, value, idx_name_from_slot(idx_slot, ctx)?))
}

fn idx_name_from_slot(slot: usize, ctx: &Ctx<'_>) -> Option<String> {
    ctx.locals
        .iter()
        .find_map(|(name, local_slot)| (*local_slot == slot).then(|| name.clone()))
}

fn body_has_final_literal_one_increment(body_node: &TypedExpression, idx_name: &str) -> bool {
    let Expression::Apply(items) = &body_node.expr else {
        return false;
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") || items.len() < 2 {
        return false;
    }
    let increment_idx = if matches!(items.last(), Some(Expression::Word(w)) if w == "nil") {
        items.len().saturating_sub(2)
    } else {
        items.len().saturating_sub(1)
    };
    let Some(Expression::Apply(increment)) = items.get(increment_idx) else {
        return false;
    };
    matches!(
        &increment[..],
        [
            Expression::Word(op),
            Expression::Word(target),
            Expression::Apply(add_items)
        ] if op == "alter!"
            && target == idx_name
            && matches!(
                &add_items[..],
                [Expression::Word(add), Expression::Word(lhs), Expression::Int(1)]
                    if add == "+" && lhs == idx_name
            )
    )
}

fn append_by_length_const_i32_value(expr: &Expression) -> Option<i32> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "do") {
        return items
            .iter()
            .skip(1)
            .find_map(append_by_length_const_i32_value);
    }
    let [Expression::Word(op), Expression::Word(target), Expression::Apply(index_items), Expression::Int(value)] =
        &items[..]
    else {
        return None;
    };
    if op == "set!"
        && matches!(
            &index_items[..],
            [Expression::Word(len_op), Expression::Word(len_target)]
                if len_op == "length" && len_target == target
        )
    {
        Some(*value)
    } else {
        None
    }
}

fn collect_loop_append_by_length_set_vector_slots(
    expr: &Expression,
    ctx: &Ctx<'_>,
    out: &mut HashMap<String, usize>,
) -> bool {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "while")
            {
                return true;
            }
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if op == "set!" {
                    let is_append_index = matches!(
                        items.get(2),
                        Some(Expression::Apply(index_items))
                            if matches!(
                                &index_items[..],
                                [Expression::Word(len_op), Expression::Word(len_target)]
                                    if len_op == "length" && len_target == target
                            )
                    );
                    if !is_append_index {
                        return false;
                    }
                    if let Some(slot) = ctx.locals.get(target) {
                        if ctx
                            .local_types
                            .get(target)
                            .map(is_scalar_vector_type)
                            .unwrap_or(false)
                        {
                            out.insert(target.clone(), *slot);
                        }
                    }
                } else if matches!(op.as_str(), "push!" | "pop!" | "pop-val!" | "pull!")
                    || op.ends_with('!')
                {
                    if ctx
                        .local_types
                        .get(target)
                        .map(is_scalar_vector_type)
                        .unwrap_or(false)
                    {
                        return false;
                    }
                }
            }
            items
                .iter()
                .all(|item| collect_loop_append_by_length_set_vector_slots(item, ctx, out))
        }
        _ => true,
    }
}

fn collect_altered_int_locals(expr: &Expression, exact_int_locals: &mut HashMap<String, i32>) {
    match expr {
        Expression::Apply(items) => {
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if op == "alter!" || op == "&alter!" {
                    exact_int_locals.remove(target);
                }
            }
            for item in items {
                collect_altered_int_locals(item, exact_int_locals);
            }
        }
        _ => {}
    }
}

fn expr_mutates_scalar_name(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::Apply(items) => {
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if (op == "alter!" || op == "mut" || op == "let") && target == name {
                    return true;
                }
            }
            items
                .iter()
                .any(|item| expr_mutates_scalar_name(item, name))
        }
        _ => false,
    }
}

fn expr_mutates_vector_name_except_self(
    expr: &Expression,
    name: &str,
    current_function: Option<&str>,
) -> bool {
    match expr {
        Expression::Apply(items) => {
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if matches!(
                    op.as_str(),
                    "set!" | "push!" | "pop!" | "pop-val!" | "pull!"
                ) && target == name
                {
                    return true;
                }
                if current_function.is_some_and(|self_name| op == self_name) && target == name {
                    return items.iter().any(|item| {
                        expr_mutates_vector_name_except_self(item, name, current_function)
                    });
                }
                if op.ends_with('!') && target == name {
                    return true;
                }
            }
            items
                .iter()
                .any(|item| expr_mutates_vector_name_except_self(item, name, current_function))
        }
        _ => false,
    }
}

fn vector_mutations_are_loop_replacement_sets(
    expr: &Expression,
    vector_name: &str,
    idx_name: &str,
    current_function: Option<&str>,
) -> bool {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "while")
            {
                return true;
            }
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if target == vector_name {
                    if op == "set!" {
                        return matches!(items.get(2), Some(Expression::Word(i)) if i == idx_name)
                            && items.iter().all(|item| {
                                vector_mutations_are_loop_replacement_sets(
                                    item,
                                    vector_name,
                                    idx_name,
                                    current_function,
                                )
                            });
                    }
                    if current_function.is_some_and(|self_name| op == self_name) {
                        return items.iter().all(|item| {
                            vector_mutations_are_loop_replacement_sets(
                                item,
                                vector_name,
                                idx_name,
                                current_function,
                            )
                        });
                    }
                    if matches!(op.as_str(), "push!" | "pop!" | "pop-val!" | "pull!")
                        || op.ends_with('!')
                    {
                        return false;
                    }
                }
            }
            items.iter().all(|item| {
                vector_mutations_are_loop_replacement_sets(
                    item,
                    vector_name,
                    idx_name,
                    current_function,
                )
            })
        }
        _ => true,
    }
}

fn collect_loop_replacement_set_vector_slots(
    expr: &Expression,
    ctx: &Ctx<'_>,
    idx_name: &str,
    out: &mut HashMap<String, usize>,
) -> bool {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "while")
            {
                return true;
            }
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if op == "set!" {
                    if !matches!(items.get(2), Some(Expression::Word(i)) if i == idx_name) {
                        return false;
                    }
                    if let Some(slot) = ctx.locals.get(target) {
                        if ctx
                            .local_types
                            .get(target)
                            .map(is_scalar_vector_type)
                            .unwrap_or(false)
                        {
                            out.insert(target.clone(), *slot);
                        }
                    }
                } else if matches!(op.as_str(), "push!" | "pop!" | "pop-val!" | "pull!")
                    || op.ends_with('!')
                {
                    if ctx
                        .current_function
                        .is_some_and(|self_name| op == self_name)
                    {
                        return items.iter().all(|item| {
                            collect_loop_replacement_set_vector_slots(item, ctx, idx_name, out)
                        });
                    }
                    if ctx
                        .local_types
                        .get(target)
                        .map(is_scalar_vector_type)
                        .unwrap_or(false)
                    {
                        return false;
                    }
                }
            }
            items
                .iter()
                .all(|item| collect_loop_replacement_set_vector_slots(item, ctx, idx_name, out))
        }
        _ => true,
    }
}

fn scalar_vector_set_target_slot(expr: &Expression, ctx: &Ctx<'_>) -> Option<usize> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let [Expression::Word(op), Expression::Word(target), ..] = &items[..] else {
        return None;
    };
    if op != "set!" {
        return None;
    }
    let slot = *ctx.locals.get(target)?;
    if ctx
        .local_types
        .get(target)
        .map(is_scalar_vector_type)
        .unwrap_or(false)
    {
        Some(slot)
    } else {
        None
    }
}

fn update_scalar_vec_min_lengths_after_expr(
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    minimums: &mut HashMap<usize, i32>,
) {
    if !node.effect.contains(EffectFlags::MUTATE)
        && !node.effect.contains(EffectFlags::UNKNOWN_CALL)
    {
        return;
    }
    let Expression::Apply(items) = &node.expr else {
        minimums.clear();
        return;
    };
    let Some(op) = items.first().and_then(|expr| match expr {
        Expression::Word(name) => Some(name.as_str()),
        _ => None,
    }) else {
        minimums.clear();
        return;
    };

    let argument_slot = |argument: &Expression| match argument {
        Expression::Word(name) => ctx.locals.get(name).copied(),
        _ => None,
    };
    match op {
        "push!" => {
            if let Some(slot) = items.get(1).and_then(argument_slot) {
                minimums
                    .entry(slot)
                    .and_modify(|length| *length = length.saturating_add(1));
            }
            return;
        }
        "pop!" | "pop-val!" | "pull!" => {
            if let Some(slot) = items.get(1).and_then(argument_slot) {
                minimums
                    .entry(slot)
                    .and_modify(|length| *length = length.saturating_sub(1));
            }
            return;
        }
        "set!" => {
            let Some(slot) = items.get(1).and_then(argument_slot) else {
                return;
            };
            let appends_at_length = items.get(2).is_some_and(|index| {
                matches!(index, Expression::Apply(length)
                    if matches!(length.as_slice(), [Expression::Word(name), target]
                        if name == "length" && argument_slot(target) == Some(slot)))
            });
            if appends_at_length {
                minimums
                    .entry(slot)
                    .and_modify(|length| *length = length.saturating_add(1));
            }
            return;
        }
        "alter!" => return,
        _ => {}
    }

    // A direct top-level function cannot capture this function's locals, so
    // only vector arguments can have changed size. Local/dynamic closures may
    // capture any local vector and therefore invalidate the whole map.
    if ctx.fn_sigs.contains_key(op) && !ctx.lambda_bindings.contains_key(op) {
        for argument in items.iter().skip(1) {
            if let Some(slot) = argument_slot(argument) {
                minimums.remove(&slot);
            }
        }
    } else {
        minimums.clear();
    }
}

fn collect_scalar_param_constant_set_requirements(
    expr: &Expression,
    ctx: &Ctx<'_>,
    param_count: usize,
    self_name: &str,
    required_min_lengths: &mut HashMap<usize, i32>,
    invalid_slots: &mut HashSet<usize>,
) {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec")
            {
                return;
            }
            if matches!(items.first(), Some(Expression::Word(op)) if op == self_name) {
                for slot in 0..param_count {
                    let is_scalar_param = ctx.local_types.iter().any(|(name, ty)| {
                        ctx.locals.get(name) == Some(&slot) && is_scalar_vector_type(ty)
                    });
                    if !is_scalar_param {
                        continue;
                    }
                    let forwards_same_slot =
                        items.get(slot + 1).and_then(|argument| match argument {
                            Expression::Word(name) => ctx.locals.get(name).copied(),
                            _ => None,
                        }) == Some(slot);
                    if !forwards_same_slot {
                        invalid_slots.insert(slot);
                    }
                }
            }
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if let Some(slot) = ctx.locals.get(target).copied() {
                    let is_scalar_param = slot < param_count
                        && ctx
                            .local_types
                            .get(target)
                            .map(is_scalar_vector_type)
                            .unwrap_or(false);
                    if is_scalar_param {
                        if op == "get" || op == "set!" {
                            match items.get(2) {
                                Some(Expression::Int(index)) if *index >= 0 => {
                                    let needed = index.saturating_add(1);
                                    required_min_lengths
                                        .entry(slot)
                                        .and_modify(|min| *min = (*min).max(needed))
                                        .or_insert(needed);
                                }
                                _ => {
                                    if op == "set!" {
                                        invalid_slots.insert(slot);
                                    }
                                }
                            }
                        } else if matches!(op.as_str(), "push!" | "pop!" | "pop-val!" | "pull!") {
                            invalid_slots.insert(slot);
                        }
                    }
                }
            }
            for item in items {
                collect_scalar_param_constant_set_requirements(
                    item,
                    ctx,
                    param_count,
                    self_name,
                    required_min_lengths,
                    invalid_slots,
                );
            }
        }
        _ => {}
    }
}

fn collect_typed_scalar_param_call_invalidations(
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    param_count: usize,
    self_name: &str,
    invalid_slots: &mut HashSet<usize>,
) {
    let Expression::Apply(items) = &node.expr else {
        return;
    };
    let op = items.first().and_then(|item| match item {
        Expression::Word(name) => Some(name.as_str()),
        _ => None,
    });
    if matches!(op, Some("lambda" | "letrec")) {
        return;
    }
    let is_language_form = op.is_some_and(|op| {
        matches!(
            op,
            "do" | "block"
                | "if"
                | "cond"
                | "and"
                | "or"
                | "let"
                | "mut"
                | "alter!"
                | "while"
                | "loop"
                | "loop/range"
                | "set!"
                | "push!"
                | "pop!"
                | "pop-val!"
                | "pull!"
        )
    });
    let may_mutate_argument = node.effect.contains(EffectFlags::MUTATE)
        || node.effect.contains(EffectFlags::UNKNOWN_CALL);
    if op != Some(self_name) && !is_language_form && may_mutate_argument {
        for argument in items.iter().skip(1) {
            let Expression::Word(name) = argument else {
                continue;
            };
            let Some(slot) = ctx.locals.get(name).copied() else {
                continue;
            };
            if slot < param_count && ctx.local_types.get(name).is_some_and(is_scalar_vector_type) {
                invalid_slots.insert(slot);
            }
        }
    }
    for child in &node.children {
        collect_typed_scalar_param_call_invalidations(
            child,
            ctx,
            param_count,
            self_name,
            invalid_slots,
        );
    }
}

fn scalar_param_constant_set_requirements(
    body: &TypedExpression,
    ctx: &Ctx<'_>,
    param_count: usize,
    self_name: &str,
) -> HashMap<usize, i32> {
    if parse_env_bool_like("QUE_BOUNDS_CHECK", true) {
        return HashMap::new();
    }
    let mut required_min_lengths = HashMap::new();
    let mut invalid_slots = HashSet::new();
    collect_scalar_param_constant_set_requirements(
        &body.expr,
        ctx,
        param_count,
        self_name,
        &mut required_min_lengths,
        &mut invalid_slots,
    );
    collect_typed_scalar_param_call_invalidations(
        body,
        ctx,
        param_count,
        self_name,
        &mut invalid_slots,
    );
    for slot in invalid_slots {
        required_min_lengths.remove(&slot);
    }
    required_min_lengths
}

fn recursive_calls_forward_guarded_params(
    expr: &Expression,
    self_name: &str,
    params: &[(String, Type)],
    requirements: &[(usize, i32)],
    found: &mut bool,
) -> bool {
    let Expression::Apply(items) = expr else {
        return true;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return true;
    }
    if matches!(items.first(), Some(Expression::Word(name)) if name == self_name) {
        *found = true;
        if items.len() != params.len() + 1 {
            return false;
        }
        return requirements.iter().all(|(slot, _)| {
            params.get(*slot).is_some_and(|(parameter, _)| {
                matches!(items.get(*slot + 1), Some(Expression::Word(argument)) if argument == parameter)
            })
        });
    }
    items.iter().skip(1).all(|child| {
        recursive_calls_forward_guarded_params(child, self_name, params, requirements, found)
    })
}

fn redirect_direct_recursive_calls(code: &str, self_name: &str, fast_name: &str) -> String {
    code.replace(
        &format!("call ${}", ident(self_name)),
        &format!("call ${fast_name}"),
    )
}

struct GuardedScalarParamBody {
    requirements: Vec<(usize, i32)>,
    fallback_code: String,
    fast_prelude: String,
    fast_code: String,
    guard_tmp: usize,
    result_ty: &'static str,
}

impl GuardedScalarParamBody {
    fn inline_code(&self) -> String {
        let guard_code = self
            .requirements
            .iter()
            .map(|(slot, min_len)| {
                format!(
                    "local.get {slot}\n\
                     i32.const 20\n\
                     i32.add\n\
                     i32.load\n\
                     i32.const 1447380017\n\
                     i32.ne\n\
                     local.get {slot}\n\
                     call $vec_len\n\
                     i32.const {min_len}\n\
                     i32.lt_s\n\
                     i32.or\n\
                     if\n\
                       i32.const 1\n\
                       local.set {guard_tmp}\n\
                     end",
                    guard_tmp = self.guard_tmp,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "i32.const 0\n\
             local.set {guard_tmp}\n\
             {guard_code}\n\
             local.get {guard_tmp}\n\
             if (result {result_ty})\n\
               {fallback_code}\n\
             else\n\
               {fast_prelude}\n\
               {fast_code}\n\
             end",
            guard_tmp = self.guard_tmp,
            result_ty = self.result_ty,
            fallback_code = self.fallback_code,
            fast_prelude = self.fast_prelude,
            fast_code = self.fast_code,
        )
    }

    fn any_short_guard_code(&self) -> String {
        let mut parts = Vec::new();
        for (i, (slot, min_len)) in self.requirements.iter().enumerate() {
            parts.push(format!(
                "local.get {slot}\n\
                 i32.const 20\n\
                 i32.add\n\
                 i32.load\n\
                 i32.const 1447380017\n\
                 i32.ne\n\
                 local.get {slot}\n\
                 call $vec_len\n\
                 i32.const {min_len}\n\
                 i32.lt_s\n\
                 i32.or"
            ));
            if i > 0 {
                parts.push("i32.or".to_string());
            }
        }
        parts.join("\n")
    }
}

fn compile_guarded_scalar_param_replacement_body_with<F>(
    body: &TypedExpression,
    ctx: &Ctx<'_>,
    self_name: &str,
    param_count: usize,
    mut compile_body: F,
) -> Result<Option<GuardedScalarParamBody>, String>
where
    F: FnMut(&Ctx<'_>) -> Result<Option<String>, String>,
{
    let requirements = scalar_param_constant_set_requirements(body, ctx, param_count, self_name);
    if requirements.is_empty() {
        return Ok(None);
    }

    let Some(fallback_code) = compile_body(ctx)? else {
        return Ok(None);
    };
    let mut proven_min_lengths = ctx.proven_scalar_vec_min_lengths.clone();
    for (slot, min_len) in &requirements {
        proven_min_lengths
            .entry(*slot)
            .and_modify(|existing| *existing = (*existing).max(*min_len))
            .or_insert(*min_len);
    }
    let guard_tmp = ctx.tmp_i32;
    let mut next_tmp_i32 = ctx.tmp_i32 + 1;
    let mut hoisted_data_slots = ctx.hoisted_scalar_vec_data_slots.clone();
    let mut materialized_slots = ctx.materialized_scalar_local_slots.clone();
    let mut sorted_requirements = requirements.iter().collect::<Vec<_>>();
    sorted_requirements.sort_by_key(|(slot, _)| **slot);
    let fast_prelude = sorted_requirements
        .iter()
        .filter_map(|(slot, _)| {
            materialized_slots.insert(**slot);
            if hoisted_data_slots.contains_key(*slot) {
                return None;
            }
            let data_slot = next_tmp_i32;
            next_tmp_i32 += 1;
            hoisted_data_slots.insert(**slot, data_slot);
            Some(format!(
                "local.get {slot}\n\
                 i32.const 16\n\
                 i32.add\n\
                 i32.load\n\
                 local.set {data_slot}",
                slot = **slot
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let fast_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: materialized_slots,
        hoisted_scalar_vec_data_slots: hoisted_data_slots,
        proven_scalar_vec_min_lengths: proven_min_lengths,
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: next_tmp_i32,
    };
    let Some(fast_code) = compile_body(&fast_ctx)? else {
        return Ok(None);
    };
    let result_ty = body
        .typ
        .as_ref()
        .ok_or_else(|| "guarded scalar replacement body missing type".to_string())
        .and_then(wasm_val_type)?;
    let mut sorted_requirements = requirements.into_iter().collect::<Vec<_>>();
    sorted_requirements.sort_by_key(|(slot, _)| *slot);

    Ok(Some(GuardedScalarParamBody {
        requirements: sorted_requirements,
        fallback_code,
        fast_prelude,
        fast_code,
        guard_tmp,
        result_ty,
    }))
}

fn compile_guarded_scalar_param_replacement_body(
    body: &TypedExpression,
    ctx: &Ctx<'_>,
    self_name: &str,
    param_count: usize,
) -> Result<Option<String>, String> {
    compile_guarded_scalar_param_replacement_body_with(
        body,
        ctx,
        self_name,
        param_count,
        |body_ctx| compile_expr(body, body_ctx).map(Some),
    )
    .map(|maybe| maybe.map(|guarded| guarded.inline_code()))
}

fn compile_guarded_scalar_param_replacement_tail_body(
    body: &TypedExpression,
    ctx: &Ctx<'_>,
    self_name: &str,
    param_count: usize,
    releasable_ref_slots: &[ManagedRefSlot],
) -> Result<Option<GuardedScalarParamBody>, String> {
    compile_guarded_scalar_param_replacement_body_with(
        body,
        ctx,
        self_name,
        param_count,
        |body_ctx| compile_tail_expr(body, body_ctx, self_name, param_count, releasable_ref_slots),
    )
}

fn collect_loop_materialized_scalar_set_slots(
    expr: &Expression,
    ctx: &Ctx<'_>,
    out: &mut HashSet<usize>,
) {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "while")
            {
                return;
            }
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if op == "set!" {
                    if let Some(slot) = ctx.locals.get(target) {
                        if ctx
                            .local_types
                            .get(target)
                            .map(is_scalar_vector_type)
                            .unwrap_or(false)
                        {
                            out.insert(*slot);
                        }
                    }
                }
            }
            for item in items {
                collect_loop_materialized_scalar_set_slots(item, ctx, out);
            }
        }
        _ => {}
    }
}

fn loop_materialize_once_plan(body: &TypedExpression, ctx: &Ctx<'_>) -> (HashSet<usize>, String) {
    if parse_env_bool_like("QUE_BOUNDS_CHECK", true) {
        return (ctx.materialized_scalar_local_slots.clone(), String::new());
    }
    let mut slots = HashSet::new();
    collect_loop_materialized_scalar_set_slots(&body.expr, ctx, &mut slots);
    slots.retain(|slot| !ctx.materialized_scalar_local_slots.contains(slot));
    if slots.is_empty() {
        return (ctx.materialized_scalar_local_slots.clone(), String::new());
    }
    let mut sorted: Vec<usize> = slots.iter().copied().collect();
    sorted.sort_unstable();
    let prelude = sorted
        .iter()
        .map(|slot| format!("local.get {slot}\ncall $vec_materialize_i32\ndrop"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut materialized = ctx.materialized_scalar_local_slots.clone();
    materialized.extend(sorted);
    (materialized, prelude)
}

fn collect_loop_vector_get_slots(expr: &Expression, ctx: &Ctx<'_>, out: &mut HashSet<usize>) {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "while")
            {
                return;
            }
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if op == "get" {
                    if let Some(slot) = ctx.locals.get(target) {
                        if matches!(ctx.local_types.get(target), Some(Type::List(_))) {
                            out.insert(*slot);
                        }
                    }
                }
            }
            for item in items {
                collect_loop_vector_get_slots(item, ctx, out);
            }
        }
        _ => {}
    }
}

fn expr_mutates_any_local_vector(expr: &Expression, ctx: &Ctx<'_>) -> bool {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda") {
                return false;
            }
            if let [Expression::Word(op), Expression::Word(target), ..] = &items[..] {
                if op.ends_with('!') && matches!(ctx.local_types.get(target), Some(Type::List(_))) {
                    return true;
                }
            }
            items
                .iter()
                .any(|item| expr_mutates_any_local_vector(item, ctx))
        }
        _ => false,
    }
}

fn loop_hoisted_data_pointer_plan(
    body: &TypedExpression,
    ctx: &Ctx<'_>,
    base_tmp: usize,
) -> (HashMap<usize, usize>, String, usize) {
    if parse_env_bool_like("QUE_BOUNDS_CHECK", true) {
        return (
            ctx.hoisted_scalar_vec_data_slots.clone(),
            String::new(),
            base_tmp,
        );
    }

    let mut slots = HashSet::new();
    collect_loop_vector_get_slots(&body.expr, ctx, &mut slots);
    slots.retain(|slot| !ctx.hoisted_scalar_vec_data_slots.contains_key(slot));
    let mutating_local_vector = expr_mutates_any_local_vector(&body.expr, ctx);
    slots.retain(|slot| {
        let names = ctx
            .locals
            .iter()
            .filter_map(|(name, local_slot)| (local_slot == slot).then_some(name));
        names.clone().next().is_some_and(|_| {
            names.into_iter().all(|name| {
                let managed_elements = ctx
                    .local_types
                    .get(name)
                    .is_some_and(|typ| matches!(typ, Type::List(inner) if is_ref_type(inner)));
                (!managed_elements || !mutating_local_vector)
                    && !expr_mutates_vector_name_except_self(&body.expr, name, ctx.current_function)
            })
        })
    });

    if slots.is_empty() {
        return (
            ctx.hoisted_scalar_vec_data_slots.clone(),
            String::new(),
            base_tmp,
        );
    }

    let mut sorted: Vec<usize> = slots.into_iter().collect();
    sorted.sort_unstable();
    let mut hoisted = ctx.hoisted_scalar_vec_data_slots.clone();
    let mut next_tmp = base_tmp;
    let mut prelude = Vec::new();
    for slot in sorted {
        let data_slot = next_tmp;
        next_tmp += 1;
        hoisted.insert(slot, data_slot);
        prelude.push(format!(
            "local.get {slot}\n\
             i32.const 16\n\
             i32.add\n\
             i32.load\n\
             local.set {data_slot}"
        ));
    }
    (hoisted, prelude.join("\n"), next_tmp)
}

fn compile_pop(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs = compile_expr(
        node.children
            .get(1)
            .ok_or_else(|| "pop! missing vector".to_string())?,
        ctx,
    )?;
    Ok(format!("{xs}\ncall $vec_pop_i32"))
}

fn compile_pop_val(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs = compile_expr(
        node.children
            .get(1)
            .ok_or_else(|| "pop-val! missing vector".to_string())?,
        ctx,
    )?;
    if static_proof_is_safe(crate::static_analysis::ProofKind::NonEmpty, &node.expr) {
        let ptr = ctx.tmp_i32;
        let new_len = ctx.tmp_i32 + 1;
        let value = ctx.tmp_i32 + 2;
        return Ok(format!(
            "{xs}\n\
             local.set {ptr}\n\
             local.get {ptr}\n\
             call $vec_materialize_i32\n\
             drop\n\
             local.get {ptr}\n\
             i32.load\n\
             i32.const 1\n\
             i32.sub\n\
             local.set {new_len}\n\
             local.get {ptr}\n\
             i32.const 16\n\
             i32.add\n\
             i32.load\n\
             local.get {new_len}\n\
             i32.const 4\n\
             i32.mul\n\
             i32.add\n\
             i32.load\n\
             local.set {value}\n\
             local.get {ptr}\n\
             local.get {new_len}\n\
             i32.store\n\
             local.get {value}"
        ));
    }
    Ok(format!("{xs}\ncall $vec_pop_val_i32"))
}

fn compile_cdr(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let xs_node = node
        .children
        .get(1)
        .ok_or_else(|| "cdr missing vector".to_string())?;
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

fn compile_generic_while_loop(
    cond_node: &TypedExpression,
    body_node: &TypedExpression,
    ctx: &Ctx<'_>,
) -> Result<String, String> {
    let cond = compile_expr(cond_node, ctx)?;
    // The condition is emitted both before and inside the loop. Its scratch
    // locals must not overlap persistent data pointers hoisted for the body;
    // otherwise re-evaluating a compound condition corrupts those pointers.
    let hoist_base = max_local_index_in_code(&cond)
        .map(|index| index + 1)
        .unwrap_or(ctx.tmp_i32)
        .max(ctx.tmp_i32);
    let (materialized_slots, materialize_once) = loop_materialize_once_plan(body_node, ctx);
    let (hoisted_data_slots, hoist_data_pointers, next_tmp_i32) =
        loop_hoisted_data_pointer_plan(body_node, ctx, hoist_base);
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: materialized_slots,
        hoisted_scalar_vec_data_slots: hoisted_data_slots,
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: next_tmp_i32,
    };
    let body_and_drop = compile_expr_discarding_result(body_node, &nested_ctx)?;

    let loop_prelude = [materialize_once.as_str(), hoist_data_pointers.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if !loop_prelude.is_empty() {
        return Ok(format!(
            "block\n\
             {cond}\n\
             i32.eqz\n\
             br_if 0\n\
             {loop_prelude}\n\
             loop\n\
               {body_and_drop}\n\
               {cond}\n\
               br_if 0\n\
             end\n\
             end\n\
             i32.const 0"
        ));
    }

    Ok(format!(
        "block\n  loop\n    {cond}\n    i32.eqz\n    br_if 1\n    {body_and_drop}\n    br 0\n  end\nend\ni32.const 0"
    ))
}

fn compile_loop_while(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let cond_node = node
        .children
        .get(1)
        .ok_or_else(|| "while missing condition".to_string())?;
    let body_node = node
        .children
        .get(2)
        .ok_or_else(|| "while missing body".to_string())?;
    if let Some((idx_name, xs_name, idx_slot, xs_slot)) =
        extract_less_than_length_loop_bound(cond_node, ctx)
    {
        let vector_mutations_are_replacement_sets = vector_mutations_are_loop_replacement_sets(
            &body_node.expr,
            &xs_name,
            &idx_name,
            ctx.current_function,
        );
        if body_has_only_final_positive_increment(body_node, &idx_name, ctx)
            && (!expr_mutates_vector_name_except_self(
                &body_node.expr,
                &xs_name,
                ctx.current_function,
            ) || vector_mutations_are_replacement_sets)
        {
            let mut proven = ctx.proven_scalar_index_loads.clone();
            proven.insert((xs_name.clone(), idx_name));
            let (materialized_slots, materialize_once) = loop_materialize_once_plan(body_node, ctx);
            let (mut hoisted_data_slots, mut hoist_data_pointers, mut next_tmp_i32) =
                loop_hoisted_data_pointer_plan(body_node, ctx, ctx.tmp_i32 + 1);
            if vector_mutations_are_replacement_sets
                && !hoisted_data_slots.contains_key(&xs_slot)
                && ctx
                    .local_types
                    .iter()
                    .any(|(name, typ)| name == &xs_name && is_scalar_vector_type(typ))
            {
                let data_slot = next_tmp_i32;
                next_tmp_i32 += 1;
                hoisted_data_slots.insert(xs_slot, data_slot);
                let data_prelude = format!(
                    "local.get {xs_slot}\n\
                     i32.const 16\n\
                     i32.add\n\
                     i32.load\n\
                     local.set {data_slot}"
                );
                if hoist_data_pointers.is_empty() {
                    hoist_data_pointers = data_prelude;
                } else {
                    hoist_data_pointers.push('\n');
                    hoist_data_pointers.push_str(&data_prelude);
                }
            }
            let nested_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                extern_names: ctx.extern_names,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: ctx.lambda_bindings,
                current_function: ctx.current_function,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                materialized_scalar_local_slots: materialized_slots,
                hoisted_scalar_vec_data_slots: hoisted_data_slots,
                proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
                definitely_materialized_top_level_scalar_names: ctx
                    .definitely_materialized_top_level_scalar_names,
                proven_scalar_index_loads: &proven,
                nonnegative_int_locals: ctx.nonnegative_int_locals,
                tmp_i32: next_tmp_i32,
            };
            let body_and_drop = compile_expr_discarding_result(body_node, &nested_ctx)?;
            let loop_prelude = [materialize_once.as_str(), hoist_data_pointers.as_str()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if loop_prelude.is_empty() {
                return Ok(format!(
                    "local.get {xs_slot}\n\
                     call $vec_len\n\
                     local.set {}\n\
                     block\n\
                       loop\n\
                         local.get {idx_slot}\n\
                         local.get {}\n\
                         i32.ge_s\n\
                         br_if 1\n\
                         {body_and_drop}\n\
                         br 0\n\
                       end\n\
                     end\n\
                     i32.const 0",
                    ctx.tmp_i32, ctx.tmp_i32
                ));
            }
            return Ok(format!(
                "local.get {xs_slot}\n\
                 call $vec_len\n\
                 local.set {}\n\
                 block\n\
                   local.get {idx_slot}\n\
                   local.get {}\n\
                   i32.ge_s\n\
                   br_if 0\n\
                  {loop_prelude}\n\
                   loop\n\
                     {body_and_drop}\n\
                     local.get {idx_slot}\n\
                     local.get {}\n\
                     i32.lt_s\n\
                     br_if 0\n\
                   end\n\
                 end\n\
                 i32.const 0",
                ctx.tmp_i32, ctx.tmp_i32, ctx.tmp_i32
            ));
        }
    }
    if !parse_env_bool_like("QUE_BOUNDS_CHECK", true) {
        if let Some((idx_name, bound, idx_slot)) =
            extract_const_exclusive_loop_bound(cond_node, ctx)
        {
            let mut replacement_set_slots = HashMap::new();
            if body_has_only_final_positive_increment(body_node, &idx_name, ctx)
                && collect_loop_replacement_set_vector_slots(
                    &body_node.expr,
                    ctx,
                    &idx_name,
                    &mut replacement_set_slots,
                )
                && !replacement_set_slots.is_empty()
            {
                let generic_loop = compile_generic_while_loop(cond_node, body_node, ctx)?;
                let mut proven = ctx.proven_scalar_index_loads.clone();
                let mut hoisted_data_slots = ctx.hoisted_scalar_vec_data_slots.clone();
                let (materialized_slots, materialize_once) =
                    loop_materialize_once_plan(body_node, ctx);
                let guard_tmp = ctx.tmp_i32;
                let mut next_tmp_i32 = ctx.tmp_i32 + 1;
                let mut guard_parts = Vec::new();
                let mut hoist_parts = Vec::new();
                let mut sorted_slots: Vec<(String, usize)> =
                    replacement_set_slots.into_iter().collect();
                sorted_slots.sort_by(|a, b| a.0.cmp(&b.0));
                for (xs_name, xs_slot) in sorted_slots {
                    proven.insert((xs_name, idx_name.clone()));
                    if !hoisted_data_slots.contains_key(&xs_slot) {
                        let data_slot = next_tmp_i32;
                        next_tmp_i32 += 1;
                        hoisted_data_slots.insert(xs_slot, data_slot);
                        hoist_parts.push(format!(
                            "local.get {xs_slot}\n\
                             i32.const 16\n\
                             i32.add\n\
                             i32.load\n\
                             local.set {data_slot}"
                        ));
                    }
                    guard_parts.push(format!(
                        "local.get {xs_slot}\n\
                         call $vec_len\n\
                         i32.const {bound}\n\
                         i32.lt_s\n\
                         if\n\
                           i32.const 1\n\
                           local.set {guard_tmp}\n\
                         end"
                    ));
                }
                let nested_ctx = Ctx {
                    fn_sigs: ctx.fn_sigs,
                    fn_ids: ctx.fn_ids,
                    extern_names: ctx.extern_names,
                    lambda_ids: ctx.lambda_ids,
                    closure_defs: ctx.closure_defs,
                    lambda_bindings: ctx.lambda_bindings,
                    current_function: ctx.current_function,
                    locals: ctx.locals.clone(),
                    local_types: ctx.local_types.clone(),
                    materialized_scalar_local_slots: materialized_slots,
                    hoisted_scalar_vec_data_slots: hoisted_data_slots,
                    proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
                    definitely_materialized_top_level_scalar_names: ctx
                        .definitely_materialized_top_level_scalar_names,
                    proven_scalar_index_loads: &proven,
                    nonnegative_int_locals: ctx.nonnegative_int_locals,
                    tmp_i32: next_tmp_i32,
                };
                let body_and_drop = compile_expr_discarding_result(body_node, &nested_ctx)?;
                let loop_prelude = [materialize_once.as_str(), &hoist_parts.join("\n")]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                return Ok(format!(
                    "i32.const 0\n\
                     local.set {guard_tmp}\n\
                     {}\n\
                     local.get {guard_tmp}\n\
                     if (result i32)\n\
                       {generic_loop}\n\
                     else\n\
                       {loop_prelude}\n\
                       block\n\
                         loop\n\
                           local.get {idx_slot}\n\
                           i32.const {bound}\n\
                           i32.ge_s\n\
                           br_if 1\n\
                           {body_and_drop}\n\
                           br 0\n\
                         end\n\
                       end\n\
                       i32.const 0\n\
                     end",
                    guard_parts.join("\n")
                ));
            }
        }
    }
    compile_generic_while_loop(cond_node, body_node, ctx)
}

fn compile_fast_box_ctor(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>,
) -> Result<String, String> {
    let value_node = node
        .children
        .get(1)
        .ok_or_else(|| format!("{} requires exactly 1 argument", op))?;
    if node.children.len() != 2 {
        return Err(format!("{} requires exactly 1 argument", op));
    }
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: ctx.tmp_i32 + 2,
    };
    let value = compile_expr(value_node, &nested_ctx)?;
    let release_value = should_release_set_rhs(value_node, ctx.lambda_bindings);
    // Keep polymorphic `box` as ref-cell like generic lowering; typed scalar ctors stay scalar cells.
    let elem_ref = if op == "box" { 1 } else { 0 };
    let normalized_value = if op == "bool" {
        format!("{value}\ni32.const 0\ni32.ne")
    } else {
        value
    };
    let set_op = vec_set_runtime_for_scalar(elem_ref == 0);
    let vec_local = ctx.tmp_i32;
    let tmp_val = ctx.tmp_i32 + 1;
    if release_value {
        Ok(
            format!(
                "i32.const 0\ni32.const {elem_ref}\ncall $vec_new_i32\nlocal.set {vec_local}\nlocal.get {vec_local}\ni32.const 0\n{normalized_value}\nlocal.tee {tmp_val}\ncall {set_op}\ndrop\nlocal.get {tmp_val}\ncall {}\ndrop\nlocal.get {vec_local}",
                rc_release_for_opt_type(value_node.typ.as_ref())
            )
        )
    } else {
        Ok(
            format!(
                "i32.const 0\ni32.const {elem_ref}\ncall $vec_new_i32\nlocal.set {vec_local}\nlocal.get {vec_local}\ni32.const 0\n{normalized_value}\ncall {set_op}\ndrop\nlocal.get {vec_local}"
            )
        )
    }
}

fn compile_fast_cell_set(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    normalize_bool: bool,
) -> Result<String, String> {
    if node.children.len() != 3 {
        return Err(format!("{} requires exactly 2 arguments", op));
    }
    let cell_node = node
        .children
        .get(1)
        .ok_or_else(|| format!("{} missing cell", op))?;
    let value_node = node
        .children
        .get(2)
        .ok_or_else(|| format!("{} missing value", op))?;
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: ctx.tmp_i32 + 2,
    };
    let cell = compile_expr(cell_node, &nested_ctx)?;
    let value_raw = compile_expr(value_node, &nested_ctx)?;
    let value = if normalize_bool {
        format!("{value_raw}\ni32.const 0\ni32.ne")
    } else {
        value_raw
    };
    let release_rhs = should_release_set_rhs(value_node, ctx.lambda_bindings);
    let managed_slots = managed_local_slots(ctx);
    let cell_prefix = cell;
    let set_op = vec_set_runtime_for_scalar(
        cell_node
            .typ
            .as_ref()
            .and_then(|t| match t {
                Type::List(inner) => Some(!is_ref_type(inner)),
                _ => None,
            })
            .unwrap_or(false),
    );
    if release_rhs {
        let tmp_val = ctx.tmp_i32 + 1;
        let keep_tmp = ctx.tmp_i32 + 2;
        let release = emit_release_managed_temp_if_not_local_alias(
            tmp_val,
            keep_tmp,
            &managed_slots,
            value_node.typ.as_ref(),
        );
        Ok(format!(
            "{cell_prefix}\ni32.const 0\n{value}\nlocal.tee {tmp_val}\ncall {set_op}\n{}",
            release
        ))
    } else {
        Ok(format!(
            "{cell_prefix}\ni32.const 0\n{value}\ncall {set_op}"
        ))
    }
}

fn compile_fast_truthy(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>,
    negate: bool,
) -> Result<String, String> {
    if node.children.len() != 2 {
        return Err(format!("{} requires exactly 1 argument", op));
    }
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: ctx.tmp_i32 + 2,
    };
    let cell = compile_expr(
        node.children
            .get(1)
            .ok_or_else(|| format!("{} missing cell", op))?,
        &nested_ctx,
    )?;
    if negate {
        Ok(format!("{cell}\ni32.const 0\ncall $vec_get_i32\ni32.eqz"))
    } else {
        Ok(format!(
            "{cell}\ni32.const 0\ncall $vec_get_i32\ni32.const 0\ni32.ne"
        ))
    }
}

fn compile_fast_cell_helper(
    op: &str,
    node: &TypedExpression,
    ctx: &Ctx<'_>,
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
    let mut slots: Vec<usize> = ctx
        .locals
        .iter()
        .filter_map(|(name, slot)| {
            ctx.local_types
                .get(name)
                .filter(|typ| is_managed_local_type(typ))
                .map(|_| *slot)
        })
        .collect();
    slots.sort_unstable();
    slots.dedup();
    slots
}

fn emit_release_managed_temp_if_not_local_alias(
    tmp_val: usize,
    tmp_keep: usize,
    managed_local_slots: &[usize],
    ty: Option<&Type>,
) -> String {
    let debug_rc = cfg!(feature = "debug-rc");
    let release = rc_release_for_opt_type(ty);
    if managed_local_slots.is_empty() {
        if debug_rc {
            return format!(
                "global.get $dbg_tmp_release_exec\ni64.const 1\ni64.add\nglobal.set $dbg_tmp_release_exec\nlocal.get {tmp_val}\ncall {release}\ndrop\nlocal.get {tmp_val}\ncall $is_vec_ptr\nif\n  local.get {tmp_val}\n  i32.const 8\n  i32.add\n  i32.load\n  i32.const 1\n  i32.eq\n  if\n    global.get $dbg_tmp_release_post_rc_eq_1\n    i64.const 1\n    i64.add\n    global.set $dbg_tmp_release_post_rc_eq_1\n  else\n    global.get $dbg_tmp_release_post_rc_other\n    i64.const 1\n    i64.add\n    global.set $dbg_tmp_release_post_rc_other\n  end\nelse\n  global.get $dbg_tmp_release_post_not_vec\n  i64.const 1\n  i64.add\n  global.set $dbg_tmp_release_post_not_vec\nend"
            );
        }
        return format!("local.get {tmp_val}\ncall {release}\ndrop");
    }
    let mut out = Vec::new();
    out.push(format!("i32.const 0\nlocal.set {}", tmp_keep));
    for slot in managed_local_slots {
        out.push(format!(
            "local.get {}\nlocal.get {}\ni32.eq\nif\n  i32.const 1\n  local.set {}\nend",
            tmp_val, slot, tmp_keep
        ));
    }
    if debug_rc {
        out.push(
            format!(
                "local.get {}\ni32.eqz\nif\n  global.get $dbg_tmp_release_exec\n  i64.const 1\n  i64.add\n  global.set $dbg_tmp_release_exec\n  local.get {}\n  call {release}\n  drop\n  local.get {}\n  call $is_vec_ptr\n  if\n    local.get {}\n    i32.const 8\n    i32.add\n    i32.load\n    i32.const 1\n    i32.eq\n    if\n      global.get $dbg_tmp_release_post_rc_eq_1\n      i64.const 1\n      i64.add\n      global.set $dbg_tmp_release_post_rc_eq_1\n    else\n      global.get $dbg_tmp_release_post_rc_other\n      i64.const 1\n      i64.add\n      global.set $dbg_tmp_release_post_rc_other\n    end\n  else\n    global.get $dbg_tmp_release_post_not_vec\n    i64.const 1\n    i64.add\n    global.set $dbg_tmp_release_post_not_vec\n  end\nelse\n  global.get $dbg_tmp_release_skip\n  i64.const 1\n  i64.add\n  global.set $dbg_tmp_release_skip\nend",
                tmp_keep,
                tmp_val,
                tmp_val,
                tmp_val
            )
        );
    } else {
        out.push(format!(
            "local.get {}\ni32.eqz\nif\n  local.get {}\n  call {release}\n  drop\nend",
            tmp_keep, tmp_val
        ));
    }
    out.join("\n")
}

fn compile_extern_direct_call(
    op: &str,
    args: &[TypedExpression],
    ret_ty: &Type,
    ctx: &Ctx<'_>,
) -> Result<String, String> {
    let ret_managed = is_managed_local_type(ret_ty);
    let result_slot = ctx.tmp_i32;
    let first_arg_slot = ctx.tmp_i32 + usize::from(ret_managed);
    let eval_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: first_arg_slot + args.len(),
    };

    let mut out = Vec::new();
    let mut release_slots = Vec::new();
    for (idx, arg) in args.iter().enumerate() {
        let av = compile_expr(arg, &eval_ctx)?;
        if should_release_set_rhs(arg, ctx.lambda_bindings) {
            let slot = first_arg_slot + idx;
            out.push(format!("{av}\nlocal.tee {}", slot));
            release_slots.push((slot, rc_release_for_opt_type(arg.typ.as_ref())));
        } else {
            out.push(av);
        }
    }
    out.push(format!("call ${}", ident(op)));
    if ret_managed {
        out.push(format!("local.set {}", result_slot));
    }
    for (slot, release) in release_slots {
        out.push(format!("local.get {slot}\ncall {release}\ndrop"));
    }
    if ret_managed {
        out.push(format!("local.get {}", result_slot));
    }
    Ok(out.join("\n"))
}

fn contains_function_type(typ: &Type) -> bool {
    match typ {
        Type::Function(_, _) => true,
        Type::List(inner) => contains_function_type(inner),
        Type::Tuple(items) => items.iter().any(contains_function_type),
        _ => false,
    }
}

fn contains_unresolved_type(typ: &Type) -> bool {
    match typ {
        Type::Var(_) => true,
        Type::List(inner) => contains_unresolved_type(inner),
        Type::Tuple(items) => items.iter().any(contains_unresolved_type),
        Type::Function(arg, result) => {
            contains_unresolved_type(arg) || contains_unresolved_type(result)
        }
        _ => false,
    }
}

fn abi_type_descriptor(typ: &Type) -> String {
    match typ {
        Type::List(inner) => format!("[{}]", abi_type_descriptor(inner)),
        Type::Tuple(items) => format!(
            "{{{}}}",
            items
                .iter()
                .map(abi_type_descriptor)
                .collect::<Vec<_>>()
                .join(" * ")
        ),
        other => other.to_string(),
    }
}

fn emit_type_descriptor(typ: &Type, vec_slot: usize, data_slot: usize) -> String {
    let text = abi_type_descriptor(typ);
    let mut out = vec![format!(
        "i32.const {}\ni32.const 0\ncall $vec_new_i32\nlocal.set {vec_slot}\nlocal.get {vec_slot}\ni32.const 16\ni32.add\ni32.load\nlocal.set {data_slot}",
        text.chars().count()
    )];
    for (idx, ch) in text.chars().enumerate() {
        out.push(format!(
            "local.get {data_slot}\ni32.const {}\ni32.add\ni32.const {}\ni32.store",
            idx * 4,
            u32::from(ch)
        ));
    }
    out.push(format!("local.get {vec_slot}"));
    out.join("\n")
}

fn wasi_serde_name(typ: &Type) -> String {
    abi_type_descriptor(typ)
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn wasi_append_literal(out: &str, text: &str) -> String {
    text.chars()
        .map(|ch| {
            format!(
                "local.get {out}\ni32.const {}\ncall $vec_push_i32\ndrop",
                u32::from(ch)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn emit_wasi_serializer(typ: &Type, emitted: &mut HashSet<String>) -> Result<String, String> {
    let name = wasi_serde_name(typ);
    if !emitted.insert(name.clone()) {
        return Ok(String::new());
    }
    let function = match typ {
        Type::Int => format!("  (func $__wasi_serialize_{name} (param i32) (result i32)\n    local.get 0\n    call $__serde_int)\n"),
        Type::Dec => format!("  (func $__wasi_serialize_{name} (param i32) (result i32)\n    local.get 0\n    call $__serde_dec)\n"),
        Type::List(inner) if matches!(inner.as_ref(), Type::Char) => format!("  (func $__wasi_serialize_{name} (param i32) (result i32)\n    local.get 0\n    call $__serde_string)\n"),
        Type::Bool | Type::Unit | Type::Char => {
            let mut body = String::from("    (local $out i32) (local $tmp i32)\n    i32.const 0\n    i32.const 0\n    call $vec_new_i32\n    local.set $out\n");
            match typ {
                Type::Bool => body.push_str(&format!("    local.get 0\n    if\n{}\n    else\n{}\n    end\n", wasi_append_literal("$out", "true").replace('\n', "\n      "), wasi_append_literal("$out", "false").replace('\n', "\n      "))),
                Type::Unit => body.push_str(&format!("    {}\n", wasi_append_literal("$out", "nil").replace('\n', "\n    "))),
                Type::Char => body.push_str(&format!("    {}\n    local.get 0\n    call $__serde_int\n    local.set $tmp\n    local.get $out\n    local.get $tmp\n    call $__serde_append\n    local.get $tmp\n    call $rc_release_vec\n    drop\n    {}\n", wasi_append_literal("$out", "(char ").replace('\n', "\n    "), wasi_append_literal("$out", ")").replace('\n', "\n    "))),
                _ => unreachable!(),
            }
            body.push_str("    local.get $out)\n");
            format!("  (func $__wasi_serialize_{name} (param i32) (result i32)\n{body}")
        }
        Type::List(inner) => {
            let child = wasi_serde_name(inner);
            let nested = emit_wasi_serializer(inner, emitted)?;
            format!("{nested}  (func $__wasi_serialize_{name} (param $value i32) (result i32)\n    (local $out i32) (local $len i32) (local $data i32) (local $i i32) (local $tmp i32)\n    i32.const 0 i32.const 0 call $vec_new_i32 local.set $out\n    local.get $out i32.const 91 call $vec_push_i32 drop\n    local.get $value i32.load local.set $len\n    local.get $value i32.const 16 i32.add i32.load local.set $data\n    block $done loop $items\n      local.get $i local.get $len i32.ge_u br_if $done\n      local.get $i i32.const 0 i32.gt_u if local.get $out i32.const 32 call $vec_push_i32 drop end\n      local.get $data local.get $i i32.const 4 i32.mul i32.add i32.load call $__wasi_serialize_{child} local.set $tmp\n      local.get $out local.get $tmp call $__serde_append\n      local.get $tmp call $rc_release_vec drop\n      local.get $i i32.const 1 i32.add local.set $i br $items\n    end end\n    local.get $out i32.const 93 call $vec_push_i32 drop\n    local.get $out)\n")
        }
        Type::Tuple(parts) => {
            let mut nested = String::new();
            for part in parts { nested.push_str(&emit_wasi_serializer(part, emitted)?); }
            let mut fields = String::new();
            for (index, part) in parts.iter().enumerate() {
                fields.push_str(&format!("    local.get $data i32.const {} i32.add i32.load call $__wasi_serialize_{} local.set $tmp\n    local.get $out local.get $tmp call $__serde_append\n    local.get $tmp call $rc_release_vec drop\n", index * 4, wasi_serde_name(part)));
                if index + 1 < parts.len() { fields.push_str("    local.get $out i32.const 32 call $vec_push_i32 drop\n"); }
            }
            format!("{nested}  (func $__wasi_serialize_{name} (param $value i32) (result i32)\n    (local $out i32) (local $data i32) (local $tmp i32)\n    i32.const 0 i32.const 0 call $vec_new_i32 local.set $out\n    local.get $out i32.const 123 call $vec_push_i32 drop\n    local.get $out i32.const 32 call $vec_push_i32 drop\n    local.get $value i32.const 16 i32.add i32.load local.set $data\n{fields}    local.get $out i32.const 32 call $vec_push_i32 drop\n    local.get $out i32.const 125 call $vec_push_i32 drop\n    local.get $out)\n")
        }
        Type::Var(_) => format!("  (func $__wasi_serialize_{name} (param i32) (result i32)\n    local.get 0\n    call $__serde_int)\n"),
        Type::Function(_, _) => return Err("unsupported WASI serialization type".into()),
    };
    Ok(function)
}

fn collect_wasi_serialize_types(node: &TypedExpression, out: &mut Vec<Type>) {
    if let Expression::Apply(items) = &node.expr {
        if matches!(items.first(), Some(Expression::Word(op)) if op == "serialize") {
            if let Some(typ) = node.children.get(1).and_then(|arg| arg.typ.clone()) {
                if !out.iter().any(|existing| existing == &typ) {
                    out.push(typ);
                }
            }
        }
    }
    for child in &node.children {
        collect_wasi_serialize_types(child, out);
    }
}

fn wasi_expect_literal(text: &str) -> String {
    let mut out = String::from("call $__deser_ws\n");
    for ch in text.chars() {
        out.push_str(&format!(
            "call $__deser_get\ni32.const {}\ni32.ne\nif unreachable end\n",
            u32::from(ch)
        ));
    }
    out
}

fn emit_wasi_deserializer(typ: &Type, emitted: &mut HashSet<String>) -> Result<String, String> {
    let name = wasi_serde_name(typ);
    if !emitted.insert(name.clone()) {
        return Ok(String::new());
    }
    let function = match typ {
        Type::Int => format!("  (func $__wasi_deserialize_{name} (result i32) call $__deser_int)\n"),
        Type::Dec => format!("  (func $__wasi_deserialize_{name} (result i32) call $__deser_dec)\n"),
        Type::List(inner) if matches!(inner.as_ref(), Type::Char) => format!("  (func $__wasi_deserialize_{name} (result i32) call $__deser_string)\n"),
        Type::Bool => format!("  (func $__wasi_deserialize_{name} (result i32)\n    call $__deser_ws\n    call $__deser_peek\n    i32.const 116\n    i32.eq\n    if (result i32)\n{}      i32.const 1\n    else\n{}      i32.const 0\n    end)\n", wasi_expect_literal("true").replace('\n', "\n      "), wasi_expect_literal("false").replace('\n', "\n      ")),
        Type::Unit => format!("  (func $__wasi_deserialize_{name} (result i32)\n    {}    i32.const 0)\n", wasi_expect_literal("nil").replace('\n', "\n    ")),
        Type::Char => format!("  (func $__wasi_deserialize_{name} (result i32)\n    {}    call $__deser_int\n    i32.const 41\n    call $__deser_expect)\n", wasi_expect_literal("(char ").replace('\n', "\n    ")),
        Type::List(inner) => {
            let child = wasi_serde_name(inner);
            let nested = emit_wasi_deserializer(inner, emitted)?;
            let elem_ref = i32::from(is_ref_type(inner));
            let release = if is_ref_type(inner) { "local.get $item\n      call $rc_release\n      drop" } else { "" };
            format!("{nested}  (func $__wasi_deserialize_{name} (result i32)\n    (local $out i32) (local $item i32)\n    i32.const 91 call $__deser_expect\n    i32.const 0 i32.const {elem_ref} call $vec_new_i32 local.set $out\n    block $done loop $items\n      call $__deser_ws\n      call $__deser_peek i32.const 93 i32.eq br_if $done\n      call $__wasi_deserialize_{child} local.set $item\n      local.get $out local.get $item call $vec_push_i32 drop\n      {release}\n      br $items\n    end end\n    i32.const 93 call $__deser_expect\n    local.get $out)\n")
        }
        Type::Tuple(parts) => {
            let mut nested = String::new();
            for part in parts { nested.push_str(&emit_wasi_deserializer(part, emitted)?); }
            let mut fields = String::new();
            for part in parts {
                fields.push_str(&format!("    call $__wasi_deserialize_{} local.set $item\n    local.get $out local.get $item call $vec_push_i32 drop\n", wasi_serde_name(part)));
                if is_ref_type(part) { fields.push_str("    local.get $item call $rc_release drop\n"); }
            }
            format!("{nested}  (func $__wasi_deserialize_{name} (result i32)\n    (local $out i32) (local $item i32)\n    i32.const 123 call $__deser_expect\n    i32.const 0 i32.const 1 call $vec_new_i32 local.set $out\n{fields}    i32.const 125 call $__deser_expect\n    local.get $out)\n")
        }
        Type::Var(_) | Type::Function(_, _) => return Err("unsupported WASI deserialization type".into()),
    };
    Ok(function)
}

fn collect_wasi_deserialize_types(node: &TypedExpression, out: &mut Vec<Type>) {
    if let Expression::Apply(items) = &node.expr {
        if matches!(items.first(), Some(Expression::Word(op)) if op == "deserialize") {
            if let Some(typ) = node.typ.clone() {
                if !out.iter().any(|existing| existing == &typ) {
                    out.push(typ);
                }
            }
        }
    }
    for child in &node.children {
        collect_wasi_deserialize_types(child, out);
    }
}

fn compile_serde_call(node: &TypedExpression, op: &str, ctx: &Ctx<'_>) -> Result<String, String> {
    let arg = node
        .children
        .get(1)
        .ok_or_else(|| format!("{op} requires exactly one argument"))?;
    if node.children.len() != 2 {
        return Err(format!("{op} requires exactly one argument"));
    }
    let inferred_value_type = if op == "serialize" {
        arg.typ
            .as_ref()
            .ok_or_else(|| "serialize argument is missing its inferred type".to_string())?
    } else {
        node.typ
            .as_ref()
            .ok_or_else(|| "deserialize result is missing its inferred type".to_string())?
    };
    let resolved_binding_type =
        if op == "serialize" && contains_unresolved_type(inferred_value_type) {
            match &arg.expr {
                Expression::Word(name) => ctx
                    .local_types
                    .get(name)
                    .filter(|typ| !contains_unresolved_type(typ)),
                _ => None,
            }
        } else {
            None
        };
    let value_type = resolved_binding_type.unwrap_or(inferred_value_type);
    if contains_function_type(value_type) {
        return Err(format!("{op} does not support function values"));
    }
    if contains_unresolved_type(value_type) {
        if op == "serialize" {
            return Err("serialize requires a concrete value type".to_string());
        }
    }

    let arg_slot = ctx.tmp_i32;
    let type_slot = ctx.tmp_i32 + 1;
    let type_data_slot = ctx.tmp_i32 + 2;
    let result_slot = ctx.tmp_i32 + 3;
    let nested_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        extern_names: ctx.extern_names,
        lambda_ids: ctx.lambda_ids,
        closure_defs: ctx.closure_defs,
        lambda_bindings: ctx.lambda_bindings,
        current_function: ctx.current_function,
        locals: ctx.locals.clone(),
        local_types: ctx.local_types.clone(),
        materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
        hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
        proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
        definitely_materialized_top_level_scalar_names: ctx
            .definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: ctx.proven_scalar_index_loads,
        nonnegative_int_locals: ctx.nonnegative_int_locals,
        tmp_i32: ctx.tmp_i32 + 4,
    };
    let arg_code = compile_expr(arg, &nested_ctx)?;
    let wasi_host = wasi_bool("QUE_WASI_HOST");
    if wasi_host && op == "serialize" {
        let release_arg = should_release_set_rhs(arg, ctx.lambda_bindings);
        let mut out = vec![
            format!("{arg_code}\nlocal.set {arg_slot}"),
            format!(
                "local.get {arg_slot}\ncall $__wasi_serialize_{}\nlocal.set {result_slot}",
                wasi_serde_name(value_type)
            ),
        ];
        if release_arg {
            out.push(format!(
                "local.get {arg_slot}\ncall {}\ndrop",
                rc_release_for_opt_type(arg.typ.as_ref())
            ));
        }
        out.push(format!("local.get {result_slot}"));
        return Ok(out.join("\n"));
    }
    if wasi_host && op == "deserialize" {
        let release_arg = should_release_set_rhs(arg, ctx.lambda_bindings);
        let mut out = vec![
            format!("{arg_code}\nlocal.set {arg_slot}"),
            format!(
                "local.get {arg_slot}\ncall $__deser_begin\ncall $__wasi_deserialize_{}\nlocal.set {result_slot}\ncall $__deser_ws\ncall $__deser_peek\ni32.const -1\ni32.ne\nif unreachable end",
                wasi_serde_name(value_type)
            ),
        ];
        if release_arg {
            out.push(format!("local.get {arg_slot}\ncall $rc_release_vec\ndrop"));
        }
        out.push(format!("local.get {result_slot}"));
        return Ok(out.join("\n"));
    }
    let type_code = emit_type_descriptor(value_type, type_slot, type_data_slot);
    let host_name = if op == "serialize" {
        "$__que_serialize"
    } else {
        "$__que_deserialize"
    };
    let release_arg = should_release_set_rhs(arg, ctx.lambda_bindings);
    let mut out = vec![
        format!("{arg_code}\nlocal.set {arg_slot}"),
        format!("{type_code}\nlocal.set {type_slot}"),
        format!(
            "local.get {arg_slot}\nlocal.get {type_slot}\ncall {host_name}\nlocal.set {result_slot}"
        ),
        format!("local.get {type_slot}\ncall $rc_release_vec\ndrop"),
    ];
    if release_arg {
        out.push(format!(
            "local.get {arg_slot}\ncall {}\ndrop",
            rc_release_for_opt_type(arg.typ.as_ref())
        ));
    }
    out.push(format!("local.get {result_slot}"));
    Ok(out.join("\n"))
}

fn compile_call(node: &TypedExpression, op: &str, ctx: &Ctx<'_>) -> Result<String, String> {
    if op == "serialize" || op == "deserialize" {
        return compile_serde_call(node, op, ctx);
    }
    if let Some(fast) = compile_fast_cell_helper(op, node, ctx) {
        return fast;
    }
    let (params, ret_ty) = if let Some(sig) = ctx.fn_sigs.get(op) {
        sig.clone()
    } else if builtin_fn_tag(op).is_some() {
        // A builtin may have no top-level library definition. Recover its concrete
        // instantiated signature from the typed call head so direct and partial
        // applications do not try to call a generated `$v_<name>` function.
        let head_ty = node
            .children
            .first()
            .and_then(|head| head.typ.as_ref())
            .ok_or_else(|| format!("Builtin '{}' is missing its inferred type", op))?;
        let (params, ret) = function_parts(head_ty);
        (params, ret.clone())
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
            let helper_id = *ctx
                .fn_ids
                .get(&helper_name)
                .ok_or_else(|| format!("Missing dynamic partial helper '{}'", helper_name))?;
            let clo_local = ctx.tmp_i32;
            let tmp_local = ctx.tmp_i32 + 1;
            let mut out = Vec::new();
            out.push(format!(
                "i32.const {}\ni32.const {}\ncall $closure_new\nlocal.set {}",
                helper_id,
                1 + provided,
                clo_local
            ));
            out.push(format!(
                "local.get {}\ni32.const 0\ncall ${}\ncall $closure_set_fun\ndrop",
                clo_local,
                ident(op)
            ));
            for (i, arg) in args.iter().enumerate() {
                let nested_ctx = Ctx {
                    fn_sigs: ctx.fn_sigs,
                    fn_ids: ctx.fn_ids,
                    extern_names: ctx.extern_names,
                    lambda_ids: ctx.lambda_ids,
                    closure_defs: ctx.closure_defs,
                    lambda_bindings: ctx.lambda_bindings,
                    current_function: ctx.current_function,
                    locals: ctx.locals.clone(),
                    local_types: ctx.local_types.clone(),
                    materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
                    hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                    proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
                    definitely_materialized_top_level_scalar_names: ctx
                        .definitely_materialized_top_level_scalar_names,
                    proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                    nonnegative_int_locals: ctx.nonnegative_int_locals,
                    tmp_i32: ctx.tmp_i32 + 2,
                };
                let av = compile_expr(arg, &nested_ctx)?;
                let idx = i + 1;
                let store_op = closure_store_op_for_type_wat(&ret_params[i]);
                let release_arg = should_release_set_rhs(arg, ctx.lambda_bindings);
                if store_op != "$closure_set" {
                    if release_arg {
                        out.push(
                            format!(
                                "local.get {}\ni32.const {}\n{}\nlocal.tee {}\ncall {}\ndrop\nlocal.get {}\ncall {}\ndrop",
                                clo_local,
                                idx,
                                av,
                                tmp_local,
                                store_op,
                                tmp_local,
                                rc_release_for_opt_type(arg.typ.as_ref())
                            )
                        );
                    } else {
                        out.push(format!(
                            "local.get {}\ni32.const {}\n{}\ncall {}\ndrop",
                            clo_local, idx, av, store_op
                        ));
                    }
                } else {
                    out.push(format!(
                        "local.get {}\ni32.const {}\n{}\ncall $closure_set\ndrop",
                        clo_local, idx, av
                    ));
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
        let helper_id = *ctx
            .fn_ids
            .get(&helper_name)
            .ok_or_else(|| format!("Missing dynamic partial helper '{}'", helper_name))?;
        let fn_ptr = if let Some(fid) = ctx.fn_ids.get(op) {
            format!("i32.const {}", fid)
        } else if let Some(tag) = builtin_fn_tag(op) {
            format!("i32.const {}", tag)
        } else {
            return Err(format!(
                "Partial application requires function id/tag for '{}', but none was found",
                op
            ));
        };
        let clo_local = ctx.tmp_i32;
        let tmp_local = ctx.tmp_i32 + 1;
        let mut out = Vec::new();
        out.push(format!(
            "i32.const {}\ni32.const {}\ncall $closure_new\nlocal.set {}",
            helper_id,
            1 + provided,
            clo_local
        ));
        out.push(format!(
            "local.get {}\ni32.const 0\n{}\ncall $closure_set_fun\ndrop",
            clo_local, fn_ptr
        ));
        for (i, arg) in args.iter().enumerate() {
            let nested_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                extern_names: ctx.extern_names,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: ctx.lambda_bindings,
                current_function: ctx.current_function,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
                hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
                definitely_materialized_top_level_scalar_names: ctx
                    .definitely_materialized_top_level_scalar_names,
                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                nonnegative_int_locals: ctx.nonnegative_int_locals,
                tmp_i32: ctx.tmp_i32 + 2,
            };
            let av = compile_expr(arg, &nested_ctx)?;
            let idx = i + 1;
            let store_op = closure_store_op_for_type_wat(&params[i]);
            let release_arg = should_release_set_rhs(arg, ctx.lambda_bindings);
            if store_op != "$closure_set" {
                if release_arg {
                    out.push(
                        format!(
                            "local.get {}\ni32.const {}\n{}\nlocal.tee {}\ncall {}\ndrop\nlocal.get {}\ncall {}\ndrop",
                            clo_local,
                            idx,
                            av,
                            tmp_local,
                            store_op,
                            tmp_local,
                            rc_release_for_opt_type(arg.typ.as_ref())
                        )
                    );
                } else {
                    out.push(format!(
                        "local.get {}\ni32.const {}\n{}\ncall {}\ndrop",
                        clo_local, idx, av, store_op
                    ));
                }
            } else {
                out.push(format!(
                    "local.get {}\ni32.const {}\n{}\ncall $closure_set\ndrop",
                    clo_local, idx, av
                ));
            }
        }
        out.push(format!("local.get {}", clo_local));
        return Ok(out.join("\n"));
    }
    if args.len() > params.len() && !unit_arity_elided {
        let (initial_args, rest_args) = args.split_at(params.len());
        if ctx.extern_names.contains(op) {
            let mut out = vec![compile_extern_direct_call(op, initial_args, &ret_ty, ctx)?];
            for arg in rest_args {
                out.push(compile_expr(arg, ctx)?);
                out.push("call $apply1_i32".to_string());
            }
            return Ok(out.join("\n"));
        }
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
    if ctx.extern_names.contains(op) {
        return compile_extern_direct_call(op, args, &ret_ty, ctx);
    }

    if let Some(tag) = builtin_fn_tag(op) {
        let mut out = vec![format!("i32.const {}", tag)];
        for arg in args {
            out.push(compile_expr(arg, ctx)?);
        }
        out.push(format!("call $apply{}_i32", args.len()));
        return Ok(out.join("\n"));
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
    let f_node = node
        .children
        .first()
        .ok_or_else(|| "call missing function".to_string())?;
    let f = compile_expr(f_node, ctx)?;
    let args = &node.children[1..];
    let head_ty = f_node
        .typ
        .as_ref()
        .ok_or_else(|| "dynamic call head missing type".to_string())?;
    let (head_params, _head_ret) = function_parts(head_ty);
    if args.is_empty() {
        // Zero-arg invocation of a function value (e.g. local thunk).
        return Ok(format!("{f}\ncall $apply0_i32"));
    }
    if !head_params.is_empty() && args.len() < head_params.len() {
        let total = head_params.len();
        let provided = args.len();
        let helper_name = format!("__partial_dyn_{}_{}", total, provided);
        let helper_id = *ctx
            .fn_ids
            .get(&helper_name)
            .ok_or_else(|| format!("Missing dynamic partial helper '{}'", helper_name))?;
        let clo_local = ctx.tmp_i32;
        let tmp_local = ctx.tmp_i32 + 1;
        let mut out = Vec::new();
        out.push(format!(
            "i32.const {}\ni32.const {}\ncall $closure_new\nlocal.set {}",
            helper_id,
            1 + provided,
            clo_local
        ));
        out.push(format!(
            "local.get {}\ni32.const 0\n{}\ncall $closure_set_fun\ndrop",
            clo_local, f
        ));
        for (i, arg) in args.iter().enumerate() {
            let nested_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                extern_names: ctx.extern_names,
                lambda_ids: ctx.lambda_ids,
                closure_defs: ctx.closure_defs,
                lambda_bindings: ctx.lambda_bindings,
                current_function: ctx.current_function,
                locals: ctx.locals.clone(),
                local_types: ctx.local_types.clone(),
                materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
                hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
                definitely_materialized_top_level_scalar_names: ctx
                    .definitely_materialized_top_level_scalar_names,
                proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                nonnegative_int_locals: ctx.nonnegative_int_locals,
                tmp_i32: ctx.tmp_i32 + 2,
            };
            let av = compile_expr(arg, &nested_ctx)?;
            let idx = i + 1;
            let store_op = closure_store_op_for_type_wat(&head_params[i]);
            let release_arg = should_release_set_rhs(arg, ctx.lambda_bindings);
            if store_op != "$closure_set" {
                if release_arg {
                    out.push(
                        format!(
                            "local.get {}\ni32.const {}\n{}\nlocal.tee {}\ncall {}\ndrop\nlocal.get {}\ncall {}\ndrop",
                            clo_local,
                            idx,
                            av,
                            tmp_local,
                            store_op,
                            tmp_local,
                            rc_release_for_opt_type(arg.typ.as_ref())
                        )
                    );
                } else {
                    out.push(format!(
                        "local.get {}\ni32.const {}\n{}\ncall {}\ndrop",
                        clo_local, idx, av, store_op
                    ));
                }
            } else {
                out.push(format!(
                    "local.get {}\ni32.const {}\n{}\ncall $closure_set\ndrop",
                    clo_local, idx, av
                ));
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
    ctx: &Ctx<'_>,
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
    Ok(ctx.fn_ids.iter().find_map(|(name, id)| {
        if *id == target_id {
            Some(name.clone())
        } else {
            None
        }
    }))
}

fn compile_capture_value(cap: &str, ctx: &Ctx<'_>) -> Result<(String, &'static str), String> {
    if let Some(local_idx) = ctx.locals.get(cap) {
        let local_ty = ctx.local_types.get(cap);
        Ok((
            format!("local.get {}", local_idx),
            if matches!(local_ty, Some(Type::Function(_, _))) {
                "$closure_set_fun"
            } else if local_ty.map(is_managed_local_type).unwrap_or(false) {
                "$closure_set_ref"
            } else {
                "$closure_set"
            },
        ))
    } else if cap == "ARGV" {
        Ok(("call $__argv_get".to_string(), "$closure_set_ref"))
    } else if let Some((ps, ret)) = ctx.fn_sigs.get(cap) {
        if ps.is_empty() {
            Ok((
                format!("call ${}", ident(cap)),
                if is_managed_local_type(ret) {
                    "$closure_set_ref"
                } else {
                    "$closure_set"
                },
            ))
        } else if let Some(id) = ctx.fn_ids.get(cap) {
            Ok((format!("i32.const {}", id), "$closure_set_fun"))
        } else if let Some(tag) = builtin_fn_tag(cap) {
            Ok((format!("i32.const {}", tag), "$closure_set_fun"))
        } else {
            Err(format!(
                "Unsupported closure capture '{}' in wasm backend (no function id/tag)",
                cap
            ))
        }
    } else if let Some(tag) = builtin_fn_tag(cap) {
        Ok((format!("i32.const {}", tag), "$closure_set_fun"))
    } else {
        Err(format!(
            "Unsupported closure capture '{}' in wasm backend",
            cap
        ))
    }
}

fn compile_direct_local_closure_call(
    node: &TypedExpression,
    local_head: &str,
    ctx: &Ctx<'_>,
) -> Result<Option<String>, String> {
    let mode = devirtualize_mode_from_env()?;
    if mode == DevirtualizeMode::Off {
        return Ok(None);
    }
    let Some(lambda_node) = ctx.lambda_bindings.get(local_head) else {
        return Ok(None);
    };
    let key = lambda_node.expr.to_lisp();
    let Some(def) = ctx.closure_defs.get(&key) else {
        return Ok(None);
    };
    let args = &node.children[1..];
    if args.len() != def.user_arity {
        return Ok(None);
    }

    let mut out = Vec::new();
    for cap in &def.captures {
        out.push(compile_capture_value(cap, ctx)?.0);
    }
    for arg in args {
        out.push(compile_expr(arg, ctx)?);
    }
    out.push(format!("call ${}", ident(&def.name)));
    Ok(Some(out.join("\n")))
}

fn compile_lambda_literal(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let key = node.expr.to_lisp();
    if let Some(id) = ctx.lambda_ids.get(&key) {
        Ok(format!("i32.const {}", id))
    } else if let Some(def) = ctx.closure_defs.get(&key) {
        let fn_id = ctx
            .fn_ids
            .get(&def.name)
            .ok_or_else(|| format!("Missing function id for closure '{}'", def.name))?;
        let clo_local = ctx.tmp_i32;
        let mut out = Vec::new();
        out.push(format!(
            "i32.const {}\ni32.const {}\ncall $closure_new\nlocal.set {}",
            fn_id,
            def.captures.len(),
            clo_local
        ));
        for (i, cap) in def.captures.iter().enumerate() {
            let (cap_v, set_fn) = compile_capture_value(cap, ctx)?;
            out.push(format!(
                "local.get {}\ni32.const {}\n{}\ncall {}\ndrop",
                clo_local, i, cap_v, set_fn
            ));
        }
        out.push(format!("local.get {}", clo_local));
        Ok(out.join("\n"))
    } else {
        Err(format!(
            "Unsupported lambda literal in wasm backend (missing lowering id): {}",
            key
        ))
    }
}

fn compile_expr(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    match &node.expr {
        Expression::Int(n) => Ok(format!("i32.const {}", n)),
        Expression::Dec(n) => Ok(format!("i32.const {}", decimal_literal_i32(n)?)),
        Expression::Word(w) => match w.as_str() {
            "true" => Ok("i32.const 1".to_string()),
            "false" => Ok("i32.const 0".to_string()),
            "nil" => Ok("i32.const 0".to_string()),
            _ => {
                if let Some(local_idx) = ctx.locals.get(w) {
                    Ok(format!("local.get {}", local_idx))
                } else if w == "ARGV" {
                    Ok("call $__argv_get".to_string())
                } else if let Some(borrowed) =
                    compile_borrowed_top_level_cached_ref(w, ctx, ctx.tmp_i32)
                {
                    Ok(borrowed)
                } else if let Some((params, _ret)) = ctx.fn_sigs.get(w) {
                    if params.is_empty() {
                        Ok(format!("call ${}", ident(w)))
                    } else if let Some(id) = ctx.fn_ids.get(w) {
                        Ok(format!("i32.const {}", id))
                    } else {
                        Err(format!(
                            "Unsupported function-valued word in wasm backend: '{}'",
                            w
                        ))
                    }
                } else if let Some(tag) = builtin_fn_tag(w) {
                    Ok(format!("i32.const {}", tag))
                } else {
                    Err(format!("Unsupported free word in wasm backend: '{}'", w))
                }
            }
        },
        Expression::Apply(items) => {
            if items.is_empty() {
                return Ok("i32.const 0".to_string());
            }
            match &items[0] {
                Expression::Word(op) => {
                    let op_full = op.as_str();
                    match op_full {
                        _ if ctx.locals.contains_key(op_full) => {
                            if let Some(target_name) =
                                resolve_local_devirtualized_head(op_full, ctx)?
                            {
                                compile_call(node, &target_name, ctx)
                            } else if let Some(call) =
                                compile_direct_local_closure_call(node, op_full, ctx)?
                            {
                                Ok(call)
                            } else {
                                compile_dynamic_call(node, ctx)
                            }
                        }
                        "lambda" => compile_lambda_literal(node, ctx),
                        "do" => compile_do(items, node, ctx),
                        "if" => compile_if(node, ctx),
                        "tuple" => compile_tuple(node, ctx),
                        "vector" | "string" => compile_vector_literal(node, ctx),
                        "__vec_new_zeroed_i32" => {
                            let len = compile_expr(
                                node.children.get(1).ok_or_else(|| {
                                    "__vec_new_zeroed_i32 missing len".to_string()
                                })?,
                                ctx,
                            )?;
                            Ok(format!("{len}\ncall $vec_new_zeroed_i32"))
                        }
                        "__vec_new_uninit_i32" => {
                            let len = compile_expr(
                                node.children.get(1).ok_or_else(|| {
                                    "__vec_new_uninit_i32 missing len".to_string()
                                })?,
                                ctx,
                            )?;
                            Ok(format!("{len}\ncall $vec_new_uninit_i32"))
                        }
                        "__vec_store_i32" => {
                            let xs = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "__vec_store_i32 missing vector".to_string())?,
                                ctx,
                            )?;
                            let idx = compile_expr(
                                node.children
                                    .get(2)
                                    .ok_or_else(|| "__vec_store_i32 missing index".to_string())?,
                                ctx,
                            )?;
                            let value = compile_expr(
                                node.children
                                    .get(3)
                                    .ok_or_else(|| "__vec_store_i32 missing value".to_string())?,
                                ctx,
                            )?;
                            Ok(emit_direct_builder_scalar_store_i32(&xs, &idx, &value, ctx))
                        }
                        "extern" | "letype" => Ok("i32.const 0".to_string()),
                        "integers" | "bools" | "decimals" | "strings" => {
                            compile_trusted_typed_vector_literal(op_full, node, ctx)
                        }
                        "length" => {
                            let a = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "length missing arg".to_string())?,
                                ctx,
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
                                ctx,
                            )?;
                            let elem = match node.typ.as_ref() {
                                Some(t) => vec_elem_kind_from_type(t)?,
                                None => {
                                    return Err("car missing return type".to_string());
                                }
                            };
                            if static_proof_is_safe(
                                crate::static_analysis::ProofKind::NonEmpty,
                                &node.expr,
                            ) || !parse_env_bool_like("QUE_BOUNDS_CHECK", true)
                            {
                                return Ok(format!(
                                    "{xs}\ni32.const 16\ni32.add\ni32.load\ni32.load"
                                ));
                            }
                            Ok(format!(
                                "{xs}\ni32.const 0\ncall $vec_get_{}",
                                elem.suffix()
                            ))
                        }
                        "cdr" => compile_cdr(node, ctx),
                        "set!" => compile_set(node, ctx),
                        "alter!" => compile_alter(node, ctx),
                        "pop!" => compile_pop(node, ctx),
                        "pop-val!" => compile_pop_val(node, ctx),
                        "while" => compile_loop_while(node, ctx),
                        "not" => {
                            let a = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "not missing arg".to_string())?,
                                ctx,
                            )?;
                            Ok(format!("{a}\ni32.eqz"))
                        }
                        "~" => {
                            let a = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "~ missing arg".to_string())?,
                                ctx,
                            )?;
                            // Bitwise NOT for i32.
                            Ok(format!("{a}\ni32.const -1\ni32.xor"))
                        }
                        "Int->Dec" => {
                            let a = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "Int->Dec missing arg".to_string())?,
                                ctx,
                            )?;
                            Ok(format!("{a}\ncall $dec_from_int"))
                        }
                        "Dec->Int" => {
                            let a = compile_expr(
                                node.children
                                    .get(1)
                                    .ok_or_else(|| "Dec->Int missing arg".to_string())?,
                                ctx,
                            )?;
                            Ok(format!("{a}\ncall $dec_to_int"))
                        }
                        "as" | "char" => node
                            .children
                            .get(1)
                            .map(|n| compile_expr(n, ctx))
                            .unwrap_or_else(|| Ok("i32.const 0".to_string())),
                        op if builtin_fn_tag(op)
                            .and_then(builtin_tag_arity)
                            .map(|arity| node.children.len().saturating_sub(1) != arity)
                            .unwrap_or(false) =>
                        {
                            compile_dynamic_call(node, ctx)
                        }
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
        if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
            return;
        }
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

fn collect_current_scope_let_locals(node: &TypedExpression, out: &mut Vec<(String, Type)>) {
    let Expression::Apply(items) = &node.expr else {
        return;
    };
    if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
        return;
    }
    if let [Expression::Word(kw), Expression::Word(name), _] = &items[..] {
        if kw == "let" || kw == "letrec" || kw == "mut" {
            if let Some(t) = node.children.get(2).and_then(|n| n.typ.as_ref()) {
                if !out.iter().any(|(n, _)| n == name) {
                    out.push((name.clone(), t.clone()));
                }
            }
            return;
        }
    }
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        return;
    }
    let child_offset = if node.children.len() + 1 == items.len() {
        1
    } else {
        0
    };
    for idx in 1..items.len() {
        if let Some(child) = idx
            .checked_sub(child_offset)
            .and_then(|child_idx| node.children.get(child_idx))
        {
            if matches!(&child.expr, Expression::Apply(child_items) if matches!(child_items.first(), Some(Expression::Word(w)) if w == "do"))
            {
                collect_current_scope_let_locals(child, out);
                continue;
            }
        }
        if let Expression::Apply(let_items) = &items[idx] {
            if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                if kw == "let" || kw == "letrec" || kw == "mut" {
                    if let Some(child) = idx
                        .checked_sub(child_offset)
                        .and_then(|child_idx| node.children.get(child_idx))
                    {
                        if let Some(t) = child.children.get(2).and_then(|n| n.typ.as_ref()) {
                            if !out.iter().any(|(n, _)| n == name) {
                                out.push((name.clone(), t.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn projection_root_name(expr: &Expression) -> Option<&str> {
    match expr {
        Expression::Word(name) => Some(name),
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if matches!(op.as_str(), "fst" | "snd" | "get" | "car" | "cdr")) => {
            items.get(1).and_then(projection_root_name)
        }
        _ => None,
    }
}

fn expr_returns_name(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::Word(word) => word == name,
        Expression::Apply(items) => match items.first() {
            Some(Expression::Word(op)) if matches!(op.as_str(), "do" | "block") => items
                .last()
                .is_some_and(|last| expr_returns_name(last, name)),
            Some(Expression::Word(op)) if op == "if" => items
                .iter()
                .skip(2)
                .any(|branch| expr_returns_name(branch, name)),
            Some(Expression::Word(op)) if op == "cond" => items
                .iter()
                .skip(2)
                .step_by(2)
                .any(|branch| expr_returns_name(branch, name)),
            _ => false,
        },
        _ => false,
    }
}

fn expr_captures_name(expr: &Expression, name: &str) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda") {
        return items
            .last()
            .is_some_and(|body| expr_uses_name_as_value(name, body, false));
    }
    items.iter().any(|item| expr_captures_name(item, name))
}

fn collect_projection_bindings<'a>(
    node: &'a TypedExpression,
    out: &mut Vec<(&'a str, &'a TypedExpression)>,
) {
    if let Expression::Apply(items) = &node.expr {
        if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda") {
            return;
        }
        if let [Expression::Word(kw), Expression::Word(name), ..] = &items[..] {
            if kw == "let" {
                if let Some(rhs) = node.children.get(2) {
                    out.push((name, rhs));
                }
            }
        }
    }
    for child in &node.children {
        collect_projection_bindings(child, out);
    }
}

fn borrowed_projection_names(body: &TypedExpression, params: &[(String, Type)]) -> HashSet<String> {
    let mut live_roots = params
        .iter()
        .filter(|(_, typ)| is_managed_local_type(typ))
        .map(|(name, _)| name.clone())
        .collect::<HashSet<_>>();
    let mut borrowed = HashSet::new();
    let mut bindings = Vec::new();
    collect_projection_bindings(body, &mut bindings);
    for (name, rhs) in bindings {
        let concrete_managed = rhs
            .typ
            .as_ref()
            .is_some_and(|typ| is_managed_local_type(typ) && !matches!(typ, Type::Var(_)));
        let projection_from_live_root = projection_root_name(&rhs.expr)
            .is_some_and(|root| live_roots.contains(root) && root != name);
        if concrete_managed
            && projection_from_live_root
            && !expr_returns_name(&body.expr, name)
            && !expr_captures_name(&body.expr, name)
        {
            borrowed.insert(name.to_string());
            live_roots.insert(name.to_string());
        }
    }
    borrowed
}

fn is_borrowed_projection_local(name: &str, ctx: &Ctx<'_>) -> bool {
    ctx.locals
        .contains_key(&format!("__borrowed_projection::{name}"))
}

#[derive(Clone, Debug)]
enum CallSpecialization {
    Unique(Vec<Type>, Type),
    Conflicting,
}

fn record_call_specialization(
    name: &str,
    params: Vec<Type>,
    ret: Type,
    out: &mut HashMap<String, CallSpecialization>,
) {
    if params.iter().any(contains_unresolved_type) || contains_unresolved_type(&ret) {
        return;
    }
    match out.get(name) {
        None => {
            out.insert(name.to_string(), CallSpecialization::Unique(params, ret));
        }
        Some(CallSpecialization::Unique(prev_params, prev_ret))
            if *prev_params == params && *prev_ret == ret => {}
        Some(CallSpecialization::Unique(_, _)) => {
            out.insert(name.to_string(), CallSpecialization::Conflicting);
        }
        Some(CallSpecialization::Conflicting) => {}
    }
}

fn collect_call_specializations(
    node: &TypedExpression,
    top_def_names: &HashSet<String>,
    out: &mut HashMap<String, CallSpecialization>,
) {
    // The inferred type on a function-valued word represents the complete
    // instantiation, including uses through higher-order functions and partial
    // application. Recording those uses makes whole-program specialization
    // safe: a definition is specialized only when every concrete use agrees.
    if let Expression::Word(name) = &node.expr {
        if top_def_names.contains(name) {
            if let Some(typ @ Type::Function(_, _)) = node.typ.as_ref() {
                let (params, ret) = function_parts(typ);
                record_call_specialization(name, params, ret, out);
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
    out: &mut HashSet<(usize, usize)>,
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
        Type::Var(v) => subst
            .get(&v.id)
            .cloned()
            .unwrap_or_else(|| Type::Var(v.clone())),
        Type::List(a) => Type::List(Box::new(apply_type_subst(a, subst))),
        Type::Tuple(xs) => Type::Tuple(xs.iter().map(|x| apply_type_subst(x, subst)).collect()),
        Type::Function(a, b) => Type::Function(
            Box::new(apply_type_subst(a, subst)),
            Box::new(apply_type_subst(b, subst)),
        ),
        _ => t.clone(),
    }
}

fn specialize_typed_expr(node: &TypedExpression, subst: &HashMap<u64, Type>) -> TypedExpression {
    TypedExpression {
        expr: node.expr.clone(),
        typ: node.typ.as_ref().map(|t| apply_type_subst(t, subst)),
        effect: node.effect,
        children: node
            .children
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
    arity: usize,
    releasable_ref_slots: &[ManagedRefSlot],
) -> Result<Option<String>, String> {
    match &node.expr {
        Expression::Apply(items) if !items.is_empty() => match &items[0] {
            Expression::Word(op) if op == self_name => {
                let args = &node.children[1..];
                if args.len() != arity {
                    return Ok(None);
                }
                let managed_param_flags: Vec<bool> = ctx
                    .fn_sigs
                    .get(self_name)
                    .map(|(params, _)| {
                        params
                            .iter()
                            .take(arity)
                            .map(is_managed_local_type)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| vec![false; arity]);
                let arg_base_tmp = ctx.tmp_i32;
                let release_scratch_slot = arg_base_tmp + args.len();
                let arg_ctx = Ctx {
                    fn_sigs: ctx.fn_sigs,
                    fn_ids: ctx.fn_ids,
                    extern_names: ctx.extern_names,
                    lambda_ids: ctx.lambda_ids,
                    closure_defs: ctx.closure_defs,
                    lambda_bindings: ctx.lambda_bindings,
                    current_function: ctx.current_function,
                    locals: ctx.locals.clone(),
                    local_types: ctx.local_types.clone(),
                    materialized_scalar_local_slots: ctx.materialized_scalar_local_slots.clone(),
                    hoisted_scalar_vec_data_slots: ctx.hoisted_scalar_vec_data_slots.clone(),
                    proven_scalar_vec_min_lengths: ctx.proven_scalar_vec_min_lengths.clone(),
                    definitely_materialized_top_level_scalar_names: ctx
                        .definitely_materialized_top_level_scalar_names,
                    proven_scalar_index_loads: ctx.proven_scalar_index_loads,
                    nonnegative_int_locals: ctx.nonnegative_int_locals,
                    tmp_i32: release_scratch_slot + 1,
                };
                let mut out = Vec::new();
                let mut managed_arg_tmp_slots = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    out.push(compile_expr(a, &arg_ctx)?);
                    out.push(format!("local.set {}", arg_base_tmp + i));
                    if managed_param_flags.get(i).copied().unwrap_or(false) {
                        managed_arg_tmp_slots.push(arg_base_tmp + i);
                    }
                }
                if !releasable_ref_slots.is_empty() {
                    out.push(emit_release_unique_refs_except(
                        releasable_ref_slots,
                        &managed_arg_tmp_slots,
                        release_scratch_slot,
                    ));
                }
                for i in 0..args.len() {
                    out.push(format!("local.get {}", arg_base_tmp + i));
                }
                out.push(format!("return_call ${}", ident(self_name)));
                Ok(Some(out.join("\n")))
            }
            Expression::Word(op) if op == "do" => {
                compile_tail_do(items, node, ctx, self_name, arity, releasable_ref_slots)
            }
            Expression::Word(op) if op == "if" => {
                let cond_node = node
                    .children
                    .get(1)
                    .ok_or_else(|| "if missing condition".to_string())?;
                let then_node = node
                    .children
                    .get(2)
                    .ok_or_else(|| "if missing then".to_string())?;
                let else_node = node
                    .children
                    .get(3)
                    .ok_or_else(|| "if missing else".to_string())?;
                let cond = compile_expr(cond_node, ctx)?;
                let result_ty = node
                    .typ
                    .as_ref()
                    .ok_or_else(|| "if missing type".to_string())
                    .and_then(wasm_val_type)?;
                let then_code = if let Some(tc) =
                    compile_tail_expr(then_node, ctx, self_name, arity, releasable_ref_slots)?
                {
                    tc
                } else {
                    compile_expr(then_node, ctx)?
                };
                let else_code = if let Some(tc) =
                    compile_tail_expr(else_node, ctx, self_name, arity, releasable_ref_slots)?
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
        },
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
    definitely_materialized_top_level_scalar_names: &HashSet<String>,
    cached_top_level_names: &HashSet<String>,
    tail_call_mode: TailCallMode,
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
    let body_node_raw = lambda_node
        .children
        .get(body_idx)
        .ok_or_else(|| format!("Missing typed body for '{}'", name))?;
    let sig = fn_sigs.get(name).cloned();
    let mut params = Vec::new();
    for (i, p) in items[1..body_idx].iter().enumerate() {
        if let Expression::Word(w) = p {
            let ty = if let Some((ps, _ret)) = &sig {
                ps.get(i).cloned().ok_or_else(|| {
                    format!("Missing specialized param type for '{}' arg {}", name, i)
                })?
            } else {
                lambda_node
                    .typ
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
        lambda_node
            .typ
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
    let borrowed_projection_names = borrowed_projection_names(body_node, &params);
    for borrowed_name in &borrowed_projection_names {
        if let Some(slot) = locals.get(borrowed_name).copied() {
            locals.insert(format!("__borrowed_projection::{borrowed_name}"), slot);
        }
    }

    let ordinary_local_count = params.len() + local_defs.len();
    let (borrowed_top_level_count, borrowed_top_level_prelude) = top_level_borrow_plan(
        &body_node.expr,
        &mut locals,
        fn_sigs,
        cached_top_level_names,
        ordinary_local_count,
    );
    let tmp_i32 = ordinary_local_count + borrowed_top_level_count;
    let mut scoped_lambda_bindings = lambda_bindings.clone();
    // Function params shadow outer lambda bindings with the same name.
    for (pname, _) in &params {
        scoped_lambda_bindings.remove(pname);
    }

    let mut local_types = HashMap::new();
    for (p, t) in &params {
        local_types.insert(p.clone(), t.clone());
    }
    for (n, t) in &local_defs {
        local_types.insert(n.clone(), t.clone());
    }
    let empty_extern_names = HashSet::new();
    let empty_proven_scalar_index_loads = HashSet::new();
    let empty_nonnegative_int_locals = HashSet::new();
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        extern_names: &empty_extern_names,
        lambda_ids,
        closure_defs,
        lambda_bindings: &scoped_lambda_bindings,
        current_function: Some(name),
        locals,
        local_types,
        materialized_scalar_local_slots: HashSet::new(),
        hoisted_scalar_vec_data_slots: HashMap::new(),
        proven_scalar_vec_min_lengths: HashMap::new(),
        definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: &empty_proven_scalar_index_loads,
        nonnegative_int_locals: &empty_nonnegative_int_locals,
        tmp_i32,
    };
    let body_code =
        compile_guarded_scalar_param_replacement_body(body_node, &ctx, name, params.len())
            .and_then(|maybe| {
                maybe
                    .map(Ok)
                    .unwrap_or_else(|| compile_expr(body_node, &ctx))
            })
            .map_err(|e| format!("in lambda '{}': {}", name, e))?;
    let ret_is_ref = is_managed_local_type(&ret_ty);
    let mut cleanup_local_defs = Vec::new();
    collect_current_scope_let_locals(body_node, &mut cleanup_local_defs);
    let mut ref_slots: Vec<ManagedRefSlot> = Vec::new();
    for (name, t) in cleanup_local_defs {
        if is_managed_local_type(&t) && !is_borrowed_projection_local(&name, &ctx) {
            if let Some(slot) = ctx.locals.get(&name) {
                ref_slots.push(ManagedRefSlot::new(*slot, &t));
            }
        }
    }
    let has_managed_locals = local_defs.iter().any(|(_, t)| is_managed_local_type(t));
    let tco_safe = match tail_call_mode {
        TailCallMode::Off => false,
        TailCallMode::Conservative => !is_managed_local_type(&ret_ty) && !has_managed_locals,
        TailCallMode::Aggressive => true,
    };
    let guarded_tail_body = if tco_safe {
        compile_guarded_scalar_param_replacement_tail_body(
            body_node,
            &ctx,
            name,
            params.len(),
            &ref_slots,
        )?
    } else {
        None
    };
    let tail_body_code = if let Some(guarded) = guarded_tail_body.as_ref() {
        Some(guarded.inline_code())
    } else if tco_safe {
        compile_tail_expr(body_node, &ctx, name, params.len(), &ref_slots)?
    } else {
        None
    };
    let base_local_count = ordinary_local_count + borrowed_top_level_count;
    let scratch_i32_locals = scratch_i32_locals_needed(
        base_local_count,
        &[
            &borrowed_top_level_prelude,
            &body_code,
            tail_body_code.as_deref().unwrap_or(""),
        ],
        !ref_slots.is_empty(),
    );
    let mut out = String::new();
    out.push_str(&format!("  (func ${}", ident(name)));
    for (_pname, pty) in &params {
        out.push_str(&format!(" (param {})", wasm_val_type(pty)?));
    }
    out.push_str(&format!(" (result {})\n", wasm_val_type(&ret_ty)?));
    for (_n, t) in &local_defs {
        out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    emit_i32_locals(&mut out, borrowed_top_level_count);
    emit_i32_locals(&mut out, scratch_i32_locals);
    if let Some(guarded) = guarded_tail_body {
        let fallback_name = format!("__que_scalar_set_fallback_{}", ident(name));
        let fast_name = format!("__que_scalar_set_fast_{}", ident(name));
        let mut found_recursive_call = false;
        let use_recursive_fast_worker = recursive_calls_forward_guarded_params(
            &body_node.expr,
            name,
            &params,
            &guarded.requirements,
            &mut found_recursive_call,
        ) && found_recursive_call;
        if use_recursive_fast_worker {
            out.push_str(&format!(
                "    {}\n",
                guarded.any_short_guard_code().replace('\n', "\n    ")
            ));
            out.push_str(&format!("    if (result {})\n", guarded.result_ty));
            for i in 0..params.len() {
                out.push_str(&format!("      local.get {}\n", i));
            }
            out.push_str(&format!("      call ${}\n", fallback_name));
            out.push_str("    else\n");
            for i in 0..params.len() {
                out.push_str(&format!("      local.get {}\n", i));
            }
            out.push_str(&format!("      call ${}\n", fast_name));
            out.push_str("    end\n");
            out.push_str("    return\n");
            out.push_str("    unreachable\n");
            out.push_str("  )\n");

            out.push_str(&format!("  (func ${}", fast_name));
            for (_pname, pty) in &params {
                out.push_str(&format!(" (param {})", wasm_val_type(pty)?));
            }
            out.push_str(&format!(" (result {})\n", wasm_val_type(&ret_ty)?));
            for (_n, t) in &local_defs {
                out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
            }
            emit_i32_locals(&mut out, borrowed_top_level_count);
            emit_i32_locals(&mut out, scratch_i32_locals);
            if !borrowed_top_level_prelude.is_empty() {
                out.push_str(&format!(
                    "    {}\n",
                    borrowed_top_level_prelude.replace('\n', "\n    ")
                ));
            }
            if !guarded.fast_prelude.is_empty() {
                out.push_str(&format!(
                    "    {}\n",
                    guarded.fast_prelude.replace('\n', "\n    ")
                ));
            }
            let fast_code = redirect_direct_recursive_calls(&guarded.fast_code, name, &fast_name);
            out.push_str(&format!("    {}\n", fast_code.replace('\n', "\n    ")));
            out.push_str("    unreachable\n");
            out.push_str("  )\n");

            out.push_str(&format!("  (func ${}", fallback_name));
            for (_pname, pty) in &params {
                out.push_str(&format!(" (param {})", wasm_val_type(pty)?));
            }
            out.push_str(&format!(" (result {})\n", wasm_val_type(&ret_ty)?));
            for (_n, t) in &local_defs {
                out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
            }
            emit_i32_locals(&mut out, borrowed_top_level_count);
            emit_i32_locals(&mut out, scratch_i32_locals);
            if !borrowed_top_level_prelude.is_empty() {
                out.push_str(&format!(
                    "    {}\n",
                    borrowed_top_level_prelude.replace('\n', "\n    ")
                ));
            }
            out.push_str(&format!(
                "    {}\n",
                guarded.fallback_code.replace('\n', "\n    ")
            ));
            out.push_str("    unreachable\n");
            out.push_str("  )\n");
            return Ok(out);
        }
        out.push_str(&format!(
            "    {}\n",
            guarded.any_short_guard_code().replace('\n', "\n    ")
        ));
        out.push_str(&format!("    if (result {})\n", guarded.result_ty));
        for i in 0..params.len() {
            out.push_str(&format!("      local.get {}\n", i));
        }
        out.push_str(&format!("      call ${}\n", fallback_name));
        out.push_str("    else\n");
        if !borrowed_top_level_prelude.is_empty() {
            out.push_str(&format!(
                "      {}\n",
                borrowed_top_level_prelude.replace('\n', "\n      ")
            ));
        }
        if !guarded.fast_prelude.is_empty() {
            out.push_str(&format!(
                "      {}\n",
                guarded.fast_prelude.replace('\n', "\n      ")
            ));
        }
        out.push_str(&format!(
            "      {}\n",
            guarded.fast_code.replace('\n', "\n      ")
        ));
        out.push_str("    end\n");
        out.push_str("    return\n");
        out.push_str("    unreachable\n");
        out.push_str("  )\n");

        out.push_str(&format!("  (func ${}", fallback_name));
        for (_pname, pty) in &params {
            out.push_str(&format!(" (param {})", wasm_val_type(pty)?));
        }
        out.push_str(&format!(" (result {})\n", wasm_val_type(&ret_ty)?));
        for (_n, t) in &local_defs {
            out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
        }
        emit_i32_locals(&mut out, borrowed_top_level_count);
        emit_i32_locals(&mut out, scratch_i32_locals);
        if !borrowed_top_level_prelude.is_empty() {
            out.push_str(&format!(
                "    {}\n",
                borrowed_top_level_prelude.replace('\n', "\n    ")
            ));
        }
        out.push_str(&format!(
            "    {}\n",
            guarded.fallback_code.replace('\n', "\n    ")
        ));
        out.push_str("    unreachable\n");
        out.push_str("  )\n");
        return Ok(out);
    }
    if let Some(tail_code) = tail_body_code {
        if !borrowed_top_level_prelude.is_empty() {
            out.push_str(&format!(
                "    {}\n",
                borrowed_top_level_prelude.replace('\n', "\n    ")
            ));
        }
        out.push_str(&format!("    {}\n", tail_code.replace('\n', "\n    ")));
        out.push_str("    unreachable\n");
        out.push_str("  )\n");
        return Ok(out);
    }
    out.push_str(&format!("    (local {})\n", wasm_val_type(&ret_ty)?));
    let ret_slot = base_local_count + scratch_i32_locals;
    if !borrowed_top_level_prelude.is_empty() {
        out.push_str(&format!(
            "    {}\n",
            borrowed_top_level_prelude.replace('\n', "\n    ")
        ));
    }
    out.push_str(&format!("    {}\n", body_code.replace('\n', "\n    ")));
    out.push_str(&format!("    local.set {}\n", ret_slot));
    if ret_is_ref
        && returns_projected_ref_from_managed_local(
            &body_node.expr,
            &managed_ref_slot_names(&ctx.locals, &ref_slots),
        )
    {
        out.push_str(&format!("    local.get {}\n", ret_slot));
        out.push_str(&format!("    call {}\n", rc_retain_for_type(&ret_ty)));
        out.push_str("    drop\n");
    }
    let scratch_slot = base_local_count;
    out.push_str(&emit_release_unique_refs(
        &ref_slots,
        ret_slot,
        ret_is_ref,
        scratch_slot,
    ));
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
    lambda_bindings: &HashMap<String, TypedExpression>,
    definitely_materialized_top_level_scalar_names: &HashSet<String>,
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
    let body_node = lambda_node
        .children
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

    let mut local_types = HashMap::new();
    for (p, t) in &params {
        local_types.insert(p.clone(), t.clone());
    }
    for (n, t) in &local_defs {
        local_types.insert(n.clone(), t.clone());
    }
    let empty_extern_names = HashSet::new();
    let empty_proven_scalar_index_loads = HashSet::new();
    let empty_nonnegative_int_locals = HashSet::new();
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        extern_names: &empty_extern_names,
        lambda_ids,
        closure_defs,
        lambda_bindings: &scoped_lambda_bindings,
        current_function: Some(name),
        locals,
        local_types,
        materialized_scalar_local_slots: HashSet::new(),
        hoisted_scalar_vec_data_slots: HashMap::new(),
        proven_scalar_vec_min_lengths: HashMap::new(),
        definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: &empty_proven_scalar_index_loads,
        nonnegative_int_locals: &empty_nonnegative_int_locals,
        tmp_i32,
    };
    let body_code =
        compile_expr(body_node, &ctx).map_err(|e| format!("in closure '{}': {}", name, e))?;
    let ret_is_ref = is_managed_local_type(&ret_ty);
    let mut cleanup_local_defs = Vec::new();
    collect_current_scope_let_locals(body_node, &mut cleanup_local_defs);
    let mut ref_slots: Vec<ManagedRefSlot> = Vec::new();
    for (name, t) in cleanup_local_defs {
        if is_managed_local_type(&t) && !is_borrowed_projection_local(&name, &ctx) {
            if let Some(slot) = ctx.locals.get(&name) {
                ref_slots.push(ManagedRefSlot::new(*slot, &t));
            }
        }
    }
    let base_local_count = params.len() + local_defs.len();
    let scratch_i32_locals =
        scratch_i32_locals_needed(base_local_count, &[&body_code], !ref_slots.is_empty());

    let mut out = String::new();
    out.push_str(&format!("  (func ${}", ident(name)));
    for (_pname, pty) in &params {
        out.push_str(&format!(" (param {})", wasm_val_type(pty)?));
    }
    out.push_str(&format!(" (result {})\n", wasm_val_type(&ret_ty)?));
    for (_n, t) in &local_defs {
        out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    emit_i32_locals(&mut out, scratch_i32_locals);
    out.push_str(&format!("    (local {})\n", wasm_val_type(&ret_ty)?));
    let ret_slot = base_local_count + scratch_i32_locals;
    out.push_str(&format!("    {}\n", body_code.replace('\n', "\n    ")));
    out.push_str(&format!("    local.set {}\n", ret_slot));
    if ret_is_ref
        && returns_projected_ref_from_managed_local(
            &body_node.expr,
            &managed_ref_slot_names(&ctx.locals, &ref_slots),
        )
    {
        out.push_str(&format!("    local.get {}\n", ret_slot));
        out.push_str(&format!("    call {}\n", rc_retain_for_type(&ret_ty)));
        out.push_str("    drop\n");
    }
    let scratch_slot = base_local_count;
    out.push_str(&emit_release_unique_refs(
        &ref_slots,
        ret_slot,
        ret_is_ref,
        scratch_slot,
    ));
    out.push_str(&format!("    local.get {}\n", ret_slot));
    out.push_str("  )\n");
    Ok(out)
}

fn emit_release_unique_refs(
    ref_slots: &[ManagedRefSlot],
    ret_slot: usize,
    ret_is_ref: bool,
    scratch_slot: usize,
) -> String {
    let mut except_slots = Vec::new();
    if ret_is_ref {
        except_slots.push(ret_slot);
    }
    emit_release_unique_refs_except(ref_slots, &except_slots, scratch_slot)
}

fn emit_release_unique_refs_except(
    ref_slots: &[ManagedRefSlot],
    except_slots: &[usize],
    scratch_slot: usize,
) -> String {
    let mut out = String::new();
    for (i, reference) in ref_slots.iter().enumerate() {
        let slot = reference.slot;
        out.push_str("    i32.const 1\n");
        out.push_str(&format!("    local.set {}\n", scratch_slot));
        for except_slot in except_slots {
            out.push_str(&format!("    local.get {}\n", slot));
            out.push_str(&format!("    local.get {}\n", except_slot));
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
            out.push_str(&format!("      local.get {}\n", prev.slot));
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
        out.push_str(&format!("      call {}\n", reference.kind.release()));
        out.push_str("      drop\n");
        out.push_str("    end\n");
    }
    out
}

fn managed_ref_slot_names(
    locals: &HashMap<String, usize>,
    ref_slots: &[ManagedRefSlot],
) -> HashSet<String> {
    ref_slots
        .iter()
        .filter_map(|reference| {
            locals.iter().find_map(|(name, local_slot)| {
                (*local_slot == reference.slot).then(|| name.clone())
            })
        })
        .collect()
}

fn returns_projected_ref_from_managed_local(
    expr: &Expression,
    managed_names: &HashSet<String>,
) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    let Some(Expression::Word(head)) = items.first() else {
        return false;
    };
    match head.as_str() {
        "do" | "block" => items
            .last()
            .map(|last| returns_projected_ref_from_managed_local(last, managed_names))
            .unwrap_or(false),
        "if" => items
            .iter()
            .skip(2)
            .any(|branch| returns_projected_ref_from_managed_local(branch, managed_names)),
        "cond" => items
            .iter()
            .skip(1)
            .step_by(2)
            .any(|branch| returns_projected_ref_from_managed_local(branch, managed_names)),
        "car" | "cdr" | "fst" | "snd" => items
            .get(1)
            .map(|target| expr_reads_from_managed_local(target, managed_names))
            .unwrap_or(false),
        "get" => items
            .get(1)
            .map(|target| expr_reads_from_managed_local(target, managed_names))
            .unwrap_or(false),
        _ => false,
    }
}

fn expr_reads_from_managed_local(expr: &Expression, managed_names: &HashSet<String>) -> bool {
    match expr {
        Expression::Word(name) => managed_names.contains(name),
        Expression::Apply(items) => {
            let Some(Expression::Word(head)) = items.first() else {
                return false;
            };
            matches!(head.as_str(), "car" | "cdr" | "fst" | "snd" | "get")
                && items
                    .get(1)
                    .map(|target| expr_reads_from_managed_local(target, managed_names))
                    .unwrap_or(false)
        }
        _ => false,
    }
}

fn compile_value_func(
    name: &str,
    value_node: &TypedExpression,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &HashMap<String, i32>,
    lambda_ids: &HashMap<String, i32>,
    closure_defs: &HashMap<String, ClosureDef>,
    lambda_bindings: &HashMap<String, TypedExpression>,
    definitely_materialized_top_level_scalar_names: &HashSet<String>,
) -> Result<String, String> {
    let ret_ty = value_node
        .typ
        .as_ref()
        .ok_or_else(|| format!("Missing value type for '{}'", name))?;

    let mut local_defs = Vec::new();
    collect_let_locals(value_node, &mut local_defs);
    let mut locals = HashMap::new();
    for (i, (n, _)) in local_defs.iter().enumerate() {
        locals.insert(n.clone(), i);
    }
    let tmp_i32 = local_defs.len();
    let scoped_lambda_bindings = lambda_bindings.clone();

    let mut local_types = HashMap::new();
    for (n, t) in &local_defs {
        local_types.insert(n.clone(), t.clone());
    }
    let empty_extern_names = HashSet::new();
    let empty_proven_scalar_index_loads = HashSet::new();
    let empty_nonnegative_int_locals = HashSet::new();
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        extern_names: &empty_extern_names,
        lambda_ids,
        closure_defs,
        lambda_bindings: &scoped_lambda_bindings,
        current_function: Some(name),
        locals,
        local_types,
        materialized_scalar_local_slots: HashSet::new(),
        hoisted_scalar_vec_data_slots: HashMap::new(),
        proven_scalar_vec_min_lengths: HashMap::new(),
        definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: &empty_proven_scalar_index_loads,
        nonnegative_int_locals: &empty_nonnegative_int_locals,
        tmp_i32,
    };
    let body_code =
        compile_expr(value_node, &ctx).map_err(|e| format!("in value '{}': {}", name, e))?;
    let ret_is_ref = is_managed_local_type(ret_ty);
    let mut cleanup_local_defs = Vec::new();
    collect_current_scope_let_locals(value_node, &mut cleanup_local_defs);
    let ref_slots: Vec<ManagedRefSlot> = cleanup_local_defs
        .iter()
        .filter_map(|(name, t)| {
            if is_managed_local_type(t) {
                ctx.locals
                    .get(name)
                    .map(|slot| ManagedRefSlot::new(*slot, t))
            } else {
                None
            }
        })
        .collect();
    let base_local_count = local_defs.len();
    let scratch_i32_locals =
        scratch_i32_locals_needed(base_local_count, &[&body_code], !ref_slots.is_empty());

    let mut out = String::new();
    out.push_str(&format!(
        "  (func ${} (result {})\n",
        ident(name),
        wasm_val_type(ret_ty)?
    ));
    for (_n, t) in &local_defs {
        out.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    emit_i32_locals(&mut out, scratch_i32_locals);
    out.push_str(&format!("    (local {})\n", wasm_val_type(ret_ty)?));
    let ret_slot = base_local_count + scratch_i32_locals;
    let scratch_slot = base_local_count;
    let g_init = cache_init_global(name);
    let g_val = cache_value_global(name);

    out.push_str(&format!("    global.get ${}\n", g_init));
    out.push_str("    if\n");
    out.push_str(&format!("      global.get ${}\n", g_val));
    out.push_str(&format!("      local.set {}\n", ret_slot));
    if ret_is_ref {
        out.push_str(&format!("      local.get {}\n", ret_slot));
        out.push_str(&format!("      call {}\n", rc_retain_for_type(ret_ty)));
        out.push_str("      drop\n");
    }
    out.push_str("    else\n");
    out.push_str(&format!("      {}\n", body_code.replace('\n', "\n      ")));
    out.push_str(&format!("      local.set {}\n", ret_slot));
    out.push_str(&indent_block(
        &emit_release_unique_refs(&ref_slots, ret_slot, ret_is_ref, scratch_slot),
        6,
    ));
    out.push('\n');
    if ret_is_ref {
        // Keep one root reference in the global cache while returning one to caller.
        out.push_str(&format!("      local.get {}\n", ret_slot));
        out.push_str(&format!("      call {}\n", rc_retain_for_type(ret_ty)));
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
    format!(
        "  (func ${} (result i32)\n    i32.const {}\n  )\n",
        ident(name),
        fn_id
    )
}

fn top_level_value_fn_ptr(
    expr: &Expression,
    top_defs: &HashMap<String, TopDef>,
    fn_ids: &HashMap<String, i32>,
) -> Option<i32> {
    fn resolve_word_fn_ptr(
        word: &str,
        top_defs: &HashMap<String, TopDef>,
        fn_ids: &HashMap<String, i32>,
        seen: &mut HashSet<String>,
    ) -> Option<i32> {
        if let Some(def) = top_defs.get(word) {
            if !seen.insert(word.to_string()) {
                return None;
            }
            match &def.expr {
                Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") =>
                {
                    return fn_ids.get(word).copied();
                }
                Expression::Word(alias) => {
                    return resolve_word_fn_ptr(alias, top_defs, fn_ids, seen);
                }
                _ => {}
            }
        }
        if let Some(fn_id) = fn_ids.get(word).copied() {
            return Some(fn_id);
        };
        builtin_fn_tag(word)
    }

    let Expression::Word(word) = expr else {
        return None;
    };
    resolve_word_fn_ptr(word, top_defs, fn_ids, &mut HashSet::new())
}

fn compile_partial_helper_func(
    h: &PartialHelper,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &HashMap<String, i32>,
    lambda_ids: &HashMap<String, i32>,
    closure_defs: &HashMap<String, ClosureDef>,
    lambda_bindings: &HashMap<String, TypedExpression>,
) -> Result<String, String> {
    let mut locals = HashMap::new();
    for i in 0..h.remaining_params.len() {
        locals.insert(format!("__p{}", i), i);
    }
    let mut local_types = HashMap::new();
    for (i, t) in h.remaining_params.iter().enumerate() {
        local_types.insert(format!("__p{}", i), t.clone());
    }
    let empty_top_level_materialized = HashSet::new();
    let empty_extern_names = HashSet::new();
    let empty_proven_scalar_index_loads = HashSet::new();
    let empty_nonnegative_int_locals = HashSet::new();
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        extern_names: &empty_extern_names,
        lambda_ids,
        closure_defs,
        lambda_bindings,
        current_function: None,
        locals,
        local_types,
        materialized_scalar_local_slots: HashSet::new(),
        hoisted_scalar_vec_data_slots: HashMap::new(),
        proven_scalar_vec_min_lengths: HashMap::new(),
        definitely_materialized_top_level_scalar_names: &empty_top_level_materialized,
        proven_scalar_index_loads: &empty_proven_scalar_index_loads,
        nonnegative_int_locals: &empty_nonnegative_int_locals,
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
    let body_code = body_parts.join("\n    ");
    let scratch_i32_locals =
        scratch_i32_locals_needed(h.remaining_params.len(), &[&body_code], false);

    let mut out = String::new();
    out.push_str(&format!("  (func ${}", ident(&h.helper_name)));
    for p in &h.remaining_params {
        out.push_str(&format!(" (param {})", wasm_val_type(p)?));
    }
    out.push_str(&format!(" (result {})\n", wasm_val_type(&h.ret)?));
    emit_i32_locals(&mut out, scratch_i32_locals);
    out.push_str(&format!("    {}\n", body_code));
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
    out.push_str("    local.get 0\n");
    for i in 1..=h.total_arity {
        out.push_str(&format!("    local.get {}\n", i));
    }
    out.push_str(&format!("    call $apply{}_i32\n", h.total_arity));
    out.push_str("  )\n");
    out
}

fn compile_program_to_wat_build_typed_with_opts(
    typed_ast: &TypedExpression,
    enable_optimizer: bool,
) -> Result<WatBuildOutput, String> {
    // Validate devirtualization mode early so invalid env values fail deterministically.
    let _ = devirtualize_mode_from_env()?;
    let tail_call_mode = tail_call_mode_from_env()?;
    let optimized_typed_ast = if enable_optimizer {
        Some(crate::op::optimize_typed_ast(typed_ast))
    } else {
        None
    };
    let typed_ast = optimized_typed_ast.as_ref().unwrap_or(typed_ast);
    // The analyzer and lowerer inspect the exact same optimized tree. Proven
    // facts may remove debug checks; unknown or conflicting occurrences keep
    // the conservative runtime guard.
    let _static_proof_guard = (parse_env_bool_like("QUE_OPT_PROOF_CODEGEN", false)
        || parse_env_bool_like("QUEC_DEBUG_ANALYSIS", false))
    .then(|| StaticProofFactsGuard::install(typed_ast));
    validate_no_rc_cycles(typed_ast)?;

    let (top_defs, extern_defs, main_expr, main_node) = match &typed_ast.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "do") =>
        {
            let child_offset = if typed_ast.children.len() + 1 == items.len() {
                1
            } else {
                0
            };
            let child_at = |item_idx: usize| -> Option<&TypedExpression> {
                if item_idx < child_offset {
                    None
                } else {
                    typed_ast.children.get(item_idx - child_offset)
                }
            };
            let mut defs = HashMap::new();
            let mut externs = HashMap::new();
            let mut main_items_expr = vec![Expression::Word("do".to_string())];
            let mut main_items_nodes: Vec<TypedExpression> = Vec::new();
            let mut main_only_names = HashSet::new();
            main_only_names.extend(collect_main_mutated_top_level_let_names(items));
            for i in 1..items.len() {
                if let Expression::Apply(let_items) = &items[i] {
                    if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                        if kw == "mut" {
                            main_only_names.insert(name.clone());
                        }
                    }
                }
            }
            for i in 1..items.len() {
                if let Expression::Apply(let_items) = &items[i] {
                    if let Ok(Some(extern_decl)) = crate::externals::parse_extern_decl(&items[i]) {
                        externs.insert(extern_decl.local_name.clone(), extern_decl);
                        continue;
                    }
                    if let [Expression::Word(kw), Expression::Word(name), rhs] = &let_items[..] {
                        if kw == "let" || kw == "letrec" {
                            if let Some(node) = child_at(i).and_then(|n| n.children.get(2)).cloned()
                            {
                                if main_only_names.contains(name)
                                    || top_level_binding_rhs_refs_main_only_names(
                                        kw,
                                        name,
                                        rhs,
                                        &main_only_names,
                                    )
                                {
                                    main_only_names.insert(name.clone());
                                    main_items_expr.push(items[i].clone());
                                    let node = child_at(i).cloned().ok_or_else(|| {
                                        "Missing typed top-level expression while building wasm main"
                                            .to_string()
                                    })?;
                                    main_items_nodes.push(node);
                                } else {
                                    defs.insert(
                                        name.clone(),
                                        TopDef {
                                            expr: rhs.clone(),
                                            node,
                                        },
                                    );
                                    // Top-level bindings are canonicalized as defs and referenced by name.
                                    // Do not also keep duplicate let expressions in main.
                                }
                                continue;
                            }
                        }
                    }
                }
                main_items_expr.push(items[i].clone());
                let node = child_at(i).cloned().ok_or_else(|| {
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
            (defs, externs, main_expr, main_node)
        }
        _ => (
            HashMap::new(),
            HashMap::new(),
            typed_ast.expr.clone(),
            typed_ast.clone(),
        ),
    };

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
    let mut used_extern_defs: HashMap<String, crate::externals::ExternDecl> = extern_defs
        .iter()
        .filter(|(name, _)| needed.contains(*name))
        .map(|(name, decl)| (name.clone(), decl.clone()))
        .collect();
    for name in &needed {
        if used_extern_defs.contains_key(name) {
            continue;
        }
        if let Some(decl) = crate::externals::builtin_host_extern_decl(name) {
            used_extern_defs.insert(name.clone(), decl);
        }
    }
    let mut builtin_host_call_heads = HashSet::new();
    collect_builtin_host_extern_call_heads(&typed_ast.expr, &mut builtin_host_call_heads);
    for name in builtin_host_call_heads {
        if used_extern_defs.contains_key(&name) {
            continue;
        }
        if let Some(decl) = crate::externals::builtin_host_extern_decl(&name) {
            used_extern_defs.insert(name, decl);
        }
    }
    let definitely_materialized_top_level_scalar_names =
        collect_definitely_materialized_top_level_scalar_names(&top_defs);

    let mut fn_sigs: HashMap<String, (Vec<Type>, Type)> = HashMap::new();
    let mut top_level_lambda_key_to_name: HashMap<String, String> = HashMap::new();
    let mut top_def_names: HashSet<String> = top_defs.keys().cloned().collect();
    top_def_names.extend(used_extern_defs.keys().cloned());
    let mut dynamic_partial_specs: HashSet<(usize, usize)> = HashSet::new();
    collect_dynamic_partial_specs(typed_ast, &top_def_names, &mut dynamic_partial_specs);
    let mut call_specs: HashMap<String, CallSpecialization> = HashMap::new();
    if enable_optimizer {
        collect_call_specializations(typed_ast, &top_def_names, &mut call_specs);
    }
    for (name, def) in &top_defs {
        let is_lambda_def = matches!(
            &def.expr,
            Expression::Apply(items)
                if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
        );
        let (ps, ret) = if is_lambda_def {
            let t = def
                .node
                .typ
                .as_ref()
                .ok_or_else(|| format!("Missing type for def '{}'", name))?;
            let (mut decl_ps, decl_ret) = function_parts(t);
            let syn_arity = lambda_syntax_arity(&def.expr);
            if syn_arity == 0 && decl_ps.len() == 1 && matches!(decl_ps[0], Type::Unit) {
                decl_ps.clear();
            } else if decl_ps.len() >= syn_arity {
                decl_ps.truncate(syn_arity);
            }
            match call_specs.get(name) {
                Some(CallSpecialization::Unique(spec_ps, spec_ret))
                    if spec_ps.len() == decl_ps.len() =>
                {
                    (spec_ps.clone(), spec_ret.clone())
                }
                _ => (decl_ps, decl_ret),
            }
        } else {
            let t = def
                .node
                .typ
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
    for extern_decl in used_extern_defs.values() {
        let (mut ps, ret) = function_parts(&extern_decl.typ);
        if ps.len() == 1 && matches!(ps[0], Type::Unit) {
            ps.clear();
        }
        for p in &ps {
            wasm_val_type(p)?;
        }
        wasm_val_type(&ret)?;
        fn_sigs.insert(extern_decl.local_name.clone(), (ps, ret));
    }
    let mut lambda_nodes = Vec::new();
    collect_lambda_nodes(typed_ast, &mut lambda_nodes);
    let mut lambda_bindings: HashMap<String, TypedExpression> = HashMap::new();
    collect_top_level_lambda_bindings(&top_defs, &mut lambda_bindings);
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
                closure_defs.insert(
                    key.clone(),
                    ClosureDef {
                        key,
                        name,
                        captures,
                        user_arity: ps.len(),
                    },
                );
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
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 21, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34,
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
        let captures = (0..cap_count)
            .map(|i| format!("__cap{}", i))
            .collect::<Vec<_>>();
        let key = format!("__partial_dyn_key_{}_{}", total, provided);
        closure_defs.insert(
            key.clone(),
            ClosureDef {
                key,
                name: name.clone(),
                captures,
                user_arity: total - provided,
            },
        );
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
    let extern_names: HashSet<String> = used_extern_defs.keys().cloned().collect();
    let main_ret_ty = main_node
        .typ
        .as_ref()
        .ok_or_else(|| "Missing main expression type".to_string())?;
    let mut emitted_funcs: Vec<String> = Vec::new();
    let mut cached_value_defs: Vec<String> = Vec::new();
    let cached_top_level_names = top_defs
        .iter()
        .filter_map(|(name, def)| {
            let is_partial = partial_helpers.iter().any(|h| h.binding_name == *name);
            let is_lambda = matches!(
                &def.expr,
                Expression::Apply(items)
                    if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
            );
            (!is_partial
                && !is_lambda
                && top_level_value_fn_ptr(&def.expr, &top_defs, &fn_ids).is_none())
            .then(|| name.clone())
        })
        .collect::<HashSet<_>>();

    for (name, def) in &top_defs {
        if partial_helpers.iter().any(|h| h.binding_name == *name) {
            continue;
        }
        match &def.expr {
            Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") =>
            {
                emitted_funcs.push(compile_lambda_func(
                    name,
                    &def.expr,
                    &def.node,
                    &fn_sigs,
                    &fn_ids,
                    &lambda_ids,
                    &closure_defs,
                    &lambda_bindings,
                    &definitely_materialized_top_level_scalar_names,
                    &cached_top_level_names,
                    tail_call_mode,
                )?);
            }
            expr if top_level_value_fn_ptr(expr, &top_defs, &fn_ids).is_some() => {
                let fn_id = top_level_value_fn_ptr(expr, &top_defs, &fn_ids).ok_or_else(|| {
                    format!("Missing function pointer for top-level value '{}'", name)
                })?;
                emitted_funcs.push(compile_value_func_fn_ptr(name, fn_id));
            }
            _ => {
                cached_value_defs.push(name.clone());
                emitted_funcs.push(compile_value_func(
                    name,
                    &def.node,
                    &fn_sigs,
                    &fn_ids,
                    &lambda_ids,
                    &closure_defs,
                    &lambda_bindings,
                    &definitely_materialized_top_level_scalar_names,
                )?);
            }
        }
    }
    for h in &partial_helpers {
        emitted_funcs.push(compile_partial_helper_func(
            h,
            &fn_sigs,
            &fn_ids,
            &lambda_ids,
            &closure_defs,
            &lambda_bindings,
        )?);
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
            emitted_funcs.push(compile_lambda_func(
                name,
                &node.expr,
                node,
                &fn_sigs,
                &fn_ids,
                &lambda_ids,
                &closure_defs,
                &lambda_bindings,
                &definitely_materialized_top_level_scalar_names,
                &cached_top_level_names,
                tail_call_mode,
            )?);
        }
    }
    for def in closure_defs.values() {
        if let Some(node) = lambda_nodes.iter().find(|n| n.expr.to_lisp() == def.key) {
            emitted_funcs.push(compile_closure_func(
                &def.name,
                node,
                &def.captures,
                &fn_sigs,
                &fn_ids,
                &lambda_ids,
                &closure_defs,
                &lambda_bindings,
                &definitely_materialized_top_level_scalar_names,
            )?);
        }
    }

    let main_wasm_ty = wasm_val_type(main_ret_ty)?;

    let mut main_local_defs = Vec::new();
    collect_let_locals(&main_node, &mut main_local_defs);
    let mut main_locals = HashMap::new();
    for (i, (n, _)) in main_local_defs.iter().enumerate() {
        main_locals.insert(n.clone(), i);
    }
    let (main_borrowed_top_level_count, main_borrowed_top_level_prelude) = top_level_borrow_plan(
        &main_node.expr,
        &mut main_locals,
        &fn_sigs,
        &cached_top_level_names,
        main_local_defs.len(),
    );

    let mut main_local_types = HashMap::new();
    for (n, t) in &main_local_defs {
        main_local_types.insert(n.clone(), t.clone());
    }
    let main_proven_scalar_index_loads = HashSet::new();
    let main_nonnegative_int_locals = HashSet::new();
    let main_ctx = Ctx {
        fn_sigs: &fn_sigs,
        fn_ids: &fn_ids,
        extern_names: &extern_names,
        lambda_ids: &lambda_ids,
        closure_defs: &closure_defs,
        lambda_bindings: &lambda_bindings,
        current_function: None,
        locals: main_locals,
        local_types: main_local_types,
        materialized_scalar_local_slots: HashSet::new(),
        hoisted_scalar_vec_data_slots: HashMap::new(),
        proven_scalar_vec_min_lengths: HashMap::new(),
        definitely_materialized_top_level_scalar_names:
            &definitely_materialized_top_level_scalar_names,
        proven_scalar_index_loads: &main_proven_scalar_index_loads,
        nonnegative_int_locals: &main_nonnegative_int_locals,
        tmp_i32: main_local_defs.len() + main_borrowed_top_level_count,
    };
    let main_code = compile_expr(&main_node, &main_ctx)?;
    let mut apply_arities: HashSet<usize> = HashSet::new();
    for func in &emitted_funcs {
        collect_apply_arities_from_code(func, &mut apply_arities);
    }
    collect_apply_arities_from_code(&main_code, &mut apply_arities);
    let uses_argv = main_code.contains("call $__argv_get")
        || emitted_funcs
            .iter()
            .any(|func| func.contains("call $__argv_get"));
    if extern_names.contains("read/chunks!")
        || extern_names.contains("stdin/chunks!")
        || extern_names.contains("read/lines!")
    {
        apply_arities.insert(1);
    }
    let main_base_local_count = main_local_defs.len() + main_borrowed_top_level_count;
    let main_scratch_i32_locals = scratch_i32_locals_needed(
        main_base_local_count,
        &[&main_borrowed_top_level_prelude, &main_code],
        false,
    );

    let mut main_func = String::new();
    main_func.push_str(&format!(
        "  ;; Type: {}\n",
        abi_type_descriptor(main_ret_ty)
    ));
    let wasi_codegen = wasi_bool("QUE_WASI_HOST");
    let wasi_no_result = wasi_bool("QUE_WASI_NO_RESULT");
    let main_name = if wasi_codegen { "$__que_main " } else { "" };
    main_func.push_str(&format!(
        "  (func {main_name}(export \"main\") (result {main_wasm_ty})\n"
    ));
    for (_n, t) in &main_local_defs {
        main_func.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    emit_i32_locals(&mut main_func, main_borrowed_top_level_count);
    emit_i32_locals(&mut main_func, main_scratch_i32_locals);
    if !main_borrowed_top_level_prelude.is_empty() {
        main_func.push_str(&format!(
            "    {}\n",
            main_borrowed_top_level_prelude.replace('\n', "\n    ")
        ));
    }
    if wasi_codegen && uses_argv {
        main_func.push_str("    call $__wasi_init_argv\n    drop\n");
    }
    main_func.push_str(&format!("    {}\n", main_code.replace('\n', "\n    ")));
    main_func.push_str("  )\n");
    if wasi_codegen {
        if wasi_no_result {
            main_func.push_str("  (func (export \"_start\")\n    call $__que_main\n    drop)\n");
        } else {
            let render = match main_ret_ty {
                Type::List(inner) if matches!(inner.as_ref(), Type::Char) => {
                    "local.get $text\n    local.get $value\n    call $__serde_append".to_string()
                }
                Type::Char => "local.get $text\n    local.get $value\n    call $vec_push_i32\n    drop".to_string(),
                Type::Unit => "i32.const 0\n    call $__serde_int\n    local.set $tmp\n    local.get $text\n    local.get $tmp\n    call $__serde_append\n    local.get $tmp\n    call $rc_release_vec\n    drop".to_string(),
                Type::Var(_) | Type::Function(_, _) => "local.get $value\n    call $__serde_int\n    local.set $tmp\n    local.get $text\n    local.get $tmp\n    call $__serde_append\n    local.get $tmp\n    call $rc_release_vec\n    drop".to_string(),
                _ => format!("local.get $value\n    call $__wasi_serialize_{}\n    local.set $tmp\n    local.get $text\n    local.get $tmp\n    call $__serde_append\n    local.get $tmp\n    call $rc_release_vec\n    drop", wasi_serde_name(main_ret_ty)),
            };
            let release_value = if is_managed_local_type(main_ret_ty) {
                format!(
                    "local.get $value\n    call {}\n    drop\n    ",
                    rc_release_for_type(main_ret_ty)
                )
            } else {
                String::new()
            };
            main_func.push_str(&format!("  (func (export \"_start\")\n    (local $value i32) (local $text i32) (local $tmp i32)\n    call $__que_main\n    local.set $value\n    i32.const 0\n    i32.const 0\n    call $vec_new_i32\n    local.set $text\n    {render}\n    {release_value}local.get $text\n    i32.const 10\n    call $vec_push_i32\n    drop\n    i32.const 1\n    local.get $text\n    call $__wasi_write_text\n    drop\n    local.get $text\n    call $rc_release_vec\n    drop)\n"));
        }
    }

    let mut extern_imports = String::new();
    let wasi_host = wasi_bool("QUE_WASI_HOST");
    let wasi_prints_result = wasi_host && !wasi_no_result;
    let wasi_guard_diagnostics = wasi_host
        && (parse_env_bool_like("QUE_INT_OVERFLOW_CHECK", false)
            || parse_env_bool_like("QUE_DEC_OVERFLOW_CHECK", false)
            || parse_env_bool_like("QUE_DIV_ZERO_CHECK", false));
    if wasi_host {
        if let Some(raw_permissions) = wasi_allow() {
            let mut permissions = raw_permissions
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<HashSet<_>>();
            if permissions.contains("all") || permissions.contains("*") {
                permissions.extend(["read", "stdin", "write", "print", "clock", "delete"]);
            }
            let required = [
                ("list-dir!", "read"),
                ("read!", "read"),
                ("read/chunks!", "read"),
                ("read/lines!", "read"),
                ("stdin!", "stdin"),
                ("stdin/chunks!", "stdin"),
                ("write!", "write"),
                ("mkdir!", "write"),
                ("move!", "write"),
                ("delete!", "delete"),
                ("print!", "print"),
                ("clear!", "print"),
                ("sleep!", "clock"),
                ("time!", "clock"),
                ("random!", "clock"),
            ];
            for (operation, permission) in required {
                if used_extern_defs.contains_key(operation) && !permissions.contains(permission) {
                    return Err(format!(
                        "{operation} requires --allow {permission} with the external WASI runtime"
                    ));
                }
            }
        }
    }
    for extern_decl in used_extern_defs.values() {
        if wasi_host
            && matches!(
                extern_decl.local_name.as_str(),
                "print!"
                    | "stdin!"
                    | "read!"
                    | "read/chunks!"
                    | "read/lines!"
                    | "stdin/chunks!"
                    | "list-dir!"
                    | "write!"
                    | "mkdir!"
                    | "delete!"
                    | "move!"
                    | "sleep!"
                    | "time!"
                    | "random!"
                    | "clear!"
            )
        {
            continue;
        }
        let (params, ret) = fn_sigs
            .get(&extern_decl.local_name)
            .ok_or_else(|| format!("Missing extern signature for '{}'", extern_decl.local_name))?;
        let wasm_params = wasm_param_types_for_signature(params)?;
        extern_imports.push_str(&format!(
            "  (import \"{}\" \"{}\" (func ${}",
            extern_decl.module,
            extern_decl.import,
            ident(&extern_decl.local_name)
        ));
        for param in wasm_params {
            extern_imports.push_str(&format!(" (param {})", param));
        }
        extern_imports.push_str(&format!(" (result {})))\n", wasm_val_type(ret)?));
    }
    if wasi_host
        && (wasi_prints_result
            || used_extern_defs.contains_key("print!")
            || used_extern_defs.contains_key("clear!")
            || used_extern_defs.contains_key("write!")
            || wasi_guard_diagnostics)
    {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"fd_write\" (func $__wasi_fd_write (param i32 i32 i32 i32) (result i32)))\n",
        );
    }
    if wasi_guard_diagnostics {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"proc_exit\" (func $__wasi_proc_exit (param i32)))\n",
        );
    }
    if wasi_host && used_extern_defs.contains_key("sleep!") {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"poll_oneoff\" (func $__wasi_poll_oneoff (param i32 i32 i32 i32) (result i32)))\n",
        );
    }
    if wasi_host && used_extern_defs.contains_key("time!") {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"clock_time_get\" (func $__wasi_clock_time_get (param i32 i64 i32) (result i32)))\n",
        );
    }
    if wasi_host && used_extern_defs.contains_key("random!") {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"random_get\" (func $__wasi_random_get (param i32 i32) (result i32)))\n",
        );
    }
    let wasi_needs_file_read = used_extern_defs.contains_key("read!")
        || used_extern_defs.contains_key("read/chunks!")
        || used_extern_defs.contains_key("read/lines!");
    let wasi_needs_stdin_read =
        used_extern_defs.contains_key("stdin!") || used_extern_defs.contains_key("stdin/chunks!");
    if wasi_host && (wasi_needs_stdin_read || wasi_needs_file_read) {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"fd_read\" (func $__wasi_fd_read (param i32 i32 i32 i32) (result i32)))\n",
        );
    }
    if wasi_host && uses_argv {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"args_sizes_get\" (func $__wasi_args_sizes_get (param i32 i32) (result i32)))\n  (import \"wasi_snapshot_preview1\" \"args_get\" (func $__wasi_args_get (param i32 i32) (result i32)))\n",
        );
    }
    let wasi_uses_path = wasi_needs_file_read
        || used_extern_defs.contains_key("write!")
        || used_extern_defs.contains_key("list-dir!")
        || used_extern_defs.contains_key("mkdir!")
        || used_extern_defs.contains_key("delete!")
        || used_extern_defs.contains_key("move!");
    if wasi_host && wasi_uses_path {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"path_open\" (func $__wasi_path_open (param i32 i32 i32 i32 i32 i64 i64 i32 i32) (result i32)))\n  (import \"wasi_snapshot_preview1\" \"fd_close\" (func $__wasi_fd_close (param i32) (result i32)))\n",
        );
    }
    if wasi_host && used_extern_defs.contains_key("mkdir!") {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"path_create_directory\" (func $__wasi_path_create_directory (param i32 i32 i32) (result i32)))\n",
        );
    }
    if wasi_host && used_extern_defs.contains_key("delete!") {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"path_unlink_file\" (func $__wasi_path_unlink_file (param i32 i32 i32) (result i32)))\n  (import \"wasi_snapshot_preview1\" \"path_remove_directory\" (func $__wasi_path_remove_directory (param i32 i32 i32) (result i32)))\n",
        );
    }
    if wasi_host && used_extern_defs.contains_key("move!") {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"path_rename\" (func $__wasi_path_rename (param i32 i32 i32 i32 i32 i32) (result i32)))\n",
        );
    }
    let wasi_needs_list_dir =
        used_extern_defs.contains_key("list-dir!") || used_extern_defs.contains_key("delete!");
    if wasi_host && wasi_needs_list_dir {
        extern_imports.push_str(
            "  (import \"wasi_snapshot_preview1\" \"fd_readdir\" (func $__wasi_fd_readdir (param i32 i32 i32 i64 i32) (result i32)))\n",
        );
    }
    let generated_code = emitted_funcs
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(main_code.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    if generated_code.contains("call $__que_serialize") {
        extern_imports.push_str(
            "  (import \"host\" \"serialize\" (func $__que_serialize (param i32 i32) (result i32)))\n",
        );
    }
    if generated_code.contains("call $__que_deserialize") {
        extern_imports.push_str(
            "  (import \"host\" \"deserialize\" (func $__que_deserialize (param i32 i32) (result i32)))\n",
        );
    }
    // WebAssembly requires every import to precede every function definition.
    if wasi_guard_diagnostics {
        extern_imports.push_str(&emit_wasi_guard_trap_runtime());
    }
    if wasi_host
        && (wasi_prints_result
            || used_extern_defs.contains_key("print!")
            || used_extern_defs.contains_key("write!"))
    {
        extern_imports.push_str(emit_wasi_print_runtime());
    }
    if wasi_host && used_extern_defs.contains_key("time!") {
        extern_imports.push_str(emit_wasi_clock_runtime());
    }
    if wasi_host && used_extern_defs.contains_key("random!") {
        extern_imports.push_str(emit_wasi_random_runtime());
    }
    if wasi_host && used_extern_defs.contains_key("sleep!") {
        extern_imports.push_str(emit_wasi_sleep_runtime());
    }
    if wasi_host && used_extern_defs.contains_key("clear!") {
        extern_imports.push_str(emit_wasi_clear_runtime());
    }
    if wasi_host && (wasi_needs_stdin_read || wasi_needs_file_read) {
        extern_imports.push_str(emit_wasi_stdin_runtime());
    }
    if wasi_host && uses_argv {
        extern_imports.push_str(emit_wasi_argv_runtime());
    }
    if wasi_host && wasi_uses_path {
        extern_imports.push_str(&emit_wasi_file_runtime(
            wasi_needs_file_read,
            used_extern_defs.contains_key("write!"),
        ));
    }
    if wasi_host && wasi_needs_list_dir {
        extern_imports.push_str(emit_wasi_list_dir_runtime());
    }
    if wasi_host
        && (used_extern_defs.contains_key("mkdir!")
            || used_extern_defs.contains_key("delete!")
            || used_extern_defs.contains_key("move!"))
    {
        extern_imports.push_str(&emit_wasi_path_mutation_runtime(
            used_extern_defs.contains_key("mkdir!"),
            used_extern_defs.contains_key("delete!"),
            used_extern_defs.contains_key("move!"),
        ));
    }
    if wasi_host
        && (used_extern_defs.contains_key("read/chunks!")
            || used_extern_defs.contains_key("stdin/chunks!")
            || used_extern_defs.contains_key("read/lines!"))
    {
        extern_imports.push_str(&emit_wasi_chunk_runtime(
            used_extern_defs.contains_key("read/chunks!"),
            used_extern_defs.contains_key("stdin/chunks!"),
            used_extern_defs.contains_key("read/lines!"),
        ));
    }
    if wasi_host && (wasi_prints_result || generated_code.contains("call $__wasi_serialize_")) {
        let scale = decimal_scale_i64();
        let digits = scale.to_string().len().saturating_sub(1);
        extern_imports.push_str(
            &include_str!("wasi_serialize.wat")
                .replace("__DEC_SCALE__", &scale.to_string())
                .replace("__DEC_DIGITS__", &digits.to_string()),
        );
        let mut types = Vec::new();
        collect_wasi_serialize_types(typed_ast, &mut types);
        if wasi_prints_result
            && !matches!(
                main_ret_ty,
                Type::Char | Type::Unit | Type::Var(_) | Type::Function(_, _)
            )
            && !matches!(main_ret_ty, Type::List(inner) if matches!(inner.as_ref(), Type::Char))
            && !types.iter().any(|existing| existing == main_ret_ty)
        {
            types.push(main_ret_ty.clone());
        }
        let mut emitted = HashSet::new();
        for typ in types {
            extern_imports.push_str(&emit_wasi_serializer(&typ, &mut emitted)?);
        }
    }
    if wasi_host && generated_code.contains("call $__wasi_deserialize_") {
        let scale = decimal_scale_i64();
        let digits = scale.to_string().len().saturating_sub(1);
        extern_imports.push_str(
            &include_str!("wasi_deserialize.wat")
                .replace("__DEC_SCALE__", &scale.to_string())
                .replace("__DEC_DIGITS__", &digits.to_string()),
        );
        let mut types = Vec::new();
        collect_wasi_deserialize_types(typed_ast, &mut types);
        let mut emitted = HashSet::new();
        for typ in types {
            extern_imports.push_str(&emit_wasi_deserializer(&typ, &mut emitted)?);
        }
    }

    let mut cached_globals = String::new();
    for name in &cached_value_defs {
        cached_globals.push_str(&format!(
            "  (global ${} (mut i32) (i32.const 0))\n",
            cache_init_global(name)
        ));
        cached_globals.push_str(&format!(
            "  (global ${} (mut i32) (i32.const 0))\n",
            cache_value_global(name)
        ));
    }

    let runtime_body = emit_vector_runtime(&fn_ids, &fn_sigs, &closure_defs, &apply_arities);
    let (generic_runtime_body, apply_runtime_body) =
        split_generic_runtime_and_apply_runtime(&runtime_body);
    let mut emitted_funcs_body = String::new();
    for func in &emitted_funcs {
        emitted_funcs_body.push_str(func);
    }

    let mut monolithic_wat = String::new();
    monolithic_wat.push_str(&format!(";; Type: {}\n", abi_type_descriptor(main_ret_ty)));
    monolithic_wat.push_str("(module\n");
    monolithic_wat.push_str(&extern_imports);
    monolithic_wat.push_str(&cached_globals);
    monolithic_wat.push_str(&runtime_body);
    monolithic_wat.push_str(&emitted_funcs_body);
    monolithic_wat.push_str(&main_func);
    monolithic_wat.push_str(")\n");

    let mut runtime_wat = String::new();
    runtime_wat.push_str(";; Que runtime module\n");
    runtime_wat.push_str("(module\n");
    runtime_wat.push_str(generic_runtime_body);
    runtime_wat.push_str(&emit_runtime_func_exports(generic_runtime_body));
    runtime_wat.push_str(")\n");

    let mut user_body = String::new();
    user_body.push_str(apply_runtime_body);
    user_body.push_str(&emitted_funcs_body);
    user_body.push_str(&main_func);

    let mut user_wat = String::new();
    user_wat.push_str(&format!(";; Type: {}\n", abi_type_descriptor(main_ret_ty)));
    user_wat.push_str(";; Que user module; imports runtime helpers from module \"que_runtime\".\n");
    user_wat.push_str("(module\n");
    user_wat.push_str(&extern_imports);
    user_wat.push_str(&emit_runtime_imports(generic_runtime_body, &user_body));
    user_wat.push_str(&cached_globals);
    user_wat.push_str(&user_body);
    user_wat.push_str(")\n");

    Ok(WatBuildOutput {
        monolithic_wat,
        split: SplitWatModules {
            runtime_wat,
            user_wat,
        },
    })
}

pub fn compile_program_to_wat_typed_with_opts(
    typed_ast: &TypedExpression,
    enable_optimizer: bool,
) -> Result<String, String> {
    compile_program_to_wat_build_typed_with_opts(typed_ast, enable_optimizer)
        .map(|output| output.monolithic_wat)
}

pub fn compile_program_to_split_wat_typed_with_opts(
    typed_ast: &TypedExpression,
    enable_optimizer: bool,
) -> Result<SplitWatModules, String> {
    compile_program_to_wat_build_typed_with_opts(typed_ast, enable_optimizer)
        .map(|output| output.split)
}

pub fn compile_program_to_wat_typed(typed_ast: &TypedExpression) -> Result<String, String> {
    compile_program_to_wat_typed_with_opts(typed_ast, true)
}

pub fn compile_program_to_split_wat_typed(
    typed_ast: &TypedExpression,
) -> Result<SplitWatModules, String> {
    compile_program_to_split_wat_typed_with_opts(typed_ast, true)
}

pub fn compile_program_to_wat_with_opts(
    expr: &Expression,
    enable_optimizer: bool,
) -> Result<String, String> {
    let wrapped = crate::externals::prepend_builtin_host_externs(expr)?;
    let (_typ, typed_ast) = crate::infer::infer_with_builtins_typed(
        &wrapped,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
    )?;
    compile_program_to_wat_typed_with_opts(&typed_ast, enable_optimizer)
}

pub fn compile_program_to_split_wat_with_opts(
    expr: &Expression,
    enable_optimizer: bool,
) -> Result<SplitWatModules, String> {
    let wrapped = crate::externals::prepend_builtin_host_externs(expr)?;
    let (_typ, typed_ast) = crate::infer::infer_with_builtins_typed(
        &wrapped,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
    )?;
    compile_program_to_split_wat_typed_with_opts(&typed_ast, enable_optimizer)
}

pub fn compile_program_to_wat(expr: &Expression) -> Result<String, String> {
    compile_program_to_wat_with_opts(expr, true)
}

pub fn compile_program_to_split_wat(expr: &Expression) -> Result<SplitWatModules, String> {
    compile_program_to_split_wat_with_opts(expr, true)
}
