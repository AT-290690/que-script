use crate::infer::TypedExpression;
use crate::parser::Expression;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminationFinding {
    pub subject: String,
    pub status: String,
    pub measure: Option<String>,
    pub reason: String,
    pub proof: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundsProof {
    pub expression: String,
    pub details: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AnalysisNodeId {
    /// Zero-based top-level form in the user's source, excluding bundled forms.
    pub user_form_index: usize,
    /// Stable preorder ordinal among checked operations in that form.
    pub operation_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AnalysisSourcePosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AnalysisSourceSpan {
    pub start: AnalysisSourcePosition,
    pub end: AnalysisSourcePosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProofKind {
    BoundsRead,
    BoundsWrite,
    /// The write is proven to replace an existing element, rather than using
    /// Que's additional append-at-length `set!` case.
    BoundsWriteReplacement,
    NonEmpty,
    IntegerArithmetic,
    NonZeroDivisor,
    Termination,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProofStatus {
    ProvenSafe,
    DefinitelyInvalid,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticProof {
    pub id: AnalysisNodeId,
    pub kind: ProofKind,
    pub status: ProofStatus,
    pub expression: String,
    pub details: Vec<String>,
    /// Present when the normalized operation can be mapped unambiguously back
    /// to the original user source. Generated/desugared operations keep their
    /// stable node ID even when no direct source spelling exists.
    pub source_span: Option<AnalysisSourceSpan>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StaticAnalysisReport {
    pub diagnostics: Vec<StaticAnalysisDiagnostic>,
    pub proofs: Vec<StaticProof>,
}

#[derive(Default)]
struct AnalysisSink {
    diagnostics: Vec<String>,
    bounds_proofs: Vec<BoundsProof>,
    proofs: Vec<StaticProof>,
    capture_proofs: bool,
    suppress_output: bool,
    user_form_index: usize,
    next_operation_index: usize,
}

impl AnalysisSink {
    fn record_proof(
        &mut self,
        kind: ProofKind,
        status: ProofStatus,
        expression: &Expression,
        details: Vec<String>,
    ) {
        if self.suppress_output {
            return;
        }
        let id = AnalysisNodeId {
            user_form_index: self.user_form_index,
            operation_index: self.next_operation_index,
        };
        self.next_operation_index += 1;
        self.proofs.push(StaticProof {
            id,
            kind,
            status,
            expression: expression.to_lisp(),
            details,
            source_span: None,
        });
    }
}

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

#[derive(Clone, Debug)]
struct ValueSummary {
    params: Vec<String>,
    body: Expression,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecursiveRangeSummary {
    parameter_ranges: Vec<Option<IntInterval>>,
}

impl PartialEq for ValueSummary {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params && self.body.to_lisp() == other.body.to_lisp()
    }
}

impl Eq for ValueSummary {}

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

const MAX_INTERVAL_ALTERNATIVES: usize = 8;

fn normalize_intervals(mut intervals: Vec<IntInterval>) -> Vec<IntInterval> {
    intervals.retain(|interval| interval.min <= interval.max);
    intervals.sort_by_key(|interval| (interval.min, interval.max));
    let mut normalized: Vec<IntInterval> = Vec::new();
    for interval in intervals {
        if let Some(previous) = normalized.last_mut() {
            if interval.min <= previous.max.saturating_add(1) {
                previous.max = previous.max.max(interval.max);
                continue;
            }
        }
        normalized.push(interval);
    }
    if normalized.len() > MAX_INTERVAL_ALTERNATIVES {
        let min = normalized.first().expect("nonempty intervals").min;
        let max = normalized.last().expect("nonempty intervals").max;
        vec![IntInterval { min, max }]
    } else {
        normalized
    }
}

fn interval_hull(intervals: &[IntInterval]) -> Option<IntInterval> {
    Some(IntInterval {
        min: intervals.iter().map(|interval| interval.min).min()?,
        max: intervals.iter().map(|interval| interval.max).max()?,
    })
}

fn alternatives_for_key(key: &str, state: &AbstractState) -> Option<Vec<IntInterval>> {
    state
        .integer_alternatives
        .get(key)
        .cloned()
        .or_else(|| {
            state
                .integer_ranges
                .get(key)
                .copied()
                .map(|range| vec![range])
        })
        .or_else(|| {
            state
                .integer_constants
                .get(key)
                .copied()
                .map(|value| vec![IntInterval::exact(value)])
        })
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
    /// A bounded disjunction of integer intervals. This preserves holes such
    /// as `x < 0 || x > 0` that a single interval would collapse back to the
    /// entire Int domain.
    integer_alternatives: HashMap<String, Vec<IntInterval>>,
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
    value_summaries: HashMap<String, ValueSummary>,
    structural_summaries: HashMap<String, StructuralSummary>,
    recursive_range_summaries: HashMap<String, RecursiveRangeSummary>,
    nonshrinking_vectors: HashSet<String>,
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
    let integer_ranges: HashMap<String, IntInterval> = left
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
    let integer_alternatives = integer_ranges
        .keys()
        .filter_map(|name| {
            let mut alternatives = alternatives_for_key(name, left)?;
            alternatives.extend(alternatives_for_key(name, right)?);
            Some((name.clone(), normalize_intervals(alternatives)))
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
        integer_alternatives,
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
        value_summaries: left.value_summaries.clone(),
        structural_summaries: left.structural_summaries.clone(),
        recursive_range_summaries: left.recursive_range_summaries.clone(),
        nonshrinking_vectors: left.nonshrinking_vectors.clone(),
    }
}

fn widen_loop_state(previous: &AbstractState, next: &AbstractState) -> AbstractState {
    let mut widened = next.clone();
    for (name, next_range) in &next.integer_ranges {
        let previous_range = previous.integer_ranges.get(name).copied().or_else(|| {
            previous
                .integer_constants
                .get(name)
                .copied()
                .map(IntInterval::exact)
        });
        let Some(previous_range) = previous_range else {
            continue;
        };
        let range = IntInterval {
            min: if next_range.min < previous_range.min {
                i32::MIN as i64
            } else {
                next_range.min
            },
            max: if next_range.max > previous_range.max {
                i32::MAX as i64
            } else {
                next_range.max
            },
        };
        widened.integer_ranges.insert(name.clone(), range);
        widened
            .integer_alternatives
            .insert(name.clone(), vec![range]);
        if range.min != range.max {
            widened.integer_constants.remove(name);
        }
    }
    widened
}

fn apply_counted_append_postcondition(
    items: &[Expression],
    entry: &AbstractState,
    exit: &mut AbstractState,
) {
    let Some(condition) = items.get(1) else {
        return;
    };
    let Expression::Apply(comparison) = condition else {
        return;
    };
    let [Expression::Word(op), Expression::Word(counter), bound] = comparison.as_slice() else {
        return;
    };
    if !matches!(op.as_str(), "<" | "<=") {
        return;
    }
    let Some(start) = entry.integer_constants.get(counter).copied() else {
        return;
    };
    let Some(end) = integer_constant(bound, entry) else {
        return;
    };
    let iterations = i64::from(end) - i64::from(start) + i64::from(op == "<=");
    if iterations < 0 {
        return;
    }
    let mut updates = HashMap::new();
    for body in items.iter().skip(2) {
        collect_altered_values(body, &mut updates);
    }
    if updates
        .get(counter)
        .is_none_or(|values| counter_step(counter, values, entry) != CounterStep::Increase)
    {
        return;
    }
    let mut mutations = HashMap::new();
    let no_summaries = HashMap::new();
    for body in items.iter().skip(2) {
        collect_size_mutations(body, &no_summaries, &mut mutations);
    }
    for (vector, effects) in mutations {
        if effects.len() != 1 || effects[0] != SizeStep::Grow {
            continue;
        }
        let Some(initial) = entry.fixed_lengths.get(&vector).copied() else {
            continue;
        };
        let Some(final_length) = initial.checked_add(iterations as usize) else {
            continue;
        };
        exit.fixed_lengths.insert(vector.clone(), final_length);
        exit.minimum_lengths.insert(vector, final_length);
    }
}

fn refine_bounded_loop_updates(
    items: &[Expression],
    entry: &AbstractState,
    header: &mut AbstractState,
) {
    let Some(Expression::Apply(comparison)) = items.get(1) else {
        return;
    };
    let [Expression::Word(op), Expression::Word(counter), bound] = comparison.as_slice() else {
        return;
    };
    if !matches!(op.as_str(), "<" | "<=") {
        return;
    }
    let Some(start) = integer_constant(&Expression::Word(counter.clone()), entry) else {
        return;
    };
    let Some(end) = integer_constant(bound, entry) else {
        return;
    };
    let iterations = i64::from(end) - i64::from(start) + i64::from(op == "<=");
    if iterations < 0 {
        return;
    }

    let mut updates = HashMap::new();
    for body in items.iter().skip(2) {
        collect_altered_values(body, &mut updates);
    }
    for (name, values) in updates {
        let Some(initial) = integer_interval(&Expression::Word(name.clone()), entry) else {
            continue;
        };
        let mut delta_min = 0_i64;
        let mut delta_max = 0_i64;
        let mut understood = true;
        for value in values {
            let delta = match &value {
                Expression::Apply(parts) => match parts.as_slice() {
                    [Expression::Word(add), Expression::Word(var), step]
                        if add == "+" && var == &name =>
                    {
                        integer_constant(step, entry)
                    }
                    [Expression::Word(add), step, Expression::Word(var)]
                        if add == "+" && var == &name =>
                    {
                        integer_constant(step, entry)
                    }
                    [Expression::Word(sub), Expression::Word(var), step]
                        if sub == "-" && var == &name =>
                    {
                        integer_constant(step, entry).and_then(i32::checked_neg)
                    }
                    _ => None,
                },
                _ => None,
            };
            let Some(delta) = delta else {
                understood = false;
                break;
            };
            // An update may be conditional, so zero is always a possible
            // per-iteration contribution.
            delta_min += i64::from(delta.min(0));
            delta_max += i64::from(delta.max(0));
        }
        if !understood {
            continue;
        }
        let Some(minimum) = delta_min
            .checked_mul(iterations)
            .and_then(|delta| initial.min.checked_add(delta))
        else {
            continue;
        };
        let Some(maximum) = delta_max
            .checked_mul(iterations)
            .and_then(|delta| initial.max.checked_add(delta))
        else {
            continue;
        };
        let range = IntInterval {
            min: minimum,
            max: maximum,
        };
        if range.fits_i32() {
            header.integer_ranges.insert(name.clone(), range);
            header
                .integer_alternatives
                .insert(name.clone(), vec![range]);
            if range.min != range.max {
                header.integer_constants.remove(&name);
            }
        }
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

fn refined_integer_interval(expr: &Expression, state: &AbstractState) -> IntInterval {
    state
        .integer_ranges
        .get(&canonical_scalar(expr, state))
        .copied()
        .or_else(|| integer_interval(expr, state))
        .unwrap_or(IntInterval::I32)
}

fn materialize_product_expression_safety(
    left: &Expression,
    right: &Expression,
    pair: &(String, String),
    state: &mut AbstractState,
) {
    let left = refined_integer_interval(left, state);
    let right = refined_integer_interval(right, state);
    let interval = integer_arithmetic_interval("*", left, right)
        .expect("multiplication interval is supported");
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
    let divisor_range = refined_integer_interval(divisor, state);
    let pair = canonical_product_pair(smaller, divisor, state);
    if divisor_range.min > 0 && bound == i32::MAX {
        state.product_upper_safe.insert(pair.clone());
    } else if divisor_range.max < 0 && bound == i32::MIN {
        state.product_lower_safe.insert(pair.clone());
    }
    materialize_product_expression_safety(smaller, divisor, &pair, state);
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
    let divisor_range = refined_integer_interval(divisor, state);
    let pair = canonical_product_pair(larger, divisor, state);
    if divisor_range.min > 0 && bound == i32::MIN {
        state.product_lower_safe.insert(pair.clone());
    } else if divisor_range.max < 0 && bound == i32::MAX {
        state.product_upper_safe.insert(pair.clone());
    }
    materialize_product_expression_safety(larger, divisor, &pair, state);
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
                facts
                    .integer_alternatives
                    .insert(lower.clone(), vec![narrowed_lower]);
                changed = true;
            }
            if narrowed_upper != upper_range {
                facts.integer_ranges.insert(upper.clone(), narrowed_upper);
                facts
                    .integer_alternatives
                    .insert(upper.clone(), vec![narrowed_upper]);
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
    integer_interval_at_depth(expr, state, 0)
}

fn integer_interval_alternatives(
    expr: &Expression,
    state: &AbstractState,
) -> Option<Vec<IntInterval>> {
    integer_interval_alternatives_at_depth(expr, state, 0)
}

fn integer_interval_alternatives_at_depth(
    expr: &Expression,
    state: &AbstractState,
    expansion_depth: usize,
) -> Option<Vec<IntInterval>> {
    match expr {
        Expression::Int(value) => Some(vec![IntInterval::exact(*value)]),
        Expression::Word(name) => {
            let key = canonical_scalar(expr, state);
            alternatives_for_key(&key, state)
                .or_else(|| alternatives_for_key(name, state))
                .or_else(|| {
                    integer_interval_at_depth(expr, state, expansion_depth).map(|x| vec![x])
                })
        }
        Expression::Apply(items) => match items.as_slice() {
            [Expression::Word(op), left, right] if matches!(op.as_str(), "+" | "-" | "*") => {
                let left = integer_interval_alternatives_at_depth(left, state, expansion_depth)?;
                let right = integer_interval_alternatives_at_depth(right, state, expansion_depth)?;
                let mut results = Vec::new();
                for left in left {
                    for right in &right {
                        results.push(integer_arithmetic_interval(op, left, *right)?);
                    }
                }
                Some(normalize_intervals(results))
            }
            [Expression::Word(op), numerator, divisor] if op == "/" => {
                // The typed `/` builtin guarantees an Int numerator. Even
                // without a narrower symbolic fact its range is therefore
                // i32, which a constant divisor can still narrow materially.
                let numerator =
                    integer_interval_alternatives_at_depth(numerator, state, expansion_depth)
                        .unwrap_or_else(|| vec![IntInterval::I32]);
                let divisor = integer_constant(divisor, state)?;
                if divisor == 0 {
                    return None;
                }
                Some(normalize_intervals(
                    numerator
                        .into_iter()
                        .map(|range| {
                            let a = range.min / divisor as i64;
                            let b = range.max / divisor as i64;
                            IntInterval {
                                min: a.min(b),
                                max: a.max(b),
                            }
                        })
                        .collect(),
                ))
            }
            [Expression::Word(op), condition, consequent, alternate] if op == "if" => {
                match predicate_truth(condition, state, 0) {
                    Some(true) => integer_interval_alternatives_at_depth(
                        consequent,
                        &state_for_true_branch(condition, state),
                        expansion_depth,
                    ),
                    Some(false) => integer_interval_alternatives_at_depth(
                        alternate,
                        &state_for_false_branch(condition, state),
                        expansion_depth,
                    ),
                    None => {
                        let mut alternatives = integer_interval_alternatives_at_depth(
                            consequent,
                            &state_for_true_branch(condition, state),
                            expansion_depth,
                        )?;
                        alternatives.extend(integer_interval_alternatives_at_depth(
                            alternate,
                            &state_for_false_branch(condition, state),
                            expansion_depth,
                        )?);
                        Some(normalize_intervals(alternatives))
                    }
                }
            }
            [Expression::Word(op), args @ ..] if expansion_depth < 16 => {
                if let Some(summary) = state.value_summaries.get(op) {
                    if summary.params.len() == args.len() {
                        let substitutions: HashMap<&str, &Expression> = summary
                            .params
                            .iter()
                            .map(String::as_str)
                            .zip(args)
                            .collect();
                        let expanded = substitute_predicate_body(&summary.body, &substitutions);
                        return integer_interval_alternatives_at_depth(
                            &expanded,
                            state,
                            expansion_depth + 1,
                        );
                    }
                }
                integer_interval_at_depth(expr, state, expansion_depth).map(|range| vec![range])
            }
            _ => integer_interval_at_depth(expr, state, expansion_depth).map(|range| vec![range]),
        },
        Expression::Dec(_) => None,
    }
}

fn integer_interval_at_depth(
    expr: &Expression,
    state: &AbstractState,
    expansion_depth: usize,
) -> Option<IntInterval> {
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
                let left_range = integer_interval_at_depth(left, state, expansion_depth)?;
                let right_range = integer_interval_at_depth(right, state, expansion_depth)?;
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
                // `/` has already type-checked as Int arithmetic. Preserve
                // that full i32 domain when no more precise fact is known so
                // division by a constant still refines the result.
                let numerator = integer_interval_at_depth(numerator, state, expansion_depth)
                    .unwrap_or(IntInterval::I32);
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
            [Expression::Word(op), condition, consequent, alternate] if op == "if" => {
                match predicate_truth(condition, state, 0) {
                    Some(true) => integer_interval_at_depth(
                        consequent,
                        &state_for_true_branch(condition, state),
                        expansion_depth,
                    ),
                    Some(false) => integer_interval_at_depth(
                        alternate,
                        &state_for_false_branch(condition, state),
                        expansion_depth,
                    ),
                    None => {
                        let consequent = integer_interval_at_depth(
                            consequent,
                            &state_for_true_branch(condition, state),
                            expansion_depth,
                        )?;
                        let alternate = integer_interval_at_depth(
                            alternate,
                            &state_for_false_branch(condition, state),
                            expansion_depth,
                        )?;
                        Some(IntInterval {
                            min: consequent.min.min(alternate.min),
                            max: consequent.max.max(alternate.max),
                        })
                    }
                }
            }
            [Expression::Word(op), sequence @ ..] if matches!(op.as_str(), "do" | "block") => {
                let mut scoped = state.clone();
                let mut result = None;
                for item in sequence {
                    result = integer_interval_at_depth(item, &scoped, expansion_depth);
                    if let Expression::Apply(binding) = item {
                        if let [Expression::Word(bind), Expression::Word(name), value] =
                            binding.as_slice()
                        {
                            if matches!(bind.as_str(), "let" | "mut" | "alter!") {
                                assign_abstract_scalar(name, value, &mut scoped);
                            }
                        }
                    }
                }
                result
            }
            [Expression::Word(op), args @ ..] if expansion_depth < 16 => {
                if let Some(summary) = state.value_summaries.get(op) {
                    if summary.params.len() == args.len() {
                        let substitutions: HashMap<&str, &Expression> = summary
                            .params
                            .iter()
                            .map(String::as_str)
                            .zip(args)
                            .collect();
                        let expanded = substitute_predicate_body(&summary.body, &substitutions);
                        return integer_interval_at_depth(&expanded, state, expansion_depth + 1);
                    }
                }
                state
                    .integer_ranges
                    .get(&canonical_scalar(expr, state))
                    .copied()
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
        || integer_interval_alternatives(expr, state)
            .is_some_and(|alternatives| alternatives.iter().all(|range| range.excludes_zero()))
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VectorLengthInfo {
    exact: Option<usize>,
    minimum: usize,
}

fn vector_length_info(expr: &Expression, state: &AbstractState) -> Option<VectorLengthInfo> {
    vector_length_info_at_depth(expr, state, 0)
}

fn vector_length_info_at_depth(
    expr: &Expression,
    state: &AbstractState,
    expansion_depth: usize,
) -> Option<VectorLengthInfo> {
    if let Some(length) = literal_vector_length(expr) {
        return Some(VectorLengthInfo {
            exact: Some(length),
            minimum: length,
        });
    }
    match expr {
        Expression::Word(_) => {
            let key = canonical_access(expr, state);
            if let Some(length) = state.fixed_lengths.get(&key).copied() {
                return Some(VectorLengthInfo {
                    exact: Some(length),
                    minimum: length,
                });
            }
            state
                .minimum_lengths
                .get(&key)
                .copied()
                .map(|minimum| VectorLengthInfo {
                    exact: None,
                    minimum,
                })
        }
        Expression::Apply(items) => match items.as_slice() {
            [Expression::Word(op), length]
                if matches!(op.as_str(), "__vec_new_zeroed_i32" | "__vec_new_uninit_i32") =>
            {
                let length = integer_constant(length, state)?;
                let length = usize::try_from(length).ok()?;
                Some(VectorLengthInfo {
                    exact: Some(length),
                    minimum: length,
                })
            }
            [Expression::Word(op), value] if op == "cdr" => {
                let inner = vector_length_info_at_depth(value, state, expansion_depth)?;
                Some(VectorLengthInfo {
                    exact: inner.exact.map(|length| length.saturating_sub(1)),
                    minimum: inner.minimum.saturating_sub(1),
                })
            }
            [Expression::Word(op), left, right] if op == "cons" => {
                let left = vector_length_info_at_depth(left, state, expansion_depth)?;
                let right = vector_length_info_at_depth(right, state, expansion_depth)?;
                Some(VectorLengthInfo {
                    exact: left
                        .exact
                        .zip(right.exact)
                        .and_then(|(left, right)| left.checked_add(right)),
                    minimum: left.minimum.saturating_add(right.minimum),
                })
            }
            [Expression::Word(op), condition, consequent, alternate] if op == "if" => {
                let consequent = vector_length_info_at_depth(
                    consequent,
                    &state_for_true_branch(condition, state),
                    expansion_depth,
                )?;
                let alternate = vector_length_info_at_depth(
                    alternate,
                    &state_for_false_branch(condition, state),
                    expansion_depth,
                )?;
                Some(VectorLengthInfo {
                    exact: (consequent.exact == alternate.exact)
                        .then_some(consequent.exact)
                        .flatten(),
                    minimum: consequent.minimum.min(alternate.minimum),
                })
            }
            [Expression::Word(op), sequence @ ..] if matches!(op.as_str(), "do" | "block") => {
                let mut scoped = state.clone();
                let mut result = None;
                for item in sequence {
                    result = vector_length_info_at_depth(item, &scoped, expansion_depth);
                    if let Expression::Apply(binding) = item {
                        if let [Expression::Word(bind), Expression::Word(name), value] =
                            binding.as_slice()
                        {
                            if matches!(bind.as_str(), "let" | "mut" | "alter!") {
                                let length_info =
                                    vector_length_info_at_depth(value, &scoped, expansion_depth);
                                assign_abstract_scalar(name, value, &mut scoped);
                                if let Some(info) = length_info {
                                    if let Some(exact) = info.exact {
                                        scoped.fixed_lengths.insert(name.clone(), exact);
                                    }
                                    scoped.minimum_lengths.insert(name.clone(), info.minimum);
                                }
                            }
                        }
                    }
                }
                result
            }
            [Expression::Word(op), args @ ..] if expansion_depth < 16 => {
                let summary = state.value_summaries.get(op)?;
                if summary.params.len() != args.len() {
                    return None;
                }
                let substitutions: HashMap<&str, &Expression> = summary
                    .params
                    .iter()
                    .map(String::as_str)
                    .zip(args)
                    .collect();
                let expanded = substitute_predicate_body(&summary.body, &substitutions);
                vector_length_info_at_depth(&expanded, state, expansion_depth + 1)
            }
            _ => None,
        },
        Expression::Int(_) | Expression::Dec(_) => None,
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

fn known_integer_predicate(expr: &Expression, state: &AbstractState) -> Option<bool> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    let [Expression::Word(op), left, right] = items.as_slice() else {
        return None;
    };
    if !matches!(op.as_str(), "=" | ">" | ">=" | "<" | "<=") {
        return None;
    }
    let (Some(left), Some(right)) = (
        integer_interval(left, state),
        integer_interval(right, state),
    ) else {
        return None;
    };
    match op.as_str() {
        "=" if left.max < right.min || right.max < left.min => Some(false),
        "=" if left.min == left.max && left == right => Some(true),
        ">" if left.min > right.max => Some(true),
        ">" if left.max <= right.min => Some(false),
        ">=" if left.min >= right.max => Some(true),
        ">=" if left.max < right.min => Some(false),
        "<" if left.max < right.min => Some(true),
        "<" if left.min >= right.max => Some(false),
        "<=" if left.max <= right.min => Some(true),
        "<=" if left.min > right.max => Some(false),
        _ => None,
    }
}

fn constrain_integer_range(expr: &Expression, facts: &mut AbstractState, constraint: IntInterval) {
    let key = canonical_scalar(expr, facts);
    let current =
        integer_interval_alternatives(expr, facts).unwrap_or_else(|| vec![IntInterval::I32]);
    let mut narrowed = Vec::new();
    for range in current {
        let intersection = IntInterval {
            min: range.min.max(constraint.min),
            max: range.max.min(constraint.max),
        };
        if intersection.min <= intersection.max {
            narrowed.push(intersection);
        }
    }
    let narrowed = normalize_intervals(narrowed);
    if let Some(hull) = interval_hull(&narrowed) {
        facts.integer_ranges.insert(key.clone(), hull);
        facts
            .integer_alternatives
            .insert(key.clone(), narrowed.clone());
        if narrowed.iter().all(|range| range.excludes_zero()) {
            facts.nonzero.insert(key);
        }
    }
}

fn exclude_integer_value(expr: &Expression, facts: &mut AbstractState, excluded: i32) {
    let key = canonical_scalar(expr, facts);
    let current =
        integer_interval_alternatives(expr, facts).unwrap_or_else(|| vec![IntInterval::I32]);
    let excluded = i64::from(excluded);
    let mut alternatives = Vec::new();
    for range in current {
        if excluded < range.min || excluded > range.max {
            alternatives.push(range);
            continue;
        }
        if range.min < excluded {
            alternatives.push(IntInterval {
                min: range.min,
                max: excluded - 1,
            });
        }
        if excluded < range.max {
            alternatives.push(IntInterval {
                min: excluded + 1,
                max: range.max,
            });
        }
    }
    let alternatives = normalize_intervals(alternatives);
    if let Some(hull) = interval_hull(&alternatives) {
        facts.integer_ranges.insert(key.clone(), hull);
        facts
            .integer_alternatives
            .insert(key.clone(), alternatives.clone());
        if alternatives.iter().all(|range| range.excludes_zero()) {
            facts.nonzero.insert(key);
        }
    }
}

fn constrain_ordered_operands(
    left: &Expression,
    right: &Expression,
    comparison: &str,
    facts: &mut AbstractState,
) {
    let left_range = integer_interval(left, facts).unwrap_or(IntInterval::I32);
    let right_range = integer_interval(right, facts).unwrap_or(IntInterval::I32);
    match comparison {
        "<" => {
            constrain_integer_range(
                left,
                facts,
                IntInterval {
                    min: i32::MIN as i64,
                    max: right_range.max.saturating_sub(1),
                },
            );
            constrain_integer_range(
                right,
                facts,
                IntInterval {
                    min: left_range.min.saturating_add(1),
                    max: i32::MAX as i64,
                },
            );
        }
        "<=" => {
            constrain_integer_range(
                left,
                facts,
                IntInterval {
                    min: i32::MIN as i64,
                    max: right_range.max,
                },
            );
            constrain_integer_range(
                right,
                facts,
                IntInterval {
                    min: left_range.min,
                    max: i32::MAX as i64,
                },
            );
        }
        ">" => constrain_ordered_operands(right, left, "<", facts),
        ">=" => constrain_ordered_operands(right, left, "<=", facts),
        _ => {}
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
    if let Some(known) = known_integer_predicate(expr, state) {
        return (known == desired).then(|| state.clone());
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
            let known_condition = known_integer_predicate(condition, facts);
            if known_condition != Some(false) {
                let mut true_path = facts.clone();
                collect_numeric_guard_facts(condition, &mut true_path, true, expansion_depth);
                if let Some(outcome) =
                    predicate_result_state(consequent, &true_path, is_true, expansion_depth)
                {
                    outcomes.push(outcome);
                }
            }
            if known_condition != Some(true) {
                let mut false_path = facts.clone();
                collect_numeric_guard_facts(condition, &mut false_path, false, expansion_depth);
                if let Some(outcome) =
                    predicate_result_state(alternate, &false_path, is_true, expansion_depth)
                {
                    outcomes.push(outcome);
                }
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
                constrain_ordered_operands(left, right, effective, facts);
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
                "!=" => exclude_integer_value(value, facts, bound),
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

fn predicate_truth(
    expr: &Expression,
    facts: &AbstractState,
    expansion_depth: usize,
) -> Option<bool> {
    if let Expression::Word(value) = expr {
        return match value.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
    }
    if let Some(known) = known_integer_predicate(expr, facts) {
        return Some(known);
    }
    let Expression::Apply(items) = expr else {
        return None;
    };
    match items.as_slice() {
        [Expression::Word(op), inner] if op == "not" => {
            predicate_truth(inner, facts, expansion_depth).map(|value| !value)
        }
        [Expression::Word(op), operands @ ..] if op == "and" => {
            let mut unknown = false;
            for operand in operands {
                match predicate_truth(operand, facts, expansion_depth) {
                    Some(false) => return Some(false),
                    Some(true) => {}
                    None => unknown = true,
                }
            }
            (!unknown).then_some(true)
        }
        [Expression::Word(op), operands @ ..] if op == "or" => {
            let mut unknown = false;
            for operand in operands {
                match predicate_truth(operand, facts, expansion_depth) {
                    Some(true) => return Some(true),
                    Some(false) => {}
                    None => unknown = true,
                }
            }
            (!unknown).then_some(false)
        }
        [Expression::Word(op), condition, consequent, alternate] if op == "if" => {
            match predicate_truth(condition, facts, expansion_depth) {
                Some(true) => predicate_truth(consequent, facts, expansion_depth),
                Some(false) => predicate_truth(alternate, facts, expansion_depth),
                None => {
                    let consequent = predicate_truth(consequent, facts, expansion_depth);
                    let alternate = predicate_truth(alternate, facts, expansion_depth);
                    (consequent == alternate).then_some(consequent).flatten()
                }
            }
        }
        _ if expansion_depth < 16 => {
            let op = items.first().and_then(word)?;
            let summary = facts.predicate_summaries.get(op)?;
            if summary.params.len() != items.len().saturating_sub(1) {
                return None;
            }
            let parsed_body = crate::parser::build(&summary.body).ok()?;
            let substitutions: HashMap<&str, &Expression> = summary
                .params
                .iter()
                .map(String::as_str)
                .zip(items.iter().skip(1))
                .collect();
            let expanded =
                substitute_predicate_body(single_built_expression(&parsed_body), &substitutions);
            predicate_truth(&expanded, facts, expansion_depth + 1)
        }
        _ => None,
    }
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

    let Some(summary) = facts.structural_summaries.get(op).cloned() else {
        return;
    };
    if summary.params.len() != items.len().saturating_sub(1) {
        return;
    }
    for (argument, effect) in items
        .iter()
        .skip(1)
        .zip(summary.parameter_effects.iter().copied())
    {
        if effect == SizeStep::Unchanged {
            continue;
        }
        let vector = canonical_access(argument, facts);
        let nested_prefix = format!("(get {vector} ");
        facts
            .fixed_lengths
            .retain(|name, _| name != &vector && !name.starts_with(&nested_prefix));
        facts
            .length_sources
            .retain(|_, source| source != &vector && !source.starts_with(&nested_prefix));
        facts
            .safe_pairs
            .retain(|(name, _)| name != &vector && !name.starts_with(&nested_prefix));
        facts.minimum_lengths.retain(|name, _| {
            (effect == SizeStep::Grow && name == &vector)
                || (name != &vector && !name.starts_with(&nested_prefix))
        });
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
    let alternatives = integer_interval_alternatives(value, facts).map(|ranges| {
        normalize_intervals(
            ranges
                .into_iter()
                .filter(|interval| interval.fits_i32())
                .collect(),
        )
    });
    let remains_nonzero = divisor_is_proven_nonzero(value, facts);
    facts.safe_pairs.retain(|(_, index)| index != name);
    facts.nonnegative.remove(name);
    facts.integer_constants.remove(name);
    facts.integer_ranges.remove(name);
    facts.integer_alternatives.remove(name);
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
    if let Some(alternatives) = alternatives.filter(|ranges| !ranges.is_empty()) {
        facts
            .integer_alternatives
            .insert(name.to_string(), alternatives);
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
            facts.integer_alternatives.remove(name);
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

fn forget_local_name(name: &str, facts: &mut AbstractState) {
    facts.safe_pairs.retain(|(vector, index)| {
        vector != name && !vector.starts_with(&format!("{name}::")) && index != name
    });
    facts.nonnegative.remove(name);
    facts.integer_constants.remove(name);
    facts.integer_ranges.remove(name);
    facts.integer_alternatives.remove(name);
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
}

fn record_diagnostic(diagnostics: &mut AnalysisSink, message: String) {
    if diagnostics.suppress_output {
        return;
    }
    if !diagnostics.diagnostics.contains(&message) {
        diagnostics.diagnostics.push(message);
    }
}

fn collect_altered_values(expr: &Expression, out: &mut HashMap<String, Vec<Expression>>) {
    let Expression::Apply(items) = expr else {
        return;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return;
    }
    if let [Expression::Word(op), Expression::Word(name), value] = items.as_slice() {
        if op == "alter!" {
            out.entry(name.clone()).or_default().push(value.clone());
            return;
        }
    }
    for child in items.iter().skip(1) {
        collect_altered_values(child, out);
    }
}

fn simple_guard_words(expr: &Expression, out: &mut HashSet<String>) -> bool {
    match expr {
        Expression::Int(_) => true,
        Expression::Word(word) if matches!(word.as_str(), "true" | "false") => true,
        Expression::Word(word) => {
            out.insert(word.clone());
            true
        }
        Expression::Dec(_) => false,
        Expression::Apply(items) if !items.is_empty() => {
            let Some(op) = items.first().and_then(word) else {
                return false;
            };
            if !matches!(
                op,
                "and" | "or" | "not" | "=" | "<" | "<=" | ">" | ">=" | "+" | "-" | "*" | "/" | "%"
            ) {
                return false;
            }
            items
                .iter()
                .skip(1)
                .all(|child| simple_guard_words(child, out))
        }
        Expression::Apply(_) => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CounterStep {
    Increase,
    Decrease,
    Unchanged,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SizeStep {
    Shrink,
    Grow,
    Unchanged,
    Unknown,
}

/// Describes whether an update happens on some or every control-flow path
/// through an expression.  Termination needs `must_update`: merely finding an
/// update in one branch is not enough to prove that a loop makes progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PathProgress<Direction> {
    direction: Direction,
    may_update: bool,
    must_update: bool,
}

impl<Direction: Copy + PartialEq> PathProgress<Direction> {
    fn unchanged(direction: Direction) -> Self {
        Self {
            direction,
            may_update: false,
            must_update: false,
        }
    }

    fn update(direction: Direction) -> Self {
        Self {
            direction,
            may_update: true,
            must_update: true,
        }
    }

    fn merged_direction(self, other: Self, unknown: Direction) -> Direction {
        match (self.may_update, other.may_update) {
            (false, false) => self.direction,
            (true, false) => self.direction,
            (false, true) => other.direction,
            (true, true) if self.direction == other.direction => self.direction,
            (true, true) => unknown,
        }
    }

    /// Both expressions execute.  It is enough for either expression to
    /// guarantee an update, provided every possible update has one direction.
    fn then(self, other: Self, unknown: Direction) -> Self {
        Self {
            direction: self.merged_direction(other, unknown),
            may_update: self.may_update || other.may_update,
            must_update: self.must_update || other.must_update,
        }
    }

    /// Exactly one of the alternatives executes.  Progress is guaranteed only
    /// when both alternatives guarantee it.
    fn either(self, other: Self, unknown: Direction) -> Self {
        Self {
            direction: self.merged_direction(other, unknown),
            may_update: self.may_update || other.may_update,
            must_update: self.must_update && other.must_update,
        }
    }

    fn optional(mut self) -> Self {
        self.must_update = false;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StructuralSummary {
    params: Vec<String>,
    parameter_effects: Vec<SizeStep>,
    result_relations: Vec<SizeStep>,
}

fn collect_size_mutations(
    expr: &Expression,
    summaries: &HashMap<String, StructuralSummary>,
    out: &mut HashMap<String, Vec<SizeStep>>,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return;
    }
    match items.as_slice() {
        [Expression::Word(op), Expression::Word(name)]
            if matches!(op.as_str(), "pop!" | "pop-val!") =>
        {
            out.entry(name.clone()).or_default().push(SizeStep::Shrink);
        }
        [Expression::Word(op), Expression::Word(name), _] if op == "push!" => {
            out.entry(name.clone()).or_default().push(SizeStep::Grow);
        }
        _ => {}
    }
    if let Some(op) = items.first().and_then(word) {
        if let Some(summary) = summaries.get(op) {
            if summary.params.len() == items.len().saturating_sub(1) {
                for (argument, effect) in items
                    .iter()
                    .skip(1)
                    .zip(summary.parameter_effects.iter().copied())
                {
                    if let Expression::Word(name) = argument {
                        if effect != SizeStep::Unchanged {
                            out.entry(name.clone()).or_default().push(effect);
                        }
                    }
                }
            }
        }
    }
    for child in items.iter().skip(1) {
        collect_size_mutations(child, summaries, out);
    }
}

fn length_operand(expr: &Expression) -> Option<&str> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    match items.as_slice() {
        [Expression::Word(op), Expression::Word(name)] if op == "length" => Some(name),
        _ => None,
    }
}

fn size_guard_exit_direction(condition: &Expression) -> Option<(&str, SizeStep)> {
    let Expression::Apply(items) = condition else {
        return None;
    };
    let [Expression::Word(op), left, right] = items.as_slice() else {
        return None;
    };
    if !matches!(op.as_str(), "<" | "<=" | ">" | ">=") {
        return None;
    }
    if let Some(name) = length_operand(left) {
        return Some((
            name,
            if matches!(op.as_str(), ">" | ">=") {
                SizeStep::Shrink
            } else {
                SizeStep::Grow
            },
        ));
    }
    length_operand(right).map(|name| {
        (
            name,
            if matches!(op.as_str(), "<" | "<=") {
                SizeStep::Shrink
            } else {
                SizeStep::Grow
            },
        )
    })
}

fn counter_step(name: &str, updates: &[Expression], facts: &AbstractState) -> CounterStep {
    let direction = |step: &Expression| {
        let interval = integer_interval(step, facts)?;
        if interval.min > 0 {
            Some(CounterStep::Increase)
        } else if interval.max < 0 {
            Some(CounterStep::Decrease)
        } else if interval.min == 0 && interval.max == 0 {
            Some(CounterStep::Unchanged)
        } else {
            Some(CounterStep::Unknown)
        }
    };
    let classify = |value: &Expression| match value {
        Expression::Word(other) if other == name => CounterStep::Unchanged,
        Expression::Apply(items) if items.len() == 3 => match items.as_slice() {
            [Expression::Word(op), Expression::Word(var), step] if var == name && op == "+" => {
                direction(step).unwrap_or(CounterStep::Unknown)
            }
            [Expression::Word(op), step, Expression::Word(var)] if var == name && op == "+" => {
                direction(step).unwrap_or(CounterStep::Unknown)
            }
            [Expression::Word(op), Expression::Word(var), step] if var == name && op == "-" => {
                match direction(step) {
                    Some(CounterStep::Increase) => CounterStep::Decrease,
                    Some(CounterStep::Decrease) => CounterStep::Increase,
                    Some(other) => other,
                    None => CounterStep::Unknown,
                }
            }
            _ => CounterStep::Unknown,
        },
        _ => CounterStep::Unknown,
    };
    let Some(first) = updates.first().map(classify) else {
        return CounterStep::Unchanged;
    };
    updates
        .iter()
        .skip(1)
        .map(classify)
        .try_fold(first, |known, next| (known == next).then_some(known))
        .unwrap_or(CounterStep::Unknown)
}

fn counter_progress_sequence<'a>(
    expressions: impl IntoIterator<Item = &'a Expression>,
    name: &str,
    facts: &AbstractState,
) -> PathProgress<CounterStep> {
    expressions.into_iter().fold(
        PathProgress::unchanged(CounterStep::Unchanged),
        |progress, expression| {
            progress.then(
                counter_path_progress(expression, name, facts),
                CounterStep::Unknown,
            )
        },
    )
}

fn counter_path_progress(
    expr: &Expression,
    name: &str,
    facts: &AbstractState,
) -> PathProgress<CounterStep> {
    let Expression::Apply(items) = expr else {
        return PathProgress::unchanged(CounterStep::Unchanged);
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return PathProgress::unchanged(CounterStep::Unchanged);
    }

    if let [Expression::Word(op), Expression::Word(target), value] = items.as_slice() {
        if op == "alter!" && target == name {
            let nested = counter_path_progress(value, name, facts);
            let direction = if nested.may_update {
                CounterStep::Unknown
            } else {
                counter_step(name, std::slice::from_ref(value), facts)
            };
            return nested.then(PathProgress::update(direction), CounterStep::Unknown);
        }
    }

    let op = items.first().and_then(word).unwrap_or("");
    match op {
        "if" if items.len() >= 3 => {
            let condition = counter_path_progress(&items[1], name, facts);
            let otherwise = PathProgress::unchanged(CounterStep::Unchanged);
            let branch = match predicate_truth(&items[1], facts, 0) {
                Some(true) => counter_path_progress(&items[2], name, facts),
                Some(false) => items
                    .get(3)
                    .map(|expr| counter_path_progress(expr, name, facts))
                    .unwrap_or(otherwise),
                None => counter_path_progress(&items[2], name, facts).either(
                    items
                        .get(3)
                        .map(|expr| counter_path_progress(expr, name, facts))
                        .unwrap_or(otherwise),
                    CounterStep::Unknown,
                ),
            };
            condition.then(branch, CounterStep::Unknown)
        }
        "do" | "block" => counter_progress_sequence(items.iter().skip(1), name, facts),
        "and" | "or" => {
            let mut operands = items.iter().skip(1);
            let Some(first) = operands.next() else {
                return PathProgress::unchanged(CounterStep::Unchanged);
            };
            let mut progress = counter_path_progress(first, name, facts);
            for operand in operands {
                progress = progress.then(
                    counter_path_progress(operand, name, facts).optional(),
                    CounterStep::Unknown,
                );
            }
            progress
        }
        "while" => counter_progress_sequence(items.iter().skip(1), name, facts).optional(),
        _ => counter_progress_sequence(items.iter().skip(1), name, facts),
    }
}

fn size_progress_sequence<'a>(
    expressions: impl IntoIterator<Item = &'a Expression>,
    name: &str,
    summaries: &HashMap<String, StructuralSummary>,
    facts: &AbstractState,
) -> PathProgress<SizeStep> {
    expressions.into_iter().fold(
        PathProgress::unchanged(SizeStep::Unchanged),
        |progress, expression| {
            progress.then(
                size_path_progress(expression, name, summaries, facts),
                SizeStep::Unknown,
            )
        },
    )
}

fn size_path_progress(
    expr: &Expression,
    name: &str,
    summaries: &HashMap<String, StructuralSummary>,
    facts: &AbstractState,
) -> PathProgress<SizeStep> {
    let Expression::Apply(items) = expr else {
        return PathProgress::unchanged(SizeStep::Unchanged);
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return PathProgress::unchanged(SizeStep::Unchanged);
    }

    match items.as_slice() {
        [Expression::Word(op), Expression::Word(target)]
            if target == name && matches!(op.as_str(), "pop!" | "pop-val!") =>
        {
            return PathProgress::update(SizeStep::Shrink);
        }
        [Expression::Word(op), Expression::Word(target), _] if target == name && op == "push!" => {
            return PathProgress::update(SizeStep::Grow);
        }
        _ => {}
    }

    let op = items.first().and_then(word).unwrap_or("");
    match op {
        "if" if items.len() >= 3 => {
            let condition = size_path_progress(&items[1], name, summaries, facts);
            let otherwise = PathProgress::unchanged(SizeStep::Unchanged);
            let branch = match predicate_truth(&items[1], facts, 0) {
                Some(true) => size_path_progress(&items[2], name, summaries, facts),
                Some(false) => items
                    .get(3)
                    .map(|expr| size_path_progress(expr, name, summaries, facts))
                    .unwrap_or(otherwise),
                None => size_path_progress(&items[2], name, summaries, facts).either(
                    items
                        .get(3)
                        .map(|expr| size_path_progress(expr, name, summaries, facts))
                        .unwrap_or(otherwise),
                    SizeStep::Unknown,
                ),
            };
            condition.then(branch, SizeStep::Unknown)
        }
        "do" | "block" => size_progress_sequence(items.iter().skip(1), name, summaries, facts),
        "and" | "or" => {
            let mut operands = items.iter().skip(1);
            let Some(first) = operands.next() else {
                return PathProgress::unchanged(SizeStep::Unchanged);
            };
            let mut progress = size_path_progress(first, name, summaries, facts);
            for operand in operands {
                progress = progress.then(
                    size_path_progress(operand, name, summaries, facts).optional(),
                    SizeStep::Unknown,
                );
            }
            progress
        }
        "while" => size_progress_sequence(items.iter().skip(1), name, summaries, facts).optional(),
        _ => {
            let mut progress = size_progress_sequence(items.iter().skip(1), name, summaries, facts);
            if let Some(summary) = summaries.get(op) {
                if summary.params.len() == items.len().saturating_sub(1) {
                    for (argument, effect) in items
                        .iter()
                        .skip(1)
                        .zip(summary.parameter_effects.iter().copied())
                    {
                        if matches!(argument, Expression::Word(target) if target == name)
                            && effect != SizeStep::Unchanged
                        {
                            progress =
                                progress.then(PathProgress::update(effect), SizeStep::Unknown);
                        }
                    }
                }
            }
            progress
        }
    }
}

fn comparisons_for_counter<'a>(expr: &'a Expression, name: &str, out: &mut Vec<(&'a str, bool)>) {
    let Expression::Apply(items) = expr else {
        return;
    };
    if let [Expression::Word(op), left, right] = items.as_slice() {
        if matches!(op.as_str(), "<" | "<=" | ">" | ">=") {
            if matches!(left, Expression::Word(var) if var == name) {
                out.push((op, true));
            } else if matches!(right, Expression::Word(var) if var == name) {
                out.push((op, false));
            }
        }
    }
    for child in items.iter().skip(1) {
        comparisons_for_counter(child, name, out);
    }
}

fn comparison_detail_for_counter<'a>(
    expr: &'a Expression,
    name: &str,
    step: CounterStep,
) -> Option<(&'a str, bool, &'a Expression)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if let [Expression::Word(op), left, right] = items.as_slice() {
        if matches!(op.as_str(), "<" | "<=" | ">" | ">=") {
            if matches!(left, Expression::Word(var) if var == name) {
                let toward_upper = matches!(op.as_str(), "<" | "<=");
                let toward_lower = matches!(op.as_str(), ">" | ">=");
                if (toward_upper && step == CounterStep::Increase)
                    || (toward_lower && step == CounterStep::Decrease)
                {
                    return Some((op, true, right));
                }
            }
            if matches!(right, Expression::Word(var) if var == name) {
                let toward_upper = matches!(op.as_str(), ">" | ">=");
                let toward_lower = matches!(op.as_str(), "<" | "<=");
                if (toward_upper && step == CounterStep::Increase)
                    || (toward_lower && step == CounterStep::Decrease)
                {
                    return Some((op, false, left));
                }
            }
        }
    }
    items
        .iter()
        .skip(1)
        .find_map(|child| comparison_detail_for_counter(child, name, step))
}

fn contains_other_altered_scalar(
    expr: &Expression,
    counter: &str,
    altered: &HashSet<&str>,
) -> bool {
    match expr {
        Expression::Word(name) => name != counter && altered.contains(name.as_str()),
        Expression::Apply(items) => items
            .iter()
            .skip(1)
            .any(|child| contains_other_altered_scalar(child, counter, altered)),
        _ => false,
    }
}

/// Returns whether monotonic movement of `counter` is sufficient to drive the
/// condition to `target_truth`.  Conjunction needs one false operand to exit;
/// disjunction needs every operand false.  This prevents a progressing counter
/// in one arm of `or` from being mistaken for a proof of loop termination.
fn counter_drives_condition_to(
    expr: &Expression,
    counter: &str,
    step: CounterStep,
    target_truth: bool,
    altered: &HashSet<&str>,
) -> bool {
    if let Expression::Word(value) = expr {
        return matches!(value.as_str(), "true" | "false") && ((value == "true") == target_truth);
    }
    let Expression::Apply(items) = expr else {
        return false;
    };
    match items.as_slice() {
        [Expression::Word(op), inner] if op == "not" => {
            counter_drives_condition_to(inner, counter, step, !target_truth, altered)
        }
        [Expression::Word(op), operands @ ..] if op == "and" => {
            if target_truth {
                operands.iter().all(|operand| {
                    counter_drives_condition_to(operand, counter, step, true, altered)
                })
            } else {
                operands.iter().any(|operand| {
                    counter_drives_condition_to(operand, counter, step, false, altered)
                })
            }
        }
        [Expression::Word(op), operands @ ..] if op == "or" => {
            if target_truth {
                operands.iter().any(|operand| {
                    counter_drives_condition_to(operand, counter, step, true, altered)
                })
            } else {
                operands.iter().all(|operand| {
                    counter_drives_condition_to(operand, counter, step, false, altered)
                })
            }
        }
        [Expression::Word(op), left, right] if matches!(op.as_str(), "<" | "<=" | ">" | ">=") => {
            let other = if matches!(left, Expression::Word(name) if name == counter) {
                right
            } else if matches!(right, Expression::Word(name) if name == counter) {
                left
            } else {
                return false;
            };
            if contains_other_altered_scalar(other, counter, altered) {
                return false;
            }
            guard_direction_for_parameter(expr, counter, target_truth) == Some(step)
        }
        _ => false,
    }
}

fn analyze_while_termination(
    whole: &Expression,
    items: &[Expression],
    structural_summaries: &HashMap<String, StructuralSummary>,
    facts: &AbstractState,
    diagnostics: &mut AnalysisSink,
) {
    if items.len() < 3 {
        return;
    }
    let condition = &items[1];
    let loop_facts = state_for_true_branch(condition, facts);
    if matches!(condition, Expression::Word(value) if value == "true") {
        record_diagnostic(
            diagnostics,
            format!(
                "termination: loop has no program-controlled exit: `{}`",
                whole.to_lisp()
            ),
        );
        return;
    }
    let mut updates = HashMap::new();
    for body in items.iter().skip(2) {
        collect_altered_values(body, &mut updates);
    }
    let mut guard_words = HashSet::new();
    if simple_guard_words(condition, &mut guard_words)
        && guard_words.iter().all(|name| !updates.contains_key(name))
    {
        record_diagnostic(
            diagnostics,
            format!(
                "termination: loop condition cannot change once entered: `{}`",
                condition.to_lisp()
            ),
        );
        return;
    }
    let mut scalar_progress_proven = false;
    let altered_names: HashSet<&str> = updates.keys().map(String::as_str).collect();
    for name in updates.keys() {
        let mut comparisons = Vec::new();
        comparisons_for_counter(condition, name, &mut comparisons);
        if comparisons.is_empty() {
            continue;
        }
        let progress = counter_progress_sequence(items.iter().skip(2), name, &loop_facts);
        let step = progress.direction;
        let progresses = progress.must_update
            && counter_drives_condition_to(condition, name, step, false, &altered_names);
        let moves_away = progress.must_update
            && counter_drives_condition_to(condition, name, step, true, &altered_names);
        scalar_progress_proven |= progresses;
        if !progresses && progress.must_update && (moves_away || step == CounterStep::Unchanged) {
            record_diagnostic(
                diagnostics,
                format!(
                    "termination: loop counter '{}' does not move toward its exit bound: `{}`",
                    name,
                    condition.to_lisp()
                ),
            );
        } else if !progresses {
            record_diagnostic(
                diagnostics,
                format!(
                    "termination: loop counter '{}' is not guaranteed to move toward its exit bound: `{}`",
                    name,
                    condition.to_lisp()
                ),
            );
        }
    }
    if let Some((name, expected)) = size_guard_exit_direction(condition) {
        let progress = size_progress_sequence(
            items.iter().skip(2),
            name,
            structural_summaries,
            &loop_facts,
        );
        let actual = progress.direction;
        let moves_away = matches!(
            (expected, actual),
            (SizeStep::Shrink, SizeStep::Grow) | (SizeStep::Grow, SizeStep::Shrink)
        );
        // When a scalar counter moves toward a fixed length bound, the length
        // is a bound rather than the measure. If both sides move in opposite
        // directions, their relative rates are currently unknown, so do not
        // claim either termination or non-termination.
        if !scalar_progress_proven && ((progress.must_update && moves_away) || !progress.may_update)
        {
            record_diagnostic(
                diagnostics,
                format!(
                    "termination: length of '{}' does not move toward the loop exit: `{}`",
                    name,
                    condition.to_lisp()
                ),
            );
        } else if !scalar_progress_proven && (!progress.must_update || actual == SizeStep::Unknown)
        {
            record_diagnostic(
                diagnostics,
                format!(
                    "termination: length of '{}' is not guaranteed to move toward the loop exit: `{}`",
                    name,
                    condition.to_lisp()
                ),
            );
        }
    }
}

fn contains_unchanged_recursive_call(expr: &Expression, function: &str, params: &[String]) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return false;
    }
    if matches!(items.first(), Some(Expression::Word(name)) if name == function)
        && items.len() == params.len() + 1
        && items
            .iter()
            .skip(1)
            .zip(params)
            .all(|(arg, param)| matches!(arg, Expression::Word(name) if name == param))
    {
        return true;
    }
    items
        .iter()
        .skip(1)
        .any(|child| contains_unchanged_recursive_call(child, function, params))
}

fn contains_recursive_call(expr: &Expression, function: &str) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return false;
    }
    matches!(items.first(), Some(Expression::Word(name)) if name == function)
        || items
            .iter()
            .skip(1)
            .any(|child| contains_recursive_call(child, function))
}

fn contains_if(expr: &Expression) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    matches!(items.first(), Some(Expression::Word(op)) if op == "if")
        || items.iter().skip(1).any(contains_if)
}

fn recursive_argument_step(
    arg: &Expression,
    parameter: &str,
    facts: &AbstractState,
) -> CounterStep {
    counter_step(parameter, std::slice::from_ref(arg), facts)
}

fn recursive_argument_size_step(
    arg: &Expression,
    parameter: &str,
    summaries: &HashMap<String, StructuralSummary>,
) -> SizeStep {
    match arg {
        Expression::Word(name) if name == parameter => SizeStep::Unchanged,
        Expression::Apply(items) => match items.as_slice() {
            [Expression::Word(op), Expression::Word(name)] if op == "cdr" && name == parameter => {
                SizeStep::Shrink
            }
            [Expression::Word(op), Expression::Word(name), _]
                if op == "cdr" && name == parameter =>
            {
                SizeStep::Shrink
            }
            [Expression::Word(op), left, right]
                if op == "cons"
                    && (matches!(left, Expression::Word(name) if name == parameter)
                        || matches!(right, Expression::Word(name) if name == parameter)) =>
            {
                SizeStep::Grow
            }
            _ => {
                let Some(op) = items.first().and_then(word) else {
                    return SizeStep::Unknown;
                };
                let Some(summary) = summaries.get(op) else {
                    return SizeStep::Unknown;
                };
                if summary.params.len() != items.len().saturating_sub(1) {
                    return SizeStep::Unknown;
                }
                summary
                    .result_relations
                    .iter()
                    .copied()
                    .zip(items.iter().skip(1))
                    .find_map(|(relation, argument)| {
                        matches!(argument, Expression::Word(name) if name == parameter)
                            .then_some(relation)
                    })
                    .unwrap_or(SizeStep::Unknown)
            }
        },
        _ => SizeStep::Unknown,
    }
}

fn length_base_case(condition: &Expression, parameter: &str) -> bool {
    let Expression::Apply(items) = condition else {
        return false;
    };
    match items.as_slice() {
        [Expression::Word(op), left, Expression::Int(bound)]
            if matches!(op.as_str(), "=" | "<=" | "<")
                && *bound <= 1
                && length_operand(left) == Some(parameter) =>
        {
            true
        }
        [Expression::Word(op), Expression::Int(bound), right]
            if matches!(op.as_str(), "=" | ">=" | ">")
                && *bound <= 1
                && length_operand(right) == Some(parameter) =>
        {
            true
        }
        _ => false,
    }
}

fn recursive_calls_fail_to_shrink(
    expr: &Expression,
    function: &str,
    params: &[String],
    shrinking_param: usize,
    summaries: &HashMap<String, StructuralSummary>,
) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return false;
    }
    if matches!(items.first(), Some(Expression::Word(name)) if name == function)
        && items.len() == params.len() + 1
    {
        return recursive_argument_size_step(
            &items[shrinking_param + 1],
            &params[shrinking_param],
            summaries,
        ) != SizeStep::Shrink;
    }
    items.iter().skip(1).any(|child| {
        recursive_calls_fail_to_shrink(child, function, params, shrinking_param, summaries)
    })
}

fn guard_direction_for_parameter(
    condition: &Expression,
    parameter: &str,
    target_truth: bool,
) -> Option<CounterStep> {
    let Expression::Apply(items) = condition else {
        return None;
    };
    let [Expression::Word(op), left, right] = items.as_slice() else {
        return None;
    };
    if !matches!(op.as_str(), "<" | "<=" | ">" | ">=") {
        return None;
    }
    let parameter_on_left = if matches!(left, Expression::Word(name) if name == parameter) {
        true
    } else if matches!(right, Expression::Word(name) if name == parameter) {
        false
    } else {
        return None;
    };
    let less_relation = matches!(op.as_str(), "<" | "<=");
    let increase_makes_true = if parameter_on_left {
        !less_relation
    } else {
        less_relation
    };
    let increase_is_target = if target_truth {
        increase_makes_true
    } else {
        !increase_makes_true
    };
    Some(if increase_is_target {
        CounterStep::Increase
    } else {
        CounterStep::Decrease
    })
}

fn recursive_calls_move_away_from_guard(
    expr: &Expression,
    function: &str,
    params: &[String],
    condition: &Expression,
    recurse_when_true: bool,
    facts: &AbstractState,
) -> bool {
    let Expression::Apply(items) = expr else {
        return false;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return false;
    }
    if matches!(items.first(), Some(Expression::Word(name)) if name == function)
        && items.len() == params.len() + 1
    {
        // The sibling branch is the exit path, so recursive progress must move
        // the guard toward the opposite truth value.
        let target_truth = !recurse_when_true;
        return params.iter().enumerate().any(|(index, parameter)| {
            let Some(expected) = guard_direction_for_parameter(condition, parameter, target_truth)
            else {
                return false;
            };
            let actual = recursive_argument_step(&items[index + 1], parameter, facts);
            matches!(
                (expected, actual),
                (CounterStep::Increase, CounterStep::Decrease)
                    | (CounterStep::Decrease, CounterStep::Increase)
            )
        });
    }
    items.iter().skip(1).any(|child| {
        recursive_calls_move_away_from_guard(
            child,
            function,
            params,
            condition,
            recurse_when_true,
            facts,
        )
    })
}

fn recursive_calls_all_move_toward_guard(
    expr: &Expression,
    function: &str,
    params: &[String],
    parameter_index: usize,
    expected: CounterStep,
    facts: &AbstractState,
    found: &mut bool,
) -> bool {
    let Expression::Apply(items) = expr else {
        return true;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return true;
    }
    if matches!(items.first(), Some(Expression::Word(name)) if name == function)
        && items.len() == params.len() + 1
    {
        *found = true;
        return recursive_argument_step(
            &items[parameter_index + 1],
            &params[parameter_index],
            facts,
        ) == expected;
    }
    items.iter().skip(1).all(|child| {
        recursive_calls_all_move_toward_guard(
            child,
            function,
            params,
            parameter_index,
            expected,
            facts,
            found,
        )
    })
}

fn analyze_recursive_progress(
    body: &Expression,
    function: &str,
    params: &[String],
    structural_summaries: &HashMap<String, StructuralSummary>,
    facts: &AbstractState,
    diagnostics: &mut AnalysisSink,
) {
    let Expression::Apply(items) = body else {
        return;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "do" || op == "block") {
        for expression in items.iter().skip(1) {
            analyze_recursive_progress(
                expression,
                function,
                params,
                structural_summaries,
                facts,
                diagnostics,
            );
        }
        return;
    }
    if matches!(items.first(), Some(Expression::Word(name)) if name == function) {
        record_diagnostic(
            diagnostics,
            format!(
                "termination: recursive call to '{}' has no conditional exit path: `{}`",
                function,
                body.to_lisp()
            ),
        );
        return;
    }
    if matches!(items.first(), Some(Expression::Word(op)) if op == "if") && items.len() >= 3 {
        let then_recurses = contains_recursive_call(&items[2], function);
        let else_recurses = items
            .get(3)
            .is_some_and(|branch| contains_recursive_call(branch, function));
        if then_recurses ^ else_recurses {
            let (recursive_branch, recurse_when_true) = if then_recurses {
                (&items[2], true)
            } else {
                (&items[3], false)
            };
            let recursive_facts = state_for_branch(&items[1], facts, recurse_when_true);
            if recursive_calls_move_away_from_guard(
                recursive_branch,
                function,
                params,
                &items[1],
                recurse_when_true,
                &recursive_facts,
            ) {
                record_diagnostic(
                    diagnostics,
                    format!(
                        "termination: recursive call to '{}' moves away from its base-case guard: `{}`",
                        function,
                        recursive_branch.to_lisp()
                    ),
                );
            }
            for (index, parameter) in params.iter().enumerate() {
                let Some(expected) =
                    guard_direction_for_parameter(&items[1], parameter, !recurse_when_true)
                else {
                    continue;
                };
                let mut found = false;
                if !recursive_calls_all_move_toward_guard(
                    recursive_branch,
                    function,
                    params,
                    index,
                    expected,
                    &recursive_facts,
                    &mut found,
                ) {
                    record_diagnostic(
                        diagnostics,
                        format!(
                            "termination: not every recursive call to '{}' moves '{}' toward its base-case guard: `{}`",
                            function,
                            parameter,
                            recursive_branch.to_lisp()
                        ),
                    );
                }
            }
            if !recurse_when_true {
                for (index, parameter) in params.iter().enumerate() {
                    if length_base_case(&items[1], parameter)
                        && recursive_calls_fail_to_shrink(
                            recursive_branch,
                            function,
                            params,
                            index,
                            structural_summaries,
                        )
                    {
                        record_diagnostic(
                            diagnostics,
                            format!(
                                "termination: recursive call to '{}' does not shrink '{}': `{}`",
                                function,
                                parameter,
                                recursive_branch.to_lisp()
                            ),
                        );
                    }
                }
            }
            return;
        }
    }
    // A top-level sequence or arithmetic wrapper containing recursion but no
    // guarding branch repeats unconditionally.
    if contains_recursive_call(body, function) && !contains_if(body) {
        record_diagnostic(
            diagnostics,
            format!(
                "termination: recursion in '{}' has no conditional exit path: `{}`",
                function,
                body.to_lisp()
            ),
        );
    }
}

fn analyze_termination_expr(
    expr: &Expression,
    structural_summaries: &HashMap<String, StructuralSummary>,
    facts: &AbstractState,
    diagnostics: &mut AnalysisSink,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    let op = items.first().and_then(word).unwrap_or("");
    if op == "while" {
        analyze_while_termination(expr, items, structural_summaries, facts, diagnostics);
    }
    if op == "letrec" && items.len() == 3 {
        if let (Expression::Word(name), Expression::Apply(lambda)) = (&items[1], &items[2]) {
            if matches!(lambda.first(), Some(Expression::Word(head)) if head == "lambda")
                && lambda.len() >= 2
            {
                let params = lambda[1..lambda.len() - 1]
                    .iter()
                    .filter_map(|param| word(param).map(str::to_string))
                    .collect::<Vec<_>>();
                if params.len() == lambda.len() - 2
                    && contains_unchanged_recursive_call(
                        lambda.last().expect("lambda body exists"),
                        name,
                        &params,
                    )
                {
                    record_diagnostic(
                        diagnostics,
                        format!(
                            "termination: recursive call to '{}' repeats all arguments unchanged: `{}`",
                            name,
                            expr.to_lisp()
                        ),
                    );
                }
                if params.len() == lambda.len() - 2 {
                    analyze_recursive_progress(
                        lambda.last().expect("lambda body exists"),
                        name,
                        &params,
                        structural_summaries,
                        facts,
                        diagnostics,
                    );
                }
            }
        }
    }
    for child in items.iter().skip(1) {
        analyze_termination_expr(child, structural_summaries, facts, diagnostics);
    }
}

#[derive(Clone)]
struct RecursiveDefinition {
    params: Vec<String>,
    body: Expression,
}

#[derive(Clone, Default)]
struct RecursiveEntryEvidence {
    ranges: Vec<Option<IntInterval>>,
    unknown: Vec<bool>,
    found: bool,
    escaped: bool,
}

fn collect_recursive_definitions(
    expressions: &[&Expression],
) -> HashMap<String, RecursiveDefinition> {
    expressions
        .iter()
        .filter_map(|expression| {
            let Expression::Apply(binding) = expression else {
                return None;
            };
            let [Expression::Word(keyword), Expression::Word(name), Expression::Apply(lambda)] =
                binding.as_slice()
            else {
                return None;
            };
            if keyword != "letrec"
                || !matches!(lambda.first(), Some(Expression::Word(op)) if op == "lambda")
                || lambda.len() < 2
            {
                return None;
            }
            let params = lambda[1..lambda.len() - 1]
                .iter()
                .filter_map(word)
                .map(str::to_string)
                .collect::<Vec<_>>();
            (params.len() == lambda.len() - 2).then(|| {
                (
                    name.clone(),
                    RecursiveDefinition {
                        params,
                        body: lambda.last().expect("lambda body exists").clone(),
                    },
                )
            })
        })
        .collect()
}

fn top_level_scalar_facts(expressions: &[&Expression]) -> AbstractState {
    let mut facts = AbstractState::default();
    for expression in expressions {
        let Expression::Apply(binding) = expression else {
            continue;
        };
        let [Expression::Word(keyword), Expression::Word(name), value] = binding.as_slice() else {
            continue;
        };
        if keyword == "let"
            && !matches!(value, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda"))
        {
            assign_abstract_scalar(name, value, &mut facts);
        }
    }
    facts
}

fn merge_entry_range(slot: &mut Option<IntInterval>, range: IntInterval) {
    *slot = Some(match *slot {
        Some(known) => IntInterval {
            min: known.min.min(range.min),
            max: known.max.max(range.max),
        },
        None => range,
    });
}

fn collect_recursive_entry_evidence(
    expr: &Expression,
    definitions: &HashMap<String, RecursiveDefinition>,
    current_definition: Option<&str>,
    facts: &AbstractState,
    evidence: &mut HashMap<String, RecursiveEntryEvidence>,
) {
    let Expression::Apply(items) = expr else {
        if let Expression::Word(name) = expr {
            if definitions.contains_key(name) {
                evidence.entry(name.clone()).or_default().escaped = true;
            }
        }
        return;
    };
    let head = items.first().and_then(word);
    if head == Some("letrec") && items.len() == 3 {
        let nested_name = word(&items[1]);
        collect_recursive_entry_evidence(&items[2], definitions, nested_name, facts, evidence);
        return;
    }
    if let Some(function) = head.filter(|name| definitions.contains_key(*name)) {
        if current_definition != Some(function) {
            let definition = &definitions[function];
            let entry =
                evidence
                    .entry(function.to_string())
                    .or_insert_with(|| RecursiveEntryEvidence {
                        ranges: vec![None; definition.params.len()],
                        unknown: vec![false; definition.params.len()],
                        ..RecursiveEntryEvidence::default()
                    });
            entry.found = true;
            if items.len() != definition.params.len() + 1 {
                entry.unknown.fill(true);
            } else {
                for (index, argument) in items.iter().skip(1).enumerate() {
                    if let Some(range) = integer_interval(argument, facts) {
                        merge_entry_range(&mut entry.ranges[index], range);
                    } else {
                        entry.unknown[index] = true;
                    }
                }
            }
        }
        for argument in items.iter().skip(1) {
            collect_recursive_entry_evidence(
                argument,
                definitions,
                current_definition,
                facts,
                evidence,
            );
        }
        return;
    }
    for child in items.iter().skip(1) {
        collect_recursive_entry_evidence(child, definitions, current_definition, facts, evidence);
    }
}

fn comparison_for_parameter<'a>(
    condition: &'a Expression,
    parameter: &str,
) -> Option<(&'a str, &'a Expression)> {
    let Expression::Apply(items) = condition else {
        return None;
    };
    let [Expression::Word(op), left, right] = items.as_slice() else {
        return None;
    };
    if !matches!(op.as_str(), "=" | "<" | "<=" | ">" | ">=") {
        return None;
    }
    if matches!(left, Expression::Word(name) if name == parameter) {
        return Some((op, right));
    }
    if matches!(right, Expression::Word(name) if name == parameter) {
        let reversed = match op.as_str() {
            "=" => "=",
            "<" => ">",
            "<=" => ">=",
            ">" => "<",
            ">=" => "<=",
            _ => unreachable!(),
        };
        return Some((reversed, left));
    }
    None
}

fn find_recursive_parameter_guard<'a>(
    expr: &'a Expression,
    function: &str,
    parameter: &str,
) -> Option<(&'a Expression, bool)> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return None;
    }
    if matches!(items.first(), Some(Expression::Word(op)) if op == "if") && items.len() >= 3 {
        let then_recurses = contains_recursive_call(&items[2], function);
        let else_recurses = items
            .get(3)
            .is_some_and(|branch| contains_recursive_call(branch, function));
        if then_recurses ^ else_recurses {
            let recurse_when_true = then_recurses;
            if comparison_for_parameter(&items[1], parameter).is_some() {
                return Some((&items[1], recurse_when_true));
            }
            let recursive_branch = if recurse_when_true {
                &items[2]
            } else {
                &items[3]
            };
            if let Some(found) =
                find_recursive_parameter_guard(recursive_branch, function, parameter)
            {
                return Some(found);
            }
        }
    }
    items
        .iter()
        .skip(1)
        .find_map(|child| find_recursive_parameter_guard(child, function, parameter))
}

fn recursive_unit_step(
    argument: &Expression,
    parameter: &str,
    facts: &AbstractState,
) -> Option<CounterStep> {
    let Expression::Apply(items) = argument else {
        return None;
    };
    match items.as_slice() {
        [Expression::Word(op), Expression::Word(name), step]
            if name == parameter && op == "+" && integer_constant(step, facts) == Some(1) =>
        {
            Some(CounterStep::Increase)
        }
        [Expression::Word(op), step, Expression::Word(name)]
            if name == parameter && op == "+" && integer_constant(step, facts) == Some(1) =>
        {
            Some(CounterStep::Increase)
        }
        [Expression::Word(op), Expression::Word(name), step]
            if name == parameter && op == "-" && integer_constant(step, facts) == Some(1) =>
        {
            Some(CounterStep::Decrease)
        }
        _ => None,
    }
}

fn all_recursive_calls_have_unit_step(
    expr: &Expression,
    function: &str,
    params: &[String],
    parameter_index: usize,
    facts: &AbstractState,
    found: &mut bool,
    direction: &mut Option<CounterStep>,
) -> bool {
    let Expression::Apply(items) = expr else {
        return true;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return true;
    }
    if matches!(items.first(), Some(Expression::Word(name)) if name == function)
        && items.len() == params.len() + 1
    {
        *found = true;
        let Some(step) =
            recursive_unit_step(&items[parameter_index + 1], &params[parameter_index], facts)
        else {
            return false;
        };
        if direction.is_some_and(|known| known != step) {
            return false;
        }
        *direction = Some(step);
        return true;
    }
    items.iter().skip(1).all(|child| {
        all_recursive_calls_have_unit_step(
            child,
            function,
            params,
            parameter_index,
            facts,
            found,
            direction,
        )
    })
}

fn comparison_truth(op: &str, left: i64, right: i64) -> bool {
    match op {
        "=" => left == right,
        "<" => left < right,
        "<=" => left <= right,
        ">" => left > right,
        ">=" => left >= right,
        _ => false,
    }
}

fn recursive_exit_value(
    op: &str,
    bound: i64,
    base_truth: bool,
    direction: CounterStep,
) -> Option<i64> {
    let delta = match direction {
        CounterStep::Increase => 1,
        CounterStep::Decrease => -1,
        CounterStep::Unchanged | CounterStep::Unknown => return None,
    };
    [bound - 1, bound, bound + 1]
        .into_iter()
        .find(|candidate| {
            comparison_truth(op, *candidate, bound) == base_truth
                && comparison_truth(op, *candidate - delta, bound) != base_truth
        })
        .filter(|value| (i32::MIN as i64..=i32::MAX as i64).contains(value))
}

fn infer_recursive_range_summaries(
    expressions: &[&Expression],
) -> HashMap<String, RecursiveRangeSummary> {
    let definitions = collect_recursive_definitions(expressions);
    if definitions.is_empty() {
        return HashMap::new();
    }
    let facts = top_level_scalar_facts(expressions);
    let mut evidence = definitions
        .iter()
        .map(|(name, definition)| {
            (
                name.clone(),
                RecursiveEntryEvidence {
                    ranges: vec![None; definition.params.len()],
                    unknown: vec![false; definition.params.len()],
                    ..RecursiveEntryEvidence::default()
                },
            )
        })
        .collect::<HashMap<_, _>>();
    for expression in expressions {
        collect_recursive_entry_evidence(expression, &definitions, None, &facts, &mut evidence);
    }
    definitions
        .iter()
        .filter_map(|(function, definition)| {
            let entries = evidence.get(function)?;
            if !entries.found || entries.escaped {
                return None;
            }
            let parameter_ranges = definition
                .params
                .iter()
                .enumerate()
                .map(|(index, parameter)| {
                    if entries.unknown[index] {
                        return None;
                    }
                    let entry = entries.ranges[index]?;
                    let (condition, recurse_when_true) =
                        find_recursive_parameter_guard(&definition.body, function, parameter)?;
                    let (op, bound_expr) = comparison_for_parameter(condition, parameter)?;
                    let bound = integer_constant(bound_expr, &facts)? as i64;
                    let mut found = false;
                    let mut direction = None;
                    if !all_recursive_calls_have_unit_step(
                        &definition.body,
                        function,
                        &definition.params,
                        index,
                        &facts,
                        &mut found,
                        &mut direction,
                    ) || !found
                    {
                        return None;
                    }
                    let direction = direction?;
                    let exit = recursive_exit_value(op, bound, !recurse_when_true, direction)?;
                    match direction {
                        CounterStep::Increase if entry.max <= exit => Some(IntInterval {
                            min: entry.min,
                            max: exit,
                        }),
                        CounterStep::Decrease if entry.min >= exit => Some(IntInterval {
                            min: exit,
                            max: entry.max,
                        }),
                        _ => None,
                    }
                })
                .collect::<Vec<_>>();
            parameter_ranges
                .iter()
                .any(Option::is_some)
                .then(|| (function.clone(), RecursiveRangeSummary { parameter_ranges }))
        })
        .collect()
}

fn collect_shrinking_vectors(
    expr: &Expression,
    facts: &AbstractState,
    structural_summaries: &HashMap<String, StructuralSummary>,
    shrinking: &mut HashSet<String>,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    let Some(op) = items.first().and_then(word) else {
        return;
    };
    if matches!(op, "pop!" | "pop-val!" | "pull!") {
        if let Some(target) = items.get(1) {
            shrinking.insert(canonical_access(target, facts));
        }
    }
    if let Some(summary) = structural_summaries.get(op) {
        if summary.params.len() == items.len().saturating_sub(1) {
            for (argument, effect) in items
                .iter()
                .skip(1)
                .zip(summary.parameter_effects.iter().copied())
            {
                if matches!(effect, SizeStep::Shrink | SizeStep::Unknown) {
                    shrinking.insert(canonical_access(argument, facts));
                }
            }
        }
    }
    for child in items.iter().skip(1) {
        collect_shrinking_vectors(child, facts, structural_summaries, shrinking);
    }
}

fn infer_nonshrinking_vectors(
    expressions: &[&Expression],
    value_summaries: &HashMap<String, ValueSummary>,
    predicate_summaries: &HashMap<String, PredicateSummary>,
) -> HashSet<String> {
    let structural_summaries = infer_structural_summaries(expressions, predicate_summaries);
    let mut facts = AbstractState {
        value_summaries: value_summaries.clone(),
        predicate_summaries: predicate_summaries.clone(),
        ..AbstractState::default()
    };
    let mut candidates = HashSet::new();
    for expression in expressions {
        let Expression::Apply(binding) = expression else {
            continue;
        };
        let [Expression::Word(keyword), Expression::Word(name), value] = binding.as_slice() else {
            continue;
        };
        if keyword != "let" {
            continue;
        }
        let length_info = vector_length_info(value, &facts);
        let alias = match value {
            Expression::Word(_) => Some(canonical_access(value, &facts)),
            Expression::Apply(rhs) if matches!(rhs.first(), Some(Expression::Word(op)) if op == "get") => {
                Some(canonical_access(value, &facts))
            }
            _ => None,
        };
        assign_abstract_scalar(name, value, &mut facts);
        if let Some(info) = length_info {
            if let Some(exact) = info.exact {
                facts.fixed_lengths.insert(name.clone(), exact);
            }
            facts.minimum_lengths.insert(name.clone(), info.minimum);
            candidates.insert(name.clone());
        }
        if let Some(alias) = alias {
            facts.aliases.insert(name.clone(), alias);
        }
    }
    let mut shrinking = HashSet::new();
    for expression in expressions {
        collect_shrinking_vectors(expression, &facts, &structural_summaries, &mut shrinking);
    }
    candidates
        .into_iter()
        .filter(|name| {
            !shrinking.contains(&canonical_access(&Expression::Word(name.clone()), &facts))
        })
        .collect()
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

    let known_length = facts
        .fixed_lengths
        .get(&vector_key)
        .copied()
        .or_else(|| literal_vector_length(vector));
    let index_constant = integer_constant(index, facts);
    let index_has_widened_range = index_constant.is_some_and(|constant| {
        integer_interval(index, facts).is_some_and(|range| {
            range.min != i64::from(constant) || range.max != i64::from(constant)
        })
    });
    if index_constant.is_none() || index_has_widened_range {
        if let (Some(index_ranges), Some(minimum)) = (
            integer_interval_alternatives(index, facts),
            facts.minimum_lengths.get(&vector_key).copied(),
        ) {
            let maximum = if allow_append {
                minimum
            } else {
                minimum.saturating_sub(1)
            };
            if index_ranges.iter().all(|index_range| {
                index_range.min >= 0
                    && (minimum > 0 || allow_append)
                    && index_range.max <= maximum as i64
            }) {
                return true;
            }
        }
        if let (Some(index_ranges), Some(length)) =
            (integer_interval_alternatives(index, facts), known_length)
        {
            let maximum = if allow_append {
                length
            } else {
                length.saturating_sub(1)
            };
            if index_ranges.iter().all(|index_range| {
                index_range.min >= 0
                    && (length > 0 || allow_append)
                    && index_range.max <= maximum as i64
            }) {
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

fn access_index_status(
    vector: &Expression,
    index: &Expression,
    facts: &AbstractState,
    allow_append: bool,
) -> ProofStatus {
    if access_index_is_proven(vector, index, facts, allow_append) {
        return ProofStatus::ProvenSafe;
    }
    let vector_key = canonical_access(vector, facts);
    let known_length = facts
        .fixed_lengths
        .get(&vector_key)
        .copied()
        .or_else(|| literal_vector_length(vector));
    let Some(index_ranges) = integer_interval_alternatives(index, facts) else {
        return ProofStatus::Unknown;
    };
    if index_ranges.iter().all(|range| range.max < 0) {
        return ProofStatus::DefinitelyInvalid;
    }
    let Some(length) = known_length else {
        return ProofStatus::Unknown;
    };
    let largest_valid = if allow_append {
        length as i64
    } else if length == 0 {
        -1
    } else {
        length.saturating_sub(1) as i64
    };
    if index_ranges
        .iter()
        .all(|range| range.max < 0 || range.min > largest_valid)
    {
        ProofStatus::DefinitelyInvalid
    } else {
        ProofStatus::Unknown
    }
}

fn access_proof_details(
    vector: &Expression,
    index: &Expression,
    facts: &AbstractState,
) -> Vec<String> {
    let vector_key = canonical_access(vector, facts);
    let mut details = Vec::new();
    if let Some(length) = facts
        .fixed_lengths
        .get(&vector_key)
        .copied()
        .or_else(|| literal_vector_length(vector))
    {
        details.push(format!("length({vector_key}) = {length}"));
    } else if let Some(minimum) = facts.minimum_lengths.get(&vector_key) {
        details.push(format!("length({vector_key}) >= {minimum}"));
    }
    if let Some(ranges) = integer_interval_alternatives(index, facts) {
        details.push(format!(
            "index range = {}",
            ranges
                .iter()
                .map(|range| format!("{}..{}", range.min, range.max))
                .collect::<Vec<_>>()
                .join(" or ")
        ));
    }
    details
}

fn validate_static_bounds_expr(
    expr: &Expression,
    facts: &mut AbstractState,
    diagnostics: &mut AnalysisSink,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    let op = items.first().and_then(word).unwrap_or("");

    if matches!(op, "/" | "%") && items.len() == 3 {
        let divisor_alternatives = integer_interval_alternatives(&items[2], facts);
        let divisor_status = if divisor_is_proven_nonzero(&items[2], facts) {
            ProofStatus::ProvenSafe
        } else if divisor_alternatives.as_ref().is_some_and(|alternatives| {
            alternatives
                .iter()
                .all(|range| *range == IntInterval::exact(0))
        }) {
            ProofStatus::DefinitelyInvalid
        } else {
            ProofStatus::Unknown
        };
        diagnostics.record_proof(
            ProofKind::NonZeroDivisor,
            divisor_status,
            expr,
            divisor_alternatives
                .map(|ranges| {
                    vec![format!(
                        "divisor range = {}",
                        ranges
                            .iter()
                            .map(|range| format!("{}..{}", range.min, range.max))
                            .collect::<Vec<_>>()
                            .join(" or ")
                    )]
                })
                .unwrap_or_default(),
        );
        if divisor_status != ProofStatus::ProvenSafe {
            let certainty = if divisor_status == ProofStatus::DefinitelyInvalid {
                "divisor may be zero (definitely zero)"
            } else {
                "divisor may be zero"
            };
            record_diagnostic(
                diagnostics,
                format!(
                    "static arithmetic: {certainty}: `{}`\nhelp: guard it with `(not (= divisor 0))`",
                    expr.to_lisp()
                ),
            );
        }
        if op == "/" {
            let numerator_ranges = integer_interval_alternatives(&items[1], facts)
                .unwrap_or_else(|| vec![IntInterval::I32]);
            let divisor_ranges = integer_interval_alternatives(&items[2], facts)
                .unwrap_or_else(|| vec![IntInterval::I32]);
            let contains =
                |range: &IntInterval, value: i64| range.min <= value && value <= range.max;
            let overflow_possible = numerator_ranges
                .iter()
                .any(|range| contains(range, i32::MIN as i64))
                && divisor_ranges.iter().any(|range| contains(range, -1));
            let overflow_certain = numerator_ranges
                .iter()
                .all(|range| *range == IntInterval::exact(i32::MIN))
                && divisor_ranges
                    .iter()
                    .all(|range| *range == IntInterval::exact(-1));
            let overflow_status = if overflow_certain {
                ProofStatus::DefinitelyInvalid
            } else if overflow_possible {
                ProofStatus::Unknown
            } else {
                ProofStatus::ProvenSafe
            };
            diagnostics.record_proof(
                ProofKind::IntegerArithmetic,
                overflow_status,
                expr,
                vec!["minimum Int divided by -1 is the only signed division overflow".to_string()],
            );
            if overflow_possible {
                let certainty = if overflow_certain {
                    "Int overflow"
                } else {
                    "Int overflow possible"
                };
                record_diagnostic(
                    diagnostics,
                    format!(
                        "static arithmetic: {certainty}: `{}`\nhelp: minimum Int cannot be divided by -1",
                        expr.to_lisp()
                    ),
                );
            }
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
        let result_alternatives = normalize_intervals(
            integer_interval_alternatives(expr, facts)
                .unwrap_or_else(|| vec![result])
                .into_iter()
                .filter_map(|range| {
                    let intersection = IntInterval {
                        min: range.min.max(result.min),
                        max: range.max.min(result.max),
                    };
                    (intersection.min <= intersection.max).then_some(intersection)
                })
                .collect(),
        );
        let result_alternatives = if result_alternatives.is_empty() {
            vec![result]
        } else {
            result_alternatives
        };
        let product_is_proven_safe = if op == "*" {
            let pair = canonical_product_pair(&items[1], &items[2], facts);
            product_upper_is_safe(&pair, facts) && product_lower_is_safe(&pair, facts)
        } else {
            false
        };
        let all_fit = result_alternatives.iter().all(|range| range.fits_i32());
        let all_invalid = result_alternatives
            .iter()
            .all(|range| range.max < IntInterval::I32.min || range.min > IntInterval::I32.max);
        let arithmetic_status = if all_fit || product_is_proven_safe {
            ProofStatus::ProvenSafe
        } else if all_invalid {
            ProofStatus::DefinitelyInvalid
        } else {
            ProofStatus::Unknown
        };
        diagnostics.record_proof(
            ProofKind::IntegerArithmetic,
            arithmetic_status,
            expr,
            vec![format!("result range = {}..{}", result.min, result.max)],
        );
        if !all_fit && !product_is_proven_safe {
            let kind = match (
                result.min < IntInterval::I32.min,
                result.max > IntInterval::I32.max,
            ) {
                (true, true) => "Int overflow/underflow possible",
                (true, false) => "Int underflow possible",
                (false, true) => "Int overflow possible",
                (false, false) => unreachable!(),
            };
            let left_range = integer_interval(&items[1], facts).unwrap_or(IntInterval::I32);
            let right_range = integer_interval(&items[2], facts).unwrap_or(IntInterval::I32);
            let operand_detail =
                if canonical_scalar(&items[1], facts) == canonical_scalar(&items[2], facts) {
                    format!(
                        "\ndetail: inferred range: {} <= {} <= {}",
                        left_range.min,
                        canonical_scalar(&items[1], facts),
                        left_range.max
                    )
                } else {
                    format!(
                        "\ndetail: left range: {}..{}\ndetail: right range: {}..{}",
                        left_range.min, left_range.max, right_range.min, right_range.max
                    )
                };
            let safe_detail = if op == "*"
                && canonical_scalar(&items[1], facts) == canonical_scalar(&items[2], facts)
            {
                "\ndetail: safe square range: -46340..46340"
            } else {
                ""
            };
            record_diagnostic(
                diagnostics,
                format!(
                    "static arithmetic: {kind}: `{}`{operand_detail}{safe_detail}\nhelp: constrain the operands to keep the result within 32-bit Int range",
                    expr.to_lisp(),
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
        let status = access_index_status(&items[1], &items[2], facts, false);
        diagnostics.record_proof(
            ProofKind::BoundsRead,
            status,
            expr,
            access_proof_details(&items[1], &items[2], facts),
        );
        if status != ProofStatus::ProvenSafe {
            let certainty = if status == ProofStatus::DefinitelyInvalid {
                "index not proven safe (definitely out of bounds)"
            } else {
                "index not proven safe"
            };
            record_diagnostic(
                diagnostics,
                format!(
                    "static bounds: {certainty}: `{}`\nhelp: guard it with `(and (>= index 0) (< index (length xs)))`",
                    expr.to_lisp()
                ),
            );
        } else if diagnostics.capture_proofs {
            let vector = canonical_access(&items[1], facts);
            let index = canonical_scalar(&items[2], facts);
            let mut details = Vec::new();
            if let Some(length) = facts
                .fixed_lengths
                .get(&vector)
                .copied()
                .or_else(|| literal_vector_length(&items[1]))
            {
                details.push(format!("length({vector}) = {length}"));
            } else if let Some(minimum) = facts.minimum_lengths.get(&vector) {
                details.push(format!("length({vector}) >= {minimum}"));
            }
            if let Some(range) = integer_interval(&items[2], facts) {
                details.push(format!("{} <= {} <= {}", range.min, index, range.max));
            } else if facts.safe_pairs.contains(&(vector.clone(), index.clone())) {
                details.push(format!("0 <= {index} < length({vector})"));
            }
            diagnostics.bounds_proofs.push(BoundsProof {
                expression: expr.to_lisp(),
                details,
            });
        }
    }

    if op == "set!" && items.len() == 4 {
        let status = access_index_status(&items[1], &items[2], facts, true);
        let replacement_status = access_index_status(&items[1], &items[2], facts, false);
        diagnostics.record_proof(
            ProofKind::BoundsWrite,
            status,
            expr,
            access_proof_details(&items[1], &items[2], facts),
        );
        diagnostics.record_proof(
            ProofKind::BoundsWriteReplacement,
            replacement_status,
            expr,
            access_proof_details(&items[1], &items[2], facts),
        );
        if status != ProofStatus::ProvenSafe {
            let certainty = if status == ProofStatus::DefinitelyInvalid {
                "set! index not proven safe (definitely out of bounds)"
            } else {
                "set! index not proven safe"
            };
            record_diagnostic(
                diagnostics,
                format!(
                    "static bounds: {certainty}: `{}`\nhelp: guard replacement with `0 <= index < length`, or append at `(length xs)`",
                    expr.to_lisp()
                ),
            );
        }
    }

    if matches!(op, "car" | "pop-val!") && items.len() == 2 {
        let zero = Expression::Int(0);
        let status = access_index_status(&items[1], &zero, facts, false);
        diagnostics.record_proof(
            ProofKind::NonEmpty,
            status,
            expr,
            access_proof_details(&items[1], &zero, facts),
        );
        if status != ProofStatus::ProvenSafe {
            let certainty = if status == ProofStatus::DefinitelyInvalid {
                "vector may be empty (definitely empty)"
            } else {
                "vector may be empty"
            };
            record_diagnostic(
                diagnostics,
                format!(
                    "static bounds: {certainty}: `{}`\nhelp: guard it with `(> (length xs) 0)`",
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
            let mut local_names = Vec::new();
            for child in items.iter().skip(1) {
                if let Expression::Apply(binding) = child {
                    if matches!(binding.first(), Some(Expression::Word(op)) if matches!(op.as_str(), "let" | "mut" | "letrec"))
                    {
                        if let Some(Expression::Word(name)) = binding.get(1) {
                            local_names.push(name.clone());
                        }
                    }
                }
                validate_static_bounds_expr(child, &mut scoped, diagnostics);
            }
            for name in local_names {
                forget_local_name(&name, &mut scoped);
            }
            *facts = scoped;
        }
        "letrec" if items.len() == 3 => {
            let (Expression::Word(function), Expression::Apply(lambda)) = (&items[1], &items[2])
            else {
                validate_static_bounds_expr(&items[2], facts, diagnostics);
                apply_vector_mutation(items, facts);
                return;
            };
            if !matches!(lambda.first(), Some(Expression::Word(op)) if op == "lambda")
                || lambda.len() < 2
            {
                validate_static_bounds_expr(&items[2], facts, diagnostics);
                apply_vector_mutation(items, facts);
                return;
            }
            let summary = facts.recursive_range_summaries.get(function).cloned();
            let captured_minimum_lengths = facts
                .minimum_lengths
                .iter()
                .filter(|(name, _)| facts.nonshrinking_vectors.contains(*name))
                .map(|(name, length)| (name.clone(), *length))
                .collect();
            let captured_aliases = facts
                .aliases
                .iter()
                .filter(|(name, _)| facts.nonshrinking_vectors.contains(*name))
                .map(|(name, target)| (name.clone(), target.clone()))
                .collect();
            // Immutable scalar facts remain valid when captured by a closure.
            // Container/liveness facts do not: the vector may be mutated
            // between closure creation and invocation.
            let mut scoped = AbstractState {
                nonnegative: facts.nonnegative.clone(),
                nonzero: facts.nonzero.clone(),
                integer_constants: facts.integer_constants.clone(),
                integer_ranges: facts.integer_ranges.clone(),
                integer_alternatives: facts.integer_alternatives.clone(),
                scalar_aliases: facts.scalar_aliases.clone(),
                leq_pairs: facts.leq_pairs.clone(),
                affine_upper_bounds: facts.affine_upper_bounds.clone(),
                product_upper_safe: facts.product_upper_safe.clone(),
                product_lower_safe: facts.product_lower_safe.clone(),
                minimum_lengths: captured_minimum_lengths,
                aliases: captured_aliases,
                guard_summaries: facts.guard_summaries.clone(),
                predicate_summaries: facts.predicate_summaries.clone(),
                value_summaries: facts.value_summaries.clone(),
                structural_summaries: facts.structural_summaries.clone(),
                recursive_range_summaries: facts.recursive_range_summaries.clone(),
                nonshrinking_vectors: facts.nonshrinking_vectors.clone(),
                ..AbstractState::default()
            };
            for parameter in lambda.iter().skip(1).take(lambda.len().saturating_sub(2)) {
                forget_lambda_parameter(parameter, &mut scoped);
            }
            if let Some(summary) = summary {
                for (parameter, range) in lambda
                    .iter()
                    .skip(1)
                    .take(lambda.len().saturating_sub(2))
                    .zip(summary.parameter_ranges)
                {
                    let (Expression::Word(parameter), Some(range)) = (parameter, range) else {
                        continue;
                    };
                    scoped.integer_ranges.insert(parameter.clone(), range);
                    scoped
                        .integer_alternatives
                        .insert(parameter.clone(), vec![range]);
                    if range.min >= 0 {
                        scoped.nonnegative.insert(parameter.clone());
                    }
                    if range.excludes_zero() {
                        scoped.nonzero.insert(parameter.clone());
                    }
                }
            }
            if let Some(body) = lambda.last() {
                validate_static_bounds_expr(body, &mut scoped, diagnostics);
            }
        }
        "lambda" => {
            // Immutable scalar facts remain valid when captured by a closure.
            // Container/liveness facts do not: the vector may be mutated
            // between closure creation and invocation.
            let captured_minimum_lengths = facts
                .minimum_lengths
                .iter()
                .filter(|(name, _)| facts.nonshrinking_vectors.contains(*name))
                .map(|(name, length)| (name.clone(), *length))
                .collect();
            let captured_aliases = facts
                .aliases
                .iter()
                .filter(|(name, _)| facts.nonshrinking_vectors.contains(*name))
                .map(|(name, target)| (name.clone(), target.clone()))
                .collect();
            let mut scoped = AbstractState {
                nonnegative: facts.nonnegative.clone(),
                nonzero: facts.nonzero.clone(),
                integer_constants: facts.integer_constants.clone(),
                integer_ranges: facts.integer_ranges.clone(),
                integer_alternatives: facts.integer_alternatives.clone(),
                scalar_aliases: facts.scalar_aliases.clone(),
                leq_pairs: facts.leq_pairs.clone(),
                affine_upper_bounds: facts.affine_upper_bounds.clone(),
                product_upper_safe: facts.product_upper_safe.clone(),
                product_lower_safe: facts.product_lower_safe.clone(),
                minimum_lengths: captured_minimum_lengths,
                aliases: captured_aliases,
                guard_summaries: facts.guard_summaries.clone(),
                predicate_summaries: facts.predicate_summaries.clone(),
                value_summaries: facts.value_summaries.clone(),
                structural_summaries: facts.structural_summaries.clone(),
                recursive_range_summaries: facts.recursive_range_summaries.clone(),
                nonshrinking_vectors: facts.nonshrinking_vectors.clone(),
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
                let length_info = vector_length_info(&items[2], facts);
                assign_abstract_scalar(name, &items[2], facts);
                if let Some(info) = length_info {
                    if let Some(length) = info.exact {
                        facts.fixed_lengths.insert(name.clone(), length);
                    }
                    facts.minimum_lengths.insert(name.clone(), info.minimum);
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
            let known = predicate_truth(&items[1], facts, 0);
            let mut consequent = state_for_true_branch(&items[1], facts);
            if known != Some(false) {
                validate_static_bounds_expr(&items[2], &mut consequent, diagnostics);
            }
            let mut alternate = state_for_false_branch(&items[1], facts);
            if known != Some(true) {
                if let Some(otherwise) = items.get(3) {
                    validate_static_bounds_expr(otherwise, &mut alternate, diagnostics);
                }
            }
            // Branch refinements remain local even when this pass can decide
            // which branch is reachable. This keeps guard facts from being
            // treated as unconditional facts by later expressions.
            *facts = join_states(&consequent, &alternate);
        }
        "while" if items.len() >= 3 => {
            // Compute a loop-header fixed point.  The intersection join is the
            // widening for this finite fact domain, so convergence is quick;
            // the cap is only a defensive guard against future domains.
            let entry = facts.clone();
            let mut header = entry.clone();
            for _ in 0..16 {
                let previous = header.clone();
                // Fixed-point iterations are speculative. Diagnostics emitted
                // before widening stabilizes would describe an intermediate
                // state rather than the actual loop invariant.
                let mut speculative_diagnostics = AnalysisSink {
                    suppress_output: true,
                    ..AnalysisSink::default()
                };
                validate_static_bounds_expr(&items[1], &mut header, &mut speculative_diagnostics);
                let mut body_exit = state_for_true_branch(&items[1], &header);
                for child in items.iter().skip(2) {
                    validate_static_bounds_expr(
                        child,
                        &mut body_exit,
                        &mut speculative_diagnostics,
                    );
                }
                let next = join_states(&entry, &body_exit);
                let next = widen_loop_state(&previous, &next);
                if next == previous {
                    header = next;
                    break;
                }
                header = next;
            }
            refine_bounded_loop_updates(items, &entry, &mut header);
            // Check the loop once using the stabilized invariant.
            validate_static_bounds_expr(&items[1], &mut header, diagnostics);
            let mut checked_body = state_for_true_branch(&items[1], &header);
            for child in items.iter().skip(2) {
                validate_static_bounds_expr(child, &mut checked_body, diagnostics);
            }
            // The loop may execute zero times; only header facts are valid on
            // entry, but code following it is reached only through the false
            // condition edge.
            let mut exit = state_for_false_branch(&items[1], &header);
            apply_counted_append_postcondition(items, &entry, &mut exit);
            *facts = exit;
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
    analyze_user_program_diagnostics_detailed(typed_program, user_form_count)
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticAnalysisDiagnostic {
    pub message: String,
    /// Zero-based index in the user's top-level forms, excluding bundled and
    /// project-library forms.
    pub user_form_index: usize,
}

pub fn analyze_user_program_diagnostics_detailed(
    typed_program: &TypedExpression,
    user_form_count: usize,
) -> Vec<StaticAnalysisDiagnostic> {
    analyze_user_program_report(typed_program, user_form_count).diagnostics
}

pub fn analyze_user_program_report(
    typed_program: &TypedExpression,
    user_form_count: usize,
) -> StaticAnalysisReport {
    let all_expressions: Vec<&Expression> = match &typed_program.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "do") => {
            items.iter().skip(1).collect()
        }
        expression => vec![expression],
    };
    let guard_summaries = infer_guard_summaries(&all_expressions);
    let predicate_summaries = infer_predicate_summaries(&all_expressions);
    let value_summaries = infer_value_summaries(&all_expressions);
    let recursive_range_summaries = infer_recursive_range_summaries(&all_expressions);
    let nonshrinking_vectors =
        infer_nonshrinking_vectors(&all_expressions, &value_summaries, &predicate_summaries);
    let structural_summaries = infer_structural_summaries(&all_expressions, &predicate_summaries);
    let start = all_expressions.len().saturating_sub(user_form_count);
    let mut facts = AbstractState {
        guard_summaries,
        predicate_summaries,
        value_summaries,
        structural_summaries: structural_summaries.clone(),
        recursive_range_summaries,
        nonshrinking_vectors,
        ..AbstractState::default()
    };
    // Seed facts from bundled/project library forms so public immutable
    // constants behave exactly like user constants. Library diagnostics are
    // intentionally discarded; only user forms are reported.
    let mut ignored_library_diagnostics = AnalysisSink::default();
    for expression in &all_expressions[..start] {
        validate_static_bounds_expr(expression, &mut facts, &mut ignored_library_diagnostics);
    }
    let mut report = StaticAnalysisReport::default();
    for (user_form_index, expression) in all_expressions[start..].iter().enumerate() {
        let mut diagnostics = AnalysisSink {
            user_form_index,
            ..AnalysisSink::default()
        };
        let entry_facts = facts.clone();
        validate_static_bounds_expr(expression, &mut facts, &mut diagnostics);
        analyze_termination_expr(
            expression,
            &structural_summaries,
            &entry_facts,
            &mut diagnostics,
        );
        let mut termination = Vec::new();
        collect_termination_findings(
            expression,
            &structural_summaries,
            &entry_facts,
            &mut termination,
        );
        for finding in termination {
            let status = match finding.status.as_str() {
                "proven" => ProofStatus::ProvenSafe,
                "warning"
                    if finding.reason.contains("no program-controlled exit")
                        || finding.reason.contains("condition cannot change") =>
                {
                    ProofStatus::DefinitelyInvalid
                }
                _ => ProofStatus::Unknown,
            };
            diagnostics.record_proof(
                ProofKind::Termination,
                status,
                expression,
                std::iter::once(finding.reason)
                    .chain(finding.proof)
                    .collect(),
            );
        }
        report
            .diagnostics
            .extend(
                diagnostics
                    .diagnostics
                    .into_iter()
                    .map(|message| StaticAnalysisDiagnostic {
                        message,
                        user_form_index,
                    }),
            );
        report.proofs.extend(diagnostics.proofs);
    }
    report
}

/// Proof-only analysis for lowering. It deliberately skips termination and
/// diagnostic/source presentation work, keeping optimized compilation close
/// to one abstract-interpretation pass over the program.
pub fn analyze_codegen_proofs(typed_program: &TypedExpression) -> Vec<StaticProof> {
    let all_expressions: Vec<&Expression> = match &typed_program.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "do") => {
            items.iter().skip(1).collect()
        }
        expression => vec![expression],
    };
    let guard_summaries = infer_guard_summaries(&all_expressions);
    let predicate_summaries = infer_predicate_summaries(&all_expressions);
    let value_summaries = infer_value_summaries(&all_expressions);
    let recursive_range_summaries = infer_recursive_range_summaries(&all_expressions);
    let nonshrinking_vectors =
        infer_nonshrinking_vectors(&all_expressions, &value_summaries, &predicate_summaries);
    let structural_summaries = infer_structural_summaries(&all_expressions, &predicate_summaries);
    let mut facts = AbstractState {
        guard_summaries,
        predicate_summaries,
        value_summaries,
        structural_summaries,
        recursive_range_summaries,
        nonshrinking_vectors,
        ..AbstractState::default()
    };
    let mut proofs = Vec::new();
    for (user_form_index, expression) in all_expressions.into_iter().enumerate() {
        let mut sink = AnalysisSink {
            user_form_index,
            ..AnalysisSink::default()
        };
        validate_static_bounds_expr(expression, &mut facts, &mut sink);
        proofs.extend(sink.proofs);
    }
    proofs
}

/// Runs the analysis and maps proof nodes back to their original source when
/// possible. Mapping is deliberately a presentation step: the proof identity
/// remains valid for compiler consumers that do not have source text.
pub fn analyze_user_program_report_with_source(
    typed_program: &TypedExpression,
    user_form_count: usize,
    source: &str,
) -> StaticAnalysisReport {
    let mut report = analyze_user_program_report(typed_program, user_form_count);
    attach_source_spans(source, &mut report);
    report
}

pub fn attach_source_spans(source: &str, report: &mut StaticAnalysisReport) {
    let mut occurrences: HashMap<(u32, u32, String), usize> = HashMap::new();
    for proof in &mut report.proofs {
        let form_start = crate::lsp_native_core::source_form_range_for_desugared_index(
            source,
            proof.id.user_form_index,
        )
        .map(|range| (range.start.line, range.start.character))
        .unwrap_or((u32::MAX, proof.id.user_form_index as u32));
        let key = (form_start.0, form_start.1, proof.expression.clone());
        let occurrence = occurrences.entry(key).or_default();
        let synthetic_message = format!(
            "static bounds: index not proven safe: `{}`",
            proof.expression
        );
        let ranges = crate::lsp_native_core::static_analysis_diagnostic_ranges(
            source,
            &synthetic_message,
            proof.id.user_form_index,
        );
        let range = ranges
            .get(*occurrence)
            .copied()
            .or_else(|| (ranges.len() == 1).then(|| ranges[0]));
        if let Some(range) = range {
            proof.source_span = Some(AnalysisSourceSpan {
                start: AnalysisSourcePosition {
                    line: range.start.line,
                    character: range.start.character,
                },
                end: AnalysisSourcePosition {
                    line: range.end.line,
                    character: range.end.character,
                },
            });
        }
        *occurrence += 1;
    }
}

pub fn explain_bounds_proofs(
    typed_program: &TypedExpression,
    user_form_count: usize,
) -> Vec<BoundsProof> {
    let all_expressions: Vec<&Expression> = match &typed_program.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "do") => {
            items.iter().skip(1).collect()
        }
        expression => vec![expression],
    };
    let guard_summaries = infer_guard_summaries(&all_expressions);
    let predicate_summaries = infer_predicate_summaries(&all_expressions);
    let value_summaries = infer_value_summaries(&all_expressions);
    let recursive_range_summaries = infer_recursive_range_summaries(&all_expressions);
    let nonshrinking_vectors =
        infer_nonshrinking_vectors(&all_expressions, &value_summaries, &predicate_summaries);
    let structural_summaries = infer_structural_summaries(&all_expressions, &predicate_summaries);
    let start = all_expressions.len().saturating_sub(user_form_count);
    let mut facts = AbstractState {
        guard_summaries,
        predicate_summaries,
        value_summaries,
        structural_summaries,
        recursive_range_summaries,
        nonshrinking_vectors,
        ..AbstractState::default()
    };
    let mut sink = AnalysisSink::default();
    for expression in &all_expressions[..start] {
        validate_static_bounds_expr(expression, &mut facts, &mut sink);
    }
    sink.capture_proofs = true;
    for expression in &all_expressions[start..] {
        validate_static_bounds_expr(expression, &mut facts, &mut sink);
    }
    sink.bounds_proofs
}

fn first_recursive_call<'a>(expr: &'a Expression, function: &str) -> Option<&'a [Expression]> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return None;
    }
    if matches!(items.first(), Some(Expression::Word(name)) if name == function) {
        return Some(items);
    }
    items
        .iter()
        .skip(1)
        .find_map(|child| first_recursive_call(child, function))
}

fn collect_termination_findings(
    expr: &Expression,
    structural_summaries: &HashMap<String, StructuralSummary>,
    facts: &AbstractState,
    findings: &mut Vec<TerminationFinding>,
) {
    let Expression::Apply(items) = expr else {
        return;
    };
    let op = items.first().and_then(word).unwrap_or("");
    if op == "while" && items.len() >= 3 {
        let condition = &items[1];
        let subject = format!("while {}", condition.to_lisp());
        let mut diagnostics = AnalysisSink::default();
        analyze_while_termination(expr, items, structural_summaries, facts, &mut diagnostics);
        if let Some(reason) = diagnostics
            .diagnostics
            .into_iter()
            .find(|message| message.starts_with("termination:"))
        {
            findings.push(TerminationFinding {
                subject,
                status: "warning".to_string(),
                measure: None,
                reason: reason.trim_start_matches("termination: ").to_string(),
                proof: Vec::new(),
            });
        } else {
            let mut updates = HashMap::new();
            for body in items.iter().skip(2) {
                collect_altered_values(body, &mut updates);
            }
            let loop_facts = state_for_true_branch(condition, facts);
            let altered_names: HashSet<&str> = updates.keys().map(String::as_str).collect();
            let scalar_proof = updates.keys().find_map(|name| {
                let progress = counter_progress_sequence(items.iter().skip(2), name, &loop_facts);
                let step = progress.direction;
                (progress.must_update
                    && counter_drives_condition_to(condition, name, step, false, &altered_names))
                .then(|| {
                    (
                        name.clone(),
                        if step == CounterStep::Increase {
                            "increases"
                        } else {
                            "decreases"
                        },
                        step,
                    )
                })
            });
            let competing_bound =
                size_guard_exit_direction(condition).is_some_and(|(name, expected)| {
                    let progress = size_progress_sequence(
                        items.iter().skip(2),
                        name,
                        structural_summaries,
                        &loop_facts,
                    );
                    matches!(
                        (expected, progress.direction),
                        (SizeStep::Shrink, SizeStep::Grow) | (SizeStep::Grow, SizeStep::Shrink)
                    )
                });
            let structural_proof =
                size_guard_exit_direction(condition).and_then(|(name, expected)| {
                    let progress = size_progress_sequence(
                        items.iter().skip(2),
                        name,
                        structural_summaries,
                        &loop_facts,
                    );
                    (progress.must_update && progress.direction == expected)
                        .then(|| name.to_string())
                });
            if competing_bound && scalar_proof.is_some() {
                findings.push(TerminationFinding {
                    subject,
                    status: "unknown".to_string(),
                    measure: None,
                    reason: "counter and length bound move in competing directions; relative progress is unknown"
                        .to_string(),
                    proof: vec![format!("condition: {}", condition.to_lisp())],
                });
            } else if let Some((name, direction, step)) = scalar_proof {
                let mut proof = Vec::new();
                if let Some(initial) = integer_constant(&Expression::Word(name.clone()), facts) {
                    proof.push(format!("initial: {name} = {initial}"));
                }
                proof.push(format!("condition: {}", condition.to_lisp()));
                if let Some(update) = updates.get(&name).and_then(|values| values.first()) {
                    proof.push(format!("update: {name} = {}", update.to_lisp()));
                }
                if let Some((_, _, bound)) = comparison_detail_for_counter(condition, &name, step) {
                    proof.push(format!("bound: {}", bound.to_lisp()));
                }
                findings.push(TerminationFinding {
                    subject,
                    status: "proven".to_string(),
                    measure: Some(name.clone()),
                    reason: format!("{} {} toward the exit bound", name, direction),
                    proof,
                });
            } else if let Some(name) = structural_proof {
                findings.push(TerminationFinding {
                    subject,
                    status: "proven".to_string(),
                    measure: Some(format!("length({name})")),
                    reason: format!("length({name}) moves toward the exit bound"),
                    proof: vec![format!("condition: {}", condition.to_lisp())],
                });
            } else {
                findings.push(TerminationFinding {
                    subject,
                    status: "unknown".to_string(),
                    measure: None,
                    reason: "no monotonic measure was inferred".to_string(),
                    proof: Vec::new(),
                });
            }
        }
    }
    if op == "while" && items.len() >= 3 {
        // A nested loop is inspected as if the enclosing loop may already
        // have completed earlier iterations. Do not carry its entry-time
        // constants (for example `i = 0`) into every body visit.
        let mut header = facts.clone();
        let mut loop_updates = HashMap::new();
        for body in items.iter().skip(2) {
            collect_altered_values(body, &mut loop_updates);
        }
        for name in loop_updates.keys() {
            forget_local_name(name, &mut header);
        }
        let mut scoped = state_for_true_branch(&items[1], &header);
        let mut ignored = AnalysisSink::default();
        for body in items.iter().skip(2) {
            collect_termination_findings(body, structural_summaries, &scoped, findings);
            validate_static_bounds_expr(body, &mut scoped, &mut ignored);
        }
        return;
    }
    if op == "if" && items.len() >= 3 {
        collect_termination_findings(&items[1], structural_summaries, facts, findings);
        let then_facts = state_for_true_branch(&items[1], facts);
        collect_termination_findings(&items[2], structural_summaries, &then_facts, findings);
        if let Some(else_branch) = items.get(3) {
            let else_facts = state_for_false_branch(&items[1], facts);
            collect_termination_findings(else_branch, structural_summaries, &else_facts, findings);
        }
        return;
    }
    if op == "letrec" && items.len() == 3 {
        if let (Expression::Word(name), Expression::Apply(lambda)) = (&items[1], &items[2]) {
            if matches!(lambda.first(), Some(Expression::Word(head)) if head == "lambda")
                && lambda.len() >= 2
            {
                let params = lambda[1..lambda.len() - 1]
                    .iter()
                    .filter_map(|param| word(param).map(str::to_string))
                    .collect::<Vec<_>>();
                let body = lambda.last().expect("lambda body exists");
                let mut diagnostics = AnalysisSink::default();
                if params.len() == lambda.len() - 2 {
                    if contains_unchanged_recursive_call(body, name, &params) {
                        diagnostics.diagnostics.push(format!(
                            "recursive call to '{}' repeats all arguments unchanged",
                            name
                        ));
                    }
                    analyze_recursive_progress(
                        body,
                        name,
                        &params,
                        structural_summaries,
                        facts,
                        &mut diagnostics,
                    );
                }
                if let Some(reason) = diagnostics.diagnostics.into_iter().next() {
                    findings.push(TerminationFinding {
                        subject: name.clone(),
                        status: "warning".to_string(),
                        measure: None,
                        reason: reason.trim_start_matches("termination: ").to_string(),
                        proof: Vec::new(),
                    });
                } else if let Expression::Apply(branch) = body {
                    if matches!(branch.first(), Some(Expression::Word(head)) if head == "if")
                        && branch.len() >= 3
                    {
                        let then_recurses = contains_recursive_call(&branch[2], name);
                        let else_recurses = branch
                            .get(3)
                            .is_some_and(|candidate| contains_recursive_call(candidate, name));
                        let recursive_branch = if then_recurses ^ else_recurses {
                            Some(if then_recurses {
                                &branch[2]
                            } else {
                                &branch[3]
                            })
                        } else {
                            None
                        };
                        let proof = recursive_branch.and_then(|recursive_branch| {
                            params.iter().enumerate().find_map(|(index, parameter)| {
                                if facts
                                    .recursive_range_summaries
                                    .get(name)
                                    .and_then(|summary| summary.parameter_ranges.get(index))
                                    .is_some_and(Option::is_some)
                                {
                                    let mut found = false;
                                    let mut direction = None;
                                    if all_recursive_calls_have_unit_step(
                                        body,
                                        name,
                                        &params,
                                        index,
                                        facts,
                                        &mut found,
                                        &mut direction,
                                    ) && found
                                    {
                                        let direction = match direction {
                                            Some(CounterStep::Increase) => "increases",
                                            Some(CounterStep::Decrease) => "decreases",
                                            _ => return None,
                                        };
                                        return Some((
                                            parameter.clone(),
                                            format!(
                                                "{} {} toward the base-case guard",
                                                parameter, direction
                                            ),
                                        ));
                                    }
                                }
                                if length_base_case(&branch[1], parameter)
                                    && !recursive_calls_fail_to_shrink(
                                        recursive_branch,
                                        name,
                                        &params,
                                        index,
                                        structural_summaries,
                                    )
                                {
                                    return Some((
                                        format!("length({parameter})"),
                                        format!(
                                            "length({parameter}) decreases on the recursive path"
                                        ),
                                    ));
                                }
                                let recurse_when_true = then_recurses;
                                let expected = guard_direction_for_parameter(
                                    &branch[1],
                                    parameter,
                                    !recurse_when_true,
                                )?;
                                let recursive_facts =
                                    state_for_branch(&branch[1], facts, then_recurses);
                                let mut found = false;
                                let all_progress = recursive_calls_all_move_toward_guard(
                                    recursive_branch,
                                    name,
                                    &params,
                                    index,
                                    expected,
                                    &recursive_facts,
                                    &mut found,
                                );
                                (found && all_progress).then(|| {
                                    let direction = if expected == CounterStep::Increase {
                                        "increases"
                                    } else {
                                        "decreases"
                                    };
                                    (
                                        parameter.clone(),
                                        format!(
                                            "{} {} toward the base-case guard",
                                            parameter, direction
                                        ),
                                    )
                                })
                            })
                        });
                        if let Some((measure, reason)) = proof {
                            findings.push(TerminationFinding {
                                subject: name.clone(),
                                status: "proven".to_string(),
                                measure: Some(measure),
                                reason,
                                proof: vec![
                                    format!("base case: {}", branch[1].to_lisp()),
                                    format!(
                                        "recursive call: {}",
                                        first_recursive_call(
                                            recursive_branch.expect("recursive branch exists"),
                                            name
                                        )
                                        .map(|call| Expression::Apply(call.to_vec()).to_lisp())
                                        .unwrap_or_else(|| name.clone())
                                    ),
                                ],
                            });
                        } else if contains_recursive_call(body, name) {
                            findings.push(TerminationFinding {
                                subject: name.clone(),
                                status: "unknown".to_string(),
                                measure: None,
                                reason:
                                    "recursive calls exist, but no decreasing measure was inferred"
                                        .to_string(),
                                proof: Vec::new(),
                            });
                        }
                    } else if contains_recursive_call(body, name) {
                        findings.push(TerminationFinding {
                            subject: name.clone(),
                            status: "unknown".to_string(),
                            measure: None,
                            reason: "recursive calls exist, but no base-case measure was inferred"
                                .to_string(),
                            proof: Vec::new(),
                        });
                    }
                }
            }
        }
    }
    if matches!(op, "do" | "block") {
        let mut scoped = facts.clone();
        let mut ignored = AnalysisSink::default();
        for child in items.iter().skip(1) {
            collect_termination_findings(child, structural_summaries, &scoped, findings);
            validate_static_bounds_expr(child, &mut scoped, &mut ignored);
        }
        return;
    }
    for child in items.iter().skip(1) {
        collect_termination_findings(child, structural_summaries, facts, findings);
    }
}

pub fn explain_termination(
    typed_program: &TypedExpression,
    user_form_count: usize,
) -> Vec<TerminationFinding> {
    let all_expressions = match &typed_program.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "do") => {
            items.iter().skip(1).collect::<Vec<_>>()
        }
        expression => vec![expression],
    };
    let guard_summaries = infer_guard_summaries(&all_expressions);
    let predicate_summaries = infer_predicate_summaries(&all_expressions);
    let value_summaries = infer_value_summaries(&all_expressions);
    let recursive_range_summaries = infer_recursive_range_summaries(&all_expressions);
    let nonshrinking_vectors =
        infer_nonshrinking_vectors(&all_expressions, &value_summaries, &predicate_summaries);
    let structural_summaries = infer_structural_summaries(&all_expressions, &predicate_summaries);
    let start = all_expressions.len().saturating_sub(user_form_count);
    let mut facts = AbstractState {
        guard_summaries,
        predicate_summaries,
        value_summaries,
        structural_summaries: structural_summaries.clone(),
        recursive_range_summaries,
        nonshrinking_vectors,
        ..AbstractState::default()
    };
    let mut ignored_diagnostics = AnalysisSink::default();
    for expression in &all_expressions[..start] {
        validate_static_bounds_expr(expression, &mut facts, &mut ignored_diagnostics);
    }
    let mut findings = Vec::new();
    for expression in &all_expressions[start..] {
        collect_termination_findings(expression, &structural_summaries, &facts, &mut findings);
        validate_static_bounds_expr(expression, &mut facts, &mut ignored_diagnostics);
    }
    findings
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

fn infer_value_summaries(expressions: &[&Expression]) -> HashMap<String, ValueSummary> {
    let mut summaries: HashMap<String, ValueSummary> = HashMap::new();
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
                || lambda.len() < 2
            {
                continue;
            }
            let params: Vec<String> = lambda[1..lambda.len() - 1]
                .iter()
                .filter_map(word)
                .map(str::to_string)
                .collect();
            if params.len() != lambda.len() - 2 {
                continue;
            }
            let body = lambda.last().expect("lambda has a body");
            // Expanding a recursive body is not a result summary: every use
            // embeds the function inside itself. Large recursive functions can
            // otherwise make analysis exponential and exhaust the compiler's
            // stack. Inferring recursive postconditions needs a fixed point;
            // until then their result remains conservatively unknown.
            if keyword == "letrec" || contains_recursive_call(body, name) {
                continue;
            }
            let summary = ValueSummary {
                params,
                body: body.clone(),
            };
            changed |= summaries.insert(name.clone(), summary.clone()) != Some(summary);
        }
        if !changed {
            break;
        }
    }
    summaries
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
            if keyword == "letrec" || contains_recursive_call(body, name) {
                continue;
            }
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

fn combine_size_steps(left: SizeStep, right: SizeStep) -> SizeStep {
    match (left, right) {
        (SizeStep::Unchanged, other) | (other, SizeStep::Unchanged) => other,
        (left, right) if left == right => left,
        _ => SizeStep::Unknown,
    }
}

fn condition_proves_empty(
    condition: &Expression,
    parameter: &str,
    is_true: bool,
    predicate_summaries: &HashMap<String, PredicateSummary>,
    depth: usize,
) -> bool {
    if depth >= 16 {
        return false;
    }
    let Expression::Apply(items) = condition else {
        return false;
    };
    if let [Expression::Word(op), inner] = items.as_slice() {
        if op == "not" {
            return condition_proves_empty(
                inner,
                parameter,
                !is_true,
                predicate_summaries,
                depth + 1,
            );
        }
    }
    if let [Expression::Word(op), left, Expression::Int(bound)] = items.as_slice() {
        if length_operand(left) == Some(parameter) {
            return matches!(
                (op.as_str(), *bound, is_true),
                ("=", 0, true)
                    | ("<=", 0, true)
                    | ("<", 1, true)
                    | (">", 0, false)
                    | (">=", 1, false)
            );
        }
    }
    if let [Expression::Word(op), Expression::Int(bound), right] = items.as_slice() {
        if length_operand(right) == Some(parameter) {
            return matches!(
                (op.as_str(), *bound, is_true),
                ("=", 0, true)
                    | (">=", 0, true)
                    | (">", 1, true)
                    | ("<", 0, false)
                    | ("<=", 0, false)
            );
        }
    }
    let Some(op) = items.first().and_then(word) else {
        return false;
    };
    let Some(summary) = predicate_summaries.get(op) else {
        return false;
    };
    if summary.params.len() != items.len().saturating_sub(1) {
        return false;
    }
    let Ok(parsed_body) = crate::parser::build(&summary.body) else {
        return false;
    };
    let substitutions = summary
        .params
        .iter()
        .map(String::as_str)
        .zip(items.iter().skip(1))
        .collect::<HashMap<_, _>>();
    let expanded = substitute_predicate_body(single_built_expression(&parsed_body), &substitutions);
    condition_proves_empty(
        &expanded,
        parameter,
        is_true,
        predicate_summaries,
        depth + 1,
    )
}

fn structural_parameter_effect(
    expr: &Expression,
    parameter: &str,
    summaries: &HashMap<String, StructuralSummary>,
    predicate_summaries: &HashMap<String, PredicateSummary>,
) -> SizeStep {
    let Expression::Apply(items) = expr else {
        return SizeStep::Unchanged;
    };
    if matches!(items.first(), Some(Expression::Word(op)) if op == "lambda" || op == "letrec") {
        return SizeStep::Unchanged;
    }
    match items.as_slice() {
        [Expression::Word(op), Expression::Word(name)]
            if name == parameter && matches!(op.as_str(), "pop!" | "pop-val!") =>
        {
            return SizeStep::Shrink;
        }
        [Expression::Word(op), Expression::Word(name), _] if name == parameter && op == "push!" => {
            return SizeStep::Grow;
        }
        _ => {}
    }
    let op = items.first().and_then(word).unwrap_or("");
    if op == "if" {
        let then_effect = items
            .get(2)
            .map(|branch| {
                structural_parameter_effect(branch, parameter, summaries, predicate_summaries)
            })
            .unwrap_or(SizeStep::Unchanged);
        let else_effect = items
            .get(3)
            .map(|branch| {
                structural_parameter_effect(branch, parameter, summaries, predicate_summaries)
            })
            .unwrap_or(SizeStep::Unchanged);
        return if then_effect == else_effect {
            then_effect
        } else if condition_proves_empty(&items[1], parameter, true, predicate_summaries, 0)
            && then_effect == SizeStep::Unchanged
            && else_effect == SizeStep::Shrink
        {
            SizeStep::Shrink
        } else if condition_proves_empty(&items[1], parameter, false, predicate_summaries, 0)
            && then_effect == SizeStep::Shrink
            && else_effect == SizeStep::Unchanged
        {
            SizeStep::Shrink
        } else {
            SizeStep::Unknown
        };
    }
    if let Some(summary) = summaries.get(op) {
        if summary.params.len() == items.len().saturating_sub(1) {
            let mut effect = SizeStep::Unchanged;
            for (argument, called_effect) in items
                .iter()
                .skip(1)
                .zip(summary.parameter_effects.iter().copied())
            {
                if matches!(argument, Expression::Word(name) if name == parameter) {
                    effect = combine_size_steps(effect, called_effect);
                }
            }
            if effect != SizeStep::Unchanged {
                return effect;
            }
        }
    }
    items
        .iter()
        .skip(1)
        .fold(SizeStep::Unchanged, |effect, child| {
            combine_size_steps(
                effect,
                structural_parameter_effect(child, parameter, summaries, predicate_summaries),
            )
        })
}

fn structural_result_relation(
    expr: &Expression,
    parameter: &str,
    summaries: &HashMap<String, StructuralSummary>,
) -> SizeStep {
    match expr {
        Expression::Word(name) if name == parameter => SizeStep::Unchanged,
        Expression::Apply(items) => {
            match items.as_slice() {
                [Expression::Word(op), Expression::Word(name)]
                    if op == "cdr" && name == parameter =>
                {
                    return SizeStep::Shrink;
                }
                [Expression::Word(op), Expression::Word(name), _]
                    if op == "cdr" && name == parameter =>
                {
                    return SizeStep::Shrink;
                }
                [Expression::Word(op), left, right]
                    if op == "cons"
                        && (matches!(left, Expression::Word(name) if name == parameter)
                            || matches!(right, Expression::Word(name) if name == parameter)) =>
                {
                    return SizeStep::Grow;
                }
                _ => {}
            }
            let op = items.first().and_then(word).unwrap_or("");
            if matches!(op, "do" | "block") {
                return items
                    .last()
                    .map(|result| structural_result_relation(result, parameter, summaries))
                    .unwrap_or(SizeStep::Unknown);
            }
            if op == "if" {
                let then_relation = items
                    .get(2)
                    .map(|branch| structural_result_relation(branch, parameter, summaries))
                    .unwrap_or(SizeStep::Unknown);
                let else_relation = items
                    .get(3)
                    .map(|branch| structural_result_relation(branch, parameter, summaries))
                    .unwrap_or(SizeStep::Unknown);
                return if then_relation == else_relation {
                    then_relation
                } else {
                    SizeStep::Unknown
                };
            }
            let Some(summary) = summaries.get(op) else {
                return SizeStep::Unknown;
            };
            if summary.params.len() != items.len().saturating_sub(1) {
                return SizeStep::Unknown;
            }
            summary
                .result_relations
                .iter()
                .copied()
                .zip(items.iter().skip(1))
                .find_map(|(relation, argument)| {
                    matches!(argument, Expression::Word(name) if name == parameter)
                        .then_some(relation)
                })
                .unwrap_or(SizeStep::Unknown)
        }
        _ => SizeStep::Unknown,
    }
}

fn infer_structural_summaries(
    expressions: &[&Expression],
    predicate_summaries: &HashMap<String, PredicateSummary>,
) -> HashMap<String, StructuralSummary> {
    let mut summaries: HashMap<String, StructuralSummary> = HashMap::new();
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
            let params = lambda[1..lambda.len() - 1]
                .iter()
                .filter_map(word)
                .map(str::to_string)
                .collect::<Vec<_>>();
            if params.len() != lambda.len() - 2 {
                continue;
            }
            let body = lambda.last().expect("lambda has a body");
            let summary = StructuralSummary {
                parameter_effects: params
                    .iter()
                    .map(|param| {
                        structural_parameter_effect(body, param, &summaries, predicate_summaries)
                    })
                    .collect(),
                result_relations: params
                    .iter()
                    .map(|param| structural_result_relation(body, param, &summaries))
                    .collect(),
                params,
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
            let body = lambda.last().expect("lambda has a body");
            if keyword == "letrec" || contains_recursive_call(body, name) {
                continue;
            }
            let mut facts = AbstractState {
                guard_summaries: summaries.clone(),
                ..AbstractState::default()
            };
            facts = state_for_true_branch(body, &facts);
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

    fn diagnostics(source: &str, user_form_count: usize) -> Vec<String> {
        let expression = crate::parser::build(source).expect("source should build");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        analyze_user_program_diagnostics(&typed, user_form_count)
    }

    fn report(source: &str, user_form_count: usize) -> StaticAnalysisReport {
        let expression = crate::parser::build(source).expect("source should build");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        analyze_user_program_report(&typed, user_form_count)
    }

    #[test]
    fn structured_proofs_distinguish_safe_invalid_and_unknown_operations() {
        let findings = report(
            "(let xs [1 2]) (get xs 1) (get xs 9) (let divide (lambda x (/ 10 x))) (+ 2147483647 1)",
            5,
        );
        assert!(findings.proofs.iter().any(|proof| {
            proof.kind == ProofKind::BoundsRead
                && proof.status == ProofStatus::ProvenSafe
                && proof.expression == "(get xs 1)"
        }));
        assert!(findings.proofs.iter().any(|proof| {
            proof.kind == ProofKind::BoundsRead
                && proof.status == ProofStatus::DefinitelyInvalid
                && proof.expression == "(get xs 9)"
        }));
        assert!(findings.proofs.iter().any(|proof| {
            proof.kind == ProofKind::NonZeroDivisor
                && proof.status == ProofStatus::Unknown
                && proof.expression == "(/ 10 x)"
        }));
        assert!(findings.proofs.iter().any(|proof| {
            proof.kind == ProofKind::IntegerArithmetic
                && proof.status == ProofStatus::DefinitelyInvalid
                && proof.expression == "(+ 2147483647 1)"
        }));
        let ids = findings
            .proofs
            .iter()
            .map(|proof| proof.id)
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), findings.proofs.len(), "{:#?}", findings.proofs);
    }

    #[test]
    fn structured_proofs_have_stable_distinct_source_spans() {
        let source = "(let xs [1])\n(do\n  (get xs 0)\n  (get xs 0))";
        let expression = crate::parser::build(source).expect("source should build");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        let findings = analyze_user_program_report_with_source(
            &typed,
            crate::lsp_native_core::desugared_user_form_count(source)
                .expect("desugared form count"),
            source,
        );
        let reads = findings
            .proofs
            .iter()
            .filter(|proof| proof.kind == ProofKind::BoundsRead)
            .collect::<Vec<_>>();
        assert_eq!(reads.len(), 2, "{:#?}", findings.proofs);
        assert_ne!(reads[0].id, reads[1].id);
        assert_eq!(
            reads[0].source_span.expect("first source span").start.line,
            2
        );
        assert_eq!(
            reads[1].source_span.expect("second source span").start.line,
            3
        );
    }

    #[test]
    fn function_postconditions_preserve_ranges_nonzero_and_vector_lengths() {
        let findings = diagnostics(
            "(let clamp (lambda x (if (< x 0) 0 (if (> x 1) 1 x)))) (let pair (lambda x [x x])) (let i (clamp 99)) (let xs (pair 7)) {(get xs i) (/ 10 (if (= i 0) 1 i))}",
            5,
        );
        assert!(
            !findings
                .iter()
                .any(|message| message.starts_with("static bounds:")),
            "{findings:?}"
        );
        assert!(
            !findings
                .iter()
                .any(|message| message.contains("divisor may be zero")),
            "{findings:?}"
        );
    }

    #[test]
    fn recursive_functions_are_not_expanded_as_value_postconditions() {
        let source = "(letrec branch (lambda (n) (if (<= n 0) 0 (+ (branch (- n 1)) (branch (- n 1)))))) (branch 4)";
        let expression = crate::parser::build(source).expect("source should build");
        let expressions = match &expression {
            Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "do") => {
                items.iter().skip(1).collect::<Vec<_>>()
            }
            expression => vec![expression],
        };
        assert!(
            !infer_value_summaries(&expressions).contains_key("branch"),
            "recursive bodies require a fixed-point summary"
        );
        assert!(
            !infer_predicate_summaries(&expressions).contains_key("branch")
                && !infer_guard_summaries(&expressions).contains_key("branch"),
            "recursive predicates and guards also require fixed-point summaries"
        );

        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        let _proofs = analyze_codegen_proofs(&typed);
    }

    #[test]
    fn recursive_unit_step_range_is_inferred_from_entry_and_base_case() {
        let source = "(let N 4) (let xs [0 0 0 0]) (letrec walk (lambda i (if (= i N) 0 (do (set! xs i 1) (walk (+ i 1)))))) (walk 0)";
        let expression = crate::parser::build(source).expect("source should build");
        let expressions = match &expression {
            Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(op)) if op == "do") => {
                items.iter().skip(1).collect::<Vec<_>>()
            }
            expression => vec![expression],
        };
        let summaries = infer_recursive_range_summaries(&expressions);
        assert_eq!(
            summaries
                .get("walk")
                .map(|summary| &summary.parameter_ranges),
            Some(&vec![Some(IntInterval { min: 0, max: 4 })])
        );
        assert_eq!(analyze(source, 4), Ok(()));
        assert!(report(source, 4).proofs.iter().any(|proof| {
            proof.kind == ProofKind::Termination && proof.status == ProofStatus::ProvenSafe
        }));
    }

    #[test]
    fn recursive_range_proof_rejects_unknown_entries_and_non_unit_steps() {
        let unknown_entry = "(let xs [0 0 0 0]) (letrec walk (lambda i (if (= i 4) 0 (do (set! xs i 1) (walk (+ i 1)))))) (let run (lambda i (walk i))) (run 0)";
        let unknown_report = report(unknown_entry, 4);
        assert!(unknown_report.proofs.iter().any(|proof| {
            proof.kind == ProofKind::BoundsWriteReplacement
                && proof.expression == "(set! xs i 1)"
                && proof.status == ProofStatus::Unknown
        }));

        let skipped_exit = "(let xs [0 0 0 0]) (letrec walk (lambda i (if (= i 4) 0 (do (set! xs i 1) (walk (+ i 2)))))) (walk 0)";
        let skipped_report = report(skipped_exit, 3);
        assert!(skipped_report.proofs.iter().any(|proof| {
            proof.kind == ProofKind::BoundsWriteReplacement
                && proof.expression == "(set! xs i 1)"
                && proof.status == ProofStatus::Unknown
        }));
    }

    #[test]
    fn captured_minimum_length_is_not_reused_when_vector_can_shrink() {
        let source = "(let xs [0 0 0 0]) (letrec walk (lambda i (if (= i 4) 0 (do (set! xs i 1) (walk (+ i 1)))))) (pop! xs) (walk 0)";
        let findings = report(source, 4);
        assert!(findings.proofs.iter().any(|proof| {
            proof.kind == ProofKind::BoundsWriteReplacement
                && proof.expression == "(set! xs i 1)"
                && proof.status == ProofStatus::Unknown
        }));
    }

    #[test]
    fn disjunctive_ranges_preserve_holes_across_control_flow() {
        let findings = report(
            "(let divide (lambda x (if (or (= x 0) (= x 2)) (/ 10 (- x 1)) 0))) (let bad (lambda x (if (or (= x -1) (= x 5)) (get [1 2 3] x) 0)))",
            2,
        );
        assert!(
            !findings
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("divisor may be zero")),
            "{:#?}",
            findings.diagnostics
        );
        assert!(findings.proofs.iter().any(|proof| {
            proof.kind == ProofKind::NonZeroDivisor
                && proof.status == ProofStatus::ProvenSafe
                && proof.expression == "(/ 10 (- x 1))"
        }));
        assert!(
            findings.proofs.iter().any(|proof| {
                proof.kind == ProofKind::BoundsRead
                    && proof.status == ProofStatus::DefinitelyInvalid
                    && proof.expression == "(get (vector 1 2 3) x)"
            }),
            "{:#?}",
            findings.proofs
        );
    }

    #[test]
    fn termination_warns_for_constant_true_and_invariant_guard_loops() {
        let constant = diagnostics("(mut i 0) (while true (alter! i i))", 2);
        assert!(constant
            .iter()
            .any(|message| message.contains("no program-controlled exit")));

        let invariant = diagnostics("(mut i 0) (mut j 0) (while (< i 10) (alter! j (+ j 1)))", 3);
        assert!(invariant
            .iter()
            .any(|message| message.contains("condition cannot change")));
    }

    #[test]
    fn termination_warns_when_counter_moves_away_but_not_toward_bound() {
        let away = diagnostics("(mut i 0) (while (< i 10) (alter! i (- i 1)))", 2);
        assert!(away
            .iter()
            .any(|message| message.contains("does not move toward")));

        let toward = diagnostics("(mut i 0) (while (< i 10) (alter! i (+ i 1)))", 2);
        assert!(!toward
            .iter()
            .any(|message| message.starts_with("termination:")));
    }

    #[test]
    fn termination_requires_scalar_progress_on_every_path() {
        let conditional = diagnostics(
            "(let run (lambda (move?) (mut i 0) (while (< i 10) (if move? (alter! i (+ i 1)))) i))",
            1,
        );
        assert!(
            conditional
                .iter()
                .any(|message| message.contains("not guaranteed to move")),
            "{conditional:?}"
        );

        let both_branches = diagnostics(
            "(let run (lambda (choose?) (mut i 0) (while (< i 10) (if choose? (alter! i (+ i 1)) (alter! i (+ i 2)))) i))",
            1,
        );
        assert!(
            !both_branches
                .iter()
                .any(|message| message.starts_with("termination:")),
            "{both_branches:?}"
        );

        let unconditional_fallback = diagnostics(
            "(let run (lambda (extra?) (mut i 0) (while (< i 10) (do (if extra? (alter! i (+ i 1))) (alter! i (+ i 1)))) i))",
            1,
        );
        assert!(
            !unconditional_fallback
                .iter()
                .any(|message| message.starts_with("termination:")),
            "{unconditional_fallback:?}"
        );

        let conflicting = diagnostics(
            "(let run (lambda (up?) (mut i 0) (while (< i 10) (if up? (alter! i (+ i 1)) (alter! i (- i 1)))) i))",
            1,
        );
        assert!(
            conflicting
                .iter()
                .any(|message| message.contains("not guaranteed to move")),
            "{conflicting:?}"
        );
    }

    #[test]
    fn termination_respects_boolean_condition_structure_and_moving_bounds() {
        let conjunction = diagnostics(
            "(let run (lambda (keep?) (mut i 0) (while (and (< i 10) keep?) (alter! i (+ i 1))) i))",
            1,
        );
        assert!(
            !conjunction
                .iter()
                .any(|message| message.starts_with("termination:")),
            "{conjunction:?}"
        );

        let disjunction = diagnostics(
            "(let run (lambda (keep?) (mut i 0) (while (or (< i 10) keep?) (alter! i (+ i 1))) i))",
            1,
        );
        assert!(
            disjunction
                .iter()
                .any(|message| message.contains("not guaranteed to move")),
            "{disjunction:?}"
        );

        let negated = diagnostics("(mut i 0) (while (not (>= i 10)) (alter! i (+ i 1)))", 2);
        assert!(
            !negated
                .iter()
                .any(|message| message.starts_with("termination:")),
            "{negated:?}"
        );

        let moving_bound = diagnostics(
            "(mut i 0) (mut n 10) (while (< i n) (do (alter! i (+ i 1)) (alter! n (+ n 1))))",
            3,
        );
        assert!(
            moving_bound
                .iter()
                .any(|message| message.contains("not guaranteed to move")),
            "{moving_bound:?}"
        );
    }

    #[test]
    fn termination_requires_structural_progress_on_every_path() {
        let conditional = diagnostics(
            "(let run (lambda (remove?) (let xs [1 2 3]) (while (> (length xs) 0) (if remove? (pop! xs))) xs))",
            1,
        );
        assert!(
            conditional
                .iter()
                .any(|message| message.contains("not guaranteed to move")),
            "{conditional:?}"
        );

        let both_branches = diagnostics(
            "(let run (lambda (front?) (let xs [1 2 3]) (while (> (length xs) 0) (if front? (pop! xs) (pop! xs))) xs))",
            1,
        );
        assert!(
            !both_branches
                .iter()
                .any(|message| message.starts_with("termination:")),
            "{both_branches:?}"
        );
    }

    #[test]
    fn termination_uses_symbolic_step_sign_facts() {
        let positive = diagnostics(
            "(let step 2) (mut i 0) (while (< i 10) (alter! i (+ i step)))",
            3,
        );
        assert!(!positive
            .iter()
            .any(|message| message.starts_with("termination:")));

        let guarded_parameter = diagnostics(
            "(let count (lambda (step) (mut i 0) (while (and (> step 0) (< i 10)) (alter! i (+ i step))) i))",
            1,
        );
        assert!(!guarded_parameter
            .iter()
            .any(|message| message.starts_with("termination:")));

        let negative = diagnostics(
            "(let step -2) (mut i 0) (while (< i 10) (alter! i (+ i step)))",
            3,
        );
        assert!(negative
            .iter()
            .any(|message| message.contains("does not move toward")));

        let recursive = diagnostics(
            "(let step 2) (letrec down (lambda (n) (if (<= n 0) 0 (down (- n step)))))",
            2,
        );
        assert!(!recursive
            .iter()
            .any(|message| message.starts_with("termination:")));
    }

    #[test]
    fn termination_treats_vector_length_as_counter_bound_not_measure() {
        let findings = diagnostics(
            "(let xs [5 8 2 1 5]) (mut i 0) (while (< i (length xs)) (do (get xs i) (alter! i (+ i 1))))",
            3,
        );
        assert!(
            !findings
                .iter()
                .any(|message| message.starts_with("termination:")),
            "{findings:?}"
        );
    }

    #[test]
    fn termination_model_matrix_separates_measures_from_bounds() {
        let cases = [
            (
                "fixed direct length bound",
                "(let xs [1 2 3]) (mut i 0) (while (< i (length xs)) (alter! i (+ i 1)))",
                false,
            ),
            (
                "reversed direct length bound",
                "(let xs [1 2 3]) (mut i 0) (while (> (length xs) i) (alter! i (+ i 1)))",
                false,
            ),
            (
                "cached length bound",
                "(let xs [1 2 3]) (let len (length xs)) (mut i 0) (while (< i len) (alter! i (+ i 1)))",
                false,
            ),
            (
                "descending scalar counter",
                "(mut i 3) (let step 1) (while (> i 0) (alter! i (- i step)))",
                false,
            ),
            (
                "shrinking vector measure",
                "(let xs [1 2 3]) (while (> (length xs) 0) (pop! xs))",
                false,
            ),
            (
                "reversed shrinking vector measure",
                "(let xs [1 2 3]) (while (< 0 (length xs)) (pop! xs))",
                false,
            ),
            (
                "counter and bound both approach exit",
                "(let xs [1 2 3]) (mut i 0) (while (< i (length xs)) (do (alter! i (+ i 1)) (pop! xs)))",
                false,
            ),
            (
                "scalar counter moves away",
                "(let xs [1 2 3]) (mut i 0) (while (< i (length xs)) (alter! i (- i 1)))",
                true,
            ),
            (
                "length measure never changes",
                "(let xs [1 2 3]) (mut n 0) (while (> (length xs) 0) (alter! n (+ n 1)))",
                true,
            ),
            (
                "length moves away",
                "(let xs [1]) (while (> (length xs) 0) (push! xs 1))",
                true,
            ),
        ];

        for (name, source, should_warn) in cases {
            let findings = diagnostics(source, 99);
            let warned = findings
                .iter()
                .any(|message| message.starts_with("termination:"));
            assert_eq!(warned, should_warn, "{name}: {findings:?}");
        }

        let competing =
            "(let xs [1]) (mut i 0) (while (< i (length xs)) (do (alter! i (+ i 1)) (push! xs 1)))";
        let expression = crate::parser::build(competing).expect("source should build");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        let findings = explain_termination(&typed, 3);
        assert!(findings.iter().any(|finding| {
            finding.status == "unknown" && finding.reason.contains("competing directions")
        }));
    }

    #[test]
    fn termination_model_matrix_covers_comparison_orientation() {
        let cases = [
            ("(< i 3)", "(+ i 1)", false),
            ("(<= i 2)", "(+ i 1)", false),
            ("(> 3 i)", "(+ i 1)", false),
            ("(>= 2 i)", "(+ i 1)", false),
            ("(> i 0)", "(- i 1)", false),
            ("(>= i 1)", "(- i 1)", false),
            ("(< 0 i)", "(- i 1)", false),
            ("(<= 1 i)", "(- i 1)", false),
            ("(< i 3)", "(- i 1)", true),
            ("(> 3 i)", "(- i 1)", true),
            ("(> i 0)", "(+ i 1)", true),
            ("(< 0 i)", "(+ i 1)", true),
        ];
        for (condition, update, should_warn) in cases {
            let source = format!("(mut i 1) (while {} (alter! i {}))", condition, update);
            let findings = diagnostics(&source, 99);
            let warned = findings
                .iter()
                .any(|message| message.starts_with("termination:"));
            assert_eq!(
                warned, should_warn,
                "condition={condition}, update={update}: {findings:?}"
            );
        }
    }

    #[test]
    fn loop_induction_range_reports_square_overflow() {
        for (n, should_warn) in [(46340, false), (46341, true), (5_000_000, true)] {
            let source = format!(
                "(let n {n}) (mut i 0) (while (<= i n) (do (if (>= i 2) (let j (* i i))) (alter! i (+ i 1))))"
            );
            let findings = diagnostics(&source, 99);
            let warned = findings.iter().any(|message| {
                message.contains("Int overflow possible") && message.contains("(* i i)")
            });
            assert_eq!(warned, should_warn, "n={n}: {findings:?}");
        }
    }

    #[test]
    fn counted_append_loop_proves_following_get_and_set_bounds() {
        let source = "(let n 100) (let flags []) (mut i 0) (while (<= i n) (do (push! flags true) (alter! i (+ i 1)))) (mut k 0) (while (<= k n) (do (get flags k) (set! flags k false) (alter! k (+ k 1))))";
        let findings = diagnostics(source, 99);
        assert!(
            !findings
                .iter()
                .any(|message| message.starts_with("static bounds:")),
            "{findings:?}"
        );
    }

    #[test]
    fn bounded_loop_updates_prove_conditional_counter_increment_safe() {
        let source = "(let n 5000000) (mut count 0) (mut i 0) (while (<= i n) (do (if (>= i 2) (alter! count (+ count 1))) (alter! i (+ i 1))))";
        let findings = diagnostics(source, 99);
        assert!(
            !findings
                .iter()
                .any(|message| message.contains("(+ count 1)")),
            "{findings:?}"
        );
    }

    #[test]
    fn termination_warns_for_unchanged_recursive_arguments() {
        let findings = diagnostics("(letrec repeat (lambda (n) (if (= n 0) 0 (repeat n))))", 1);
        assert!(findings
            .iter()
            .any(|message| message.contains("repeats all arguments unchanged")));

        let decreasing = diagnostics(
            "(letrec count-down (lambda (n) (if (<= n 0) 0 (count-down (- n 1)))))",
            1,
        );
        assert!(!decreasing
            .iter()
            .any(|message| message.starts_with("termination:")));
    }

    #[test]
    fn termination_compares_recursive_steps_with_base_case_guards() {
        let away = diagnostics(
            "(letrec grow (lambda (n) (if (<= n 0) 0 (grow (+ n 1)))))",
            1,
        );
        assert!(away
            .iter()
            .any(|message| message.contains("moves away from its base-case guard")));

        let toward = diagnostics(
            "(letrec shrink (lambda (n) (if (<= n 0) 0 (shrink (- n 1)))))",
            1,
        );
        assert!(!toward
            .iter()
            .any(|message| message.starts_with("termination:")));

        let unconditional = diagnostics("(letrec grow (lambda (n) (grow (+ n 1))))", 1);
        assert!(unconditional
            .iter()
            .any(|message| message.contains("no conditional exit path")));
    }

    #[test]
    fn recursive_termination_requires_every_recursive_call_to_progress() {
        let mixed = diagnostics(
            "(letrec walk (lambda (n m) (if (<= n 0) 0 (do (walk (- n 1) m) (walk n (+ m 1))))))",
            1,
        );
        assert!(
            mixed
                .iter()
                .any(|message| message.contains("not every recursive call")),
            "{mixed:?}"
        );

        let all_decrease = diagnostics(
            "(letrec walk (lambda (n m) (if (<= n 0) 0 (do (walk (- n 1) m) (walk (- n 2) (+ m 1))))))",
            1,
        );
        assert!(
            !all_decrease
                .iter()
                .any(|message| message.starts_with("termination:")),
            "{all_decrease:?}"
        );

        let expression = crate::parser::build(
            "(letrec walk (lambda (n m) (if (<= n 0) 0 (do (walk (- n 1) m) (walk n (+ m 1))))))",
        )
        .expect("source should build");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        assert!(explain_termination(&typed, 1)
            .iter()
            .all(|finding| finding.status != "proven"));
    }

    #[test]
    fn structural_recursive_termination_requires_every_call_to_shrink() {
        let mixed = diagnostics(
            "(letrec drain (lambda (xs n) (if (= (length xs) 0) 0 (do (drain (cdr xs) n) (drain xs (- n 1))))))",
            1,
        );
        assert!(
            mixed
                .iter()
                .any(|message| message.contains("does not shrink 'xs'")),
            "{mixed:?}"
        );
    }

    #[test]
    fn termination_tracks_structural_vector_recursion() {
        let shrinking = diagnostics(
            "(letrec drain (lambda (xs) (if (= (length xs) 0) 0 (drain (cdr xs)))))",
            1,
        );
        assert!(
            !shrinking
                .iter()
                .any(|message| message.starts_with("termination:")),
            "{shrinking:?}"
        );

        let unchanged = diagnostics(
            "(letrec stuck (lambda (xs n) (if (= (length xs) 0) 0 (stuck xs (- n 1)))))",
            1,
        );
        assert!(unchanged
            .iter()
            .any(|message| message.contains("does not shrink 'xs'")));

        let growing = diagnostics(
            "(letrec grow (lambda (xs) (if (= (length xs) 0) 0 (grow (cons [1] xs)))))",
            1,
        );
        assert!(growing
            .iter()
            .any(|message| message.contains("does not shrink 'xs'")));
    }

    #[test]
    fn termination_tracks_vectors_consumed_by_loops() {
        let shrinking = diagnostics("(let xs [1 2 3]) (while (> (length xs) 0) (pop! xs))", 2);
        assert!(!shrinking
            .iter()
            .any(|message| message.starts_with("termination:")));

        let unchanged = diagnostics(
            "(let xs [1 2 3]) (mut n 0) (while (> (length xs) 0) (alter! n (+ n 1)))",
            3,
        );
        assert!(unchanged
            .iter()
            .any(|message| message.contains("length of 'xs' does not move")));

        let growing = diagnostics("(let xs [1]) (while (> (length xs) 0) (push! xs 1))", 2);
        assert!(growing
            .iter()
            .any(|message| message.contains("length of 'xs' does not move")));
    }

    #[test]
    fn termination_infers_structural_effects_from_helpers_and_aliases() {
        let shrinking_helper = diagnostics(
            "(let remove-last (lambda xs (pop! xs))) (let shrink remove-last) (let xs [1 2 3]) (while (> (length xs) 0) (shrink xs))",
            4,
        );
        assert!(!shrinking_helper
            .iter()
            .any(|message| message.starts_with("termination:")));

        let growing_helper = diagnostics(
            "(let add-one (lambda xs (push! xs 1))) (let xs [1]) (while (> (length xs) 0) (add-one xs))",
            3,
        );
        assert!(growing_helper
            .iter()
            .any(|message| message.contains("length of 'xs' does not move")));

        let nested_helper = diagnostics(
            "(let clear (lambda xs (while (> (length xs) 0) (pop! xs)))) (let xs [1 2]) (while (> (length xs) 0) (clear xs))",
            3,
        );
        assert!(!nested_helper
            .iter()
            .any(|message| message.starts_with("termination:")));

        let predicate_wrapped_helper = diagnostics(
            "(let zero-sized (lambda ys (= (length ys) 0))) (let clear-all (lambda zs (if (zero-sized zs) zs (do (while (> (length zs) 0) (pop! zs)) zs)))) (let clear clear-all) (let xs [1 2]) (while (> (length xs) 0) (clear xs))",
            5,
        );
        assert!(!predicate_wrapped_helper
            .iter()
            .any(|message| message.starts_with("termination:")));
    }

    #[test]
    fn termination_infers_structural_result_relations_from_helpers() {
        let through_helper = diagnostics(
            "(let tail (lambda xs (cdr xs))) (let next tail) (letrec drain (lambda (xs) (if (= (length xs) 0) 0 (drain (next xs)))))",
            3,
        );
        assert!(!through_helper
            .iter()
            .any(|message| message.starts_with("termination:")));

        let growing_helper = diagnostics(
            "(let extend (lambda xs (cons [1] xs))) (letrec grow (lambda (xs) (if (= (length xs) 0) 0 (grow (extend xs)))))",
            2,
        );
        assert!(growing_helper
            .iter()
            .any(|message| message.contains("does not shrink 'xs'")));
    }

    #[test]
    fn termination_checks_unconditional_recursion_after_an_earlier_if() {
        let findings = diagnostics(
            "(letrec grow (lambda (n) (if (= n 0) 0 0) (grow (+ n 1))))",
            1,
        );
        assert!(
            findings
                .iter()
                .any(|message| message.contains("no conditional exit path")),
            "{findings:?}"
        );
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
        let source = "(let xs [1 2 3]) (push! xs 10) {(get xs 2) (get xs 3)}";
        assert_eq!(analyze(source, 2), Ok(()));
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
    fn set_proofs_distinguish_replacement_from_append_at_length() {
        let replacement = report(
            "(let replace! (lambda (xs i) (if (and (>= i 0) (< i (length xs))) (set! xs i 7) nil)))",
            1,
        );
        assert!(replacement.proofs.iter().any(|proof| {
            proof.kind == ProofKind::BoundsWriteReplacement
                && proof.status == ProofStatus::ProvenSafe
                && proof.expression == "(set! xs i 7)"
        }));

        let append = report("(let xs []) (set! xs (length xs) 7)", 2);
        assert!(append.proofs.iter().any(|proof| {
            proof.kind == ProofKind::BoundsWrite
                && proof.status == ProofStatus::ProvenSafe
                && proof.expression == "(set! xs (length xs) 7)"
        }));
        assert!(append.proofs.iter().any(|proof| {
            proof.kind == ProofKind::BoundsWriteReplacement
                && proof.status != ProofStatus::ProvenSafe
                && proof.expression == "(set! xs (length xs) 7)"
        }));
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
        let source = "(let history []) (let alt []) (let undo! (lambda () (push! alt (pop-val! history)))) (let redo! (lambda () (push! history (pop-val! alt))))";
        let expression = crate::parser::build(source).expect("source should parse");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &expression,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        let diagnostics = analyze_user_program_diagnostics(&typed, 4);
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("(pop-val! history)")));
        assert!(diagnostics
            .iter()
            .any(|message| message.contains("(pop-val! alt)")));
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
        assert!(analyze("(% 10 0)", 1).is_err());
        assert_eq!(analyze("(/ 10 2)", 1), Ok(()));
    }

    #[test]
    fn constant_division_narrows_an_unknown_int_before_later_arithmetic() {
        let safe = "(let req (lambda (x) (- (/ x 3) 2)))";
        assert_eq!(analyze(safe, 1), Ok(()));

        let unsafe_identity = "(let req (lambda (x) (- (/ x 1) 2)))";
        assert!(analyze(unsafe_identity, 1)
            .expect_err("division by one must retain the possible underflow")
            .contains("underflow"));
    }

    #[test]
    fn division_tracks_the_minimum_int_over_negative_one_overflow() {
        let possible = diagnostics("(let negate (lambda (x) (/ x -1)))", 1);
        assert!(possible
            .iter()
            .any(|message| message.contains("Int overflow possible")));

        let certain = diagnostics("(/ -2147483648 -1)", 1);
        assert!(certain
            .iter()
            .any(|message| message.contains("Int overflow:")));

        assert_eq!(
            analyze("(let halve-negated (lambda (x) (/ x -2)))", 1),
            Ok(())
        );
    }

    #[test]
    fn branch_facts_prove_division_is_nonzero() {
        let guarded = "(let divide (lambda x divisor (if (= divisor 0) 0 (/ x divisor))))";
        let guarded_diagnostics = diagnostics(guarded, 1);
        assert!(!guarded_diagnostics
            .iter()
            .any(|message| message.contains("divisor may be zero")));
        assert!(guarded_diagnostics
            .iter()
            .any(|message| message.contains("Int overflow possible")));

        let positive = "(let divide (lambda x divisor (if (> divisor 0) (/ x divisor) 0)))";
        assert_eq!(analyze(positive, 1), Ok(()));

        let predicate = "(let nonzero? (lambda x (not (= x 0)))) (let divide (lambda x divisor (if (nonzero? divisor) (/ x divisor) 0)))";
        let predicate_diagnostics = diagnostics(predicate, 2);
        assert!(!predicate_diagnostics
            .iter()
            .any(|message| message.contains("divisor may be zero")));
        assert!(predicate_diagnostics
            .iter()
            .any(|message| message.contains("Int overflow possible")));
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

        let max_inequality = "(let infinity 2147483647) (let add-one (lambda left (if (= left infinity) left (+ left 1))))";
        assert_eq!(analyze(max_inequality, 2), Ok(()));

        let min_inequality = "(let minimum -2147483648) (let sub-one (lambda right (if (= right minimum) right (- right 1))))";
        assert_eq!(analyze(min_inequality, 2), Ok(()));
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

        let literal_operand = r#"
            (let INT-MIN -2147483648)
            (let INT-MAX 2147483647)
            (let add-fits?
              (lambda (a b)
                (if (> b 0)
                    (<= a (- INT-MAX b))
                    (if (< b 0) (>= a (- INT-MIN b)) true))))
            (let increment
              (lambda index
                (if (not (add-fits? index 1)) index (+ index 1))))
        "#;
        assert_eq!(analyze(literal_operand, 4), Ok(()));

        let statically_false_guard = r#"
            (let INT-MIN -2147483648)
            (let INT-MAX 2147483647)
            (let add-fits?
              (lambda (a b)
                (if (> b 0)
                    (<= a (- INT-MAX b))
                    (if (< b 0) (>= a (- INT-MIN b)) true))))
            (let x INT-MAX)
            (if (add-fits? x 1) (+ x 1) 0)
        "#;
        assert_eq!(analyze(statically_false_guard, 5), Ok(()));

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

        let repeated_expression = r#"
            (let INT-MIN -2147483648)
            (let INT-MAX 2147483647)
            (let add-fits?
              (lambda (a b)
                (if (> b 0)
                    (<= a (- INT-MAX b))
                    (if (< b 0) (>= a (- INT-MIN b)) true))))
            (let multiply-fits?
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
            (let combine
              (lambda (a b c)
                (if (not (add-fits? a b)) 0
                  (if (not (multiply-fits? (+ a b) c)) 0
                    (* (+ a b) c)))))
        "#;
        assert_eq!(analyze(repeated_expression, 5), Ok(()));

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
    fn constant_division_narrows_unknown_ranges_before_offset_arithmetic() {
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
            diagnostics.iter().all(|message| {
                !(message.contains("(+ __block_0_index 1)")
                    || message.contains("(- __block_0_index 1)"))
            }),
            "division by two should narrow the index enough to prove both offsets safe: {diagnostics:?}"
        );
    }

    #[test]
    fn relational_ranges_prove_safe_binary_search_midpoint() {
        let source = "(let midpoint (lambda left right (if (or (> left right) (< left 0)) 0 (+ left (/ (- right left) 2)))))";
        assert_eq!(analyze(source, 1), Ok(()));

        let missing_lower_bound =
            "(let midpoint (lambda left right (if (> left right) 0 (+ left (/ (- right left) 2)))))";
        let result = analyze(missing_lower_bound, 1);
        assert!(
            result
                .as_ref()
                .is_err_and(|message| message.contains("overflow")),
            "{result:?}"
        );
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
