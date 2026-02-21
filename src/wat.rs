use crate::infer::TypedExpression;
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

struct Ctx<'a> {
    fn_sigs: &'a HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &'a HashMap<String, i32>,
    lambda_ids: &'a HashMap<String, i32>,
    lambda_bindings: &'a HashMap<String, TypedExpression>,
    locals: HashMap<String, usize>,
    tmp_i32: usize,
}
const EXTRA_I32_LOCALS: usize = 16;

#[derive(Clone, Copy)]
enum VecElemKind {
    I32,
}

fn builtin_fn_tag(name: &str) -> Option<i32> {
    let core = name.rsplit('/').next().unwrap_or(name);
    match core {
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
        _ => None,
    }
}

fn is_i32ish_type(t: &Type) -> bool {
    matches!(
        t,
        Type::Int |
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
    matches!(t, Type::List(_))
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
            _ => s.push('_'),
        }
    }
    let mut s = String::new();
    for c in name.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => s.push(c.to_ascii_lowercase()),
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

fn wasm_val_type(typ: &Type) -> Result<&'static str, String> {
    match typ {
        Type::Int | Type::Bool | Type::Char | Type::Unit => Ok("i32"),
        Type::Float => Err("Float is currently unsupported in wasm backend".to_string()),
        Type::List(_) | Type::Tuple(_) => Ok("i32"),
        Type::Var(_) => Ok("i32"),
        Type::Function(_, _) => Ok("i32"),
    }
}

fn vec_elem_kind_from_type(typ: &Type) -> Result<VecElemKind, String> {
    match typ {
        Type::Float => Err("Float vectors are currently unsupported in wasm backend".to_string()),
        Type::Int | Type::Bool | Type::Char | Type::Unit | Type::List(_) | Type::Tuple(_) => {
            Ok(VecElemKind::I32)
        }
        Type::Var(_) => Ok(VecElemKind::I32),
        Type::Function(_, _) =>
            Err("Function values inside vectors are not supported in wasm backend".to_string()),
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
            "let~" |
            "let*" |
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
            "pop!" |
            "loop" |
            "loop-finish" |
            "as" |
            "char" |
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
            "Int->Float" |
            "Float->Int" |
            "true" |
            "false"
    )
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
                        if let Expression::Word(name) = p {
                            scoped.insert(name.clone());
                        }
                    }
                    if let Some(body) = items.last() {
                        collect_refs(body, &mut scoped, out);
                    }
                    return;
                }
                if op == "let" || op == "let~" || op == "let*" {
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
            if kw == "let" || kw == "let~" || kw == "let*" {
                if let Some(rhs) = node.children.get(2) {
                    if
                        matches!(&rhs.expr, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                    {
                        out.insert(name.clone(), rhs.clone());
                    }
                }
            }
        }
    }
    for ch in &node.children {
        collect_let_lambda_bindings(ch, out);
    }
}

fn lambda_is_hoistable(node: &TypedExpression, top_defs: &HashMap<String, TopDef>) -> bool {
    let items = match &node.expr {
        Expression::Apply(xs) => xs,
        _ => {
            return false;
        }
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") || items.len() < 3 {
        return false;
    }
    let mut bound = HashSet::new();
    for p in &items[1..items.len() - 1] {
        if let Expression::Word(n) = p {
            bound.insert(n.clone());
        }
    }
    let mut refs = HashSet::new();
    if let Some(body) = items.last() {
        collect_refs(body, &mut bound, &mut refs);
    }
    refs.into_iter().all(|r| {
        top_defs
            .get(&r)
            .map(|d| {
                matches!(
                &d.expr,
                Expression::Apply(xs)
                    if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
            )
            })
            .unwrap_or(false)
    })
}

fn emit_vector_runtime(
    fn_ids: &HashMap<String, i32>,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>
) -> String {
    let mut out = String::new();
    out.push_str(
        r#"
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 1024))
  (global $free_head (mut i32) (i32.const 0))

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
    local.get $ptr
    i32.const 8
    i32.sub
    local.set $base
    local.get $base
    i32.load
    local.set $size
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

  (func $rc_retain (param $ptr i32) (result i32)
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

  (func $rc_release (param $ptr i32) (result i32)
    (local $rc i32)
    (local $len i32)
    (local $i i32)
    (local $elem_ref i32)
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
    local.tee $rc
    i32.const 1
    i32.sub
    local.tee $rc
    local.get $ptr
    i32.const 8
    i32.add
    i32.store
    local.get $rc
    i32.const 0
    i32.gt_s
    if
      i32.const 0
      return
    end
    local.get $ptr
    i32.const 12
    i32.add
    i32.load
    local.set $elem_ref
    local.get $elem_ref
    i32.const 0
    i32.eq
    if
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
        local.get $ptr
        i32.const 16
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
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $ptr
    call $free
    drop
    i32.const 0
  )

  (func $tuple_new (param $a i32) (param $b i32) (result i32)
    (local $ptr i32)
    i32.const 8
    call $alloc
    local.set $ptr
    local.get $ptr
    local.get $a
    i32.store
    local.get $ptr
    i32.const 4
    i32.add
    local.get $b
    i32.store
    local.get $ptr
  )

  (func $tuple_fst (param $ptr i32) (result i32)
    local.get $ptr
    i32.load
  )

  (func $tuple_snd (param $ptr i32) (result i32)
    local.get $ptr
    i32.const 4
    i32.add
    i32.load
  )

  (func $vec_new_i32 (param $len i32) (param $elem_ref i32) (result i32)
    (local $cap i32)
    (local $ptr i32)
    local.get $len
    i32.const 64
    i32.lt_s
    if (result i32)
      i32.const 64
    else
      local.get $len
    end
    local.set $cap
    i32.const 16
    local.get $cap
    i32.const 4
    i32.mul
    i32.add
    call $alloc
    local.set $ptr
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
  )

  (func $vec_new_f32 (param $len i32) (result i32)
    (local $cap i32)
    (local $ptr i32)
    local.get $len
    i32.const 64
    i32.lt_s
    if (result i32)
      i32.const 64
    else
      local.get $len
    end
    local.set $cap
    i32.const 16
    local.get $cap
    i32.const 8
    i32.mul
    i32.add
    call $alloc
    local.set $ptr
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
  )

  (func $vec_get_i32 (param $ptr i32) (param $idx i32) (result i32)
    local.get $ptr
    i32.const 16
    i32.add
    local.get $idx
    i32.const 4
    i32.mul
    i32.add
    i32.load
  )

  (func $vec_get_f32 (param $ptr i32) (param $idx i32) (result f32)
    local.get $ptr
    i32.const 16
    i32.add
    local.get $idx
    i32.const 8
    i32.mul
    i32.add
    f32.load
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
    if
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
    unreachable
  )

  (func $vec_push_f32 (param $ptr i32) (param $v f32) (result i32)
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
    local.get $len
    local.get $cap
    i32.lt_s
    if
      local.get $ptr
      i32.const 16
      i32.add
      local.get $len
      i32.const 8
      i32.mul
      i32.add
      local.set $addr
      local.get $addr
      local.get $v
      f32.store
      local.get $ptr
      local.get $len
      i32.const 1
      i32.add
      i32.store
      i32.const 0
      return
    end
    unreachable
  )

  (func $vec_set_i32 (param $ptr i32) (param $idx i32) (param $v i32) (result i32)
    (local $len i32)
    (local $cap i32)
    (local $addr i32)
    (local $elem_ref i32)
    (local $old i32)
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

    local.get $idx
    local.get $len
    i32.eq
    if
      local.get $len
      local.get $cap
      i32.lt_s
      if
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
      unreachable
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
      local.get $idx
      i32.const 4
      i32.mul
      i32.add
      local.set $addr
      local.get $elem_ref
      i32.const 0
      i32.ne
      if
        local.get $addr
        i32.load
        local.set $old
        local.get $old
        call $rc_release
        drop
        local.get $v
        call $rc_retain
        drop
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

  (func $vec_set_f32 (param $ptr i32) (param $idx i32) (param $v f32) (result i32)
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
      if
        local.get $ptr
        i32.const 16
        i32.add
        local.get $len
        i32.const 8
        i32.mul
        i32.add
        local.set $addr
        local.get $addr
        local.get $v
        f32.store
        local.get $ptr
        local.get $len
        i32.const 1
        i32.add
        i32.store
        i32.const 0
        return
      end
      unreachable
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
      local.get $idx
      i32.const 8
      i32.mul
      i32.add
      local.set $addr
      local.get $addr
      local.get $v
      f32.store
      i32.const 0
      return
    end

    unreachable
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

  (func $vec_slice_f32 (param $ptr i32) (param $start i32) (result i32)
    (local $len i32)
    (local $new_len i32)
    (local $i i32)
    (local $out i32)
    local.get $ptr
    i32.load
    local.set $len

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
        call $vec_new_f32
      else
        local.get $len
        local.get $start
        i32.sub
        local.set $new_len
        local.get $new_len
        call $vec_new_f32
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
            call $vec_get_f32
            call $vec_set_f32
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

  (func $apply1_i32 (param $f i32) (param $a i32) (result i32)
"#
    );
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
        unreachable
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
    out.push_str(
        r#"
  )

  (func $apply2_i32 (param $f i32) (param $a i32) (param $b i32) (result i32)
"#
    );
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
                                      unreachable
                                    end
    "#
    );
    for _ in 0..apply2_open_ends {
        out.push_str("                                    end\n");
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
  )

  (func $apply2_f32 (param $f i32) (param $a f32) (param $b f32) (result f32)
    local.get $f
    i32.const 31
    i32.eq
    if (result f32)
      local.get $a
      local.get $b
      f32.add
    else
      local.get $f
      i32.const 32
      i32.eq
      if (result f32)
        local.get $a
        local.get $b
        f32.sub
      else
        local.get $f
        i32.const 33
        i32.eq
        if (result f32)
          local.get $a
          local.get $b
          f32.mul
        else
          local.get $f
          i32.const 34
          i32.eq
          if (result f32)
            local.get $a
            local.get $b
            f32.div
          else
            unreachable
          end
        end
      end
    end
  )
"#
    );

    out.push_str(
        "  (func $apply3_i32 (param $f i32) (param $a i32) (param $b i32) (param $c i32) (result i32)\n"
    );
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
    out.push_str("      unreachable\n");
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
    out.push_str("  )\n");
    out
}

fn emit_builtin(op: &str, node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let a = node.children
        .get(1)
        .ok_or_else(|| format!("Missing lhs for {}", op))
        .and_then(|n| compile_expr(n, ctx))?;
    let b = node.children
        .get(2)
        .ok_or_else(|| format!("Missing rhs for {}", op))
        .and_then(|n| compile_expr(n, ctx))?;
    let code = match op {
        "+" | "+#" => "i32.add",
        "-" | "-#" => "i32.sub",
        "*" | "*#" => "i32.mul",
        "/" | "/#" => "i32.div_s",
        "mod" => "i32.rem_s",
        "=" | "=?" | "=#" => "i32.eq",
        "<" | "<#" => "i32.lt_s",
        ">" | ">#" => "i32.gt_s",
        "<=" | "<=#" => "i32.le_s",
        ">=" | ">=#" => "i32.ge_s",
        "and" => "i32.and",
        "or" => "i32.or",
        "^" => "i32.xor",
        "|" => "i32.or",
        "&" => "i32.and",
        "<<" => "i32.shl",
        ">>" => "i32.shr_s",
        "+." | "-." | "*." | "/." | "mod." | "=." | "<." | ">." | "<=." | ">=." => {
            return Err("Float is currently unsupported in wasm backend".to_string());
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
    let mut parts = Vec::new();
    let mut scoped_lambda_bindings = ctx.lambda_bindings.clone();
    for i in 1..items.len() - 1 {
        if let Expression::Apply(let_items) = &items[i] {
            if let [Expression::Word(kw), Expression::Word(name), _] = &let_items[..] {
                if kw == "let" || kw == "let~" || kw == "let*" {
                    let val_node = child_at(i).and_then(|n| n.children.get(2));
                    if let Some(n) = val_node {
                        if
                            matches!(&n.expr, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                        {
                            scoped_lambda_bindings.insert(name.clone(), n.clone());
                        }
                    }
                    let value = val_node
                        .ok_or_else(|| format!("Missing let value for {}", name))
                        .and_then(|n| {
                            let scoped_ctx = Ctx {
                                fn_sigs: ctx.fn_sigs,
                                fn_ids: ctx.fn_ids,
                                lambda_ids: ctx.lambda_ids,
                                lambda_bindings: &scoped_lambda_bindings,
                                locals: ctx.locals.clone(),
                                tmp_i32: ctx.tmp_i32,
                            };
                            if
                                matches!(&n.expr, Expression::Apply(xs) if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda"))
                            {
                                match compile_expr(n, &scoped_ctx) {
                                    Ok(v) => Ok(v),
                                    Err(e) if e.contains("missing lowering id") =>
                                        Ok("i32.const 0".to_string()),
                                    Err(e) => Err(e),
                                }
                            } else {
                                compile_expr(n, &scoped_ctx)
                            }
                        })?;
                    if let Some(local_idx) = ctx.locals.get(name) {
                        parts.push(format!("{value}\nlocal.set {}", local_idx));
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
                lambda_bindings: &scoped_lambda_bindings,
                locals: ctx.locals.clone(),
                tmp_i32: ctx.tmp_i32,
            };
            let c = compile_expr(n, &scoped_ctx)?;
            parts.push(format!("{c}\ndrop"));
        }
    }
    let last = child_at(items.len() - 1)
        .ok_or_else(|| "Missing final do expression".to_string())
        .and_then(|n| {
            let scoped_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                lambda_ids: ctx.lambda_ids,
                lambda_bindings: &scoped_lambda_bindings,
                locals: ctx.locals.clone(),
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
            lambda_bindings: ctx.lambda_bindings,
            locals: ctx.locals.clone(),
            tmp_i32: ctx.tmp_i32 + 1,
        };
        let v = compile_expr(a, &nested_ctx)?;
        out.push(
            format!("local.get {}\n{}\ncall $vec_push_{}\ndrop", ctx.tmp_i32, v, elem_kind.suffix())
        );
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
    let xs = compile_expr(
        node.children.get(1).ok_or_else(|| "set! missing vector".to_string())?,
        ctx
    )?;
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
    Ok(format!("{xs}\n{idx}\n{v}\ncall $vec_set_{}", elem.suffix()))
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

fn compile_loop(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let start = compile_expr(
        node.children.get(1).ok_or_else(|| "loop missing start".to_string())?,
        ctx
    )?;
    let end = compile_expr(
        node.children.get(2).ok_or_else(|| "loop missing end".to_string())?,
        ctx
    )?;
    let fn_node = node.children.get(3).ok_or_else(|| "loop missing fn".to_string())?;

    let i_local = ctx.tmp_i32 + 1;
    let end_local = ctx.tmp_i32 + 2;

    let body_and_drop = match &fn_node.expr {
        Expression::Apply(items) if
            matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
        => {
            if items.len() < 3 {
                return Err("loop lambda must have one param and a body".to_string());
            }
            let param = match &items[1] {
                Expression::Word(w) => w.clone(),
                _ => {
                    return Err("loop lambda param must be a word".to_string());
                }
            };
            let body_idx = items.len() - 1;
            let body_node = fn_node.children
                .get(body_idx)
                .ok_or_else(|| "loop lambda missing typed body".to_string())?;
            let mut locals = ctx.locals.clone();
            locals.insert(param, i_local);
            let body_ctx = Ctx {
                fn_sigs: ctx.fn_sigs,
                fn_ids: ctx.fn_ids,
                lambda_ids: ctx.lambda_ids,
                lambda_bindings: ctx.lambda_bindings,
                locals,
                tmp_i32: ctx.tmp_i32,
            };
            let body = compile_expr(body_node, &body_ctx)?;
            format!("{body}\ndrop")
        }
        Expression::Word(name) => {
            if let Some(lambda_node) = ctx.lambda_bindings.get(name) {
                let items = match &lambda_node.expr {
                    Expression::Apply(xs) => xs,
                    _ => {
                        return Err(format!("loop local '{}' is not a lambda", name));
                    }
                };
                if items.len() < 3 {
                    return Err(format!("loop local lambda '{}' missing body", name));
                }
                let param = match &items[1] {
                    Expression::Word(w) => w.clone(),
                    _ => {
                        return Err(format!("loop local lambda '{}' has non-word param", name));
                    }
                };
                let body_idx = items.len() - 1;
                let body_node = lambda_node.children
                    .get(body_idx)
                    .ok_or_else(|| format!("loop local lambda '{}' missing typed body", name))?;
                let mut locals = ctx.locals.clone();
                locals.insert(param, i_local);
                let body_ctx = Ctx {
                    fn_sigs: ctx.fn_sigs,
                    fn_ids: ctx.fn_ids,
                    lambda_ids: ctx.lambda_ids,
                    lambda_bindings: ctx.lambda_bindings,
                    locals,
                    tmp_i32: ctx.tmp_i32,
                };
                let body = compile_expr(body_node, &body_ctx)?;
                format!("{body}\ndrop")
            } else {
                let (params, _ret) = ctx.fn_sigs
                    .get(name)
                    .ok_or_else(|| format!("Unknown loop fn '{}'", name))?;
                if params.len() == 1 {
                    format!("local.get {i_local}\ncall ${}\ndrop", ident(name))
                } else {
                    return Err(
                        format!("loop fn '{}' must take exactly one argument in wasm backend", name)
                    );
                }
            }
        }
        _ => {
            return Err("Unsupported loop function form in wasm backend".to_string());
        }
    };

    Ok(
        format!(
            "{start}\nlocal.set {i_local}\n{end}\nlocal.set {end_local}\nblock\n  loop\n    local.get {i_local}\n    local.get {end_local}\n    i32.ge_s\n    br_if 1\n    {body_and_drop}\n    local.get {i_local}\n    i32.const 1\n    i32.add\n    local.set {i_local}\n    br 0\n  end\nend\ni32.const 0"
        )
    )
}

fn compile_loop_finish(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let cond = compile_expr(
        node.children.get(1).ok_or_else(|| "loop-finish missing condition".to_string())?,
        ctx
    )?;
    let fn_node = node.children.get(2).ok_or_else(|| "loop-finish missing fn".to_string())?;

    let body_and_drop = match &fn_node.expr {
        Expression::Apply(items) if
            matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
        => {
            if items.len() < 2 {
                return Err("loop-finish lambda missing body".to_string());
            }
            // zero-arg thunk only
            if items.len() != 2 {
                return Err("loop-finish lambda must have 0 params".to_string());
            }
            let body_node = fn_node.children
                .last()
                .ok_or_else(|| "loop-finish lambda missing typed body".to_string())?;
            let body = compile_expr(body_node, ctx)?;
            format!("{body}\ndrop")
        }
        Expression::Word(name) => {
            if let Some(lambda_node) = ctx.lambda_bindings.get(name) {
                let items = match &lambda_node.expr {
                    Expression::Apply(xs) => xs,
                    _ => {
                        return Err(format!("loop-finish local '{}' is not a lambda", name));
                    }
                };
                if items.len() != 2 {
                    return Err(format!("loop-finish local lambda '{}' must take 0 args", name));
                }
                let body_node = lambda_node.children
                    .last()
                    .ok_or_else(||
                        format!("loop-finish local lambda '{}' missing typed body", name)
                    )?;
                let body = compile_expr(body_node, ctx)?;
                format!("{body}\ndrop")
            } else if let Some((params, _ret)) = ctx.fn_sigs.get(name) {
                if params.is_empty() {
                    format!("call ${}\ndrop", ident(name))
                } else {
                    return Err(
                        format!("loop-finish fn '{}' must take 0 args in wasm backend", name)
                    );
                }
            } else {
                return Err(format!("Unknown loop-finish function '{}'", name));
            }
        }
        _ => {
            return Err("Unsupported loop-finish function form in wasm backend".to_string());
        }
    };

    Ok(
        format!(
            "block\n  loop\n    {cond}\n    i32.eqz\n    br_if 1\n    {body_and_drop}\n    br 0\n  end\nend\ni32.const 0"
        )
    )
}

fn compile_call(node: &TypedExpression, op: &str, ctx: &Ctx<'_>) -> Result<String, String> {
    let (params, _ret) = if let Some(sig) = ctx.fn_sigs.get(op) {
        sig.clone()
    } else if builtin_fn_tag(op).is_some() {
        // Builtin used via namespaced value (e.g. std/vector/set!) without explicit top def.
        (Vec::new(), Type::Int)
    } else {
        return Err(format!("Unknown function '{}'", op));
    };
    let args = &node.children[1..];
    if params.is_empty() && !args.is_empty() {
        let mut out = vec![format!("call ${}", ident(op))];
        match args.len() {
            1 => {
                out.push(compile_expr(&args[0], ctx)?);
                out.push("call $apply1_i32".to_string());
            }
            2 => {
                out.push(compile_expr(&args[0], ctx)?);
                out.push(compile_expr(&args[1], ctx)?);
                out.push("call $apply2_i32".to_string());
            }
            3 => {
                out.push(compile_expr(&args[0], ctx)?);
                out.push(compile_expr(&args[1], ctx)?);
                out.push(compile_expr(&args[2], ctx)?);
                out.push("call $apply3_i32".to_string());
            }
            n => {
                return Err(
                    format!("Dynamic function application with {} args is not supported in wasm backend", n)
                );
            }
        }
        return Ok(out.join("\n"));
    }
    let unit_arity_elided = params.len() == 1 && matches!(params[0], Type::Unit) && args.is_empty();
    if args.len() != params.len() && !unit_arity_elided {
        return Err(
            format!(
                "Partial application/extra args not yet supported in wasm backend: '{}' expected {} args, got {}",
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
    let f = compile_expr(
        node.children.first().ok_or_else(|| "call missing function".to_string())?,
        ctx
    )?;
    let args = &node.children[1..];
    match args.len() {
        1 => {
            let a = compile_expr(&args[0], ctx)?;
            Ok(format!("{f}\n{a}\ncall $apply1_i32"))
        }
        2 => {
            let a = compile_expr(&args[0], ctx)?;
            let b = compile_expr(&args[1], ctx)?;
            let float_call =
                matches!(node.typ.as_ref(), Some(Type::Float)) ||
                args.iter().any(|n| matches!(n.typ.as_ref(), Some(Type::Float)));
            if float_call {
                Ok(format!("{f}\n{a}\n{b}\ncall $apply2_f32"))
            } else {
                Ok(format!("{f}\n{a}\n{b}\ncall $apply2_i32"))
            }
        }
        3 => {
            let a = compile_expr(&args[0], ctx)?;
            let b = compile_expr(&args[1], ctx)?;
            let c = compile_expr(&args[2], ctx)?;
            Ok(format!("{f}\n{a}\n{b}\n{c}\ncall $apply3_i32"))
        }
        n =>
            Err(
                format!("Dynamic function application with {} args is not supported in wasm backend", n)
            ),
    }
}

fn compile_local_lambda_call(
    node: &TypedExpression,
    name: &str,
    ctx: &Ctx<'_>
) -> Result<String, String> {
    let lambda_node = ctx.lambda_bindings
        .get(name)
        .ok_or_else(|| format!("Unknown local lambda '{}'", name))?;
    let items = match &lambda_node.expr {
        Expression::Apply(xs)
            if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda") => xs,
        _ => {
            return Err(format!("Local '{}' is not a lambda", name));
        }
    };
    if items.len() < 2 {
        return Err(format!("Local lambda '{}' missing body", name));
    }
    let body_idx = items.len() - 1;
    let args = &node.children[1..];
    let param_exprs = &items[1..body_idx];
    if args.len() != param_exprs.len() {
        return Err(
            format!(
                "Partial application/extra args not yet supported in wasm backend: '{}' expected {} args, got {}",
                name,
                param_exprs.len(),
                args.len()
            )
        );
    }

    let param_local_base = ctx.tmp_i32;
    let arg_tmp_base = ctx.tmp_i32 + args.len() + 1;

    let mut out = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let arg_ctx = Ctx {
            fn_sigs: ctx.fn_sigs,
            fn_ids: ctx.fn_ids,
            lambda_ids: ctx.lambda_ids,
            lambda_bindings: ctx.lambda_bindings,
            locals: ctx.locals.clone(),
            tmp_i32: arg_tmp_base,
        };
        let arg_code = compile_expr(arg, &arg_ctx)?;
        out.push(format!("{arg_code}\nlocal.set {}", param_local_base + i));
    }

    let mut locals = ctx.locals.clone();
    for (i, p) in param_exprs.iter().enumerate() {
        let pname = match p {
            Expression::Word(w) => w.clone(),
            _ => {
                return Err(format!("Local lambda '{}' has non-word parameter", name));
            }
        };
        locals.insert(pname, param_local_base + i);
    }

    let body_node = lambda_node.children
        .get(body_idx)
        .ok_or_else(|| format!("Local lambda '{}' missing typed body", name))?;
    let body_ctx = Ctx {
        fn_sigs: ctx.fn_sigs,
        fn_ids: ctx.fn_ids,
        lambda_ids: ctx.lambda_ids,
        lambda_bindings: ctx.lambda_bindings,
        locals,
        tmp_i32: arg_tmp_base,
    };
    out.push(compile_expr(body_node, &body_ctx)?);
    Ok(out.join("\n"))
}

fn compile_lambda_literal(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    let key = node.expr.to_lisp();
    if let Some(id) = ctx.lambda_ids.get(&key) {
        Ok(format!("i32.const {}", id))
    } else {
        Err("Unsupported lambda literal in wasm backend (missing lowering id)".to_string())
    }
}

fn compile_expr(node: &TypedExpression, ctx: &Ctx<'_>) -> Result<String, String> {
    match &node.expr {
        Expression::Int(n) => Ok(format!("i32.const {}", n)),
        Expression::Float(_n) => {
            Err("Float is currently unsupported in wasm backend".to_string())
        }
        Expression::Word(w) =>
            match w.as_str() {
                "true" => Ok("i32.const 1".to_string()),
                "false" => Ok("i32.const 0".to_string()),
                _ => {
                    if let Some(local_idx) = ctx.locals.get(w) {
                        Ok(format!("local.get {}", local_idx))
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
                Expression::Word(op) =>
                    match op.as_str() {
                        _ if ctx.locals.contains_key(op) => {
                            if ctx.lambda_bindings.contains_key(op) {
                                compile_local_lambda_call(node, op, ctx)
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
                        "pop!" => compile_pop(node, ctx),
                        "loop" => compile_loop(node, ctx),
                        "loop-finish" => compile_loop_finish(node, ctx),
                        "not" => {
                            let a = compile_expr(
                                node.children.get(1).ok_or_else(|| "not missing arg".to_string())?,
                                ctx
                            )?;
                            Ok(format!("{a}\ni32.eqz"))
                        }
                        "Int->Float" | "Float->Int" => {
                            Err("Float is currently unsupported in wasm backend".to_string())
                        }
                        "as" | "char" =>
                            node.children
                                .get(1)
                                .map(|n| compile_expr(n, ctx))
                                .unwrap_or_else(|| Ok("i32.const 0".to_string())),
                        op if is_special_word(op) => emit_builtin(op, node, ctx),
                        _ => compile_call(node, op, ctx),
                    }
                _ =>
                    Err(
                        "Higher-order call heads are not yet supported in wasm backend".to_string()
                    ),
            }
        }
    }
}

fn collect_let_locals(node: &TypedExpression, out: &mut Vec<(String, Type)>) {
    if let Expression::Apply(items) = &node.expr {
        if let [Expression::Word(kw), Expression::Word(name), _] = &items[..] {
            if kw == "let" || kw == "let~" || kw == "let*" {
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

fn compile_lambda_func(
    name: &str,
    lambda_expr: &Expression,
    lambda_node: &TypedExpression,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &HashMap<String, i32>,
    lambda_ids: &HashMap<String, i32>,
    lambda_bindings: &HashMap<String, TypedExpression>
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
    let body_node = lambda_node.children
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
                    .ok_or_else(||
                        format!("Missing specialized param type for '{}' arg {}", name, i)
                    )?
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
    let mut local_lambda_bindings = HashMap::new();
    collect_let_lambda_bindings(body_node, &mut local_lambda_bindings);
    for (k, v) in local_lambda_bindings {
        scoped_lambda_bindings.insert(k, v);
    }

    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        lambda_ids,
        lambda_bindings: &scoped_lambda_bindings,
        locals,
        tmp_i32,
    };
    let body_code = compile_expr(body_node, &ctx)?;
    let ret_is_ref = is_ref_type(&ret_ty);
    let mut ref_slots: Vec<usize> = Vec::new();
    for (i, (_n, t)) in local_defs.iter().enumerate() {
        if is_ref_type(t) {
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
    for slot in ref_slots {
        if ret_is_ref {
            out.push_str(&format!("    local.get {}\n", slot));
            out.push_str(&format!("    local.get {}\n", ret_slot));
            out.push_str("    i32.eq\n");
            out.push_str("    if\n");
            out.push_str("    else\n");
            out.push_str(&format!("      local.get {}\n", slot));
            out.push_str("      call $rc_release\n");
            out.push_str("      drop\n");
            out.push_str("    end\n");
        } else {
            out.push_str(&format!("    local.get {}\n", slot));
            out.push_str("    call $rc_release\n");
            out.push_str("    drop\n");
        }
    }
    out.push_str(&format!("    local.get {}\n", ret_slot));
    out.push_str("  )\n");
    Ok(out)
}

fn compile_value_func(
    name: &str,
    value_node: &TypedExpression,
    fn_sigs: &HashMap<String, (Vec<Type>, Type)>,
    fn_ids: &HashMap<String, i32>,
    lambda_ids: &HashMap<String, i32>,
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

    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        lambda_ids,
        lambda_bindings: &scoped_lambda_bindings,
        locals,
        tmp_i32,
    };
    let body_code = compile_expr(value_node, &ctx)?;
    let ret_is_ref = is_ref_type(ret_ty);
    let ref_slots: Vec<usize> = local_defs
        .iter()
        .enumerate()
        .filter_map(|(i, (_n, t))| if is_ref_type(t) { Some(i) } else { None })
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
    out.push_str(&format!("    {}\n", body_code.replace('\n', "\n    ")));
    out.push_str(&format!("    local.set {}\n", ret_slot));
    for slot in ref_slots {
        if ret_is_ref {
            out.push_str(&format!("    local.get {}\n", slot));
            out.push_str(&format!("    local.get {}\n", ret_slot));
            out.push_str("    i32.eq\n");
            out.push_str("    if\n");
            out.push_str("    else\n");
            out.push_str(&format!("      local.get {}\n", slot));
            out.push_str("      call $rc_release\n");
            out.push_str("      drop\n");
            out.push_str("    end\n");
        } else {
            out.push_str(&format!("    local.get {}\n", slot));
            out.push_str("    call $rc_release\n");
            out.push_str("    drop\n");
        }
    }
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
    lambda_bindings: &HashMap<String, TypedExpression>
) -> Result<String, String> {
    let mut locals = HashMap::new();
    for i in 0..h.remaining_params.len() {
        locals.insert(format!("__p{}", i), i);
    }
    let ctx = Ctx {
        fn_sigs,
        fn_ids,
        lambda_ids,
        lambda_bindings,
        locals,
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

pub fn compile_program_to_wat_typed(typed_ast: &TypedExpression) -> Result<String, String> {
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
                        if kw == "let" || kw == "let~" || kw == "let*" {
                            if
                                let Some(node) = child_at(i)
                                    .and_then(|n| n.children.get(2))
                                    .cloned()
                            {
                                defs.insert(name.clone(), TopDef {
                                    expr: rhs.clone(),
                                    node,
                                });
                                let is_lambda_rhs =
                                    matches!(
                                    rhs,
                                    Expression::Apply(xs)
                                        if matches!(xs.first(), Some(Expression::Word(w)) if w == "lambda")
                                );
                                let is_function_value = matches!(
                                    defs.get(name).and_then(|d| d.node.typ.as_ref()),
                                    Some(Type::Function(_, _))
                                );
                                if is_lambda_rhs || is_function_value {
                                    continue;
                                }
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
                    children: Vec::new(),
                });
            }
            let main_expr = Expression::Apply(main_items_expr);
            let main_typ = main_items_nodes.last().and_then(|n| n.typ.clone());
            let main_node = TypedExpression {
                expr: main_expr.clone(),
                typ: main_typ,
                children: main_items_nodes,
            };
            (defs, main_expr, main_node)
        }
        _ => (HashMap::new(), typed_ast.expr.clone(), typed_ast.clone()),
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

    let mut fn_sigs: HashMap<String, (Vec<Type>, Type)> = HashMap::new();
    let top_def_names: HashSet<String> = top_defs.keys().cloned().collect();
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
            let (decl_ps, decl_ret) = function_parts(t);
            if let Some((sp, sr)) = call_specs.get(name) {
                if sp.len() == decl_ps.len() {
                    (sp.clone(), sr.clone())
                } else {
                    (decl_ps, decl_ret)
                }
            } else {
                (decl_ps, decl_ret)
            }
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
    }
    let mut lambda_nodes = Vec::new();
    collect_lambda_nodes(typed_ast, &mut lambda_nodes);
    let mut lambda_bindings: HashMap<String, TypedExpression> = HashMap::new();
    collect_let_lambda_bindings(typed_ast, &mut lambda_bindings);
    let mut lambda_names: HashMap<String, String> = HashMap::new();
    let mut lambda_ids: HashMap<String, i32> = HashMap::new();
    let mut next_lambda_idx = 0i32;
    for node in &lambda_nodes {
        if !lambda_is_hoistable(node, &top_defs) {
            continue;
        }
        let key = node.expr.to_lisp();
        if lambda_names.contains_key(&key) {
            continue;
        }
        let name = format!("__lambda{}", next_lambda_idx);
        next_lambda_idx += 1;
        lambda_names.insert(key.clone(), name.clone());
        if let Some(t) = node.typ.as_ref() {
            let (ps, ret) = function_parts(t);
            fn_sigs.insert(name.clone(), (ps, ret));
        }
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
        if let Some((ps, _ret)) = fn_sigs.get(name) {
            if !ps.is_empty() {
                fn_ids.insert(name.clone(), next_fn_id);
                next_fn_id += 1;
            }
        }
    }
    for (key, name) in &lambda_names {
        if let Some(id) = fn_ids.get(name) {
            lambda_ids.insert(key.clone(), *id);
        }
    }

    let mut wat = String::new();
    wat.push_str("(module\n");
    wat.push_str(&emit_vector_runtime(&fn_ids, &fn_sigs));

    for (name, def) in &top_defs {
        if partial_helpers.iter().any(|h| h.binding_name == *name) {
            continue;
        }
        match &def.expr {
            Expression::Apply(items) if
                matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
            => {
                wat.push_str(
                    &compile_lambda_func(
                        name,
                        &def.expr,
                        &def.node,
                        &fn_sigs,
                        &fn_ids,
                        &lambda_ids,
                        &lambda_bindings
                    )?
                );
            }
            _ => {
                wat.push_str(
                    &compile_value_func(
                        name,
                        &def.node,
                        &fn_sigs,
                        &fn_ids,
                        &lambda_ids,
                        &lambda_bindings
                    )?
                );
            }
        }
    }
    for h in &partial_helpers {
        wat.push_str(
            &compile_partial_helper_func(h, &fn_sigs, &fn_ids, &lambda_ids, &lambda_bindings)?
        );
    }
    for h in &partial_helpers {
        let helper_id = fn_ids
            .get(&h.helper_name)
            .copied()
            .ok_or_else(|| format!("Missing function id for helper '{}'", h.helper_name))?;
        wat.push_str(&compile_value_func_fn_ptr(&h.binding_name, helper_id));
    }
    for node in &lambda_nodes {
        let key = node.expr.to_lisp();
        if let Some(name) = lambda_names.get(&key) {
            wat.push_str(
                &compile_lambda_func(
                    name,
                    &node.expr,
                    node,
                    &fn_sigs,
                    &fn_ids,
                    &lambda_ids,
                    &lambda_bindings
                )?
            );
        }
    }

    let main_ret_ty = main_node.typ
        .as_ref()
        .ok_or_else(|| "Missing main expression type".to_string())?;
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

    let main_ctx = Ctx {
        fn_sigs: &fn_sigs,
        fn_ids: &fn_ids,
        lambda_ids: &lambda_ids,
        lambda_bindings: &scoped_lambda_bindings,
        locals: main_locals,
        tmp_i32: main_local_defs.len(),
    };
    let main_code = compile_expr(&main_node, &main_ctx)?;

    wat.push_str(&format!("  (func (export \"main\") (result {main_wasm_ty})\n"));
    for (_n, t) in &main_local_defs {
        wat.push_str(&format!("    (local {})\n", wasm_val_type(t)?));
    }
    for _ in 0..EXTRA_I32_LOCALS {
        wat.push_str("    (local i32)\n");
    }
    wat.push_str(&format!("    {}\n", main_code.replace('\n', "\n    ")));
    wat.push_str("  )\n");
    wat.push_str(")\n");
    Ok(wat)
}

pub fn compile_program_to_wat(expr: &Expression) -> Result<String, String> {
    let (_typ, typed_ast) = crate::infer::infer_with_builtins_typed(
        expr,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
    )?;
    compile_program_to_wat_typed(&typed_ast)
}
