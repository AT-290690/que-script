use super::*;
use std::collections::HashSet;

fn diagnostics(source: &str, user_form_count: usize) -> Vec<String> {
    let expression = crate::parser::build(source)
        .unwrap_or_else(|error| panic!("generated source did not parse:\n{source}\n{error}"));
    let (_typ, typed) = crate::infer::infer_with_builtins_typed(
        &expression,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
    )
    .unwrap_or_else(|error| panic!("generated source did not infer:\n{source}\n{error}"));
    analyze_user_program_diagnostics(&typed, user_form_count)
}

fn has(diagnostics: &[String], needle: &str) -> bool {
    diagnostics.iter().any(|message| message.contains(needle))
}

fn vector_literal(length: usize) -> String {
    format!(
        "[{}]",
        (0..length)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

const INTEGER_BOUNDARIES: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    -1_500_000_000,
    -65_536,
    -46_341,
    -46_340,
    -2,
    -1,
    0,
    1,
    2,
    46_340,
    46_341,
    65_536,
    1_500_000_000,
    i32::MAX - 1,
    i32::MAX,
];

const FINAL_MUTATION_INDICES: std::ops::RangeInclusive<i32> = -2..=10;

#[test]
fn model_check_all_boundary_integer_add_sub_mul_pairs() {
    for &left in INTEGER_BOUNDARIES {
        for &right in INTEGER_BOUNDARIES {
            for op in ["+", "-", "*"] {
                let source = format!("({op} {left} {right})");
                let found = diagnostics(&source, 1);
                let mathematical = match op {
                    "+" => i64::from(left) + i64::from(right),
                    "-" => i64::from(left) - i64::from(right),
                    "*" => i64::from(left) * i64::from(right),
                    _ => unreachable!(),
                };
                let below = mathematical < i64::from(i32::MIN);
                let above = mathematical > i64::from(i32::MAX);
                let expected = below || above;
                assert_eq!(
                    has(&found, "overflow") || has(&found, "underflow"),
                    expected,
                    "arithmetic model mismatch\nsource: {source}\nmathematical result: {mathematical}\nexpected overflow={expected}\ndiagnostics: {found:#?}"
                );
                if below && !above {
                    assert!(has(&found, "underflow"), "{source}\n{found:#?}");
                }
                if above && !below {
                    assert!(has(&found, "overflow"), "{source}\n{found:#?}");
                }
            }
        }
    }
}

#[test]
fn model_check_all_boundary_integer_division_and_modulo_pairs() {
    for &left in INTEGER_BOUNDARIES {
        for &right in INTEGER_BOUNDARIES {
            for op in ["/", "%"] {
                let source = format!("({op} {left} {right})");
                let found = diagnostics(&source, 1);
                assert_eq!(
                    has(&found, "divisor may be zero"),
                    right == 0,
                    "zero-divisor model mismatch\nsource: {source}\ndiagnostics: {found:#?}"
                );
                let division_overflow = op == "/" && left == i32::MIN && right == -1;
                assert_eq!(
                    has(&found, "Int overflow:"),
                    division_overflow,
                    "division-overflow model mismatch\nsource: {source}\ndiagnostics: {found:#?}"
                );
            }
        }
    }
}

#[test]
fn model_check_literal_get_and_set_index_spaces() {
    for length in 0..=8usize {
        let vector = vector_literal(length);
        for index in -3..=11i32 {
            let get_source = format!("(get {vector} {index})");
            let get_diagnostics = diagnostics(&get_source, 1);
            let get_safe = index >= 0 && (index as usize) < length;
            assert_eq!(
                !has(&get_diagnostics, "index not proven safe"),
                get_safe,
                "get model mismatch\nsource: {get_source}\nlength: {length}\nindex: {index}\ndiagnostics: {get_diagnostics:#?}"
            );

            let set_source = format!("(set! {vector} {index} 99)");
            let set_diagnostics = diagnostics(&set_source, 1);
            let set_safe = index >= 0 && (index as usize) <= length;
            assert_eq!(
                !has(&set_diagnostics, "set! index not proven safe"),
                set_safe,
                "set! model mismatch\nsource: {set_source}\nlength: {length}\nindex: {index}\ndiagnostics: {set_diagnostics:#?}"
            );
        }
    }
}

#[test]
fn model_check_empty_sensitive_operations_for_literal_lengths() {
    for length in 0..=8usize {
        let vector = vector_literal(length);
        for op in ["car", "pop-val!"] {
            let source = format!("({op} {vector})");
            let found = diagnostics(&source, 1);
            assert_eq!(
                has(&found, "vector may be empty"),
                length == 0,
                "nonempty model mismatch\nsource: {source}\nlength: {length}\ndiagnostics: {found:#?}"
            );
        }
        let pop_source = format!("(pop! {vector})");
        assert!(
            diagnostics(&pop_source, 1).is_empty(),
            "safe pop! warned for length {length}: {pop_source}"
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum Mutation {
    Push,
    ReplaceFirst,
    AppendWithSet,
    Pop,
    PopValue,
}

fn enumerate_mutations(depth: usize, prefix: &mut Vec<Mutation>, out: &mut Vec<Vec<Mutation>>) {
    out.push(prefix.clone());
    if prefix.len() == depth {
        return;
    }
    for mutation in [
        Mutation::Push,
        Mutation::ReplaceFirst,
        Mutation::AppendWithSet,
        Mutation::Pop,
        Mutation::PopValue,
    ] {
        prefix.push(mutation);
        enumerate_mutations(depth, prefix, out);
        prefix.pop();
    }
}

fn mutation_program(initial_length: usize, sequence: &[Mutation]) -> (String, usize, usize) {
    let mut length = initial_length;
    let mut forms = vec![format!("(let xs {})", vector_literal(initial_length))];
    for mutation in sequence {
        match mutation {
            Mutation::Push => {
                forms.push("(push! xs 7)".to_string());
                length += 1;
            }
            Mutation::ReplaceFirst if length > 0 => {
                forms.push("(set! xs 0 7)".to_string());
            }
            Mutation::ReplaceFirst => continue,
            Mutation::AppendWithSet => {
                forms.push("(set! xs (length xs) 7)".to_string());
                length += 1;
            }
            Mutation::Pop => {
                forms.push("(pop! xs)".to_string());
                length = length.saturating_sub(1);
            }
            Mutation::PopValue if length > 0 => {
                forms.push("(pop-val! xs)".to_string());
                length -= 1;
            }
            Mutation::PopValue => continue,
        }
    }
    for final_index in FINAL_MUTATION_INDICES {
        forms.push(format!("(get xs {final_index})"));
    }
    let user_form_count = forms.len() - 1;
    (forms.join(" "), length, user_form_count)
}

#[test]
fn model_check_all_vector_mutation_sequences_through_depth_five() {
    let mut sequences = Vec::new();
    enumerate_mutations(5, &mut Vec::new(), &mut sequences);
    for initial_length in 0..=5usize {
        for sequence in &sequences {
            let (source, final_length, user_form_count) =
                mutation_program(initial_length, sequence);
            let found = diagnostics(&source, user_form_count);
            for final_index in FINAL_MUTATION_INDICES {
                let expected_safe = final_index >= 0 && (final_index as usize) < final_length;
                let final_get_warning = found.iter().any(|message| {
                    message.contains("index not proven safe")
                        && message.contains(&format!("(get xs {final_index})"))
                });
                assert_eq!(
                    !final_get_warning,
                    expected_safe,
                    "mutation model mismatch\ninitial length: {initial_length}\nsequence: {sequence:?}\nfinal length: {final_length}\nfinal index: {final_index}\nsource: {source}\ndiagnostics: {found:#?}"
                );
            }
        }
    }
}

#[test]
fn model_check_equivalent_nonempty_guards() {
    let guards = [
        "(> (length xs) 0)",
        "(< 0 (length xs))",
        "(not (= (length xs) 0))",
        "(not (= 0 (length xs)))",
        "(>= (length xs) 1)",
        "(not (< (length xs) 1))",
        "(= (length xs) 1)",
        "(= 1 (length xs))",
    ];
    for guard in guards {
        for operation in ["(car xs)", "(pop-val! xs)"] {
            let source = format!("(let xs []) (if {guard} {operation} 0)");
            let found = diagnostics(&source, 2);
            assert!(
                !has(&found, "vector may be empty"),
                "equivalent nonempty guard was not recognized\nsource: {source}\ndiagnostics: {found:#?}"
            );
        }
    }
}

#[test]
fn model_check_exact_length_guards_prove_all_smaller_literal_indices() {
    for length in 1..=10usize {
        for index in 0..length {
            for guard in [
                format!("(= (length xs) {length})"),
                format!("(= {length} (length xs))"),
                format!("(let n {length}) (= (length xs) n)"),
            ] {
                let source = if guard.starts_with("(let n") {
                    format!(
                        "(let xs []) (let n {length}) (if (= (length xs) n) (get xs {index}) -1)"
                    )
                } else {
                    format!("(let xs []) (if {guard} (get xs {index}) -1)")
                };
                let found = diagnostics(&source, source.matches("(let ").count() + 1);
                assert!(
                    !has(&found, "index not proven safe"),
                    "exact-length guard did not prove access\nsource: {source}\ndiagnostics: {found:#?}"
                );
            }
        }
    }
}

#[test]
fn model_check_equivalent_nonzero_divisor_guards() {
    let guarded_expressions = [
        "(if (= d 0) 0 (/ n d))",
        "(if (= 0 d) 0 (/ n d))",
        "(if (not (= d 0)) (/ n d) 0)",
        "(if (> d 0) (/ n d) 0)",
        "(if (< d 0) (/ n d) 0)",
        "(if (or (> d 0) (< d 0)) (/ n d) 0)",
    ];
    for body in guarded_expressions {
        let source = format!("(let divide (lambda n d {body}))");
        let found = diagnostics(&source, 1);
        assert!(
            !has(&found, "divisor may be zero"),
            "equivalent nonzero guard was not recognized\nsource: {source}\ndiagnostics: {found:#?}"
        );
    }
}

#[test]
fn model_check_alias_capture_and_comparison_direction_equivalence() {
    let safe_forms = [
        "(let max 2147483647) (let f (lambda x (if (>= x max) x (+ x 1))))",
        "(let max 2147483647) (let f (lambda x (if (<= max x) x (+ x 1))))",
        "(let max 2147483647) (let f (lambda x (if (< x max) (+ x 1) x)))",
        "(let max 2147483647) (let alias max) (let f (lambda x (if (>= x alias) x (+ x 1))))",
    ];
    for source in safe_forms {
        let found = diagnostics(source, source.matches("(let ").count());
        assert!(
            !has(&found, "overflow possible"),
            "constant/ordering equivalence was not recognized\nsource: {source}\ndiagnostics: {found:#?}"
        );
    }
}

#[test]
fn model_check_diagnostic_categories_are_stable_and_nonduplicated() {
    let source = "(let xs []) (let i 4) (get xs i) (set! xs 5 0) (car xs) (pop-val! xs) (/ 1 0) (+ 2147483647 1) (- -2147483648 1)";
    let found = diagnostics(source, 9);
    let expected = [
        "index not proven safe",
        "set! index not proven safe",
        "vector may be empty",
        "divisor may be zero",
        "Int overflow possible",
        "Int underflow possible",
    ];
    for category in expected {
        assert!(has(&found, category), "missing {category}\n{found:#?}");
    }
    let unique: HashSet<_> = found.iter().collect();
    assert_eq!(
        unique.len(),
        found.len(),
        "duplicate diagnostics: {found:#?}"
    );
}
