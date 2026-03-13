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
        float: bool,
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
            return dead_code_eliminate_top_level_defs(&next);
        }
        cur = next;
    }
    dead_code_eliminate_top_level_defs(&cur)
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
    let mut top_def_names: HashSet<String> = HashSet::new();
    let mut roots: Vec<Expression> = Vec::new();

    for item in norm_items.iter().skip(1) {
        if let Some((name, rhs)) = top_level_let_def(item) {
            defs_rhs.insert(name.clone(), rhs.clone());
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
    do_node
        .children
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
    if kw == "let" || kw == "let*" {
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
                if op == "let" || op == "let*" {
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
    let prepared = expr.clone();
    let mut name_state = FuseNameState::default();
    match &prepared {
        Expression::Apply(items) if
            matches!(items.first(), Some(Expression::Word(w)) if w == "do") &&
            items.len() > 1
        => {
            let mut out = items.clone();
            let last = out.len() - 1;
            out[last] = fuse_map_filter_reduce_chains_expr(&out[last], &mut name_state);
            Expression::Apply(out)
        }
        _ => fuse_map_filter_reduce_chains_expr(&prepared, &mut name_state),
    }
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
        // sum/float xs => reduce +. 0. xs
        "sum/float" if items.len() == 2 =>
            Some((
                FuseSink::Reduce {
                    reduce_fn: Expression::Word("+.".to_string()),
                    init_expr: Expression::Float(0.0),
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
        "product/float" if items.len() == 2 =>
            Some((
                FuseSink::Reduce {
                    reduce_fn: Expression::Word("*.".to_string()),
                    init_expr: Expression::Float(1.0),
                    with_index: false,
                },
                items.get(1)?.clone(),
            )),
        // mean aliases over vectors
        "mean" | "mean/int" if items.len() == 2 =>
            Some((FuseSink::Average { float: false }, items.get(1)?.clone())),
        "mean/float" if items.len() == 2 =>
            Some((FuseSink::Average { float: true }, items.get(1)?.clone())),
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
            matches!(items.first(), Some(Expression::Word(w)) if w == "range/float")
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
        let has_indexed_stage = ops_outer_to_inner
            .iter()
            .any(|op| {
                matches!(
                    op,
                    MapFilterOp::Map { with_index: true, .. } |
                        MapFilterOp::Filter { with_index: true, .. }
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
    match sink {
        FuseSink::Some { predicate, with_index } =>
            build_some_every_loop(source, ops_outer_to_inner, predicate, with_index, true, &suffix),
        FuseSink::Every { predicate, with_index } =>
            build_some_every_loop(
                source,
                ops_outer_to_inner,
                predicate,
                with_index,
                false,
                &suffix
            ),
        FuseSink::Collect => build_collect_loop(source, ops_outer_to_inner, &suffix),
        FuseSink::Reduce { reduce_fn, init_expr, with_index } =>
            build_reduce_loop(
                source,
                ops_outer_to_inner,
                reduce_fn,
                init_expr,
                with_index,
                &suffix
            ),
        FuseSink::ReduceUntil { reduce_fn, stop_fn, init_expr, with_index } =>
            build_reduce_until_loop(
                source,
                ops_outer_to_inner,
                reduce_fn,
                stop_fn,
                init_expr,
                with_index,
                &suffix
            ),
        FuseSink::Average { float } =>
            build_average_loop(source, ops_outer_to_inner, float, &suffix),
        FuseSink::Unzip => build_unzip_loop(source, ops_outer_to_inner, &suffix),
        FuseSink::Find { predicate } =>
            build_find_loop(source, ops_outer_to_inner, predicate, &suffix),
    }
}

fn fuse_tmp_name(base: &str, suffix: &str) -> String {
    if suffix.is_empty() { base.to_string() } else { format!("{}{}", base, suffix) }
}

fn build_while_range_call(
    start_expr: Expression,
    end_expr: Expression,
    i_name: &str,
    process_name: &str
) -> Expression {
    let i_word = Expression::Word(i_name.to_string());
    let end_name = format!("{}_end", i_name);
    let end_word = Expression::Word(end_name.clone());
    let process_call = Expression::Apply(
        vec![Expression::Word(process_name.to_string()), i_word.clone()]
    );
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
            process_call,
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
where
    F: Fn(Expression, Expression) -> Option<Expression>,
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
where
    F: Fn(Expression, Expression) -> Option<Expression>,
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
                    vec![Expression::Word("if".to_string()), pass_cond, then_expr, no_op_unit_expr()]
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
    let process_name = fuse_tmp_name("__fuse_process", suffix);
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
    let process_lambda = Expression::Apply(
        vec![Expression::Word("lambda".to_string()), Expression::Word(i_name.clone()), process_body]
    );

    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(out_name.clone()),
                Expression::Apply(vec![Expression::Word("vector".to_string())])
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(process_name.clone()),
                process_lambda
            ]
        )
    );
    setup_bindings.push(build_while_range_call(start_expr, end_expr, &i_name, &process_name));
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
    let process_name = fuse_tmp_name("__fuse_process", suffix);
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
                call_callable_expr(
                    &reduce_fn_for_sink,
                    vec![acc_get.clone(), mapped, logical_i]
                )?
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
    let process_lambda = Expression::Apply(
        vec![Expression::Word("lambda".to_string()), Expression::Word(i_name.clone()), process_body]
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
                Expression::Word(process_name.clone()),
                process_lambda
            ]
        )
    );
    setup_bindings.push(build_while_range_call(start_expr, end_expr, &i_name, &process_name));
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
    let process_name = fuse_tmp_name("__fuse_process", suffix);

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
        &move |mapped: Expression, logical_i: Expression| {
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
                    vec![
                        Expression::Word("if".to_string()),
                        stop_value,
                        set_placed_true,
                        set_out
                    ]
                )
            )
        }
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
    let process_lambda = Expression::Apply(
        vec![
            Expression::Word("lambda".to_string()),
            Expression::Apply(vec![Expression::Word("do".to_string()), step_action, idx_inc])
        ]
    );

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
                Expression::Word("let".to_string()),
                Expression::Word(process_name.clone()),
                process_lambda
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("while".to_string()),
                continue_cond,
                Expression::Apply(vec![Expression::Word(process_name)])
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
    float: bool,
    suffix: &str
) -> Option<Expression> {
    let (mut setup_bindings, start_expr, end_expr, value_expr_for_i) = make_loop_source_bindings(
        source,
        suffix
    )?;

    let sum_name = fuse_tmp_name("__fuse_sum", suffix);
    let count_name = fuse_tmp_name("__fuse_count", suffix);
    let process_name = fuse_tmp_name("__fuse_process", suffix);
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
            vec![Expression::Word((if float { "+." } else { "+" }).to_string()), sum_get, mapped]
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
    let process_lambda = Expression::Apply(
        vec![Expression::Word("lambda".to_string()), Expression::Word(i_name.clone()), process_body]
    );

    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(sum_name.clone()),
                Expression::Apply(
                    vec![Expression::Word("vector".to_string()), if float {
                        Expression::Float(0.0)
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
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(process_name.clone()),
                process_lambda
            ]
        )
    );
    setup_bindings.push(build_while_range_call(start_expr, end_expr, &i_name, &process_name));

    let count_get = Expression::Apply(
        vec![Expression::Word("get".to_string()), Expression::Word(count_name), Expression::Int(0)]
    );
    let sum_get = Expression::Apply(
        vec![Expression::Word("get".to_string()), Expression::Word(sum_name), Expression::Int(0)]
    );
    let mean_expr = if float {
        Expression::Apply(
            vec![
                Expression::Word("/.".to_string()),
                sum_get,
                Expression::Apply(vec![Expression::Word("Int->Float".to_string()), count_get])
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
    let process_name = fuse_tmp_name("__fuse_process", suffix);
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
    let process_lambda = Expression::Apply(
        vec![Expression::Word("lambda".to_string()), Expression::Word(i_name.clone()), process_body]
    );

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
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                Expression::Word(process_name.clone()),
                process_lambda
            ]
        )
    );
    setup_bindings.push(build_while_range_call(start_expr, end_expr, &i_name, &process_name));
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
    let process_name = fuse_tmp_name("__fuse_process", suffix);
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
        &move |mapped: Expression, logical_i: Expression| {
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
        }
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
    let process_lambda = Expression::Apply(
        vec![
            Expression::Word("lambda".to_string()),
            Expression::Apply(vec![Expression::Word("do".to_string()), step_action, idx_inc])
        ]
    );

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
                Expression::Word("let".to_string()),
                Expression::Word(process_name.clone()),
                process_lambda
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("while".to_string()),
                continue_cond,
                Expression::Apply(vec![Expression::Word(process_name)])
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
    let process_name = fuse_tmp_name("__fuse_process", suffix);

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
        &move |mapped: Expression, logical_i: Expression| {
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
                    vec![Expression::Word("if".to_string()), pred_value, set_found, no_op_unit_expr()]
                )
            )
        }
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
    let process_lambda = Expression::Apply(
        vec![
            Expression::Word("lambda".to_string()),
            Expression::Apply(vec![Expression::Word("do".to_string()), guarded_step, idx_inc])
        ]
    );

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
                Expression::Word("let".to_string()),
                Expression::Word(process_name.clone()),
                process_lambda
            ]
        )
    );
    setup_bindings.push(
        Expression::Apply(
            vec![
                Expression::Word("while".to_string()),
                continue_cond,
                Expression::Apply(vec![Expression::Word(process_name)])
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
                        Expression::Apply(vec![Expression::Word("length".to_string()), left_word.clone()])
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
                            vec![Expression::Word("get".to_string()), left_word.clone(), i_expr.clone()]
                        ),
                        Expression::Apply(
                            vec![Expression::Word("get".to_string()), right_word.clone(), i_expr.clone()]
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
                    Expression::Apply(vec![Expression::Word("-".to_string()), to_word, from_word.clone()]),
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
                    Expression::Apply(vec![Expression::Word("-".to_string()), to_word, from_word.clone()]),
                    Expression::Int(1)
                ]
            );
            let value = Box::new(move |i_expr: &Expression| {
                Expression::Apply(
                    vec![
                        Expression::Word("Int->Float".to_string()),
                        Expression::Apply(
                            vec![Expression::Word("+".to_string()), from_word.clone(), i_expr.clone()]
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
                            vec![Expression::Word("+".to_string()), from_word.clone(), i_expr.clone()]
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
                        Expression::Apply(vec![Expression::Word("length".to_string()), left_word.clone()])
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
                            vec![Expression::Word("get".to_string()), left_word.clone(), i_expr.clone()]
                        ),
                        Expression::Apply(
                            vec![Expression::Word("get".to_string()), right_word.clone(), i_expr.clone()]
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
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(from_name),
                        start
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
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(vec![Expression::Word("vector".to_string()), Expression::Int(0)])
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
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(from_name),
                        start
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
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(vec![Expression::Word("vector".to_string()), Expression::Int(0)])
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
                        Expression::Word("Int->Float".to_string()),
                        Expression::Apply(
                            vec![Expression::Word("+".to_string()), from_word.clone(), i_expr.clone()]
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
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(from_name),
                        start
                    ]
                )
            );
            setup.push(
                Expression::Apply(
                    vec![
                        Expression::Word("let".to_string()),
                        Expression::Word(idx_ref_name.clone()),
                        Expression::Apply(vec![Expression::Word("vector".to_string()), Expression::Int(0)])
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
            let proc_name = next_flatten_tmp_name("__fuse_flat_process", suffix, flat_tmp_counter);
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
                        Expression::Apply(
                            vec![
                                Expression::Word("let".to_string()),
                                Expression::Word(proc_name.clone()),
                                Expression::Apply(
                                    vec![
                                        Expression::Word("lambda".to_string()),
                                        Expression::Word(i_name.clone()),
                                        process_body
                                    ]
                                )
                            ]
                        ),
                        build_while_range_call(
                            Expression::Int(0),
                            Expression::Apply(
                                vec![
                                    Expression::Word("length".to_string()),
                                    Expression::Word(xs_name)
                                ]
                            ),
                            &i_name,
                            &proc_name
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
            let proc_name = next_flatten_tmp_name("__fuse_flat_process", suffix, flat_tmp_counter);
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
                        Expression::Apply(
                            vec![
                                Expression::Word("let".to_string()),
                                Expression::Word(proc_name.clone()),
                                Expression::Apply(
                                    vec![
                                        Expression::Word("lambda".to_string()),
                                        Expression::Word(i_name.clone()),
                                        process_body
                                    ]
                                )
                            ]
                        ),
                        build_while_range_call(
                            Expression::Int(0),
                            Expression::Apply(
                                vec![
                                    Expression::Word("length".to_string()),
                                    Expression::Word(xs_name)
                                ]
                            ),
                            &i_name,
                            &proc_name
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
        FuseSink::Unzip => true,
        FuseSink::Some { predicate, .. } | FuseSink::Every { predicate, .. } =>
            is_fusion_safe_callable(predicate),
        FuseSink::Find { predicate } => is_fusion_safe_callable(predicate),
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
        Expression::Apply(items) =>
            matches!(items.first(), Some(Expression::Word(w)) if w == "lambda"),
        _ => false,
    }
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
        Expression::Apply(items) if items.len() == children.len() =>
            Expression::Apply(
                children
                    .iter()
                    .map(|ch| ch.expr.clone())
                    .collect()
            ),
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

        "Int->Float" => fold_int_to_float(node, &items),
        "Float->Int" => fold_float_to_int(node, &items),

        _ => node,
    }
}

fn fuse_apply_wrapper_call(
    node: &TypedExpression,
    call_items: &[Expression]
) -> Option<TypedExpression> {
    let op = match call_items.first() {
        Some(Expression::Word(w)) => w.as_str(),
        _ => {
            return None;
        }
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
            effect: node.effect,
            children: new_children,
        };
        return beta_reduce_immediate_lambda_call(&rewritten, match &rewritten.expr {
            Expression::Apply(items) => items,
            _ => {
                return None;
            }
        });
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
    new_items.extend(
        call_items
            .iter()
            .skip(1)
            .take(call_items.len() - 2)
            .cloned()
    );

    let mut new_children = vec![node.children.last()?.clone()];
    new_children.extend(
        node.children
            .iter()
            .skip(1)
            .take(node.children.len() - 2)
            .cloned()
    );
    let rewritten = TypedExpression {
        expr: Expression::Apply(new_items),
        typ: node.typ.clone(),
        effect: node.effect,
        children: new_children,
    };
    beta_reduce_immediate_lambda_call(&rewritten, match &rewritten.expr {
        Expression::Apply(items) => items,
        _ => {
            return None;
        }
    })
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
        let Some(child) = inlined_children.get(i) else {
            kept_indices.push(i);
            continue;
        };
        if !is_elidable_do_statement_expr(&inlined_items[i], child) {
            kept_indices.push(i);
        }
    }
    kept_indices.push(last_idx);

    if kept_indices.len() == inlined_items.len() {
        return TypedExpression {
            expr: Expression::Apply(inlined_items),
            typ: normalized_do.typ,
            effect: normalized_do.effect,
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
        effect: normalized_do.effect,
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
    if parse_env_bool_like("QUE_DIV_ZERO_CHECK", false) && (op == "/." || op == "mod.") && b == 0.0
    {
        return node;
    }
    let result = f(a, b);
    if parse_env_bool_like("QUE_FLOAT_OVERFLOW_CHECK", false) && !result.is_finite() {
        return node;
    }
    make_folded_literal(&node, Expression::Float(result), Type::Float)
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
            vec![
                Expression::Word("do".to_string()),
                first.expr.clone(),
                second.expr.clone()
            ]
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
            second,
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
    if !matches!(
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
            "Int->Float" |
            "Float->Int"
    ) {
        return false;
    }

    items.iter().skip(1).all(|arg| {
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
            Expression::Apply(
                new_children
                    .iter()
                    .map(|ch| ch.expr.clone())
                    .collect()
            ),
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
    matches!(expr, Expression::Word(_) | Expression::Int(_) | Expression::Float(_))
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
    matches!(typ, Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit)
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
            Expression::Apply(
                new_children
                    .iter()
                    .map(|ch| ch.expr.clone())
                    .collect()
            ),
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
        Some(Expression::Word(w)) if w == "let" || w == "let*" => w.clone(),
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
