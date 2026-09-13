use crate::infer::TypedExpression;
use crate::parser::Expression;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct GuardRelation {
    vector_arg: usize,
    projection_index_args: Vec<usize>,
    index_arg: usize,
}

type GuardSummary = Vec<GuardRelation>;

/// Abstract state at one program point.  This is deliberately independent of
/// the runtime representation: later analyses (division, overflow, and so on)
/// can add domains here without becoming part of WAT lowering.
#[derive(Clone, Default, PartialEq, Eq)]
struct AbstractState {
    safe_pairs: HashSet<(String, String)>,
    nonnegative: HashSet<String>,
    integer_constants: HashMap<String, i32>,
    fixed_lengths: HashMap<String, usize>,
    minimum_lengths: HashMap<String, usize>,
    length_sources: HashMap<String, String>,
    /// Value-numbering table for immutable aliases and projected vectors.
    /// The canonical expression is the symbolic identity used by proofs.
    aliases: HashMap<String, String>,
    /// Symbolic values for immutable scalar bindings. This lets a guard on an
    /// expression prove an access through a later `let` bound to that expression.
    scalar_aliases: HashMap<String, String>,
    guard_summaries: HashMap<String, GuardSummary>,
}

/// Conservative control-flow merge.  A fact is available after a join only
/// when it was true on every incoming edge.
fn join_states(left: &AbstractState, right: &AbstractState) -> AbstractState {
    let fixed_lengths = left
        .fixed_lengths
        .iter()
        .filter(|(name, len)| right.fixed_lengths.get(*name) == Some(*len))
        .map(|(name, len)| (name.clone(), *len))
        .collect();
    let integer_constants = left
        .integer_constants
        .iter()
        .filter(|(name, value)| right.integer_constants.get(*name) == Some(*value))
        .map(|(name, value)| (name.clone(), *value))
        .collect();
    let length_sources = left
        .length_sources
        .iter()
        .filter(|(name, source)| right.length_sources.get(*name) == Some(*source))
        .map(|(name, source)| (name.clone(), source.clone()))
        .collect();
    let minimum_lengths = left
        .minimum_lengths
        .iter()
        .filter_map(|(name, left_min)| {
            right
                .minimum_lengths
                .get(name)
                .map(|right_min| (name.clone(), (*left_min).min(*right_min)))
        })
        .collect();
    let aliases = left
        .aliases
        .iter()
        .filter(|(name, value)| right.aliases.get(*name) == Some(*value))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    let scalar_aliases = left
        .scalar_aliases
        .iter()
        .filter(|(name, value)| right.scalar_aliases.get(*name) == Some(*value))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    AbstractState {
        safe_pairs: left
            .safe_pairs
            .intersection(&right.safe_pairs)
            .cloned()
            .collect(),
        nonnegative: left
            .nonnegative
            .intersection(&right.nonnegative)
            .cloned()
            .collect(),
        integer_constants,
        fixed_lengths,
        minimum_lengths,
        length_sources,
        aliases,
        scalar_aliases,
        // Function summaries are immutable analysis metadata rather than a
        // path-sensitive fact.
        guard_summaries: left.guard_summaries.clone(),
    }
}

fn canonical_scalar(expr: &Expression, state: &AbstractState) -> String {
    match expr {
        Expression::Word(name) => state
            .scalar_aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.clone()),
        Expression::Apply(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|item| canonical_scalar(item, state))
                .collect();
            format!("({})", parts.join(" "))
        }
        _ => expr.to_lisp(),
    }
}

fn integer_constant(expr: &Expression, state: &AbstractState) -> Option<i32> {
    match expr {
        Expression::Int(value) => Some(*value),
        Expression::Word(name) => state.integer_constants.get(name).copied(),
        Expression::Apply(items) => match items.as_slice() {
            [Expression::Word(op), left, right] if op == "+" => {
                integer_constant(left, state)?.checked_add(integer_constant(right, state)?)
            }
            [Expression::Word(op), left, right] if op == "-" => {
                integer_constant(left, state)?.checked_sub(integer_constant(right, state)?)
            }
            [Expression::Word(op), left, right] if op == "*" => {
                integer_constant(left, state)?.checked_mul(integer_constant(right, state)?)
            }
            [Expression::Word(op), left, right] if op == "/" => {
                let divisor = integer_constant(right, state)?;
                (divisor != 0)
                    .then(|| integer_constant(left, state)?.checked_div(divisor))
                    .flatten()
            }
            _ => None,
        },
        Expression::Dec(_) => None,
    }
}

fn canonical_access(expr: &Expression, state: &AbstractState) -> String {
    match expr {
        Expression::Word(name) => state
            .aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.clone()),
        Expression::Apply(items)
            if matches!(items.first(), Some(Expression::Word(op)) if op == "get")
                && items.len() == 3 =>
        {
            format!(
                "(get {} {})",
                canonical_access(&items[1], state),
                items[2].to_lisp()
            )
        }
        _ => expr.to_lisp(),
    }
}

fn word(expr: &Expression) -> Option<&str> {
    match expr {
        Expression::Word(name) => Some(name),
        _ => None,
    }
}

fn literal_vector_length(expr: &Expression) -> Option<usize> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    match items.first().and_then(word) {
        Some("vector" | "string" | "integers" | "bools" | "decimals" | "strings") => {
            Some(items.len().saturating_sub(1))
        }
        _ => None,
    }
}

fn add_upper_bound(
    vector: &Expression,
    index: &Expression,
    facts: &AbstractState,
    lower: &mut HashSet<String>,
    upper: &mut Vec<(String, String)>,
    minimum_lengths: &mut Vec<(String, usize)>,
) {
    let index_key = canonical_scalar(index, facts);
    let constant_index = integer_constant(index, facts);
    if constant_index.is_some_and(|value| value >= 0)
        || matches!(index, Expression::Word(name) if facts.nonnegative.contains(name))
    {
        lower.insert(index_key.clone());
    }
    let vector_key = canonical_access(vector, facts);
    if let Some(index) = constant_index.filter(|value| *value >= 0) {
        minimum_lengths.push((vector_key.clone(), (index as usize).saturating_add(1)));
    }
    upper.push((vector_key, index_key));
}

fn collect_static_bound_guard_facts(
    expr: &Expression,
    facts: &AbstractState,
    is_true: bool,
    lower: &mut HashSet<String>,
    upper: &mut Vec<(String, String)>,
    minimum_lengths: &mut Vec<(String, usize)>,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    match items.as_slice() {
        [Expression::Word(op), operands @ ..]
            if operands.len() >= 2 && ((op == "and" && is_true) || (op == "or" && !is_true)) =>
        {
            for operand in operands {
                collect_static_bound_guard_facts(
                    operand,
                    facts,
                    is_true,
                    lower,
                    upper,
                    minimum_lengths,
                );
            }
        }
        [Expression::Word(op), inner] if op == "not" => {
            collect_static_bound_guard_facts(inner, facts, !is_true, lower, upper, minimum_lengths);
        }
        [Expression::Word(op), Expression::Word(index), Expression::Int(bound)]
            if (is_true && ((op == ">=" && *bound == 0) || (op == ">" && *bound == -1)))
                || (!is_true && ((op == "<" && *bound == 0) || (op == "<=" && *bound == -1))) =>
        {
            lower.insert(canonical_scalar(&Expression::Word(index.clone()), facts));
        }
        [Expression::Word(op), index, Expression::Apply(length)]
            if ((is_true && op == "<") || (!is_true && op == ">="))
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            add_upper_bound(&length[1], index, facts, lower, upper, minimum_lengths);
        }
        [Expression::Word(op), Expression::Apply(length), index]
            if ((is_true && op == ">") || (!is_true && op == "<="))
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            add_upper_bound(&length[1], index, facts, lower, upper, minimum_lengths);
        }
        [Expression::Word(op), index, Expression::Word(length)]
            if ((is_true && op == "<") || (!is_true && op == ">="))
                && facts.length_sources.contains_key(length) =>
        {
            let index_key = canonical_scalar(index, facts);
            if matches!(index, Expression::Int(value) if *value >= 0)
                || matches!(index, Expression::Word(name) if facts.nonnegative.contains(name))
            {
                lower.insert(index_key.clone());
            }
            upper.push((
                facts
                    .length_sources
                    .get(length)
                    .expect("checked cached length source")
                    .clone(),
                index_key,
            ));
            if let Expression::Int(index) = index {
                if *index >= 0 {
                    minimum_lengths.push((
                        facts
                            .length_sources
                            .get(length)
                            .expect("checked cached length source")
                            .clone(),
                        (*index as usize).saturating_add(1),
                    ));
                }
            }
        }
        [Expression::Word(op), Expression::Word(length), index]
            if ((is_true && op == ">") || (!is_true && op == "<="))
                && facts.length_sources.contains_key(length) =>
        {
            let index_key = canonical_scalar(index, facts);
            if matches!(index, Expression::Int(value) if *value >= 0)
                || matches!(index, Expression::Word(name) if facts.nonnegative.contains(name))
            {
                lower.insert(index_key.clone());
            }
            upper.push((
                facts
                    .length_sources
                    .get(length)
                    .expect("checked cached length source")
                    .clone(),
                index_key,
            ));
            if let Expression::Int(index) = index {
                if *index >= 0 {
                    minimum_lengths.push((
                        facts
                            .length_sources
                            .get(length)
                            .expect("checked cached length source")
                            .clone(),
                        (*index as usize).saturating_add(1),
                    ));
                }
            }
        }
        _ => {
            if is_true {
                let Some(op) = items.first().and_then(word) else {
                    return;
                };
                if let Some(relations) = facts.guard_summaries.get(op) {
                    for relation in relations {
                        let Some(mut vector) = items.get(relation.vector_arg + 1).cloned() else {
                            continue;
                        };
                        let Some(index) = items.get(relation.index_arg + 1) else {
                            continue;
                        };
                        for projection_arg in &relation.projection_index_args {
                            let Some(projection_index) = items.get(*projection_arg + 1) else {
                                continue;
                            };
                            vector = Expression::Apply(vec![
                                Expression::Word("get".to_string()),
                                vector,
                                projection_index.clone(),
                            ]);
                        }
                        let index_key = canonical_scalar(index, facts);
                        lower.insert(index_key.clone());
                        upper.push((canonical_access(&vector, facts), index_key));
                    }
                }
            }
        }
    }
    lower.extend(facts.nonnegative.iter().cloned());
}

fn state_for_true_branch(expr: &Expression, facts: &AbstractState) -> AbstractState {
    state_for_branch(expr, facts, true)
}

fn state_for_false_branch(expr: &Expression, facts: &AbstractState) -> AbstractState {
    state_for_branch(expr, facts, false)
}

fn state_for_branch(expr: &Expression, facts: &AbstractState, is_true: bool) -> AbstractState {
    let mut next = facts.clone();
    let mut lower = HashSet::new();
    let mut upper = Vec::new();
    let mut minimum_lengths = Vec::new();
    collect_static_bound_guard_facts(
        expr,
        facts,
        is_true,
        &mut lower,
        &mut upper,
        &mut minimum_lengths,
    );
    for (xs, index) in upper {
        if lower.contains(&index) {
            next.safe_pairs.insert((xs, index));
        }
    }
    for (vector, minimum) in minimum_lengths {
        next.minimum_lengths
            .entry(vector)
            .and_modify(|known| *known = (*known).max(minimum))
            .or_insert(minimum);
    }
    next
}

fn invalidate_resized_vector(items: &[Expression], facts: &mut AbstractState) {
    let Some(op) = items.first().and_then(word) else {
        return;
    };
    if matches!(op, "push!" | "pop!" | "pop-val!" | "set!") {
        if let Some(xs_expr) = items.get(1) {
            let xs = canonical_access(xs_expr, facts);
            facts
                .safe_pairs
                .retain(|(name, _)| name != &xs && !name.starts_with(&format!("(get {xs} ")));
            facts.fixed_lengths.remove(&xs);
            facts
                .minimum_lengths
                .retain(|name, _| name != &xs && !name.starts_with(&format!("(get {xs} ")));
            facts
                .length_sources
                .retain(|_, source| source != &xs && !source.starts_with(&format!("(get {xs} ")));
        }
    }
}

fn expression_is_nonnegative(expr: &Expression, facts: &AbstractState) -> bool {
    if let Some(value) = integer_constant(expr, facts) {
        return value >= 0;
    }
    match expr {
        Expression::Int(number) => *number >= 0,
        Expression::Word(name) => facts.nonnegative.contains(name),
        Expression::Apply(items) => match items.as_slice() {
            [Expression::Word(op), value] if op == "length" => {
                let _ = value;
                true
            }
            [Expression::Word(op), left, right] if op == "+" => {
                expression_is_nonnegative(left, facts) && expression_is_nonnegative(right, facts)
            }
            _ => false,
        },
        Expression::Dec(_) => false,
    }
}

fn assign_abstract_scalar(name: &str, value: &Expression, facts: &mut AbstractState) {
    let remains_nonnegative = expression_is_nonnegative(value, facts);
    let constant = integer_constant(value, facts);
    facts.safe_pairs.retain(|(_, index)| index != name);
    facts.nonnegative.remove(name);
    facts.integer_constants.remove(name);
    facts.fixed_lengths.remove(name);
    facts.minimum_lengths.remove(name);
    facts.length_sources.remove(name);
    facts.aliases.remove(name);
    facts.scalar_aliases.remove(name);

    if remains_nonnegative {
        facts.nonnegative.insert(name.to_string());
    }
    if let Some(constant) = constant {
        facts.integer_constants.insert(name.to_string(), constant);
    }
    if let Expression::Apply(length) = value {
        if matches!(length.first(), Some(Expression::Word(op)) if op == "length")
            && length.len() == 2
        {
            facts
                .length_sources
                .insert(name.to_string(), canonical_access(&length[1], facts));
            facts.nonnegative.insert(name.to_string());
        }
    }
}

fn record_diagnostic(diagnostics: &mut Vec<String>, message: String) {
    if !diagnostics.contains(&message) {
        diagnostics.push(message);
    }
}

fn validate_static_bounds_expr(
    expr: &Expression,
    facts: &mut AbstractState,
    diagnostics: &mut Vec<String>,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    let op = items.first().and_then(word).unwrap_or("");

    if op == "get" && items.len() > 3 {
        let mut accessed = items[1].clone();
        for index in items.iter().skip(2) {
            accessed = Expression::Apply(vec![
                Expression::Word("get".to_string()),
                accessed,
                index.clone(),
            ]);
            validate_static_bounds_expr(&accessed, facts, diagnostics);
        }
        return;
    }

    if op == "get" && items.len() == 3 {
        let proven = match (&items[1], &items[2]) {
            (xs, Expression::Word(index)) => facts.safe_pairs.contains(&(
                canonical_access(xs, facts),
                canonical_scalar(&Expression::Word(index.clone()), facts),
            )),
            (xs, Expression::Int(index)) => {
                facts
                    .safe_pairs
                    .contains(&(canonical_access(xs, facts), index.to_string()))
                    || facts
                        .minimum_lengths
                        .get(&canonical_access(xs, facts))
                        .is_some_and(|minimum| *index >= 0 && (*index as usize) < *minimum)
                    || facts
                        .fixed_lengths
                        .get(&canonical_access(xs, facts))
                        .copied()
                        .or_else(|| literal_vector_length(xs))
                        .is_some_and(|len| *index >= 0 && (*index as usize) < len)
            }
            _ => false,
        };
        if !proven {
            record_diagnostic(diagnostics, format!(
                "static bounds: cannot prove `{}` is within bounds for `{}`; guard the access with `(and (>= index 0) (< index (length xs)))`",
                items[2].to_lisp(),
                items[1].to_lisp()
            ));
        }
    }

    match op {
        "and" => {
            let mut scoped = facts.clone();
            for child in items.iter().skip(1) {
                validate_static_bounds_expr(child, &mut scoped, diagnostics);
                scoped = state_for_true_branch(child, &scoped);
            }
        }
        "do" => {
            let mut scoped = facts.clone();
            for child in items.iter().skip(1) {
                validate_static_bounds_expr(child, &mut scoped, diagnostics);
            }
            *facts = scoped;
        }
        "block" => {
            let mut scoped = facts.clone();
            for child in items.iter().skip(1) {
                validate_static_bounds_expr(child, &mut scoped, diagnostics);
            }
        }
        "lambda" => {
            // A lambda starts with no caller-local value facts, but inferred
            // function contracts are global immutable analysis metadata and
            // remain available inside nested functions.
            let mut scoped = AbstractState {
                guard_summaries: facts.guard_summaries.clone(),
                ..AbstractState::default()
            };
            for child in items.iter().skip(2) {
                validate_static_bounds_expr(child, &mut scoped, diagnostics);
            }
        }
        "let" | "mut" if items.len() >= 3 => {
            validate_static_bounds_expr(&items[2], facts, diagnostics);
            if let Expression::Word(name) = &items[1] {
                let scalar_alias = (op == "let").then(|| canonical_scalar(&items[2], facts));
                let alias = match &items[2] {
                    Expression::Word(_) => Some(canonical_access(&items[2], facts)),
                    Expression::Apply(rhs) if matches!(rhs.first(), Some(Expression::Word(op)) if op == "get") => {
                        Some(canonical_access(&items[2], facts))
                    }
                    _ => None,
                };
                assign_abstract_scalar(name, &items[2], facts);
                if let Some(len) = literal_vector_length(&items[2]) {
                    facts.fixed_lengths.insert(name.clone(), len);
                }
                if let Some(alias) = alias {
                    facts.aliases.insert(name.clone(), alias);
                }
                if let Some(scalar_alias) = scalar_alias {
                    facts.scalar_aliases.insert(name.clone(), scalar_alias);
                }
            }
        }
        "alter!" if items.len() == 3 => {
            validate_static_bounds_expr(&items[2], facts, diagnostics);
            if let Expression::Word(name) = &items[1] {
                assign_abstract_scalar(name, &items[2], facts);
            }
        }
        "if" if items.len() >= 3 => {
            validate_static_bounds_expr(&items[1], facts, diagnostics);
            let mut consequent = state_for_true_branch(&items[1], facts);
            validate_static_bounds_expr(&items[2], &mut consequent, diagnostics);
            let mut alternate = state_for_false_branch(&items[1], facts);
            if let Some(otherwise) = items.get(3) {
                validate_static_bounds_expr(otherwise, &mut alternate, diagnostics);
            }
            *facts = join_states(&consequent, &alternate);
        }
        "while" if items.len() >= 3 => {
            // Compute a loop-header fixed point.  The intersection join is the
            // widening for this finite fact domain, so convergence is quick;
            // the cap is only a defensive guard against future domains.
            let entry = facts.clone();
            let mut header = entry.clone();
            for _ in 0..16 {
                validate_static_bounds_expr(&items[1], &mut header, diagnostics);
                let mut body_exit = state_for_true_branch(&items[1], &header);
                for child in items.iter().skip(2) {
                    validate_static_bounds_expr(child, &mut body_exit, diagnostics);
                }
                let next = join_states(&entry, &body_exit);
                if next == header {
                    break;
                }
                header = next;
            }
            // The loop may execute zero times; only header facts are valid on
            // exit.  False-condition refinement can be added as another
            // abstract domain without changing traversal.
            *facts = header;
        }
        "loop" if items.len() >= 4 => {
            let mut scoped = facts.clone();
            if let Expression::Word(index) = &items[1] {
                scoped.nonnegative.insert(index.clone());
            }
            let guarded = state_for_true_branch(&items[2], &scoped);
            scoped = guarded;
            for child in items.iter().skip(3) {
                validate_static_bounds_expr(child, &mut scoped, diagnostics);
            }
        }
        "loop/range" if items.len() >= 5 => {
            let mut scoped = facts.clone();
            if let Expression::Word(index) = &items[1] {
                if matches!(&items[2], Expression::Int(start) if *start >= 0) {
                    scoped.nonnegative.insert(index.clone());
                }
                if let Expression::Apply(end) = &items[3] {
                    if matches!(end.as_slice(), [Expression::Word(len), Expression::Word(_)] if len == "length")
                    {
                        if let Expression::Word(xs) = &end[1] {
                            scoped.safe_pairs.insert((xs.clone(), index.clone()));
                        }
                    }
                }
            }
            for child in items.iter().skip(4) {
                validate_static_bounds_expr(child, &mut scoped, diagnostics);
            }
        }
        _ => {
            for child in items.iter().skip(1) {
                validate_static_bounds_expr(child, facts, diagnostics);
            }
        }
    }
    invalidate_resized_vector(items, facts);
}

pub fn analyze_user_program(
    typed_program: &TypedExpression,
    user_form_count: usize,
) -> Result<(), String> {
    match analyze_user_program_diagnostics(typed_program, user_form_count)
        .into_iter()
        .next()
    {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

pub fn analyze_user_program_diagnostics(
    typed_program: &TypedExpression,
    user_form_count: usize,
) -> Vec<String> {
    let all_expressions: Vec<&Expression> = match &typed_program.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "do") => {
            items.iter().skip(1).collect()
        }
        expression => vec![expression],
    };
    let guard_summaries = infer_guard_summaries(&all_expressions);
    let start = all_expressions.len().saturating_sub(user_form_count);
    let mut facts = AbstractState {
        guard_summaries,
        ..AbstractState::default()
    };
    let mut diagnostics = Vec::new();
    for expression in &all_expressions[start..] {
        validate_static_bounds_expr(expression, &mut facts, &mut diagnostics);
    }
    diagnostics
}

fn infer_guard_summaries(expressions: &[&Expression]) -> HashMap<String, GuardSummary> {
    let mut summaries: HashMap<String, GuardSummary> = HashMap::new();
    for _ in 0..expressions.len().max(1) {
        let mut changed = false;
        for expression in expressions {
            let Expression::Apply(binding) = expression else {
                continue;
            };
            let [Expression::Word(keyword), Expression::Word(name), rhs] = binding.as_slice()
            else {
                continue;
            };
            if keyword != "let" && keyword != "letrec" {
                continue;
            }
            if let Expression::Word(alias) = rhs {
                if let Some(summary) = summaries.get(alias).cloned() {
                    changed |= summaries.insert(name.clone(), summary.clone()) != Some(summary);
                }
                continue;
            }
            let Expression::Apply(lambda) = rhs else {
                continue;
            };
            if !matches!(lambda.first(), Some(Expression::Word(op)) if op == "lambda")
                || lambda.len() < 3
            {
                continue;
            }
            let params: Vec<&str> = lambda[1..lambda.len() - 1]
                .iter()
                .filter_map(word)
                .collect();
            let mut facts = AbstractState {
                guard_summaries: summaries.clone(),
                ..AbstractState::default()
            };
            facts = state_for_true_branch(lambda.last().expect("lambda has a body"), &facts);
            let mut summary = Vec::new();
            for (vector, index) in &facts.safe_pairs {
                let Some(index_arg) = params.iter().position(|param| *param == index) else {
                    continue;
                };
                let Ok(vector_expr) = crate::parser::build(vector) else {
                    continue;
                };
                if let Some((vector_arg, projection_index_args)) =
                    guard_path_from_params(single_built_expression(&vector_expr), &params)
                {
                    summary.push(GuardRelation {
                        vector_arg,
                        projection_index_args,
                        index_arg,
                    });
                }
            }
            summary.sort_by_key(|relation| {
                (
                    relation.projection_index_args.len(),
                    relation.vector_arg,
                    relation.index_arg,
                )
            });
            summary.dedup();
            if !summary.is_empty() {
                changed |= summaries.insert(name.clone(), summary.clone()) != Some(summary);
            }
        }
        if !changed {
            break;
        }
    }
    summaries
}

fn single_built_expression(expr: &Expression) -> &Expression {
    if let Expression::Apply(items) = expr {
        if matches!(items.first(), Some(Expression::Word(op)) if op == "do") && items.len() == 2 {
            return &items[1];
        }
    }
    expr
}

fn guard_path_from_params(expr: &Expression, params: &[&str]) -> Option<(usize, Vec<usize>)> {
    match expr {
        Expression::Word(name) => params
            .iter()
            .position(|param| *param == name)
            .map(|root| (root, Vec::new())),
        Expression::Apply(items)
            if matches!(items.first(), Some(Expression::Word(op)) if op == "get")
                && items.len() == 3 =>
        {
            let (root, mut projections) = guard_path_from_params(&items[1], params)?;
            let Expression::Word(index) = &items[2] else {
                return None;
            };
            projections.push(params.iter().position(|param| *param == index)?);
            Some((root, projections))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(source: &str, user_form_count: usize) -> Result<(), String> {
        let expression = crate::parser::build(source)?;
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )?;
        analyze_user_program(&typed, user_form_count)
    }

    #[test]
    fn inferred_user_predicate_can_prove_a_later_access() {
        let source = "(let valid? (lambda (xs i) (and (= 1 1) (>= i 0) (< i (length xs))))) (let xs [1 2]) (let i 1) (if (valid? xs i) (get xs i) 0)";
        assert_eq!(analyze(source, 4), Ok(()));
    }

    #[test]
    fn nested_matrix_access_requires_both_path_proofs() {
        let guarded = "(let xs [[1] [2]]) (let x 0) (let y 0) (if (and (>= x 0) (< x (length xs)) (>= y 0) (< y (length (get xs x)))) (get xs x y) -1)";
        assert!(analyze(guarded, 4).is_ok());

        let unguarded = "(let xs [[1] [2]]) (let x 10) (let y 10) (get xs x y)";
        assert!(analyze(unguarded, 4)
            .expect_err("unproven access should fail")
            .contains("static bounds: cannot prove"));
    }

    #[test]
    fn cached_length_proves_loop_access_after_macro_expansion() {
        let source = "(let xs [1 2 3 4]) (let len (length xs)) (mut i 0) (while (< i len) (do (let x (get xs i)) (alter! i (+ i 1))))";
        assert_eq!(analyze(source, 4), Ok(()));
    }

    #[test]
    fn cached_nested_lengths_prove_matrix_accesses() {
        let source = "(let rows [[1] [2]]) (let height (length rows)) (mut y 0) (while (< y height) (do (let row (get rows y)) (let width (length row)) (mut x 0) (while (< x width) (do (let item (get row x)) (alter! x (+ x 1)))) (alter! y (+ y 1))))";
        assert_eq!(analyze(source, 4), Ok(()));
    }

    #[test]
    fn branch_refinement_does_not_escape_the_branch() {
        let source = "(let xs [1 2]) (let i 1) (if (and (>= i 0) (< i (length xs))) (get xs i) 0) (get xs i)";
        assert!(analyze(source, 4)
            .expect_err("a path-local proof must not escape its branch")
            .contains("static bounds: cannot prove"));
    }

    #[test]
    fn assignment_kills_old_index_refinement() {
        let source = "(let xs [1 2]) (mut i 1) (if (and (>= i 0) (< i (length xs))) (do (alter! i -1) (get xs i)) 0)";
        assert!(analyze(source, 3)
            .expect_err("assigning the index must kill its prior proof")
            .contains("static bounds: cannot prove"));
    }

    #[test]
    fn resize_kills_a_cached_length_relation() {
        let source = "(let xs [1 2]) (let len (length xs)) (pop! xs) (mut i 0) (while (< i len) (do (let x (get xs i)) (alter! i (+ i 1))))";
        assert!(analyze(source, 5)
            .expect_err("resizing a vector must invalidate its cached length")
            .contains("static bounds: cannot prove"));
    }

    #[test]
    fn vector_aliases_share_a_symbolic_bounds_identity() {
        let source = "(let xs [1 2]) (let ys xs) (let i 1) (if (and (>= i 0) (< i (length ys))) (get xs i) 0)";
        assert_eq!(analyze(source, 4), Ok(()));
    }

    #[test]
    fn resizing_through_an_alias_kills_original_vector_proofs() {
        let source = "(let xs [1 2]) (let ys xs) (let len (length xs)) (pop! ys) (mut i 0) (while (< i len) (do (let x (get xs i)) (alter! i (+ i 1))))";
        assert!(analyze(source, 6)
            .expect_err("an alias resize must invalidate the shared vector identity")
            .contains("static bounds: cannot prove"));
    }

    #[test]
    fn false_or_and_negation_refine_the_else_branch() {
        let source = "(let in-bounds? (lambda (xs i) (and (>= i 0) (< i (length xs))))) (let xs [1 2]) (let i 1) (if (or (> i 10) (not (in-bounds? xs i))) 0 (get xs i))";
        assert_eq!(analyze(source, 4), Ok(()));
    }

    #[test]
    fn diagnostic_mode_collects_all_unproven_accesses() {
        let source = "(let xs [1]) (let ys [2]) (let i 0) (let j 0) (get xs i) (get ys j)";
        let expression = crate::parser::build(source).expect("source should parse");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        let diagnostics = analyze_user_program_diagnostics(&typed, 6);
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    }

    #[test]
    fn positive_dynamic_length_proves_literal_zero_access() {
        let source = "(let xs []) (if (> (length xs) 0) (get xs 0) -1)";
        assert_eq!(analyze(source, 2), Ok(()));

        let equivalent = "(let xs []) (if (< 0 (length xs)) (get xs 0) -1)";
        assert_eq!(analyze(equivalent, 2), Ok(()));

        let stronger = "(let xs []) (if (> (length xs) 1) (get xs 0) -1)";
        assert_eq!(analyze(stronger, 2), Ok(()));

        let constant_alias = "(let xs []) (let index 0) (if (> (length xs) index) (get xs 0) -1)";
        assert_eq!(analyze(constant_alias, 3), Ok(()));
    }

    #[test]
    fn guarded_scalar_expression_proves_its_later_let_alias() {
        let source = "(let xs [1 2 3]) (let left 0) (let right 2) (if (in-bounds? xs (/ (+ left right) 2)) (block (let index (/ (+ left right) 2)) (get xs index)) -1)";
        // Supply the predicate definition because this unit helper intentionally
        // infers without merging the baked standard library.
        let source =
            format!("(let in-bounds? (lambda (xs i) (and (>= i 0) (< i (length xs))))) {source}");
        assert_eq!(analyze(&source, 5), Ok(()));
    }

    #[test]
    fn inferred_wrapper_contract_substitutes_reordered_arguments() {
        let source = "(let base? (lambda (xs i) (and (>= i 0) (< i (length xs))))) (let flipped? (lambda (i xs) (base? xs i))) (let xs [1 2]) (let i 1) (if (flipped? i xs) (get xs i) -1)";
        assert_eq!(analyze(source, 5), Ok(()));
    }

    #[test]
    fn inferred_contract_is_available_inside_nested_lambda() {
        let source = "(let valid? (lambda (xs i) (and (>= i 0) (< i (length xs))))) (let search (lambda (xs) (let index 0) (if (valid? xs index) (get xs index) -1)))";
        assert_eq!(analyze(source, 2), Ok(()));
    }

    #[test]
    fn false_out_of_bounds_comparisons_refine_else_branch() {
        let source = "(let xs [1 2]) (let index 1) (if (or false (< index 0) (>= index (length xs))) -1 (get xs index))";
        assert_eq!(analyze(source, 3), Ok(()));
    }
}
