use crate::parser::Expression;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::panic::{ catch_unwind, AssertUnwindSafe };
use std::rc::{ Rc, Weak };
#[derive(Clone)]
pub enum BiteCodeEvaluated {
    Bool(bool),
    Int(i32),
    Float(f32),
    Function(usize, Vec<String>, Vec<Instruction>, Rc<RefCell<BiteCodeEnv>>),
    Array(Rc<RefCell<Vec<BiteCodeEvaluated>>>),
}

impl fmt::Debug for BiteCodeEvaluated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BiteCodeEvaluated::Bool(value) => write!(f, "{}", value),
            BiteCodeEvaluated::Int(value) => write!(f, "{}", value),
            BiteCodeEvaluated::Float(value) => write!(f, "{:?}", value),
            BiteCodeEvaluated::Function(_, _, _, _) => write!(f, "Function"),
            BiteCodeEvaluated::Array(arr) => {
                let arr_ref = arr.borrow();
                let elements: Vec<String> = arr_ref
                    .iter()
                    .map(|x| format!("{:?}", x))
                    .collect();
                write!(f, "[{}]", elements.join(" "))
            }
        }
    }
}

#[derive(Clone)]
pub struct BiteCodeEnv {
    vars: HashMap<String, BiteCodeEvaluated>,
    parent: Option<Weak<RefCell<BiteCodeEnv>>>,
}

impl BiteCodeEnv {
    fn new() -> Self {
        BiteCodeEnv {
            vars: HashMap::with_capacity(16), // Pre-allocate space for common variables
            parent: None,
        }
    }

    fn with_parent(parent: Rc<RefCell<BiteCodeEnv>>) -> Self {
        BiteCodeEnv {
            vars: HashMap::with_capacity(16), // Pre-allocate space for common variables
            parent: Some(Rc::downgrade(&parent)),
        }
    }

    fn get(&self, name: &str) -> Option<BiteCodeEvaluated> {
        if let Some(var) = self.vars.get(name) {
            return Some(var.clone());
        }
        if let Some(ref weak_parent) = self.parent {
            return weak_parent.upgrade()?.borrow().get(name);
        }
        None
    }

    fn set(&mut self, name: String, value: BiteCodeEvaluated) {
        self.vars.insert(name, value);
    }
}

#[derive(Clone, Debug)]
pub enum Instruction {
    PushInt(i32),
    PushFloat(f32),
    PushBool(bool),

    StoreVar(String),
    LoadVar(String),
    MakeLambda(Vec<String>, Vec<Instruction>),
    MakeLambdaUid(usize, Vec<String>, Vec<Instruction>),
    Call(usize),
    MakeVector(usize),
    If(usize, Vec<Instruction>, Vec<Instruction>),

    Loop(
        Vec<Instruction>,
        Vec<Instruction>,
        Vec<Instruction>, // code for the lambda expression
    ),
    LoopFinish(Vec<Instruction>, Vec<Instruction>),
    Length,
    Add,
    AddF,
    Mult,
    MultF,
    Div,
    DivF,
    Sub,
    SubF,
    Mod,
    ModF,
    Pop,
    Lt,
    Gt,
    Lte,
    Gte,
    Eq,
    Not,
    EqBool,
    EqF,
    GtF,
    LtF,
    GteF,
    LteF,

    BitXor,
    BitRs,
    BitLs,
    BitNot,
    BitOr,
    BitAnd,

    SetArray, // expects stack: [value,index,vector]
    GetArray,
    PopArray,
    RestArray,

    IntToFloat,
    FloatToInt,
}

#[derive(Clone, Debug, Default)]
pub struct BranchSiteCoverage {
    pub true_hits: usize,
    pub false_hits: usize,
}

const MAXIMUM_CALL_STACK_DEPTH_LIMIT: usize = 200;
const MAX_FUNCTION_REPORT_ENTRIES: usize = 100;
const MAX_INPUT_PREVIEW_LEN: usize = 160;
const INPUT_PREVIEW_EDGE_LEN: usize = 25;
const MAX_RESULT_REPR_LEN: usize = 160;
const RESULT_REPR_EDGE_LEN: usize = 25;
const MAX_INNER_VALUE_PREVIEW_LEN: usize = 80;
const INNER_VALUE_PREVIEW_EDGE_LEN: usize = 20;

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct RuntimeReport {
    pub total_instructions: usize,
    pub instruction_counts: HashMap<String, usize>,
    pub max_stack_size: usize,
    pub final_stack_size: usize,
    pub max_call_depth: usize,
    pub call_count: usize,
    pub partial_application_count: usize,
    pub branch_true_count: usize,
    pub branch_false_count: usize,
    pub branch_site_outcomes: HashMap<usize, BranchSiteCoverage>,
    pub loop_iterations: usize,
    pub loop_finish_iterations: usize,
    pub max_loop_iterations: usize,
    pub max_loop_finish_iterations: usize,
    pub array_alloc_count: usize,
    pub max_array_len_seen: usize,
    pub set_array_append_count: usize,
    pub set_array_overwrite_count: usize,
    pub rest_array_empty_result_count: usize,
    pub get_array_count: usize,
    pub scalar_box_get_count: usize,
    pub min_get_index: Option<i32>,
    pub max_get_index: Option<i32>,
    pub min_set_index: Option<i32>,
    pub max_set_index: Option<i32>,
    pub scalar_box_set_count: usize,
    pub out_of_bounds_count: usize,
    pub call_depth_limit_exceeded_count: usize,
    pub int_overflow_risk_count: usize,
    pub int_div_by_zero_count: usize,
    pub float_non_finite_count: usize,
    pub final_result_type_tag: Option<String>,
    pub final_result_repr: Option<String>,
    pub function_reports: Vec<FunctionRuntimeReport>,
    pub function_reports_truncated: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct FunctionRuntimeReport {
    pub function_id: String,
    pub arity: usize,
    pub call_depth: usize,
    pub report: RuntimeReportDelta,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct GroupedFunctionRuntimePattern {
    pub function_id: String,
    pub arity: usize,
    pub final_result_type_tag: Option<String>,
    pub final_result_repr: Option<String>,
    pub count: usize,
    pub input_examples: Vec<String>,
    pub totals: RuntimeReportDelta,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct FunctionAnomalySignals {
    pub function_id: String,
    pub final_result_type_tag: Option<String>,
    pub final_result_repr: Option<String>,
    pub input_examples: Vec<String>,
    pub count: usize,
    pub total_instructions: usize,
    pub avg_instructions_per_call: f32,
    pub iterative_signal: usize,
    pub branch_signal: usize,
    pub branch_sites_total: usize,
    pub branch_sites_full: usize,
    pub branch_site_coverage_pct: f32,
    pub branch_outcome_coverage_pct: f32,
    pub vector_mutation_signal: usize,
    pub vector_read_signal: usize,
    pub scalar_update_signal: usize,
    pub scalar_read_signal: usize,
    pub literal_build_signal: usize,
    pub input_array_avg_len: f32,
    pub output_array_avg_len: f32,
    pub writes_per_input_elem: f32,
    pub reads_per_input_elem: f32,
    pub update_coverage_ratio: f32,
    pub coverage_interpretation: String,
    pub get_index_range: Option<(i32, i32)>,
    pub set_index_range: Option<(i32, i32)>,
    pub out_of_bounds_count: usize,
    pub call_depth_limit_exceeded_count: usize,
    pub int_overflow_risk_count: usize,
    pub int_div_by_zero_count: usize,
    pub float_non_finite_count: usize,
    pub algorithm_style: String,
    pub mutation_ratio: f32,
    pub literal_ratio: f32,
    pub potential_complexity: String,
    pub complexity_confidence: f32,
    pub suspicious_flags: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct LlmAnomalyReport {
    pub final_result_type_tag: Option<String>,
    pub final_result_repr: Option<String>,
    pub total_instructions: usize,
    pub int_overflow_risk_count: usize,
    pub int_div_by_zero_count: usize,
    pub float_non_finite_count: usize,
    pub branch_sites_total: usize,
    pub branch_sites_full: usize,
    pub branch_site_coverage_pct: f32,
    pub branch_outcome_coverage_pct: f32,
    pub dominant_potential_complexity: String,
    pub dominant_complexity_function_id: Option<String>,
    pub dominant_complexity_confidence: f32,
    pub function_groups: Vec<FunctionAnomalySignals>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct RuntimeReportDelta {
    pub total_instructions: usize,
    pub instruction_counts: HashMap<String, usize>,
    pub max_stack_size: usize,
    pub final_stack_size: usize,
    pub max_call_depth: usize,
    pub call_count: usize,
    pub partial_application_count: usize,
    pub branch_true_count: usize,
    pub branch_false_count: usize,
    pub branch_site_outcomes: HashMap<usize, BranchSiteCoverage>,
    pub loop_iterations: usize,
    pub loop_finish_iterations: usize,
    pub max_loop_iterations: usize,
    pub max_loop_finish_iterations: usize,
    pub array_alloc_count: usize,
    pub max_array_len_seen: usize,
    pub set_array_append_count: usize,
    pub set_array_overwrite_count: usize,
    pub rest_array_empty_result_count: usize,
    pub get_array_count: usize,
    pub scalar_box_get_count: usize,
    pub min_get_index: Option<i32>,
    pub max_get_index: Option<i32>,
    pub min_set_index: Option<i32>,
    pub max_set_index: Option<i32>,
    pub scalar_box_set_count: usize,
    pub out_of_bounds_count: usize,
    pub call_depth_limit_exceeded_count: usize,
    pub int_overflow_risk_count: usize,
    pub int_div_by_zero_count: usize,
    pub float_non_finite_count: usize,
    pub input_array_total_len: usize,
    pub input_array_count: usize,
    pub output_array_total_len: usize,
    pub output_array_count: usize,
    pub input_preview: Option<String>,
    pub final_result_type_tag: Option<String>,
    pub final_result_repr: Option<String>,
    pub function_reports: Vec<FunctionRuntimeReport>,
    pub function_reports_truncated: bool,
}

impl RuntimeReportDelta {
    #[allow(dead_code)]
    pub fn grouped_function_reports(&self) -> Vec<GroupedFunctionRuntimePattern> {
        let report = RuntimeReport {
            total_instructions: self.total_instructions,
            instruction_counts: self.instruction_counts.clone(),
            max_stack_size: self.max_stack_size,
            final_stack_size: self.final_stack_size,
            max_call_depth: self.max_call_depth,
            call_count: self.call_count,
            partial_application_count: self.partial_application_count,
            branch_true_count: self.branch_true_count,
            branch_false_count: self.branch_false_count,
            branch_site_outcomes: self.branch_site_outcomes.clone(),
            loop_iterations: self.loop_iterations,
            loop_finish_iterations: self.loop_finish_iterations,
            max_loop_iterations: self.max_loop_iterations,
            max_loop_finish_iterations: self.max_loop_finish_iterations,
            array_alloc_count: self.array_alloc_count,
            max_array_len_seen: self.max_array_len_seen,
            set_array_append_count: self.set_array_append_count,
            set_array_overwrite_count: self.set_array_overwrite_count,
            rest_array_empty_result_count: self.rest_array_empty_result_count,
            get_array_count: self.get_array_count,
            scalar_box_get_count: self.scalar_box_get_count,
            min_get_index: self.min_get_index,
            max_get_index: self.max_get_index,
            min_set_index: self.min_set_index,
            max_set_index: self.max_set_index,
            scalar_box_set_count: self.scalar_box_set_count,
            out_of_bounds_count: self.out_of_bounds_count,
            call_depth_limit_exceeded_count: self.call_depth_limit_exceeded_count,
            int_overflow_risk_count: self.int_overflow_risk_count,
            int_div_by_zero_count: self.int_div_by_zero_count,
            float_non_finite_count: self.float_non_finite_count,
            final_result_type_tag: self.final_result_type_tag.clone(),
            final_result_repr: self.final_result_repr.clone(),
            function_reports: self.function_reports.clone(),
            function_reports_truncated: self.function_reports_truncated,
        };
        report.grouped_function_patterns()
    }
}

impl RuntimeReport {
    pub fn difference_from(&self, baseline: &RuntimeReport) -> RuntimeReportDelta {
        let mut instruction_counts = HashMap::new();
        for (instr, count) in &self.instruction_counts {
            let base_count = baseline.instruction_counts.get(instr).copied().unwrap_or(0);
            let delta = count.saturating_sub(base_count);
            if delta > 0 {
                instruction_counts.insert(instr.clone(), delta);
            }
        }
        let function_reports = if self.function_reports.len() >= baseline.function_reports.len() {
            self.function_reports[baseline.function_reports.len()..].to_vec()
        } else {
            Vec::new()
        };

        RuntimeReportDelta {
            total_instructions: self.total_instructions.saturating_sub(baseline.total_instructions),
            instruction_counts,
            max_stack_size: self.max_stack_size.saturating_sub(baseline.max_stack_size),
            final_stack_size: self.final_stack_size.saturating_sub(baseline.final_stack_size),
            max_call_depth: self.max_call_depth.saturating_sub(baseline.max_call_depth),
            call_count: self.call_count.saturating_sub(baseline.call_count),
            partial_application_count: self.partial_application_count.saturating_sub(
                baseline.partial_application_count
            ),
            branch_true_count: self.branch_true_count.saturating_sub(baseline.branch_true_count),
            branch_false_count: self.branch_false_count.saturating_sub(baseline.branch_false_count),
            branch_site_outcomes: subtract_branch_site_outcomes(
                &self.branch_site_outcomes,
                &baseline.branch_site_outcomes
            ),
            loop_iterations: self.loop_iterations.saturating_sub(baseline.loop_iterations),
            loop_finish_iterations: self.loop_finish_iterations.saturating_sub(
                baseline.loop_finish_iterations
            ),
            max_loop_iterations: self.max_loop_iterations.saturating_sub(
                baseline.max_loop_iterations
            ),
            max_loop_finish_iterations: self.max_loop_finish_iterations.saturating_sub(
                baseline.max_loop_finish_iterations
            ),
            array_alloc_count: self.array_alloc_count.saturating_sub(baseline.array_alloc_count),
            max_array_len_seen: self.max_array_len_seen.saturating_sub(baseline.max_array_len_seen),
            set_array_append_count: self.set_array_append_count.saturating_sub(
                baseline.set_array_append_count
            ),
            set_array_overwrite_count: self.set_array_overwrite_count.saturating_sub(
                baseline.set_array_overwrite_count
            ),
            rest_array_empty_result_count: self.rest_array_empty_result_count.saturating_sub(
                baseline.rest_array_empty_result_count
            ),
            get_array_count: self.get_array_count.saturating_sub(baseline.get_array_count),
            scalar_box_get_count: self.scalar_box_get_count.saturating_sub(
                baseline.scalar_box_get_count
            ),
            min_get_index: pick_min_delta_index(self.min_get_index, baseline.min_get_index),
            max_get_index: pick_max_delta_index(self.max_get_index, baseline.max_get_index),
            min_set_index: pick_min_delta_index(self.min_set_index, baseline.min_set_index),
            max_set_index: pick_max_delta_index(self.max_set_index, baseline.max_set_index),
            scalar_box_set_count: self.scalar_box_set_count.saturating_sub(
                baseline.scalar_box_set_count
            ),
            out_of_bounds_count: self.out_of_bounds_count.saturating_sub(
                baseline.out_of_bounds_count
            ),
            call_depth_limit_exceeded_count: self.call_depth_limit_exceeded_count.saturating_sub(
                baseline.call_depth_limit_exceeded_count
            ),
            int_overflow_risk_count: self.int_overflow_risk_count.saturating_sub(
                baseline.int_overflow_risk_count
            ),
            int_div_by_zero_count: self.int_div_by_zero_count.saturating_sub(
                baseline.int_div_by_zero_count
            ),
            float_non_finite_count: self.float_non_finite_count.saturating_sub(
                baseline.float_non_finite_count
            ),
            input_array_total_len: 0,
            input_array_count: 0,
            output_array_total_len: 0,
            output_array_count: 0,
            input_preview: None,
            final_result_type_tag: self.final_result_type_tag.clone(),
            final_result_repr: self.final_result_repr.clone(),
            function_reports,
            function_reports_truncated: self.function_reports_truncated ||
            baseline.function_reports_truncated,
        }
    }

    #[allow(dead_code)]
    pub fn grouped_function_patterns(&self) -> Vec<GroupedFunctionRuntimePattern> {
        let mut groups: HashMap<
            (String, Option<String>, Option<String>),
            GroupedFunctionRuntimePattern
        > = HashMap::new();

        for call in &self.function_reports {
            let key = (
                call.function_id.clone(),
                call.report.final_result_type_tag.clone(),
                call.report.final_result_repr.clone(),
            );
            if let Some(group) = groups.get_mut(&key) {
                group.count += 1;
                if let Some(inp) = call.report.input_preview.clone() {
                    if !group.input_examples.contains(&inp) && group.input_examples.len() < 5 {
                        group.input_examples.push(inp);
                    }
                }
                accumulate_report_delta(&mut group.totals, &call.report);
                continue;
            }

            let mut totals = RuntimeReportDelta::default();
            accumulate_report_delta(&mut totals, &call.report);

            groups.insert(key, GroupedFunctionRuntimePattern {
                function_id: call.function_id.clone(),
                arity: call.arity,
                final_result_type_tag: call.report.final_result_type_tag.clone(),
                final_result_repr: call.report.final_result_repr.clone(),
                count: 1,
                input_examples: call.report.input_preview
                    .clone()
                    .map(|s| vec![s])
                    .unwrap_or_default(),
                totals,
            });
        }

        let mut out: Vec<GroupedFunctionRuntimePattern> = groups.into_values().collect();
        out.sort_by(|a, b| {
            a.function_id
                .cmp(&b.function_id)
                .then_with(|| a.final_result_repr.cmp(&b.final_result_repr))
        });
        out
    }

    #[allow(dead_code)]
    pub fn llm_anomaly_report(&self) -> LlmAnomalyReport {
        let grouped = self.grouped_function_patterns();
        let mut function_groups = Vec::with_capacity(grouped.len());
        let mut dominant_rank: usize = 0;
        let mut dominant_instr: usize = 0;
        let mut dominant_complexity = "O(1)".to_string();
        let mut dominant_confidence = 0.0_f32;
        let mut dominant_function_id: Option<String> = None;

        for group in grouped {
            let total_instructions = group.totals.total_instructions;
            let avg_instructions_per_call = if group.count == 0 {
                0.0
            } else {
                (total_instructions as f32) / (group.count as f32)
            };
            let iterative_signal =
                group.totals.loop_iterations +
                group.totals.loop_finish_iterations +
                group.totals.call_count;
            let branch_signal = group.totals.branch_true_count + group.totals.branch_false_count;
            let (
                branch_sites_total,
                branch_sites_full,
                branch_site_coverage_pct,
                branch_outcome_coverage_pct,
            ) = branch_coverage_stats(&group.totals.branch_site_outcomes);
            let vector_mutation_signal =
                group.totals.set_array_append_count +
                group.totals.set_array_overwrite_count +
                group.totals.instruction_counts.get("PopArray").copied().unwrap_or(0);
            let vector_read_signal = group.totals.get_array_count;
            let scalar_update_signal = group.totals.scalar_box_set_count;
            let scalar_read_signal = group.totals.scalar_box_get_count;
            let effective_vector_mutation_signal =
                vector_mutation_signal.saturating_sub(scalar_update_signal);
            let effective_vector_read_signal =
                vector_read_signal.saturating_sub(scalar_read_signal);
            let literal_build_signal =
                group.totals.instruction_counts.get("PushInt").copied().unwrap_or(0) +
                group.totals.instruction_counts.get("PushFloat").copied().unwrap_or(0) +
                group.totals.instruction_counts.get("PushBool").copied().unwrap_or(0) +
                group.totals.instruction_counts.get("MakeVector").copied().unwrap_or(0);
            let vector_construction_signal = group.totals.instruction_counts
                .get("MakeVector")
                .copied()
                .unwrap_or(0);
            let input_array_avg_len = if group.totals.input_array_count == 0 {
                0.0
            } else {
                (group.totals.input_array_total_len as f32) /
                    (group.totals.input_array_count as f32)
            };
            let output_array_avg_len = if group.totals.output_array_count == 0 {
                0.0
            } else {
                (group.totals.output_array_total_len as f32) /
                    (group.totals.output_array_count as f32)
            };
            let writes_per_input_elem = if group.totals.input_array_total_len == 0 {
                0.0
            } else {
                (effective_vector_mutation_signal as f32) /
                    (group.totals.input_array_total_len as f32)
            };
            let reads_per_input_elem = if group.totals.input_array_total_len == 0 {
                0.0
            } else {
                (effective_vector_read_signal as f32) / (group.totals.input_array_total_len as f32)
            };
            let update_coverage_ratio = if input_array_avg_len <= 0.0 {
                0.0
            } else {
                (output_array_avg_len / input_array_avg_len).max(0.0)
            };
            let returns_array = group.final_result_type_tag.as_deref() == Some("Array");
            let coverage_interpretation = classify_coverage_interpretation(
                returns_array,
                reads_per_input_elem,
                writes_per_input_elem,
                input_array_avg_len,
                output_array_avg_len
            );
            let get_index_range = match (group.totals.min_get_index, group.totals.max_get_index) {
                (Some(a), Some(b)) => Some((a, b)),
                _ => None,
            };
            let set_index_range = match (group.totals.min_set_index, group.totals.max_set_index) {
                (Some(a), Some(b)) => Some((a, b)),
                _ => None,
            };
            let (algorithm_style, mutation_ratio, literal_ratio) = classify_algorithm_style(
                effective_vector_mutation_signal,
                vector_construction_signal,
                iterative_signal,
                branch_signal,
                effective_vector_read_signal
            );
            let (potential_complexity, complexity_confidence, complexity_rank) =
                infer_potential_complexity(
                    input_array_avg_len,
                    iterative_signal,
                    group.totals.call_count,
                    reads_per_input_elem,
                    writes_per_input_elem,
                    branch_signal
                );

            let mut suspicious_flags = Vec::new();
            if
                returns_array &&
                iterative_signal == 0 &&
                vector_mutation_signal == 0 &&
                branch_signal == 0
            {
                suspicious_flags.push("array_result_without_iteration_or_mutation".to_string());
            }
            if returns_array && literal_build_signal > 0 && iterative_signal == 0 {
                suspicious_flags.push("literal_array_construction_pattern".to_string());
            }
            if avg_instructions_per_call < 4.0 && group.count > 0 {
                suspicious_flags.push("very_low_work_per_call".to_string());
            }
            if returns_array && input_array_avg_len > 0.0 && writes_per_input_elem < 0.8 {
                suspicious_flags.push(
                    "low_write_to_input_ratio_possible_partial_traversal".to_string()
                );
            }
            if input_array_avg_len >= 1.0 && writes_per_input_elem < 0.9 {
                if
                    algorithm_style == "mutable_style" ||
                    algorithm_style == "hybrid_mutation_dominant"
                {
                    if coverage_interpretation != "filter_like_expected_partial_write" {
                        suspicious_flags.push("incomplete_update_coverage".to_string());
                    } else {
                        suspicious_flags.push(
                            "incomplete_update_coverage_filter_like_expected".to_string()
                        );
                    }
                } else if algorithm_style == "hybrid_balanced" {
                    if coverage_interpretation != "filter_like_expected_partial_write" {
                        suspicious_flags.push(
                            "incomplete_update_coverage_low_confidence".to_string()
                        );
                    } else {
                        suspicious_flags.push(
                            "incomplete_update_coverage_filter_like_expected".to_string()
                        );
                    }
                }
            }
            if returns_array && input_array_avg_len > 0.0 && reads_per_input_elem < 0.8 {
                suspicious_flags.push(
                    "low_read_to_input_ratio_possible_partial_traversal".to_string()
                );
            }
            if
                returns_array &&
                input_array_avg_len > 1.0 &&
                output_array_avg_len + 0.5 < input_array_avg_len
            {
                suspicious_flags.push("output_shorter_than_input_possible_drop".to_string());
            }
            if let Some((min_i, _)) = get_index_range {
                if min_i > 0 {
                    suspicious_flags.push(
                        "get_index_starts_above_zero_possible_off_by_one".to_string()
                    );
                }
            }
            if returns_array || group.final_result_type_tag.as_deref() == Some("Int") {
                if input_array_avg_len >= 1.0 {
                    if let Some((_, max_i)) = get_index_range {
                        let expected_max = (input_array_avg_len.ceil() as i32) - 1;
                        if max_i < expected_max {
                            suspicious_flags.push("incomplete_index_coverage".to_string());
                        }
                    }
                }
            }
            if group.totals.int_overflow_risk_count > 0 {
                suspicious_flags.push("int_overflow_risk_detected".to_string());
            }
            if group.totals.int_div_by_zero_count > 0 {
                suspicious_flags.push("int_div_by_zero_detected".to_string());
            }
            if group.totals.out_of_bounds_count > 0 {
                suspicious_flags.push("out_of_bounds_detected".to_string());
            }
            if group.totals.call_depth_limit_exceeded_count > 0 {
                suspicious_flags.push("call_depth_limit_exceeded_detected".to_string());
            }
            if group.totals.float_non_finite_count > 0 {
                suspicious_flags.push("float_non_finite_detected".to_string());
            }
            if complexity_rank >= 2 {
                suspicious_flags.push("potential_superlinear_growth".to_string());
            }

            if
                complexity_rank > dominant_rank ||
                (complexity_rank == dominant_rank && total_instructions > dominant_instr)
            {
                dominant_rank = complexity_rank;
                dominant_instr = total_instructions;
                dominant_complexity = potential_complexity.to_string();
                dominant_confidence = complexity_confidence;
                dominant_function_id = Some(group.function_id.clone());
            }

            function_groups.push(FunctionAnomalySignals {
                function_id: group.function_id,
                final_result_type_tag: group.final_result_type_tag,
                final_result_repr: group.final_result_repr,
                input_examples: group.input_examples,
                count: group.count,
                total_instructions,
                avg_instructions_per_call,
                iterative_signal,
                branch_signal,
                branch_sites_total,
                branch_sites_full,
                branch_site_coverage_pct,
                branch_outcome_coverage_pct,
                vector_mutation_signal,
                vector_read_signal,
                scalar_update_signal,
                scalar_read_signal,
                literal_build_signal,
                input_array_avg_len,
                output_array_avg_len,
                writes_per_input_elem,
                reads_per_input_elem,
                update_coverage_ratio,
                coverage_interpretation: coverage_interpretation.to_string(),
                get_index_range,
                set_index_range,
                out_of_bounds_count: group.totals.out_of_bounds_count,
                call_depth_limit_exceeded_count: group.totals.call_depth_limit_exceeded_count,
                int_overflow_risk_count: group.totals.int_overflow_risk_count,
                int_div_by_zero_count: group.totals.int_div_by_zero_count,
                float_non_finite_count: group.totals.float_non_finite_count,
                algorithm_style: algorithm_style.to_string(),
                mutation_ratio,
                literal_ratio,
                potential_complexity: potential_complexity.to_string(),
                complexity_confidence,
                suspicious_flags,
            });
        }

        let (
            root_branch_sites_total,
            root_branch_sites_full,
            root_branch_site_coverage_pct,
            root_branch_outcome_coverage_pct,
        ) = branch_coverage_stats(&self.branch_site_outcomes);

        LlmAnomalyReport {
            final_result_type_tag: self.final_result_type_tag.clone(),
            final_result_repr: self.final_result_repr.clone(),
            total_instructions: self.total_instructions,
            int_overflow_risk_count: self.int_overflow_risk_count,
            int_div_by_zero_count: self.int_div_by_zero_count,
            float_non_finite_count: self.float_non_finite_count,
            branch_sites_total: root_branch_sites_total,
            branch_sites_full: root_branch_sites_full,
            branch_site_coverage_pct: root_branch_site_coverage_pct,
            branch_outcome_coverage_pct: root_branch_outcome_coverage_pct,
            dominant_potential_complexity: dominant_complexity,
            dominant_complexity_function_id: dominant_function_id,
            dominant_complexity_confidence: dominant_confidence,
            function_groups,
        }
    }
}

#[allow(dead_code)]
pub fn format_anomaly_report_text(
    final_result: Option<&str>,
    runtime_error: Option<&str>,
    report: &LlmAnomalyReport
) -> String {
    let mut out = String::new();
    // out.push_str(&format!("status={}\n", if runtime_error.is_some() { "error" } else { "ok" }));
    let final_result = truncate_middle(
        final_result.unwrap_or("-"),
        MAX_RESULT_REPR_LEN,
        RESULT_REPR_EDGE_LEN
    );
    out.push_str(&format!("final_result={}\n", final_result));
    out.push_str(&format!("runtime_error={}\n", runtime_error.unwrap_or("-")));
    out.push_str(
        &format!("final_result_type={}\n", report.final_result_type_tag.as_deref().unwrap_or("-"))
    );
    out.push_str(
        &format!("final_result_repr={}\n", report.final_result_repr.as_deref().unwrap_or("-"))
    );
    out.push_str(&format!("total_instructions={}\n", report.total_instructions));
    out.push_str(&format!("int_overflow_risk_count={}\n", report.int_overflow_risk_count));
    out.push_str(&format!("int_div_by_zero_count={}\n", report.int_div_by_zero_count));
    out.push_str(&format!("float_non_finite_count={}\n", report.float_non_finite_count));
    out.push_str(
        &format!(
            "branch_coverage_sites={}/{} ({:.3})\n",
            report.branch_sites_full,
            report.branch_sites_total,
            report.branch_site_coverage_pct
        )
    );
    out.push_str(&format!("branch_coverage_outcomes={:.3}\n", report.branch_outcome_coverage_pct));
    out.push_str(
        &format!("dominant_potential_complexity={}\n", report.dominant_potential_complexity)
    );
    out.push_str(
        &format!(
            "dominant_complexity_function_id={}\n",
            report.dominant_complexity_function_id.as_deref().unwrap_or("-")
        )
    );
    out.push_str(
        &format!("dominant_complexity_confidence={:.3}\n", report.dominant_complexity_confidence)
    );
    out.push_str("functions:\n");

    for g in &report.function_groups {
        let get_idx = match g.get_index_range {
            Some((a, b)) => format!("{}..{}", a, b),
            None => "-".to_string(),
        };
        let set_idx = match g.set_index_range {
            Some((a, b)) => format!("{}..{}", a, b),
            None => "-".to_string(),
        };
        let flags = if g.suspicious_flags.is_empty() {
            "-".to_string()
        } else {
            g.suspicious_flags.join("|")
        };
        let inputs = if g.input_examples.is_empty() {
            "-".to_string()
        } else {
            g.input_examples.join(" || ")
        };

        out.push_str(
            &format!(
                "  fn id={} type={} result={} inputs={} count={} instr={} style={} iter={} branch={} branch_sites={}/{} branch_site_cov={:.3} branch_outcome_cov={:.3} read={} write={} in_avg={:.3} out_avg={:.3} rpi={:.3} wpi={:.3} idx_get={} idx_set={} oob={} depth_limit={} int_ovf={} int_div0={} float_nf={} flags={}\n",
                g.function_id,
                g.final_result_type_tag.as_deref().unwrap_or("-"),
                g.final_result_repr.as_deref().unwrap_or("-"),
                inputs,
                g.count,
                g.total_instructions,
                format!(
                    "{} complexity={} conf={:.3}",
                    g.algorithm_style,
                    g.potential_complexity,
                    g.complexity_confidence
                ),
                g.iterative_signal,
                g.branch_signal,
                g.branch_sites_full,
                g.branch_sites_total,
                g.branch_site_coverage_pct,
                g.branch_outcome_coverage_pct,
                g.vector_read_signal,
                g.vector_mutation_signal,
                g.input_array_avg_len,
                g.output_array_avg_len,
                g.reads_per_input_elem,
                g.writes_per_input_elem,
                // coverage details in compact form
                // ucr = update coverage ratio, cint = coverage interpretation
                // appended below for readability
                get_idx,
                set_idx,
                g.out_of_bounds_count,
                g.call_depth_limit_exceeded_count,
                g.int_overflow_risk_count,
                g.int_div_by_zero_count,
                g.float_non_finite_count,
                flags
            )
        );
        out.push_str(
            &format!(
                "    scalar: read={} update={}\n",
                g.scalar_read_signal,
                g.scalar_update_signal
            )
        );
        out.push_str(
            &format!(
                "    coverage: update_ratio={:.3} interpretation={}\n",
                g.update_coverage_ratio,
                g.coverage_interpretation
            )
        );
    }

    out
}

fn classify_coverage_interpretation(
    returns_array: bool,
    reads_per_input_elem: f32,
    writes_per_input_elem: f32,
    input_array_avg_len: f32,
    output_array_avg_len: f32
) -> &'static str {
    if input_array_avg_len <= 0.0 {
        return "no_array_input";
    }
    let output_ratio = if input_array_avg_len <= 0.0 {
        0.0
    } else {
        output_array_avg_len / input_array_avg_len
    };
    if reads_per_input_elem >= 0.9 && writes_per_input_elem >= 0.9 {
        return "full_traversal_full_update";
    }
    if
        returns_array &&
        reads_per_input_elem >= 0.9 &&
        writes_per_input_elem < 0.9 &&
        output_ratio < 0.95
    {
        return "filter_like_expected_partial_write";
    }
    if !returns_array && reads_per_input_elem >= 0.9 && writes_per_input_elem < 0.9 {
        return "reducer_like_full_read_low_write";
    }
    if reads_per_input_elem < 0.9 && writes_per_input_elem < 0.9 {
        return "partial_traversal_possible_issue";
    }
    if reads_per_input_elem >= 0.9 && writes_per_input_elem < 0.9 {
        return "full_read_partial_write_mixed";
    }
    "mixed_coverage_pattern"
}

fn branch_coverage_stats(
    outcomes: &HashMap<usize, BranchSiteCoverage>
) -> (usize, usize, f32, f32) {
    let total = outcomes.len();
    if total == 0 {
        return (0, 0, 1.0, 1.0);
    }
    let mut full = 0usize;
    let mut seen_outcomes = 0usize;
    for v in outcomes.values() {
        let t = v.true_hits > 0;
        let f = v.false_hits > 0;
        if t && f {
            full += 1;
        }
        if t {
            seen_outcomes += 1;
        }
        if f {
            seen_outcomes += 1;
        }
    }
    (total, full, (full as f32) / (total as f32), (seen_outcomes as f32) / ((2 * total) as f32))
}

fn subtract_branch_site_outcomes(
    current: &HashMap<usize, BranchSiteCoverage>,
    baseline: &HashMap<usize, BranchSiteCoverage>
) -> HashMap<usize, BranchSiteCoverage> {
    let mut out = HashMap::new();
    for (id, cur) in current {
        let base = baseline.get(id).cloned().unwrap_or_default();
        let true_hits = cur.true_hits.saturating_sub(base.true_hits);
        let false_hits = cur.false_hits.saturating_sub(base.false_hits);
        if true_hits > 0 || false_hits > 0 {
            out.insert(*id, BranchSiteCoverage {
                true_hits,
                false_hits,
            });
        }
    }
    out
}

fn infer_potential_complexity(
    input_array_avg_len: f32,
    iterative_signal: usize,
    call_count: usize,
    reads_per_input_elem: f32,
    writes_per_input_elem: f32,
    branch_signal: usize
) -> (&'static str, f32, usize) {
    let n = input_array_avg_len.max(1.0);
    let iter_per_elem = (iterative_signal as f32) / n;
    let rw_per_elem = reads_per_input_elem + writes_per_input_elem;
    let call_per_elem = (call_count as f32) / n;
    let superlinear_density = if n > 0.0 { rw_per_elem / n } else { 0.0 };
    let iter_density = if n > 0.0 { iter_per_elem / n } else { 0.0 };

    // Not enough size signal (scalar boxes / tiny arrays): avoid false superlinear labels.
    if input_array_avg_len <= 1.5 {
        if iterative_signal == 0 && call_count <= 1 {
            return ("O(1)", 0.8, 0);
        }
        return ("O(1)~O(n) low-signal", 0.25, 0);
    }

    if iterative_signal == 0 && call_count <= 1 && rw_per_elem <= 0.2 {
        return ("O(1)", 0.9, 0);
    }
    // Strong quadratic cue:
    // for O(n^2), work per input element grows ~O(n), so normalized density vs n stays high.
    if input_array_avg_len >= 6.0 && (superlinear_density >= 0.3 || iter_density >= 0.3) {
        return ("O(n^2+)?", 0.72, 3);
    }
    // Typical single/triple pass array processing stays linear even with VM overhead.
    if iter_per_elem <= 12.0 && rw_per_elem <= 4.5 && call_per_elem <= 8.0 {
        return ("O(n)", 0.8, 1);
    }
    if iter_per_elem <= 3.0 && rw_per_elem <= 3.5 && call_per_elem <= 2.5 {
        return ("O(n)", 0.75, 1);
    }
    if
        input_array_avg_len >= 4.0 &&
        iter_per_elem <= 20.0 &&
        rw_per_elem <= 10.0 &&
        branch_signal > 0
    {
        return ("O(n log n)?", 0.45, 2);
    }
    // Only call this potentially quadratic when size signal is meaningful.
    if input_array_avg_len >= 6.0 && (iter_per_elem > 20.0 || rw_per_elem > 10.0) {
        return ("O(n^2+)?", 0.55, 3);
    }
    ("O(n log n)?", 0.35, 2)
}

fn classify_algorithm_style(
    vector_mutation_signal: usize,
    vector_construction_signal: usize,
    iterative_signal: usize,
    branch_signal: usize,
    vector_read_signal: usize
) -> (&'static str, f32, f32) {
    let mut_sig = vector_mutation_signal as f32;
    let lit_sig = vector_construction_signal as f32;
    let denom = mut_sig + lit_sig;
    let mutation_ratio = if denom > 0.0 { mut_sig / denom } else { 0.0 };
    let literal_ratio = if denom > 0.0 { lit_sig / denom } else { 0.0 };

    if vector_mutation_signal > 0 && vector_construction_signal == 0 {
        return ("mutable_style", mutation_ratio, literal_ratio);
    }
    if vector_mutation_signal == 0 && vector_construction_signal > 0 {
        if iterative_signal > 0 || branch_signal > 0 || vector_read_signal > 0 {
            return ("persistent_style", mutation_ratio, literal_ratio);
        }
        return ("read_only_or_constant_style", mutation_ratio, literal_ratio);
    }
    if vector_mutation_signal > 0 && vector_construction_signal > 0 {
        if mutation_ratio > 0.7 {
            return ("hybrid_mutation_dominant", mutation_ratio, literal_ratio);
        }
        if mutation_ratio < 0.3 {
            return ("hybrid_persistent_dominant", mutation_ratio, literal_ratio);
        }
        return ("hybrid_balanced", mutation_ratio, literal_ratio);
    }
    ("read_only_or_constant_style", mutation_ratio, literal_ratio)
}

fn accumulate_report_delta(total: &mut RuntimeReportDelta, delta: &RuntimeReportDelta) {
    total.total_instructions += delta.total_instructions;
    total.max_stack_size += delta.max_stack_size;
    total.final_stack_size += delta.final_stack_size;
    total.max_call_depth += delta.max_call_depth;
    total.call_count += delta.call_count;
    total.partial_application_count += delta.partial_application_count;
    total.branch_true_count += delta.branch_true_count;
    total.branch_false_count += delta.branch_false_count;
    for (id, c) in &delta.branch_site_outcomes {
        let e = total.branch_site_outcomes.entry(*id).or_default();
        e.true_hits += c.true_hits;
        e.false_hits += c.false_hits;
    }
    total.loop_iterations += delta.loop_iterations;
    total.loop_finish_iterations += delta.loop_finish_iterations;
    total.max_loop_iterations += delta.max_loop_iterations;
    total.max_loop_finish_iterations += delta.max_loop_finish_iterations;
    total.array_alloc_count += delta.array_alloc_count;
    total.max_array_len_seen += delta.max_array_len_seen;
    total.set_array_append_count += delta.set_array_append_count;
    total.set_array_overwrite_count += delta.set_array_overwrite_count;
    total.rest_array_empty_result_count += delta.rest_array_empty_result_count;
    total.get_array_count += delta.get_array_count;
    total.scalar_box_get_count += delta.scalar_box_get_count;
    total.min_get_index = min_opt_i32(total.min_get_index, delta.min_get_index);
    total.max_get_index = max_opt_i32(total.max_get_index, delta.max_get_index);
    total.min_set_index = min_opt_i32(total.min_set_index, delta.min_set_index);
    total.max_set_index = max_opt_i32(total.max_set_index, delta.max_set_index);
    total.scalar_box_set_count += delta.scalar_box_set_count;
    total.out_of_bounds_count += delta.out_of_bounds_count;
    total.call_depth_limit_exceeded_count += delta.call_depth_limit_exceeded_count;
    total.int_overflow_risk_count += delta.int_overflow_risk_count;
    total.int_div_by_zero_count += delta.int_div_by_zero_count;
    total.float_non_finite_count += delta.float_non_finite_count;
    total.input_array_total_len += delta.input_array_total_len;
    total.input_array_count += delta.input_array_count;
    total.output_array_total_len += delta.output_array_total_len;
    total.output_array_count += delta.output_array_count;
    total.final_result_type_tag = delta.final_result_type_tag.clone();
    total.final_result_repr = delta.final_result_repr.clone();

    for (instr, count) in &delta.instruction_counts {
        *total.instruction_counts.entry(instr.clone()).or_insert(0) += *count;
    }
    total.function_reports_truncated |= delta.function_reports_truncated;
}

fn merge_runtime_report_into(total: &mut RuntimeReport, src: &RuntimeReport) {
    total.total_instructions += src.total_instructions;
    for (instr, count) in &src.instruction_counts {
        *total.instruction_counts.entry(instr.clone()).or_insert(0) += *count;
    }
    total.max_stack_size = total.max_stack_size.max(src.max_stack_size);
    total.max_call_depth = total.max_call_depth.max(src.max_call_depth);
    total.call_count += src.call_count;
    total.partial_application_count += src.partial_application_count;
    total.branch_true_count += src.branch_true_count;
    total.branch_false_count += src.branch_false_count;
    for (id, c) in &src.branch_site_outcomes {
        let e = total.branch_site_outcomes.entry(*id).or_default();
        e.true_hits += c.true_hits;
        e.false_hits += c.false_hits;
    }
    total.loop_iterations += src.loop_iterations;
    total.loop_finish_iterations += src.loop_finish_iterations;
    total.max_loop_iterations = total.max_loop_iterations.max(src.max_loop_iterations);
    total.max_loop_finish_iterations = total.max_loop_finish_iterations.max(
        src.max_loop_finish_iterations
    );
    total.array_alloc_count += src.array_alloc_count;
    total.max_array_len_seen = total.max_array_len_seen.max(src.max_array_len_seen);
    total.set_array_append_count += src.set_array_append_count;
    total.set_array_overwrite_count += src.set_array_overwrite_count;
    total.rest_array_empty_result_count += src.rest_array_empty_result_count;
    total.get_array_count += src.get_array_count;
    total.scalar_box_get_count += src.scalar_box_get_count;
    total.min_get_index = min_opt_i32(total.min_get_index, src.min_get_index);
    total.max_get_index = max_opt_i32(total.max_get_index, src.max_get_index);
    total.min_set_index = min_opt_i32(total.min_set_index, src.min_set_index);
    total.max_set_index = max_opt_i32(total.max_set_index, src.max_set_index);
    total.scalar_box_set_count += src.scalar_box_set_count;
    total.out_of_bounds_count += src.out_of_bounds_count;
    total.call_depth_limit_exceeded_count += src.call_depth_limit_exceeded_count;
    total.int_overflow_risk_count += src.int_overflow_risk_count;
    total.int_div_by_zero_count += src.int_div_by_zero_count;
    total.float_non_finite_count += src.float_non_finite_count;
    total.function_reports.extend(src.function_reports.clone());
    compact_function_reports_if_needed(total);
    total.function_reports_truncated |= src.function_reports_truncated;
}

fn compact_function_reports_if_needed(report: &mut RuntimeReport) {
    if report.function_reports.len() <= MAX_FUNCTION_REPORT_ENTRIES {
        return;
    }

    report.function_reports.retain(|f| function_report_has_anomaly(&f.report));

    if report.function_reports.len() > MAX_FUNCTION_REPORT_ENTRIES {
        report.function_reports.truncate(MAX_FUNCTION_REPORT_ENTRIES);
    }
    report.function_reports_truncated = true;
}

fn function_report_has_anomaly(r: &RuntimeReportDelta) -> bool {
    if
        r.out_of_bounds_count > 0 ||
        r.call_depth_limit_exceeded_count > 0 ||
        r.int_overflow_risk_count > 0 ||
        r.int_div_by_zero_count > 0 ||
        r.float_non_finite_count > 0
    {
        return true;
    }

    let iterative_signal = r.loop_iterations + r.loop_finish_iterations + r.call_count;
    let branch_signal = r.branch_true_count + r.branch_false_count;
    let vector_mutation_signal =
        r.set_array_append_count +
        r.set_array_overwrite_count +
        r.instruction_counts.get("PopArray").copied().unwrap_or(0);
    let vector_read_signal = r.get_array_count;
    let scalar_update_signal = r.scalar_box_set_count;
    let scalar_read_signal = r.scalar_box_get_count;
    let effective_vector_mutation_signal =
        vector_mutation_signal.saturating_sub(scalar_update_signal);
    let effective_vector_read_signal = vector_read_signal.saturating_sub(scalar_read_signal);
    let literal_build_signal =
        r.instruction_counts.get("PushInt").copied().unwrap_or(0) +
        r.instruction_counts.get("PushFloat").copied().unwrap_or(0) +
        r.instruction_counts.get("PushBool").copied().unwrap_or(0) +
        r.instruction_counts.get("MakeVector").copied().unwrap_or(0);
    let vector_construction_signal = r.instruction_counts.get("MakeVector").copied().unwrap_or(0);
    let input_array_avg_len = if r.input_array_count == 0 {
        0.0
    } else {
        (r.input_array_total_len as f32) / (r.input_array_count as f32)
    };
    let output_array_avg_len = if r.output_array_count == 0 {
        0.0
    } else {
        (r.output_array_total_len as f32) / (r.output_array_count as f32)
    };
    let writes_per_input_elem = if r.input_array_total_len == 0 {
        0.0
    } else {
        (effective_vector_mutation_signal as f32) / (r.input_array_total_len as f32)
    };
    let reads_per_input_elem = if r.input_array_total_len == 0 {
        0.0
    } else {
        (effective_vector_read_signal as f32) / (r.input_array_total_len as f32)
    };
    let (style, _, _) = classify_algorithm_style(
        effective_vector_mutation_signal,
        vector_construction_signal,
        iterative_signal,
        branch_signal,
        effective_vector_read_signal
    );
    let returns_array = r.final_result_type_tag.as_deref() == Some("Array");

    if returns_array && iterative_signal == 0 && vector_mutation_signal == 0 && branch_signal == 0 {
        return true;
    }
    if returns_array && literal_build_signal > 0 && iterative_signal == 0 {
        return true;
    }
    if r.total_instructions < 4 {
        return true;
    }
    if returns_array && input_array_avg_len > 0.0 && writes_per_input_elem < 0.8 {
        return true;
    }
    if input_array_avg_len >= 1.0 && writes_per_input_elem < 0.9 {
        if
            style == "mutable_style" ||
            style == "hybrid_mutation_dominant" ||
            style == "hybrid_balanced"
        {
            return true;
        }
    }
    if returns_array && input_array_avg_len > 0.0 && reads_per_input_elem < 0.8 {
        return true;
    }
    if
        returns_array &&
        input_array_avg_len > 1.0 &&
        output_array_avg_len + 0.5 < input_array_avg_len
    {
        return true;
    }
    if let Some(min_i) = r.min_get_index {
        if min_i > 0 {
            return true;
        }
    }
    if returns_array || r.final_result_type_tag.as_deref() == Some("Int") {
        if input_array_avg_len >= 1.0 {
            if let Some(max_i) = r.max_get_index {
                let expected_max = (input_array_avg_len.ceil() as i32) - 1;
                if max_i < expected_max {
                    return true;
                }
            }
        }
    }

    false
}

fn format_input_preview(args: &[BiteCodeEvaluated]) -> String {
    let s = args
        .iter()
        .map(|x| {
            truncate_middle(&format_value_preview(x), MAX_INPUT_PREVIEW_LEN, INPUT_PREVIEW_EDGE_LEN)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let s = truncate_middle(&s, MAX_INPUT_PREVIEW_LEN, INPUT_PREVIEW_EDGE_LEN);
    format!("[{}]", s)
}

fn format_value_preview(value: &BiteCodeEvaluated) -> String {
    match value {
        BiteCodeEvaluated::Bool(v) => format!("{}", v),
        BiteCodeEvaluated::Int(v) => format!("{}", v),
        BiteCodeEvaluated::Float(v) => format!("{:?}", v),
        BiteCodeEvaluated::Function(_, _, _, _) => "Function".to_string(),
        BiteCodeEvaluated::Array(arr) => {
            let elems = arr
                .borrow()
                .iter()
                .map(|x| {
                    truncate_middle(
                        &format_value_preview(x),
                        MAX_INNER_VALUE_PREVIEW_LEN,
                        INNER_VALUE_PREVIEW_EDGE_LEN
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("[{}]", elems)
        }
    }
}

fn truncate_middle(s: &str, max_len: usize, edge_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let edge = edge_len.min(chars.len() / 2);
    let prefix: String = chars[..edge].iter().collect();
    let suffix: String = chars[chars.len() - edge..].iter().collect();
    format!("{}...{}", prefix, suffix)
}

fn min_opt_i32(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

fn max_opt_i32(a: Option<i32>, b: Option<i32>) -> Option<i32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

fn pick_min_delta_index(current: Option<i32>, baseline: Option<i32>) -> Option<i32> {
    match (current, baseline) {
        (Some(c), Some(b)) => {
            if c < b { Some(c) } else { None }
        }
        (Some(c), None) => Some(c),
        _ => None,
    }
}

fn pick_max_delta_index(current: Option<i32>, baseline: Option<i32>) -> Option<i32> {
    match (current, baseline) {
        (Some(c), Some(b)) => {
            if c > b { Some(c) } else { None }
        }
        (Some(c), None) => Some(c),
        _ => None,
    }
}

fn instruction_name(instr: &Instruction) -> &'static str {
    match instr {
        Instruction::PushInt(_) => "PushInt",
        Instruction::PushFloat(_) => "PushFloat",
        Instruction::PushBool(_) => "PushBool",
        Instruction::StoreVar(_) => "StoreVar",
        Instruction::LoadVar(_) => "LoadVar",
        Instruction::MakeLambda(_, _) => "MakeLambda",
        Instruction::MakeLambdaUid(_, _, _) => "MakeLambda",
        Instruction::Call(_) => "Call",
        Instruction::MakeVector(_) => "MakeVector",
        Instruction::If(_, _, _) => "If",
        Instruction::Loop(_, _, _) => "Loop",
        Instruction::LoopFinish(_, _) => "LoopFinish",
        Instruction::Length => "Length",
        Instruction::Add => "Add",
        Instruction::AddF => "AddF",
        Instruction::Mult => "Mult",
        Instruction::MultF => "MultF",
        Instruction::Div => "Div",
        Instruction::DivF => "DivF",
        Instruction::Sub => "Sub",
        Instruction::SubF => "SubF",
        Instruction::Mod => "Mod",
        Instruction::ModF => "ModF",
        Instruction::Pop => "Pop",
        Instruction::Lt => "Lt",
        Instruction::Gt => "Gt",
        Instruction::Lte => "Lte",
        Instruction::Gte => "Gte",
        Instruction::Eq => "Eq",
        Instruction::Not => "Not",
        Instruction::EqBool => "EqBool",
        Instruction::EqF => "EqF",
        Instruction::GtF => "GtF",
        Instruction::LtF => "LtF",
        Instruction::GteF => "GteF",
        Instruction::LteF => "LteF",
        Instruction::BitXor => "BitXor",
        Instruction::BitRs => "BitRs",
        Instruction::BitLs => "BitLs",
        Instruction::BitNot => "BitNot",
        Instruction::BitOr => "BitOr",
        Instruction::BitAnd => "BitAnd",
        Instruction::SetArray => "SetArray",
        Instruction::GetArray => "GetArray",
        Instruction::PopArray => "PopArray",
        Instruction::RestArray => "RestArray",
        Instruction::IntToFloat => "IntToFloat",
        Instruction::FloatToInt => "FloatToInt",
    }
}

#[allow(dead_code)]
fn value_type_tag(v: &BiteCodeEvaluated) -> &'static str {
    match v {
        BiteCodeEvaluated::Bool(_) => "Bool",
        BiteCodeEvaluated::Int(_) => "Int",
        BiteCodeEvaluated::Float(_) => "Float",
        BiteCodeEvaluated::Function(_, _, _, _) => "Function",
        BiteCodeEvaluated::Array(_) => "Array",
    }
}

fn function_id(uid: usize) -> String {
    format!("λ {}", uid)
}

fn assign_lambda_uids(code: Vec<Instruction>, next_uid: &mut usize) -> Vec<Instruction> {
    code.into_iter()
        .map(|instr| {
            match instr {
                Instruction::MakeLambda(params, body) => {
                    let uid = *next_uid;
                    *next_uid += 1;
                    Instruction::MakeLambdaUid(uid, params, assign_lambda_uids(body, next_uid))
                }
                Instruction::MakeLambdaUid(uid, params, body) => {
                    Instruction::MakeLambdaUid(uid, params, assign_lambda_uids(body, next_uid))
                }
                Instruction::If(branch_id, then_branch, else_branch) =>
                    Instruction::If(
                        branch_id,
                        assign_lambda_uids(then_branch, next_uid),
                        assign_lambda_uids(else_branch, next_uid)
                    ),
                Instruction::Loop(start, end, func) =>
                    Instruction::Loop(
                        assign_lambda_uids(start, next_uid),
                        assign_lambda_uids(end, next_uid),
                        assign_lambda_uids(func, next_uid)
                    ),
                Instruction::LoopFinish(cond, func) =>
                    Instruction::LoopFinish(
                        assign_lambda_uids(cond, next_uid),
                        assign_lambda_uids(func, next_uid)
                    ),
                other => other,
            }
        })
        .collect()
}

fn assign_branch_uids(code: Vec<Instruction>, next_branch_id: &mut usize) -> Vec<Instruction> {
    code.into_iter()
        .map(|instr| {
            match instr {
                Instruction::If(_, then_branch, else_branch) => {
                    let id = *next_branch_id;
                    *next_branch_id += 1;
                    Instruction::If(
                        id,
                        assign_branch_uids(then_branch, next_branch_id),
                        assign_branch_uids(else_branch, next_branch_id)
                    )
                }
                Instruction::MakeLambda(params, body) => {
                    Instruction::MakeLambda(params, assign_branch_uids(body, next_branch_id))
                }
                Instruction::MakeLambdaUid(uid, params, body) => {
                    Instruction::MakeLambdaUid(
                        uid,
                        params,
                        assign_branch_uids(body, next_branch_id)
                    )
                }
                Instruction::Loop(start, end, func) =>
                    Instruction::Loop(
                        assign_branch_uids(start, next_branch_id),
                        assign_branch_uids(end, next_branch_id),
                        assign_branch_uids(func, next_branch_id)
                    ),
                Instruction::LoopFinish(cond, func) =>
                    Instruction::LoopFinish(
                        assign_branch_uids(cond, next_branch_id),
                        assign_branch_uids(func, next_branch_id)
                    ),
                other => other,
            }
        })
        .collect()
}

pub struct VM {
    stack: Vec<BiteCodeEvaluated>,
    locals: Rc<RefCell<BiteCodeEnv>>, // or Rc<RefCell<_>> if needed
    call_depth: usize,
    report: RuntimeReport,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::with_capacity(64), // Pre-allocate stack space
            locals: Rc::new(RefCell::new(BiteCodeEnv::new())),
            call_depth: 0,
            report: RuntimeReport::default(),
        }
    }

    fn bump_stack_peak(&mut self) {
        self.report.max_stack_size = self.report.max_stack_size.max(self.stack.len());
    }

    fn observe_array_len(&mut self, len: usize) {
        self.report.max_array_len_seen = self.report.max_array_len_seen.max(len);
    }

    #[allow(dead_code)]
    pub fn runtime_report(&self) -> &RuntimeReport {
        &self.report
    }

    pub fn run(&mut self, code: &[Instruction]) -> Result<(), String> {
        if self.call_depth > MAXIMUM_CALL_STACK_DEPTH_LIMIT {
            self.report.call_depth_limit_exceeded_count += 1;
            return Err("Error! Maximum stack depth limit exceeded".to_string());
        }
        self.call_depth += 1;
        self.report.max_call_depth = self.report.max_call_depth.max(self.call_depth);

        let mut current_pc = 0usize;
        let mut current_instr = "ENTRY";
        let run_result = catch_unwind(
            AssertUnwindSafe(
                || -> Result<(), String> {
                    for (pc, instr) in code.iter().enumerate() {
                        current_pc = pc;
                        current_instr = instruction_name(instr);
                        self.report.total_instructions += 1;
                        *self.report.instruction_counts
                            .entry(current_instr.to_string())
                            .or_insert(0) += 1;
                        match instr {
                            Instruction::GetArray => {
                                let index_val = self.stack.pop().ok_or("stack underflow (index)")?;
                                let array_val = self.stack.pop().ok_or("stack underflow (vector)")?;
                                match (array_val, index_val) {
                                    (BiteCodeEvaluated::Array(arr), BiteCodeEvaluated::Int(i)) => {
                                        self.report.get_array_count += 1;
                                        self.report.min_get_index = min_opt_i32(
                                            self.report.min_get_index,
                                            Some(i)
                                        );
                                        self.report.max_get_index = max_opt_i32(
                                            self.report.max_get_index,
                                            Some(i)
                                        );
                                        let r = arr.borrow();
                                        if r.len() == 1 && i == 0 {
                                            self.report.scalar_box_get_count += 1;
                                        }
                                        if i < 0 || (i as usize) >= r.len() {
                                            self.report.out_of_bounds_count += 1;
                                            self.stack.push(BiteCodeEvaluated::Int(0));
                                            continue;
                                        }
                                        self.stack.push(r[i as usize].clone());
                                    }
                                    _ => {
                                        return Err("Error! get expects (vector,int)".to_string());
                                    }
                                }
                            }

                            Instruction::RestArray => {
                                let index_val = self.stack.pop().ok_or("stack underflow (index)")?;
                                let array_val: BiteCodeEvaluated = self.stack
                                    .pop()
                                    .ok_or("stack underflow (vector)")?;
                                match (array_val, index_val) {
                                    (BiteCodeEvaluated::Array(arr), BiteCodeEvaluated::Int(i)) => {
                                        let r = arr.borrow();
                                        if i < 0 || (i as usize) > r.len() {
                                            self.report.rest_array_empty_result_count += 1;
                                            self.stack.push(
                                                BiteCodeEvaluated::Array(
                                                    Rc::new(RefCell::new(Vec::new()))
                                                )
                                            );
                                        } else {
                                            self.observe_array_len(r.len() - (i as usize));
                                            self.stack.push(
                                                BiteCodeEvaluated::Array(
                                                    Rc::new(RefCell::new(r[i as usize..].to_vec()))
                                                )
                                            );
                                        }
                                    }
                                    _ => {
                                        return Err("Error! cdr expects (vector,int)".to_string());
                                    }
                                }
                            }

                            Instruction::Length => {
                                let arr = self.stack
                                    .pop()
                                    .ok_or("stack underflow: length needs an vector")?;
                                match arr {
                                    BiteCodeEvaluated::Array(elements) => {
                                        self.stack.push(
                                            BiteCodeEvaluated::Int(elements.borrow().len() as i32)
                                        );
                                    }
                                    _ => {
                                        return Err("Error! length expects an vector".to_string());
                                    }
                                }
                            }
                            Instruction::PopArray => {
                                let array_val = self.stack.pop().ok_or("stack underflow")?;
                                match array_val {
                                    BiteCodeEvaluated::Array(arr) => {
                                        arr.borrow_mut().pop();
                                        self.stack.push(BiteCodeEvaluated::Int(0));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! pop! argument not an vector".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::SetArray => {
                                // Stack: [...,vector(Rc<RefCell<Vec<BiteCodeEvaluated>>>),index(Int),value(BiteCodeEvaluated)]
                                let value = self.stack.pop().ok_or("stack underflow")?;
                                let index_val = self.stack.pop().ok_or("stack underflow")?;
                                let array_val = self.stack.pop().ok_or("stack underflow")?;

                                if
                                    let (
                                        BiteCodeEvaluated::Array(arr),
                                        BiteCodeEvaluated::Int(idx),
                                    ) = (array_val, index_val)
                                {
                                    let len = arr.borrow().len();
                                    if idx >= 0 && (idx as usize) <= len {
                                        self.report.min_set_index = min_opt_i32(
                                            self.report.min_set_index,
                                            Some(idx)
                                        );
                                        self.report.max_set_index = max_opt_i32(
                                            self.report.max_set_index,
                                            Some(idx)
                                        );
                                        if idx == (len as i32) {
                                            self.report.set_array_append_count += 1;
                                            arr.borrow_mut().push(value);
                                        } else {
                                            self.report.set_array_overwrite_count += 1;
                                            if len == 1 && idx == 0 {
                                                self.report.scalar_box_set_count += 1;
                                            }
                                            arr.borrow_mut()[idx as usize] = value;
                                        }
                                    } else {
                                        self.report.out_of_bounds_count += 1;
                                        self.stack.push(BiteCodeEvaluated::Int(0));
                                        continue;
                                    }
                                    self.observe_array_len(arr.borrow().len());
                                    self.stack.push(BiteCodeEvaluated::Int(0));
                                } else {
                                    return Err(
                                        "Error! set! expects vector and integer index".to_string()
                                    );
                                }
                            }
                            Instruction::If(branch_id, then_branch, else_branch) => {
                                let cond = self.stack.pop().ok_or("stack underflow")?;
                                let cond_val = match cond {
                                    BiteCodeEvaluated::Bool(n) => n,
                                    _ => {
                                        return Err(
                                            "Error! if condition must be true or false".to_string()
                                        );
                                    }
                                };
                                if cond_val {
                                    self.report.branch_true_count += 1;
                                    self.report.branch_site_outcomes
                                        .entry(*branch_id)
                                        .or_default().true_hits += 1;
                                    self.run(&then_branch)?;
                                } else {
                                    self.report.branch_false_count += 1;
                                    self.report.branch_site_outcomes
                                        .entry(*branch_id)
                                        .or_default().false_hits += 1;
                                    self.run(&else_branch)?;
                                }
                            }
                            Instruction::PushInt(n) => self.stack.push(BiteCodeEvaluated::Int(*n)),
                            Instruction::PushFloat(n) =>
                                self.stack.push(BiteCodeEvaluated::Float(*n)),

                            Instruction::PushBool(n) =>
                                self.stack.push(BiteCodeEvaluated::Bool(*n)),

                            Instruction::Add => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        if a.checked_add(b).is_none() {
                                            self.report.int_overflow_risk_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Int(a.wrapping_add(b)));
                                    }
                                    _ => {
                                        return Err(
                                            "Error!  Both arguments must be ints at (+)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::AddF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        let out = a + b;
                                        if !out.is_finite() {
                                            self.report.float_non_finite_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Float(out));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (+.)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::Mult => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        if a.checked_mul(b).is_none() {
                                            self.report.int_overflow_risk_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Int(a.wrapping_mul(b)));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (*)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::MultF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        let out = a * b;
                                        if !out.is_finite() {
                                            self.report.float_non_finite_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Float(out));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (*.)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::Div => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        if b == 0 {
                                            self.report.int_div_by_zero_count += 1;
                                            self.stack.push(BiteCodeEvaluated::Int(0));
                                            continue;
                                        }
                                        if a.checked_div(b).is_none() {
                                            self.report.int_overflow_risk_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Int(a.wrapping_div(b)));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (/)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::DivF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        let out = a / b;
                                        if !out.is_finite() {
                                            self.report.float_non_finite_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Float(out));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (/.)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::Sub => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        if a.checked_sub(b).is_none() {
                                            self.report.int_overflow_risk_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Int(a.wrapping_sub(b)));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (-)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::SubF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        let out = a - b;
                                        if !out.is_finite() {
                                            self.report.float_non_finite_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Float(out));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (-.)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::Mod => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        if b == 0 {
                                            self.report.int_div_by_zero_count += 1;
                                            self.stack.push(BiteCodeEvaluated::Int(0));
                                            continue;
                                        }
                                        if a.checked_rem(b).is_none() {
                                            self.report.int_overflow_risk_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Int(a.wrapping_rem(b)));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (mod)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::ModF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        let out = a % b;
                                        if !out.is_finite() {
                                            self.report.float_non_finite_count += 1;
                                        }
                                        self.stack.push(BiteCodeEvaluated::Float(out));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (mod.)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::BitXor => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Int(a ^ b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (^)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::BitRs => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Int(a >> b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (>>)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::BitLs => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Int(a << b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (<<)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::BitAnd => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Int(a & b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (&)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::BitOr => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Int(a | b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (|)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::BitNot => {
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match a {
                                    BiteCodeEvaluated::Int(a) => {
                                        self.stack.push(BiteCodeEvaluated::Int(!a));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Arguments must be a number at (~)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::IntToFloat => {
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match a {
                                    BiteCodeEvaluated::Int(a) => {
                                        self.stack.push(BiteCodeEvaluated::Float(a as f32));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Arguments must be a number at (Int->Float)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::FloatToInt => {
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match a {
                                    BiteCodeEvaluated::Float(a) => {
                                        self.stack.push(BiteCodeEvaluated::Int(a as i32));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Arguments must be a number at (Float->Int)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::Eq => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;

                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a == b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (=)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::EqBool => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;

                                match (a, b) {
                                    (BiteCodeEvaluated::Bool(a), BiteCodeEvaluated::Bool(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a == b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be bools at (=?)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::EqF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;

                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a == b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (=.)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::GtF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a > b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (>.)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::LtF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a < b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (<.)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::GteF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a >= b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (>=.)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::LteF => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Float(a), BiteCodeEvaluated::Float(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a <= b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be floats at (<=.)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::Lt => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a < b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (<)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::Gt => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a > b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (>)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::Lte => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a <= b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (<=)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::Gte => {
                                let b = self.stack.pop().ok_or("stack underflow")?;
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match (a, b) {
                                    (BiteCodeEvaluated::Int(a), BiteCodeEvaluated::Int(b)) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(a >= b));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Both arguments must be ints at (>=)".to_string()
                                        );
                                    }
                                }
                            }
                            Instruction::Not => {
                                let a = self.stack.pop().ok_or("stack underflow")?;
                                match a {
                                    BiteCodeEvaluated::Bool(a) => {
                                        self.stack.push(BiteCodeEvaluated::Bool(!a));
                                    }
                                    _ => {
                                        return Err(
                                            "Error! Argument must be a number at (not)".to_string()
                                        );
                                    }
                                }
                            }

                            Instruction::Pop => {
                                self.stack.pop().ok_or("stack underflow")?;
                            }

                            Instruction::StoreVar(name) => {
                                let val = self.stack.pop().ok_or("stack underflow")?;
                                let mut locals = self.locals.borrow_mut();
                                locals.vars.insert(name.clone(), val);
                            }

                            Instruction::LoadVar(name) => {
                                let val = self.locals
                                    .borrow()
                                    .get(name)
                                    .ok_or(format!("undefined variable: {}", name))?;
                                self.stack.push(val);
                            }

                            Instruction::MakeVector(n) => {
                                let mut elements = Vec::with_capacity(*n);
                                for _ in 0..*n {
                                    elements.push(self.stack.pop().ok_or("stack underflow")?);
                                }
                                elements.reverse(); // preserve order
                                self.report.array_alloc_count += 1;
                                self.observe_array_len(elements.len());
                                self.stack.push(
                                    BiteCodeEvaluated::Array(Rc::new(RefCell::new(elements)))
                                );
                            }

                            Instruction::MakeLambda(params, body) => {
                                let closure = BiteCodeEvaluated::Function(
                                    usize::MAX,
                                    params.clone(),
                                    body.clone(),
                                    Rc::new(
                                        RefCell::new(
                                            BiteCodeEnv::with_parent(Rc::clone(&self.locals))
                                        )
                                    )
                                );
                                self.stack.push(closure);
                            }

                            Instruction::MakeLambdaUid(uid, params, body) => {
                                // this solution will capture inner closure scope but at the cost of a memory leak!
                                // let closure = BiteCodeEvaluated::Function(
                                //     params.clone(),
                                //     body.clone(),
                                //     Rc::clone(&self.locals), // shared mutable environment
                                // );
                                // self.stack.push(closure);
                                let closure = BiteCodeEvaluated::Function(
                                    *uid,
                                    params.clone(),
                                    body.clone(),
                                    Rc::new(
                                        RefCell::new(
                                            BiteCodeEnv::with_parent(Rc::clone(&self.locals))
                                        )
                                    )
                                );
                                self.stack.push(closure);
                            }

                            Instruction::Call(arg_count) => {
                                self.report.call_count += 1;
                                let func = self.stack
                                    .pop()
                                    .ok_or("Error! Runtime stack underflow")?;
                                let mut args: Vec<BiteCodeEvaluated> = (0..*arg_count)
                                    .map(|_|
                                        self.stack.pop().ok_or("Error! Runtime stack underflow")
                                    )
                                    .collect::<Result<Vec<_>, _>>()?
                                    .into_iter()
                                    .rev()
                                    .collect();

                                let mut current_func = func;
                                loop {
                                    match current_func {
                                        BiteCodeEvaluated::Function(uid, params, body, env) => {
                                            let current_function_id = function_id(uid);
                                            let consumed = args.len().min(params.len());
                                            let (used_args, remaining_args) =
                                                args.split_at(consumed);
                                            let mut input_array_total_len = 0usize;
                                            let mut input_array_count = 0usize;
                                            for arg in used_args {
                                                if let BiteCodeEvaluated::Array(arr) = arg {
                                                    input_array_total_len += arr.borrow().len();
                                                    input_array_count += 1;
                                                }
                                            }

                                            let local_env = Rc::new(
                                                RefCell::new(BiteCodeEnv::with_parent(env.clone()))
                                            );
                                            {
                                                let mut local_env_ref = local_env.borrow_mut();
                                                for (p, v) in params
                                                    .iter()
                                                    .take(consumed)
                                                    .zip(used_args.iter()) {
                                                    local_env_ref.set(p.clone(), v.clone());
                                                }
                                            }

                                            if consumed == params.len() {
                                                // Run body if we've satisfied this function's arguments
                                                // Save current environment and stack
                                                let old_env = self.locals.clone();
                                                let old_stack_len = self.stack.len();
                                                let report_before_call = self.report.clone();

                                                // Switch to function environment
                                                self.locals = local_env;

                                                // Run function body
                                                self.run(&body)?;

                                                let mut function_report =
                                                    self.report.difference_from(
                                                        &report_before_call
                                                    );
                                                if let Some(value) = self.stack.last() {
                                                    function_report.final_result_type_tag = Some(
                                                        value_type_tag(value).to_string()
                                                    );
                                                    function_report.final_result_repr = Some(
                                                        truncate_middle(
                                                            &format_value_preview(value),
                                                            MAX_RESULT_REPR_LEN,
                                                            RESULT_REPR_EDGE_LEN
                                                        )
                                                    );
                                                    if let BiteCodeEvaluated::Array(arr) = value {
                                                        function_report.output_array_total_len = arr
                                                            .borrow()
                                                            .len();
                                                        function_report.output_array_count = 1;
                                                    }
                                                }
                                                function_report.input_array_total_len =
                                                    input_array_total_len;
                                                function_report.input_array_count =
                                                    input_array_count;
                                                function_report.input_preview = Some(
                                                    format_input_preview(used_args)
                                                );
                                                // Keep per-call report flat to avoid recursive report trees.
                                                function_report.function_reports.clear();
                                                function_report.function_reports_truncated = false;
                                                self.report.function_reports.push(
                                                    FunctionRuntimeReport {
                                                        function_id: current_function_id.clone(),
                                                        arity: params.len(),
                                                        call_depth: self.call_depth + 1,
                                                        report: function_report,
                                                    }
                                                );
                                                compact_function_reports_if_needed(
                                                    &mut self.report
                                                );

                                                // Get result
                                                let result = self.stack
                                                    .pop()
                                                    .unwrap_or(BiteCodeEvaluated::Int(0));

                                                // Restore environment and stack
                                                self.locals = old_env;
                                                // Remove any extra stack elements that might have been added
                                                self.stack.truncate(old_stack_len);

                                                if remaining_args.is_empty() {
                                                    // No more args to apply -> we're done
                                                    self.stack.push(result);
                                                    break;
                                                } else {
                                                    // More args left,result must be a function -> continue applying
                                                    current_func = result;
                                                    args = remaining_args.to_vec();
                                                    continue;
                                                }
                                            } else {
                                                // Partial application -> return closure waiting for the rest
                                                self.report.partial_application_count += 1;
                                                let remaining_params = params[consumed..].to_vec();
                                                self.stack.push(
                                                    BiteCodeEvaluated::Function(
                                                        uid,
                                                        remaining_params,
                                                        body,
                                                        local_env
                                                    )
                                                );
                                                break;
                                            }
                                        }
                                        _ => {
                                            return Err(
                                                "Error! Cannot call non-function".to_string()
                                            );
                                        }
                                    }
                                }
                            }

                            Instruction::Loop(start, end, func) => {
                                // Evaluate start
                                let mut start_vm = VM {
                                    stack: Vec::new(),
                                    locals: self.locals.clone(),
                                    call_depth: 0,
                                    report: RuntimeReport::default(),
                                };
                                start_vm.run(start)?;
                                merge_runtime_report_into(&mut self.report, &start_vm.report);
                                let start_val = start_vm.stack
                                    .pop()
                                    .ok_or("Error! Runtime loop: missing start value")?;

                                // Evaluate end
                                let mut end_vm = VM {
                                    stack: Vec::new(),
                                    locals: self.locals.clone(),
                                    call_depth: 0,
                                    report: RuntimeReport::default(),
                                };
                                end_vm.run(end)?;
                                merge_runtime_report_into(&mut self.report, &end_vm.report);
                                let end_val = end_vm.stack
                                    .pop()
                                    .ok_or("Error! Runtime loop: missing end value")?;

                                let start_int = match start_val {
                                    BiteCodeEvaluated::Int(n) => n,
                                    _ => {
                                        return Err("Error! loop start must be int".to_string());
                                    }
                                };
                                let end_int = match end_val {
                                    BiteCodeEvaluated::Int(n) => n,
                                    _ => {
                                        return Err("Error! loop end must be int".to_string());
                                    }
                                };
                                // Pre-resolve the function ONCE
                                let mut func_vm = VM {
                                    stack: Vec::new(),
                                    locals: self.locals.clone(),
                                    call_depth: 0,
                                    report: RuntimeReport::default(),
                                };
                                func_vm.run(func)?;
                                merge_runtime_report_into(&mut self.report, &func_vm.report);
                                let func_val = func_vm.stack
                                    .pop()
                                    .ok_or("Error! Runtime loop: missing function")?;
                                let (params, body, captured_env) = match func_val {
                                    BiteCodeEvaluated::Function(_, p, b, e) => (p, b, e),
                                    _ => {
                                        return Err(
                                            "Error! loop: third argument must be a lambda".to_string()
                                        );
                                    }
                                };

                                if params.len() != 1 {
                                    return Err(
                                        "Error! loop: lambda must take exactly one parameter".to_string()
                                    );
                                }

                                let mut inner_vm = VM {
                                    stack: Vec::new(),
                                    locals: captured_env.clone(),
                                    call_depth: 0,
                                    report: RuntimeReport::default(),
                                };

                                let mut loop_count = 0usize;
                                for i in start_int..end_int {
                                    loop_count += 1;
                                    inner_vm.stack.clear(); // reuse stack
                                    inner_vm.locals
                                        .borrow_mut()
                                        .set(params[0].clone(), BiteCodeEvaluated::Int(i));
                                    inner_vm.run(&body)?;
                                }
                                merge_runtime_report_into(&mut self.report, &inner_vm.report);
                                self.report.loop_iterations += loop_count;
                                self.report.max_loop_iterations =
                                    self.report.max_loop_iterations.max(loop_count);

                                self.stack.push(BiteCodeEvaluated::Int(0));
                            }

                            Instruction::LoopFinish(cond, func) => {
                                // Pre-resolve function once (like in Loop)
                                let mut func_vm = VM {
                                    stack: Vec::new(),
                                    locals: self.locals.clone(),
                                    call_depth: 0,
                                    report: RuntimeReport::default(),
                                };
                                func_vm.run(func)?;
                                merge_runtime_report_into(&mut self.report, &func_vm.report);
                                let func_val = func_vm.stack
                                    .pop()
                                    .ok_or("Error! Runtime loop-finish: missing function")?;
                                let (params, body, captured_env) = match func_val {
                                    BiteCodeEvaluated::Function(_, p, b, e) => (p, b, e),
                                    _ => {
                                        return Err(
                                            "Error! loop-finish: second argument must be a lambda".to_string()
                                        );
                                    }
                                };

                                if !params.is_empty() {
                                    return Err(
                                        "Error! loop-finish: lambda must take 0 params".to_string()
                                    );
                                }

                                // Reuse the same VMs
                                let mut cond_vm = VM {
                                    stack: Vec::new(),
                                    locals: self.locals.clone(),
                                    call_depth: 0,
                                    report: RuntimeReport::default(),
                                };

                                let mut inner_vm = VM {
                                    stack: Vec::new(),
                                    locals: captured_env.clone(),
                                    call_depth: 0,
                                    report: RuntimeReport::default(),
                                };

                                let mut loop_finish_count = 0usize;
                                loop {
                                    cond_vm.stack.clear();
                                    cond_vm.run(cond)?;

                                    let cond_val = cond_vm.stack
                                        .pop()
                                        .ok_or("Error! Runtime loop-finish: missing condition")?;
                                    let cond_bool = match cond_val {
                                        BiteCodeEvaluated::Bool(n) => n,
                                        _ => {
                                            return Err(
                                                "Error! loop-finish condition must be bool".to_string()
                                            );
                                        }
                                    };

                                    if !cond_bool {
                                        break;
                                    }

                                    loop_finish_count += 1;
                                    inner_vm.stack.clear();
                                    inner_vm.run(&body)?;
                                }
                                merge_runtime_report_into(&mut self.report, &cond_vm.report);
                                merge_runtime_report_into(&mut self.report, &inner_vm.report);
                                self.report.loop_finish_iterations += loop_finish_count;
                                self.report.max_loop_finish_iterations =
                                    self.report.max_loop_finish_iterations.max(loop_finish_count);
                                self.stack.push(BiteCodeEvaluated::Int(0)); // by convention returning Int even though the type is Unit (nil is still 0)
                            }
                        }
                        self.bump_stack_peak();
                    }
                    Ok(())
                }
            )
        );

        self.call_depth -= 1;
        match run_result {
            Ok(Ok(())) => {
                self.report.final_stack_size = self.stack.len();
                Ok(())
            }
            Ok(Err(err)) => Err(err),
            Err(_) =>
                Err(
                    format!(
                        "Error! VM panic while executing instruction {} at pc {}",
                        current_instr,
                        current_pc
                    )
                ),
        }
    }

    pub fn result(&self) -> Option<&BiteCodeEvaluated> {
        self.stack.last()
    }
}

pub fn compile(expr: &Expression, code: &mut Vec<Instruction>) -> Result<(), String> {
    match expr {
        Expression::Int(n) => {
            code.push(Instruction::PushInt(*n));
            Ok(())
        }

        Expression::Float(n) => {
            code.push(Instruction::PushFloat(*n));
            Ok(())
        }

        Expression::Word(name) => {
            match name.as_str() {
                // TODO add get fst snd and other missing stuff from here
                "true" => {
                    code.push(Instruction::PushBool(true));
                    Ok(())
                }
                "false" => {
                    code.push(Instruction::PushBool(false));
                    Ok(())
                }
                // push a closure representing these
                "/" | "/#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Div
                            ]
                        )
                    );
                    Ok(())
                }
                "/." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::DivF
                            ]
                        )
                    );
                    Ok(())
                }
                "*" | "*#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Mult
                            ]
                        )
                    );
                    Ok(())
                }

                "*." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::MultF
                            ]
                        )
                    );
                    Ok(())
                }
                "mod" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Mod
                            ]
                        )
                    );
                    Ok(())
                }
                "mod." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::ModF
                            ]
                        )
                    );
                    Ok(())
                }
                "+" | "+#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Add
                            ]
                        )
                    );
                    Ok(())
                }

                "+." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::AddF
                            ]
                        )
                    );
                    Ok(())
                }

                "-" | "-#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Sub
                            ]
                        )
                    );
                    Ok(())
                }
                "-." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::SubF
                            ]
                        )
                    );
                    Ok(())
                }
                ">" | ">#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Gt
                            ]
                        )
                    );
                    Ok(())
                }
                "<" | "<#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Lt
                            ]
                        )
                    );
                    Ok(())
                }
                ">=" | ">=#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Gte
                            ]
                        )
                    );
                    Ok(())
                }
                "<=" | "<=#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Lte
                            ]
                        )
                    );
                    Ok(())
                }
                "=?" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::EqBool
                            ]
                        )
                    );
                    Ok(())
                }
                "=." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::EqF
                            ]
                        )
                    );
                    Ok(())
                }
                ">." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::GtF
                            ]
                        )
                    );
                    Ok(())
                }
                "<." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::LtF
                            ]
                        )
                    );
                    Ok(())
                }
                ">=." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::GteF
                            ]
                        )
                    );
                    Ok(())
                }
                "<=." => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::LteF
                            ]
                        )
                    );
                    Ok(())
                }
                "=" | "=#" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::Eq
                            ]
                        )
                    );
                    Ok(())
                }
                "length" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["xs".to_string()],
                            vec![Instruction::LoadVar("xs".to_string()), Instruction::Length]
                        )
                    );
                    Ok(())
                }
                "not" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string()],
                            vec![Instruction::LoadVar("a".to_string()), Instruction::Not]
                        )
                    );
                    Ok(())
                }

                ">>" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::BitRs
                            ]
                        )
                    );
                    Ok(())
                }
                "<<" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::BitLs
                            ]
                        )
                    );
                    Ok(())
                }
                "^" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::BitXor
                            ]
                        )
                    );
                    Ok(())
                }
                "|" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::BitOr
                            ]
                        )
                    );
                    Ok(())
                }
                "&" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string(), "b".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::LoadVar("b".to_string()),
                                Instruction::BitAnd
                            ]
                        )
                    );
                    Ok(())
                }
                "~" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string()],
                            vec![Instruction::LoadVar("a".to_string()), Instruction::BitNot]
                        )
                    );
                    Ok(())
                }
                "Int->Float" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string()],
                            vec![Instruction::LoadVar("a".to_string()), Instruction::IntToFloat]
                        )
                    );
                    Ok(())
                }
                "Float->Int" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string()],
                            vec![Instruction::LoadVar("a".to_string()), Instruction::FloatToInt]
                        )
                    );
                    Ok(())
                }
                "car" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::PushInt(0),
                                Instruction::GetArray
                            ]
                        )
                    );
                    Ok(())
                }
                "cdr" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::PushInt(1),
                                Instruction::RestArray
                            ]
                        )
                    );
                    Ok(())
                }
                "fst" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::PushInt(0),
                                Instruction::GetArray
                            ]
                        )
                    );
                    Ok(())
                }
                "snd" => {
                    code.push(
                        Instruction::MakeLambda(
                            vec!["a".to_string()],
                            vec![
                                Instruction::LoadVar("a".to_string()),
                                Instruction::PushInt(1),
                                Instruction::GetArray
                            ]
                        )
                    );
                    Ok(())
                }
                _ => {
                    code.push(Instruction::LoadVar(name.clone()));
                    Ok(())
                }
            }
        }

        Expression::Apply(exprs) => {
            if let Expression::Word(op) = &exprs[0] {
                match op.as_str() {
                    "+" | "+#" => {
                        if exprs.len() != 3 {
                            return Err("Error! + expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Add);
                        Ok(())
                    }
                    "+." => {
                        if exprs.len() != 3 {
                            return Err("Error! +. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::AddF);
                        Ok(())
                    }
                    "*" | "*#" => {
                        if exprs.len() != 3 {
                            return Err("Error! * expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Mult);
                        Ok(())
                    }
                    "*." => {
                        if exprs.len() != 3 {
                            return Err("Error! *. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::MultF);
                        Ok(())
                    }
                    "/" | "/#" => {
                        if exprs.len() != 3 {
                            return Err("Error! / expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Div);
                        Ok(())
                    }
                    "/." => {
                        if exprs.len() != 3 {
                            return Err("Error! /. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::DivF);
                        Ok(())
                    }
                    "-" | "-#" => {
                        if exprs.len() != 3 {
                            return Err("Error! - expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Sub);
                        Ok(())
                    }
                    "-." => {
                        if exprs.len() != 3 {
                            return Err("Error! -. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::SubF);
                        Ok(())
                    }
                    "mod" => {
                        if exprs.len() != 3 {
                            return Err("Error! mod expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Mod);
                        Ok(())
                    }
                    "mod." => {
                        if exprs.len() != 3 {
                            return Err("Error! mod. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::ModF);
                        Ok(())
                    }
                    "=?" => {
                        if exprs.len() != 3 {
                            return Err("Error! = expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::EqBool);
                        Ok(())
                    }
                    "=." => {
                        if exprs.len() != 3 {
                            return Err("Error! =. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::EqF);
                        Ok(())
                    }
                    ">." => {
                        if exprs.len() != 3 {
                            return Err("Error! >. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::GtF);
                        Ok(())
                    }
                    "<." => {
                        if exprs.len() != 3 {
                            return Err("Error! <. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::LtF);
                        Ok(())
                    }
                    ">=." => {
                        if exprs.len() != 3 {
                            return Err("Error! >=. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::GteF);
                        Ok(())
                    }
                    "<=." => {
                        if exprs.len() != 3 {
                            return Err("Error! <=. expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::LteF);
                        Ok(())
                    }
                    "=" | "=#" => {
                        if exprs.len() != 3 {
                            return Err("Error! = expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Eq);
                        Ok(())
                    }
                    "<" | "<#" => {
                        if exprs.len() != 3 {
                            return Err("Error! < expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Lt);
                        Ok(())
                    }
                    ">" | ">#" => {
                        if exprs.len() != 3 {
                            return Err("Error! > expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Gt);
                        Ok(())
                    }
                    "<=" | "<=#" => {
                        if exprs.len() != 3 {
                            return Err("Error! <= expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Lte);
                        Ok(())
                    }
                    ">=" | ">=#" => {
                        if exprs.len() != 3 {
                            return Err("Error! >= expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::Gte);
                        Ok(())
                    }
                    "and" => {
                        if exprs.len() != 3 {
                            return Err("Error! and expects exactly 2 arguments".to_string());
                        }

                        let mut then_code = Vec::new();
                        compile(&exprs[2], &mut then_code)?;
                        // First argument is the condition
                        compile(&exprs[1], code)?;
                        code.push(
                            Instruction::If(0, then_code, vec![Instruction::PushBool(false)])
                        );
                        Ok(())
                    }
                    "or" => {
                        if exprs.len() != 3 {
                            return Err("Error! or expects exactly 2 arguments".to_string());
                        }

                        // Evaluate second argument (b)
                        let mut else_code = Vec::new();
                        compile(&exprs[2], &mut else_code)?;

                        // Evaluate first argument (a)
                        compile(&exprs[1], code)?;
                        // If a is true, return true immediately; otherwise, evaluate b
                        code.push(
                            Instruction::If(
                                0,
                                vec![Instruction::PushBool(true)], // then branch
                                else_code // else branch
                            )
                        );

                        Ok(())
                    }
                    "not" => {
                        if exprs.len() != 2 {
                            return Err("Error! not expects exactly 1 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        code.push(Instruction::Not);
                        Ok(())
                    }

                    ">>" => {
                        if exprs.len() != 3 {
                            return Err("Error! >> expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitRs);
                        Ok(())
                    }
                    "<<" => {
                        if exprs.len() != 3 {
                            return Err("Error! << expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitLs);
                        Ok(())
                    }
                    "^" => {
                        if exprs.len() != 3 {
                            return Err("Error! ^ expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitXor);
                        Ok(())
                    }
                    "&" => {
                        if exprs.len() != 3 {
                            return Err("Error! & expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitAnd);
                        Ok(())
                    }
                    "|" => {
                        if exprs.len() != 3 {
                            return Err("Error! | expects exactly 2 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::BitOr);
                        Ok(())
                    }
                    "~" => {
                        if exprs.len() != 2 {
                            return Err("Error! ~ expects exactly 1 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        code.push(Instruction::BitNot);
                        Ok(())
                    }
                    "Int->Float" => {
                        if exprs.len() != 2 {
                            return Err("Error! ~ expects exactly 1 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        code.push(Instruction::IntToFloat);
                        Ok(())
                    }
                    "Float->Int" => {
                        if exprs.len() != 2 {
                            return Err("Error! ~ expects exactly 1 arguments".to_string());
                        }
                        compile(&exprs[1], code)?;
                        code.push(Instruction::FloatToInt);
                        Ok(())
                    }
                    "do" => {
                        if exprs.len() <= 0 {
                            return Err("Error! do expects atleast 1 argument".to_string());
                        }
                        for (i, e) in exprs[1..].iter().enumerate() {
                            compile(e, code)?;
                            if i < exprs.len() - 2 {
                                code.push(Instruction::Pop);
                            }
                        }
                        Ok(())
                    }
                    "length" => {
                        if exprs.len() != 2 {
                            return Err("Error! length expects exactly 1 argument".to_string());
                        }
                        compile(&exprs[1], code)?; // compile vector expression
                        code.push(Instruction::Length);
                        Ok(())
                    }
                    "let" | "let*" => {
                        if exprs.len() != 3 {
                            return Err(
                                "let requires exactly 2 arguments: name and value".to_string()
                            );
                        }
                        let var_name = match &exprs[1] {
                            Expression::Word(name) => name.clone(),
                            _ => {
                                return Err("Error! let variable must be a word".to_string());
                            }
                        };
                        compile(&exprs[2], code)?; // evaluate value
                        code.push(Instruction::StoreVar(var_name));
                        // push sentinel Unit (here just Int 0) so do sees something
                        code.push(Instruction::PushInt(0));
                        Ok(())
                    }
                    "lambda" => {
                        // args: (lambda param1 param2 ... body)
                        if exprs.len() < 2 {
                            return Err(
                                "Error! lambda requires at least 1 param and a body".to_string()
                            );
                        }

                        let mut params = Vec::new();
                        for e in &exprs[1..exprs.len() - 1] {
                            match e {
                                Expression::Word(name) => params.push(name.clone()),
                                _ => {
                                    return Err("Error! lambda params must be words".to_string());
                                }
                            }
                        }

                        let body_expr = &exprs[exprs.len() - 1];
                        let mut body_code = Vec::new();
                        compile(body_expr, &mut body_code)?;

                        code.push(Instruction::MakeLambda(params, body_code));
                        Ok(())
                    }
                    "as" => {
                        // Just ensure syntax correctness — (as expr type)
                        if exprs.len() != 3 {
                            return Err(
                                "Error! as expects two arguments: (as expr Type)".to_string()
                            );
                        }

                        // Compile the first argument normally
                        compile(&exprs[1], code)?;

                        // We intentionally do NOT emit anything for the type annotation.
                        // The type info is only used by the type checker / inference layer.

                        Ok(())
                    }
                    "char" => {
                        for arg in &exprs[1..] {
                            compile(arg, code)?;
                        }
                        Ok(())
                    }
                    "vector" | "string" | "tuple" => {
                        let count = exprs.len() - 1;
                        for arg in &exprs[1..] {
                            compile(arg, code)?;
                        }
                        code.push(Instruction::MakeVector(count));
                        Ok(())
                    }
                    "if" => {
                        if exprs.len() != 4 {
                            return Err("Error! if requires exactly 3 arguments".to_string());
                        }
                        compile(&exprs[1], code)?; // compile condition

                        let mut then_code = Vec::new();
                        compile(&exprs[2], &mut then_code)?;

                        let mut else_code = Vec::new();
                        compile(&exprs[3], &mut else_code)?;

                        code.push(Instruction::If(0, then_code, else_code));
                        Ok(())
                    }
                    "loop-finish" => {
                        if exprs.len() != 3 {
                            return Err(
                                "loop-finish expects 2 arguments: condition,lambda".to_string()
                            );
                        }

                        let mut cond_code = Vec::new();
                        compile(&exprs[1], &mut cond_code)?;

                        let mut func_code = Vec::new();
                        compile(&exprs[2], &mut func_code)?;

                        code.push(Instruction::LoopFinish(cond_code, func_code));
                        Ok(())
                    }
                    "loop" => {
                        if exprs.len() != 4 {
                            return Err(
                                "Error! loop expects 3 arguments: start,end,lambda".to_string()
                            );
                        }

                        let mut start_code = Vec::new();
                        compile(&exprs[1], &mut start_code)?;

                        let mut end_code = Vec::new();
                        compile(&exprs[2], &mut end_code)?;

                        let mut func_code = Vec::new();
                        compile(&exprs[3], &mut func_code)?;

                        code.push(Instruction::Loop(start_code, end_code, func_code));
                        Ok(())
                    }
                    "set!" => {
                        if exprs.len() != 4 {
                            return Err(
                                "Error! set! expects 3 arguments: vector,index,value".to_string()
                            );
                        }
                        compile(&exprs[1], code)?; // vector
                        compile(&exprs[2], code)?; // index
                        compile(&exprs[3], code)?; // value
                        code.push(Instruction::SetArray);
                        Ok(())
                    }
                    "pop!" => {
                        if exprs.len() != 2 {
                            return Err("Error! pop! expects 1 argument: vector".to_string());
                        }
                        compile(&exprs[1], code)?; // vector
                        code.push(Instruction::PopArray);
                        Ok(())
                    }

                    "get" => {
                        if exprs.len() != 3 {
                            return Err("Error! get expects 2 arguments: vector,index".to_string());
                        }
                        // push vector
                        compile(&exprs[1], code)?;
                        // push index
                        compile(&exprs[2], code)?;
                        // emit get
                        code.push(Instruction::GetArray);
                        Ok(())
                    }
                    "cdr" => {
                        // push vector
                        compile(&exprs[1], code)?;
                        compile(&exprs[2], code)?;
                        code.push(Instruction::RestArray);
                        Ok(())
                    }
                    "fst" | "car" => {
                        if exprs.len() != 2 {
                            return Err("Error! fst expects 1 arguments: tuple".to_string());
                        }
                        // push vector
                        compile(&exprs[1], code)?;
                        // push index
                        code.push(Instruction::PushInt(0));
                        // emit get
                        code.push(Instruction::GetArray);
                        Ok(())
                    }
                    "snd" => {
                        if exprs.len() != 2 {
                            return Err("Error! snd expects 1 arguments: tuple".to_string());
                        }
                        // push vector
                        compile(&exprs[1], code)?;
                        // push index
                        code.push(Instruction::PushInt(1));
                        // emit get
                        code.push(Instruction::GetArray);
                        Ok(())
                    }
                    _ =>
                        match &exprs[0] {
                            Expression::Word(name) => {
                                // push all arguments first
                                for arg in &exprs[1..] {
                                    compile(arg, code)?;
                                }
                                // load the function/variable and call
                                code.push(Instruction::LoadVar(name.clone()));
                                code.push(Instruction::Call(exprs.len() - 1));
                                Ok(())
                            }
                            _ => {
                                return Err("Error! Cannot call non-word expression".to_string());
                            }
                        }
                }
            } else {
                Ok(())
            }
        }
    }
}

#[allow(dead_code)]
pub fn run_with_report(
    expr: &crate::parser::Expression,
    mut vm: VM
) -> (Result<BiteCodeEvaluated, String>, RuntimeReport) {
    let mut code = Vec::new();
    if let Err(err) = compile(&expr, &mut code) {
        return (Err(err), vm.report);
    }
    let mut next_lambda_uid = 0usize;
    code = assign_lambda_uids(code, &mut next_lambda_uid);
    let mut next_branch_id = 0usize;
    code = assign_branch_uids(code, &mut next_branch_id);

    let result = match vm.run(&code) {
        Ok(_) => Ok(vm.result().unwrap_or(&BiteCodeEvaluated::Int(0)).clone()),
        Err(err) => Err(err),
    };

    if let Ok(value) = &result {
        vm.report.final_result_type_tag = Some(value_type_tag(value).to_string());
        vm.report.final_result_repr = Some(
            truncate_middle(&format_value_preview(value), MAX_RESULT_REPR_LEN, RESULT_REPR_EDGE_LEN)
        );
    }

    (result, vm.report)
}
