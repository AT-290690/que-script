use crate::infer::TypedExpression;
use crate::parser::Expression;
use std::collections::{HashMap, HashSet};

/// Abstract state at one program point.  This is deliberately independent of
/// the runtime representation: later analyses (division, overflow, and so on)
/// can add domains here without becoming part of WAT lowering.
#[derive(Clone, Default, PartialEq, Eq)]
struct AbstractState {
    safe_pairs: HashSet<(String, String)>,
    nonnegative: HashSet<String>,
    fixed_lengths: HashMap<String, usize>,
    length_sources: HashMap<String, String>,
    /// Value-numbering table for immutable aliases and projected vectors.
    /// The canonical expression is the symbolic identity used by proofs.
    aliases: HashMap<String, String>,
    guard_summaries: HashMap<String, (usize, usize)>,
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
    let length_sources = left
        .length_sources
        .iter()
        .filter(|(name, source)| right.length_sources.get(*name) == Some(*source))
        .map(|(name, source)| (name.clone(), source.clone()))
        .collect();
    let aliases = left
        .aliases
        .iter()
        .filter(|(name, value)| right.aliases.get(*name) == Some(*value))
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
        fixed_lengths,
        length_sources,
        aliases,
        // Function summaries are immutable analysis metadata rather than a
        // path-sensitive fact.
        guard_summaries: left.guard_summaries.clone(),
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

fn collect_static_bound_guard_facts(
    expr: &Expression,
    facts: &AbstractState,
    is_true: bool,
    lower: &mut HashSet<String>,
    upper: &mut Vec<(String, String)>,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    match items.as_slice() {
        [Expression::Word(op), left, right]
            if (op == "and" && is_true) || (op == "or" && !is_true) =>
        {
            collect_static_bound_guard_facts(left, facts, is_true, lower, upper);
            collect_static_bound_guard_facts(right, facts, is_true, lower, upper);
        }
        [Expression::Word(op), inner] if op == "not" => {
            collect_static_bound_guard_facts(inner, facts, !is_true, lower, upper);
        }
        [Expression::Word(op), Expression::Word(index), Expression::Int(bound)]
            if is_true && ((op == ">=" && *bound == 0) || (op == ">" && *bound == -1)) =>
        {
            lower.insert(index.clone());
        }
        [Expression::Word(op), Expression::Word(index), Expression::Apply(length)]
            if is_true
                && op == "<"
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            upper.push((canonical_access(&length[1], facts), index.clone()));
        }
        [Expression::Word(op), Expression::Word(index), Expression::Word(length)]
            if is_true && op == "<" && facts.length_sources.contains_key(length) =>
        {
            upper.push((
                facts
                    .length_sources
                    .get(length)
                    .expect("checked cached length source")
                    .clone(),
                index.clone(),
            ));
        }
        [Expression::Word(op), xs, Expression::Word(index)]
            if is_true && matches!(op.as_str(), "in-bounds?" | "std/vector/in-bounds?") =>
        {
            lower.insert(index.clone());
            upper.push((canonical_access(xs, facts), index.clone()));
        }
        [Expression::Word(op), Expression::Word(index), Expression::Word(xs)]
            if is_true && op == "Vector/in-bounds?" =>
        {
            lower.insert(index.clone());
            upper.push((
                canonical_access(&Expression::Word(xs.clone()), facts),
                index.clone(),
            ));
        }
        _ => {
            if is_true {
                let Some(op) = items.first().and_then(word) else {
                    return;
                };
                if let Some((vector_arg, index_arg)) = facts.guard_summaries.get(op) {
                    if let (Some(vector), Some(Expression::Word(index))) =
                        (items.get(*vector_arg + 1), items.get(*index_arg + 1))
                    {
                        lower.insert(index.clone());
                        upper.push((canonical_access(vector, facts), index.clone()));
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
    collect_static_bound_guard_facts(expr, facts, is_true, &mut lower, &mut upper);
    for (xs, index) in upper {
        if lower.contains(&index) {
            next.safe_pairs.insert((xs, index));
        }
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
                .length_sources
                .retain(|_, source| source != &xs && !source.starts_with(&format!("(get {xs} ")));
        }
    }
}

fn expression_is_nonnegative(expr: &Expression, facts: &AbstractState) -> bool {
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
    facts.safe_pairs.retain(|(_, index)| index != name);
    facts.nonnegative.remove(name);
    facts.fixed_lengths.remove(name);
    facts.length_sources.remove(name);
    facts.aliases.remove(name);

    if remains_nonnegative {
        facts.nonnegative.insert(name.to_string());
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

fn validate_static_bounds_expr(expr: &Expression, facts: &mut AbstractState) -> Result<(), String> {
    let Expression::Apply(items) = expr else {
        return Ok(());
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
            validate_static_bounds_expr(&accessed, facts)?;
        }
        return Ok(());
    }

    if op == "get" && items.len() == 3 {
        let proven = match (&items[1], &items[2]) {
            (xs, Expression::Word(index)) => facts
                .safe_pairs
                .contains(&(canonical_access(xs, facts), index.clone())),
            (xs, Expression::Int(index)) => facts
                .fixed_lengths
                .get(&canonical_access(xs, facts))
                .copied()
                .or_else(|| literal_vector_length(xs))
                .is_some_and(|len| *index >= 0 && (*index as usize) < len),
            _ => false,
        };
        if !proven {
            return Err(format!(
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
                validate_static_bounds_expr(child, &mut scoped)?;
                scoped = state_for_true_branch(child, &scoped);
            }
        }
        "do" => {
            let mut scoped = facts.clone();
            for child in items.iter().skip(1) {
                validate_static_bounds_expr(child, &mut scoped)?;
            }
            *facts = scoped;
        }
        "block" => {
            let mut scoped = facts.clone();
            for child in items.iter().skip(1) {
                validate_static_bounds_expr(child, &mut scoped)?;
            }
        }
        "lambda" => {
            let mut scoped = AbstractState::default();
            for child in items.iter().skip(2) {
                validate_static_bounds_expr(child, &mut scoped)?;
            }
        }
        "let" | "mut" if items.len() >= 3 => {
            validate_static_bounds_expr(&items[2], facts)?;
            if let Expression::Word(name) = &items[1] {
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
            }
        }
        "alter!" if items.len() == 3 => {
            validate_static_bounds_expr(&items[2], facts)?;
            if let Expression::Word(name) = &items[1] {
                assign_abstract_scalar(name, &items[2], facts);
            }
        }
        "if" if items.len() >= 3 => {
            validate_static_bounds_expr(&items[1], facts)?;
            let mut consequent = state_for_true_branch(&items[1], facts);
            validate_static_bounds_expr(&items[2], &mut consequent)?;
            let mut alternate = state_for_false_branch(&items[1], facts);
            if let Some(otherwise) = items.get(3) {
                validate_static_bounds_expr(otherwise, &mut alternate)?;
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
                validate_static_bounds_expr(&items[1], &mut header)?;
                let mut body_exit = state_for_true_branch(&items[1], &header);
                for child in items.iter().skip(2) {
                    validate_static_bounds_expr(child, &mut body_exit)?;
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
                validate_static_bounds_expr(child, &mut scoped)?;
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
                validate_static_bounds_expr(child, &mut scoped)?;
            }
        }
        _ => {
            for child in items.iter().skip(1) {
                validate_static_bounds_expr(child, facts)?;
            }
        }
    }
    invalidate_resized_vector(items, facts);
    Ok(())
}

pub fn analyze_user_program(
    typed_program: &TypedExpression,
    user_form_count: usize,
) -> Result<(), String> {
    let all_expressions: Vec<&Expression> = match &typed_program.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "do") => {
            items.iter().skip(1).collect()
        }
        expression => vec![expression],
    };
    let mut guard_summaries = infer_guard_summaries(&all_expressions);
    // Retain the canonical predicates even when tree shaking has removed their
    // definitions before this pass sees the program.
    guard_summaries
        .entry("in-bounds?".to_string())
        .or_insert((0, 1));
    guard_summaries
        .entry("std/vector/in-bounds?".to_string())
        .or_insert((0, 1));
    guard_summaries
        .entry("Vector/in-bounds?".to_string())
        .or_insert((1, 0));
    let start = all_expressions.len().saturating_sub(user_form_count);
    let mut facts = AbstractState {
        guard_summaries,
        ..AbstractState::default()
    };
    for expression in &all_expressions[start..] {
        validate_static_bounds_expr(expression, &mut facts)?;
    }
    Ok(())
}

fn infer_guard_summaries(expressions: &[&Expression]) -> HashMap<String, (usize, usize)> {
    let mut summaries = HashMap::new();
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
                if let Some(summary) = summaries.get(alias).copied() {
                    changed |= summaries.insert(name.clone(), summary) != Some(summary);
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
            for (vector, index) in &facts.safe_pairs {
                if let (Some(vector_arg), Some(index_arg)) = (
                    params.iter().position(|param| *param == vector),
                    params.iter().position(|param| *param == index),
                ) {
                    let summary = (vector_arg, index_arg);
                    changed |= summaries.insert(name.clone(), summary) != Some(summary);
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }
    summaries
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
}
