use crate::infer::{ EffectFlags, TypedExpression };
use crate::parser::Expression;
use crate::types::Type;
use std::collections::{ HashMap, HashSet };

const MAX_INLINE_BODY_COST: usize = 16;
const MAX_INLINE_FIXPOINT_PASSES: usize = 16;
const MAX_OPT_FIXPOINT_PASSES: usize = 8;

#[derive(Clone)]
enum MapFilterOp {
    Map {
        func: Expression,
        with_index: bool,
    },
    FlatMap {
        func: Expression,
    },
    Flat,
    Filter {
        predicate: Expression,
        keep_when_true: bool,
        with_index: bool,
    },
}

#[derive(Clone)]
enum FuseSink {
    Collect,
    Reduce {
        reduce_fn: Expression,
        init_expr: Expression,
        with_index: bool,
    },
    ReduceUntil {
        reduce_fn: Expression,
        stop_fn: Expression,
        init_expr: Expression,
        with_index: bool,
    },
    Average {
        dec: bool,
    },
    Unzip,
    Some {
        predicate: Expression,
        with_index: bool,
    },
    Every {
        predicate: Expression,
        with_index: bool,
    },
    Find {
        predicate: Expression,
    },
}

#[derive(Clone)]
enum FuseSource {
    Vector(Expression),
    Zip {
        left: Expression,
        right: Expression,
    },
    RangeInt {
        start: Expression,
        end: Expression,
    },
    RangeFloat {
        start: Expression,
        end: Expression,
    },
    Slice {
        xs: Expression,
        start: Expression,
        end: Expression,
    },
    Window {
        xs: Expression,
        size: Expression,
    },
}

#[derive(Default)]
struct FuseNameState {
    next_loop_id: usize,
}

impl FuseNameState {
    fn next_suffix(&mut self) -> String {
        let id = self.next_loop_id;
        self.next_loop_id += 1;
        if id == 0 {
            String::new()
        } else {
            format!("_{}", id)
        }
    }
}

pub fn optimize_typed_ast(node: &TypedExpression) -> TypedExpression {
    if std::env::var("QUE_DEBUG_DISABLE_OPTS").ok().as_deref() == Some("1") {
        return node.clone();
    }
    let mut seed = node.clone();
    let fused_expr = fuse_entry_expression_for_program(&node.expr);
    if fused_expr.to_lisp() != node.expr.to_lisp() {
        if
            let Ok((_typ, fused_typed)) = crate::infer::infer_with_builtins_typed(
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
            let next = run_tuple_return_destructuring_env_pass(&next);
            return dead_code_eliminate_top_level_defs(&next);
        }
        cur = next;
    }
    let cur = run_tuple_return_destructuring_env_pass(&cur);
    dead_code_eliminate_top_level_defs(&cur)
}

fn run_tuple_return_destructuring_env_pass(node: &TypedExpression) -> TypedExpression {
    let mut state = InlineState::new(&node.expr);
    rewrite_tuple_return_destructuring_with_env(node, &HashMap::new(), &mut state)
}

fn rewrite_tuple_return_destructuring_with_env(
    node: &TypedExpression,
    inherited_defs: &HashMap<String, TupleReturnLambdaDef>,
    state: &mut InlineState,
) -> TypedExpression {
    let Expression::Apply(items) = &node.expr else {
        let new_children = node
            .children
            .iter()
            .map(|child| rewrite_tuple_return_destructuring_with_env(child, inherited_defs, state))
            .collect::<Vec<_>>();
        return TypedExpression {
            expr: rebuild_expr_from_children(&node.expr, &new_children),
            typ: node.typ.clone(),
            effect: node.effect,
            children: new_children,
        };
    };

    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        let new_children = node
            .children
            .iter()
            .map(|child| rewrite_tuple_return_destructuring_with_env(child, inherited_defs, state))
            .collect::<Vec<_>>();
        return TypedExpression {
            expr: rebuild_expr_from_children(&node.expr, &new_children),
            typ: node.typ.clone(),
            effect: node.effect,
            children: new_children,
        };
    }

    let Some(normalized_do) = normalize_do_node(node, items) else {
        return node.clone();
    };
    let Expression::Apply(norm_items) = &normalized_do.expr else {
        return normalized_do;
    };
    let mut scoped_defs = inherited_defs.clone();
    let mut rebuilt_items = vec![norm_items[0].clone()];
    let mut rebuilt_children = vec![normalized_do.children[0].clone()];

    for idx in 1..norm_items.len() {
        let child = normalized_do.children.get(idx).cloned().unwrap_or_else(|| normalized_do.children[0].clone());
        let rewritten_child =
            rewrite_tuple_return_destructuring_with_env(&child, &scoped_defs, state);
        let rewritten_expr = rewritten_child.expr.clone();
        rebuilt_items.push(rewritten_expr.clone());
        rebuilt_children.push(rewritten_child.clone());
        if let Some((name, def)) = extract_tuple_return_lambda_def(&rewritten_expr, &rewritten_child) {
            scoped_defs.insert(name, def);
        }
    }

    let (rebuilt_items, rebuilt_children) = eliminate_tuple_return_destructuring_calls_with_defs(
        rebuilt_items,
        rebuilt_children,
        state,
        inherited_defs,
    );

    TypedExpression {
        expr: Expression::Apply(rebuilt_items),
        typ: normalized_do.typ.clone(),
        effect: normalized_do.effect,
        children: rebuilt_children,
    }
}

fn dead_code_eliminate_top_level_defs(node: &TypedExpression) -> TypedExpression {
    let Expression::Apply(items) = &node.expr else {
        return node.clone();
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") || items.len() <= 1 {
        return node.clone();
    }
    let Some(normalized_do) = normalize_do_node(node, items) else {
        return node.clone();
    };
    let Expression::Apply(norm_items) = &normalized_do.expr else {
        return normalized_do;
    };
    if norm_items.len() != normalized_do.children.len() {
        return normalized_do;
    }

    let mut defs_rhs: HashMap<String, Expression> = HashMap::new();
    let mut def_indices: HashMap<String, usize> = HashMap::new();
    let mut top_def_names: HashSet<String> = HashSet::new();
    let mut roots: Vec<Expression> = Vec::new();

    for (idx, item) in norm_items.iter().enumerate().skip(1) {
        if let Some((name, rhs)) = top_level_let_def(item) {
            defs_rhs.insert(name.clone(), rhs.clone());
            def_indices.insert(name.clone(), idx);
            top_def_names.insert(name.clone());
        } else {
            roots.push(item.clone());
        }
    }

    if defs_rhs.is_empty() {
        return normalized_do;
    }

    let mut needed: HashSet<String> = HashSet::new();
    for root in &roots {
        let mut refs = HashSet::new();
        let mut bound = HashSet::new();
        collect_unbound_words(root, &mut bound, &mut refs);
        for r in refs {
            if top_def_names.contains(&r) {
                needed.insert(r);
            }
        }
    }

    // Keep unused but non-removable top-level defs (impure or otherwise side-effectful),
    // and seed dependency closure from them too.
    for (name, idx) in &def_indices {
        if !needed.contains(name) && !top_level_let_rhs_is_removable(&normalized_do, *idx) {
            needed.insert(name.clone());
        }
    }

    let mut stack: Vec<String> = needed.iter().cloned().collect();
    while let Some(name) = stack.pop() {
        let Some(rhs) = defs_rhs.get(&name) else {
            continue;
        };
        let mut refs = HashSet::new();
        let mut bound = HashSet::new();
        collect_unbound_words(rhs, &mut bound, &mut refs);
        for r in refs {
            if top_def_names.contains(&r) && needed.insert(r.clone()) {
                stack.push(r);
            }
        }
    }

    let mut removed_any = false;
    let mut new_items = Vec::with_capacity(norm_items.len());
    let mut new_children = Vec::with_capacity(normalized_do.children.len());
    new_items.push(norm_items[0].clone());
    new_children.push(normalized_do.children[0].clone());

    for (idx, item) in norm_items.iter().enumerate().skip(1) {
        if let Some((name, _rhs)) = top_level_let_def(item) {
            if !needed.contains(name) {
                if top_level_let_rhs_is_removable(&normalized_do, idx) {
                    removed_any = true;
                    continue;
                }
            }
        }
        new_items.push(item.clone());
        if let Some(child) = normalized_do.children.get(idx).cloned() {
            new_children.push(child);
        } else {
            return normalized_do;
        }
    }

    if !removed_any {
        return normalized_do;
    }
    TypedExpression {
        expr: Expression::Apply(new_items),
        typ: normalized_do.typ.clone(),
        effect: normalized_do.effect,
        children: new_children,
    }
}

fn top_level_let_rhs_is_removable(do_node: &TypedExpression, idx: usize) -> bool {
    do_node.children
        .get(idx)
        .and_then(|let_node| let_node.children.get(2))
        .map(|rhs| {
            rhs.effect.is_pure() ||
                matches!(
                    &rhs.expr,
                    Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
                )
        })
        .unwrap_or(false)
}

fn top_level_let_def(expr: &Expression) -> Option<(&String, &Expression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), rhs] = &items[..] else {
        return None;
    };
    if kw == "let" || kw == "letrec" {
        Some((name, rhs))
    } else {
        None
    }
}

fn collect_bound_pattern_words(expr: &Expression, out: &mut HashSet<String>) {
    match expr {
        Expression::Word(w) => {
            out.insert(w.clone());
        }
        Expression::Apply(items) => {
            for it in items {
                collect_bound_pattern_words(it, out);
            }
        }
        _ => {}
    }
}

fn collect_unbound_words(
    expr: &Expression,
    bound: &mut HashSet<String>,
    out: &mut HashSet<String>
) {
    match expr {
        Expression::Word(w) => {
            if !bound.contains(w) {
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
                        collect_bound_pattern_words(p, &mut scoped);
                    }
                    if let Some(body) = items.last() {
                        collect_unbound_words(body, &mut scoped, out);
                    }
                    return;
                }
                if op == "do" {
                    for it in &items[1..] {
                        if let Some((name, rhs)) = top_level_let_def(it) {
                            collect_unbound_words(rhs, bound, out);
                            bound.insert(name.clone());
                            continue;
                        }
                        collect_unbound_words(it, bound, out);
                    }
                    return;
                }
                if op == "let" || op == "letrec" {
                    if let [_, bind, rhs] = &items[..] {
                        collect_unbound_words(rhs, bound, out);
                        collect_bound_pattern_words(bind, bound);
                        return;
                    }
                    if let Some(rhs) = items.get(2) {
                        collect_unbound_words(rhs, bound, out);
                    }
                    return;
                }
                // Type/cast hints are compile-time-only.
                if op == "as" || op == "char" {
                    if let Some(v) = items.get(1) {
                        collect_unbound_words(v, bound, out);
                    }
                    return;
                }
            }
            for it in items {
                collect_unbound_words(it, bound, out);
            }
        }
        _ => {}
    }
}

fn fuse_entry_expression_for_program(expr: &Expression) -> Expression {
    let mut name_state = FuseNameState::default();
    fuse_map_filter_reduce_chains_expr(expr, &mut name_state)
}

fn fuse_map_filter_reduce_chains_expr(
    expr: &Expression,
    name_state: &mut FuseNameState
) -> Expression {
    match expr {
        Expression::Apply(items) => {
            // Prioritize whole-chain fusions before rewriting children, so map/filter
            // combos become a single loop rather than nested loop wrappers.
            if let Some(fused) = fuse_terminal_over_map_filter_chain(expr, name_state) {
                return fuse_map_filter_reduce_chains_expr(&fused, name_state);
            }
            if let Some(fused) = fuse_map_filter_chain_to_collect(expr, name_state) {
                return fused;
            }

            let rewritten_items = items
                .iter()
                .map(|item| fuse_map_filter_reduce_chains_expr(item, name_state))
                .collect::<Vec<_>>();
            let rewritten = Expression::Apply(rewritten_items);
            rewritten
        }
        _ => expr.clone(),
    }
}

fn fuse_terminal_over_map_filter_chain(
    expr: &Expression,
    name_state: &mut FuseNameState
) -> Option<Expression> {
    let (sink, input_expr) = parse_terminal_call(expr)?;
    let (base, ops) = match collect_map_filter_chain(input_expr.clone()) {
        Some((base, ops)) => (base, ops),
        None => (input_expr, Vec::new()),
    };
    if !ops.iter().all(map_filter_op_is_fusion_safe) {
        return None;
    }
    if !sink_is_fusion_safe(&sink) {
        return None;
    }
    let source = parse_fuse_source(base);
    if matches!(source, FuseSource::Zip { .. }) || matches!(sink, FuseSink::Unzip) {
        return None;
    }
    build_direct_fused_loop(source, &ops, sink, name_state)
}

fn fuse_map_filter_chain_to_collect(
    expr: &Expression,
    name_state: &mut FuseNameState
) -> Option<Expression> {
    let (_, _) = parse_map_or_filter_call(expr)?;
    let (base, ops) = collect_map_filter_chain(expr.clone())?;
    if ops.is_empty() {
        return None;
    }
    if !ops.iter().all(map_filter_op_is_fusion_safe) {
        return None;
    }
    let source = parse_fuse_source(base);
    if matches!(source, FuseSource::Zip { .. }) && !zip_collect_fusion_is_supported(&ops) {
        return None;
    }
    build_direct_fused_loop(source, &ops, FuseSink::Collect, name_state)
}

fn parse_terminal_call(expr: &Expression) -> Option<(FuseSink, Expression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let Expression::Word(name) = items.first()? else {
        return None;
    };
    match name.as_str() {
        // reduce fn init xs
        "reduce" if items.len() == 4 =>
            Some((
                FuseSink::Reduce {
                    reduce_fn: items.get(1)?.clone(),
                    init_expr: items.get(2)?.clone(),
                    with_index: false,
                },
                items.get(3)?.clone(),
            )),
        // reduce/i fn init xs
        "reduce/i" if items.len() == 4 =>
            Some((
                FuseSink::Reduce {
                    reduce_fn: items.get(1)?.clone(),
                    init_expr: items.get(2)?.clone(),
                    with_index: true,
                },
                items.get(3)?.clone(),
            )),
        // reduce/until fn stop? init xs
        "reduce/until" if items.len() == 5 =>
            Some((
                FuseSink::ReduceUntil {
                    reduce_fn: items.get(1)?.clone(),
                    stop_fn: items.get(2)?.clone(),
                    init_expr: items.get(3)?.clone(),
                    with_index: false,
                },
                items.get(4)?.clone(),
            )),
        // reduce/until/i fn stop? init xs
        "reduce/until/i" if items.len() == 5 =>
            Some((
                FuseSink::ReduceUntil {
                    reduce_fn: items.get(1)?.clone(),
                    stop_fn: items.get(2)?.clone(),
                    init_expr: items.get(3)?.clone(),
                    with_index: true,
                },
                items.get(4)?.clone(),
            )),
        // sum xs => reduce + 0 xs
        "sum" | "sum/int" if items.len() == 2 =>
            Some((
                FuseSink::Reduce {
                    reduce_fn: Expression::Word("+".to_string()),
                    init_expr: Expression::Int(0),
                    with_index: false,
                },
                items.get(1)?.clone(),
            )),
        // sum/dec xs => reduce +. 0. xs
        "sum/dec" if items.len() == 2 =>
            Some((
                FuseSink::Reduce {
                    reduce_fn: Expression::Word("+.".to_string()),
                    init_expr: Expression::Dec(0.0),
                    with_index: false,
                },
                items.get(1)?.clone(),
            )),
        // product xs => reduce * 1 xs
        "product" | "product/int" if items.len() == 2 =>
            Some((
                FuseSink::Reduce {
                    reduce_fn: Expression::Word("*".to_string()),
                    init_expr: Expression::Int(1),
                    with_index: false,
                },
                items.get(1)?.clone(),
            )),
        // product xs => reduce *. 1. xs
        "product/dec" if items.len() == 2 =>
            Some((
                FuseSink::Reduce {
                    reduce_fn: Expression::Word("*.".to_string()),
                    init_expr: Expression::Dec(1.0),
                    with_index: false,
                },
                items.get(1)?.clone(),
            )),
        // mean aliases over vectors
        "mean" | "mean/int" if items.len() == 2 => {
            Some((FuseSink::Average { dec: false }, items.get(1)?.clone()))
        }
        "mean/dec" if items.len() == 2 => {
            Some((FuseSink::Average { dec: true }, items.get(1)?.clone()))
        }
        // unzip xs => tuple of mapped first/second in one pass
        "unzip" if items.len() == 2 => Some((FuseSink::Unzip, items.get(1)?.clone())),
        // some? pred xs
        "some?" if items.len() == 3 =>
            Some((
                FuseSink::Some {
                    predicate: items.get(1)?.clone(),
                    with_index: false,
                },
                items.get(2)?.clone(),
            )),
        // some/i? pred xs
        "some/i?" if items.len() == 3 =>
            Some((
                FuseSink::Some {
                    predicate: items.get(1)?.clone(),
                    with_index: true,
                },
                items.get(2)?.clone(),
            )),
        // every? pred xs
        "every?" if items.len() == 3 =>
            Some((
                FuseSink::Every {
                    predicate: items.get(1)?.clone(),
                    with_index: false,
                },
                items.get(2)?.clone(),
            )),
        // every/i? pred xs
        "every/i?" if items.len() == 3 =>
            Some((
                FuseSink::Every {
                    predicate: items.get(1)?.clone(),
                    with_index: true,
                },
                items.get(2)?.clone(),
            )),
        // find pred xs => first matching index, -1 if none
        "find" if items.len() == 3 =>
            Some((
                FuseSink::Find {
                    predicate: items.get(1)?.clone(),
                },
                items.get(2)?.clone(),
            )),
        _ => None,
    }
}

fn parse_map_or_filter_call(expr: &Expression) -> Option<(MapFilterOp, Expression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if items.len() < 2 {
        return None;
    }
    let Expression::Word(name) = items.first()? else {
        return None;
    };
    match name.as_str() {
        // map fn xs
        "map" =>
            Some((
                MapFilterOp::Map {
                    func: items.get(1)?.clone(),
                    with_index: false,
                },
                items.get(2)?.clone(),
            )),
        // map/i fn xs
        "map/i" =>
            Some((
                MapFilterOp::Map {
                    func: items.get(1)?.clone(),
                    with_index: true,
                },
                items.get(2)?.clone(),
            )),
        // flat-map fn xs
        "flat-map" if items.len() == 3 =>
            Some((
                MapFilterOp::FlatMap {
                    func: items.get(1)?.clone(),
                },
                items.get(2)?.clone(),
            )),
        // flat xs (one-level flatten)
        "flat" if items.len() == 2 => Some((MapFilterOp::Flat, items.get(1)?.clone())),
        // filter fn xs
        "filter" if items.len() == 3 =>
            Some((
                MapFilterOp::Filter {
                    predicate: items.get(1)?.clone(),
                    keep_when_true: true,
                    with_index: false,
                },
                items.get(2)?.clone(),
            )),
        // filter/i fn xs
        "filter/i" if items.len() == 3 =>
            Some((
                MapFilterOp::Filter {
                    predicate: items.get(1)?.clone(),
                    keep_when_true: true,
                    with_index: true,
                },
                items.get(2)?.clone(),
            )),
        // select fn xs (same behavior as filter)
        "select" if items.len() == 3 =>
            Some((
                MapFilterOp::Filter {
                    predicate: items.get(1)?.clone(),
                    keep_when_true: true,
                    with_index: false,
                },
                items.get(2)?.clone(),
            )),
        // exclude fn xs (inverse filter)
        "exclude" if items.len() == 3 =>
            Some((
                MapFilterOp::Filter {
                    predicate: items.get(1)?.clone(),
                    keep_when_true: false,
                    with_index: false,
                },
                items.get(2)?.clone(),
            )),
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

fn parse_fuse_source(base_expr: Expression) -> FuseSource {
    match &base_expr {
        Expression::Apply(items) if
            items.len() == 2 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "zip")
        => {
            if let Some((left, right)) = parse_zip_pair_expr(&items[1]) {
                FuseSource::Zip { left, right }
            } else {
                FuseSource::Vector(base_expr)
            }
        }
        Expression::Apply(items) if
            items.len() == 3 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "range")
        => {
            FuseSource::RangeInt {
                start: items[1].clone(),
                end: items[2].clone(),
            }
        }
        Expression::Apply(items) if
            items.len() == 3 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "range/int")
        => {
            FuseSource::RangeInt {
                start: items[1].clone(),
                end: items[2].clone(),
            }
        }
        Expression::Apply(items) if
            items.len() == 3 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "range/dec")
        => {
            FuseSource::RangeFloat {
                start: items[1].clone(),
                end: items[2].clone(),
            }
        }
        Expression::Apply(items) if
            items.len() == 4 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "slice")
        => {
            FuseSource::Slice {
                start: items[1].clone(),
                end: items[2].clone(),
                xs: items[3].clone(),
            }
        }
        Expression::Apply(items) if
            items.len() == 3 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "window")
        => {
            FuseSource::Window {
                size: items[1].clone(),
                xs: items[2].clone(),
            }
        }
        // take/first n xs => slice 0 n xs
        Expression::Apply(items) if
            items.len() == 3 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "take/first")
        => {
            FuseSource::Slice {
                start: Expression::Int(0),
                end: items[1].clone(),
                xs: items[2].clone(),
            }
        }
        // drop/first n xs => slice n (length xs) xs
        Expression::Apply(items) if
            items.len() == 3 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "drop/first")
        => {
            FuseSource::Slice {
                start: items[1].clone(),
                end: Expression::Apply(
                    vec![Expression::Word("length".to_string()), items[2].clone()]
                ),
                xs: items[2].clone(),
            }
        }
        // take/last n xs => slice (length xs - n) (length xs) xs
        Expression::Apply(items) if
            items.len() == 3 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "take/last")
        => {
            let len_expr = Expression::Apply(
                vec![Expression::Word("length".to_string()), items[2].clone()]
            );
            FuseSource::Slice {
                start: Expression::Apply(
                    vec![Expression::Word("-".to_string()), len_expr.clone(), items[1].clone()]
                ),
                end: len_expr,
                xs: items[2].clone(),
            }
        }
        // drop/last n xs => slice 0 (length xs - n) xs
        Expression::Apply(items) if
            items.len() == 3 &&
            matches!(items.first(), Some(Expression::Word(w)) if w == "drop/last")
        => {
            FuseSource::Slice {
                start: Expression::Int(0),
                end: Expression::Apply(
                    vec![
                        Expression::Word("-".to_string()),
                        Expression::Apply(
                            vec![Expression::Word("length".to_string()), items[2].clone()]
                        ),
                        items[1].clone()
                    ]
                ),
                xs: items[2].clone(),
            }
        }
        _ => FuseSource::Vector(base_expr),
    }
}

fn build_direct_fused_loop(
    source: FuseSource,
    ops_outer_to_inner: &[MapFilterOp],
    sink: FuseSink,
    name_state: &mut FuseNameState
) -> Option<Expression> {
    let has_flatten = ops_outer_to_inner
        .iter()
        .any(|op| matches!(op, MapFilterOp::Flat | MapFilterOp::FlatMap { .. }));
    if has_flatten {
        let has_indexed_stage = ops_outer_to_inner.iter().any(|op| {
            matches!(
                op,
                MapFilterOp::Map {
                    with_index: true,
                    ..
                } |
                    MapFilterOp::Filter {
                        with_index: true,
                        ..
                    }
            )
        });
        let unsupported_sink = match &sink {
            FuseSink::Collect => false,
            FuseSink::Reduce { with_index, .. } => *with_index,
            FuseSink::ReduceUntil { .. } => true,
            FuseSink::Average { .. } => true,
            FuseSink::Unzip => true,
            FuseSink::Some { .. } | FuseSink::Every { .. } | FuseSink::Find { .. } => true,
        };
        if has_indexed_stage || unsupported_sink {
            return None;
        }
    }
    let suffix = name_state.next_suffix();
    let (hoisted_bindings, hoisted_ops, hoisted_sink) = hoist_fusion_callables(
        ops_outer_to_inner,
        sink,
        &suffix
    );
    let fused = (match hoisted_sink {
        FuseSink::Some { predicate, with_index } =>
            build_some_every_loop(source, &hoisted_ops, predicate, with_index, true, &suffix),
        FuseSink::Every { predicate, with_index } =>
            build_some_every_loop(source, &hoisted_ops, predicate, with_index, false, &suffix),
        FuseSink::Collect => build_collect_loop(source, &hoisted_ops, &suffix),
        FuseSink::Reduce { reduce_fn, init_expr, with_index } =>
            build_reduce_loop(source, &hoisted_ops, reduce_fn, init_expr, with_index, &suffix),
        FuseSink::ReduceUntil { reduce_fn, stop_fn, init_expr, with_index } =>
            build_reduce_until_loop(
                source,
                &hoisted_ops,
                reduce_fn,
                stop_fn,
                init_expr,
                with_index,
                &suffix
            ),
        FuseSink::Average { dec } => build_average_loop(source, &hoisted_ops, dec, &suffix),
        FuseSink::Unzip => build_unzip_loop(source, &hoisted_ops, &suffix),
        FuseSink::Find { predicate } => build_find_loop(source, &hoisted_ops, predicate, &suffix),
    })?;
    if hoisted_bindings.is_empty() {
        Some(fused)
    } else {
        Some(
            Expression::Apply(
                vec![Expression::Word("do".to_string())]
                    .into_iter()
                    .chain(hoisted_bindings)
                    .chain(std::iter::once(fused))
                    .collect()
            )
        )
    }
}

fn fuse_tmp_name(base: &str, suffix: &str) -> String {
    if suffix.is_empty() { base.to_string() } else { format!("{}{}", base, suffix) }
}

fn build_while_range_body(
    start_expr: Expression,
    end_expr: Expression,
    i_name: &str,
    step_body: Expression
) -> Expression {
    let i_word = Expression::Word(i_name.to_string());
    let end_name = format!("{}_end", i_name);
    let end_word = Expression::Word(end_name.clone());
    let inc_i = Expression::Apply(
        vec![
            Expression::Word("alter!".to_string()),
            i_word.clone(),
            Expression::Apply(
                vec![Expression::Word("+".to_string()), i_word.clone(), Expression::Int(1)]
            )
        ]
    );
    let body = Expression::Apply(
        vec![
            Expression::Word("do".to_string()),
            step_body,
            inc_i,
            Expression::Word("nil".to_string())
        ]
    );
    Expression::Apply(
        vec![
            Expression::Word("do".to_string()),
            Expression::Apply(
                vec![
                    Expression::Word("mut".to_string()),
                    Expression::Word(i_name.to_string()),
                    start_expr
                ]
            ),
            Expression::Apply(
                vec![Expression::Word("let".to_string()), Expression::Word(end_name), end_expr]
            ),
            Expression::Apply(
                vec![
                    Expression::Word("while".to_string()),
                    Expression::Apply(vec![Expression::Word("<".to_string()), i_word, end_word]),
                    body
                ]
            )
        ]
    )
}

fn build_non_flatten_chain_process<F>(
    ops_outer_to_inner: &[MapFilterOp],
    input_value: Expression,
    raw_index: Expression,
    suffix: &str,
    setup_bindings: &mut Vec<Expression>,
    sink_builder: &F
) -> Option<Expression>
    where F: Fn(Expression, Expression) -> Option<Expression>
{
    let ops_inner_to_outer = ops_outer_to_inner.iter().rev().cloned().collect::<Vec<_>>();
    let mut filter_output_index_refs: Vec<Option<String>> = Vec::with_capacity(
        ops_inner_to_outer.len()
    );
    for (idx, op) in ops_inner_to_outer.iter().enumerate() {
        if matches!(op, MapFilterOp::Filter { .. }) {
            let ref_name = fuse_tmp_name(&format!("__fuse_idx_after_filter_{}", idx), suffix);
            setup_bindings.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(ref_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("vector".to_string()), Expression::Int(0)]
                        )
                    ]
                )
            );
            filter_output_index_refs.push(Some(ref_name));
        } else {
            filter_output_index_refs.push(None);
        }
    }

    build_non_flatten_chain_step(
        &ops_inner_to_outer,
        0,
        input_value,
        raw_index,
        &filter_output_index_refs,
        sink_builder
    )
}

fn build_non_flatten_chain_step<F>(
    ops_inner_to_outer: &[MapFilterOp],
    idx: usize,
    current_value: Expression,
    current_index: Expression,
    filter_output_index_refs: &[Option<String>],
    sink_builder: &F
) -> Option<Expression>
    where F: Fn(Expression, Expression) -> Option<Expression>
{
    if idx >= ops_inner_to_outer.len() {
        return sink_builder(current_value, current_index);
    }

    match &ops_inner_to_outer[idx] {
        MapFilterOp::Map { func, with_index } => {
            let mapped = if *with_index {
                call_callable_expr(func, vec![current_value, current_index.clone()])?
            } else {
                call_callable_expr(func, vec![current_value])?
            };
            build_non_flatten_chain_step(
                ops_inner_to_outer,
                idx + 1,
                mapped,
                current_index,
                filter_output_index_refs,
                sink_builder
            )
        }
        MapFilterOp::Filter { predicate, keep_when_true, with_index } => {
            let pred_value = if *with_index {
                call_callable_expr(predicate, vec![current_value.clone(), current_index.clone()])?
            } else {
                call_callable_expr(predicate, vec![current_value.clone()])?
            };
            let pass_cond = if *keep_when_true {
                pred_value
            } else {
                Expression::Apply(vec![Expression::Word("not".to_string()), pred_value])
            };

            let counter_name = match filter_output_index_refs.get(idx).and_then(|n| n.as_ref()) {
                Some(name) => name,
                None => {
                    return None;
                }
            };
            let counter_word = Expression::Word(counter_name.clone());
            let next_stage_index = Expression::Apply(
                vec![Expression::Word("get".to_string()), counter_word.clone(), Expression::Int(0)]
            );

            let then_stage = build_non_flatten_chain_step(
                ops_inner_to_outer,
                idx + 1,
                current_value,
                next_stage_index,
                filter_output_index_refs,
                sink_builder
            )?;
            let inc_counter = Expression::Apply(
                vec![
                    Expression::Word("set!".to_string()),
                    counter_word.clone(),
                    Expression::Int(0),
                    Expression::Apply(
                        vec![
                            Expression::Word("+".to_string()),
                            Expression::Apply(
                                vec![
                                    Expression::Word("get".to_string()),
                                    counter_word,
                                    Expression::Int(0)
                                ]
                            ),
                            Expression::Int(1)
                        ]
                    )
                ]
            );
            let then_expr = Expression::Apply(
                vec![
                    Expression::Word("do".to_string()),
                    then_stage,
                    inc_counter,
                    Expression::Word("nil".to_string())
                ]
            );
            Some(
                Expression::Apply(
                    vec![
                        Expression::Word("if".to_string()),
                        pass_cond,
                        then_expr,
                        no_op_unit_expr()
                    ]
                )
            )
        }
        MapFilterOp::Flat | MapFilterOp::FlatMap { .. } => None,
    }
}

fn build_collect_loop(
    source: FuseSource,
    ops_outer_to_inner: &[MapFilterOp],
    suffix: &str
) -> Option<Expression> {
    let (mut setup_bindings, start_expr, end_expr, value_expr_for_i) = make_loop_source_bindings(
        source,
        suffix
    )?;

    let out_name = fuse_tmp_name("__fuse_out", suffix);
    let i_name = fuse_tmp_name("__fuse_i", suffix);
    let i_word = Expression::Word(i_name.clone());
    let x_expr = value_expr_for_i(&i_word);
    let process_body = if
        ops_outer_to_inner
            .iter()
            .any(|op| matches!(op, MapFilterOp::Flat | MapFilterOp::FlatMap { .. }))
    {
        let mut flat_tmp_counter = 0usize;
        let ops_inner_to_outer = ops_outer_to_inner.iter().rev().cloned().collect::<Vec<_>>();
        build_collect_step_with_flatten(
            &ops_inner_to_outer,
            0,
            x_expr,
            i_word.clone(),
            &out_name,
            suffix,
            &mut flat_tmp_counter
        )?
    } else {
        let out_name_for_sink = out_name.clone();
        let sink_builder = |mapped: Expression, _logical_i: Expression| {
            Some(
                Expression::Apply(
                    vec![
                        Expression::Word("set!".to_string()),
                        Expression::Word(out_name_for_sink.clone()),
                        Expression::Apply(
                            vec![
                                Expression::Word("length".to_string()),
                                Expression::Word(out_name_for_sink.clone())
                            ]
                        ),
                        mapped
                    ]
                )
            )
        };
        build_non_flatten_chain_process(
            ops_outer_to_inner,
            x_expr,
            i_word.clone(),
            suffix,
            &mut setup_bindings,
            &sink_builder
        )?
    };
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string())])
            ]
        )
    );
    setup_bindings.push(build_while_range_body(start_expr, end_expr, &i_name, process_body));
    setup_bindings.push(Expression::Word(out_name));

    let mut do_items = vec![Expression::Word("do".to_string())];
    do_items.extend(setup_bindings);
    Some(Expression::Apply(do_items))
}

fn build_reduce_loop(
    source: FuseSource,
    ops_outer_to_inner: &[MapFilterOp],
    reduce_fn: Expression,
    init_expr: Expression,
    with_index: bool,
    suffix: &str
) -> Option<Expression> {
    let (mut setup_bindings, start_expr, end_expr, value_expr_for_i) = make_loop_source_bindings(
        source,
        suffix
    )?;

    let out_name = fuse_tmp_name("__fuse_out", suffix);
    let i_name = fuse_tmp_name("__fuse_i", suffix);
    let i_word = Expression::Word(i_name.clone());
    let x_expr = value_expr_for_i(&i_word);
    let process_body = if
        ops_outer_to_inner
            .iter()
            .any(|op| matches!(op, MapFilterOp::Flat | MapFilterOp::FlatMap { .. }))
    {
        let mut flat_tmp_counter = 0usize;
        let ops_inner_to_outer = ops_outer_to_inner.iter().rev().cloned().collect::<Vec<_>>();
        build_reduce_step_with_flatten(
            &ops_inner_to_outer,
            0,
            x_expr,
            i_word.clone(),
            &reduce_fn,
            &out_name,
            suffix,
            &mut flat_tmp_counter
        )?
    } else {
        let out_name_for_sink = out_name.clone();
        let reduce_fn_for_sink = reduce_fn.clone();
        let sink_builder = move |mapped: Expression, logical_i: Expression| {
            let acc_get = Expression::Apply(
                vec![
                    Expression::Word("get".to_string()),
                    Expression::Word(out_name_for_sink.clone()),
                    Expression::Int(0)
                ]
            );
            let reduced = if with_index {
                call_callable_expr(&reduce_fn_for_sink, vec![acc_get.clone(), mapped, logical_i])?
            } else {
                call_callable_expr(&reduce_fn_for_sink, vec![acc_get.clone(), mapped])?
            };
            Some(
                Expression::Apply(
                    vec![
                        Expression::Word("set!".to_string()),
                        Expression::Word(out_name_for_sink.clone()),
                        Expression::Int(0),
                        reduced
                    ]
                )
            )
        };
        build_non_flatten_chain_process(
            ops_outer_to_inner,
            x_expr,
            i_word.clone(),
            suffix,
            &mut setup_bindings,
            &sink_builder
        )?
    };
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string()), init_expr])
            ]
        )
    );
    setup_bindings.push(build_while_range_body(start_expr, end_expr, &i_name, process_body));
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("get".to_string()),
                Expression::Word(out_name),
                Expression::Int(0)
            ]
        )
    );

    let mut do_items = vec![Expression::Word("do".to_string())];
    do_items.extend(setup_bindings);
    Some(Expression::Apply(do_items))
}

fn build_reduce_until_loop(
    source: FuseSource,
    ops_outer_to_inner: &[MapFilterOp],
    reduce_fn: Expression,
    stop_fn: Expression,
    init_expr: Expression,
    with_index: bool,
    suffix: &str
) -> Option<Expression> {
    let (mut setup_bindings, idx_ref_name, cond_bound_expr, value_expr_for_idx_ref) =
        make_short_circuit_source_bindings(source, suffix)?;

    let out_name = fuse_tmp_name("__fuse_out", suffix);
    let placed_name = fuse_tmp_name("__fuse_placed", suffix);

    let idx_get = Expression::Apply(
        vec![
            Expression::Word("get".to_string()),
            Expression::Word(idx_ref_name.clone()),
            Expression::Int(0)
        ]
    );
    let x_expr = value_expr_for_idx_ref(&idx_get);
    let out_name_for_sink = out_name.clone();
    let placed_name_for_sink = placed_name.clone();
    let reduce_fn_for_sink = reduce_fn.clone();
    let stop_fn_for_sink = stop_fn.clone();
    let step_action = build_non_flatten_chain_process(
        ops_outer_to_inner,
        x_expr,
        idx_get.clone(),
        suffix,
        &mut setup_bindings,
        &(move |mapped: Expression, logical_i: Expression| {
            let acc_get = Expression::Apply(
                vec![
                    Expression::Word("get".to_string()),
                    Expression::Word(out_name_for_sink.clone()),
                    Expression::Int(0)
                ]
            );
            let stop_value = if with_index {
                call_callable_expr(
                    &stop_fn_for_sink,
                    vec![acc_get.clone(), mapped.clone(), logical_i.clone()]
                )?
            } else {
                call_callable_expr(&stop_fn_for_sink, vec![acc_get.clone(), mapped.clone()])?
            };
            let reduced = if with_index {
                call_callable_expr(&reduce_fn_for_sink, vec![acc_get, mapped, logical_i])?
            } else {
                call_callable_expr(&reduce_fn_for_sink, vec![acc_get, mapped])?
            };
            let set_placed_true = Expression::Apply(
                vec![
                    Expression::Word("set!".to_string()),
                    Expression::Word(placed_name_for_sink.clone()),
                    Expression::Int(0),
                    Expression::Word("true".to_string())
                ]
            );
            let set_out = Expression::Apply(
                vec![
                    Expression::Word("set!".to_string()),
                    Expression::Word(out_name_for_sink.clone()),
                    Expression::Int(0),
                    reduced
                ]
            );
            Some(
                Expression::Apply(
                    vec![Expression::Word("if".to_string()), stop_value, set_placed_true, set_out]
                )
            )
        })
    )?;
    let idx_inc = Expression::Apply(
        vec![
            Expression::Word("set!".to_string()),
            Expression::Word(idx_ref_name.clone()),
            Expression::Int(0),
            Expression::Apply(
                vec![
                    Expression::Word("+".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Int(1)
                ]
            )
        ]
    );
    let step_body = Expression::Apply(vec![Expression::Word("do".to_string()), step_action, idx_inc]);

    let continue_cond = Expression::Apply(
        vec![
            Expression::Word("and".to_string()),
            cond_bound_expr,
            Expression::Apply(
                vec![
                    Expression::Word("not".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(placed_name.clone()),
                            Expression::Int(0)
                        ]
                    )
                ]
            )
        ]
    );

    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string()), init_expr])
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(placed_name.clone()),
                Expression::Apply(
                    vec![
                        Expression::Word("vector".to_string()),
                        Expression::Word("false".to_string())
                    ]
                )
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("while".to_string()),
                continue_cond,
                step_body
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("get".to_string()),
                Expression::Word(out_name),
                Expression::Int(0)
            ]
        )
    );

    let mut do_items = vec![Expression::Word("do".to_string())];
    do_items.extend(setup_bindings);
    Some(Expression::Apply(do_items))
}

fn build_average_loop(
    source: FuseSource,
    ops_outer_to_inner: &[MapFilterOp],
    dec: bool,
    suffix: &str
) -> Option<Expression> {
    let (mut setup_bindings, start_expr, end_expr, value_expr_for_i) = make_loop_source_bindings(
        source,
        suffix
    )?;

    let sum_name = fuse_tmp_name("__fuse_sum", suffix);
    let count_name = fuse_tmp_name("__fuse_count", suffix);
    let i_name = fuse_tmp_name("__fuse_i", suffix);
    let i_word = Expression::Word(i_name.clone());
    let x_expr = value_expr_for_i(&i_word);
    let sum_name_for_sink = sum_name.clone();
    let count_name_for_sink = count_name.clone();
    let sink_builder = move |mapped: Expression, _logical_i: Expression| {
        let sum_get = Expression::Apply(
            vec![
                Expression::Word("get".to_string()),
                Expression::Word(sum_name_for_sink.clone()),
                Expression::Int(0)
            ]
        );
        let next_sum = Expression::Apply(
            vec![Expression::Word((if dec { "+." } else { "+" }).to_string()), sum_get, mapped]
        );
        let set_sum = Expression::Apply(
            vec![
                Expression::Word("set!".to_string()),
                Expression::Word(sum_name_for_sink.clone()),
                Expression::Int(0),
                next_sum
            ]
        );
        let set_count = Expression::Apply(
            vec![
                Expression::Word("set!".to_string()),
                Expression::Word(count_name_for_sink.clone()),
                Expression::Int(0),
                Expression::Apply(
                    vec![
                        Expression::Word("+".to_string()),
                        Expression::Apply(
                            vec![
                                Expression::Word("get".to_string()),
                                Expression::Word(count_name_for_sink.clone()),
                                Expression::Int(0)
                            ]
                        ),
                        Expression::Int(1)
                    ]
                )
            ]
        );
        Some(
            Expression::Apply(
                vec![
                    Expression::Word("do".to_string()),
                    set_sum,
                    set_count,
                    Expression::Word("nil".to_string())
                ]
            )
        )
    };
    let process_body = build_non_flatten_chain_process(
        ops_outer_to_inner,
        x_expr,
        i_word.clone(),
        suffix,
        &mut setup_bindings,
        &sink_builder
    )?;
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(sum_name.clone()),
                Expression::Apply(
                    vec![Expression::Word("vector".to_string()), if dec {
                        Expression::Dec(0.0)
                    } else {
                        Expression::Int(0)
                    }]
                )
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(count_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string()), Expression::Int(0)])
            ]
        )
    );
    setup_bindings.push(build_while_range_body(start_expr, end_expr, &i_name, process_body));

    let count_get = Expression::Apply(
        vec![Expression::Word("get".to_string()), Expression::Word(count_name), Expression::Int(0)]
    );
    let sum_get = Expression::Apply(
        vec![Expression::Word("get".to_string()), Expression::Word(sum_name), Expression::Int(0)]
    );
    let mean_expr = if dec {
        Expression::Apply(
            vec![
                Expression::Word("/.".to_string()),
                sum_get,
                Expression::Apply(vec![Expression::Word("Int->Dec".to_string()), count_get])
            ]
        )
    } else {
        Expression::Apply(vec![Expression::Word("/".to_string()), sum_get, count_get])
    };
    setup_bindings.push(mean_expr);

    let mut do_items = vec![Expression::Word("do".to_string())];
    do_items.extend(setup_bindings);
    Some(Expression::Apply(do_items))
}

fn build_unzip_loop(
    source: FuseSource,
    ops_outer_to_inner: &[MapFilterOp],
    suffix: &str
) -> Option<Expression> {
    let (mut setup_bindings, start_expr, end_expr, value_expr_for_i) = make_loop_source_bindings(
        source,
        suffix
    )?;

    let out_a_name = fuse_tmp_name("__fuse_out_a", suffix);
    let out_b_name = fuse_tmp_name("__fuse_out_b", suffix);
    let i_name = fuse_tmp_name("__fuse_i", suffix);
    let i_word = Expression::Word(i_name.clone());
    let x_expr = value_expr_for_i(&i_word);
    let out_a_for_sink = out_a_name.clone();
    let out_b_for_sink = out_b_name.clone();
    let sink_builder = move |mapped: Expression, _logical_i: Expression| {
        let push_a = Expression::Apply(
            vec![
                Expression::Word("set!".to_string()),
                Expression::Word(out_a_for_sink.clone()),
                Expression::Apply(
                    vec![
                        Expression::Word("length".to_string()),
                        Expression::Word(out_a_for_sink.clone())
                    ]
                ),
                Expression::Apply(vec![Expression::Word("fst".to_string()), mapped.clone()])
            ]
        );
        let push_b = Expression::Apply(
            vec![
                Expression::Word("set!".to_string()),
                Expression::Word(out_b_for_sink.clone()),
                Expression::Apply(
                    vec![
                        Expression::Word("length".to_string()),
                        Expression::Word(out_b_for_sink.clone())
                    ]
                ),
                Expression::Apply(vec![Expression::Word("snd".to_string()), mapped])
            ]
        );
        Some(
            Expression::Apply(
                vec![
                    Expression::Word("do".to_string()),
                    push_a,
                    push_b,
                    Expression::Word("nil".to_string())
                ]
            )
        )
    };
    let process_body = build_non_flatten_chain_process(
        ops_outer_to_inner,
        x_expr,
        i_word.clone(),
        suffix,
        &mut setup_bindings,
        &sink_builder
    )?;
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_a_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string())])
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_b_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string())])
            ]
        )
    );
    setup_bindings.push(build_while_range_body(start_expr, end_expr, &i_name, process_body));
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("tuple".to_string()),
                Expression::Word(out_a_name),
                Expression::Word(out_b_name)
            ]
        )
    );

    let mut do_items = vec![Expression::Word("do".to_string())];
    do_items.extend(setup_bindings);
    Some(Expression::Apply(do_items))
}

fn build_some_every_loop(
    source: FuseSource,
    ops_outer_to_inner: &[MapFilterOp],
    predicate: Expression,
    with_index: bool,
    is_some: bool,
    suffix: &str
) -> Option<Expression> {
    let (mut setup_bindings, idx_ref_name, cond_bound_expr, value_expr_for_idx_ref) =
        make_short_circuit_source_bindings(source, suffix)?;

    let flag_name = fuse_tmp_name("__fuse_flag", suffix);
    let idx_get = Expression::Apply(
        vec![
            Expression::Word("get".to_string()),
            Expression::Word(idx_ref_name.clone()),
            Expression::Int(0)
        ]
    );
    let x_expr = value_expr_for_idx_ref(&idx_get);

    let flag_get = Expression::Apply(
        vec![
            Expression::Word("get".to_string()),
            Expression::Word(flag_name.clone()),
            Expression::Int(0)
        ]
    );
    let set_flag_true = Expression::Apply(
        vec![
            Expression::Word("set!".to_string()),
            Expression::Word(flag_name.clone()),
            Expression::Int(0),
            Expression::Word("true".to_string())
        ]
    );
    let set_flag_false = Expression::Apply(
        vec![
            Expression::Word("set!".to_string()),
            Expression::Word(flag_name.clone()),
            Expression::Int(0),
            Expression::Word("false".to_string())
        ]
    );
    let predicate_for_sink = predicate.clone();
    let set_flag_true_for_sink = set_flag_true.clone();
    let set_flag_false_for_sink = set_flag_false.clone();
    let step_action = build_non_flatten_chain_process(
        ops_outer_to_inner,
        x_expr,
        idx_get.clone(),
        suffix,
        &mut setup_bindings,
        &(move |mapped: Expression, logical_i: Expression| {
            let pred_value = if with_index {
                call_callable_expr(&predicate_for_sink, vec![mapped, logical_i])?
            } else {
                call_callable_expr(&predicate_for_sink, vec![mapped])?
            };
            let action = if is_some {
                Expression::Apply(
                    vec![
                        Expression::Word("if".to_string()),
                        pred_value,
                        set_flag_true_for_sink.clone(),
                        no_op_unit_expr()
                    ]
                )
            } else {
                Expression::Apply(
                    vec![
                        Expression::Word("if".to_string()),
                        pred_value,
                        no_op_unit_expr(),
                        set_flag_false_for_sink.clone()
                    ]
                )
            };
            Some(action)
        })
    )?;
    let idx_inc = Expression::Apply(
        vec![
            Expression::Word("set!".to_string()),
            Expression::Word(idx_ref_name.clone()),
            Expression::Int(0),
            Expression::Apply(
                vec![
                    Expression::Word("+".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Int(1)
                ]
            )
        ]
    );
    let step_body = Expression::Apply(vec![Expression::Word("do".to_string()), step_action, idx_inc]);

    let continue_cond = if is_some {
        Expression::Apply(
            vec![
                Expression::Word("and".to_string()),
                cond_bound_expr,
                Expression::Apply(vec![Expression::Word("not".to_string()), flag_get.clone()])
            ]
        )
    } else {
        Expression::Apply(
            vec![Expression::Word("and".to_string()), cond_bound_expr, flag_get.clone()]
        )
    };

    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(flag_name.clone()),
                Expression::Apply(
                    vec![
                        Expression::Word("vector".to_string()),
                        Expression::Word((if is_some { "false" } else { "true" }).to_string())
                    ]
                )
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("while".to_string()),
                continue_cond,
                step_body
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("get".to_string()),
                Expression::Word(flag_name),
                Expression::Int(0)
            ]
        )
    );

    let mut do_items = vec![Expression::Word("do".to_string())];
    do_items.extend(setup_bindings);
    Some(Expression::Apply(do_items))
}

fn build_find_loop(
    source: FuseSource,
    ops_outer_to_inner: &[MapFilterOp],
    predicate: Expression,
    suffix: &str
) -> Option<Expression> {
    let (mut setup_bindings, idx_ref_name, cond_bound_expr, value_expr_for_idx_ref) =
        make_short_circuit_source_bindings(source, suffix)?;

    let out_name = fuse_tmp_name("__fuse_out", suffix);

    let idx_get = Expression::Apply(
        vec![
            Expression::Word("get".to_string()),
            Expression::Word(idx_ref_name.clone()),
            Expression::Int(0)
        ]
    );
    let x_expr = value_expr_for_idx_ref(&idx_get);
    let out_name_for_sink = out_name.clone();
    let predicate_for_sink = predicate.clone();
    let guarded_step = build_non_flatten_chain_process(
        ops_outer_to_inner,
        x_expr,
        idx_get.clone(),
        suffix,
        &mut setup_bindings,
        &(move |mapped: Expression, logical_i: Expression| {
            let pred_value = call_callable_expr(&predicate_for_sink, vec![mapped])?;
            let set_found = Expression::Apply(
                vec![
                    Expression::Word("set!".to_string()),
                    Expression::Word(out_name_for_sink.clone()),
                    Expression::Int(0),
                    logical_i
                ]
            );
            Some(
                Expression::Apply(
                    vec![
                        Expression::Word("if".to_string()),
                        pred_value,
                        set_found,
                        no_op_unit_expr()
                    ]
                )
            )
        })
    )?;
    let idx_inc = Expression::Apply(
        vec![
            Expression::Word("set!".to_string()),
            Expression::Word(idx_ref_name.clone()),
            Expression::Int(0),
            Expression::Apply(
                vec![
                    Expression::Word("+".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Int(1)
                ]
            )
        ]
    );
    let step_body = Expression::Apply(vec![Expression::Word("do".to_string()), guarded_step, idx_inc]);

    let continue_cond = Expression::Apply(
        vec![
            Expression::Word("and".to_string()),
            cond_bound_expr,
            Expression::Apply(
                vec![
                    Expression::Word("=".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(out_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Int(-1)
                ]
            )
        ]
    );

    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string()), Expression::Int(-1)])
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("while".to_string()),
                continue_cond,
                step_body
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("get".to_string()),
                Expression::Word(out_name),
                Expression::Int(0)
            ]
        )
    );

    let mut do_items = vec![Expression::Word("do".to_string())];
    do_items.extend(setup_bindings);
    Some(Expression::Apply(do_items))
}

fn make_loop_source_bindings(
    source: FuseSource,
    suffix: &str
) -> Option<(Vec<Expression>, Expression, Expression, Box<dyn Fn(&Expression) -> Expression>)> {
    match source {
        FuseSource::Vector(xs_expr) => {
            let xs_name = fuse_tmp_name("__fuse_xs", suffix);
            let xs_word = Expression::Word(xs_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(xs_name), xs_expr]
                )
            );
            let start_expr = Expression::Int(0);
            let end_expr = Expression::Apply(
                vec![Expression::Word("length".to_string()), xs_word.clone()]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![Expression::Word("get".to_string()), xs_word.clone(), i_expr.clone()]
                )
            });
            Some((setup, start_expr, end_expr, value))
        }
        FuseSource::Zip { left, right } => {
            let left_name = fuse_tmp_name("__fuse_left", suffix);
            let right_name = fuse_tmp_name("__fuse_right", suffix);
            let len_name = fuse_tmp_name("__fuse_zip_len", suffix);
            let left_word = Expression::Word(left_name.clone());
            let right_word = Expression::Word(right_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(left_name), left]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(right_name), right]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(len_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("length".to_string()), left_word.clone()]
                        )
                    ]
                )
            );
            let start_expr = Expression::Int(0);
            let end_expr = Expression::Word(len_name);
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("tuple".to_string()),
                        Expression::Apply(
                            vec![
                                Expression::Word("get".to_string()),
                                left_word.clone(),
                                i_expr.clone()
                            ]
                        ),
                        Expression::Apply(
                            vec![
                                Expression::Word("get".to_string()),
                                right_word.clone(),
                                i_expr.clone()
                            ]
                        )
                    ]
                )
            });
            Some((setup, start_expr, end_expr, value))
        }
        FuseSource::RangeInt { start, end } => {
            let from_name = fuse_tmp_name("__fuse_from", suffix);
            let to_name = fuse_tmp_name("__fuse_to", suffix);
            let from_word = Expression::Word(from_name.clone());
            let to_word = Expression::Word(to_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(from_name), start]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(to_name), end]
                )
            );
            // Use a normalized 0-based loop counter for consistent /i semantics.
            let start_expr = Expression::Int(0);
            // range is inclusive in user language, loop end is exclusive; normalize to count.
            let end_expr = Expression::Apply(
                vec![
                    Expression::Word("+".to_string()),
                    Expression::Apply(
                        vec![Expression::Word("-".to_string()), to_word, from_word.clone()]
                    ),
                    Expression::Int(1)
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![Expression::Word("+".to_string()), from_word.clone(), i_expr.clone()]
                )
            });
            Some((setup, start_expr, end_expr, value))
        }
        FuseSource::RangeFloat { start, end } => {
            let from_name = fuse_tmp_name("__fuse_from", suffix);
            let to_name = fuse_tmp_name("__fuse_to", suffix);
            let from_word = Expression::Word(from_name.clone());
            let to_word = Expression::Word(to_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(from_name), start]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(to_name), end]
                )
            );
            let start_expr = Expression::Int(0);
            let end_expr = Expression::Apply(
                vec![
                    Expression::Word("+".to_string()),
                    Expression::Apply(
                        vec![Expression::Word("-".to_string()), to_word, from_word.clone()]
                    ),
                    Expression::Int(1)
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("Int->Dec".to_string()),
                        Expression::Apply(
                            vec![
                                Expression::Word("+".to_string()),
                                from_word.clone(),
                                i_expr.clone()
                            ]
                        )
                    ]
                )
            });
            Some((setup, start_expr, end_expr, value))
        }
        FuseSource::Slice { xs, start, end } => {
            let xs_name = fuse_tmp_name("__fuse_xs", suffix);
            let from_name = fuse_tmp_name("__fuse_from", suffix);
            let to_name = fuse_tmp_name("__fuse_to", suffix);
            let xs_word = Expression::Word(xs_name.clone());
            let from_word = Expression::Word(from_name.clone());
            let to_word = Expression::Word(to_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(xs_name), xs]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(from_name), start]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(to_name), end]
                )
            );
            let start_expr = Expression::Int(0);
            let end_expr = Expression::Apply(
                vec![Expression::Word("-".to_string()), to_word, from_word.clone()]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("get".to_string()),
                        xs_word.clone(),
                        Expression::Apply(
                            vec![
                                Expression::Word("+".to_string()),
                                from_word.clone(),
                                i_expr.clone()
                            ]
                        )
                    ]
                )
            });
            Some((setup, start_expr, end_expr, value))
        }
        FuseSource::Window { xs, size } => {
            let xs_name = fuse_tmp_name("__fuse_xs", suffix);
            let size_name = fuse_tmp_name("__fuse_window_size", suffix);
            let len_name = fuse_tmp_name("__fuse_window_len", suffix);
            let end_name = fuse_tmp_name("__fuse_window_end", suffix);
            let xs_word = Expression::Word(xs_name.clone());
            let size_word = Expression::Word(size_name.clone());
            let len_word = Expression::Word(len_name.clone());
            let end_word = Expression::Word(end_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(xs_name), xs]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(size_name), size]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(len_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("length".to_string()), xs_word.clone()]
                        )
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(end_name),
                        build_window_end_expr(len_word.clone(), size_word.clone())
                    ]
                )
            );
            let start_expr = Expression::Int(0);
            let end_expr = end_word;
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("slice".to_string()),
                        i_expr.clone(),
                        Expression::Apply(
                            vec![
                                Expression::Word("+".to_string()),
                                i_expr.clone(),
                                size_word.clone()
                            ]
                        ),
                        xs_word.clone()
                    ]
                )
            });
            Some((setup, start_expr, end_expr, value))
        }
    }
}

fn make_short_circuit_source_bindings(
    source: FuseSource,
    suffix: &str
) -> Option<(Vec<Expression>, String, Expression, Box<dyn Fn(&Expression) -> Expression>)> {
    match source {
        FuseSource::Vector(xs_expr) => {
            let xs_name = fuse_tmp_name("__fuse_xs", suffix);
            let len_name = fuse_tmp_name("__fuse_len", suffix);
            let idx_ref_name = fuse_tmp_name("__fuse_i", suffix);
            let xs_word = Expression::Word(xs_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(xs_name), xs_expr]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(len_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("length".to_string()), xs_word.clone()]
                        )
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("vector".to_string()), Expression::Int(0)]
                        )
                    ]
                )
            );
            let cond = Expression::Apply(
                vec![
                    Expression::Word("<".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Word(len_name)
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![Expression::Word("get".to_string()), xs_word.clone(), i_expr.clone()]
                )
            });
            Some((setup, idx_ref_name, cond, value))
        }
        FuseSource::Zip { left, right } => {
            let left_name = fuse_tmp_name("__fuse_left", suffix);
            let right_name = fuse_tmp_name("__fuse_right", suffix);
            let len_name = fuse_tmp_name("__fuse_zip_len", suffix);
            let idx_ref_name = fuse_tmp_name("__fuse_i", suffix);
            let left_word = Expression::Word(left_name.clone());
            let right_word = Expression::Word(right_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(left_name), left]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(right_name), right]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(len_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("length".to_string()), left_word.clone()]
                        )
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("vector".to_string()), Expression::Int(0)]
                        )
                    ]
                )
            );
            let cond = Expression::Apply(
                vec![
                    Expression::Word("<".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Word(len_name)
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("tuple".to_string()),
                        Expression::Apply(
                            vec![
                                Expression::Word("get".to_string()),
                                left_word.clone(),
                                i_expr.clone()
                            ]
                        ),
                        Expression::Apply(
                            vec![
                                Expression::Word("get".to_string()),
                                right_word.clone(),
                                i_expr.clone()
                            ]
                        )
                    ]
                )
            });
            Some((setup, idx_ref_name, cond, value))
        }
        FuseSource::RangeInt { start, end } => {
            let idx_ref_name = fuse_tmp_name("__fuse_i", suffix);
            let from_name = fuse_tmp_name("__fuse_from", suffix);
            let to_name = fuse_tmp_name("__fuse_to", suffix);
            let from_word = Expression::Word(from_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(from_name), start]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(to_name.clone()),
                        end
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("vector".to_string()), Expression::Int(0)]
                        )
                    ]
                )
            );
            let cond = Expression::Apply(
                vec![
                    Expression::Word("<".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Apply(
                        vec![
                            Expression::Word("+".to_string()),
                            Expression::Apply(
                                vec![
                                    Expression::Word("-".to_string()),
                                    Expression::Word(to_name),
                                    from_word.clone()
                                ]
                            ),
                            Expression::Int(1)
                        ]
                    )
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![Expression::Word("+".to_string()), from_word.clone(), i_expr.clone()]
                )
            });
            Some((setup, idx_ref_name, cond, value))
        }
        FuseSource::RangeFloat { start, end } => {
            let idx_ref_name = fuse_tmp_name("__fuse_i", suffix);
            let from_name = fuse_tmp_name("__fuse_from", suffix);
            let to_name = fuse_tmp_name("__fuse_to", suffix);
            let from_word = Expression::Word(from_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(from_name), start]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(to_name.clone()),
                        end
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("vector".to_string()), Expression::Int(0)]
                        )
                    ]
                )
            );
            let cond = Expression::Apply(
                vec![
                    Expression::Word("<".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Apply(
                        vec![
                            Expression::Word("+".to_string()),
                            Expression::Apply(
                                vec![
                                    Expression::Word("-".to_string()),
                                    Expression::Word(to_name),
                                    from_word.clone()
                                ]
                            ),
                            Expression::Int(1)
                        ]
                    )
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("Int->Dec".to_string()),
                        Expression::Apply(
                            vec![
                                Expression::Word("+".to_string()),
                                from_word.clone(),
                                i_expr.clone()
                            ]
                        )
                    ]
                )
            });
            Some((setup, idx_ref_name, cond, value))
        }
        FuseSource::Slice { xs, start, end } => {
            let xs_name = fuse_tmp_name("__fuse_xs", suffix);
            let idx_ref_name = fuse_tmp_name("__fuse_i", suffix);
            let from_name = fuse_tmp_name("__fuse_from", suffix);
            let to_name = fuse_tmp_name("__fuse_to", suffix);
            let xs_word = Expression::Word(xs_name.clone());
            let from_word = Expression::Word(from_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(xs_name), xs]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(from_name), start]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("vector".to_string()), Expression::Int(0)]
                        )
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(to_name.clone()),
                        end
                    ]
                )
            );
            let cond = Expression::Apply(
                vec![
                    Expression::Word("<".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Apply(
                        vec![
                            Expression::Word("-".to_string()),
                            Expression::Word(to_name),
                            from_word.clone()
                        ]
                    )
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("get".to_string()),
                        xs_word.clone(),
                        Expression::Apply(
                            vec![
                                Expression::Word("+".to_string()),
                                from_word.clone(),
                                i_expr.clone()
                            ]
                        )
                    ]
                )
            });
            Some((setup, idx_ref_name, cond, value))
        }
        FuseSource::Window { xs, size } => {
            let xs_name = fuse_tmp_name("__fuse_xs", suffix);
            let size_name = fuse_tmp_name("__fuse_window_size", suffix);
            let len_name = fuse_tmp_name("__fuse_window_len", suffix);
            let end_name = fuse_tmp_name("__fuse_window_end", suffix);
            let idx_ref_name = fuse_tmp_name("__fuse_i", suffix);
            let xs_word = Expression::Word(xs_name.clone());
            let size_word = Expression::Word(size_name.clone());
            let len_word = Expression::Word(len_name.clone());
            let mut setup = Vec::new();
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(xs_name), xs]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![Expression::Word("let".to_string()), Expression::Word(size_name), size]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(len_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("length".to_string()), xs_word.clone()]
                        )
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(end_name.clone()),
                        build_window_end_expr(len_word, size_word.clone())
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(
                            vec![Expression::Word("vector".to_string()), Expression::Int(0)]
                        )
                    ]
                )
            );
            let cond = Expression::Apply(
                vec![
                    Expression::Word("<".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("get".to_string()),
                            Expression::Word(idx_ref_name.clone()),
                            Expression::Int(0)
                        ]
                    ),
                    Expression::Word(end_name)
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("slice".to_string()),
                        i_expr.clone(),
                        Expression::Apply(
                            vec![
                                Expression::Word("+".to_string()),
                                i_expr.clone(),
                                size_word.clone()
                            ]
                        ),
                        xs_word.clone()
                    ]
                )
            });
            Some((setup, idx_ref_name, cond, value))
        }
    }
}

fn build_window_end_expr(len_expr: Expression, size_expr: Expression) -> Expression {
    Expression::Apply(
        vec![
            Expression::Word("if".to_string()),
            Expression::Apply(
                vec![Expression::Word(">".to_string()), size_expr.clone(), len_expr.clone()]
            ),
            Expression::Int(0),
            Expression::Apply(
                vec![
                    Expression::Word("if".to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("=".to_string()),
                            size_expr.clone(),
                            Expression::Int(0)
                        ]
                    ),
                    len_expr.clone(),
                    Expression::Apply(
                        vec![
                            Expression::Word("+".to_string()),
                            Expression::Apply(
                                vec![Expression::Word("-".to_string()), len_expr, size_expr]
                            ),
                            Expression::Int(1)
                        ]
                    )
                ]
            )
        ]
    )
}

fn next_flatten_tmp_name(prefix: &str, suffix: &str, counter: &mut usize) -> String {
    let name = format!("{}_{}", fuse_tmp_name(prefix, suffix), *counter);
    *counter += 1;
    name
}

fn build_collect_step_with_flatten(
    ops_inner_to_outer: &[MapFilterOp],
    idx: usize,
    current_value: Expression,
    current_index: Expression,
    out_name: &str,
    suffix: &str,
    flat_tmp_counter: &mut usize
) -> Option<Expression> {
    if idx >= ops_inner_to_outer.len() {
        return Some(
            Expression::Apply(
                vec![
                    Expression::Word("set!".to_string()),
                    Expression::Word(out_name.to_string()),
                    Expression::Apply(
                        vec![
                            Expression::Word("length".to_string()),
                            Expression::Word(out_name.to_string())
                        ]
                    ),
                    current_value
                ]
            )
        );
    }

    match &ops_inner_to_outer[idx] {
        MapFilterOp::Map { func, with_index } => {
            let mapped = if *with_index {
                call_callable_expr(func, vec![current_value, current_index.clone()])?
            } else {
                call_callable_expr(func, vec![current_value])?
            };
            build_collect_step_with_flatten(
                ops_inner_to_outer,
                idx + 1,
                mapped,
                current_index,
                out_name,
                suffix,
                flat_tmp_counter
            )
        }
        MapFilterOp::Filter { predicate, keep_when_true, with_index } => {
            let pred = if *with_index {
                call_callable_expr(predicate, vec![current_value.clone(), current_index.clone()])?
            } else {
                call_callable_expr(predicate, vec![current_value.clone()])?
            };
            let cond = if *keep_when_true {
                pred
            } else {
                Expression::Apply(vec![Expression::Word("not".to_string()), pred])
            };
            let then_expr = build_collect_step_with_flatten(
                ops_inner_to_outer,
                idx + 1,
                current_value,
                current_index,
                out_name,
                suffix,
                flat_tmp_counter
            )?;
            Some(
                Expression::Apply(
                    vec![Expression::Word("if".to_string()), cond, then_expr, no_op_unit_expr()]
                )
            )
        }
        MapFilterOp::Flat | MapFilterOp::FlatMap { .. } => {
            let list_expr = match &ops_inner_to_outer[idx] {
                MapFilterOp::Flat => current_value,
                MapFilterOp::FlatMap { func } => call_callable_expr(func, vec![current_value])?,
                _ => unreachable!(),
            };
            let xs_name = next_flatten_tmp_name("__fuse_flat_xs", suffix, flat_tmp_counter);
            let i_name = next_flatten_tmp_name("__fuse_flat_i", suffix, flat_tmp_counter);
            let i_word = Expression::Word(i_name.clone());
            let item_expr = Expression::Apply(
                vec![
                    Expression::Word("get".to_string()),
                    Expression::Word(xs_name.clone()),
                    i_word.clone()
                ]
            );
            let process_body = build_collect_step_with_flatten(
                ops_inner_to_outer,
                idx + 1,
                item_expr,
                i_word,
                out_name,
                suffix,
                flat_tmp_counter
            )?;
            Some(
                Expression::Apply(
                    vec![
                        Expression::Word("do".to_string()),
                        Expression::Apply(
                            vec![
                                Expression::Word("let".to_string()),
                                Expression::Word(xs_name.clone()),
                                list_expr
                            ]
                        ),
                        build_while_range_body(
                            Expression::Int(0),
                            Expression::Apply(
                                vec![
                                    Expression::Word("length".to_string()),
                                    Expression::Word(xs_name)
                                ]
                            ),
                            &i_name,
                            process_body
                        )
                    ]
                )
            )
        }
    }
}

fn build_reduce_step_with_flatten(
    ops_inner_to_outer: &[MapFilterOp],
    idx: usize,
    current_value: Expression,
    current_index: Expression,
    reduce_fn: &Expression,
    out_name: &str,
    suffix: &str,
    flat_tmp_counter: &mut usize
) -> Option<Expression> {
    if idx >= ops_inner_to_outer.len() {
        let acc_get = Expression::Apply(
            vec![
                Expression::Word("get".to_string()),
                Expression::Word(out_name.to_string()),
                Expression::Int(0)
            ]
        );
        let reduced = call_callable_expr(reduce_fn, vec![acc_get, current_value])?;
        return Some(
            Expression::Apply(
                vec![
                    Expression::Word("set!".to_string()),
                    Expression::Word(out_name.to_string()),
                    Expression::Int(0),
                    reduced
                ]
            )
        );
    }

    match &ops_inner_to_outer[idx] {
        MapFilterOp::Map { func, with_index } => {
            let mapped = if *with_index {
                call_callable_expr(func, vec![current_value, current_index.clone()])?
            } else {
                call_callable_expr(func, vec![current_value])?
            };
            build_reduce_step_with_flatten(
                ops_inner_to_outer,
                idx + 1,
                mapped,
                current_index,
                reduce_fn,
                out_name,
                suffix,
                flat_tmp_counter
            )
        }
        MapFilterOp::Filter { predicate, keep_when_true, with_index } => {
            let pred = if *with_index {
                call_callable_expr(predicate, vec![current_value.clone(), current_index.clone()])?
            } else {
                call_callable_expr(predicate, vec![current_value.clone()])?
            };
            let cond = if *keep_when_true {
                pred
            } else {
                Expression::Apply(vec![Expression::Word("not".to_string()), pred])
            };
            let then_expr = build_reduce_step_with_flatten(
                ops_inner_to_outer,
                idx + 1,
                current_value,
                current_index,
                reduce_fn,
                out_name,
                suffix,
                flat_tmp_counter
            )?;
            Some(
                Expression::Apply(
                    vec![Expression::Word("if".to_string()), cond, then_expr, no_op_unit_expr()]
                )
            )
        }
        MapFilterOp::Flat | MapFilterOp::FlatMap { .. } => {
            let list_expr = match &ops_inner_to_outer[idx] {
                MapFilterOp::Flat => current_value,
                MapFilterOp::FlatMap { func } => call_callable_expr(func, vec![current_value])?,
                _ => unreachable!(),
            };
            let xs_name = next_flatten_tmp_name("__fuse_flat_xs", suffix, flat_tmp_counter);
            let i_name = next_flatten_tmp_name("__fuse_flat_i", suffix, flat_tmp_counter);
            let i_word = Expression::Word(i_name.clone());
            let item_expr = Expression::Apply(
                vec![
                    Expression::Word("get".to_string()),
                    Expression::Word(xs_name.clone()),
                    i_word.clone()
                ]
            );
            let process_body = build_reduce_step_with_flatten(
                ops_inner_to_outer,
                idx + 1,
                item_expr,
                i_word,
                reduce_fn,
                out_name,
                suffix,
                flat_tmp_counter
            )?;
            Some(
                Expression::Apply(
                    vec![
                        Expression::Word("do".to_string()),
                        Expression::Apply(
                            vec![
                                Expression::Word("let".to_string()),
                                Expression::Word(xs_name.clone()),
                                list_expr
                            ]
                        ),
                        build_while_range_body(
                            Expression::Int(0),
                            Expression::Apply(
                                vec![
                                    Expression::Word("length".to_string()),
                                    Expression::Word(xs_name)
                                ]
                            ),
                            &i_name,
                            process_body
                        )
                    ]
                )
            )
        }
    }
}

fn call_callable_expr(callable: &Expression, args: Vec<Expression>) -> Option<Expression> {
    match callable {
        Expression::Word(w) => {
            let mut items = Vec::with_capacity(1 + args.len());
            items.push(Expression::Word(w.clone()));
            items.extend(args);
            Some(Expression::Apply(items))
        }
        Expression::Apply(items) if
            matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
        => {
            if items.len() < 2 {
                return None;
            }
            let params = &items[1..items.len() - 1];
            if params.len() != args.len() {
                return None;
            }
            let mut out = alpha_rename_lambda_local_bindings(items.last()?, "__fuse_lambda");
            for (p, arg) in params.iter().zip(args.iter()) {
                let Expression::Word(name) = p else {
                    return None;
                };
                out = substitute_word_with_expr(&out, name, arg);
            }
            Some(out)
        }
        Expression::Apply(items) => {
            let mut out = items.clone();
            out.extend(args);
            Some(Expression::Apply(out))
        }
        _ => None,
    }
}

fn alpha_rename_lambda_local_bindings(expr: &Expression, prefix: &str) -> Expression {
    let mut counter = 0usize;
    alpha_rename_local_bindings_expr(expr, &mut HashMap::new(), &mut counter, prefix)
}

fn alpha_rename_local_bindings_typed(
    node: &TypedExpression,
    env: &mut HashMap<String, String>,
    state: &mut InlineState,
) -> TypedExpression {
    match &node.expr {
        Expression::Word(w) => {
            if let Some(mapped) = env.get(w) {
                TypedExpression {
                    expr: Expression::Word(mapped.clone()),
                    typ: node.typ.clone(),
                    effect: node.effect,
                    children: Vec::new(),
                }
            } else {
                node.clone()
            }
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "do") => {
            let mut scoped = env.clone();
            let children = node
                .children
                .iter()
                .map(|child| alpha_rename_local_bindings_typed(child, &mut scoped, state))
                .collect::<Vec<_>>();
            TypedExpression {
                expr: rebuild_expr_from_children(&node.expr, &children),
                typ: node.typ.clone(),
                effect: node.effect,
                children,
            }
        }
        Expression::Apply(items) if items.len() == 3 => {
            if let [Expression::Word(kw), Expression::Word(name), _] = &items[..] {
                if kw == "let" || kw == "mut" || kw == "letrec" {
                    let rhs = alpha_rename_local_bindings_typed(node.children.get(2).unwrap_or(node), env, state);
                    let fresh = state.fresh_tmp();
                    env.insert(name.clone(), fresh.clone());
                    let head = node.children.first().cloned().unwrap_or_else(|| pure_word(kw));
                    let bind = TypedExpression {
                        expr: Expression::Word(fresh.clone()),
                        typ: node.children.get(1).and_then(|n| n.typ.clone()),
                        effect: EffectFlags::PURE,
                        children: Vec::new(),
                    };
                    return TypedExpression {
                        expr: Expression::Apply(vec![
                            Expression::Word(kw.clone()),
                            Expression::Word(fresh),
                            rhs.expr.clone(),
                        ]),
                        typ: node.typ.clone(),
                        effect: node.effect,
                        children: vec![head, bind, rhs],
                    };
                }
            }
            let children = node
                .children
                .iter()
                .map(|child| {
                    let mut scoped = env.clone();
                    alpha_rename_local_bindings_typed(child, &mut scoped, state)
                })
                .collect::<Vec<_>>();
            TypedExpression {
                expr: rebuild_expr_from_children(&node.expr, &children),
                typ: node.typ.clone(),
                effect: node.effect,
                children,
            }
        }
        Expression::Apply(_) => {
            let children = node
                .children
                .iter()
                .map(|child| {
                    let mut scoped = env.clone();
                    alpha_rename_local_bindings_typed(child, &mut scoped, state)
                })
                .collect::<Vec<_>>();
            TypedExpression {
                expr: rebuild_expr_from_children(&node.expr, &children),
                typ: node.typ.clone(),
                effect: node.effect,
                children,
            }
        }
        _ => node.clone(),
    }
}

fn alpha_rename_local_bindings_expr(
    expr: &Expression,
    env: &mut HashMap<String, String>,
    counter: &mut usize,
    prefix: &str
) -> Expression {
    match expr {
        Expression::Word(w) =>
            env
                .get(w)
                .cloned()
                .map(Expression::Word)
                .unwrap_or_else(|| Expression::Word(w.clone())),
        Expression::Apply(items) if
            matches!(items.first(), Some(Expression::Word(w)) if w == "lambda")
        => {
            if items.len() < 2 {
                return Expression::Apply(items.clone());
            }
            let mut scoped = env.clone();
            for p in &items[1..items.len() - 1] {
                let mut bound = HashSet::new();
                collect_bound_pattern_words(p, &mut bound);
                for name in bound {
                    scoped.remove(&name);
                }
            }
            let mut out = items[..items.len() - 1].to_vec();
            if let Some(body) = items.last() {
                out.push(alpha_rename_local_bindings_expr(body, &mut scoped, counter, prefix));
            }
            Expression::Apply(out)
        }
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "do") => {
            let mut out = Vec::with_capacity(items.len());
            if let Some(head) = items.first() {
                out.push(head.clone());
            }
            for item in &items[1..] {
                out.push(alpha_rename_local_bindings_expr(item, env, counter, prefix));
            }
            Expression::Apply(out)
        }
        Expression::Apply(items) if items.len() == 3 => {
            if let [Expression::Word(kw), Expression::Word(name), rhs] = &items[..] {
                if kw == "let" || kw == "letrec" || kw == "mut" {
                    let fresh = format!("{}_{}_{}", prefix, name.replace('/', "_"), *counter);
                    *counter += 1;
                    let rewritten_rhs = if kw == "letrec" {
                        let mut rhs_env = env.clone();
                        rhs_env.insert(name.clone(), fresh.clone());
                        alpha_rename_local_bindings_expr(rhs, &mut rhs_env, counter, prefix)
                    } else {
                        alpha_rename_local_bindings_expr(rhs, env, counter, prefix)
                    };
                    env.insert(name.clone(), fresh.clone());
                    return Expression::Apply(vec![
                        Expression::Word(kw.clone()),
                        Expression::Word(fresh),
                        rewritten_rhs,
                    ]);
                }
            }
            Expression::Apply(
                items
                    .iter()
                    .map(|item| alpha_rename_local_bindings_expr(item, env, counter, prefix))
                    .collect()
            )
        }
        Expression::Apply(items) =>
            Expression::Apply(
                items
                    .iter()
                    .map(|item| {
                        let mut scoped = env.clone();
                        alpha_rename_local_bindings_expr(item, &mut scoped, counter, prefix)
                    })
                    .collect()
            ),
        Expression::Int(n) => Expression::Int(*n),
        Expression::Dec(n) => Expression::Dec(*n),
    }
}

fn no_op_unit_expr() -> Expression {
    // A std-independent no-op expression with Unit type.
    Expression::Word("nil".to_string())
}

fn map_filter_op_is_fusion_safe(op: &MapFilterOp) -> bool {
    match op {
        MapFilterOp::Map { func, .. } => is_fusion_safe_callable(func),
        MapFilterOp::Filter { predicate, .. } => is_fusion_safe_callable(predicate),
        MapFilterOp::Flat => true,
        MapFilterOp::FlatMap { func } => is_fusion_safe_callable(func),
    }
}

fn sink_is_fusion_safe(sink: &FuseSink) -> bool {
    match sink {
        FuseSink::Collect => true,
        FuseSink::Reduce { reduce_fn, .. } => is_fusion_safe_callable(reduce_fn),
        FuseSink::ReduceUntil { reduce_fn, stop_fn, .. } =>
            is_fusion_safe_callable(reduce_fn) && is_fusion_safe_callable(stop_fn),
        FuseSink::Average { .. } => true,
        FuseSink::Unzip => false,
        FuseSink::Some { predicate, .. } | FuseSink::Every { predicate, .. } => {
            is_fusion_safe_callable(predicate)
        }
        FuseSink::Find { predicate } => is_fusion_safe_callable(predicate),
    }
}

fn zip_collect_fusion_is_supported(ops: &[MapFilterOp]) -> bool {
    let [
        MapFilterOp::Map {
            func,
            with_index: false,
        },
    ] = ops
    else {
        return false;
    };
    zip_map_lambda_can_avoid_row_tuple(func)
}

fn zip_map_lambda_can_avoid_row_tuple(func: &Expression) -> bool {
    let Expression::Apply(items) = func else {
        return false;
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") || items.len() < 3 {
        return false;
    }
    let Some(body) = items.last() else {
        return false;
    };
    if expression_materializes_tuple(body) {
        return false;
    }
    items[1..items.len() - 1].iter().all(|param| {
        let Expression::Word(name) = param else {
            return false;
        };
        !expression_returns_word(body, name)
    })
}

fn expression_materializes_tuple(expr: &Expression) -> bool {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(w)) if w == "tuple" || w == "pair") {
                return true;
            }
            if matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
                return items
                    .last()
                    .map(expression_materializes_tuple)
                    .unwrap_or(false);
            }
            false
        }
        _ => false,
    }
}

fn expression_returns_word(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::Word(w) => w == name,
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "do") =>
            items.last()
                .map(|last| expression_returns_word(last, name))
                .unwrap_or(false),
        _ => false,
    }
}

fn parse_zip_pair_expr(expr: &Expression) -> Option<(Expression, Expression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let Expression::Word(head) = items.first()? else {
        return None;
    };
    if (head == "tuple" || head == "pair") && items.len() == 3 {
        Some((items[1].clone(), items[2].clone()))
    } else {
        None
    }
}

fn is_fusion_safe_callable(expr: &Expression) -> bool {
    match expr {
        Expression::Word(_) => true,
        Expression::Apply(_) => true,
        _ => false,
    }
}

fn fusion_callable_needs_hoist(expr: &Expression) -> bool {
    matches!(expr, Expression::Apply(items) if !matches!(items.first(), Some(Expression::Word(w)) if w == "lambda"))
}

fn hoist_fusion_callable_expr(
    expr: Expression,
    suffix: &str,
    counter: &mut usize,
    hoisted_bindings: &mut Vec<Expression>
) -> Expression {
    if !fusion_callable_needs_hoist(&expr) {
        return expr;
    }
    let name = fuse_tmp_name(&format!("__fuse_callable_{}", *counter), suffix);
    *counter += 1;
    hoisted_bindings.push(
        Expression::Apply(
            vec![Expression::Word("let".to_string()), Expression::Word(name.clone()), expr]
        )
    );
    Expression::Word(name)
}

fn hoist_fusion_callables(
    ops_outer_to_inner: &[MapFilterOp],
    sink: FuseSink,
    suffix: &str
) -> (Vec<Expression>, Vec<MapFilterOp>, FuseSink) {
    let mut counter = 0usize;
    let mut hoisted_bindings = Vec::new();
    let hoisted_ops = ops_outer_to_inner
        .iter()
        .cloned()
        .map(|op| {
            match op {
                MapFilterOp::Map { func, with_index } =>
                    MapFilterOp::Map {
                        func: hoist_fusion_callable_expr(
                            func,
                            suffix,
                            &mut counter,
                            &mut hoisted_bindings
                        ),
                        with_index,
                    },
                MapFilterOp::FlatMap { func } =>
                    MapFilterOp::FlatMap {
                        func: hoist_fusion_callable_expr(
                            func,
                            suffix,
                            &mut counter,
                            &mut hoisted_bindings
                        ),
                    },
                MapFilterOp::Flat => MapFilterOp::Flat,
                MapFilterOp::Filter { predicate, keep_when_true, with_index } =>
                    MapFilterOp::Filter {
                        predicate: hoist_fusion_callable_expr(
                            predicate,
                            suffix,
                            &mut counter,
                            &mut hoisted_bindings
                        ),
                        keep_when_true,
                        with_index,
                    },
            }
        })
        .collect::<Vec<_>>();
    let hoisted_sink = match sink {
        FuseSink::Collect => FuseSink::Collect,
        FuseSink::Reduce { reduce_fn, init_expr, with_index } =>
            FuseSink::Reduce {
                reduce_fn: hoist_fusion_callable_expr(
                    reduce_fn,
                    suffix,
                    &mut counter,
                    &mut hoisted_bindings
                ),
                init_expr,
                with_index,
            },
        FuseSink::ReduceUntil { reduce_fn, stop_fn, init_expr, with_index } =>
            FuseSink::ReduceUntil {
                reduce_fn: hoist_fusion_callable_expr(
                    reduce_fn,
                    suffix,
                    &mut counter,
                    &mut hoisted_bindings
                ),
                stop_fn: hoist_fusion_callable_expr(
                    stop_fn,
                    suffix,
                    &mut counter,
                    &mut hoisted_bindings
                ),
                init_expr,
                with_index,
            },
        FuseSink::Average { dec } => FuseSink::Average { dec },
        FuseSink::Unzip => FuseSink::Unzip,
        FuseSink::Some { predicate, with_index } =>
            FuseSink::Some {
                predicate: hoist_fusion_callable_expr(
                    predicate,
                    suffix,
                    &mut counter,
                    &mut hoisted_bindings
                ),
                with_index,
            },
        FuseSink::Every { predicate, with_index } =>
            FuseSink::Every {
                predicate: hoist_fusion_callable_expr(
                    predicate,
                    suffix,
                    &mut counter,
                    &mut hoisted_bindings
                ),
                with_index,
            },
        FuseSink::Find { predicate } =>
            FuseSink::Find {
                predicate: hoist_fusion_callable_expr(
                    predicate,
                    suffix,
                    &mut counter,
                    &mut hoisted_bindings
                ),
            },
    };
    (hoisted_bindings, hoisted_ops, hoisted_sink)
}

#[cfg(test)]
pub(crate) fn fuse_map_filter_reduce_for_test(expr: &Expression) -> Expression {
    let mut name_state = FuseNameState::default();
    fuse_map_filter_reduce_chains_expr(expr, &mut name_state)
}

fn optimize_typed_ast_once(node: &TypedExpression) -> TypedExpression {
    let optimized_children = node.children.iter().map(optimize_typed_ast_once).collect::<Vec<_>>();
    let rebuilt_expr = rebuild_expr_from_children(&node.expr, &optimized_children);
    let rebuilt_node = TypedExpression {
        expr: rebuilt_expr,
        typ: node.typ.clone(),
        effect: node.effect,
        children: optimized_children,
    };
    fold_constants(rebuilt_node)
}

fn rebuild_expr_from_children(expr: &Expression, children: &[TypedExpression]) -> Expression {
    match expr {
        Expression::Apply(items) if items.len() == children.len() => {
            Expression::Apply(
                children
                    .iter()
                    .map(|ch| ch.expr.clone())
                    .collect()
            )
        }
        _ => expr.clone(),
    }
}

fn fold_constants(node: TypedExpression) -> TypedExpression {
    let items = match &node.expr {
        Expression::Apply(items) => items.clone(),
        _ => {
            return node;
        }
    };
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
        "-" | "-#" => fold_int_sub(node, &items),
        "*" | "*#" => fold_int_mul(node, &items),
        "/" | "/#" => fold_int_checked_bin(node, &items, i32::checked_div),
        "mod" => fold_int_checked_bin(node, &items, i32::checked_rem),

        "=" | "=?" | "=#" => fold_int_cmp(node, &items, |a, b| a == b),
        "<" | "<#" => fold_int_cmp(node, &items, |a, b| a < b),
        ">" | ">#" => fold_int_cmp(node, &items, |a, b| a > b),
        "<=" | "<=#" => fold_int_cmp(node, &items, |a, b| a <= b),
        ">=" | ">=#" => fold_int_cmp(node, &items, |a, b| a >= b),

        "+." => fold_float_bin(node, &items, "+.", |a, b| a + b),
        "-." => fold_float_bin(node, &items, "-.", |a, b| a - b),
        "*." => fold_float_bin(node, &items, "*.", |a, b| a * b),
        "/." => fold_float_bin(node, &items, "/.", |a, b| a / b),
        "mod." => fold_float_bin(node, &items, "mod.", |a, b| a - (a / b).trunc() * b),

        "=." => fold_float_cmp(node, &items, |a, b| a == b),
        "<." => fold_float_cmp(node, &items, |a, b| a < b),
        ">." => fold_float_cmp(node, &items, |a, b| a > b),
        "<=." => fold_float_cmp(node, &items, |a, b| a <= b),
        ">=." => fold_float_cmp(node, &items, |a, b| a >= b),

        "Int->Dec" => fold_int_to_float(node, &items),
        "Dec->Int" => fold_float_to_int(node, &items),

        _ => node,
    }
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
        .map(|p| {
            match p {
                Expression::Word(w) => Some(w.clone()),
                _ => None,
            }
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
        if !can_no_temp_inline_arg(&arg_expr, &arg_node, uses) {
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
    let (inlined_items, inlined_children) = inline_do_simple_calls(
        &normalized_do,
        &mut inline_state
    );
    let (inlined_items, inlined_children) = eliminate_tuple_projection_lets(
        inlined_items,
        inlined_children
    );
    let (inlined_items, inlined_children) = eliminate_single_use_let_bindings(
        inlined_items,
        inlined_children
    );
    let tuple_inline_root = Expression::Apply(inlined_items.clone());
    let mut tuple_inline_state = InlineState::new(&tuple_inline_root);
    let (inlined_items, inlined_children) = eliminate_tuple_return_destructuring_calls(
        inlined_items,
        inlined_children,
        &mut tuple_inline_state,
    );

    let rebuilt_after_inline = TypedExpression {
        expr: Expression::Apply(inlined_items.clone()),
        typ: normalized_do.typ.clone(),
        effect: normalized_do.effect,
        children: inlined_children.clone(),
    };
    let rebuilt_after_inline = lower_non_escaping_local_cells_in_do(&rebuilt_after_inline)
        .unwrap_or(rebuilt_after_inline);
    let Expression::Apply(rewritten_items) = &rebuilt_after_inline.expr else {
        return rebuilt_after_inline;
    };
    let rewritten_children = rebuilt_after_inline.children.clone();
    if let Some(lowered) = lower_scalar_builder_do(&rebuilt_after_inline) {
        return lowered;
    }

    // Always collapse single-item do, even if no cleanup happened.
    if rewritten_items.len() == 2 {
        return rewritten_children
            .get(1)
            .cloned()
            .or_else(|| rewritten_children.last().cloned())
            .unwrap_or(rebuilt_after_inline);
    }

    let last_idx = rewritten_items.len() - 1;
    let mut kept_indices: Vec<usize> = Vec::new();
    kept_indices.push(0); // keep "do"
    for i in 1..last_idx {
        let Some(child) = rewritten_children.get(i) else {
            kept_indices.push(i);
            continue;
        };
        if !is_elidable_do_statement_expr(&rewritten_items[i], child) {
            kept_indices.push(i);
        }
    }
    kept_indices.push(last_idx);

    if kept_indices.len() == rewritten_items.len() {
        return TypedExpression {
            expr: Expression::Apply(rewritten_items.clone()),
            typ: rebuilt_after_inline.typ.clone(),
            effect: rebuilt_after_inline.effect,
            children: rewritten_children.clone(),
        };
    }

    // (do x) => x
    if kept_indices.len() == 2 {
        let only_expr_idx = kept_indices[1];
        return rewritten_children.get(only_expr_idx).cloned().unwrap_or(rebuilt_after_inline);
    }

    let new_expr_items = kept_indices
        .iter()
        .filter_map(|idx| rewritten_items.get(*idx).cloned())
        .collect::<Vec<_>>();
    let new_children = kept_indices
        .iter()
        .filter_map(|idx| rewritten_children.get(*idx).cloned())
        .collect::<Vec<_>>();

    TypedExpression {
        expr: Expression::Apply(new_expr_items),
        typ: rebuilt_after_inline.typ,
        effect: rebuilt_after_inline.effect,
        children: new_children,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LocalCellKind {
    Value,
    Bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LocalCellAliasKind {
    Set,
    Get,
}

fn lower_non_escaping_local_cells_in_do(node: &TypedExpression) -> Option<TypedExpression> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    if items.len() != node.children.len() || items.len() <= 1 {
        return None;
    }
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        return None;
    }

    let mut out_items = items.clone();
    let mut out_children = node.children.clone();
    let mut changed = false;
    let mut cell_set_aliases: HashSet<String> = HashSet::new();
    let mut cell_get_aliases: HashSet<String> = HashSet::new();

    for i in 1..out_items.len().saturating_sub(1) {
        if let Some((alias_name, alias_kind)) = extract_local_cell_alias(out_children.get(i)?) {
            match alias_kind {
                LocalCellAliasKind::Set => {
                    cell_set_aliases.insert(alias_name);
                }
                LocalCellAliasKind::Get => {
                    cell_get_aliases.insert(alias_name);
                }
            }
        }
        let Some((name, init_node, kind, payload_type)) =
            extract_non_escaping_local_cell_candidate(out_children.get(i)?) else {
            continue;
        };
        let end = find_next_do_rebinding(&out_items, &name, i + 1).unwrap_or(out_items.len());
        if out_items[i + 1..end]
            .iter()
            .any(|expr| contains_nested_lambda_or_letrec(expr) && contains_word(expr, &name))
        {
            continue;
        }

        let mut rewritten_range: Vec<(Expression, TypedExpression)> = Vec::new();
        let mut valid = true;
        for j in i + 1..end {
            let Some(rewritten) = rewrite_local_cell_uses_in_typed_expr(
                out_children.get(j)?,
                &name,
                kind,
                &payload_type,
                &cell_get_aliases,
                &cell_set_aliases,
                false,
            ) else {
                valid = false;
                break;
            };
            rewritten_range.push((rewritten.expr.clone(), rewritten));
        }
        if !valid {
            continue;
        }

        let rewritten_binding = make_mut_binding_from_local_cell(&name, init_node, &payload_type);
        out_items[i] = rewritten_binding.expr.clone();
        out_children[i] = rewritten_binding;
        for (offset, (expr, child)) in rewritten_range.into_iter().enumerate() {
            out_items[i + 1 + offset] = expr;
            out_children[i + 1 + offset] = child;
        }
        changed = true;
    }

    if !changed {
        return None;
    }

    Some(TypedExpression {
        expr: Expression::Apply(out_items),
        typ: node.typ.clone(),
        effect: node.effect,
        children: out_children,
    })
}

fn extract_non_escaping_local_cell_candidate(
    node: &TypedExpression,
) -> Option<(String, TypedExpression, LocalCellKind, Type)> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), _] = &items[..] else {
        return None;
    };
    if kw != "let" {
        return None;
    }
    let rhs = node.children.get(2)?;
    let Expression::Apply(rhs_items) = &rhs.expr else {
        return None;
    };
    let Some(Expression::Word(op)) = rhs_items.first() else {
        return None;
    };
    let kind = match op.as_str() {
        "box" | "int" | "dec" => LocalCellKind::Value,
        "bool" => LocalCellKind::Bool,
        _ => return None,
    };
    let init_node = rhs.children.get(1)?.clone();
    let payload_type = init_node.typ.clone()?;
    Some((name.clone(), init_node, kind, payload_type))
}

fn find_next_do_rebinding(items: &[Expression], name: &str, start: usize) -> Option<usize> {
    for idx in start..items.len() {
        let Expression::Apply(bind_items) = &items[idx] else {
            continue;
        };
        let [Expression::Word(kw), Expression::Word(bound), _] = &bind_items[..] else {
            continue;
        };
        if (kw == "let" || kw == "letrec" || kw == "mut") && bound == name {
            return Some(idx);
        }
    }
    None
}

fn extract_local_cell_alias(node: &TypedExpression) -> Option<(String, LocalCellAliasKind)> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), _] = &items[..] else {
        return None;
    };
    if kw != "let" && kw != "letrec" {
        return None;
    }
    let rhs = node.children.get(2)?;
    let Expression::Apply(lambda_items) = &rhs.expr else {
        return None;
    };
    if !matches!(lambda_items.first(), Some(Expression::Word(w)) if w == "lambda") {
        return None;
    }
    match lambda_items.as_slice() {
        [Expression::Word(_), Expression::Word(vrbl), body]
            if matches!(
                body,
                Expression::Apply(body_items)
                    if matches!(body_items.as_slice(),
                        [Expression::Word(op), Expression::Word(cell), Expression::Int(0)]
                            if op == "get" && cell == vrbl
                    )
            ) =>
        {
            Some((name.clone(), LocalCellAliasKind::Get))
        }
        [Expression::Word(_), Expression::Word(vrbl), Expression::Word(value), body]
            if matches!(
                body,
                Expression::Apply(body_items)
                    if matches!(body_items.as_slice(),
                        [Expression::Word(op), Expression::Word(cell), Expression::Int(0), Expression::Word(v)]
                            if op == "set!" && cell == vrbl && v == value
                    )
            ) =>
        {
            Some((name.clone(), LocalCellAliasKind::Set))
        }
        _ => None,
    }
}

fn typed_local_binding_name(node: &TypedExpression) -> Option<String> {
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

fn make_typed_word(name: &str, typ: Type) -> TypedExpression {
    TypedExpression {
        expr: Expression::Word(name.to_string()),
        typ: Some(typ),
        effect: EffectFlags::PURE,
        children: Vec::new(),
    }
}

fn make_mut_binding_from_local_cell(
    name: &str,
    init_node: TypedExpression,
    payload_type: &Type,
) -> TypedExpression {
    TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("mut".to_string()),
            Expression::Word(name.to_string()),
            init_node.expr.clone(),
        ]),
        typ: None,
        effect: init_node.effect | EffectFlags::MUTATE,
        children: vec![
            pure_word("mut"),
            make_typed_word(name, payload_type.clone()),
            init_node,
        ],
    }
}

fn rewrite_local_cell_uses_in_typed_expr(
    node: &TypedExpression,
    name: &str,
    kind: LocalCellKind,
    payload_type: &Type,
    cell_get_aliases: &HashSet<String>,
    cell_set_aliases: &HashSet<String>,
    shadowed: bool,
) -> Option<TypedExpression> {
    match &node.expr {
        Expression::Word(w) => {
            if !shadowed && w == name {
                None
            } else {
                Some(node.clone())
            }
        }
        Expression::Apply(items) => {
            let Some(Expression::Word(op)) = items.first() else {
                let rewritten_children = node
                    .children
                    .iter()
                    .map(|ch| {
                        rewrite_local_cell_uses_in_typed_expr(
                            ch,
                            name,
                            kind,
                            payload_type,
                            cell_get_aliases,
                            cell_set_aliases,
                            shadowed,
                        )
                    })
                    .collect::<Option<Vec<_>>>()?;
                return Some(TypedExpression {
                    expr: rebuild_expr_from_children(&node.expr, &rewritten_children),
                    typ: node.typ.clone(),
                    effect: node.effect,
                    children: rewritten_children,
                });
            };

            if !shadowed {
                match op.as_str() {
                    "get"
                        if items.len() == 3
                            && matches!(items.get(1), Some(Expression::Word(w)) if w == name)
                            && matches!(items.get(2), Some(Expression::Int(0))) =>
                    {
                        return Some(make_typed_word(name, payload_type.clone()));
                    }
                    op_name
                        if cell_get_aliases.contains(op_name)
                            && items.len() == 2
                            && matches!(items.get(1), Some(Expression::Word(w)) if w == name) =>
                    {
                        return Some(make_typed_word(name, payload_type.clone()));
                    }
                    "true?"
                        if items.len() == 2 && matches!(items.get(1), Some(Expression::Word(w)) if w == name) =>
                    {
                        if kind != LocalCellKind::Bool {
                            return None;
                        }
                        return Some(make_typed_word(name, Type::Bool));
                    }
                    "false?"
                        if items.len() == 2 && matches!(items.get(1), Some(Expression::Word(w)) if w == name) =>
                    {
                        if kind != LocalCellKind::Bool {
                            return None;
                        }
                        let local_word = make_typed_word(name, Type::Bool);
                        return Some(TypedExpression {
                            expr: Expression::Apply(vec![
                                Expression::Word("not".to_string()),
                                local_word.expr.clone(),
                            ]),
                            typ: Some(Type::Bool),
                            effect: EffectFlags::PURE,
                            children: vec![pure_word("not"), local_word],
                        });
                    }
                    "set!"
                        if items.len() == 4
                            && matches!(items.get(1), Some(Expression::Word(w)) if w == name)
                            && matches!(items.get(2), Some(Expression::Int(0))) =>
                    {
                        let value = rewrite_local_cell_uses_in_typed_expr(
                            node.children.get(3)?,
                            name,
                            kind,
                            payload_type,
                            cell_get_aliases,
                            cell_set_aliases,
                            shadowed,
                        )?;
                        return Some(TypedExpression {
                            expr: Expression::Apply(vec![
                                Expression::Word("alter!".to_string()),
                                Expression::Word(name.to_string()),
                                value.expr.clone(),
                            ]),
                            typ: node.typ.clone(),
                            effect: node.effect,
                            children: vec![
                                pure_word("alter!"),
                                make_typed_word(name, payload_type.clone()),
                                value,
                            ],
                        });
                    }
                    "set" | "=!" | "&alter!"
                        if items.len() == 3 && matches!(items.get(1), Some(Expression::Word(w)) if w == name) =>
                    {
                        let value = rewrite_local_cell_uses_in_typed_expr(
                            node.children.get(2)?,
                            name,
                            kind,
                            payload_type,
                            cell_get_aliases,
                            cell_set_aliases,
                            shadowed,
                        )?;
                        return Some(TypedExpression {
                            expr: Expression::Apply(vec![
                                Expression::Word("alter!".to_string()),
                                Expression::Word(name.to_string()),
                                value.expr.clone(),
                            ]),
                            typ: node.typ.clone(),
                            effect: node.effect,
                            children: vec![
                                pure_word("alter!"),
                                make_typed_word(name, payload_type.clone()),
                                value,
                            ],
                        });
                    }
                    op_name
                        if cell_set_aliases.contains(op_name)
                            && items.len() == 3
                            && matches!(items.get(1), Some(Expression::Word(w)) if w == name) =>
                    {
                        let value = rewrite_local_cell_uses_in_typed_expr(
                            node.children.get(2)?,
                            name,
                            kind,
                            payload_type,
                            cell_get_aliases,
                            cell_set_aliases,
                            shadowed,
                        )?;
                        return Some(TypedExpression {
                            expr: Expression::Apply(vec![
                                Expression::Word("alter!".to_string()),
                                Expression::Word(name.to_string()),
                                value.expr.clone(),
                            ]),
                            typ: node.typ.clone(),
                            effect: node.effect,
                            children: vec![
                                pure_word("alter!"),
                                make_typed_word(name, payload_type.clone()),
                                value,
                            ],
                        });
                    }
                    _ => {}
                }
            }

            if op == "do" {
                let Some(normalized_do) = normalize_do_node(node, items) else {
                    return Some(node.clone());
                };
                let Expression::Apply(do_items) = &normalized_do.expr else {
                    return Some(normalized_do);
                };
                let mut rebuilt_items = vec![do_items[0].clone()];
                let mut rebuilt_children = vec![normalized_do.children[0].clone()];
                let mut do_shadowed = shadowed;
                for idx in 1..do_items.len() {
                    let child = normalized_do.children.get(idx)?;
                    let rewritten = rewrite_local_cell_uses_in_typed_expr(
                        child,
                        name,
                        kind,
                        payload_type,
                        cell_get_aliases,
                        cell_set_aliases,
                        do_shadowed,
                    )?;
                    rebuilt_items.push(rewritten.expr.clone());
                    rebuilt_children.push(rewritten.clone());
                    if let Some(bound) = typed_local_binding_name(&rewritten) {
                        if bound == name {
                            do_shadowed = true;
                        }
                    }
                }
                return Some(TypedExpression {
                    expr: Expression::Apply(rebuilt_items),
                    typ: normalized_do.typ.clone(),
                    effect: normalized_do.effect,
                    children: rebuilt_children,
                });
            }

            let mut rewritten_children = Vec::with_capacity(node.children.len());
            let binding_shadows = if matches!(op.as_str(), "let" | "letrec" | "mut") {
                matches!(items.get(1), Some(Expression::Word(w)) if w == name)
            } else {
                false
            };
            for (idx, child) in node.children.iter().enumerate() {
                let child_shadowed = if op == "lambda" {
                    if idx + 1 == items.len() - 1 {
                        let mut bound = HashSet::new();
                        for p in &items[1..items.len().saturating_sub(1)] {
                            collect_bound_pattern_words(p, &mut bound);
                        }
                        shadowed || bound.contains(name)
                    } else {
                        shadowed
                    }
                } else if matches!(op.as_str(), "let" | "mut") {
                    shadowed || (binding_shadows && idx != 2)
                } else if op == "letrec" {
                    shadowed || binding_shadows
                } else {
                    shadowed
                };
                rewritten_children.push(rewrite_local_cell_uses_in_typed_expr(
                    child,
                    name,
                    kind,
                    payload_type,
                    cell_get_aliases,
                    cell_set_aliases,
                    child_shadowed,
                )?);
            }
            Some(TypedExpression {
                expr: rebuild_expr_from_children(&node.expr, &rewritten_children),
                typ: node.typ.clone(),
                effect: node.effect,
                children: rewritten_children,
            })
        }
        _ => Some(node.clone()),
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
            effect: EffectFlags::PURE,
            children: Vec::new(),
        });
        children.extend(node.children.clone());
        return Some(TypedExpression {
            expr: node.expr.clone(),
            typ: node.typ.clone(),
            effect: node.effect,
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
    fold_int_bin_with_overflow_policy(node, items, i32::wrapping_add, i32::checked_add)
}

fn fold_int_sub(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    fold_int_bin_with_overflow_policy(node, items, i32::wrapping_sub, i32::checked_sub)
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
    fold_int_bin_with_overflow_policy(node, items, i32::wrapping_mul, i32::checked_mul)
}

fn fold_if(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    if items.len() != 4 {
        return node;
    }
    let Some(cond) = items.get(1).and_then(bool_literal) else {
        if items[2].to_lisp() == items[3].to_lisp() {
            let Some(cond_node) = node.children.get(1).cloned() else {
                return node;
            };
            let Some(branch_node) = node.children.get(2).cloned() else {
                return node;
            };
            if cond_node.effect.is_pure() {
                return branch_node;
            }
            return make_do_pair(&node, cond_node, branch_node);
        }
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
    if let Some(rhs) = items.get(2).and_then(bool_literal) {
        let Some(lhs_node) = node.children.get(1).cloned() else {
            return node;
        };
        if rhs {
            return lhs_node;
        }
        let false_node = make_folded_literal(
            &node,
            Expression::Word("false".to_string()),
            Type::Bool
        );
        if lhs_node.effect.is_pure() {
            return false_node;
        }
        return make_do_pair(&node, lhs_node, false_node);
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
    if let Some(rhs) = items.get(2).and_then(bool_literal) {
        let Some(lhs_node) = node.children.get(1).cloned() else {
            return node;
        };
        if !rhs {
            return lhs_node;
        }
        let true_node = make_folded_literal(
            &node,
            Expression::Word("true".to_string()),
            Type::Bool
        );
        if lhs_node.effect.is_pure() {
            return true_node;
        }
        return make_do_pair(&node, lhs_node, true_node);
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
        Expression::Word((if !v { "true" } else { "false" }).to_string()),
        Type::Bool
    )
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

fn fold_int_bin_with_overflow_policy(
    node: TypedExpression,
    items: &[Expression],
    wrapping: fn(i32, i32) -> i32,
    checked: fn(i32, i32) -> Option<i32>
) -> TypedExpression {
    let (Some(a), Some(b)) = (
        items.get(1).and_then(int_literal),
        items.get(2).and_then(int_literal),
    ) else {
        return node;
    };
    if parse_env_bool_like("QUE_INT_OVERFLOW_CHECK", false) {
        let Some(v) = checked(a, b) else {
            return node;
        };
        return make_folded_literal(&node, Expression::Int(v), Type::Int);
    }
    make_folded_literal(&node, Expression::Int(wrapping(a, b)), Type::Int)
}

fn fold_int_checked_bin(
    node: TypedExpression,
    items: &[Expression],
    f: fn(i32, i32) -> Option<i32>
) -> TypedExpression {
    let (Some(a), Some(b)) = (
        items.get(1).and_then(int_literal),
        items.get(2).and_then(int_literal),
    ) else {
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
    let (Some(a), Some(b)) = (
        items.get(1).and_then(int_literal),
        items.get(2).and_then(int_literal),
    ) else {
        return node;
    };
    make_folded_literal(
        &node,
        Expression::Word((if f(a, b) { "true" } else { "false" }).to_string()),
        Type::Bool
    )
}

fn fold_float_bin(
    node: TypedExpression,
    items: &[Expression],
    op: &str,
    f: fn(f32, f32) -> f32
) -> TypedExpression {
    let (Some(a), Some(b)) = (
        items.get(1).and_then(float_literal),
        items.get(2).and_then(float_literal),
    ) else {
        return node;
    };
    if parse_env_bool_like("QUE_DIV_ZERO_CHECK", false) && (op == "/." || op == "mod.") && b == 0.0 {
        return node;
    }
    let result = f(a, b);
    if parse_env_bool_like("QUE_FLOAT_OVERFLOW_CHECK", false) && !result.is_finite() {
        return node;
    }
    make_folded_literal(&node, Expression::Dec(quantize_float_literal(result)), Type::Dec)
}

fn fold_float_cmp(
    node: TypedExpression,
    items: &[Expression],
    f: fn(f32, f32) -> bool
) -> TypedExpression {
    let (Some(a), Some(b)) = (
        items.get(1).and_then(float_literal),
        items.get(2).and_then(float_literal),
    ) else {
        return node;
    };
    make_folded_literal(
        &node,
        Expression::Word((if f(a, b) { "true" } else { "false" }).to_string()),
        Type::Bool
    )
}

fn fold_int_to_float(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    let Some(a) = items.get(1).and_then(int_literal) else {
        return node;
    };
    make_folded_literal(&node, Expression::Dec(quantize_float_literal(a as f32)), Type::Dec)
}

fn fold_float_to_int(node: TypedExpression, items: &[Expression]) -> TypedExpression {
    let Some(a) = items.get(1).and_then(float_literal) else {
        return node;
    };
    make_folded_literal(&node, Expression::Int(a.trunc() as i32), Type::Int)
}

fn lower_scalar_builder_do(node: &TypedExpression) -> Option<TypedExpression> {
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    if items.len() != 5 || node.children.len() != 5 {
        return None;
    }
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        return None;
    }

    let list_type = match node.typ.as_ref()? {
        Type::List(inner) => inner.as_ref(),
        _ => return None,
    };
    if !matches!(list_type, Type::Int | Type::Bool | Type::Char | Type::Dec) {
        return None;
    }

    let (board_name, board_rhs) = let_binding_parts(items.get(1)?)?;
    if !is_empty_vector_expr(board_rhs) {
        return None;
    }

    let (idx_name, idx_rhs) = let_binding_parts(items.get(2)?)?;
    if !matches!(idx_rhs, Expression::Int(0)) {
        return None;
    }

    if !matches!(items.get(4), Some(Expression::Word(w)) if w == board_name) {
        return None;
    }

    let while_node = node.children.get(3)?;
    let (len_node, value_node) = extract_counted_scalar_builder_parts(while_node, board_name, idx_name)?;
    if is_zero_scalar_literal_expr(&value_node.expr) {
        let op_expr = Expression::Apply(vec![
            Expression::Word("__vec_new_zeroed_i32".to_string()),
            len_node.expr.clone(),
        ]);
        return Some(TypedExpression {
            expr: op_expr,
            typ: node.typ.clone(),
            effect: EffectFlags::PURE,
            children: vec![
                pure_word("__vec_new_zeroed_i32"),
                len_node,
            ],
        });
    }

    let alloc_node = TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("__vec_new_uninit_i32".to_string()),
            len_node.expr.clone(),
        ]),
        typ: node.typ.clone(),
        effect: EffectFlags::PURE,
        children: vec![pure_word("__vec_new_uninit_i32"), len_node.clone()],
    };
    let board_let = TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("let".to_string()),
            Expression::Word(board_name.to_string()),
            alloc_node.expr.clone(),
        ]),
        typ: None,
        effect: alloc_node.effect,
        children: vec![pure_word("let"), pure_word(board_name), alloc_node],
    };
    let idx_mut = TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("mut".to_string()),
            Expression::Word(idx_name.to_string()),
            Expression::Int(0),
        ]),
        typ: None,
        effect: EffectFlags::MUTATE,
        children: vec![
            pure_word("mut"),
            pure_word(idx_name),
            TypedExpression {
                expr: Expression::Int(0),
                typ: Some(Type::Int),
                effect: EffectFlags::PURE,
                children: Vec::new(),
            },
        ],
    };
    let store_node = TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("__vec_store_i32".to_string()),
            Expression::Word(board_name.to_string()),
            Expression::Word(idx_name.to_string()),
            value_node.expr.clone(),
        ]),
        typ: Some(Type::Int),
        effect: EffectFlags::MUTATE,
        children: vec![
            pure_word("__vec_store_i32"),
            TypedExpression {
                expr: Expression::Word(board_name.to_string()),
                typ: node.typ.clone(),
                effect: EffectFlags::PURE,
                children: Vec::new(),
            },
            TypedExpression {
                expr: Expression::Word(idx_name.to_string()),
                typ: Some(Type::Int),
                effect: EffectFlags::PURE,
                children: Vec::new(),
            },
            value_node,
        ],
    };
    let alter_node = TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("alter!".to_string()),
            Expression::Word(idx_name.to_string()),
            Expression::Apply(vec![
                Expression::Word("+".to_string()),
                Expression::Word(idx_name.to_string()),
                Expression::Int(1),
            ]),
        ]),
        typ: Some(Type::Int),
        effect: EffectFlags::MUTATE,
        children: vec![
            pure_word("alter!"),
            pure_word(idx_name),
            TypedExpression {
                expr: Expression::Apply(vec![
                    Expression::Word("+".to_string()),
                    Expression::Word(idx_name.to_string()),
                    Expression::Int(1),
                ]),
                typ: Some(Type::Int),
                effect: EffectFlags::PURE,
                children: vec![
                    pure_word("+"),
                    TypedExpression {
                        expr: Expression::Word(idx_name.to_string()),
                        typ: Some(Type::Int),
                        effect: EffectFlags::PURE,
                        children: Vec::new(),
                    },
                    TypedExpression {
                        expr: Expression::Int(1),
                        typ: Some(Type::Int),
                        effect: EffectFlags::PURE,
                        children: Vec::new(),
                    },
                ],
            },
        ],
    };
    let while_body = TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("do".to_string()),
            store_node.expr.clone(),
            alter_node.expr.clone(),
        ]),
        typ: alter_node.typ.clone(),
        effect: store_node.effect | alter_node.effect,
        children: vec![pure_word("do"), store_node, alter_node],
    };
    let while_node = TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("while".to_string()),
            Expression::Apply(vec![
                Expression::Word("<".to_string()),
                Expression::Word(idx_name.to_string()),
                len_node.expr.clone(),
            ]),
            while_body.expr.clone(),
        ]),
        typ: while_node.typ.clone(),
        effect: EffectFlags::MUTATE,
        children: vec![
            pure_word("while"),
            TypedExpression {
                expr: Expression::Apply(vec![
                    Expression::Word("<".to_string()),
                    Expression::Word(idx_name.to_string()),
                    len_node.expr.clone(),
                ]),
                typ: Some(Type::Bool),
                effect: len_node.effect,
                children: vec![
                    pure_word("<"),
                    TypedExpression {
                        expr: Expression::Word(idx_name.to_string()),
                        typ: Some(Type::Int),
                        effect: EffectFlags::PURE,
                        children: Vec::new(),
                    },
                    len_node,
                ],
            },
            while_body,
        ],
    };
    let board_result = TypedExpression {
        expr: Expression::Word(board_name.to_string()),
        typ: node.typ.clone(),
        effect: EffectFlags::PURE,
        children: Vec::new(),
    };
    Some(TypedExpression {
        expr: Expression::Apply(vec![
            Expression::Word("do".to_string()),
            board_let.expr.clone(),
            idx_mut.expr.clone(),
            while_node.expr.clone(),
            board_result.expr.clone(),
        ]),
        typ: node.typ.clone(),
        effect: board_let.effect | idx_mut.effect | while_node.effect,
        children: vec![pure_word("do"), board_let, idx_mut, while_node, board_result],
    })
}

fn pure_word(word: &str) -> TypedExpression {
    TypedExpression {
        expr: Expression::Word(word.to_string()),
        typ: None,
        effect: EffectFlags::PURE,
        children: Vec::new(),
    }
}

fn let_binding_parts<'a>(expr: &'a Expression) -> Option<(&'a str, &'a Expression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), rhs] = &items[..] else {
        return None;
    };
    if kw != "let" && kw != "mut" {
        return None;
    }
    Some((name.as_str(), rhs))
}

fn is_empty_vector_expr(expr: &Expression) -> bool {
    matches!(expr, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(w)) if w == "vector") && items.len() == 1)
}

fn is_zero_scalar_literal_expr(expr: &Expression) -> bool {
    match expr {
        Expression::Int(0) => true,
        Expression::Dec(n) => *n == 0.0,
        Expression::Word(w) if w == "false" || w == "nil" => true,
        _ => false,
    }
}

fn extract_counted_scalar_builder_parts(
    while_node: &TypedExpression,
    board_name: &str,
    idx_name: &str,
) -> Option<(TypedExpression, TypedExpression)> {
    let Expression::Apply(items) = &while_node.expr else {
        return None;
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "while") || items.len() != 3 {
        return None;
    }
    let len_node = extract_counted_zero_fill_len_expr(while_node, idx_name)?;
    let Some(body_node) = while_node.children.get(2) else {
        return None;
    };
    let Expression::Apply(body) = &body_node.expr else {
        return None;
    };
    if !matches!(body.first(), Some(Expression::Word(w)) if w == "do") || body.len() != 3 || body_node.children.len() != 3 {
        return None;
    }
    let Some(set_node) = body_node.children.get(1) else {
        return None;
    };
    let Expression::Apply(set_items) = &set_node.expr else {
        return None;
    };
    if !matches!(set_items.first(), Some(Expression::Word(w)) if w == "set!") || set_items.len() != 4 {
        return None;
    }
    if !matches!(set_items.get(1), Some(Expression::Word(w)) if w == board_name) {
        return None;
    }
    if !matches!(
        set_items.get(2),
        Some(Expression::Apply(len_items))
            if matches!(len_items.first(), Some(Expression::Word(w)) if w == "length")
                && matches!(len_items.get(1), Some(Expression::Word(w)) if w == board_name)
    ) {
        return None;
    }
    let value_node = set_node.children.get(3)?.clone();
    if expression_mentions_name(&value_node.expr, board_name) {
        return None;
    }
    let Some(Expression::Apply(alter_items)) = body.get(2) else {
        return None;
    };
    if !matches!(
        &alter_items[..],
        [Expression::Word(op), Expression::Word(name), Expression::Apply(add_items)]
            if op == "alter!"
                && name == idx_name
                && matches!(add_items.first(), Some(Expression::Word(w)) if w == "+")
                && matches!(add_items.get(1), Some(Expression::Word(w)) if w == idx_name)
                && matches!(add_items.get(2), Some(Expression::Int(1)))
    ) {
        return None;
    }
    Some((len_node, value_node))
}

fn expression_mentions_name(expr: &Expression, name: &str) -> bool {
    match expr {
        Expression::Word(w) => w == name,
        Expression::Apply(items) => items.iter().any(|item| expression_mentions_name(item, name)),
        _ => false,
    }
}

fn extract_counted_zero_fill_len_expr<'a>(
    while_node: &'a TypedExpression,
    idx_name: &str,
) -> Option<TypedExpression> {
    let Expression::Apply(items) = &while_node.expr else {
        return None;
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "while") || while_node.children.len() != 3 {
        return None;
    }
    let cond_node = while_node.children.get(1)?;
    let Expression::Apply(cond_items) = &cond_node.expr else {
        return None;
    };
    if !matches!(cond_items.first(), Some(Expression::Word(w)) if w == "<") || cond_node.children.len() != 3 {
        return None;
    }
    if !matches!(cond_items.get(1), Some(Expression::Word(w)) if w == idx_name) {
        return None;
    }
    Some(cond_node.children.get(2)?.clone())
}

fn make_folded_literal(node: &TypedExpression, expr: Expression, typ: Type) -> TypedExpression {
    TypedExpression {
        expr,
        typ: node.typ.clone().or(Some(typ)),
        effect: EffectFlags::PURE,
        children: Vec::new(),
    }
}

fn make_do_pair(
    parent: &TypedExpression,
    first: TypedExpression,
    second: TypedExpression
) -> TypedExpression {
    TypedExpression {
        expr: Expression::Apply(
            vec![Expression::Word("do".to_string()), first.expr.clone(), second.expr.clone()]
        ),
        typ: second.typ.clone().or(parent.typ.clone()),
        effect: first.effect | second.effect,
        children: vec![
            TypedExpression {
                expr: Expression::Word("do".to_string()),
                typ: None,
                effect: EffectFlags::PURE,
                children: Vec::new(),
            },
            first,
            second
        ],
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
        Expression::Dec(v) => Some(quantize_float_literal(*v)),
        _ => None,
    }
}

fn quantize_float_literal(v: f32) -> f32 {
    let scale = decimal_scale_f32();
    (v * scale).round() / scale
}

fn decimal_scale_f32() -> f32 {
    match
        std::env
            ::var("QUE_DECIMAL_SCALE")
            .ok()
            .and_then(|v| v.trim().parse::<i32>().ok())
    {
        Some(scale) if scale > 0 && is_power_of_ten_i32(scale) && scale <= 1_000_000 => {
            scale as f32
        }
        _ => 1000.0,
    }
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

fn bool_literal(expr: &Expression) -> Option<bool> {
    match expr {
        Expression::Word(w) if w == "true" => Some(true),
        Expression::Word(w) if w == "false" => Some(false),
        _ => None,
    }
}

fn is_pure_literal_expr(expr: &Expression) -> bool {
    match expr {
        Expression::Int(_) | Expression::Dec(_) => true,
        Expression::Word(w) if w == "true" || w == "false" => true,
        _ => false,
    }
}

fn is_elidable_do_statement_expr(expr: &Expression, typed: &TypedExpression) -> bool {
    if !typed.effect.is_pure() {
        return false;
    }
    if is_pure_literal_expr(expr) {
        return true;
    }
    is_safe_pure_call_expr(expr)
}

fn is_safe_pure_call_expr(expr: &Expression) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    let Some(Expression::Word(op)) = items.first() else {
        return false;
    };
    if
        !matches!(
            op.as_str(),
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
                "~" |
                "^" |
                "|" |
                "&" |
                "<<" |
                ">>" |
                "length" |
                "Int->Dec" |
                "Dec->Int"
        )
    {
        return false;
    }

    items
        .iter()
        .skip(1)
        .all(|arg| {
            is_pure_literal_expr(arg) ||
                matches!(arg, Expression::Word(_)) ||
                is_safe_pure_call_expr(arg)
        })
}

#[derive(Clone)]
struct InlineLambdaDef {
    params: Vec<String>,
    body_expr: Expression,
    body_typed: TypedExpression,
}

#[derive(Clone)]
struct TupleReturnLambdaDef {
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
            effect: node.effect,
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

fn eliminate_tuple_return_destructuring_calls(
    items: Vec<Expression>,
    children: Vec<TypedExpression>,
    state: &mut InlineState,
) -> (Vec<Expression>, Vec<TypedExpression>) {
    eliminate_tuple_return_destructuring_calls_with_defs(
        items,
        children,
        state,
        &HashMap::new(),
    )
}

fn eliminate_tuple_return_destructuring_calls_with_defs(
    mut items: Vec<Expression>,
    mut children: Vec<TypedExpression>,
    state: &mut InlineState,
    seed_defs: &HashMap<String, TupleReturnLambdaDef>,
) -> (Vec<Expression>, Vec<TypedExpression>) {
    if items.len() != children.len() || items.len() <= 3 {
        return (items, children);
    }

    for _ in 0..MAX_INLINE_FIXPOINT_PASSES {
        let mut defs: HashMap<String, TupleReturnLambdaDef> = seed_defs.clone();
        let mut changed = false;
        let mut i = 1usize;
        while i < items.len() {
            if let Some((name, def)) = extract_tuple_return_lambda_def(&items[i], &children[i]) {
                defs.insert(name, def);
                i += 1;
                continue;
            }

            let Some((temp_name, call_expr, call_node)) = let_call_binding(&items[i], &children[i]) else {
                i += 1;
                continue;
            };
            let Some(def) = call_expr
                .first()
                .and_then(|head| match head {
                    Expression::Word(callee) => defs.get(callee),
                    _ => None,
                })
                .cloned() else {
                i += 1;
                continue;
            };

            let Some((projection_count, projected_bindings)) =
                contiguous_tuple_projection_bindings(&items, i + 1, &temp_name) else {
                i += 1;
                continue;
            };
            if projection_count == 0 || count_word_uses_in_slice(&items[i + 1 + projection_count..], &temp_name) != 0 {
                i += 1;
                continue;
            }

            let Some((prep, _inlined_expr, inlined_typed)) =
                inline_call_with_def(
                    &InlineLambdaDef {
                        params: def.params.clone(),
                        body_expr: def.body_expr.clone(),
                        body_typed: def.body_typed.clone(),
                    },
                    &call_expr[1..],
                    &call_node.children[1..],
                    state,
                ) else {
                    i += 1;
                    continue;
                };
            let inlined_typed =
                alpha_rename_local_bindings_typed(&inlined_typed, &mut HashMap::new(), state);
            let Some((prefix_pairs, fst_typed, snd_typed)) =
                extract_terminal_tuple_parts(&inlined_typed) else {
                    i += 1;
                    continue;
                };

            let fst_tmp = state.fresh_tmp();
            let snd_tmp = state.fresh_tmp();
            let fst_tmp_word = typed_word(fst_tmp.clone(), fst_typed.typ.clone());
            let snd_tmp_word = typed_word(snd_tmp.clone(), snd_typed.typ.clone());

            let mut replacement: Vec<(Expression, TypedExpression)> = prep;
            replacement.extend(prefix_pairs);
            replacement.push(make_let_binding(
                fst_tmp.clone(),
                fst_typed.clone(),
            ));
            replacement.push(make_let_binding(
                snd_tmp.clone(),
                snd_typed.clone(),
            ));
            for (bind_name, is_fst) in projected_bindings {
                let rhs = if is_fst {
                    fst_tmp_word.clone()
                } else {
                    snd_tmp_word.clone()
                };
                replacement.push(make_named_let_binding(bind_name, rhs));
            }

            items.splice(
                i..i + 1 + projection_count,
                replacement.iter().map(|(expr, _)| expr.clone()),
            );
            children.splice(
                i..i + 1 + projection_count,
                replacement.into_iter().map(|(_, typed)| typed),
            );
            changed = true;
            break;
        }
        if !changed {
            break;
        }
    }

    (items, children)
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
            let Some(rhs_typed) = children
                .get(i)
                .and_then(|n| n.children.get(2))
                .cloned() else {
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

fn eliminate_tuple_projection_lets(
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
            let Some((name, fst_typed, snd_typed)) = tuple_projection_let(&items[i], &children[i])
                else {
                    i += 1;
                    continue;
            };
            let end = find_shadowing_binding(&items, i + 1, &name).unwrap_or(items.len());
            if find_shadowing_binding(&items, 1, "fst").is_some_and(|idx| idx < end)
                || find_shadowing_binding(&items, 1, "snd").is_some_and(|idx| idx < end)
            {
                i += 1;
                continue;
            }
            let mut projection_uses = 0usize;
            let mut valid = true;
            for item in &items[i + 1..end] {
                match count_tuple_projection_uses(item, &name) {
                    Some(n) => projection_uses += n,
                    None => {
                        valid = false;
                        break;
                    }
                }
            }
            if !valid || projection_uses == 0 {
                i += 1;
                continue;
            }

            for j in i + 1..end {
                items[j] = replace_tuple_projection_expr(
                    &items[j],
                    &name,
                    &fst_typed.expr,
                    &snd_typed.expr
                );
                children[j] = replace_tuple_projection_typed(
                    &children[j],
                    &name,
                    &fst_typed,
                    &snd_typed
                );
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

fn tuple_projection_let(
    expr: &Expression,
    node: &TypedExpression
) -> Option<(String, TypedExpression, TypedExpression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), rhs] = &items[..] else {
        return None;
    };
    if kw != "let" && kw != "letrec" {
        return None;
    }
    if parse_zip_pair_expr(rhs).is_none() {
        return None;
    }
    let rhs_typed = node.children.get(2)?;
    let fst_typed = rhs_typed.children.get(1)?.clone();
    let snd_typed = rhs_typed.children.get(2)?.clone();
    Some((name.clone(), fst_typed, snd_typed))
}

fn find_shadowing_binding(items: &[Expression], from: usize, name: &str) -> Option<usize> {
    items.iter().enumerate().skip(from).find_map(|(idx, expr)| {
        let Expression::Apply(xs) = expr else {
            return None;
        };
        match &xs[..] {
            [Expression::Word(kw), Expression::Word(bound), _]
                if (kw == "let" || kw == "letrec" || kw == "mut") && bound == name =>
            {
                Some(idx)
            }
            _ => None,
        }
    })
}

fn count_tuple_projection_uses(expr: &Expression, name: &str) -> Option<usize> {
    match expr {
        Expression::Word(w) => {
            if w == name { None } else { Some(0) }
        }
        Expression::Apply(items) => {
            if is_direct_tuple_projection(items, name).is_some() {
                return Some(1);
            }
            if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
                let mut bound = HashSet::new();
                if items.len() >= 2 {
                    for p in &items[1..items.len() - 1] {
                        collect_bound_pattern_words(p, &mut bound);
                    }
                    if bound.contains(name) {
                        return Some(0);
                    }
                }
                let uses = items
                    .last()
                    .map(|body| count_tuple_projection_uses(body, name))
                    .unwrap_or(Some(0))?;
                return if uses == 0 { Some(0) } else { None };
            }
            let mut total = 0usize;
            for item in items {
                total += count_tuple_projection_uses(item, name)?;
            }
            Some(total)
        }
        _ => Some(0),
    }
}

fn is_direct_tuple_projection(items: &[Expression], name: &str) -> Option<bool> {
    match items {
        [Expression::Word(op), Expression::Word(arg)]
            if (op == "fst" || op == "snd") && arg == name =>
        {
            Some(op == "fst")
        }
        _ => None,
    }
}

fn replace_tuple_projection_expr(
    expr: &Expression,
    name: &str,
    fst_expr: &Expression,
    snd_expr: &Expression
) -> Expression {
    match expr {
        Expression::Apply(items) => {
            if let Some(op) = is_direct_tuple_projection(items, name) {
                return if op { fst_expr.clone() } else { snd_expr.clone() };
            }
            if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
                let mut bound = HashSet::new();
                if items.len() >= 2 {
                    for p in &items[1..items.len() - 1] {
                        collect_bound_pattern_words(p, &mut bound);
                    }
                    if bound.contains(name) {
                        return Expression::Apply(items.clone());
                    }
                }
            }
            Expression::Apply(
                items
                    .iter()
                    .map(|item| replace_tuple_projection_expr(item, name, fst_expr, snd_expr))
                    .collect()
            )
        }
        Expression::Word(w) => Expression::Word(w.clone()),
        Expression::Int(n) => Expression::Int(*n),
        Expression::Dec(n) => Expression::Dec(*n),
    }
}

fn replace_tuple_projection_typed(
    node: &TypedExpression,
    name: &str,
    fst_typed: &TypedExpression,
    snd_typed: &TypedExpression
) -> TypedExpression {
    if let Expression::Apply(items) = &node.expr {
        if let Some(op) = is_direct_tuple_projection(items, name) {
            return if op { fst_typed.clone() } else { snd_typed.clone() };
        }
        if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda") {
            let mut bound = HashSet::new();
            if items.len() >= 2 {
                for p in &items[1..items.len() - 1] {
                    collect_bound_pattern_words(p, &mut bound);
                }
                if bound.contains(name) {
                    return node.clone();
                }
            }
        }
    }

    let new_children = node.children
        .iter()
        .map(|ch| replace_tuple_projection_typed(ch, name, fst_typed, snd_typed))
        .collect::<Vec<_>>();
    let new_expr = match &node.expr {
        Expression::Apply(items) if items.len() == new_children.len() => {
            Expression::Apply(
                new_children
                    .iter()
                    .map(|ch| ch.expr.clone())
                    .collect()
            )
        }
        _ => replace_tuple_projection_expr(&node.expr, name, &fst_typed.expr, &snd_typed.expr),
    };

    TypedExpression {
        expr: new_expr,
        typ: node.typ.clone(),
        effect: node.effect,
        children: new_children,
    }
}

fn eliminable_let_name(expr: &Expression) -> Option<(String, bool)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), rhs] = &items[..] else {
        return None;
    };
    if kw != "let" && kw != "letrec" {
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
    items
        .iter()
        .map(|e| count_word_uses_expr(e, name))
        .sum()
}

fn count_word_uses_expr(expr: &Expression, name: &str) -> usize {
    match expr {
        Expression::Word(w) => {
            if w == name { 1 } else { 0 }
        }
        Expression::Apply(items) =>
            items
                .iter()
                .map(|it| count_word_uses_expr(it, name))
                .sum(),
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

fn substitute_word_with_expr(
    expr: &Expression,
    name: &str,
    replacement: &Expression
) -> Expression {
    match expr {
        Expression::Word(w) => {
            if w == name { replacement.clone() } else { Expression::Word(w.clone()) }
        }
        Expression::Apply(items) =>
            Expression::Apply(
                items
                    .iter()
                    .map(|it| substitute_word_with_expr(it, name, replacement))
                    .collect()
            ),
        Expression::Int(n) => Expression::Int(*n),
        Expression::Dec(n) => Expression::Dec(*n),
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
        Expression::Apply(items) if items.len() == new_children.len() => {
            Expression::Apply(
                new_children
                    .iter()
                    .map(|ch| ch.expr.clone())
                    .collect()
            )
        }
        _ => substitute_word_with_expr(&node.expr, name, &replacement.expr),
    };

    TypedExpression {
        expr: new_expr,
        typ: node.typ.clone(),
        effect: node.effect,
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

        if
            let Some((prep, rewritten_expr, rewritten_child)) = try_inline_let_rhs(
                expr_i,
                child_i,
                &defs,
                state
            )
        {
            changed = true;
            for (e, c) in prep {
                out_items.push(e);
                out_children.push(c);
            }
            out_items.push(rewritten_expr);
            out_children.push(rewritten_child);
            continue;
        }

        if
            let Some((prep, inlined_expr, inlined_child)) = try_inline_call(
                expr_i,
                child_i,
                &defs,
                state
            )
        {
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
    if kw != "let" && kw != "letrec" {
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
        .map(|p| {
            match p {
                Expression::Word(w) => Some(w.clone()),
                _ => None,
            }
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

fn extract_tuple_return_lambda_def(
    expr: &Expression,
    node: &TypedExpression,
) -> Option<(String, TupleReturnLambdaDef)> {
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
    if kw != "let" && kw != "letrec" {
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
    if contains_word(&body_expr, name) || contains_nested_lambda_or_letrec(&body_expr) {
        return None;
    }
    let body_typed = node.children.get(2)?.children.last()?.clone();
    extract_terminal_tuple_parts(&body_typed)?;
    let params = lambda_items[1..lambda_items.len() - 1]
        .iter()
        .map(|p| match p {
            Expression::Word(w) => Some(w.clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some((
        name.clone(),
        TupleReturnLambdaDef {
            params,
            body_expr,
            body_typed,
        },
    ))
}

fn contains_nested_lambda_or_letrec(expr: &Expression) -> bool {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(w)) if w == "lambda" || w == "letrec") {
                return true;
            }
            items.iter().any(contains_nested_lambda_or_letrec)
        }
        _ => false,
    }
}

fn let_call_binding<'a>(
    expr: &'a Expression,
    node: &'a TypedExpression,
) -> Option<(String, &'a [Expression], &'a TypedExpression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let [Expression::Word(kw), Expression::Word(name), Expression::Apply(call_expr)] = &items[..] else {
        return None;
    };
    if kw != "let" && kw != "letrec" {
        return None;
    }
    let rhs_typed = node.children.get(2)?;
    Some((name.clone(), call_expr.as_slice(), rhs_typed))
}

fn contiguous_tuple_projection_bindings(
    items: &[Expression],
    start: usize,
    temp_name: &str,
) -> Option<(usize, Vec<(String, bool)>)> {
    let mut consumed = 0usize;
    let mut current_name = temp_name.to_string();
    let mut idx = start;
    while idx < items.len() {
        let Expression::Apply(let_items) = &items[idx] else {
            break;
        };
        let [Expression::Word(kw), Expression::Word(bind_name), Expression::Word(rhs_name)] = &let_items[..] else {
            break;
        };
        if (kw != "let" && kw != "letrec") || rhs_name != &current_name {
            break;
        }
        current_name = bind_name.clone();
        consumed += 1;
        idx += 1;
    }

    let mut out = Vec::new();
    while idx < items.len() {
        let Expression::Apply(let_items) = &items[idx] else {
            break;
        };
        let [Expression::Word(kw), Expression::Word(bind_name), rhs] = &let_items[..] else {
            break;
        };
        if kw != "let" && kw != "letrec" {
            break;
        }
        let Expression::Apply(rhs_items) = rhs else {
            break;
        };
        let Some(Expression::Word(op)) = rhs_items.first() else {
            break;
        };
        if rhs_items.len() != 2 || !matches!(rhs_items.get(1), Some(Expression::Word(w)) if w == &current_name) {
            break;
        }
        let is_fst = match op.as_str() {
            "fst" => true,
            "snd" => false,
            _ => break,
        };
        out.push((bind_name.clone(), is_fst));
        idx += 1;
        consumed += 1;
        if out.len() == 2 {
            break;
        }
    }
    if out.is_empty() {
        None
    } else {
        Some((consumed, out))
    }
}

fn extract_terminal_tuple_parts(
    node: &TypedExpression,
) -> Option<(Vec<(Expression, TypedExpression)>, TypedExpression, TypedExpression)> {
    if parse_zip_pair_expr(&node.expr).is_some() && node.children.len() >= 3 {
        return Some((Vec::new(), node.children.get(1)?.clone(), node.children.get(2)?.clone()));
    }
    let Expression::Apply(items) = &node.expr else {
        return None;
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") || items.len() != node.children.len() {
        return None;
    }
    let last = node.children.last()?;
    let (mut prefix, fst_typed, snd_typed) = extract_terminal_tuple_parts(last)?;
    let direct_prefix = items[1..items.len() - 1]
        .iter()
        .cloned()
        .zip(node.children[1..node.children.len() - 1].iter().cloned())
        .collect::<Vec<_>>();
    let mut all_prefix = direct_prefix;
    all_prefix.append(&mut prefix);
    Some((all_prefix, fst_typed, snd_typed))
}

fn typed_word(name: String, typ: Option<Type>) -> TypedExpression {
    TypedExpression {
        expr: Expression::Word(name),
        typ,
        effect: EffectFlags::PURE,
        children: Vec::new(),
    }
}

fn make_let_binding(name: String, rhs: TypedExpression) -> (Expression, TypedExpression) {
    let rhs_typ = rhs.typ.clone();
    let expr = Expression::Apply(vec![
        Expression::Word("let".to_string()),
        Expression::Word(name.clone()),
        rhs.expr.clone(),
    ]);
    let typed = TypedExpression {
        expr: expr.clone(),
        typ: rhs_typ.clone(),
        effect: rhs.effect,
        children: vec![
            pure_word("let"),
            typed_word(name, rhs_typ),
            rhs,
        ],
    };
    (expr, typed)
}

fn make_named_let_binding(name: String, rhs: TypedExpression) -> (Expression, TypedExpression) {
    let expr = Expression::Apply(vec![
        Expression::Word("let".to_string()),
        Expression::Word(name.clone()),
        rhs.expr.clone(),
    ]);
    let typed = TypedExpression {
        expr: expr.clone(),
        typ: rhs.typ.clone(),
        effect: rhs.effect,
        children: vec![
            pure_word("let"),
            typed_word(name, rhs.typ.clone()),
            rhs,
        ],
    };
    (expr, typed)
}

fn is_inline_safe_body(expr: &Expression) -> bool {
    match expr {
        Expression::Int(_) | Expression::Dec(_) | Expression::Word(_) => true,
        Expression::Apply(items) => {
            if let Some(Expression::Word(head)) = items.first() {
                if head == "let" || head == "letrec" || head == "lambda" {
                    return false;
                }
            }
            items.iter().all(is_inline_safe_body)
        }
    }
}

fn inline_body_cost(expr: &Expression) -> usize {
    match expr {
        Expression::Int(_) | Expression::Dec(_) | Expression::Word(_) => 1,
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
        _ => {
            return None;
        }
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
        let can_no_temp = can_no_temp_inline_arg(&arg_expr, &arg_node, uses);
        let direct_lambda =
            can_no_temp &&
            is_lambda_expr(&arg_expr) &&
            lambda_takes_only_scalar_args(arg_typ) &&
            !head_used;
        let direct_scalar =
            can_no_temp &&
            is_no_temp_inline_scalar_type(arg_typ) &&
            (uses <= 1 || is_atomic_inline_arg_expr(&arg_expr));
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
            effect: EffectFlags::PURE,
            children: Vec::new(),
        });

        let let_expr = Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(tmp.clone()),
                arg_exprs[idx].clone()
            ]
        );
        let let_typed = TypedExpression {
            expr: let_expr.clone(),
            typ: arg_node.typ.clone(),
            effect: arg_node.effect,
            children: vec![
                TypedExpression {
                    expr: Expression::Word("let".to_string()),
                    typ: None,
                    effect: EffectFlags::PURE,
                    children: Vec::new(),
                },
                TypedExpression {
                    expr: Expression::Word(tmp),
                    typ: arg_node.typ.clone(),
                    effect: EffectFlags::PURE,
                    children: Vec::new(),
                },
                arg_node
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
        effect: node.effect,
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
        _ => {
            return None;
        }
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
        if !can_no_temp_inline_arg(&arg_expr, &arg_node, uses) {
            return None;
        }
        expr_subst.insert(param.clone(), arg_expr);
        typed_subst.insert(param.clone(), arg_node);
    }

    Some(substitute_params_typed(&def.body_typed, &expr_subst, &typed_subst))
}

fn is_atomic_inline_arg_expr(expr: &Expression) -> bool {
    matches!(expr, Expression::Word(_) | Expression::Int(_) | Expression::Dec(_))
}

fn can_no_temp_inline_arg(arg_expr: &Expression, arg_node: &TypedExpression, uses: usize) -> bool {
    if uses > 1 && !is_atomic_inline_arg_expr(arg_expr) {
        return false;
    }

    if !arg_node.effect.is_pure() {
        // Without a temp binding, substitution can drop or re-order evaluation.
        // Keep no-temp inlining only when the argument is a single atomic read.
        return is_atomic_inline_arg_expr(arg_expr) && uses == 1;
    }

    true
}

fn is_no_temp_inline_scalar_type(typ: &Type) -> bool {
    matches!(typ, Type::Int | Type::Dec | Type::Bool | Type::Char | Type::Unit)
}

fn substitute_params_expr(expr: &Expression, subst: &HashMap<String, Expression>) -> Expression {
    match expr {
        Expression::Word(w) =>
            subst
                .get(w)
                .cloned()
                .unwrap_or_else(|| Expression::Word(w.clone())),
        Expression::Apply(items) =>
            Expression::Apply(
                items
                    .iter()
                    .map(|it| substitute_params_expr(it, subst))
                    .collect()
            ),
        Expression::Int(n) => Expression::Int(*n),
        Expression::Dec(n) => Expression::Dec(*n),
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
        Expression::Apply(items) if items.len() == new_children.len() => {
            Expression::Apply(
                new_children
                    .iter()
                    .map(|ch| ch.expr.clone())
                    .collect()
            )
        }
        _ => substitute_params_expr(&node.expr, expr_subst),
    };
    TypedExpression {
        expr: new_expr,
        typ: node.typ.clone(),
        effect: node.effect,
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
        Some(Expression::Word(w)) if w == "let" || w == "letrec" => w.clone(),
        _ => {
            return None;
        }
    };
    let rhs_expr = items.get(2)?;
    let rhs_typed = node.children.get(2)?;
    let (prep, inlined_rhs_expr, inlined_rhs_typed) = try_inline_call(
        rhs_expr,
        rhs_typed,
        defs,
        state
    )?;

    let rewritten_expr = Expression::Apply(
        vec![Expression::Word(kw), items.get(1)?.clone(), inlined_rhs_expr]
    );
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
            _ => {
                return true;
            }
        }
    }
}
