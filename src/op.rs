use crate::infer::TypedExpression;
use crate::parser::Expression;
use crate::types::Type;
use std::collections::{ HashMap, HashSet };

const MAX_INLINE_BODY_COST: usize = 16;
const MAX_INLINE_FIXPOINT_PASSES: usize = 16;
const MAX_OPT_FIXPOINT_PASSES: usize = 8;

pub fn optimize_typed_ast(node: &TypedExpression) -> TypedExpression {
    let mut cur = optimize_typed_ast_once(node);
    for _ in 0..MAX_OPT_FIXPOINT_PASSES {
        let next = optimize_typed_ast_once(&cur);
        if next.expr.to_lisp() == cur.expr.to_lisp() {
            return next;
        }
        cur = next;
    }
    cur
}

fn optimize_typed_ast_once(node: &TypedExpression) -> TypedExpression {
    let optimized_children = node.children
        .iter()
        .map(optimize_typed_ast_once)
        .collect::<Vec<_>>();
    let rebuilt_expr = rebuild_expr_from_children(&node.expr, &optimized_children);
    let rebuilt_node = TypedExpression {
        expr: rebuilt_expr,
        typ: node.typ.clone(),
        children: optimized_children,
    };
    fold_constants(rebuilt_node)
}

fn rebuild_expr_from_children(expr: &Expression, children: &[TypedExpression]) -> Expression {
    match expr {
        Expression::Apply(items) if items.len() == children.len() =>
            Expression::Apply(children.iter().map(|ch| ch.expr.clone()).collect()),
        _ => expr.clone(),
    }
}

fn fold_constants(node: TypedExpression) -> TypedExpression {
    let items = match &node.expr {
        Expression::Apply(items) => items.clone(),
        _ => return node,
    };
    let Some(Expression::Word(op)) = items.first() else {
        return node;
    };

    match op.as_str() {
        "do" => fold_do(node, &items),
        "if" => fold_if(node, &items),
        "and" => fold_and(node, &items),
        "or" => fold_or(node, &items),
        "not" => fold_not(node, &items),

        "+" | "+#" => fold_int_add(node, &items),
        "-" | "-#" => fold_int_bin(node, &items, i32::wrapping_sub),
        "*" | "*#" => fold_int_mul(node, &items),
        "/" | "/#" => fold_int_checked_bin(node, &items, i32::checked_div),
        "mod" => fold_int_checked_bin(node, &items, i32::checked_rem),

        "=" | "=?" | "=#" => fold_int_cmp(node, &items, |a, b| a == b),
        "<" | "<#" => fold_int_cmp(node, &items, |a, b| a < b),
        ">" | ">#" => fold_int_cmp(node, &items, |a, b| a > b),
        "<=" | "<=#" => fold_int_cmp(node, &items, |a, b| a <= b),
        ">=" | ">=#" => fold_int_cmp(node, &items, |a, b| a >= b),

        "+." => fold_float_bin(node, &items, |a, b| a + b),
        "-." => fold_float_bin(node, &items, |a, b| a - b),
        "*." => fold_float_bin(node, &items, |a, b| a * b),
        "/." => fold_float_bin(node, &items, |a, b| a / b),
        "mod." => fold_float_bin(node, &items, |a, b| a - (a / b).trunc() * b),

        "=." => fold_float_cmp(node, &items, |a, b| a == b),
        "<." => fold_float_cmp(node, &items, |a, b| a < b),
        ">." => fold_float_cmp(node, &items, |a, b| a > b),
        "<=." => fold_float_cmp(node, &items, |a, b| a <= b),
        ">=." => fold_float_cmp(node, &items, |a, b| a >= b),

        "Int->Float" => fold_int_to_float(node, &items),
        "Float->Int" => fold_float_to_int(node, &items),

        _ => node,
    }
}

fn fold_do(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() <= 1 {
        return node;
    }
    let Some(normalized_do) = normalize_do_node(&node, items) else {
        return node;
    };

    // First, inline simple direct lambda calls at do-scope with hygienic temp args.
    let mut inline_state = InlineState::new(&normalized_do.expr);
    let (inlined_items, inlined_children) = inline_do_simple_calls(&normalized_do, &mut inline_state);
    let (inlined_items, inlined_children) = eliminate_single_use_let_bindings(
        inlined_items,
        inlined_children
    );

    // Always collapse single-item do, even if no cleanup happened.
    if inlined_items.len() == 2 {
        return inlined_children
            .get(1)
            .cloned()
            .or_else(|| inlined_children.last().cloned())
            .unwrap_or(normalized_do);
    }

    let last_idx = inlined_items.len() - 1;
    let mut kept_indices: Vec<usize> = Vec::new();
    kept_indices.push(0); // keep "do"
    for i in 1..last_idx {
        if !is_pure_literal_expr(&inlined_items[i]) {
            kept_indices.push(i);
        }
    }
    kept_indices.push(last_idx);

    if kept_indices.len() == inlined_items.len() {
        return TypedExpression {
            expr: Expression::Apply(inlined_items),
            typ: normalized_do.typ,
            children: inlined_children,
        };
    }

    // (do x) => x
    if kept_indices.len() == 2 {
        let only_expr_idx = kept_indices[1];
        return inlined_children.get(only_expr_idx).cloned().unwrap_or(normalized_do);
    }

    let new_expr_items = kept_indices
        .iter()
        .filter_map(|idx| inlined_items.get(*idx).cloned())
        .collect::<Vec<_>>();
    let new_children = kept_indices
        .iter()
        .filter_map(|idx| inlined_children.get(*idx).cloned())
        .collect::<Vec<_>>();

    TypedExpression {
        expr: Expression::Apply(new_expr_items),
        typ: normalized_do.typ,
        children: new_children,
    }
}

fn normalize_do_node(node: &TypedExpression, items: &[Expression]) -> Option<TypedExpression> {
    if node.children.len() == items.len() {
        return Some(node.clone());
    }
    if
        node.children.len() + 1 == items.len() &&
        matches!(items.first(), Some(Expression::Word(w)) if w == "do")
    {
        let mut children = Vec::with_capacity(items.len());
        children.push(TypedExpression {
            expr: Expression::Word("do".to_string()),
            typ: None,
            children: Vec::new(),
        });
        children.extend(node.children.clone());
        return Some(TypedExpression {
            expr: node.expr.clone(),
            typ: node.typ.clone(),
            children,
        });
    }
    None
}

fn fold_int_add(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 3 {
        return node;
    }
    if let Some(0) = items.get(1).and_then(int_literal) {
        return node.children.get(2).cloned().unwrap_or(node);
    }
    if let Some(0) = items.get(2).and_then(int_literal) {
        return node.children.get(1).cloned().unwrap_or(node);
    }
    fold_int_bin(node, items, i32::wrapping_add)
}

fn fold_int_mul(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 3 {
        return node;
    }
    if let Some(1) = items.get(1).and_then(int_literal) {
        return node.children.get(2).cloned().unwrap_or(node);
    }
    if let Some(1) = items.get(2).and_then(int_literal) {
        return node.children.get(1).cloned().unwrap_or(node);
    }
    fold_int_bin(node, items, i32::wrapping_mul)
}

fn fold_if(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 4 {
        return node;
    }
    let Some(cond) = items.get(1).and_then(bool_literal) else {
        return node;
    };
    if cond {
        node.children.get(2).cloned().unwrap_or(node)
    } else {
        node.children.get(3).cloned().unwrap_or(node)
    }
}

fn fold_and(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 3 {
        return node;
    }
    if let Some(lhs) = items.get(1).and_then(bool_literal) {
        if lhs {
            return node.children.get(2).cloned().unwrap_or(node);
        }
        return make_folded_literal(&node, Expression::Word("false".to_string()), Type::Bool);
    }
    node
}

fn fold_or(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 3 {
        return node;
    }
    if let Some(lhs) = items.get(1).and_then(bool_literal) {
        if lhs {
            return make_folded_literal(&node, Expression::Word("true".to_string()), Type::Bool);
        }
        return node.children.get(2).cloned().unwrap_or(node);
    }
    node
}

fn fold_not(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 2 {
        return node;
    }
    let Some(v) = items.get(1).and_then(bool_literal) else {
        return node;
    };
    make_folded_literal(
        &node,
        Expression::Word(if !v { "true" } else { "false" }.to_string()),
        Type::Bool
    )
}

fn fold_int_bin(node: TypedExpression, items: &[Expression], f: fn(i32, i32) -> i32) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(int_literal), items.get(2).and_then(int_literal)) else {
        return node;
    };
    make_folded_literal(&node, Expression::Int(f(a, b)), Type::Int)
}

fn fold_int_checked_bin(
    node: TypedExpression,
    items: &[Expression],
    f: fn(i32, i32) -> Option<i32>
) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(int_literal), items.get(2).and_then(int_literal)) else {
        return node;
    };
    let Some(v) = f(a, b) else {
        // Preserve runtime semantics for trap cases (divide/rem by zero, overflow).
        return node;
    };
    make_folded_literal(&node, Expression::Int(v), Type::Int)
}

fn fold_int_cmp(
    node: TypedExpression,
    items: &[Expression],
    f: fn(i32, i32) -> bool
) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(int_literal), items.get(2).and_then(int_literal)) else {
        return node;
    };
    make_folded_literal(
        &node,
        Expression::Word(if f(a, b) { "true" } else { "false" }.to_string()),
        Type::Bool
    )
}

fn fold_float_bin(
    node: TypedExpression,
    items: &[Expression],
    f: fn(f32, f32) -> f32
) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(float_literal), items.get(2).and_then(float_literal)) else {
        return node;
    };
    make_folded_literal(&node, Expression::Float(f(a, b)), Type::Float)
}

fn fold_float_cmp(
    node: TypedExpression,
    items: &[Expression],
    f: fn(f32, f32) -> bool
) -> TypedExpression {
    let (Some(a), Some(b)) = (items.get(1).and_then(float_literal), items.get(2).and_then(float_literal)) else {
        return node;
    };
    make_folded_literal(
        &node,
        Expression::Word(if f(a, b) { "true" } else { "false" }.to_string()),
        Type::Bool
    )
}

fn fold_int_to_float(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    let Some(a) = items.get(1).and_then(int_literal) else {
        return node;
    };
    make_folded_literal(&node, Expression::Float(a as f32), Type::Float)
}

fn fold_float_to_int(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    let Some(a) = items.get(1).and_then(float_literal) else {
        return node;
    };
    make_folded_literal(&node, Expression::Int(a as i32), Type::Int)
}

fn make_folded_literal(node: &TypedExpression, expr: Expression, typ: Type) -> TypedExpression {
    TypedExpression {
        expr,
        typ: node.typ.clone().or(Some(typ)),
        children: Vec::new(),
    }
}

fn int_literal(expr: &Expression) -> Option<i32> {
    match expr {
        Expression::Int(v) => Some(*v),
        _ => None,
    }
}

fn float_literal(expr: &Expression) -> Option<f32> {
    match expr {
        Expression::Float(v) => Some(*v),
        _ => None,
    }
}

fn bool_literal(expr: &Expression) -> Option<bool> {
    match expr {
        Expression::Word(w) if w == "true" => Some(true),
        Expression::Word(w) if w == "false" => Some(false),
        _ => None,
    }
}

fn is_pure_literal_expr(expr: &Expression) -> bool {
    match expr {
        Expression::Int(_) | Expression::Float(_) => true,
        Expression::Word(w) if w == "true" || w == "false" => true,
        _ => false,
    }
}

#[derive(Clone)]
struct InlineLambdaDef {
    params: Vec<String>,
    body_expr: Expression,
    body_typed: TypedExpression,
}

struct InlineState {
    used_names: HashSet<String>,
    next_id: usize,
}

impl InlineState {
    fn new(root: &Expression) -> Self {
        let mut used_names = HashSet::new();
        collect_word_names(root, &mut used_names);
        Self {
            used_names,
            next_id: 0,
        }
    }

    fn fresh_tmp(&mut self) -> String {
        loop {
            let name = format!("__inline_arg_{}", self.next_id);
            self.next_id += 1;
            if self.used_names.insert(name.clone()) {
                return name;
            }
        }
    }
}

fn collect_word_names(expr: &Expression, out: &mut HashSet<String>) {
    match expr {
        Expression::Word(w) => {
            out.insert(w.clone());
        }
        Expression::Apply(items) => {
            for it in items {
                collect_word_names(it, out);
            }
        }
        _ => {}
    }
}

fn inline_do_simple_calls(
    node: &TypedExpression,
    state: &mut InlineState
) -> (Vec<Expression>, Vec<TypedExpression>) {
    let Expression::Apply(items) = &node.expr else {
        return (vec![node.expr.clone()], vec![node.clone()]);
    };
    if items.is_empty() || !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        return (items.clone(), node.children.clone());
    }

    let mut cur_items = items.clone();
    let mut cur_children = node.children.clone();

    for _ in 0..MAX_INLINE_FIXPOINT_PASSES {
        let cur_node = TypedExpression {
            expr: Expression::Apply(cur_items.clone()),
            typ: node.typ.clone(),
            children: cur_children.clone(),
        };
        let (next_items, next_children, changed) = inline_do_simple_calls_once(&cur_node, state);
        cur_items = next_items;
        cur_children = next_children;
        if !changed {
            break;
        }
    }

    (cur_items, cur_children)
}

fn eliminate_single_use_let_bindings(
    mut items: Vec<Expression>,
    mut children: Vec<TypedExpression>
) -> (Vec<Expression>, Vec<TypedExpression>) {
    if items.len() != children.len() || items.len() <= 2 {
        return (items, children);
    }

    for _ in 0..MAX_INLINE_FIXPOINT_PASSES {
        let mut changed = false;
        let mut i = 1usize;
        while i + 1 < items.len() {
            let Some((name, can_duplicate_rhs)) = eliminable_let_name(&items[i]) else {
                i += 1;
                continue;
            };
            let uses = count_word_uses_in_slice(&items[i + 1..], &name);
            if uses == 0 || (!can_duplicate_rhs && uses != 1) {
                i += 1;
                continue;
            }
            let Some(rhs_typed) = children.get(i).and_then(|n| n.children.get(2)).cloned() else {
                i += 1;
                continue;
            };
            let rhs_expr = rhs_typed.expr.clone();

            for j in i + 1..items.len() {
                items[j] = substitute_word_with_expr(&items[j], &name, &rhs_expr);
                children[j] = substitute_word_with_typed(&children[j], &name, &rhs_typed);
            }
            items.remove(i);
            children.remove(i);
            changed = true;
            break;
        }
        if !changed {
            break;
        }
    }
    (items, children)
}

fn eliminable_let_name(expr: &Expression) -> Option<(String, bool)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), rhs] = &items[..] else {
        return None;
    };
    if kw != "let" && kw != "let*" {
        None
    } else if name.starts_with("__inline_arg_") {
        // Inline temps are compiler-generated and safe to substitute in-place.
        Some((name.clone(), false))
    } else if is_pure_literal_expr(rhs) {
        // Literals are side-effect free and cheap; allow substitution for all uses.
        Some((name.clone(), true))
    } else {
        None
    }
}

fn count_word_uses_in_slice(items: &[Expression], name: &str) -> usize {
    items.iter().map(|e| count_word_uses_expr(e, name)).sum()
}

fn count_word_uses_expr(expr: &Expression, name: &str) -> usize {
    match expr {
        Expression::Word(w) => {
            if w == name { 1 } else { 0 }
        }
        Expression::Apply(items) => items.iter().map(|it| count_word_uses_expr(it, name)).sum(),
        _ => 0,
    }
}

fn substitute_word_with_expr(expr: &Expression, name: &str, replacement: &Expression) -> Expression {
    match expr {
        Expression::Word(w) => {
            if w == name {
                replacement.clone()
            } else {
                Expression::Word(w.clone())
            }
        }
        Expression::Apply(items) => Expression::Apply(
            items.iter().map(|it| substitute_word_with_expr(it, name, replacement)).collect()
        ),
        Expression::Int(n) => Expression::Int(*n),
        Expression::Float(n) => Expression::Float(*n),
    }
}

fn substitute_word_with_typed(
    node: &TypedExpression,
    name: &str,
    replacement: &TypedExpression
) -> TypedExpression {
    if matches!(&node.expr, Expression::Word(w) if w == name) {
        return replacement.clone();
    }

    let new_children = node.children
        .iter()
        .map(|ch| substitute_word_with_typed(ch, name, replacement))
        .collect::<Vec<_>>();

    let new_expr = match &node.expr {
        Expression::Apply(items) if items.len() == new_children.len() =>
            Expression::Apply(new_children.iter().map(|ch| ch.expr.clone()).collect()),
        _ => substitute_word_with_expr(&node.expr, name, &replacement.expr),
    };

    TypedExpression {
        expr: new_expr,
        typ: node.typ.clone(),
        children: new_children,
    }
}

fn inline_do_simple_calls_once(
    node: &TypedExpression,
    state: &mut InlineState
) -> (Vec<Expression>, Vec<TypedExpression>, bool) {
    let Expression::Apply(items) = &node.expr else {
        return (vec![node.expr.clone()], vec![node.clone()], false);
    };
    if items.is_empty() || !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        return (items.clone(), node.children.clone(), false);
    }

    let mut defs: HashMap<String, InlineLambdaDef> = HashMap::new();
    let mut out_items = vec![items[0].clone()];
    let mut out_children = vec![node.children[0].clone()];
    let mut changed = false;

    for i in 1..items.len() {
        let expr_i = &items[i];
        let child_i = &node.children[i];

        if let Some((let_expr, let_child, def)) = extract_inline_lambda_def(expr_i, child_i) {
            defs.insert(def.0.clone(), def.1);
            out_items.push(let_expr);
            out_children.push(let_child);
            continue;
        }

        if let Some((prep, rewritten_expr, rewritten_child)) = try_inline_let_rhs(expr_i, child_i, &defs, state) {
            changed = true;
            for (e, c) in prep {
                out_items.push(e);
                out_children.push(c);
            }
            out_items.push(rewritten_expr);
            out_children.push(rewritten_child);
            continue;
        }

        if let Some((prep, inlined_expr, inlined_child)) = try_inline_call(expr_i, child_i, &defs, state) {
            changed = true;
            for (e, c) in prep {
                out_items.push(e);
                out_children.push(c);
            }
            out_items.push(inlined_expr);
            out_children.push(inlined_child);
            continue;
        }

        let (nested_expr, nested_child, nested_changed) = inline_nested_calls(child_i, &defs);
        if nested_changed {
            changed = true;
            out_items.push(nested_expr);
            out_children.push(nested_child);
            continue;
        }

        out_items.push(expr_i.clone());
        out_children.push(child_i.clone());
    }

    (out_items, out_children, changed)
}

fn extract_inline_lambda_def(
    expr: &Expression,
    node: &TypedExpression
) -> Option<(Expression, TypedExpression, (String, InlineLambdaDef))> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if items.len() != 3 {
        return None;
    }
    let (kw, name, rhs) = (items.first()?, items.get(1)?, items.get(2)?);
    let (Expression::Word(kw), Expression::Word(name)) = (kw, name) else {
        return None;
    };
    if kw != "let" && kw != "let*" {
        return None;
    }
    let Expression::Apply(lambda_items) = rhs else {
        return None;
    };
    if !matches!(lambda_items.first(), Some(Expression::Word(w)) if w == "lambda") {
        return None;
    }
    if lambda_items.len() < 2 {
        return None;
    }
    let body_expr = lambda_items.last()?.clone();
    if !is_inline_safe_body(&body_expr) || contains_word(&body_expr, name) {
        return None;
    }
    if inline_body_cost(&body_expr) > MAX_INLINE_BODY_COST {
        return None;
    }
    let params = lambda_items[1..lambda_items.len() - 1]
        .iter()
        .map(|p| match p {
            Expression::Word(w) => Some(w.clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let lambda_typed = node.children.get(2)?;
    if typed_contains_type_var(lambda_typed) {
        return None;
    }
    let body_typed = lambda_typed.children.last()?.clone();

    let def = InlineLambdaDef {
        params,
        body_expr,
        body_typed,
    };
    Some((expr.clone(), node.clone(), (name.clone(), def)))
}

fn is_inline_safe_body(expr: &Expression) -> bool {
    match expr {
        Expression::Int(_) | Expression::Float(_) | Expression::Word(_) => true,
        Expression::Apply(items) => {
            if let Some(Expression::Word(head)) = items.first() {
                if head == "let" || head == "let*" || head == "lambda" {
                    return false;
                }
            }
            items.iter().all(is_inline_safe_body)
        }
    }
}

fn inline_body_cost(expr: &Expression) -> usize {
    match expr {
        Expression::Int(_) | Expression::Float(_) | Expression::Word(_) => 1,
        Expression::Apply(items) => 1 + items.iter().map(inline_body_cost).sum::<usize>(),
    }
}

fn typed_contains_type_var(node: &TypedExpression) -> bool {
    if let Some(t) = &node.typ {
        if type_contains_var(t) {
            return true;
        }
    }
    node.children.iter().any(typed_contains_type_var)
}

fn type_contains_var(typ: &Type) -> bool {
    match typ {
        Type::Var(_) => true,
        Type::List(inner) => type_contains_var(inner),
        Type::Function(a, b) => type_contains_var(a) || type_contains_var(b),
        Type::Tuple(items) => items.iter().any(type_contains_var),
        _ => false,
    }
}

fn contains_word(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::Word(w) => w == name,
        Expression::Apply(items) => items.iter().any(|it| contains_word(it, name)),
        _ => false,
    }
}

fn try_inline_call(
    expr: &Expression,
    node: &TypedExpression,
    defs: &HashMap<String, InlineLambdaDef>,
    state: &mut InlineState
) -> Option<(Vec<(Expression, TypedExpression)>, Expression, TypedExpression)> {
    let Expression::Apply(call_items) = expr else {
        return None;
    };
    if call_items.is_empty() {
        return None;
    }
    let callee = match call_items.first() {
        Some(Expression::Word(w)) => w,
        _ => return None,
    };
    let def = defs.get(callee)?;
    let arg_exprs = &call_items[1..];
    if arg_exprs.len() != def.params.len() || node.children.len() != call_items.len() {
        return None;
    }

    let mut subst = HashMap::new();
    let mut prep = Vec::new();
    for (idx, param) in def.params.iter().enumerate() {
        let arg_expr = arg_exprs[idx].clone();
        let arg_node = node.children.get(idx + 1)?.clone();
        let tmp = state.fresh_tmp();
        subst.insert(param.clone(), tmp.clone());

        let let_expr = Expression::Apply(vec![
            Expression::Word("let".to_string()),
            Expression::Word(tmp.clone()),
            arg_expr.clone(),
        ]);
        let let_typed = TypedExpression {
            expr: let_expr.clone(),
            typ: arg_node.typ.clone(),
            children: vec![
                TypedExpression {
                    expr: Expression::Word("let".to_string()),
                    typ: None,
                    children: Vec::new(),
                },
                TypedExpression {
                    expr: Expression::Word(tmp),
                    typ: arg_node.typ.clone(),
                    children: Vec::new(),
                },
                arg_node,
            ],
        };
        prep.push((let_expr, let_typed));
    }

    let inlined_expr = substitute_words_expr(&def.body_expr, &subst);
    let inlined_typed = substitute_words_typed(&def.body_typed, &subst);
    Some((prep, inlined_expr, inlined_typed))
}

fn inline_nested_calls(
    node: &TypedExpression,
    defs: &HashMap<String, InlineLambdaDef>
) -> (Expression, TypedExpression, bool) {
    let mut changed = false;
    let mut rewritten_children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        let (_expr, rewritten_child, child_changed) = inline_nested_calls(child, defs);
        if child_changed {
            changed = true;
        }
        rewritten_children.push(rewritten_child);
    }

    let rewritten_expr = rebuild_expr_from_children(&node.expr, &rewritten_children);
    let rewritten_node = TypedExpression {
        expr: rewritten_expr,
        typ: node.typ.clone(),
        children: rewritten_children,
    };

    if let Some(inlined) = try_inline_call_no_temps(&rewritten_node, defs) {
        return (inlined.expr.clone(), inlined, true);
    }

    (rewritten_node.expr.clone(), rewritten_node, changed)
}

fn try_inline_call_no_temps(
    node: &TypedExpression,
    defs: &HashMap<String, InlineLambdaDef>
) -> Option<TypedExpression> {
    let Expression::Apply(call_items) = &node.expr else {
        return None;
    };
    if call_items.is_empty() {
        return None;
    }
    let callee = match call_items.first() {
        Some(Expression::Word(w)) => w,
        _ => return None,
    };
    let def = defs.get(callee)?;
    let arg_exprs = &call_items[1..];
    if arg_exprs.len() != def.params.len() || node.children.len() != call_items.len() {
        return None;
    }

    let mut expr_subst: HashMap<String, Expression> = HashMap::new();
    let mut typed_subst: HashMap<String, TypedExpression> = HashMap::new();
    for (idx, param) in def.params.iter().enumerate() {
        let arg_expr = arg_exprs[idx].clone();
        let arg_node = node.children.get(idx + 1)?.clone();
        let uses = count_word_uses_expr(&def.body_expr, param);
        if uses > 1 && !is_atomic_inline_arg_expr(&arg_expr) {
            return None;
        }
        expr_subst.insert(param.clone(), arg_expr);
        typed_subst.insert(param.clone(), arg_node);
    }

    Some(substitute_params_typed(&def.body_typed, &expr_subst, &typed_subst))
}

fn is_atomic_inline_arg_expr(expr: &Expression) -> bool {
    matches!(expr, Expression::Word(_) | Expression::Int(_) | Expression::Float(_))
}

fn substitute_params_expr(expr: &Expression, subst: &HashMap<String, Expression>) -> Expression {
    match expr {
        Expression::Word(w) => subst.get(w).cloned().unwrap_or_else(|| Expression::Word(w.clone())),
        Expression::Apply(items) =>
            Expression::Apply(items.iter().map(|it| substitute_params_expr(it, subst)).collect()),
        Expression::Int(n) => Expression::Int(*n),
        Expression::Float(n) => Expression::Float(*n),
    }
}

fn substitute_params_typed(
    node: &TypedExpression,
    expr_subst: &HashMap<String, Expression>,
    typed_subst: &HashMap<String, TypedExpression>
) -> TypedExpression {
    if let Expression::Word(w) = &node.expr {
        if let Some(repl) = typed_subst.get(w) {
            return repl.clone();
        }
    }

    let new_children = node.children
        .iter()
        .map(|ch| substitute_params_typed(ch, expr_subst, typed_subst))
        .collect::<Vec<_>>();
    let new_expr = match &node.expr {
        Expression::Apply(items) if items.len() == new_children.len() =>
            Expression::Apply(new_children.iter().map(|ch| ch.expr.clone()).collect()),
        _ => substitute_params_expr(&node.expr, expr_subst),
    };
    TypedExpression {
        expr: new_expr,
        typ: node.typ.clone(),
        children: new_children,
    }
}

fn try_inline_let_rhs(
    expr: &Expression,
    node: &TypedExpression,
    defs: &HashMap<String, InlineLambdaDef>,
    state: &mut InlineState
) -> Option<(Vec<(Expression, TypedExpression)>, Expression, TypedExpression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if items.len() != 3 || node.children.len() != 3 {
        return None;
    }
    let kw = match items.first() {
        Some(Expression::Word(w)) if w == "let" || w == "let*" => w.clone(),
        _ => return None,
    };
    let rhs_expr = items.get(2)?;
    let rhs_typed = node.children.get(2)?;
    let (prep, inlined_rhs_expr, inlined_rhs_typed) = try_inline_call(rhs_expr, rhs_typed, defs, state)?;

    let rewritten_expr = Expression::Apply(vec![
        Expression::Word(kw),
        items.get(1)?.clone(),
        inlined_rhs_expr,
    ]);
    let mut rewritten_typed = node.clone();
    rewritten_typed.expr = rewritten_expr.clone();
    rewritten_typed.children[2] = inlined_rhs_typed;
    Some((prep, rewritten_expr, rewritten_typed))
}

fn substitute_words_expr(expr: &Expression, subst: &HashMap<String, String>) -> Expression {
    match expr {
        Expression::Word(w) => {
            if let Some(repl) = subst.get(w) {
                Expression::Word(repl.clone())
            } else {
                Expression::Word(w.clone())
            }
        }
        Expression::Apply(items) =>
            Expression::Apply(items.iter().map(|it| substitute_words_expr(it, subst)).collect()),
        Expression::Int(n) => Expression::Int(*n),
        Expression::Float(n) => Expression::Float(*n),
    }
}

fn substitute_words_typed(node: &TypedExpression, subst: &HashMap<String, String>) -> TypedExpression {
    let new_children = node.children
        .iter()
        .map(|ch| substitute_words_typed(ch, subst))
        .collect::<Vec<_>>();
    TypedExpression {
        expr: substitute_words_expr(&node.expr, subst),
        typ: node.typ.clone(),
        children: new_children,
    }
}
