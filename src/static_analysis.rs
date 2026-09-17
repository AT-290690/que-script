use crate::infer::TypedExpression;
use crate::parser::Expression;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct GuardRelation {
    vector_arg: usize,
    projection_index_args: Vec<usize>,
    index_arg: usize,
}

type GuardSummary = Vec<GuardRelation>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct PredicateSummary {
    params: Vec<String>,
    body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IntInterval {
    min: i64,
    max: i64,
}

/// A normalized affine expression without its constant term.  Keeping these
/// relations lets the interval domain retain correlations such as
/// `a <= INT-MAX - b`, which is exactly `a + b <= INT-MAX`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct AffineTerms(Vec<(String, i64)>);

impl IntInterval {
    const I32: Self = Self {
        min: i32::MIN as i64,
        max: i32::MAX as i64,
    };

    fn exact(value: i32) -> Self {
        Self {
            min: value as i64,
            max: value as i64,
        }
    }

    fn excludes_zero(self) -> bool {
        self.max < 0 || self.min > 0
    }

    fn fits_i32(self) -> bool {
        self.min >= Self::I32.min && self.max <= Self::I32.max
    }
}

/// Abstract state at one program point.  This is deliberately independent of
/// the runtime representation: later analyses (division, overflow, and so on)
/// can add domains here without becoming part of WAT lowering.
#[derive(Clone, Default, PartialEq, Eq)]
struct AbstractState {
    safe_pairs: HashSet<(String, String)>,
    nonnegative: HashSet<String>,
    integer_constants: HashMap<String, i32>,
    integer_ranges: HashMap<String, IntInterval>,
    nonzero: HashSet<String>,
    /// Symbolic integer ordering facts: `(a, b)` means `a <= b`.
    leq_pairs: HashSet<(String, String)>,
    /// Upper bounds for normalized affine expressions: `terms <= bound`.
    affine_upper_bounds: HashMap<AffineTerms, i64>,
    /// A canonical operand pair whose product is proven not to cross the
    /// corresponding signed Int boundary.
    product_upper_safe: HashSet<(String, String)>,
    product_lower_safe: HashSet<(String, String)>,
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
    predicate_summaries: HashMap<String, PredicateSummary>,
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
    let integer_ranges = left
        .integer_ranges
        .iter()
        .filter_map(|(name, left_range)| {
            right.integer_ranges.get(name).map(|right_range| {
                (
                    name.clone(),
                    IntInterval {
                        min: left_range.min.min(right_range.min),
                        max: left_range.max.max(right_range.max),
                    },
                )
            })
        })
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
    let mut affine_candidates: HashSet<AffineTerms> =
        left.affine_upper_bounds.keys().cloned().collect();
    affine_candidates.extend(right.affine_upper_bounds.keys().cloned());
    let affine_upper_bounds = affine_candidates
        .into_iter()
        .filter_map(|terms| {
            let left_bound = affine_upper_bound(&terms, left)?;
            let right_bound = affine_upper_bound(&terms, right)?;
            Some((terms, left_bound.max(right_bound)))
        })
        .collect();
    let mut product_candidates: HashSet<(String, String)> = left
        .product_upper_safe
        .union(&right.product_upper_safe)
        .cloned()
        .collect();
    product_candidates.extend(
        left.product_lower_safe
            .union(&right.product_lower_safe)
            .cloned(),
    );
    let product_upper_safe = product_candidates
        .iter()
        .filter(|pair| product_upper_is_safe(pair, left) && product_upper_is_safe(pair, right))
        .cloned()
        .collect();
    let product_lower_safe = product_candidates
        .iter()
        .filter(|pair| product_lower_is_safe(pair, left) && product_lower_is_safe(pair, right))
        .cloned()
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
        integer_ranges,
        nonzero: left.nonzero.intersection(&right.nonzero).cloned().collect(),
        leq_pairs: left
            .leq_pairs
            .intersection(&right.leq_pairs)
            .cloned()
            .collect(),
        affine_upper_bounds,
        product_upper_safe,
        product_lower_safe,
        fixed_lengths,
        minimum_lengths,
        length_sources,
        aliases,
        scalar_aliases,
        // Function summaries are immutable analysis metadata rather than a
        // path-sensitive fact.
        guard_summaries: left.guard_summaries.clone(),
        predicate_summaries: left.predicate_summaries.clone(),
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

fn canonical_product_pair(
    left: &Expression,
    right: &Expression,
    state: &AbstractState,
) -> (String, String) {
    let mut pair = (
        canonical_scalar(left, state),
        canonical_scalar(right, state),
    );
    if pair.1 < pair.0 {
        pair = (pair.1, pair.0);
    }
    pair
}

fn range_for_canonical_atom(atom: &str, state: &AbstractState) -> IntInterval {
    state
        .integer_ranges
        .get(atom)
        .copied()
        .or_else(|| {
            state
                .integer_constants
                .get(atom)
                .copied()
                .map(IntInterval::exact)
        })
        .unwrap_or(IntInterval::I32)
}

fn product_interval_for_pair(pair: &(String, String), state: &AbstractState) -> IntInterval {
    let left = range_for_canonical_atom(&pair.0, state);
    let right = range_for_canonical_atom(&pair.1, state);
    let products = [
        left.min * right.min,
        left.min * right.max,
        left.max * right.min,
        left.max * right.max,
    ];
    IntInterval {
        min: *products.iter().min().expect("four product endpoints"),
        max: *products.iter().max().expect("four product endpoints"),
    }
}

fn product_upper_is_safe(pair: &(String, String), state: &AbstractState) -> bool {
    state.product_upper_safe.contains(pair)
        || product_interval_for_pair(pair, state).max <= i32::MAX as i64
}

fn product_lower_is_safe(pair: &(String, String), state: &AbstractState) -> bool {
    state.product_lower_safe.contains(pair)
        || product_interval_for_pair(pair, state).min >= i32::MIN as i64
}

fn materialize_product_interval_safety(pair: &(String, String), state: &mut AbstractState) {
    let interval = product_interval_for_pair(pair, state);
    if interval.max <= i32::MAX as i64 {
        state.product_upper_safe.insert(pair.clone());
    }
    if interval.min >= i32::MIN as i64 {
        state.product_lower_safe.insert(pair.clone());
    }
}

fn record_product_division_bound(
    smaller: &Expression,
    larger: &Expression,
    state: &mut AbstractState,
) {
    // `factor <= bound / divisor`. Multiplication reverses the relation for a
    // negative divisor. The comparison is useful only once the current path
    // has proved the divisor's sign.
    let Expression::Apply(div) = larger else {
        return;
    };
    let [Expression::Word(op), numerator, divisor] = div.as_slice() else {
        return;
    };
    if op != "/" {
        return;
    }
    let Some(bound) = integer_constant(numerator, state) else {
        return;
    };
    let divisor_range = integer_interval(divisor, state).unwrap_or(IntInterval::I32);
    let pair = canonical_product_pair(smaller, divisor, state);
    if divisor_range.min > 0 && bound == i32::MAX {
        state.product_upper_safe.insert(pair.clone());
    } else if divisor_range.max < 0 && bound == i32::MIN {
        state.product_lower_safe.insert(pair.clone());
    }
    materialize_product_interval_safety(&pair, state);
}

fn record_product_comparison(
    left: &Expression,
    right: &Expression,
    comparison: &str,
    state: &mut AbstractState,
) {
    match comparison {
        "<" | "<=" => {
            record_product_division_bound(left, right, state);
            // `(bound / divisor) <= factor` is the lower-bound form for a
            // positive divisor and the upper-bound form for a negative one.
            record_reversed_product_division_bound(left, right, state);
        }
        ">" | ">=" => {
            record_product_division_bound(right, left, state);
            record_reversed_product_division_bound(right, left, state);
        }
        _ => {}
    }
}

fn record_reversed_product_division_bound(
    smaller: &Expression,
    larger: &Expression,
    state: &mut AbstractState,
) {
    let Expression::Apply(div) = smaller else {
        return;
    };
    let [Expression::Word(op), numerator, divisor] = div.as_slice() else {
        return;
    };
    if op != "/" {
        return;
    }
    let Some(bound) = integer_constant(numerator, state) else {
        return;
    };
    let divisor_range = integer_interval(divisor, state).unwrap_or(IntInterval::I32);
    let pair = canonical_product_pair(larger, divisor, state);
    if divisor_range.min > 0 && bound == i32::MIN {
        state.product_lower_safe.insert(pair.clone());
    } else if divisor_range.max < 0 && bound == i32::MAX {
        state.product_upper_safe.insert(pair.clone());
    }
    materialize_product_interval_safety(&pair, state);
}

fn affine_expression(expr: &Expression, state: &AbstractState) -> Option<(AffineTerms, i64)> {
    if let Some(value) = integer_constant(expr, state) {
        return Some((AffineTerms(Vec::new()), value as i64));
    }
    fn collect(
        expr: &Expression,
        state: &AbstractState,
        scale: i64,
        terms: &mut BTreeMap<String, i64>,
        constant: &mut i64,
    ) -> Option<()> {
        if let Some(value) = integer_constant(expr, state) {
            *constant = constant.checked_add(scale.checked_mul(value as i64)?)?;
            return Some(());
        }
        match expr {
            Expression::Apply(items) => match items.as_slice() {
                [Expression::Word(op), left, right] if op == "+" => {
                    collect(left, state, scale, terms, constant)?;
                    collect(right, state, scale, terms, constant)
                }
                [Expression::Word(op), left, right] if op == "-" => {
                    collect(left, state, scale, terms, constant)?;
                    collect(right, state, -scale, terms, constant)
                }
                _ => {
                    let atom = canonical_scalar(expr, state);
                    *terms.entry(atom).or_default() += scale;
                    Some(())
                }
            },
            Expression::Word(_) => {
                let atom = canonical_scalar(expr, state);
                *terms.entry(atom).or_default() += scale;
                Some(())
            }
            Expression::Int(value) => {
                *constant = constant.checked_add(scale.checked_mul(*value as i64)?)?;
                Some(())
            }
            Expression::Dec(_) => None,
        }
    }

    let mut terms = BTreeMap::new();
    let mut constant = 0;
    collect(expr, state, 1, &mut terms, &mut constant)?;
    terms.retain(|_, coefficient| *coefficient != 0);
    Some((AffineTerms(terms.into_iter().collect()), constant))
}

fn negate_affine_terms(terms: &AffineTerms) -> AffineTerms {
    AffineTerms(
        terms
            .0
            .iter()
            .map(|(atom, coefficient)| (atom.clone(), -*coefficient))
            .collect(),
    )
}

fn affine_interval_from_atoms(terms: &AffineTerms, state: &AbstractState) -> Option<IntInterval> {
    let mut result = IntInterval { min: 0, max: 0 };
    for (atom, coefficient) in &terms.0 {
        let range = state
            .integer_ranges
            .get(atom)
            .copied()
            .or_else(|| {
                state
                    .integer_constants
                    .get(atom)
                    .copied()
                    .map(IntInterval::exact)
            })
            .unwrap_or(IntInterval::I32);
        let (min, max) = if *coefficient >= 0 {
            (
                range.min.checked_mul(*coefficient)?,
                range.max.checked_mul(*coefficient)?,
            )
        } else {
            (
                range.max.checked_mul(*coefficient)?,
                range.min.checked_mul(*coefficient)?,
            )
        };
        result.min = result.min.checked_add(min)?;
        result.max = result.max.checked_add(max)?;
    }
    Some(result)
}

fn affine_upper_bound(terms: &AffineTerms, state: &AbstractState) -> Option<i64> {
    let interval_bound = affine_interval_from_atoms(terms, state)?.max;
    Some(
        state
            .affine_upper_bounds
            .get(terms)
            .copied()
            .map_or(interval_bound, |known| known.min(interval_bound)),
    )
}

fn affine_interval(expr: &Expression, state: &AbstractState) -> Option<IntInterval> {
    let (terms, constant) = affine_expression(expr, state)?;
    let max = affine_upper_bound(&terms, state)?.checked_add(constant)?;
    let negated = negate_affine_terms(&terms);
    let min = affine_upper_bound(&negated, state)?
        .checked_neg()?
        .checked_add(constant)?;
    Some(IntInterval { min, max })
}

fn record_affine_comparison(
    left: &Expression,
    right: &Expression,
    comparison: &str,
    state: &mut AbstractState,
) {
    let (left_terms, left_constant) = match affine_expression(left, state) {
        Some(value) => value,
        None => return,
    };
    let (right_terms, right_constant) = match affine_expression(right, state) {
        Some(value) => value,
        None => return,
    };
    let mut combined: BTreeMap<String, i64> = left_terms.0.into_iter().collect();
    for (atom, coefficient) in right_terms.0 {
        *combined.entry(atom).or_default() -= coefficient;
    }
    combined.retain(|_, coefficient| *coefficient != 0);
    let terms = AffineTerms(combined.into_iter().collect());
    let Some(mut bound) = right_constant.checked_sub(left_constant) else {
        return;
    };
    if comparison == "<" {
        bound -= 1;
    }
    state
        .affine_upper_bounds
        .entry(terms)
        .and_modify(|known| *known = (*known).min(bound))
        .or_insert(bound);
}

fn record_effective_affine_comparison(
    left: &Expression,
    right: &Expression,
    comparison: &str,
    state: &mut AbstractState,
) {
    match comparison {
        "<" | "<=" => record_affine_comparison(left, right, comparison, state),
        ">" => record_affine_comparison(right, left, "<", state),
        ">=" => record_affine_comparison(right, left, "<=", state),
        "=" => {
            record_affine_comparison(left, right, "<=", state);
            record_affine_comparison(right, left, "<=", state);
        }
        _ => {}
    }
}

fn relation_is_known_leq(left: &Expression, right: &Expression, state: &AbstractState) -> bool {
    let start = canonical_scalar(left, state);
    let goal = canonical_scalar(right, state);
    if start == goal {
        return true;
    }
    let mut pending = vec![start];
    let mut seen = HashSet::new();
    while let Some(current) = pending.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        for (_, upper) in state
            .leq_pairs
            .iter()
            .filter(|(lower, _)| lower == &current)
        {
            if upper == &goal {
                return true;
            }
            pending.push(upper.clone());
        }
    }
    false
}

fn propagate_relational_ranges(facts: &mut AbstractState) {
    for _ in 0..facts.leq_pairs.len().saturating_add(1) {
        let mut changed = false;
        for (lower, upper) in facts.leq_pairs.clone() {
            let lower_range = facts
                .integer_ranges
                .get(&lower)
                .copied()
                .unwrap_or(IntInterval::I32);
            let upper_range = facts
                .integer_ranges
                .get(&upper)
                .copied()
                .unwrap_or(IntInterval::I32);
            let narrowed_lower = IntInterval {
                min: lower_range.min,
                max: lower_range.max.min(upper_range.max),
            };
            let narrowed_upper = IntInterval {
                min: upper_range.min.max(lower_range.min),
                max: upper_range.max,
            };
            if narrowed_lower != lower_range {
                facts.integer_ranges.insert(lower.clone(), narrowed_lower);
                changed = true;
            }
            if narrowed_upper != upper_range {
                facts.integer_ranges.insert(upper.clone(), narrowed_upper);
                changed = true;
            }
        }
        if !changed {
            break;
        }
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

fn safe_midpoint_interval(
    base: &Expression,
    offset: &Expression,
    state: &AbstractState,
) -> Option<IntInterval> {
    let Expression::Apply(div) = offset else {
        return None;
    };
    let [Expression::Word(div_op), Expression::Apply(sub), divisor] = div.as_slice() else {
        return None;
    };
    let [Expression::Word(sub_op), upper, repeated_base] = sub.as_slice() else {
        return None;
    };
    if div_op != "/"
        || sub_op != "-"
        || canonical_scalar(base, state) != canonical_scalar(repeated_base, state)
        || integer_constant(divisor, state).is_none_or(|value| value < 1)
        || !relation_is_known_leq(base, upper, state)
    {
        return None;
    }
    let base_range = integer_interval(base, state)?;
    let upper_range = integer_interval(upper, state)?;
    // With base <= upper and a positive divisor, the offset lies between zero
    // and upper-base, so the result remains between base and upper. Requiring
    // a nonnegative base also proves upper-base itself cannot overflow Int.
    if base_range.min < 0 {
        return None;
    }
    Some(IntInterval {
        min: base_range.min,
        max: upper_range.max,
    })
}

fn integer_interval(expr: &Expression, state: &AbstractState) -> Option<IntInterval> {
    match expr {
        Expression::Int(value) => Some(IntInterval::exact(*value)),
        Expression::Word(name) => {
            let key = canonical_scalar(expr, state);
            state
                .integer_ranges
                .get(&key)
                .copied()
                .or_else(|| state.integer_ranges.get(name).copied())
                .or_else(|| {
                    state
                        .integer_constants
                        .get(name)
                        .copied()
                        .map(IntInterval::exact)
                })
                .or_else(|| {
                    state.nonnegative.contains(name).then_some(IntInterval {
                        min: 0,
                        max: i32::MAX as i64,
                    })
                })
        }
        Expression::Apply(items) => match items.as_slice() {
            [Expression::Word(op), value] if op == "length" => {
                let vector = canonical_access(value, state);
                state
                    .fixed_lengths
                    .get(&vector)
                    .copied()
                    .or_else(|| literal_vector_length(value))
                    .map(|len| IntInterval {
                        min: len.min(i32::MAX as usize) as i64,
                        max: len.min(i32::MAX as usize) as i64,
                    })
                    .or(Some(IntInterval {
                        min: 0,
                        max: i32::MAX as i64,
                    }))
            }
            [Expression::Word(op), left, right] if matches!(op.as_str(), "+" | "-" | "*") => {
                if op == "+" {
                    if let Some(midpoint) = safe_midpoint_interval(left, right, state)
                        .or_else(|| safe_midpoint_interval(right, left, state))
                    {
                        return Some(midpoint);
                    }
                }
                let left_range = integer_interval(left, state)?;
                let right_range = integer_interval(right, state)?;
                let mut result = integer_arithmetic_interval(op, left_range, right_range)?;
                if op == "-" {
                    if relation_is_known_leq(&items[2], &items[1], state) {
                        result.min = result.min.max(0);
                    }
                    if relation_is_known_leq(&items[1], &items[2], state) {
                        result.max = result.max.min(0);
                    }
                }
                Some(result)
            }
            [Expression::Word(op), numerator, divisor] if op == "/" => {
                let numerator = integer_interval(numerator, state)?;
                let divisor = integer_constant(divisor, state)?;
                if divisor == 0 {
                    return None;
                }
                let a = numerator.min / divisor as i64;
                let b = numerator.max / divisor as i64;
                Some(IntInterval {
                    min: a.min(b),
                    max: a.max(b),
                })
            }
            _ => state
                .integer_ranges
                .get(&canonical_scalar(expr, state))
                .copied(),
        },
        Expression::Dec(_) => None,
    }
}

fn integer_arithmetic_interval(
    op: &str,
    left: IntInterval,
    right: IntInterval,
) -> Option<IntInterval> {
    Some(match op {
        "+" => IntInterval {
            min: left.min.saturating_add(right.min),
            max: left.max.saturating_add(right.max),
        },
        "-" => IntInterval {
            min: left.min.saturating_sub(right.max),
            max: left.max.saturating_sub(right.min),
        },
        "*" => {
            let products = [
                left.min.saturating_mul(right.min),
                left.min.saturating_mul(right.max),
                left.max.saturating_mul(right.min),
                left.max.saturating_mul(right.max),
            ];
            IntInterval {
                min: *products.iter().min().expect("four products"),
                max: *products.iter().max().expect("four products"),
            }
        }
        _ => return None,
    })
}

fn divisor_is_proven_nonzero(expr: &Expression, state: &AbstractState) -> bool {
    let key = canonical_scalar(expr, state);
    state.nonzero.contains(&key)
        || integer_interval(expr, state).is_some_and(IntInterval::excludes_zero)
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
    expansion_depth: usize,
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
                    expansion_depth,
                );
            }
        }
        [Expression::Word(op), inner] if op == "not" => {
            collect_static_bound_guard_facts(
                inner,
                facts,
                !is_true,
                lower,
                upper,
                minimum_lengths,
                expansion_depth,
            );
        }
        [Expression::Word(op), Expression::Word(index), Expression::Int(bound)]
            if (is_true && ((op == ">=" && *bound == 0) || (op == ">" && *bound == -1)))
                || (!is_true && ((op == "<" && *bound == 0) || (op == "<=" && *bound == -1))) =>
        {
            lower.insert(canonical_scalar(&Expression::Word(index.clone()), facts));
        }
        [Expression::Word(op), Expression::Apply(length), bound]
            if op == "="
                && is_true
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            if let Some(minimum) = integer_constant(bound, facts).filter(|value| *value > 0) {
                minimum_lengths.push((canonical_access(&length[1], facts), minimum as usize));
            }
        }
        [Expression::Word(op), bound, Expression::Apply(length)]
            if op == "="
                && is_true
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            if let Some(minimum) = integer_constant(bound, facts).filter(|value| *value > 0) {
                minimum_lengths.push((canonical_access(&length[1], facts), minimum as usize));
            }
        }
        [Expression::Word(op), Expression::Apply(length), Expression::Int(0)]
            if op == "="
                && !is_true
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            minimum_lengths.push((canonical_access(&length[1], facts), 1));
        }
        [Expression::Word(op), Expression::Apply(length), Expression::Int(bound)]
            if matches!(op.as_str(), ">" | ">=" | "<" | "<=")
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            let minimum = match (op.as_str(), is_true) {
                (">", true) | ("<=", false) => i64::from(*bound) + 1,
                (">=", true) | ("<", false) => i64::from(*bound),
                _ => return,
            };
            if minimum > 0 {
                minimum_lengths.push((canonical_access(&length[1], facts), minimum as usize));
            }
        }
        [Expression::Word(op), Expression::Int(bound), Expression::Apply(length)]
            if matches!(op.as_str(), ">" | ">=" | "<" | "<=")
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            let minimum = match (op.as_str(), is_true) {
                ("<", true) | (">=", false) => i64::from(*bound) + 1,
                ("<=", true) | (">", false) => i64::from(*bound),
                _ => return,
            };
            if minimum > 0 {
                minimum_lengths.push((canonical_access(&length[1], facts), minimum as usize));
            }
        }
        [Expression::Word(op), Expression::Int(0), Expression::Apply(length)]
            if op == "="
                && !is_true
                && matches!(length.first(), Some(Expression::Word(len)) if len == "length")
                && length.len() == 2 =>
        {
            minimum_lengths.push((canonical_access(&length[1], facts), 1));
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
            if expansion_depth < 16 {
                if let Some(op) = items.first().and_then(word) {
                    if let Some(summary) = facts.predicate_summaries.get(op) {
                        if summary.params.len() == items.len().saturating_sub(1) {
                            let Ok(parsed_body) = crate::parser::build(&summary.body) else {
                                return;
                            };
                            let substitutions: HashMap<&str, &Expression> = summary
                                .params
                                .iter()
                                .map(String::as_str)
                                .zip(items.iter().skip(1))
                                .collect();
                            let expanded = substitute_predicate_body(
                                single_built_expression(&parsed_body),
                                &substitutions,
                            );
                            collect_static_bound_guard_facts(
                                &expanded,
                                facts,
                                is_true,
                                lower,
                                upper,
                                minimum_lengths,
                                expansion_depth + 1,
                            );
                        }
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

fn constrain_integer_range(expr: &Expression, facts: &mut AbstractState, constraint: IntInterval) {
    let key = canonical_scalar(expr, facts);
    let current = integer_interval(expr, facts).unwrap_or(IntInterval::I32);
    let mut narrowed = IntInterval {
        min: current.min.max(constraint.min),
        max: current.max.min(constraint.max),
    };
    if facts.nonzero.contains(&key) {
        if narrowed.min == 0 && narrowed.max > 0 {
            narrowed.min = 1;
        } else if narrowed.max == 0 && narrowed.min < 0 {
            narrowed.max = -1;
        }
    }
    if narrowed.min <= narrowed.max {
        facts.integer_ranges.insert(key.clone(), narrowed);
        if narrowed.excludes_zero() {
            facts.nonzero.insert(key);
        }
    }
}

fn predicate_result_state(
    expr: &Expression,
    state: &AbstractState,
    desired: bool,
    expansion_depth: usize,
) -> Option<AbstractState> {
    if let Expression::Word(value) = expr {
        if value == "true" || value == "false" {
            return ((value == "true") == desired).then(|| state.clone());
        }
    }
    let mut result = state.clone();
    collect_numeric_guard_facts(expr, &mut result, desired, expansion_depth);
    Some(result)
}

fn collect_numeric_guard_facts(
    expr: &Expression,
    facts: &mut AbstractState,
    is_true: bool,
    expansion_depth: usize,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    match items.as_slice() {
        [Expression::Word(op), condition, consequent, alternate] if op == "if" => {
            let mut outcomes = Vec::new();
            let mut true_path = facts.clone();
            collect_numeric_guard_facts(condition, &mut true_path, true, expansion_depth);
            if let Some(outcome) =
                predicate_result_state(consequent, &true_path, is_true, expansion_depth)
            {
                outcomes.push(outcome);
            }
            let mut false_path = facts.clone();
            collect_numeric_guard_facts(condition, &mut false_path, false, expansion_depth);
            if let Some(outcome) =
                predicate_result_state(alternate, &false_path, is_true, expansion_depth)
            {
                outcomes.push(outcome);
            }
            if let Some(first) = outcomes
                .into_iter()
                .reduce(|left, right| join_states(&left, &right))
            {
                *facts = first;
            }
        }
        [Expression::Word(op), operands @ ..]
            if operands.len() >= 2 && ((op == "or" && is_true) || (op == "and" && !is_true)) =>
        {
            let mut alternatives = operands.iter().map(|operand| {
                let mut branch = facts.clone();
                collect_numeric_guard_facts(operand, &mut branch, is_true, expansion_depth);
                branch
            });
            if let Some(first) = alternatives.next() {
                *facts = alternatives.fold(first, |joined, branch| join_states(&joined, &branch));
            }
        }
        [Expression::Word(op), operands @ ..]
            if operands.len() >= 2 && ((op == "and" && is_true) || (op == "or" && !is_true)) =>
        {
            for operand in operands {
                collect_numeric_guard_facts(operand, facts, is_true, expansion_depth);
            }
        }
        [Expression::Word(op), inner] if op == "not" => {
            collect_numeric_guard_facts(inner, facts, !is_true, expansion_depth);
        }
        [Expression::Word(op), left, right]
            if matches!(op.as_str(), "=" | ">" | ">=" | "<" | "<=") =>
        {
            let (value, bound, comparison) = if let Some(bound) = integer_constant(right, facts) {
                (left, bound, op.as_str())
            } else if let Some(bound) = integer_constant(left, facts) {
                let reversed = match op.as_str() {
                    "=" => "=",
                    ">" => "<",
                    ">=" => "<=",
                    "<" => ">",
                    "<=" => ">=",
                    _ => return,
                };
                (right, bound, reversed)
            } else {
                let effective = if is_true {
                    op.as_str()
                } else {
                    match op.as_str() {
                        ">" => "<=",
                        ">=" => "<",
                        "<" => ">=",
                        "<=" => ">",
                        "=" => "!=",
                        _ => return,
                    }
                };
                record_effective_affine_comparison(left, right, effective, facts);
                record_product_comparison(left, right, effective, facts);
                let left_key = canonical_scalar(left, facts);
                let right_key = canonical_scalar(right, facts);
                match effective {
                    "<" | "<=" => {
                        facts.leq_pairs.insert((left_key, right_key));
                    }
                    ">" | ">=" => {
                        facts.leq_pairs.insert((right_key, left_key));
                    }
                    "=" => {
                        facts
                            .leq_pairs
                            .insert((left_key.clone(), right_key.clone()));
                        facts.leq_pairs.insert((right_key, left_key));
                    }
                    _ => {}
                }
                propagate_relational_ranges(facts);
                return;
            };
            let effective = if is_true {
                comparison
            } else {
                match comparison {
                    ">" => "<=",
                    ">=" => "<",
                    "<" => ">=",
                    "<=" => ">",
                    "=" => "!=",
                    _ => return,
                }
            };
            record_effective_affine_comparison(left, right, effective, facts);
            record_product_comparison(left, right, effective, facts);
            match effective {
                "=" => constrain_integer_range(value, facts, IntInterval::exact(bound)),
                "!=" if bound == 0 => {
                    facts.nonzero.insert(canonical_scalar(value, facts));
                }
                ">" => constrain_integer_range(
                    value,
                    facts,
                    IntInterval {
                        min: (bound as i64) + 1,
                        max: i32::MAX as i64,
                    },
                ),
                ">=" => constrain_integer_range(
                    value,
                    facts,
                    IntInterval {
                        min: bound as i64,
                        max: i32::MAX as i64,
                    },
                ),
                "<" => constrain_integer_range(
                    value,
                    facts,
                    IntInterval {
                        min: i32::MIN as i64,
                        max: (bound as i64) - 1,
                    },
                ),
                "<=" => constrain_integer_range(
                    value,
                    facts,
                    IntInterval {
                        min: i32::MIN as i64,
                        max: bound as i64,
                    },
                ),
                _ => {}
            }
            propagate_relational_ranges(facts);
        }
        _ if expansion_depth < 16 => {
            let Some(op) = items.first().and_then(word) else {
                return;
            };
            let Some(summary) = facts.predicate_summaries.get(op).cloned() else {
                return;
            };
            if summary.params.len() != items.len().saturating_sub(1) {
                return;
            }
            let Ok(parsed_body) = crate::parser::build(&summary.body) else {
                return;
            };
            let substitutions: HashMap<&str, &Expression> = summary
                .params
                .iter()
                .map(String::as_str)
                .zip(items.iter().skip(1))
                .collect();
            let expanded =
                substitute_predicate_body(single_built_expression(&parsed_body), &substitutions);
            collect_numeric_guard_facts(&expanded, facts, is_true, expansion_depth + 1);
        }
        _ => {}
    }
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
        0,
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
    collect_numeric_guard_facts(expr, &mut next, is_true, 0);
    next
}

fn apply_vector_mutation(items: &[Expression], facts: &mut AbstractState) {
    let Some(op) = items.first().and_then(word) else {
        return;
    };
    let Some(xs_expr) = items.get(1) else {
        return;
    };
    let xs = canonical_access(xs_expr, facts);
    let nested_prefix = format!("(get {xs} ");

    match op {
        "push!" => {
            facts
                .fixed_lengths
                .entry(xs.clone())
                .and_modify(|length| *length = length.saturating_add(1));
            facts
                .minimum_lengths
                .entry(xs)
                .and_modify(|length| *length = length.saturating_add(1))
                .or_insert(1);
        }
        "set!" => {
            // Replacing an element can invalidate facts about a nested vector,
            // but never invalidates an existing index into the container.
            facts
                .safe_pairs
                .retain(|(name, _)| !name.starts_with(&nested_prefix));
            facts
                .fixed_lengths
                .retain(|name, _| !name.starts_with(&nested_prefix));
            facts
                .minimum_lengths
                .retain(|name, _| !name.starts_with(&nested_prefix));
            facts
                .length_sources
                .retain(|_, source| !source.starts_with(&nested_prefix));

            let direct_append = items.get(2).is_some_and(|index| {
                matches!(index, Expression::Apply(length)
                    if matches!(length.first(), Some(Expression::Word(name)) if name == "length")
                        && length.len() == 2
                        && canonical_access(&length[1], facts) == xs)
            });
            let known_append = items
                .get(2)
                .and_then(|index| integer_constant(index, facts))
                .zip(
                    facts
                        .fixed_lengths
                        .get(&xs)
                        .copied()
                        .and_then(|length| i32::try_from(length).ok()),
                )
                .is_some_and(|(index, length)| index == length);
            if direct_append || known_append {
                facts
                    .fixed_lengths
                    .entry(xs.clone())
                    .and_modify(|length| *length = length.saturating_add(1));
                facts
                    .minimum_lengths
                    .entry(xs)
                    .and_modify(|length| *length = length.saturating_add(1))
                    .or_insert(1);
            }
        }
        "pop!" | "pop-val!" => {
            facts
                .safe_pairs
                .retain(|(name, _)| name != &xs && !name.starts_with(&nested_prefix));
            facts
                .fixed_lengths
                .retain(|name, _| name == &xs || !name.starts_with(&nested_prefix));
            if let Some(length) = facts.fixed_lengths.get_mut(&xs) {
                *length = length.saturating_sub(1);
            }
            facts
                .minimum_lengths
                .retain(|name, _| name == &xs || !name.starts_with(&nested_prefix));
            if let Some(length) = facts.minimum_lengths.get_mut(&xs) {
                *length = length.saturating_sub(1);
            }
            facts
                .length_sources
                .retain(|_, source| source != &xs && !source.starts_with(&nested_prefix));
        }
        _ => {}
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
    let interval = integer_interval(value, facts).filter(|range| range.fits_i32());
    let remains_nonzero = divisor_is_proven_nonzero(value, facts);
    facts.safe_pairs.retain(|(_, index)| index != name);
    facts.nonnegative.remove(name);
    facts.integer_constants.remove(name);
    facts.integer_ranges.remove(name);
    facts.nonzero.remove(name);
    facts.fixed_lengths.remove(name);
    facts.minimum_lengths.remove(name);
    facts.length_sources.remove(name);
    facts.aliases.remove(name);
    facts.scalar_aliases.remove(name);
    facts
        .leq_pairs
        .retain(|(left, right)| left != name && right != name);
    facts
        .affine_upper_bounds
        .retain(|terms, _| !terms.0.iter().any(|(atom, _)| atom == name));
    facts
        .product_upper_safe
        .retain(|(left, right)| left != name && right != name);
    facts
        .product_lower_safe
        .retain(|(left, right)| left != name && right != name);

    if remains_nonnegative {
        facts.nonnegative.insert(name.to_string());
    }
    if let Some(constant) = constant {
        facts.integer_constants.insert(name.to_string(), constant);
    }
    if let Some(interval) = interval {
        facts.integer_ranges.insert(name.to_string(), interval);
    }
    if remains_nonzero {
        facts.nonzero.insert(name.to_string());
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

fn forget_lambda_parameter(expr: &Expression, facts: &mut AbstractState) {
    match expr {
        Expression::Word(name) => {
            facts.nonnegative.remove(name);
            facts.nonzero.remove(name);
            facts.integer_constants.remove(name);
            facts.integer_ranges.remove(name);
            facts.scalar_aliases.remove(name);
            facts
                .leq_pairs
                .retain(|(left, right)| left != name && right != name);
            facts
                .affine_upper_bounds
                .retain(|terms, _| !terms.0.iter().any(|(atom, _)| atom == name));
            facts
                .product_upper_safe
                .retain(|(left, right)| left != name && right != name);
            facts
                .product_lower_safe
                .retain(|(left, right)| left != name && right != name);
        }
        Expression::Apply(items) => {
            for item in items {
                forget_lambda_parameter(item, facts);
            }
        }
        Expression::Int(_) | Expression::Dec(_) => {}
    }
}

fn record_diagnostic(diagnostics: &mut Vec<String>, message: String) {
    if !diagnostics.contains(&message) {
        diagnostics.push(message);
    }
}

fn access_index_is_proven(
    vector: &Expression,
    index: &Expression,
    facts: &AbstractState,
    allow_append: bool,
) -> bool {
    let vector_key = canonical_access(vector, facts);
    let index_key = canonical_scalar(index, facts);

    // A normal bounds proof establishes 0 <= index < length. This is also
    // sufficient for set!, whose additional valid case is index == length.
    if facts.safe_pairs.contains(&(vector_key.clone(), index_key)) {
        return true;
    }

    // `set! xs (length xs) value` is Que's append-at-end operation.
    if allow_append {
        if let Expression::Apply(length) = index {
            if matches!(length.first(), Some(Expression::Word(op)) if op == "length")
                && length.len() == 2
                && canonical_access(&length[1], facts) == vector_key
            {
                return true;
            }
        }
        if let Expression::Word(name) = index {
            if facts.length_sources.get(name) == Some(&vector_key) {
                return true;
            }
        }
    }

    // Preserve the existing get analysis here: constant propagation through a
    // name is not yet treated as a general access proof. set! may use it for
    // its append-aware rule, while get continues to require a literal or an
    // explicit path fact.
    let constant_index = if allow_append {
        integer_constant(index, facts)
    } else if let Expression::Int(index) = index {
        Some(*index)
    } else {
        None
    };
    let Some(index) = constant_index.filter(|index| *index >= 0) else {
        return false;
    };
    let index = index as usize;
    if facts
        .minimum_lengths
        .get(&vector_key)
        .is_some_and(|minimum| index < *minimum || (allow_append && index <= *minimum))
    {
        return true;
    }
    facts
        .fixed_lengths
        .get(&vector_key)
        .copied()
        .or_else(|| literal_vector_length(vector))
        .is_some_and(|len| index < len || (allow_append && index == len))
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

    if matches!(op, "/" | "mod") && items.len() == 3 {
        if !divisor_is_proven_nonzero(&items[2], facts) {
            record_diagnostic(
                diagnostics,
                format!(
                    "static arithmetic: divisor may be zero: `{}`\nhelp: guard it with `(not (= divisor 0))`",
                    expr.to_lisp()
                ),
            );
        }
        if op == "/"
            && integer_interval(&items[1], facts) == Some(IntInterval::exact(i32::MIN))
            && integer_interval(&items[2], facts) == Some(IntInterval::exact(-1))
        {
            record_diagnostic(
                diagnostics,
                format!(
                    "static arithmetic: Int overflow: `{}`\nhelp: minimum Int cannot be divided by -1",
                    expr.to_lisp()
                ),
            );
        }
    }

    if matches!(op, "+" | "-" | "*") && items.len() == 3 {
        let relational = (op != "*").then(|| affine_interval(expr, facts)).flatten();
        let ordinary = integer_interval(expr, facts);
        let refined = match (relational, ordinary) {
            (Some(relational), Some(ordinary)) => Some(IntInterval {
                min: relational.min.max(ordinary.min),
                max: relational.max.min(ordinary.max),
            }),
            (relational, ordinary) => relational.or(ordinary),
        };
        let result = refined.unwrap_or_else(|| {
            let left = integer_interval(&items[1], facts).unwrap_or(IntInterval::I32);
            let right = integer_interval(&items[2], facts).unwrap_or(IntInterval::I32);
            integer_arithmetic_interval(op, left, right)
                .expect("validated integer arithmetic operator")
        });
        let product_is_proven_safe = if op == "*" {
            let pair = canonical_product_pair(&items[1], &items[2], facts);
            product_upper_is_safe(&pair, facts) && product_lower_is_safe(&pair, facts)
        } else {
            false
        };
        if !result.fits_i32() && !product_is_proven_safe {
            let kind = match (
                result.min < IntInterval::I32.min,
                result.max > IntInterval::I32.max,
            ) {
                (true, true) => "Int overflow/underflow possible",
                (true, false) => "Int underflow possible",
                (false, true) => "Int overflow possible",
                (false, false) => unreachable!(),
            };
            record_diagnostic(
                diagnostics,
                format!(
                    "static arithmetic: {kind}: `{}`\nhelp: constrain the operands to keep the result within 32-bit Int range",
                    expr.to_lisp()
                ),
            );
        }
    }

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
        let proven = access_index_is_proven(&items[1], &items[2], facts, false);
        if !proven {
            record_diagnostic(
                diagnostics,
                format!(
                    "static bounds: index not proven safe: `{}`\nhelp: guard it with `(and (>= index 0) (< index (length xs)))`",
                    expr.to_lisp()
                ),
            );
        }
    }

    if op == "set!" && items.len() == 4 {
        let proven = access_index_is_proven(&items[1], &items[2], facts, true);
        if !proven {
            record_diagnostic(
                diagnostics,
                format!(
                    "static bounds: set! index not proven safe: `{}`\nhelp: guard replacement with `0 <= index < length`, or append at `(length xs)`",
                    expr.to_lisp()
                ),
            );
        }
    }

    if matches!(op, "car" | "pop-val!") && items.len() == 2 {
        let zero = Expression::Int(0);
        if !access_index_is_proven(&items[1], &zero, facts, false) {
            record_diagnostic(
                diagnostics,
                format!(
                    "static bounds: vector may be empty: `{}`\nhelp: guard it with `(> (length xs) 0)`",
                    expr.to_lisp()
                ),
            );
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
            // Immutable scalar facts remain valid when captured by a closure.
            // Container/liveness facts do not: the vector may be mutated
            // between closure creation and invocation.
            let mut scoped = AbstractState {
                nonnegative: facts.nonnegative.clone(),
                nonzero: facts.nonzero.clone(),
                integer_constants: facts.integer_constants.clone(),
                integer_ranges: facts.integer_ranges.clone(),
                scalar_aliases: facts.scalar_aliases.clone(),
                leq_pairs: facts.leq_pairs.clone(),
                affine_upper_bounds: facts.affine_upper_bounds.clone(),
                product_upper_safe: facts.product_upper_safe.clone(),
                product_lower_safe: facts.product_lower_safe.clone(),
                guard_summaries: facts.guard_summaries.clone(),
                predicate_summaries: facts.predicate_summaries.clone(),
                ..AbstractState::default()
            };
            for parameter in items.iter().skip(1).take(items.len().saturating_sub(2)) {
                forget_lambda_parameter(parameter, &mut scoped);
            }
            if let Some(body) = items.last().filter(|_| items.len() >= 2) {
                validate_static_bounds_expr(body, &mut scoped, diagnostics);
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
    apply_vector_mutation(items, facts);
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
    let predicate_summaries = infer_predicate_summaries(&all_expressions);
    let start = all_expressions.len().saturating_sub(user_form_count);
    let mut facts = AbstractState {
        guard_summaries,
        predicate_summaries,
        ..AbstractState::default()
    };
    // Seed facts from bundled/project library forms so public immutable
    // constants behave exactly like user constants. Library diagnostics are
    // intentionally discarded; only user forms are reported.
    let mut ignored_library_diagnostics = Vec::new();
    for expression in &all_expressions[..start] {
        validate_static_bounds_expr(expression, &mut facts, &mut ignored_library_diagnostics);
    }
    let mut diagnostics = Vec::new();
    for expression in &all_expressions[start..] {
        validate_static_bounds_expr(expression, &mut facts, &mut diagnostics);
    }
    diagnostics
}

fn substitute_predicate_body(
    expr: &Expression,
    substitutions: &HashMap<&str, &Expression>,
) -> Expression {
    match expr {
        Expression::Word(name) => substitutions
            .get(name.as_str())
            .map(|replacement| (*replacement).clone())
            .unwrap_or_else(|| expr.clone()),
        Expression::Apply(items) => Expression::Apply(
            items
                .iter()
                .map(|item| substitute_predicate_body(item, substitutions))
                .collect(),
        ),
        _ => expr.clone(),
    }
}

fn infer_predicate_summaries(expressions: &[&Expression]) -> HashMap<String, PredicateSummary> {
    let mut summaries: HashMap<String, PredicateSummary> = HashMap::new();
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
            let Some(body) = lambda.last() else {
                continue;
            };
            let params: Vec<String> = lambda[1..lambda.len() - 1]
                .iter()
                .filter_map(word)
                .map(str::to_string)
                .collect();
            if params.len() != lambda.len() - 2 {
                continue;
            }
            let summary = PredicateSummary {
                params,
                body: body.to_lisp(),
            };
            changed |= summaries.insert(name.clone(), summary.clone()) != Some(summary);
        }
        if !changed {
            break;
        }
    }
    summaries
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
            .contains("index not proven safe"));
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
            .contains("index not proven safe"));
    }

    #[test]
    fn assignment_kills_old_index_refinement() {
        let source = "(let xs [1 2]) (mut i 1) (if (and (>= i 0) (< i (length xs))) (do (alter! i -1) (get xs i)) 0)";
        assert!(analyze(source, 3)
            .expect_err("assigning the index must kill its prior proof")
            .contains("index not proven safe"));
    }

    #[test]
    fn resize_kills_a_cached_length_relation() {
        let source = "(let xs [1 2]) (let len (length xs)) (pop! xs) (mut i 0) (while (< i len) (do (let x (get xs i)) (alter! i (+ i 1))))";
        assert!(analyze(source, 5)
            .expect_err("resizing a vector must invalidate its cached length")
            .contains("index not proven safe"));
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
            .contains("index not proven safe"));
    }

    #[test]
    fn push_preserves_old_bounds_and_extends_known_length() {
        let source = "(let push! (lambda xs value (set! xs (length xs) value))) (let xs [1 2 3]) (push! xs 10) {(get xs 2) (get xs 3)}";
        assert_eq!(analyze(source, 3), Ok(()));
    }

    #[test]
    fn set_replacement_preserves_length_and_known_bounds() {
        let source = "(let xs [1 2 3]) (set! xs 1 10) (get xs 2)";
        assert_eq!(analyze(source, 3), Ok(()));
    }

    #[test]
    fn set_append_extends_known_length() {
        let source = "(let xs [1 2 3]) (set! xs (length xs) 10) (get xs 3)";
        assert_eq!(analyze(source, 3), Ok(()));
    }

    #[test]
    fn set_replacement_invalidates_nested_element_facts() {
        let source = "(let rows [[1]]) (let x 0) (let y 0) (if (and (in-bounds? rows x) (in-bounds? (get rows x) y)) (block (set! rows x []) (get rows x y)) -1)";
        let source =
            format!("(let in-bounds? (lambda xs i (and (>= i 0) (< i (length xs))))) {source}");
        assert!(analyze(&source, 5)
            .expect_err("replacing a row must invalidate facts about its old contents")
            .contains("index not proven safe"));
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

    #[test]
    fn predicate_body_substitution_preserves_false_comparison_implication() {
        let source = "(let gte? (lambda a b (>= b a))) (let xs [1 2]) (let index 1) (if (gte? (length xs) index) -1 (get xs index))";
        assert_eq!(analyze(source, 4), Ok(()));
    }

    #[test]
    fn set_requires_a_proven_replacement_or_append_index() {
        let unguarded = "(let xs [1 2]) (let i 10) (set! xs i 3)";
        assert!(analyze(unguarded, 3)
            .expect_err("unproven set! should fail")
            .contains("set! index not proven safe"));

        let guarded =
            "(let xs [1 2]) (let i 1) (if (and (>= i 0) (< i (length xs))) (set! xs i 3) nil)";
        assert_eq!(analyze(guarded, 3), Ok(()));
    }

    #[test]
    fn set_accepts_ques_append_at_length_semantics() {
        assert_eq!(analyze("(let xs []) (set! xs (length xs) 1)", 2), Ok(()));
        assert_eq!(
            analyze("(let xs [1]) (let end (length xs)) (set! xs end 2)", 3),
            Ok(())
        );
        assert_eq!(analyze("(let xs [1]) (set! xs 1 2)", 2), Ok(()));
    }

    #[test]
    fn set_rejects_negative_and_past_end_literal_indices() {
        assert!(analyze("(let xs [1]) (set! xs -1 2)", 2).is_err());
        assert!(analyze("(let xs [1]) (set! xs 2 2)", 2).is_err());
    }

    #[test]
    fn car_and_pop_val_require_a_proven_nonempty_vector() {
        assert!(analyze("(let xs []) (car xs)", 2)
            .expect_err("car of a known empty vector should fail")
            .contains("may be empty"));
        assert!(analyze("(let xs []) (pop-val! xs)", 2)
            .expect_err("pop-val! of a known empty vector should fail")
            .contains("may be empty"));

        assert_eq!(analyze("(let xs [1]) (car xs)", 2), Ok(()));
        assert_eq!(analyze("(let xs [1]) (pop-val! xs)", 2), Ok(()));
    }

    #[test]
    fn zero_argument_lambda_body_is_statically_analyzed() {
        let source = "(let push! (lambda xs x (set! xs (length xs) x))) (let history []) (let alt []) (let undo! (lambda () (push! alt (pop-val! history)))) (let redo! (lambda () (push! history (pop-val! alt))))";
        let expression = crate::parser::build(source).expect("source should parse");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        let diagnostics = analyze_user_program_diagnostics(&typed, 5);
        assert!(diagnostics.iter().any(|message| message.contains("(pop-val! history)")));
        assert!(diagnostics.iter().any(|message| message.contains("(pop-val! alt)")));
    }

    #[test]
    fn nonempty_branch_proves_car_and_pop_val_safety() {
        let car = "(let xs []) (if (> (length xs) 0) (car xs) 0)";
        assert_eq!(analyze(car, 2), Ok(()));

        let pop = "(let xs []) (if (> (length xs) 0) (pop-val! xs) 0)";
        assert_eq!(analyze(pop, 2), Ok(()));
    }

    #[test]
    fn false_zero_length_branch_proves_nonempty_vector() {
        let direct = "(let xs []) (if (= (length xs) 0) 0 (car xs))";
        assert_eq!(analyze(direct, 2), Ok(()));

        let reversed = "(let xs []) (if (= 0 (length xs)) 0 (pop-val! xs))";
        assert_eq!(analyze(reversed, 2), Ok(()));

        let negated = "(let xs []) (if (not (= (length xs) 0)) (car xs) 0)";
        assert_eq!(analyze(negated, 2), Ok(()));
    }

    #[test]
    fn recursive_car_after_empty_base_case_is_proven_safe() {
        let source = "(letrec rev (lambda (xs ys) (if (= (length xs) 0) ys (rev (cdr xs) (cons [(car xs)] ys))))) (rev [1 2 3] [])";
        assert_eq!(analyze(source, 2), Ok(()));
    }

    #[test]
    fn empty_pop_remains_a_safe_noop() {
        assert_eq!(analyze("(let xs []) (pop! xs)", 2), Ok(()));
    }

    #[test]
    fn division_and_modulo_require_a_nonzero_divisor_proof() {
        assert!(analyze("(let x 10) (let divisor 0) (/ x divisor)", 3)
            .expect_err("zero divisor should fail")
            .contains("divisor may be zero"));
        assert!(analyze("(let divide (lambda x divisor (/ x divisor)))", 1)
            .expect_err("unknown divisor should require a guard")
            .contains("divisor may be zero"));
        assert!(analyze("(mod 10 0)", 1).is_err());
        assert_eq!(analyze("(/ 10 2)", 1), Ok(()));
    }

    #[test]
    fn branch_facts_prove_division_is_nonzero() {
        let guarded = "(let divide (lambda x divisor (if (= divisor 0) 0 (/ x divisor))))";
        assert_eq!(analyze(guarded, 1), Ok(()));

        let positive = "(let divide (lambda x divisor (if (> divisor 0) (/ x divisor) 0)))";
        assert_eq!(analyze(positive, 1), Ok(()));

        let predicate = "(let nonzero? (lambda x (not (= x 0)))) (let divide (lambda x divisor (if (nonzero? divisor) (/ x divisor) 0)))";
        assert_eq!(analyze(predicate, 2), Ok(()));
    }

    #[test]
    fn known_integer_ranges_report_possible_overflow() {
        assert!(analyze("(+ 2147483647 1)", 1)
            .expect_err("literal addition overflow should fail")
            .contains("overflow"));
        assert!(analyze("(* 50000 50000)", 1)
            .expect_err("literal multiplication overflow should fail")
            .contains("overflow"));
        assert_eq!(analyze("(+ 20 22)", 1), Ok(()));
    }

    #[test]
    fn branch_ranges_can_prove_or_expose_overflow() {
        let safe = "(let add-one (lambda x (if (< x 2147483647) (+ x 1) x)))";
        assert_eq!(analyze(safe, 1), Ok(()));

        let unsafe_range = "(let add-one (lambda x (if (>= x 2147483647) (+ x 1) x)))";
        assert!(analyze(unsafe_range, 1)
            .expect_err("upper-edge range should expose overflow")
            .contains("overflow"));
    }

    #[test]
    fn relational_predicate_proves_guarded_addition_safe() {
        let safe = r#"
            (let INT-MIN -2147483648)
            (let INT-MAX 2147483647)
            (let int/add-safe?
              (lambda (a b)
                (if (> b 0)
                    (<= a (- INT-MAX b))
                    (if (< b 0)
                        (>= a (- INT-MIN b))
                        true))))
            (let guarded-add
              (lambda (a b)
                (if (int/add-safe? a b) (+ a b) a)))
        "#;
        assert_eq!(analyze(safe, 4), Ok(()));

        let wrong_upper_guard = r#"
            (let INT-MAX 2147483647)
            (let bad-add
              (lambda (a b)
                (if (and (> b 0) (<= a INT-MAX)) (+ a b) a)))
        "#;
        assert!(analyze(wrong_upper_guard, 2)
            .expect_err("an unrelated upper guard must not prove addition safe")
            .contains("overflow"));
    }

    #[test]
    fn structural_predicate_proves_guarded_multiplication_safe() {
        let direct_positive = r#"
            (let guarded-multiply
              (lambda (a b)
                (if (and (> a 0) (> b 0) (<= a (/ 2147483647 b)))
                    (* a b)
                    0)))
        "#;
        assert_eq!(analyze(direct_positive, 1), Ok(()));
        assert_eq!(analyze("(let f (lambda a b (if (and (> a 0) (< b 0) (>= b (/ -2147483648 a))) (* a b) 0)))", 1), Ok(()));
        assert_eq!(analyze("(let f (lambda a b (if (and (< a 0) (> b 0) (>= a (/ -2147483648 b))) (* a b) 0)))", 1), Ok(()));
        assert_eq!(
            analyze(
                "(let f (lambda a b (if (and (< a 0) (< b 0) (>= a (/ 2147483647 b))) (* a b) 0)))",
                1
            ),
            Ok(())
        );

        let safe = r#"
            (let INT-MIN -2147483648)
            (let INT-MAX 2147483647)
            (let multiplication-fits?
              (lambda (a b)
                (if (= a 0) true
                  (if (= b 0) true
                    (if (> a 0)
                      (if (> b 0)
                          (<= a (/ INT-MAX b))
                          (>= b (/ INT-MIN a)))
                      (if (> b 0)
                          (>= a (/ INT-MIN b))
                          (>= a (/ INT-MAX b))))))))
            (let guarded-multiply
              (lambda (a b)
                (if (multiplication-fits? a b) (* a b) 0)))
        "#;
        assert_eq!(analyze(safe, 4), Ok(()));

        let wrong_guard = r#"
            (let INT-MAX 2147483647)
            (let multiplication-fits?
              (lambda (a b) (or (= a 0) (<= a INT-MAX))))
            (let guarded-multiply
              (lambda (a b)
                (if (multiplication-fits? a b) (* a b) 0)))
        "#;
        assert!(analyze(wrong_guard, 3)
            .expect_err("an unrelated guard must not prove multiplication safe")
            .contains("overflow"));

        let invalidated = r#"
            (let guarded-multiply
              (lambda (a b)
                (block
                  (mut x a)
                  (if (and (> x 0) (> b 0) (<= x (/ 2147483647 b)))
                      (block (alter! x 2147483647) (* x b))
                      0))))
        "#;
        assert!(analyze(invalidated, 1)
            .expect_err("mutation must invalidate an earlier product proof")
            .contains("overflow"));
    }

    #[test]
    fn captured_constant_alias_refines_overflow_guard() {
        let source = "(let mi 2147483647) (let increment (lambda index (if (>= index mi) index (+ index 1))))";
        assert_eq!(analyze(source, 2), Ok(()));

        let local_capture = "(let make-increment (lambda () (let mi 2147483647) (lambda index (if (>= index mi) index (+ index 1)))))";
        assert_eq!(analyze(local_capture, 1), Ok(()));
    }

    #[test]
    fn library_constant_alias_refines_user_overflow_guard() {
        let source = "(let const/int/max-safe 2147483647) (let increment (lambda index (if (>= index const/int/max-safe) index (+ index 1))))";
        // Only `increment` is a user form; the constant models a bundled
        // standard-library definition.
        assert_eq!(analyze(source, 1), Ok(()));
    }

    #[test]
    fn unknown_integer_ranges_report_both_bounds_and_arithmetic_risks() {
        let source = "(let inspect (lambda xs left right (block (let index (/ (+ left right) 2)) (let current (get xs index)) {(+ index 1) (- index 1)})))";
        let expression = crate::parser::build(source).expect("source should parse");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        let diagnostics = analyze_user_program_diagnostics(&typed, 1);
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("index not proven safe")));
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("(+ left right)")
                && message.contains("overflow/underflow")));
        assert!(
            diagnostics
                .iter()
                .any(|message| message.contains("Int overflow possible")),
            "{diagnostics:?}"
        );
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("Int underflow possible")));
    }

    #[test]
    fn relational_ranges_prove_safe_binary_search_midpoint() {
        let source = "(let midpoint (lambda left right (if (or (> left right) (< left 0)) 0 (+ left (/ (- right left) 2)))))";
        assert_eq!(analyze(source, 1), Ok(()));

        let missing_lower_bound =
            "(let midpoint (lambda left right (if (> left right) 0 (+ left (/ (- right left) 2)))))";
        assert!(analyze(missing_lower_bound, 1)
            .expect_err("signed endpoints still need a nonnegative invariant")
            .contains("underflow"));
    }

    #[test]
    fn relational_ranges_cover_recursive_binary_search_shape() {
        let source = r#"
            (let in-bounds? (lambda xs i (and (>= i 0) (< i (length xs)))))
            (let max-safe 2147483647)
            (let search? (lambda (target xs)
              (letrec bs (lambda (left right)
                (if (or (> left right) (< left 0)) false
                  (block
                    (let index (+ left (/ (- right left) 2)))
                    (if (not (in-bounds? xs index)) false
                      (block
                        (let current (get xs index))
                        (if (= target current) true
                          (if (>= index max-safe) false
                            (if (> target current)
                              (bs (+ index 1) right)
                              (bs left (- index 1)))))))))))
              (bs 0 (- (length xs) 1))))
        "#;
        assert_eq!(analyze(source, 3), Ok(()));
    }
}

#[cfg(test)]
#[path = "static_analysis_model_tests.rs"]
mod model_tests;
