use crate::infer::TypedExpression;
use crate::parser::Expression;
use crate::types::Type;
use std::collections::{ HashMap, HashSet };

const MAX_INLINE_BODY_COST: usize = 16;
const MAX_INLINE_FIXPOINT_PASSES: usize = 16;
const MAX_OPT_FIXPOINT_PASSES: usize = 8;

#[derive(Clone)]
enum MapFilterOp {
    Map(Expression),
    Filter(Expression),
}

pub fn optimize_typed_ast(node: &TypedExpression) -> TypedExpression {
    let mut seed = node.clone();
    let fused_expr = fuse_entry_expression_for_program(&node.expr);
    if fused_expr.to_lisp() != node.expr.to_lisp() {
        if
            let Ok((_typ, fused_typed)) = crate::infer
                ::infer_with_builtins_typed(
                    &fused_expr,
                    crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                )
        {
            seed = fused_typed;
        }
    }

    let mut cur = optimize_typed_ast_once(&seed);
    for _ in 0..MAX_OPT_FIXPOINT_PASSES {
        let next = optimize_typed_ast_once(&cur);
        if next.expr.to_lisp() == cur.expr.to_lisp() {
            return next;
        }
        cur = next;
    }
    cur
}

fn fuse_entry_expression_for_program(expr: &Expression) -> Expression {
    match expr {
        Expression::Apply(items) if
            matches!(items.first(), Some(Expression::Word(w)) if w == "do") && items.len() > 1
        => {
            let mut out = items.clone();
            let last = out.len() - 1;
            out[last] = fuse_map_filter_reduce_chains_expr(&out[last]);
            Expression::Apply(out)
        }
        _ => fuse_map_filter_reduce_chains_expr(expr),
    }
}

fn fuse_map_filter_reduce_chains_expr(expr: &Expression) -> Expression {
    match expr {
        Expression::Apply(items) => {
            // Prioritize whole-chain fusions before rewriting children, so map/filter
            // combos become a single loop rather than nested loop wrappers.
            if let Some(fused) = fuse_reduce_over_map_filter_chain(expr) {
                return fuse_map_filter_reduce_chains_expr(&fused);
            }
            if let Some(fused) = fuse_map_filter_chain_to_reduce(expr) {
                return fused;
            }

            let rewritten_items = items
                .iter()
                .map(fuse_map_filter_reduce_chains_expr)
                .collect::<Vec<_>>();
            let rewritten = Expression::Apply(rewritten_items);
            rewritten
        }
        _ => expr.clone(),
    }
}

fn fuse_reduce_over_map_filter_chain(expr: &Expression) -> Option<Expression> {
    let (reduce_fn, init, reduce_input) = parse_reduce_call(expr)?;
    let (base, ops) = collect_map_filter_chain(reduce_input)?;
    if ops.is_empty() {
        return None;
    }
    if !is_fusion_safe_callable(&reduce_fn) || !ops.iter().all(map_filter_op_is_fusion_safe) {
        return None;
    }
    build_direct_reduce_loop(base, &ops, reduce_fn, init)
}

fn fuse_map_filter_chain_to_reduce(expr: &Expression) -> Option<Expression> {
    let (_, _) = parse_map_or_filter_call(expr)?;
    let (base, ops) = collect_map_filter_chain(expr.clone())?;
    if ops.is_empty() {
        return None;
    }
    if !ops.iter().all(map_filter_op_is_fusion_safe) {
        return None;
    }
    build_direct_map_filter_loop(base, &ops)
}

fn parse_reduce_call(expr: &Expression) -> Option<(Expression, Expression, Expression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if items.len() != 4 {
        return None;
    }
    let Expression::Word(name) = items.first()? else {
        return None;
    };
    match name.as_str() {
        // reduce fn init xs
        "reduce" => Some((items.get(1)?.clone(), items.get(2)?.clone(), items.get(3)?.clone())),
        _ => None,
    }
}

fn parse_map_or_filter_call(expr: &Expression) -> Option<(MapFilterOp, Expression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if items.len() != 3 {
        return None;
    }
    let Expression::Word(name) = items.first()? else {
        return None;
    };
    match name.as_str() {
        // map fn xs
        "map" => Some((MapFilterOp::Map(items.get(1)?.clone()), items.get(2)?.clone())),
        // filter fn xs
        "filter" => Some((MapFilterOp::Filter(items.get(1)?.clone()), items.get(2)?.clone())),
        _ => None,
    }
}

fn collect_map_filter_chain(root: Expression) -> Option<(Expression, Vec<MapFilterOp>)> {
    let mut ops: Vec<MapFilterOp> = Vec::new();
    let mut current = root;
    while let Some((op, next)) = parse_map_or_filter_call(&current) {
        ops.push(op);
        current = next;
    }
    if ops.is_empty() {
        None
    } else {
        Some((current, ops))
    }
}

fn build_direct_reduce_loop(
    xs_expr: Expression,
    ops_outer_to_inner: &[MapFilterOp],
    reduce_fn: Expression,
    init_expr: Expression
) -> Option<Expression> {
    let xs_name = "__fuse_xs".to_string();
    let out_name = "__fuse_out".to_string();
    let process_name = "__fuse_process".to_string();
    let i_name = "__fuse_i".to_string();

    let xs_word = Expression::Word(xs_name.clone());
    let i_word = Expression::Word(i_name.clone());
    let x_expr = Expression::Apply(vec![Expression::Word("get".to_string()), xs_word.clone(), i_word.clone()]);
    let (mapped, guard) = compose_map_filter_value_and_guard(ops_outer_to_inner, x_expr)?;

    let acc_get = Expression::Apply(vec![
        Expression::Word("get".to_string()),
        Expression::Word(out_name.clone()),
        Expression::Int(0),
    ]);
    let reduced = call_callable_expr(&reduce_fn, vec![acc_get.clone(), mapped])?;
    let next_acc = match guard {
        Some(cond) =>
            Expression::Apply(vec![
                Expression::Word("if".to_string()),
                cond,
                reduced,
                acc_get,
            ]),
        None => reduced,
    };

    let process_body = Expression::Apply(vec![
        Expression::Word("set!".to_string()),
        Expression::Word(out_name.clone()),
        Expression::Int(0),
        next_acc,
    ]);
    let process_lambda = Expression::Apply(vec![
        Expression::Word("lambda".to_string()),
        Expression::Word(i_name),
        process_body,
    ]);

    Some(
        Expression::Apply(vec![
            Expression::Word("do".to_string()),
            Expression::Apply(vec![
                Expression::Word("let".to_string()),
                Expression::Word(xs_name),
                xs_expr,
            ]),
            Expression::Apply(vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string()), init_expr]),
            ]),
            Expression::Apply(vec![
                Expression::Word("let".to_string()),
                Expression::Word(process_name.clone()),
                process_lambda,
            ]),
            Expression::Apply(vec![
                Expression::Word("loop".to_string()),
                Expression::Int(0),
                Expression::Apply(vec![Expression::Word("length".to_string()), xs_word]),
                Expression::Word(process_name),
            ]),
            Expression::Apply(vec![
                Expression::Word("get".to_string()),
                Expression::Word(out_name),
                Expression::Int(0),
            ]),
        ])
    )
}

fn build_direct_map_filter_loop(xs_expr: Expression, ops_outer_to_inner: &[MapFilterOp]) -> Option<Expression> {
    let xs_name = "__fuse_xs".to_string();
    let out_name = "__fuse_out".to_string();
    let process_name = "__fuse_process".to_string();
    let i_name = "__fuse_i".to_string();

    let xs_word = Expression::Word(xs_name.clone());
    let i_word = Expression::Word(i_name.clone());
    let x_expr = Expression::Apply(vec![Expression::Word("get".to_string()), xs_word.clone(), i_word.clone()]);
    let (mapped, guard) = compose_map_filter_value_and_guard(ops_outer_to_inner, x_expr)?;

    let push_expr = Expression::Apply(vec![
        Expression::Word("set!".to_string()),
        Expression::Word(out_name.clone()),
        Expression::Apply(vec![
            Expression::Word("length".to_string()),
            Expression::Word(out_name.clone()),
        ]),
        mapped,
    ]);
    let process_body = match guard {
        Some(cond) =>
            Expression::Apply(vec![
                Expression::Word("if".to_string()),
                cond,
                push_expr,
                no_op_unit_expr(),
            ]),
        None => push_expr,
    };
    let process_lambda = Expression::Apply(vec![
        Expression::Word("lambda".to_string()),
        Expression::Word(i_name),
        process_body,
    ]);

    Some(
        Expression::Apply(vec![
            Expression::Word("do".to_string()),
            Expression::Apply(vec![
                Expression::Word("let".to_string()),
                Expression::Word(xs_name),
                xs_expr,
            ]),
            Expression::Apply(vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string())]),
            ]),
            Expression::Apply(vec![
                Expression::Word("let".to_string()),
                Expression::Word(process_name.clone()),
                process_lambda,
            ]),
            Expression::Apply(vec![
                Expression::Word("loop".to_string()),
                Expression::Int(0),
                Expression::Apply(vec![Expression::Word("length".to_string()), xs_word]),
                Expression::Word(process_name),
            ]),
            Expression::Word(out_name),
        ])
    )
}

fn compose_map_filter_value_and_guard(
    ops_outer_to_inner: &[MapFilterOp],
    input_expr: Expression
) -> Option<(Expression, Option<Expression>)> {
    let mut cur = input_expr;
    let mut guards: Vec<Expression> = Vec::new();
    for op in ops_outer_to_inner.iter().rev() {
        match op {
            MapFilterOp::Map(map_fn) => {
                cur = call_callable_expr(map_fn, vec![cur])?;
            }
            MapFilterOp::Filter(pred_fn) => {
                guards.push(call_callable_expr(pred_fn, vec![cur.clone()])?);
            }
        }
    }
    let guard = if guards.is_empty() {
        None
    } else {
        let mut cond = guards[0].clone();
        for g in guards.iter().skip(1) {
            cond = Expression::Apply(vec![Expression::Word("and".to_string()), cond, g.clone()]);
        }
        Some(cond)
    };
    Some((cur, guard))
}

fn call_callable_expr(callable: &Expression, args: Vec<Expression>) -> Option<Expression> {
    match callable {
        Expression::Word(w) => {
            let mut items = Vec::with_capacity(1 + args.len());
            items.push(Expression::Word(w.clone()));
            items.extend(args);
            Some(Expression::Apply(items))
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") => {
            if items.len() < 2 {
                return None;
            }
            let params = &items[1..items.len() - 1];
            if params.len() != args.len() {
                return None;
            }
            let mut out = items.last()?.clone();
            for (p, arg) in params.iter().zip(args.iter()) {
                let Expression::Word(name) = p else {
                    return None;
                };
                out = substitute_word_with_expr(&out, name, arg);
            }
            Some(out)
        }
        _ => None,
    }
}

fn no_op_unit_expr() -> Expression {
    // A std-independent no-op expression with Unit type.
    Expression::Apply(vec![
        Expression::Word("loop-finish".to_string()),
        Expression::Word("false".to_string()),
        Expression::Apply(vec![
            Expression::Word("lambda".to_string()),
            Expression::Int(0),
        ]),
    ])
}

fn map_filter_op_is_fusion_safe(op: &MapFilterOp) -> bool {
    match op {
        MapFilterOp::Map(f) | MapFilterOp::Filter(f) => is_fusion_safe_callable(f),
    }
}

fn is_fusion_safe_callable(expr: &Expression) -> bool {
    match expr {
        Expression::Word(_) => true,
        Expression::Apply(items) =>
            matches!(items.first(), Some(Expression::Word(w)) if w == "lambda"),
        _ => false,
    }
}

#[cfg(test)]
pub(crate) fn fuse_map_filter_reduce_for_test(expr: &Expression) -> Expression {
    fuse_map_filter_reduce_chains_expr(expr)
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
    if let Some(apply_fused) = fuse_apply_wrapper_call(&node, &items) {
        return apply_fused;
    }
    if let Some(beta_reduced) = beta_reduce_immediate_lambda_call(&node, &items) {
        return beta_reduced;
    }
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

fn fuse_apply_wrapper_call(node: &TypedExpression, call_items: &[Expression]) -> Option<TypedExpression> {
    let op = match call_items.first() {
        Some(Expression::Word(w)) => w.as_str(),
        _ => return None,
    };
    if !op.starts_with("std/fn/apply/") {
        return None;
    }
    if node.children.len() != call_items.len() {
        return None;
    }

    if op.starts_with("std/fn/apply/first/") {
        if call_items.len() < 2 {
            return None;
        }
        let callee = call_items.get(1)?.clone();
        if !is_lambda_expr(&callee) {
            return None;
        }
        let mut new_items = vec![callee];
        new_items.extend(call_items.iter().skip(2).cloned());

        let mut new_children = vec![node.children.get(1)?.clone()];
        new_children.extend(node.children.iter().skip(2).cloned());
        let rewritten = TypedExpression {
            expr: Expression::Apply(new_items),
            typ: node.typ.clone(),
            children: new_children,
        };
        return beta_reduce_immediate_lambda_call(
            &rewritten,
            match &rewritten.expr {
                Expression::Apply(items) => items,
                _ => return None,
            }
        );
    }

    // (std/fn/apply/N a b ... fn) => (fn a b ...)
    if call_items.len() < 2 {
        return None;
    }
    let callee = call_items.last()?.clone();
    if !is_lambda_expr(&callee) {
        return None;
    }
    let mut new_items = vec![callee];
    new_items.extend(call_items.iter().skip(1).take(call_items.len() - 2).cloned());

    let mut new_children = vec![node.children.last()?.clone()];
    new_children.extend(node.children.iter().skip(1).take(node.children.len() - 2).cloned());
    let rewritten = TypedExpression {
        expr: Expression::Apply(new_items),
        typ: node.typ.clone(),
        children: new_children,
    };
    beta_reduce_immediate_lambda_call(
        &rewritten,
        match &rewritten.expr {
            Expression::Apply(items) => items,
            _ => return None,
        }
    )
}

fn is_lambda_expr(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
    )
}

fn beta_reduce_immediate_lambda_call(
    node: &TypedExpression,
    call_items: &[Expression]
) -> Option<TypedExpression> {
    let Expression::Apply(lambda_items) = call_items.first()? else {
        return None;
    };
    if !matches!(lambda_items.first(), Some(Expression::Word(w)) if w == "lambda") {
        return None;
    }
    if lambda_items.len() < 2 || node.children.len() != call_items.len() {
        return None;
    }

    let params_expr = &lambda_items[1..lambda_items.len() - 1];
    if params_expr.len() != call_items.len().saturating_sub(1) {
        return None;
    }

    let lambda_typed = node.children.first()?;
    let body_expr = lambda_items.last()?;
    let body_typed = lambda_typed.children.last()?;

    let params = params_expr
        .iter()
        .map(|p| match p {
            Expression::Word(w) => Some(w.clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;

    let mut expr_subst: HashMap<String, Expression> = HashMap::new();
    let mut typed_subst: HashMap<String, TypedExpression> = HashMap::new();

    for (idx, param) in params.iter().enumerate() {
        let arg_expr = call_items.get(idx + 1)?.clone();
        let arg_node = node.children.get(idx + 1)?.clone();
        let arg_typ = arg_node.typ.as_ref()?;

        if !is_no_temp_inline_scalar_type(arg_typ) {
            return None;
        }

        let uses = count_word_uses_expr(body_expr, param);
        if uses > 1 && !is_atomic_inline_arg_expr(&arg_expr) {
            return None;
        }

        expr_subst.insert(param.clone(), arg_expr);
        typed_subst.insert(param.clone(), arg_node);
    }

    Some(substitute_params_typed(body_typed, &expr_subst, &typed_subst))
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

fn word_used_as_call_head(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(w)) if w == name) {
                return true;
            }
            items.iter().any(|it| word_used_as_call_head(it, name))
        }
        _ => false,
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
    inline_call_with_def(def, &call_items[1..], &node.children[1..], state)
}

fn inline_call_with_def(
    def: &InlineLambdaDef,
    arg_exprs: &[Expression],
    arg_nodes: &[TypedExpression],
    state: &mut InlineState
) -> Option<(Vec<(Expression, TypedExpression)>, Expression, TypedExpression)> {
    if arg_exprs.len() != def.params.len() || arg_nodes.len() != def.params.len() {
        return None;
    }

    let mut expr_subst: HashMap<String, Expression> = HashMap::new();
    let mut typed_subst: HashMap<String, TypedExpression> = HashMap::new();
    let mut prep = Vec::new();

    for (idx, param) in def.params.iter().enumerate() {
        let arg_expr = arg_exprs[idx].clone();
        let arg_node = arg_nodes[idx].clone();
        let arg_typ = arg_node.typ.as_ref()?;
        let uses = count_word_uses_expr(&def.body_expr, param);
        let head_used = word_used_as_call_head(&def.body_expr, param);
        let direct_lambda =
            is_lambda_expr(&arg_expr) && lambda_takes_only_scalar_args(arg_typ) && !head_used;
        let direct_scalar = is_no_temp_inline_scalar_type(arg_typ) && (uses <= 1 || is_atomic_inline_arg_expr(&arg_expr));
        if direct_lambda || direct_scalar {
            expr_subst.insert(param.clone(), arg_expr);
            typed_subst.insert(param.clone(), arg_node);
            continue;
        }

        let tmp = state.fresh_tmp();
        let tmp_expr = Expression::Word(tmp.clone());
        expr_subst.insert(param.clone(), tmp_expr.clone());
        typed_subst.insert(param.clone(), TypedExpression {
            expr: tmp_expr.clone(),
            typ: arg_node.typ.clone(),
            children: Vec::new(),
        });

        let let_expr = Expression::Apply(vec![
            Expression::Word("let".to_string()),
            Expression::Word(tmp.clone()),
            arg_exprs[idx].clone(),
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

    let inlined_expr = substitute_params_expr(&def.body_expr, &expr_subst);
    let inlined_typed = substitute_params_typed(&def.body_typed, &expr_subst, &typed_subst);
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
        let arg_typ = arg_node.typ.as_ref()?;
        if !is_no_temp_inline_scalar_type(arg_typ) {
            return None;
        }
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

fn is_no_temp_inline_scalar_type(typ: &Type) -> bool {
    matches!(typ, Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit)
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

fn lambda_takes_only_scalar_args(typ: &Type) -> bool {
    let mut cur = typ;
    loop {
        match cur {
            Type::Function(a, b) => {
                if !is_no_temp_inline_scalar_type(a) {
                    return false;
                }
                cur = b;
            }
            _ => return true,
        }
    }
}
