#[cfg(test)]
mod tests {
    #[test]
    fn test_type_inference_passing_cases() {
        let test_cases = [
            ("(+ 1 2)", "Int"),
            ("(and (> 2 1) (= 1 1))", "Bool"),
            ("(do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b)", "Bool"),
            ("(do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b)", "Bool"),
            ("(do (let xs (vector (vector 1))) (let x (get xs 0)) (let y (get x 0)) y)", "Int"),
            ("(do (let xs (vector (vector 1))) (let x (get (get xs 0) 0)) x)", "Int"),
            ("(lambda x (+ x 1))", "Int -> Int"),
            ("(lambda x (and x (or x x)))", "Bool -> Bool"),
            ("(do (let fn (lambda a b (and a b))) (fn (= 1 1) (= 1 2)))", "Bool"),
            ("(do (let process (lambda xs (get xs 0))) (process (vector 1 2 3 )))", "Int"),
            (
                "(do (let process (lambda xs (do (let x (get xs 0)) x))) (process (vector (= 1 1))))",
                "Bool",
            ),
            ("(vector 1 2 3)", "[Int]"),
            ("(vector (vector (vector 1)))", "[[[Int]]]"),
            ("nil", "()"),
            ("(do (let x 10) (let fn (lambda (do (let x 2) (* x x)))) (fn))", "Int"),
            (
                "(do (let fn! (lambda a b c d (do (set! a (length a) (if d (lambda x (> (+ c b) x)) (lambda _ false))) nil))) fn!)",
                "[Int -> Bool] -> Int -> Int -> Bool -> ()",
            ),
            (
                "(do (let Int 0) (let as (lambda _ t t)) (let xs (as (vector) (vector Int))) xs)",
                "[Int]",
            ),
            ("(tuple 0 true)", "{Int * Bool}"),
            ("(vector (tuple 0 true) (tuple 1 false))", "[{Int * Bool}]"),
            ("(+. 1.23 2.112)", "Dec"),
            ("(tuple (Int->Dec 5) (Dec->Int 5.2))", "{Dec * Int}"),
            (
                r#"(do 
(let xs (vector (vector (vector))))
(set! xs (length xs) (vector (vector true)))
(set! xs (length xs) (vector (vector false)))
xs)"#,
                "[[[Bool]]]",
            ),
            (
                "(do (mut x 0) (mut i 0) (while (< i 3) (do (alter! x (+ x i)) (alter! i (+ i 1)))) x)",
                "Int",
            ),
        ];

        for (inp, out) in &test_cases {
            let exprs = crate::parser::parse(inp).unwrap();

            if let Some(expr) = exprs.first() {
                let result = crate::infer
                    ::infer_with_builtins_typed(
                        expr,
                        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                    )
                    .map(|(typ, _)| typ);
                // Assert that the result is Ok
                assert!(result.is_ok(), "Type inference should succeed for expression: {}", inp);
                // Optionally, check that the type is Int
                if let Ok(typ) = result {
                    // println!("{:?}", inp);
                    assert_eq!(typ.to_string(), *out, "Type of expression should match expected");
                }
            } else {
                panic!("No expressions found in parsed result for: {}", inp);
            }
        }
    }

    #[test]
    fn test_type_inference_allows_repeated_discard_params() {
        let exprs = crate::parser::parse("(lambda _ _ 0)").unwrap();
        let expr = exprs.first().expect("lambda should parse");
        let result = crate::infer
            ::infer_with_builtins_typed(
                expr,
                crate::types::create_builtin_environment(crate::types::TypeEnv::new())
            )
            .map(|(typ, _)| typ.to_string());
        assert!(result.is_ok(), "discard params should infer successfully");
        let typ = result.unwrap();
        assert!(
            typ.ends_with("-> Int"),
            "discard-param lambda should still infer as a function, got {}",
            typ
        );
    }

    #[test]
    fn test_type_inference_failure() {
        // Test cases that should result in type inference errors
        let test_cases = [
            ("(+ 1 (= 1 1))", r#"Cannot unify Int with Bool
(+ 1 (= 1 1))"#),
            ("(1 2)", "Cannot apply non-function type: Int\n(1 2)"),
            ("(do (let t 10) (t))", "Cannot apply non-function type: Int\n(t)"),
            ("(let x (vector 1 2 (= 1 2)))", "Cannot unify Int with Bool\n(vector 1 2 (= 1 2))"),
            ("(vector 1 2 (> 1 2))", "Cannot unify Int with Bool\n(vector 1 2 (> 1 2))"),
            ("(lambda x (and x 42))", "Cannot unify Bool with Int\n(and x 42)"),
            ("(summation (range 1 10))", "Undefined variable: summation"),
            ("(if 1 10 20)", r#"Cannot unify Int with Bool
Condition must be Bool
(if 1 10 20)"#),
            (
                "(if (= 1 2) 10 (= 0 1))",
                r#"Cannot unify Int with Bool
Concequent and alternative must match types
(if (= 1 2) 10 (= 0 1))"#,
            ),
            ("(do (let x 10) (let x 2))", "Variable 'x' already defined in this scope"),
            (
                "(vector (tuple 0 true) (tuple true 0))",
                "Cannot unify Int with Bool\n(vector (tuple 0 true) (tuple true 0))",
            ),
            ("(+ 1.23 2)", "Cannot unify Int with Dec\n(+ 1.23 2)"),
            (
                r#"(do (let xs (vector (vector (vector))))
(set! xs (length xs) (vector (vector true)))
(set! xs (length xs) (vector (vector 1))))"#,
                "Cannot unify Int with Bool\n(set! xs (length xs) (vector (vector 1)))",
            ),
            (
                r#"(do 
(let xs (vector))
(set! xs (length xs) false)
(set! xs (length xs) 1))"#,
                "Cannot unify Int with Bool\n(set! xs (length xs) 1)",
            ),
            (
                "(do (mut x 1) (let f (lambda y x)) (f 0))",
                "mut variable 'x' cannot be captured by lambda; use &mut cells for closure-shared mutation",
            ),
            (
                "(do (mut x 1) (let f (lambda y (alter! x y))) (f 0))",
                "mut variable 'x' cannot be captured by lambda; use &mut cells for closure-shared mutation",
            ),
        ];

        for (idx, (inp, out)) in test_cases.iter().enumerate() {
            eprintln!("test_correctness case {}", idx);
            let exprs = crate::parser::parse(inp).unwrap();

            if let Some(expr) = exprs.first() {
                // Check that type inference returns an Err
                let result = crate::infer
                    ::infer_with_builtins_typed(
                        expr,
                        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                    )
                    .map(|(typ, _)| typ);
                // Assert that the result is an Err

                assert!(result.is_err(), "Expected type inference error for expression: {}", inp);

                // Optionally, you can check the error message
                if let Err(error_msg) = result {
                    assert_eq!(error_msg.to_string(), *out, "Type error should match expected");
                }
            } else {
                panic!("No expressions found in parsed result");
            }
        }
    }

    #[test]
    fn test_merge_std_rejects_forbidden_fusion_name_shadowing() {
        let std_ast = crate::baked::load_ast();
        let wrapped = match std_ast {
            crate::parser::Expression::Apply(items) => {
                crate::parser::merge_std_and_program("(let map 10)\nmap", items[1..].to_vec())
            }
            _ => panic!("std ast should be (do ...)"),
        };
        assert_eq!(
            wrapped.err().unwrap_or_else(|| "expected error".to_string()),
            "Variable 'map' is forbidden"
        );
    }

    #[test]
    fn test_merge_std_rejects_reserved_lambda_param_name() {
        let std_ast = crate::baked::load_ast();
        let wrapped = match std_ast {
            crate::parser::Expression::Apply(items) =>
                crate::parser::merge_std_and_program(
                    "(let lower (lambda char char))\nlower",
                    items[1..].to_vec()
                ),
            _ => panic!("std ast should be (do ...)"),
        };
        assert_eq!(
            wrapped.err().unwrap_or_else(|| "expected error".to_string()),
            "Variable 'char' is forbidden"
        );
    }

    #[test]
    fn test_baked_ast_to_definitions_flattens_nested_top_level_do() {
        let ast = crate::parser
            ::build("(do (do (let inc (lambda x (+ x 1))) (let dec (lambda x (- x 1)))))")
            .expect("nested do should parse");
        let defs = crate::baked
            ::ast_to_definitions(ast, "test")
            .expect("nested top-level do should be flattened");
        assert_eq!(defs.len(), 2);
    }

    #[test]
    fn test_parser_reports_structured_unexpected_closer_for_offbalance_delimiters() {
        let err = crate::parser::build("(do (let x 1)))").expect_err("should fail delimiter check");
        assert!(
            err.contains("parse.delimiter_error: unexpected_closer"),
            "missing error kind, got:\n{}",
            err
        );
        assert!(err.contains("parse.found: ')' at"), "missing found location, got:\n{}", err);
        assert!(err.contains("parse.fix_hint[0]:"), "missing repair hint, got:\n{}", err);
        assert!(err.contains("parse.line_balance["), "missing line balance window, got:\n{}", err);
    }

    #[test]
    fn test_parser_reports_structured_unclosed_opener_at_eof() {
        let err = crate::parser::build("(do (let x 1)").expect_err("should fail delimiter check");
        assert!(
            err.contains("parse.delimiter_error: unclosed_opener"),
            "missing error kind, got:\n{}",
            err
        );
        assert!(
            err.contains("parse.expected_before_eof: ')'"),
            "missing eof expectation, got:\n{}",
            err
        );
        assert!(err.contains("parse.open_stack[0]:"), "missing open stack, got:\n{}", err);
    }

    #[test]
    fn test_parser_normalizes_nested_application_syntax() {
        let expr = crate::parser
            ::build("((make-adder 2) 3)")
            .expect("nested application should parse");
        assert_eq!(expr.to_lisp(), "(do ((make-adder 2) 3))");
    }

    #[test]
    fn test_parser_apply_alias_matches_direct_application() {
        let expr = crate::parser
            ::build("(apply (make-adder 2) 3)")
            .expect("apply alias should parse");
        assert_eq!(expr.to_lisp(), "(do ((make-adder 2) 3))");
    }

    #[test]
    fn test_parser_expands_top_level_letmacro_before_desugar() {
        let expr = crate::parser
            ::build(
                "(do
                    (letmacro unless (lambda cond body (qq (if (not (uq cond)) (uq body) nil))))
                    (unless false (+ 1 2)))"
            )
            .expect("letmacro program should build");
        let built = expr.to_lisp();
        assert!(
            !built.contains("letmacro"),
            "macro definitions should be compile-time only, got: {}",
            built
        );
        assert!(
            !built.contains("(unless "),
            "macro call should be expanded away before desugar, got: {}",
            built
        );
        assert!(
            built.contains("(if (not false) (+ 1 2) nil)"),
            "unless macro should expand into if/not form, got: {}",
            built
        );
    }

    #[test]
    fn test_parser_macroexpand_and_macroexpand_1_render_expanded_source() {
        let expr = crate::parser
            ::build(
                "(do
                    (letmacro unless (lambda cond body (qq (if (not (uq cond)) (uq body) nil))))
                    (letmacro when-not (lambda cond body (qq (unless (uq cond) (uq body)))))
                    [(macroexpand-1 (when-not false (+ 1 2)))
                     (macroexpand (when-not false (+ 1 2)))])"
            )
            .expect("macroexpand forms should build");
        let built = expr.to_lisp();
        assert!(
            built.contains("(string 40 117 110 108 101 115 115"),
            "macroexpand-1 should render one-step expansion as a string literal, got: {}",
            built
        );
        assert!(
            built.contains("(string 40 105 102 32 40 110 111 116"),
            "macroexpand should render full expansion as a string literal, got: {}",
            built
        );
    }

    #[test]
    fn test_parser_variadic_letmacro_supports_rest_param_and_splice() {
        let expr = crate::parser
            ::build(
                "(do
                    (letmacro when (lambda cond . body (qq (if (uq cond) (do (uqs body)) nil))))
                    (when true (+ 1 2) (+ 3 4) nil))"
            )
            .expect("variadic letmacro should build");
        let built = expr.to_lisp();
        assert!(
            built.contains("(if true (do (+ 1 2) (+ 3 4) nil) nil)"),
            "variadic macro should splice rest body into do, got: {}",
            built
        );
    }

    #[test]
    fn test_parser_multiclause_letmacro_dispatches_by_arity() {
        let expr = crate::parser
            ::build(
                "(do
                    (letmacro unless
                      ((cond) (qq (if (uq cond) nil nil)))
                      ((cond body) (qq (if (uq cond) nil (uq body))))
                      ((cond then else) (qq (if (uq cond) (uq else) (uq then)))))
                    [(unless false)
                     (unless false (+ 1 2))
                     (unless false (+ 1 2) 9)])"
            )
            .expect("multi-clause letmacro should build");
        let built = expr.to_lisp();
        assert!(
            built.contains("(if false nil nil)"),
            "one-arg clause should expand correctly, got: {}",
            built
        );
        assert!(
            built.contains("(if false nil (+ 1 2))"),
            "two-arg clause should expand correctly, got: {}",
            built
        );
        assert!(
            built.contains("(if false 9 (+ 1 2))"),
            "three-arg clause should expand correctly, got: {}",
            built
        );
    }

    #[test]
    fn test_parser_vector_destructure_uses_explicit_dot_rest() {
        let expr = crate::parser
            ::build("(lambda [a b . rest] [a b rest])")
            .expect("explicit vector rest pattern should build");
        let built = expr.to_lisp();
        assert!(
            built.contains("(let rest (cdr"),
            "vector rest should lower through cdr binding, got: {}",
            built
        );
    }

    #[test]
    fn test_parser_vector_destructure_without_dot_no_longer_captures_implicit_rest() {
        let expr = crate::parser
            ::build("(lambda [a b c] [a b c])")
            .expect("fixed-width vector pattern should build");
        let built = expr.to_lisp();
        assert!(
            !built.contains("(let c (cdr"),
            "last vector binding should not capture implicit rest anymore, got: {}",
            built
        );
    }

    #[test]
    fn test_parser_right_nests_tuple_literals_with_more_than_two_items() {
        let expr = crate::parser::build("{1 2 3 4}").expect("tuple literal should build");
        assert_eq!(expr.to_lisp(), "(do (tuple 1 (tuple 2 (tuple 3 4))))");
    }

    #[test]
    fn test_runtime_tuple_destructure_accepts_flat_surface_sugar() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let unpack (lambda {a b c} {a {b c}}))
                (unpack {1 2 3}))"#,
            true
        );
        assert_eq!(output.trim(), "{ 1 { 2 3 } }");
    }

    #[test]
    fn test_parser_tuple_destructure_single_element_skips_tail() {
        let expr = crate::parser
            ::build("(do (let {candidate_id} {[] 1}) candidate_id)")
            .expect("single-element tuple pattern should build");
        let built = expr.to_lisp();
        assert!(
            built.contains("(let candidate_id (fst"),
            "expected single-element tuple pattern to bind fst and skip snd, got: {}",
            built
        );
    }

    #[test]
    fn test_runtime_tuple_destructure_single_element_skips_tail() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let { candidate_id } { [] 1 })
                candidate_id)"#,
            true
        );
        assert_eq!(output.trim(), "[]");
    }

    #[test]
    fn test_runtime_tuple_destructure_flat_pattern_keeps_right_nested_tail() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let { a b c } { 1 2 3 })
                [a b c])"#,
            true
        );
        assert_eq!(output.trim(), "[1 2 3]");
    }

    #[test]
    fn test_runtime_specialized_literal_constructors_support_strings_and_ints() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let xs (strings "a" "b"))
                (let ys (integers 1 2 3))
                {xs ys})"#,
            true
        );
        assert_eq!(output.trim(), "{ [a b] [1 2 3] }");
    }

    #[test]
    fn test_lambda_grouped_params_multiple_body_forms_wrap_implicit_do() {
        let expr = crate::parser
            ::build("(lambda (x) (print! x) (+ x 1))")
            .expect("grouped-param lambda with multiple body forms should build");
        let lisp = expr.to_lisp();
        assert!(
            lisp.contains("(lambda x (do (print! x) (+ x 1)))"),
            "expected implicit do-wrapped lambda body, got: {}",
            lisp
        );
    }

    #[test]
    fn test_runtime_grouped_param_lambda_multiple_body_forms_execute_without_explicit_do() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let f! (lambda (x)
                  (+ x 1)
                  (+ x 2)))
                (f! 4))"#,
            true
        );
        assert_eq!(output.trim(), "6");
    }

    #[test]
    fn test_runtime_std_dec_log_and_aliases_work() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                [
                  (=. (log 1.0) 0.0)
                  (and (>. (log 2.0) 0.68) (<. (log 2.0) 0.71))
                  (and (>. (log 8.0) 2.07) (<. (log 8.0) 2.09))
                  (fst (std/dec/log/option 2.0))
                  (not (fst (std/dec/log/option 0.0)))
                ])"#,
            true
        );
        assert_eq!(output.trim(), "[true true true true true]");
    }

    #[test]
    fn test_lsp_base_environment_includes_macro_keywords() {
        let (_, _, signatures, _) = crate::lsp_native_core::build_base_environment(&[]);
        let signature = signatures
            .get("letmacro")
            .expect("letmacro should be exposed as an LSP special keyword");
        assert!(
            signature.contains("compile-time macro definition"),
            "unexpected letmacro signature: {}",
            signature
        );
    }

    #[test]
    fn test_lsp_collects_letmacro_names_as_bound_symbols() {
        let exprs = crate::lsp_native_core
            ::parse_user_exprs_for_symbol_collection(
                "(do (letmacro unless (lambda cond body (qq (if (not (uq cond)) (uq body) nil)))) (unless false 1))"
            )
            .expect("macro program should parse for symbol collection");
        let mut names = std::collections::HashSet::new();
        crate::lsp_native_core::collect_user_bound_symbols_from_exprs(&exprs, &mut names);
        assert!(
            names.contains("unless"),
            "letmacro-bound names should be visible to LSP symbol collection"
        );
    }

    #[test]
    fn test_parser_macro_expansion_errors_include_call_context() {
        let err = crate::parser
            ::build("(do (letmacro two (lambda a b (qq (+ (uq a) (uq b))))) (two 1))")
            .expect_err("macro arity mismatch should fail");
        assert!(
            err.contains("Macro 'two' expected one of [2] args"),
            "error should include macro arity expectation, got: {}",
            err
        );
        assert!(err.contains("(two 1)"), "error should include failing macro call, got: {}", err);
    }

    #[test]
    fn test_lsp_undefined_variable_suggestions_rank_typo_closest() {
        let suggestions = crate::lsp_native_core::suggest_undefined_variable_candidates(
            "Undefined variable: rnage",
            ["range", "reduce", "map", "window"].iter().copied(),
            3
        );
        assert_eq!(suggestions.first().map(String::as_str), Some("range"));
    }

    #[test]
    fn test_lsp_undefined_variable_suggestions_append_only_for_undefined_variable_errors() {
        let unchanged = crate::lsp_native_core::append_undefined_variable_suggestions(
            "Cannot unify Int with Bool",
            ["range", "reduce", "map"].iter().copied(),
            3
        );
        assert_eq!(unchanged, "Cannot unify Int with Bool");

        let with_hint = crate::lsp_native_core::append_undefined_variable_suggestions(
            "Undefined variable: mpa",
            ["map", "filter", "reduce"].iter().copied(),
            3
        );
        assert!(with_hint.contains("Did you mean: map"), "got:\n{}", with_hint);
    }

    #[test]
    fn test_lsp_infer_error_ranges_unresolved_returns_empty_for_whole_file_fallback() {
        let text = "(map (lambda x x) [1 2 3])";
        let message = "Cannot unify Int with Bool";
        let ranges = crate::lsp_native_core::infer_error_ranges(text, message, None);
        assert!(
            ranges.is_empty(),
            "expected unresolved location to return empty ranges, got: {:?}",
            ranges
        );
    }

    #[test]
    fn test_loop_while_desugars_to_do_body_with_trailing_nil() {
        let expr = crate::parser::build("(while false (+ 1 2))").expect("while should desugar");
        let lisp = expr.to_lisp();
        assert!(
            lisp.contains("(while false (do (+ 1 2) nil))"),
            "expected while to desugar to do body + trailing nil, got: {}",
            lisp
        );
    }

    #[test]
    fn test_loop_while_allows_mutating_body_without_lambda_argument() {
        let expr = crate::parser
            ::build("(do (mut i 0) (while (< i 3) (alter! i (+ i 1))) i)")
            .expect("program should build");
        let (typ, _typed) = crate::infer
            ::infer_with_builtins_typed(
                &expr,
                crate::types::create_builtin_environment(crate::types::TypeEnv::new())
            )
            .expect("program should infer");
        assert_eq!(typ.to_string(), "Int");
    }

    #[test]
    fn test_loop_while_multiple_body_forms_wrap_implicit_do() {
        let expr = crate::parser
            ::build("(while (< i 3) (alter! i (+ i 1)) (alter! acc (+ acc i)))")
            .expect("while with multiple body forms should build");
        let lisp = expr.to_lisp();
        assert!(
            lisp.contains("(while (< i 3) (do (alter! i (+ i 1)) (alter! acc (+ acc i)) nil))"),
            "expected implicit do-wrapped while body, got: {}",
            lisp
        );
    }

    #[cfg(feature = "runtime")]
    fn runtime_exec_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    #[cfg(feature = "runtime")]
    struct ScopedEnvVar {
        key: &'static str,
        prev: Option<String>,
    }

    #[cfg(feature = "runtime")]
    impl ScopedEnvVar {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    #[cfg(feature = "runtime")]
    impl Drop for ScopedEnvVar {
        fn drop(&mut self) {
            if let Some(prev) = self.prev.as_ref() {
                std::env::set_var(self.key, prev);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[cfg(feature = "runtime")]
    fn run_program_output_unlocked(src: &str) -> String {
        let expr = crate::parser::build(src).expect("program should build");
        let wat = crate::wat
            ::compile_program_to_wat_with_opts(&expr, true)
            .expect("program should compile");
        let argv: Vec<String> = Vec::new();
        #[cfg(feature = "io")]
        let store_data = crate::io::ShellStoreData
            ::new_with_security(None, crate::io::ShellPolicy::disabled())
            .map_err(|e| e.to_string())
            .expect("io store should initialize");
        #[cfg(feature = "io")]
        let run_result = crate::runtime::run_wat_text(&wat, store_data, &argv, |linker| {
            crate::io::add_shell_to_linker(linker).map_err(|e| e.to_string())
        });
        #[cfg(not(feature = "io"))]
        let run_result = crate::runtime::run_wat_text(&wat, (), &argv, |_linker| Ok(()));
        run_result.expect("program should run without trap")
    }

    #[cfg(feature = "runtime")]
    fn run_program_error_unlocked(src: &str) -> String {
        let expr = crate::parser::build(src).expect("program should build");
        let wat = crate::wat
            ::compile_program_to_wat_with_opts(&expr, true)
            .expect("program should compile");
        let argv: Vec<String> = Vec::new();
        #[cfg(feature = "io")]
        let store_data = crate::io::ShellStoreData
            ::new_with_security(None, crate::io::ShellPolicy::disabled())
            .map_err(|e| e.to_string())
            .expect("io store should initialize");
        #[cfg(feature = "io")]
        let run_result = crate::runtime::run_wat_text(&wat, store_data, &argv, |linker| {
            crate::io::add_shell_to_linker(linker).map_err(|e| e.to_string())
        });
        #[cfg(not(feature = "io"))]
        let run_result = crate::runtime::run_wat_text(&wat, (), &argv, |_linker| Ok(()));
        run_result.expect_err("program should fail at runtime")
    }

    #[cfg(feature = "runtime")]
    fn run_program_error_with_debug_guards(src: &str) -> String {
        let _lock = runtime_exec_lock().lock().expect("runtime test lock should not be poisoned");
        let _int_overflow = ScopedEnvVar::set("QUE_INT_OVERFLOW_CHECK", "1");
        let _float_overflow = ScopedEnvVar::set("QUE_FLOAT_OVERFLOW_CHECK", "1");
        let _div_zero = ScopedEnvVar::set("QUE_DIV_ZERO_CHECK", "1");
        let _bounds = ScopedEnvVar::set("QUE_BOUNDS_CHECK", "1");
        run_program_error_unlocked(src)
    }

    #[cfg(feature = "runtime")]
    fn run_program_output(src: &str) -> String {
        let _lock = runtime_exec_lock().lock().expect("runtime test lock should not be poisoned");
        run_program_output_unlocked(src)
    }

    #[cfg(feature = "runtime")]
    fn run_program_error(src: &str) -> String {
        let _lock = runtime_exec_lock().lock().expect("runtime test lock should not be poisoned");
        run_program_error_unlocked(src)
    }

    #[cfg(feature = "runtime")]
    fn run_program_output_with_std_and_opts(src: &str, enable_optimizer: bool) -> String {
        let _lock = runtime_exec_lock().lock().expect("runtime test lock should not be poisoned");
        let std_ast = crate::baked::load_ast();
        let expr = match std_ast {
            crate::parser::Expression::Apply(items) => {
                crate::parser
                    ::merge_std_and_program(src, items[1..].to_vec())
                    .expect("program + std should merge")
            }
            _ => panic!("std ast should be (do ...)"),
        };
        let wat = crate::wat
            ::compile_program_to_wat_with_opts(&expr, enable_optimizer)
            .expect("program should compile");
        let argv: Vec<String> = Vec::new();
        #[cfg(feature = "io")]
        let store_data = crate::io::ShellStoreData
            ::new_with_security(None, crate::io::ShellPolicy::disabled())
            .map_err(|e| e.to_string())
            .expect("io store should initialize");
        #[cfg(feature = "io")]
        let run_result = crate::runtime::run_wat_text(&wat, store_data, &argv, |linker| {
            crate::io::add_shell_to_linker(linker).map_err(|e| e.to_string())
        });
        #[cfg(not(feature = "io"))]
        let run_result = crate::runtime::run_wat_text(&wat, (), &argv, |_linker| Ok(()));
        run_result.expect("program should run without trap")
    }

    #[cfg(feature = "runtime")]
    fn assert_std_program_output_matches_with_and_without_optimizer(src: &str) {
        let output_no_opts = run_program_output_with_std_and_opts(src, false);
        let output_with_opts = run_program_output_with_std_and_opts(src, true);
        assert_eq!(
            output_with_opts,
            output_no_opts,
            "optimizer output must match non-optimized output"
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_loop_range_runtime_produces_expected_sequence() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let push! (lambda xs x (do (set! xs (length xs) x) xs)))
                (let xs [])
                (loop 0 10 (lambda i (push! xs i)))
                xs)"#,
            true
        );
        assert_eq!(output, "[0 1 2 3 4 5 6 7 8 9]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_cons_builtin_concatenates_vectors_without_std() {
        let output = run_program_output(r#"(cons [ 1 2 3 ] [ 4 5 6 ])"#);
        assert_eq!(output, "[1 2 3 4 5 6]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_cons_builtin_preferred_over_std_definition() {
        let output = run_program_output_with_std_and_opts(r#"(cons [ 1 2 ] [ 3 4 ])"#, true);
        assert_eq!(output, "[1 2 3 4]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_inline_zip_map_pipeline_with_distinct_lambda_names_works() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let { candidate_id skill } {
                  [123 123 123 234 234 234 345 345]
                  ["Python" "Tableau" "PostgreSQL" "R" "PowerBI" "SQL Server" "Python" "Tableau"]
                })
                (|> (zip { candidate_id skill })
                    (map (lambda { cid sk } { cid sk }))))"#,
            true
        );
        assert_eq!(
            output,
            r#"[{ 123 Python } { 123 Tableau } { 123 PostgreSQL } { 234 R } { 234 PowerBI } { 234 SQL Server } { 345 Python } { 345 Tableau }]"#
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_while_multiple_body_forms_execute_without_explicit_do() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (mut i 0)
                (mut acc 0)
                (while (< i 3)
                  (alter! acc (+ acc i))
                  (alter! i (+ i 1)))
                acc)"#,
            true
        );
        assert_eq!(output, "3");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_inline_zip_map_pipeline_shadowing_regression() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let { candidate_id skill } {
                  [123 123 123 234 234 234 345 345]
                  ["Python" "Tableau" "PostgreSQL" "R" "PowerBI" "SQL Server" "Python" "Tableau"]
                })
                (|> (zip { candidate_id skill })
                    (map (lambda { candidate_id skill } { candidate_id skill }))))"#,
            true
        );
        assert_eq!(
            output,
            r#"[{ 123 Python } { 123 Tableau } { 123 PostgreSQL } { 234 R } { 234 PowerBI } { 234 SQL Server } { 345 Python } { 345 Tableau }]"#
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_nested_letrec_inside_lambda_survives_optimized_capture_analysis() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let INPUT "12
14
1969
100756")
                (let parse
                    (lambda input (|> input (String->Vector nl) (map Chars->Integer))))
                (let PARSED (parse INPUT))
                (let part1
                    (lambda input
                        (|> input (map (lambda x (- (/ x 3) 2))) (sum))))
                (let part2
                    (lambda input
                        (do
                            (let retry
                                (lambda x
                                    (do
                                        (letrec tail-call:retry!
                                            (lambda x out
                                                (do
                                                    (let result (- (/ x 3) 2))
                                                    (if (<= result 0)
                                                        out
                                                        (tail-call:retry! result
                                                            (do (push! out result) out))))))
                                        (tail-call:retry! x []))))
                            (|> input (map retry) (map sum) (sum)))))
                [(part1 PARSED) (part2 PARSED)])"#,
            true
        );
        assert_eq!(output, "[34241 51316]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_letmacro_expands_and_runs_through_normal_pipeline() {
        let output = run_program_output(
            r#"(do
                (letmacro inc1 (lambda x (qq (+ (uq x) 1))))
                (inc1 41))"#
        );
        assert_eq!(output, "42");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_library_macros_expand_library_defs_and_user_program() {
        let lib_ast = crate::parser
            ::build_library(
                r#"(do
                    (letmacro inc1 (lambda x (qq (+ (uq x) 1))))
                    (let add2 (lambda x (inc1 (inc1 x)))))"#
            )
            .expect("library source should parse without stripping macros");
        let lib_defs = crate::baked
            ::ast_to_definitions(lib_ast, "test library")
            .expect("library defs should flatten");
        let expr = crate::parser
            ::merge_std_and_program("(inc1 (add2 40))", lib_defs)
            .expect("library macro should expand in std defs and user program");
        let wat = crate::wat
            ::compile_program_to_wat_with_opts(&expr, true)
            .expect("expanded program should compile");
        #[cfg(feature = "io")]
        let store_data = crate::io::ShellStoreData
            ::new_with_security(None, crate::io::ShellPolicy::disabled())
            .map_err(|e| e.to_string())
            .expect("io store should initialize");
        let argv: Vec<String> = Vec::new();
        #[cfg(feature = "io")]
        let run_result = crate::runtime::run_wat_text(&wat, store_data, &argv, |linker| {
            crate::io::add_shell_to_linker(linker).map_err(|e| e.to_string())
        });
        #[cfg(not(feature = "io"))]
        let run_result = crate::runtime::run_wat_text(&wat, (), &argv, |_linker| Ok(()));
        assert_eq!(run_result.expect("program should run"), "43");
    }

    #[test]
    fn test_baked_embedded_library_preserves_macros_for_user_program_merge() {
        let std_ast = crate::baked::load_ast();
        let lib_defs = crate::baked
            ::ast_to_definitions(std_ast, "active library")
            .expect("embedded library should flatten to definitions");
        let expr = crate::parser
            ::merge_std_and_program("(do (let out []) (when true (set! out 0 0)) out)", lib_defs)
            .expect("user program should be able to use baked library macros");
        let rendered = expr.to_lisp();
        assert!(
            rendered.contains("(if true"),
            "when macro should expand through baked embedded library path, got: {}",
            rendered
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_macroexpand_and_variadic_when_macro_work() {
        let expand_output = run_program_output(
            r#"(do
                (letmacro unless
                    ((cond) (qq (if (uq cond) nil nil)))
                    ((cond body) (qq (if (uq cond) nil (uq body))))
                    ((cond then else) (qq (if (uq cond) (uq else) (uq then)))))
                (letmacro when-not (lambda cond body (qq (unless (uq cond) (uq body)))))
                [
                    (macroexpand-1 (when-not false (+ 1 2)))
                    (macroexpand (when-not false (+ 1 2)))
                ])"#
        );
        assert_eq!(expand_output, "[(unless false (+ 1 2)) (if false nil (+ 1 2))]");

        let when_output = run_program_output(
            r#"(do
                (letmacro when (lambda cond . body (qq (if (uq cond) (do (uqs body)) nil))))
                (when true (+ 1 2) (+ 3 4) nil))"#
        );
        assert_eq!(when_output, "0");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_letmacro_supports_compile_time_do_let_and_gensym() {
        let output = run_program_output(
            r#"(do
                (letmacro with-temp
                    (lambda expr body
                        (do
                            (let tmp (gensym))
                            (qq (do
                                    (let (uq tmp) (uq expr))
                                    ((uq body) (uq tmp)))))))
                (with-temp (+ 1 2) (lambda t (+ t 10))))"#
        );
        assert_eq!(output, "13");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_letstar_macro_supports_sequential_bindings_with_single_body_expr() {
        let output = run_program_output_with_std_and_opts(r#"(let* a 1 b (+ a 2) (+ a b))"#, true);
        assert_eq!(output, "4");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_csv_helpers_and_with_csv_columns_macro_work() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let text (cons
                    "Ids,User Names,User Ages,Active,Score" [nl]
                    "1,Alice,30,true,1.5" [nl]
                    "2,Bob,,false,"))
                (with-csv-columns text [',']
                    ids "Ids" csv/column/int 0
                    user_names "User Names" csv/column/string ""
                    user_ages "User Ages" csv/column/int 0
                    active "Active" csv/column/bool false
                    score "Score" csv/column/decimal 0.0
                    [(sum ids)
                     (sum user_ages)
                     (if (get active 0) 1 0)
                     (Dec->Int (get score 0))
                     (length user_names)]))"#,
            true
        );
        assert_eq!(output, "[3 30 1 1 2]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_cdr_tail_view_materializes_on_mutation_without_touching_source() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let xs [1 2 3 4 5])
                (let ys (cdr xs 2))
                (set! ys 0 99)
                { xs ys })"#,
            true
        );
        assert_eq!(output, "{ [1 2 3 4 5] [99 4 5] }");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_multiclause_unless_macro_expands_legacy_shapes_for_unit_bodies() {
        let output = run_program_output(
            r#"(do
                (letmacro unless
                    ((cond) (qq (if (uq cond) nil nil)))
                    ((cond body) (qq (if (uq cond) nil (uq body))))
                    ((cond then else) (qq (if (uq cond) (uq else) (uq then)))))
                [(unless false)
                 (unless false nil)
                 (unless false nil nil)])"#
        );
        assert_eq!(output, "[0 0 0]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_not_equal_alias_macros_expand_from_baked_library() {
        let output = run_program_output_with_std_and_opts(
            r#"[(!= 1 2) (!= 2 2) (<> 3 4) (<> 5 5)]"#,
            true
        );
        assert_eq!(output, "[true false true false]");
    }

    #[test]
    fn test_baked_cond_macro_preserves_legacy_parser_expansion_shapes() {
        let std_ast = crate::baked::load_ast();
        let lib_defs = crate::baked
            ::ast_to_definitions(std_ast, "active library")
            .expect("embedded library should flatten to definitions");
        let expr = crate::parser
            ::merge_std_and_program("[(cond) (cond false 1 true 2 3)]", lib_defs)
            .expect("cond macro should expand through baked library merge");
        let rendered = expr.to_lisp();
        assert!(
            rendered.contains("(if false 1 (if true 2 3))"),
            "cond macro should expand into nested ifs, got: {}",
            rendered
        );
        assert!(
            rendered.contains("(vector 0") || rendered.contains("[0 "),
            "empty cond should expand to 0, got: {}",
            rendered
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_runtime_cond_macro_runs_through_baked_library() {
        let output = run_program_output_with_std_and_opts(r#"(cond false 1 true 2 3)"#, true);
        assert_eq!(output, "2");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_cons_builtin_works_as_higher_order_function_value() {
        let output = run_program_output_with_std_and_opts(
            r#"(reduce cons [] [[1 2] [3] [4 5]])"#,
            true
        );
        assert_eq!(output, "[1 2 3 4 5]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_vector_get_out_of_bounds_traps() {
        let err = run_program_error(
            r#"(do
                (let xs [ 1 2 3 4 ])
                [(get xs -1) (get xs 4) (get xs 10)])"#
        );
        assert!(
            err.contains("call error"),
            "expected runtime call error for out-of-bounds get, got: {}",
            err
        );
        assert!(
            err.contains("unreachable"),
            "expected unreachable trap for out-of-bounds get, got: {}",
            err
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_int_overflow_wraps_by_default() {
        let output = run_program_output(r#"(+ 2147483647 1)"#);
        assert_eq!(output, "-2147483648");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_alter_inline_rhs_matches_precompute_with_int_overflow_check() {
        let _lock = runtime_exec_lock().lock().expect("runtime test lock should not be poisoned");
        let _int_overflow = ScopedEnvVar::set("QUE_INT_OVERFLOW_CHECK", "1");
        let output = run_program_output_unlocked(
            r#"(do
                (let digits [1 2])
                (mut num-inline 0)
                (mut base-inline 10)
                (mut i-inline 0)
                (while (< i-inline 2) (do
                  (alter! num-inline (+ num-inline (* base-inline (get digits i-inline))))
                  (alter! base-inline (/ base-inline 10))
                  (alter! i-inline (+ i-inline 1))))
                (mut num-pre 0)
                (mut base-pre 10)
                (mut i-pre 0)
                (while (< i-pre 2) (do
                  (let term (* base-pre (get digits i-pre)))
                  (alter! num-pre (+ num-pre term))
                  (alter! base-pre (/ base-pre 10))
                  (alter! i-pre (+ i-pre 1))))
                [num-inline num-pre])"#
        );
        assert_eq!(output, "[12 12]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_int_div_zero_traps_with_debug_guards() {
        let err = run_program_error_with_debug_guards(r#"(do (let id (lambda x x)) (/ 1 (id 0)))"#);
        assert!(
            err.contains("integer divide by zero") || err.contains("unreachable"),
            "expected unreachable trap for int divide-by-zero, got: {}",
            err
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_float_div_zero_traps_with_debug_guards() {
        let err = run_program_error_with_debug_guards(
            r#"(do (let f (lambda z (/. (Int->Dec 1) z))) (f (Int->Dec 0)))"#
        );
        assert!(
            err.contains("unreachable"),
            "expected unreachable trap for dec divide-by-zero, got: {}",
            err
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_decimal_literal_scaling_and_basic_ops() {
        let output = run_program_output(
            r#"[ 3.14
                3.1425
                (+. 1.5 2.25)
                (*. 1.5 2.25)
                (/. 7.5 2.5) ]"#
        );
        assert_eq!(output, "[3.14 3.142 3.75 3.375 3]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_decimal_int_conversions() {
        let output = run_program_output(
            r#"(do
                (let a (Dec->Int 3.99))
                (let b (Int->Dec 7))
                (tuple a b))"#
        );
        assert_eq!(output, "{ 3 7 }");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_decimal_scale_env_changes_literal_quantization_and_display() {
        let _lock = runtime_exec_lock().lock().expect("runtime test lock should not be poisoned");
        let _scale = ScopedEnvVar::set("QUE_DECIMAL_SCALE", "100");
        let output = run_program_output_unlocked(r#"[ 3.141 3.146 (+. 1.11 2.22) ]"#);
        assert_eq!(output, "[3.14 3.15 3.33]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_loop_condition_runtime_produces_expected_sequence() {
        let output = run_program_output(
            r#"(do
                (let push! (lambda xs x (do (set! xs (length xs) x) xs)))
                (let xs [])
                (let i [0])
                (while (< (get i 0) 10) (do (push! xs (get i 0)) (set! i 0 (+ (get i 0) 1))))
                xs)"#
        );
        assert_eq!(output, "[0 1 2 3 4 5 6 7 8 9]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_regression_graph_build_two_calls_preserves_first_result() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
        (let std/vector/empty? (lambda xs (= (length xs) 0)))
            (let std/vector/for (lambda xs fn (do
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do (fn (get xs i)) (alter! i (+ i 1)))))))
            (let std/vector/map (lambda xs fn (if (std/vector/empty? xs) [] (do
     (let out [(fn (get xs 0))])
     (mut i 1)
     (while (< i (length xs)) (do (set! out (length out) (fn (get xs i))) (alter! i (+ i 1))))
     out))))
      (let std/vector/int/zeroes (lambda n (do
     (let out [ 0 ])
     (let process (lambda i (set! out (length out) 0)))
     (loop 1 n process)
     out)))
     (let std/vector/push! (lambda xs x (do (set! xs (length xs) x) nil)))

                (let valid-path (lambda n edges source destination (do
                  (let graph (std/vector/map (std/vector/int/zeroes n) (lambda _ [])))
                  (std/vector/for edges (lambda edge (do
                    (let u (get edge 0))
                    (let v (get edge 1))
                    (std/vector/push! (get graph u) v)
                    (std/vector/push! (get graph v) u))))
                  graph)))

                [(valid-path 3 [[ 0 1 ] [ 1 2 ] [ 2 0 ]] 0 2)
                 (valid-path 6 [[ 0 1 ] [ 0 2 ] [ 3 5 ] [ 5 4 ] [ 4 3 ]] 0 5)])"#,
            true
        );
        assert_eq!(output, "[[[1 2] [0 2] [1 0]] [[1 2] [0] [0] [5 4] [5 3] [3 4]]]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness_fusion_opt_equivalence_smoke() {
        assert_std_program_output_matches_with_and_without_optimizer(
            r#"(do
  (let z (|> (zip (pair (range 1 5) (map odd? (range 1 5))))
             (map (lambda t (tuple (+ (fst t) 1) (snd t))))
             (filter (lambda t (> (fst t) 2)))
             unzip))
  [(sum (fst z)) (length (snd z)) (|> (window 2 [1 2 3 4]) (map length) sum)])"#
        )
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness_fusion_opt_equivalence_smoke2() {
        assert_std_program_output_matches_with_and_without_optimizer(
            r#"(|> 
  (range 1 10)
  (filter even?) 
  (map square)
  (map/i (lambda x i { x i }))
  unzip
  zip
  (map (lambda { a b } (* a b)))
  sum)"#
        )
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness_fusion_opt_equivalence_smoke3() {
        assert_std_program_output_matches_with_and_without_optimizer(
            r#"(|> (range 1 (|> (range 1 20) (map (lambda x (+ x 1))) (find (lambda x (> x 15))))) (map odd?) (every? (Bool/eq? true)))"#
        )
    }
    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness_fusion_opt_equivalence_smoke4() {
        assert_std_program_output_matches_with_and_without_optimizer(
            r#"
            (let a (|> (range 1 5) (map/i (lambda x i { x i }))))
(let b (map/i (lambda x i { x i }) [1 2 3 4 5]))
[a b]"#
        )
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_oversaturated_returned_function_call_matches_explicit_apply() {
        let src =
            r#"(do
  (let make-adder (lambda x (lambda y (+ x y))))
  [
    (apply (make-adder 2) 3)
    (make-adder 2 3)
  ])"#;
        assert_std_program_output_matches_with_and_without_optimizer(src);
        let output = run_program_output_with_std_and_opts(src, true);
        assert_eq!(output, "[5 5]");
    }

    #[test]
    #[ignore = "known remaining issue: returned closures over nested higher-order partials still trap in wasm backend"]
    #[cfg(feature = "runtime")]
    fn test_mapcar_comp_works_with_nested_and_apply_call_forms() {
        let src =
            r#"(do
  (let mymap (lambda fn xs (if (= (length xs) 0) [] (do
    (let out [])
    (mut i 0)
    (while (< i (length xs)) (do
      (set! out (length out) (fn (get xs i)))
      (alter! i (+ i 1))))
    out))))
  (let mapcar (lambda fn (comp (mymap (mymap fn)))))
  [
    ((mapcar square) [[1 2 3] [2 4 5 6] [3]])
    (apply (mapcar square) [[1 2 3] [2 4 5 6] [3]])
  ])"#;
        assert_std_program_output_matches_with_and_without_optimizer(src);
        let output = run_program_output_with_std_and_opts(src, true);
        assert_eq!(output, "[[[1 4 9] [4 16 25 36] [9]] [[1 4 9] [4 16 25 36] [9]]]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness_fusion_opt_equivalence_reduce_until_i() {
        assert_std_program_output_matches_with_and_without_optimizer(
            r#"(let two-sum-test (lambda nums target (snd (|> nums 
            (filter (lambda x (< x 1000))) 
            (filter/i (lambda x i (and (< x 100) (< i 10)))) 
            (map identity) (reduce/until/i (lambda { a out } b i (do
        (let check (Integer->String (- target b)))
        (if (Table/has? check a) (do 
            (push! out i)  (push! out (snd (get (Table/get check a))))))
        (let key (Integer->String b))
        (Table/set! a key i)
        { a out }))
        (lambda { _ out } _ _ (not-empty? out))
        { (Table/new) [] } )))))
[
    (two-sum-test [ 2 7 11 15 ] 9)
    (two-sum-test [3 2 4] 6)
    (two-sum-test [ 2 7 11 15 ] 9)
]"#
        )
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness_fusion_opt_equivalence_reduce_until() {
        assert_std_program_output_matches_with_and_without_optimizer(
            r#"(|> [1 2 3 4 5 6]
    (filter odd?)
    (map (lambda x (* x 2)))
    (reduce/until
        (lambda a x (+ a x))
        (lambda a x (> (+ a x) 7))
        0))"#
        )
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness_fusion_opt_equivalence_template() {
        assert_std_program_output_matches_with_and_without_optimizer(
            r#"(|> (range 1 10) (map (lambda x (+ x 1))) (filter (lambda x (> x 3))) (reduce + 0))"#
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness_fusion_opt_equivalence_template6() {
        assert_std_program_output_matches_with_and_without_optimizer(
            r#"(|> (range 1 10) (map (lambda x (+ x 1))) (window 3) (map sum) (filter even?) sum)"#
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_regression_loop_callback_discard_does_not_overrelease_borrowed_result() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let push! (lambda xs x (do (set! xs (length xs) x) xs)))
                (let sort-array-by-parity2 (lambda nums (do
                  (let odd [])
                  (let even [])
                  (let out [])
                  (loop 0 (length nums) (lambda i (push! (if (= (mod i 2) 0) even odd) (get nums i))))
                  (loop 0 (length even) (lambda i (do (push! out (get even i)) (push! out (get odd i)))))
                  out)))
                [(sort-array-by-parity2 [4 2 5 7])
                 (sort-array-by-parity2 [2 3])
                 (sort-array-by-parity2 [4 3])])"#,
            true
        );
        assert_eq!(output, "[[4 2 5 7] [2 3] [4 3]]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_regression_std_vector_for_isolated_behavior() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let std/vector/for (lambda xs fn (loop 0 (length xs) (lambda i (fn (get xs i))))))
                (let out [])
                (std/vector/for [1 2 3 4] (lambda x (set! out (length out) (* x 2))))
                out)"#,
            true
        );
        assert_eq!(output, "[2 4 6 8]");
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_regression_std_vector_3d_rotate_isolated_behavior() {
        let output = run_program_output_with_std_and_opts(
            r#"(do
                (let std/vector/empty? (lambda xs (= (length xs) 0)))
                (let std/vector/push! (lambda xs x (do (set! xs (length xs) x) nil)))
                (let std/vector/at (lambda xs i (if (< i 0) (get xs (+ (length xs) i)) (get xs i))))
                (let std/vector/3d/rotate (lambda matrix (if (std/vector/empty? matrix) matrix (do 
                    (let H (length matrix))
                    (let W (length (get matrix 0)))
                    (let out [])
                    (loop 0 W (lambda i (do
                        (std/vector/push! out [])
                        (loop 0 H (lambda j 
                            (std/vector/push! (std/vector/at out -1) (get matrix j i)))))))
                    out))))
                (std/vector/3d/rotate [[1 2 3] [4 5 6]]))"#,
            true
        );
        assert_eq!(output, "[[1 4] [2 5] [3 6]]");
    }

    fn infer_typed(input: &str) -> crate::infer::TypedExpression {
        let exprs = crate::parser::parse(input).expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let (_typ, typed) = crate::infer
            ::infer_with_builtins_typed(
                expr,
                crate::types::create_builtin_environment(crate::types::TypeEnv::new())
            )
            .expect("input should infer");
        typed
    }

    #[test]
    fn test_typed_optimization_constant_folds_nested_int_ops() {
        let typed = infer_typed("(+ 2 (* 3 4))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "14");
    }

    #[test]
    fn test_typed_optimization_constant_folds_if_branch() {
        let typed = infer_typed("(if true (+ 1 2) (+ 100 200))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "3");
    }

    #[test]
    fn test_typed_optimization_if_with_same_branches_drops_pure_condition() {
        let typed = infer_typed("(if (= 1 1) (+ 1 2) (+ 1 2))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "3");
    }

    #[test]
    fn test_typed_optimization_if_with_same_branches_keeps_impure_condition_eval() {
        let typed = infer_typed("(if (do (print! (vector)) true) 1 1)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();
        assert!(
            optimized_lisp.contains("(print! (vector))"),
            "impure condition should still be evaluated, got: {}",
            optimized_lisp
        );
        assert!(
            !optimized_lisp.contains("(if "),
            "if with equal branches should rewrite away, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_keeps_div_by_zero_unfolded() {
        let typed = infer_typed("(/ 4 0)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "(/ 4 0)");
    }

    #[test]
    fn test_typed_optimization_simplifies_add_zero_identity() {
        let typed = infer_typed("(lambda x (+ x 0))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "(lambda x x)");
    }

    #[test]
    fn test_typed_optimization_simplifies_mul_one_identity() {
        let typed = infer_typed("(lambda x (* 1 x))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "(lambda x x)");
    }

    #[test]
    fn test_typed_optimization_do_cleanup_drops_non_last_literals_and_collapses() {
        let typed = infer_typed("(do 1 2 (+ 3 4))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "7");
    }

    #[test]
    fn test_typed_optimization_do_cleanup_keeps_non_literal_middle_expr() {
        let typed = infer_typed("(lambda x (do 1 x 2))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "(lambda x (do x 2))");
    }

    #[test]
    fn test_typed_optimization_do_cleanup_drops_safe_pure_call_statement() {
        let typed = infer_typed("(do (= 1 1) 7)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "7");
    }

    #[test]
    fn test_typed_optimization_do_cleanup_keeps_impure_call_statement() {
        let typed = infer_typed("(do (print! (vector)) 7)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();
        assert!(
            optimized_lisp.contains("(print! (vector))"),
            "impure call statement should not be dropped, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_and_rhs_true_reduces_to_lhs() {
        let typed = infer_typed("(lambda x (and x true))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "(lambda x x)");
    }

    #[test]
    fn test_typed_optimization_or_rhs_false_reduces_to_lhs() {
        let typed = infer_typed("(lambda x (or x false))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "(lambda x x)");
    }

    #[test]
    fn test_typed_optimization_and_rhs_false_keeps_impure_lhs_eval() {
        let typed = infer_typed("(and (do (print! (vector)) true) false)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();
        assert!(
            optimized_lisp.contains("(print! (vector))"),
            "impure lhs should still be evaluated, got: {}",
            optimized_lisp
        );
        assert!(
            optimized_lisp.ends_with(" false)"),
            "expected constant false result after evaluation, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_or_rhs_true_keeps_impure_lhs_eval() {
        let typed = infer_typed("(or (do (print! (vector)) false) true)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();
        assert!(
            optimized_lisp.contains("(print! (vector))"),
            "impure lhs should still be evaluated, got: {}",
            optimized_lisp
        );
        assert!(
            optimized_lisp.ends_with(" true)"),
            "expected constant true result after evaluation, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_inline_avoids_duplicate_vector_eval() {
        let typed = infer_typed(
            "(do (let f (lambda x (+ (get x 0) (get x 0)))) (f (vector 1 2 3)))"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();

        assert!(
            !optimized_lisp.contains("(f "),
            "expected direct call to be inlined, got: {}",
            optimized_lisp
        );
        assert_eq!(
            optimized_lisp.matches("(vector 1 2 3)").count(),
            1,
            "inlining should evaluate vector argument once, got: {}",
            optimized_lisp
        );
        assert!(
            optimized_lisp.matches("get __inline_arg_").count() >= 2,
            "inlined body should reuse temp arg binding, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_inline_skips_large_lambda_body() {
        let typed = infer_typed(
            "(do (let f (lambda x (+ (+ (+ (+ (+ (+ (+ (+ x 1) 2) 3) 4) 5) 6) 7) 8))) (f 1))"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();

        assert!(
            optimized_lisp.contains("(f 1)"),
            "large lambda body should not inline, got: {}",
            optimized_lisp
        );
        assert!(
            !optimized_lisp.contains("__inline_arg_"),
            "no temp vars should be introduced when inlining is skipped, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_inline_fixpoint_inlines_let_rhs_calls() {
        let typed = infer_typed(
            "(do (let add (lambda a b (+ a b))) (let sub (lambda a b (- a b))) (sub (add 1323 22) (add 4222 122)))"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();

        assert!(
            !optimized_lisp.contains("(sub "),
            "expected sub call to be inlined, got: {}",
            optimized_lisp
        );
        assert!(
            !optimized_lisp.contains("(add "),
            "expected add calls in temp let RHS to be inlined, got: {}",
            optimized_lisp
        );
        assert!(
            !optimized_lisp.contains("__inline_arg_"),
            "single-use inline temp lets should be eliminated, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_post_inline_constant_folds_to_single_literal() {
        let typed = infer_typed(
            "(do (let add (lambda a b (+ a b))) (let sub (lambda a b (- a b))) (sub (add 1323 22) (add 4222 122)))"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();
        assert!(
            optimized_lisp.ends_with("-2999)"),
            "final expression should be folded to literal after inlining, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_nested_calls_fold_after_inline() {
        let typed = infer_typed(
            "(do (let add (lambda a b (+ a b))) (let sub (lambda a b (- a b))) (* (sub (add 1323 22) (add 4222 122)) 25))"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();

        assert!(
            optimized_lisp.ends_with("-74975)"),
            "nested call expression should fold after inline, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_nested_inline_skips_managed_vector_args() {
        let typed = infer_typed("(do (let f (lambda xs (length xs))) (+ (f (vector 1 2 3)) 1))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();

        assert!(
            optimized_lisp.contains("(f (vector 1 2 3))"),
            "nested no-temp inline should skip managed vector args, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_eliminates_single_use_literal_let_binding() {
        let typed = infer_typed("(do (let res -74975) res)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "-74975");
    }

    #[test]
    fn test_infer_impure_function_requires_bang_suffix() {
        let exprs = crate::parser
            ::parse("(let fn (lambda xs (set! xs 0 1)))")
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        let err = inferred.expect_err("impure function without ! should fail");
        assert!(
            err.contains("Impure function 'fn' must end with '!'"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_infer_impure_function_alias_requires_bang_suffix() {
        let exprs = crate::parser
            ::parse("(do (let reverse! (lambda xs (set! xs 0 1))) (let reverse reverse!) reverse)")
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        let err = inferred.expect_err("impure function alias without ! should fail");
        assert!(
            err.contains("Impure function 'reverse' must end with '!'"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_infer_impure_wrapper_call_requires_bang_suffix() {
        let exprs = crate::parser
            ::parse(
                "(do (let reverse! (lambda xs (set! xs 0 1))) (let wrap (lambda xs (reverse! xs))) wrap)"
            )
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        let err = inferred.expect_err("impure wrapper without ! should fail");
        assert!(
            err.contains("Impure function 'wrap' must end with '!'"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_infer_impure_function_non_unit_return_allowed_by_default() {
        let exprs = crate::parser
            ::parse("(let append-ten! (lambda xs (do (set! xs 0 1) xs)))")
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        assert!(
            inferred.is_ok(),
            "non-unit impure return should be allowed by default, got: {:?}",
            inferred
        );
    }

    #[test]
    fn test_infer_impure_hyphenated_function_requires_bang_suffix() {
        let exprs = crate::parser
            ::parse("(let append-ten (lambda xs (set! xs (length xs) 10)))")
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        let err = inferred.expect_err("impure hyphenated function without ! should fail");
        assert!(
            err.contains("Impure function 'append-ten' must end with '!'"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_infer_impure_function_mutation_target_must_be_first_param() {
        let exprs = crate::parser
            ::parse("(let set-middle! (lambda a b c (set! b 0 1)))")
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        let err = inferred.expect_err("impure function mutating middle param should fail");
        assert!(err.contains("must mutate its first parameter"), "unexpected error: {}", err);
    }

    #[test]
    fn test_infer_impure_function_mutating_first_param_is_allowed() {
        let exprs = crate::parser
            ::parse("(let sort! (lambda xs fn (set! xs 0 1)))")
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        assert!(
            inferred.is_ok(),
            "impure function mutating arg1 should be allowed, got: {:?}",
            inferred
        );
    }

    #[test]
    fn test_infer_impure_function_mutating_last_param_is_rejected() {
        let exprs = crate::parser
            ::parse("(let sort! (lambda fn xs (set! xs 0 1)))")
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        let err = inferred.expect_err("impure function mutating last param should fail");
        assert!(err.contains("must mutate its first parameter"), "unexpected error: {}", err);
    }

    #[test]
    fn test_infer_impure_nested_mutation_target_rooted_in_first_param_is_allowed() {
        let exprs = crate::parser
            ::parse(
                "(let std/vector/3d/set! (lambda matrix y x value (do (set! (get matrix y) x value) 0)))"
            )
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        assert!(
            inferred.is_ok(),
            "nested target rooted in first param should be allowed, got: {:?}",
            inferred
        );
    }

    #[test]
    fn test_infer_impure_operator_aliases_and_set_are_exempt_from_bang_suffix() {
        let exprs = crate::parser
            ::parse(
                "(do
                    (let update-set! (lambda vrbl x (set! vrbl 0 x)))
                    (let boolean-set! (lambda vrbl x (set! vrbl 0 x)))
                    1)"
            )
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        assert!(
            inferred.is_ok(),
            "set aliases ending with ! should stay exempt from ! suffix, got: {:?}",
            inferred
        );
    }

    #[test]
    fn test_runtime_reclaimed_operatorish_names_compile_as_normal_bindings() {
        let program =
            r#"(do
  (let ++ (lambda x (+ x 1)))
  (let -- (lambda x (- x 1)))
  (let += (lambda a b (+ a b)))
  (let -= (lambda a b (- a b)))
  (let +=. (lambda a b (+. a b)))
  (let *=. (lambda a b (*. a b)))
  (let ++. (lambda x (+. x 1.0)))
  (and
    (= (++ 1) 2)
    (= (-- 5) 4)
    (= (+= 2 3) 5)
    (= (-= 7 4) 3)
    (= (Dec->Int (+=. 2.5 0.5)) 3)
    (= (Dec->Int (*=. 1.5 2.0)) 3)
    (= (Dec->Int (++. 3.0)) 4)))"#;
        let output = run_program_output(program);
        assert_eq!(output.trim(), "true");
    }

    #[test]
    fn test_infer_local_nested_set_target_does_not_require_bang() {
        let exprs = crate::parser
            ::parse(
                "(do
                    (let at (lambda xs i (get xs i)))
                    (let partition (lambda xs n (do
                        (let a (vector (vector 0)))
                        (set! (at a 0) 0 1)
                        a)))
                    partition)"
            )
            .expect("input should parse");
        let expr = exprs.first().expect("input should contain one expression");
        let inferred = crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        assert!(
            inferred.is_ok(),
            "local nested set! target should not require ! suffix, got: {:?}",
            inferred
        );
    }

    #[test]
    fn test_typed_optimization_beta_reduces_apply_alias_lambda_call() {
        let typed = infer_typed("(do ((lambda x (+ x 1)) 41))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "42");
    }

    #[test]
    fn test_typed_optimization_apply_alias_beta_reduce_skips_managed_args() {
        let typed = infer_typed("(do ((lambda x (+ (get x 0) (get x 1))) (vector 10 20)))");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert_eq!(optimized.expr.to_lisp(), "((lambda x (+ (get x 0) (get x 1))) (vector 10 20))");
    }

    #[test]
    fn test_typed_optimization_inline_does_not_drop_impure_unused_arg() {
        let typed = infer_typed("(do (let f (lambda x 1)) (f (print! (vector))) 0)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();
        assert!(
            optimized_lisp.contains("(print! (vector))"),
            "inlining should preserve eager evaluation of impure args, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_dce_keeps_unused_impure_top_level_definition() {
        let typed = infer_typed("(do (let side (print! (vector))) 1)");
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();
        assert!(
            optimized_lisp.contains("(let side (print! (vector)))"),
            "unused impure top-level definitions must not be removed, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_dce_keeps_dependencies_of_kept_impure_definition() {
        let typed = infer_typed(
            "(do (let helper (lambda x x)) (let side (print! (helper (vector)))) 1)"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();
        assert!(
            optimized_lisp.contains("(let helper (lambda x x))"),
            "DCE must keep deps of retained impure defs, got: {}",
            optimized_lisp
        );
        assert!(
            optimized_lisp.contains("(let side (print! (helper (vector))))"),
            "retained impure def should still be present, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_map_short_name_fuses_to_direct_loop() {
        let typed = infer_typed(
            "(do (let std/vector/map (lambda xs fn (fn (get xs 0)))) (let map (lambda fn xs (std/vector/map xs fn))) (map (lambda x (+ x 1)) (vector 41)))"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();

        assert!(
            !optimized_lisp.contains("(map "),
            "short map call should be inlined, got: {}",
            optimized_lisp
        );
        assert!(
            optimized_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "map chain should lower to one loop, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_filter_short_name_fuses_to_direct_loop() {
        let typed = infer_typed(
            "(do (let std/vector/filter (lambda xs fn (fn (get xs 0)))) (let filter (lambda fn xs (std/vector/filter xs fn))) (filter (lambda x (> x 0)) (vector 41)))"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();

        assert!(
            !optimized_lisp.contains("(filter "),
            "short filter call should be inlined, got: {}",
            optimized_lisp
        );
        assert!(
            optimized_lisp.contains("(> (get __fuse_xs __fuse_i) 0)"),
            "filter predicate should be preserved in optimized expression, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_reduce_short_name_fuses_to_direct_loop() {
        let typed = infer_typed(
            "(do (let std/vector/reduce (lambda xs fn init (fn init (get xs 0)))) (let reduce (lambda fn init xs (std/vector/reduce xs fn init))) (reduce (lambda a x (+ a x)) 10 (vector 32)))"
        );
        let optimized = crate::op::optimize_typed_ast(&typed);
        let optimized_lisp = optimized.expr.to_lisp();

        assert!(
            !optimized_lisp.contains("(reduce (lambda a x (+ a x)) 10 (vector 32))"),
            "reduce call should be lowered into direct loop, got: {}",
            optimized_lisp
        );
        assert!(
            optimized_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "reduce should lower to one loop, got: {}",
            optimized_lisp
        );
    }

    #[test]
    fn test_typed_optimization_map_map_map_chain_fuses_to_single_reduce_loop() {
        let expr = crate::parser
            ::parse(
                "(map (lambda x (+ x 1))
                (map (lambda x (+ x 2))
                    (map (lambda x (+ x 3)) (vector 1 2 3))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused = crate::op::fuse_map_filter_reduce_for_test(&expr);
        let fused_lisp = fused.to_lisp();

        assert!(
            fused_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "map-only chain should fuse to a single loop, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(map (lambda"),
            "fused expression should not keep map calls in pipeline, got: {}",
            fused_lisp
        );
        assert_eq!(
            fused_lisp.matches("(while ").count(),
            1,
            "map-only chain should lower to exactly one loop, got: {}",
            fused_lisp
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_typed_optimization_map_partial_application_fuses_by_hoisting_callable_once() {
        let expr = crate::parser
            ::parse("(map (add 1) (range 0 10))")
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(
            fused_lisp.contains("(let __fuse_callable_0"),
            "inline partial callable should be hoisted once before fused loop, got: {}",
            fused_lisp
        );
        assert!(
            fused_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "map partial application should still fuse to a direct loop, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(map (add 1)"),
            "fused expression should not keep original map call, got: {}",
            fused_lisp
        );

        assert_std_program_output_matches_with_and_without_optimizer(
            "(|> (range 0 10) (map (add 1)))"
        );
    }

    #[test]
    fn test_typed_optimization_map_filter_filter_map_map_reduce_chain_fuses_to_single_reduce_loop() {
        let expr = crate::parser
            ::parse(
                "(reduce
                (lambda a x (+ a x))
                0
                (map (lambda x (+ x 1))
                    (map (lambda x (+ x 2))
                        (filter (lambda x (> x 1))
                                (filter (lambda x (> x 0))
                                        (map (lambda x (+ x 3)) (vector 1 2 3)))))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused = crate::op::fuse_map_filter_reduce_for_test(&expr);
        let fused_lisp = fused.to_lisp();

        assert!(
            fused_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "map/filter/reduce chain should fuse into one loop, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(map (lambda"),
            "fused expression should not keep map calls in pipeline, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(filter (lambda"),
            "fused expression should not keep filter calls in pipeline, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(reduce "),
            "fused expression should not keep short reduce call, got: {}",
            fused_lisp
        );
        assert_eq!(
            fused_lisp.matches("(while ").count(),
            1,
            "map/filter/reduce chain should lower to exactly one loop, got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_select_exclude_sum_range_fuses_with_whitelist_names() {
        let expr = crate::parser
            ::parse(
                "(sum (map (lambda x (+ x 1))
                  (select (lambda x (> x 2))
                    (exclude (lambda x (= x 5))
                      (range 1 10)))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused = crate::op::fuse_map_filter_reduce_for_test(&expr);
        let fused_lisp = fused.to_lisp();

        assert!(
            fused_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "range source should fuse into a direct numeric loop, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(sum "),
            "sum should be absorbed into loop sink, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(select "),
            "select stage should be absorbed into loop guard, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(exclude "),
            "exclude stage should be absorbed into loop guard, got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_mean_aliases_fuse_to_single_pass_mean_loop() {
        let int_expr = crate::parser
            ::parse("(mean (map (lambda x (+ x 1)) (filter (lambda x (> x 2)) (range 1 10))))")
            .expect("mean input should parse")
            .remove(0);
        let float_expr = crate::parser
            ::parse(
                "(mean/dec (map (lambda x (+. x 1.0)) (filter (lambda x (>. x 2.0)) (range/dec 1 10))))"
            )
            .expect("mean/dec input should parse")
            .remove(0);
        let avg_expr = crate::parser::parse("(avg 2 4)").expect("avg input should parse").remove(0);

        let int_fused = crate::op::fuse_map_filter_reduce_for_test(&int_expr).to_lisp();
        let float_fused = crate::op::fuse_map_filter_reduce_for_test(&float_expr).to_lisp();
        let avg_fused = crate::op::fuse_map_filter_reduce_for_test(&avg_expr).to_lisp();

        assert!(
            int_fused.contains("(while (< __fuse_i __fuse_i_end)"),
            "mean should fuse to one direct loop, got: {}",
            int_fused
        );
        assert!(
            !int_fused.contains("(mean "),
            "mean call should be fused away, got: {}",
            int_fused
        );
        assert!(
            float_fused.contains("(/. (get __fuse_sum 0) (Int->Dec (get __fuse_count 0)))"),
            "mean/dec should emit dec division by Int->Dec(count), got: {}",
            float_fused
        );
        assert!(
            !float_fused.contains("(mean/dec "),
            "mean/dec call should be fused away, got: {}",
            float_fused
        );
        assert_eq!(avg_fused, "(avg 2 4)", "avg should remain binary and unfused");
    }

    #[test]
    fn test_typed_optimization_window_source_fuses_with_pipeline_sinks() {
        let expr = crate::parser
            ::parse(
                "(reduce + 0 (map length (window 2 (vector (vector 1) (vector 2) (vector 3)))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(
            fused_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "window source should fuse to bounded loop, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(window "),
            "window call should be lowered to source bindings, got: {}",
            fused_lisp
        );
        assert!(
            fused_lisp.contains("(slice __fuse_i (+ __fuse_i __fuse_window_size) __fuse_xs)"),
            "window fusion should derive values via slice(i, i+size, xs), got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_zip_map_filter_unzip_is_not_fused() {
        let expr = crate::parser
            ::parse(
                "(unzip (filter (lambda t (> (fst t) 2))
                    (map (lambda t (tuple (+ (fst t) 1) (snd t)))
                        (zip (tuple (vector 1 2 3 4) (vector true false true false))))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(fused_lisp.contains("(zip "), "zip fusion should be disabled, got: {}", fused_lisp);
        assert!(
            fused_lisp.contains("(unzip "),
            "unzip fusion should be disabled, got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_zip_pair_form_is_not_fused() {
        let expr = crate::parser
            ::parse(
                "(unzip (map (lambda t t)
                    (zip (pair (vector 1 2 3) (vector true false true)))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(
            fused_lisp.contains("(zip "),
            "zip(pair ..) fusion should be disabled, got: {}",
            fused_lisp
        );
        assert!(
            fused_lisp.contains("(unzip "),
            "unzip fusion should be disabled, got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_zip_map_collect_is_not_fused_inline() {
        let expr = crate::parser
            ::parse("(map (lambda t t) (zip (tuple (vector 1 2 3) (vector true false true))))")
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(
            fused_lisp.contains("(zip "),
            "zip->map collect should not fuse inline until the zip-source collect bug is fixed, got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_some_and_every_fuse_to_short_circuit_loops() {
        let some_expr = crate::parser
            ::parse(
                "(some? (lambda x (> x 20))
               (map (lambda x (+ x 1)) (range 1 10)))"
            )
            .expect("some input should parse")
            .remove(0);
        let every_expr = crate::parser
            ::parse(
                "(every? (lambda x (> x 0))
                (filter (lambda x (> x 1)) (range 1 10)))"
            )
            .expect("every input should parse")
            .remove(0);

        let some_fused = crate::op::fuse_map_filter_reduce_for_test(&some_expr).to_lisp();
        let every_fused = crate::op::fuse_map_filter_reduce_for_test(&every_expr).to_lisp();

        assert!(
            some_fused.contains("(while"),
            "some? should lower to short-circuit while, got: {}",
            some_fused
        );
        assert!(
            every_fused.contains("(while"),
            "every? should lower to short-circuit while, got: {}",
            every_fused
        );
        assert!(!some_fused.contains("(some? "), "some? call should be fused, got: {}", some_fused);
        assert!(
            !every_fused.contains("(every? "),
            "every? call should be fused, got: {}",
            every_fused
        );
    }

    #[test]
    fn test_typed_optimization_indexed_variants_fuse_with_whitelist() {
        let expr = crate::parser
            ::parse(
                "(reduce/i (lambda a x i (+ a (+ x i))) 0
                (map/i (lambda x i (+ x i))
                    (filter/i (lambda x i (> (+ x i) 3))
                        (range/int 1 8))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(
            fused_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "indexed variants over range/int should fuse to direct loop, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(reduce/i "),
            "reduce/i should be fused, got: {}",
            fused_lisp
        );
        assert!(!fused_lisp.contains("(map/i "), "map/i should be fused, got: {}", fused_lisp);
        assert!(
            !fused_lisp.contains("(filter/i "),
            "filter/i should be fused, got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_some_i_every_i_and_range_float_fuse() {
        let some_i = crate::parser
            ::parse("(some/i? (lambda x i (> (+ x i) 10)) (range/dec 1 6))")
            .expect("input should parse")
            .remove(0);
        let every_i = crate::parser
            ::parse("(every/i? (lambda x i (> (+ x i) 0)) (range/int 1 6))")
            .expect("input should parse")
            .remove(0);
        let fused_some = crate::op::fuse_map_filter_reduce_for_test(&some_i).to_lisp();
        let fused_every = crate::op::fuse_map_filter_reduce_for_test(&every_i).to_lisp();

        assert!(
            fused_some.contains("(while"),
            "some/i? should short-circuit fuse, got: {}",
            fused_some
        );
        assert!(
            fused_every.contains("(while"),
            "every/i? should short-circuit fuse, got: {}",
            fused_every
        );
        assert!(!fused_some.contains("(some/i? "), "some/i? should be fused, got: {}", fused_some);
        assert!(
            !fused_every.contains("(every/i? "),
            "every/i? should be fused, got: {}",
            fused_every
        );
        assert!(
            fused_some.contains("(Int->Dec"),
            "range/dec fusion should convert loop index to dec value, got: {}",
            fused_some
        );
    }

    #[test]
    fn test_typed_optimization_slice_source_fuses_with_map_filter_reduce() {
        let expr = crate::parser
            ::parse(
                "(reduce + 0 (map (lambda x (* x x)) (filter even? (slice 1 6 (vector 1 2 3 4 5 6 7)))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(
            fused_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
            "slice source should fuse to start/end bounded loop, got: {}",
            fused_lisp
        );
        assert!(
            !fused_lisp.contains("(slice "),
            "slice call should be eliminated by fusion source lowering, got: {}",
            fused_lisp
        );
        assert!(!fused_lisp.contains("(reduce "), "reduce should be fused, got: {}", fused_lisp);
    }

    #[test]
    fn test_typed_optimization_find_fuses_over_filtered_stream() {
        let expr = crate::parser
            ::parse(
                "(find (lambda x (= x 16)) (map (lambda x (* x x)) (filter even? (vector 1 2 3 4 5 6))))"
            )
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(
            fused_lisp.contains("(while"),
            "find should fuse to short-circuit while, got: {}",
            fused_lisp
        );
        assert!(!fused_lisp.contains("(find "), "find call should be fused, got: {}", fused_lisp);
        assert!(
            fused_lisp.contains("(set! __fuse_out 0"),
            "find fusion should update output index inside fused loop, got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_take_drop_aliases_fuse_as_slice_sources() {
        let programs = [
            "(reduce + 0 (map (lambda x x) (take/first 3 (vector 1 2 3 4 5))))",
            "(reduce + 0 (map (lambda x x) (drop/first 2 (vector 1 2 3 4 5))))",
            "(reduce + 0 (map (lambda x x) (take/last 2 (vector 1 2 3 4 5))))",
            "(reduce + 0 (map (lambda x x) (drop/last 2 (vector 1 2 3 4 5))))",
        ];

        for program in programs {
            let expr = crate::parser::parse(program).expect("input should parse").remove(0);
            let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();
            assert!(
                fused_lisp.contains("(while (< __fuse_i __fuse_i_end)"),
                "take/drop alias source should fuse via slice-style bounded loop, got: {}",
                fused_lisp
            );
            assert!(
                !fused_lisp.contains("(reduce "),
                "reduce should be fused, got: {}",
                fused_lisp
            );
        }
    }

    #[test]
    fn test_typed_optimization_flat_stage_fuses_as_one_level_nested_loops() {
        let expr = crate::parser
            ::parse("(flat (map (lambda x (vector x x)) (vector 1 2 3)))")
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(!fused_lisp.contains("(flat "), "flat should be fused away, got: {}", fused_lisp);
        assert!(
            fused_lisp.matches("(while ").count() >= 2,
            "flat fusion should emit nested loops (outer + one-level inner), got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_typed_optimization_flat_map_reduce_fuses_as_one_level_nested_loops() {
        let expr = crate::parser
            ::parse("(reduce + 0 (flat-map (lambda x (vector x x)) (vector 1 2 3)))")
            .expect("input should parse")
            .remove(0);
        let fused_lisp = crate::op::fuse_map_filter_reduce_for_test(&expr).to_lisp();

        assert!(
            !fused_lisp.contains("(flat-map "),
            "flat-map should be fused away, got: {}",
            fused_lisp
        );
        assert!(!fused_lisp.contains("(reduce "), "reduce should be fused, got: {}", fused_lisp);
        assert!(
            fused_lisp.matches("(while ").count() >= 2,
            "flat-map reduce fusion should emit nested loops, got: {}",
            fused_lisp
        );
    }

    #[test]
    fn test_wat_pipeline_map_filter_reduce_fuses_to_single_reduce_loop_in_codegen_path() {
        let program = "(|> [ 1 2 3 4 5 ] (filter even?) (map square) (reduce + 0))";
        let std_ast = crate::baked::load_ast();
        let wrapped = match std_ast {
            crate::parser::Expression::Apply(items) => {
                crate::parser
                    ::merge_std_and_program(program, items[1..].to_vec())
                    .expect("program should merge with std")
            }
            _ => panic!("std ast should be (do ...)"),
        };

        let wat = crate::wat
            ::compile_program_to_wat(&wrapped)
            .expect("wat compilation should succeed");
        let main_start = wat.find("(func (export \"main\")").expect("main export should exist");
        let main_wat = &wat[main_start..];

        assert!(
            !main_wat.contains("call $v_filter"),
            "main should not call v_filter after fusion, got:\n{}",
            main_wat
        );
        assert!(
            !main_wat.contains("call $v_map"),
            "main should not call v_map after fusion, got:\n{}",
            main_wat
        );
        assert!(
            !main_wat.contains("call $v_reduce"),
            "main should not call v_reduce after direct loop fusion, got:\n{}",
            main_wat
        );
        assert!(
            main_wat.contains("\n    block\n      loop\n"),
            "main should contain a single lowered loop after fusion, got:\n{}",
            main_wat
        );
    }

    #[test]
    fn test_wat_pipeline_with_wrapper_barrier_still_segment_fuses() {
        let program =
            "(do (let mymap (lambda fn xs (map square xs))) (|> (range 1 10) (filter even?) (mymap square) (map square) (reduce + 0)))";
        let std_ast = crate::baked::load_ast();
        let wrapped = match std_ast {
            crate::parser::Expression::Apply(items) => {
                crate::parser
                    ::merge_std_and_program(program, items[1..].to_vec())
                    .expect("program should merge with std")
            }
            _ => panic!("std ast should be (do ...)"),
        };

        let fused_wrapped = match &wrapped {
            crate::parser::Expression::Apply(items) if
                matches!(items.first(), Some(crate::parser::Expression::Word(w)) if w == "do") &&
                items.len() > 1
            => {
                let mut out = items.clone();
                let last = out.len() - 1;
                out[last] = crate::op::fuse_map_filter_reduce_for_test(&out[last]);
                crate::parser::Expression::Apply(out)
            }
            _ => crate::op::fuse_map_filter_reduce_for_test(&wrapped),
        };
        let reinfer = crate::infer::infer_with_builtins_typed(
            &fused_wrapped,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        assert!(
            reinfer.is_ok(),
            "segmented fused wrapper case must re-infer; got: {}",
            reinfer.err().unwrap_or_else(|| "unknown error".to_string())
        );

        let wat = crate::wat
            ::compile_program_to_wat(&wrapped)
            .expect("wat compilation should succeed");
        let main_start = wat.find("(func (export \"main\")").expect("main export should exist");
        let main_wat = &wat[main_start..];

        assert!(
            !main_wat.contains("call $v_filter"),
            "main should not call v_filter when left segment fuses, got:\n{}",
            main_wat
        );
        assert!(
            !main_wat.contains("call $v_reduce"),
            "main should not call v_reduce when right segment fuses, got:\n{}",
            main_wat
        );
        assert!(
            main_wat.contains("\n    block\n      loop\n"),
            "main should contain lowered loop(s) after segmentation fusion, got:\n{}",
            main_wat
        );
    }

    #[test]
    fn test_wat_post_opt_dce_drops_unused_top_level_user_def() {
        let program =
            "(do (let unused_dce_probe (lambda x (+ x 1))) (let used_dce_probe (lambda x (+ x 2))) (used_dce_probe 3))";
        let std_ast = crate::baked::load_ast();
        let wrapped = match std_ast {
            crate::parser::Expression::Apply(items) => {
                crate::parser
                    ::merge_std_and_program(program, items[1..].to_vec())
                    .expect("program should merge with std")
            }
            _ => panic!("std ast should be (do ...)"),
        };

        let wat = crate::wat
            ::compile_program_to_wat(&wrapped)
            .expect("wat compilation should succeed");
        assert!(
            !wat.contains("$v_unused_dce_probe"),
            "post-optimization DCE should remove unused top-level def from emitted wat, got:\n{}",
            wat
        );
    }

    #[test]
    fn test_fused_user_entry_reinfers_in_optimizer_seed_path() {
        let program = "(|> [ 1 2 3 4 5 ] (filter even?) (map square) (reduce + 0))";
        let std_ast = crate::baked::load_ast();
        let wrapped = match std_ast {
            crate::parser::Expression::Apply(items) => {
                crate::parser
                    ::merge_std_and_program(program, items[1..].to_vec())
                    .expect("program should merge with std")
            }
            _ => panic!("std ast should be (do ...)"),
        };
        let fused_wrapped = match &wrapped {
            crate::parser::Expression::Apply(items) if
                matches!(items.first(), Some(crate::parser::Expression::Word(w)) if w == "do") &&
                items.len() > 1
            => {
                let mut out = items.clone();
                let last = out.len() - 1;
                out[last] = crate::op::fuse_map_filter_reduce_for_test(&out[last]);
                crate::parser::Expression::Apply(out)
            }
            _ => crate::op::fuse_map_filter_reduce_for_test(&wrapped),
        };
        let reinfer = crate::infer::infer_with_builtins_typed(
            &fused_wrapped,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        );
        assert!(
            reinfer.is_ok(),
            "fused whole-program expression must re-infer; got: {}",
            reinfer.err().unwrap_or_else(|| "unknown error".to_string())
        );
    }

    fn has_non_word_call_head(expr: &crate::parser::Expression) -> bool {
        match expr {
            crate::parser::Expression::Apply(items) => {
                if items.is_empty() {
                    return false;
                }
                if !matches!(items.first(), Some(crate::parser::Expression::Word(_))) {
                    return true;
                }
                items.iter().any(has_non_word_call_head)
            }
            _ => false,
        }
    }

    #[test]
    fn test_fused_pipeline_expression_has_word_only_call_heads() {
        let expr = crate::parser
            ::parse("(reduce + 0 (map square (filter even? (vector 1 2 3 4 5))))")
            .expect("input should parse")
            .remove(0);
        let fused = crate::op::fuse_map_filter_reduce_for_test(&expr);
        assert!(
            !has_non_word_call_head(&fused),
            "fused expression must not contain non-word call heads: {}",
            fused.to_lisp()
        );
    }

    #[test]
    fn test_optimized_wrapped_pipeline_has_word_only_call_heads() {
        let program = "(|> [ 1 2 3 4 5 ] (filter even?) (map square) (reduce + 0))";
        let std_ast = crate::baked::load_ast();
        let wrapped = match std_ast {
            crate::parser::Expression::Apply(items) => {
                crate::parser
                    ::merge_std_and_program(program, items[1..].to_vec())
                    .expect("program should merge with std")
            }
            _ => panic!("std ast should be (do ...)"),
        };
        let (_typ, typed) = crate::infer
            ::infer_with_builtins_typed(
                &wrapped,
                crate::types::create_builtin_environment(crate::types::TypeEnv::new())
            )
            .expect("wrapped should infer");
        let optimized = crate::op::optimize_typed_ast(&typed);
        assert!(
            !has_non_word_call_head(&optimized.expr),
            "optimized wrapped expression must not contain non-word call heads: {}",
            optimized.expr.to_lisp()
        );
    }

    #[test]
    fn test_wasm_lsp_hover_map_is_specialized_in_call_context() {
        let hover_json = crate::wasm_api::lsp_hover(r#"(map reverse ["G"])"#.to_string(), 0, 1);
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert_eq!(contents, "map : ([Char] -> [Char]) -> [[Char]] -> [[Char]]");
    }

    #[test]
    fn test_wasm_lsp_hover_map_alone_is_generic() {
        let hover_json = crate::wasm_api::lsp_hover(
            "(let xs (map reverse [\"G\"]))\nmap".to_string(),
            1,
            1
        );
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert_eq!(contents, "map : (T -> T) -> [T] -> [T]");
    }

    #[test]
    fn test_wasm_lsp_hover_let_binding_uses_rhs_type_without_extra_usage() {
        let hover_json = crate::wasm_api::lsp_hover(
            r#"(let xs (map reverse ["G"]))"#.to_string(),
            0,
            5
        );
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert_eq!(contents, "xs : [[Char]]");
    }

    #[test]
    fn test_wasm_lsp_hover_user_fn_includes_effects_for_mutation() {
        let hover_json = crate::wasm_api::lsp_hover(
            "(let touch! (lambda xs (do (set! xs 0 1) nil)))\ntouch!".to_string(),
            1,
            2
        );
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert!(
            contents.contains("touch! : [Int] -> ()"),
            "expected touch hover type info, got: {}",
            contents
        );
        assert!(
            contents.contains("effects: mutate"),
            "expected touch hover to include mutate effect, got: {}",
            contents
        );
    }

    #[test]
    fn test_wasm_lsp_hover_alias_preserves_mutation_effect() {
        let program =
            "(let std/vector/reverse! (lambda xs (do (set! xs 0 1) nil)))\n(let reverse! std/vector/reverse!)\nreverse!";
        let hover_json = crate::wasm_api::lsp_hover(program.to_string(), 2, 3);
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert!(
            contents.contains("effects: mutate"),
            "expected alias hover to include mutate effect, got: {}",
            contents
        );
    }

    #[test]
    fn test_wasm_lsp_hover_std_usage_includes_global_effects() {
        let hover_json = crate::wasm_api::lsp_hover(
            "(std/vector/reverse! [ 1 2 3 ])".to_string(),
            0,
            6
        );
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert!(
            contents.contains("effects: mutate"),
            "expected std symbol usage hover to include mutate effect, got: {}",
            contents
        );
    }

    #[test]
    fn test_wasm_lsp_hover_local_mutation_without_bang_is_local_mutate() {
        let hover_json = crate::wasm_api::lsp_hover(
            "(let fn (lambda xs (do (let out []) (push! out 0) out)))\nfn".to_string(),
            1,
            1
        );
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert!(
            contents.contains("effects: local-mutate"),
            "expected local mutation classification, got: {}",
            contents
        );
        assert!(
            !contents.contains("effects: mutate"),
            "expected not to classify as external mutate, got: {}",
            contents
        );
    }

    #[test]
    fn test_wasm_lsp_hover_param_mutation_with_bang_is_mutate() {
        let hover_json = crate::wasm_api::lsp_hover(
            "(let append-ten! (lambda xs (push! xs 10)))\nappend-ten!".to_string(),
            1,
            3
        );
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert!(
            contents.contains("effects: mutate"),
            "expected parameter mutation classification, got: {}",
            contents
        );
    }

    #[test]
    fn test_wasm_lsp_hover_exempt_name_calling_impure_callee_is_mutate() {
        let hover_json = crate::wasm_api::lsp_hover(
            "(let _fn (lambda xs (std/vector/reverse! xs)))\n_fn".to_string(),
            1,
            1
        );
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert!(
            contents.contains("effects: mutate"),
            "expected external mutation classification via impure callee, got: {}",
            contents
        );
        assert!(
            !contents.contains("effects: local-mutate"),
            "expected not to classify as local-only mutation, got: {}",
            contents
        );
    }

    #[test]
    fn test_wasm_lsp_hover_string_literal_has_fenced_que_format() {
        let hover_json = crate::wasm_api::lsp_hover("\"dsadas\"".to_string(), 0, 2);
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert_eq!(contents, "[Char] length : 6 preview : \"dsadas\"");
    }

    #[test]
    fn test_wasm_lsp_hover_numeric_literal_shows_type_without_echo() {
        let hover_json = crate::wasm_api::lsp_hover("123".to_string(), 0, 1);
        let hover: serde_json::Value = serde_json
            ::from_str(&hover_json)
            .expect("hover response should be valid JSON");

        let contents = hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("hover response should include string contents");

        assert_eq!(contents, "Int");
    }

    #[test]
    fn test_wasm_lsp_hover_zip_lambda_params_use_local_element_type_not_std_impl_param_type() {
        let program =
            r#"(let xs [ 1 2 3 4 ])
(|>
    { (|> xs (map identity) (sort <)) xs }
    (zip)
    (map (lambda { a b } (<> a b)))
)"#;

        let needle = "(<> a b)";
        let base = program.find(needle).expect("program should contain comparison form");
        let a_off = base + needle.find('a').expect("comparison should contain 'a'");
        let b_off = base + needle.rfind('b').expect("comparison should contain 'b'");

        let a_pos = crate::lsp_native_core::byte_offset_to_position(program, a_off);
        let b_pos = crate::lsp_native_core::byte_offset_to_position(program, b_off);

        let a_hover_json = crate::wasm_api::lsp_hover(
            program.to_string(),
            a_pos.line,
            a_pos.character
        );
        let b_hover_json = crate::wasm_api::lsp_hover(
            program.to_string(),
            b_pos.line,
            b_pos.character
        );

        let a_hover: serde_json::Value = serde_json
            ::from_str(&a_hover_json)
            .expect("a hover response should be valid JSON");
        let b_hover: serde_json::Value = serde_json
            ::from_str(&b_hover_json)
            .expect("b hover response should be valid JSON");

        let a_contents = a_hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("a hover response should include string contents");
        let b_contents = b_hover
            .get("contents")
            .and_then(|v| v.as_str())
            .expect("b hover response should include string contents");

        assert!(
            a_contents.contains("a : Int"),
            "expected a to resolve to element type Int, got: {}",
            a_contents
        );
        assert!(
            b_contents.contains("b : Int"),
            "expected b to resolve to element type Int, got: {}",
            b_contents
        );
        assert!(
            !a_contents.contains("a : ["),
            "expected a not to resolve to std zip impl vector param type, got: {}",
            a_contents
        );
        assert!(
            !b_contents.contains("b : ["),
            "expected b not to resolve to std zip impl vector param type, got: {}",
            b_contents
        );
    }

    #[test]
    fn test_wasm_lsp_diagnostics_reports_if_branch_type_mismatch() {
        let diagnostics_json = crate::wasm_api::lsp_diagnostics("(if true 8 2.)".to_string());
        let diagnostics: serde_json::Value = serde_json
            ::from_str(&diagnostics_json)
            .expect("diagnostics response should be valid JSON");

        let has_unify_message = diagnostics
            .as_array()
            .map(|items| {
                items.iter().any(|item| {
                    item.get("message")
                        .and_then(|v| v.as_str())
                        .map(|msg| msg.contains("Cannot unify Int with Dec"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        let has_branch_message = diagnostics
            .as_array()
            .map(|items| {
                items.iter().any(|item| {
                    item.get("message")
                        .and_then(|v| v.as_str())
                        .map(|msg| msg.contains("Concequent and alternative must match types"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        assert!(
            has_unify_message,
            "expected diagnostics to include 'Cannot unify Int with Dec', got: {}",
            diagnostics_json
        );
        assert!(
            has_branch_message,
            "expected diagnostics to include branch mismatch detail, got: {}",
            diagnostics_json
        );
    }

    #[test]
    fn test_type_inference_occurs_check_message_order() {
        let program = "(let v [])\n(push! v v)";
        let std_ast = crate::baked::load_ast();
        let wrapped = match &std_ast {
            crate::parser::Expression::Apply(items) => {
                crate::parser
                    ::merge_std_and_program(program, items[1..].to_vec())
                    .expect("program should parse with std")
            }
            _ => panic!("expected baked std ast to be an application"),
        };

        let err = crate::infer
            ::infer_with_builtins_typed(
                &wrapped,
                crate::types::create_builtin_environment(crate::types::TypeEnv::new())
            )
            .map(|_| ())
            .expect_err("expected occurs check inference error");

        let lines: Vec<&str> = err.lines().collect();
        assert!(
            !lines.is_empty() && lines[0].starts_with("Occurs check failed: t"),
            "expected first line to be occurs-check message, got: {}",
            err
        );
        assert!(
            lines.last().copied() == Some("(push! v v)"),
            "expected source snippet to be last line, got: {}",
            err
        );
    }

    #[test]
    fn test_wasm_lsp_diagnostics_occurs_check_summary_drops_snippet() {
        let diagnostics_json = crate::wasm_api::lsp_diagnostics(
            "(let v [])\n(push! v v)".to_string()
        );
        let diagnostics: serde_json::Value = serde_json
            ::from_str(&diagnostics_json)
            .expect("diagnostics response should be valid JSON");

        let messages: Vec<&str> = diagnostics
            .as_array()
            .expect("diagnostics should be an array")
            .iter()
            .filter_map(|item| item.get("message").and_then(|v| v.as_str()))
            .collect();

        assert!(
            messages.iter().any(|msg| msg.starts_with("Occurs check failed: t")),
            "expected occurs-check diagnostic, got: {}",
            diagnostics_json
        );
        assert!(
            messages.iter().all(|msg| !msg.contains("(push! v v)")),
            "expected summary label to drop trailing snippet, got: {}",
            diagnostics_json
        );
    }

    #[test]
    fn test_wasm_lsp_diagnostics_whitespace_is_empty() {
        let diagnostics_json = crate::wasm_api::lsp_diagnostics(" \n\t  ".to_string());
        let diagnostics: serde_json::Value = serde_json
            ::from_str(&diagnostics_json)
            .expect("diagnostics response should be valid JSON");

        let items = diagnostics.as_array().expect("diagnostics response should be an array");
        assert!(
            items.is_empty(),
            "expected no diagnostics for whitespace-only text, got: {}",
            diagnostics_json
        );
    }

    #[test]
    fn test_wasm_lsp_diagnostics_scope_fallback_avoids_outer_loop_condition() {
        let program =
            "(let fn (lambda x (get x)))\n\n(integer x 1)\n(while (< (get x) 10) (do\n (match? (get x) \"10\")\n(&alter! x (+ (&get x) 1))))";
        let diagnostics_json = crate::wasm_api::lsp_diagnostics(program.to_string());
        let diagnostics: serde_json::Value = serde_json
            ::from_str(&diagnostics_json)
            .expect("diagnostics response should be valid JSON");
        let items = diagnostics.as_array().expect("diagnostics response should be an array");

        assert!(!items.is_empty(), "expected at least one diagnostic, got: {}", diagnostics_json);

        let loop_condition_pos = (3u64, 9u64);
        let match_scope_pos = (4u64, 14u64);
        let mut hits_match_scope = false;

        for item in items {
            let range = item.get("range").expect("diagnostic should include range");
            let start = range.get("start").expect("range should include start");
            let end = range.get("end").expect("range should include end");
            let sl = start
                .get("line")
                .and_then(|v| v.as_u64())
                .expect("start.line should be u64");
            let sc = start
                .get("character")
                .and_then(|v| v.as_u64())
                .expect("start.character should be u64");
            let el = end
                .get("line")
                .and_then(|v| v.as_u64())
                .expect("end.line should be u64");
            let ec = end
                .get("character")
                .and_then(|v| v.as_u64())
                .expect("end.character should be u64");

            let contains = |line: u64, ch: u64| {
                let after_start = line > sl || (line == sl && ch >= sc);
                let before_end = line < el || (line == el && ch < ec);
                after_start && before_end
            };

            assert!(
                !contains(loop_condition_pos.0, loop_condition_pos.1),
                "diagnostic should not include outer loop condition site, got: {}",
                diagnostics_json
            );
            if contains(match_scope_pos.0, match_scope_pos.1) {
                hits_match_scope = true;
            }
        }

        assert!(
            hits_match_scope,
            "expected at least one diagnostic to cover lambda/match? scope, got: {}",
            diagnostics_json
        );
    }

    #[test]
    fn test_wasm_lsp_diagnostics_scope_path_keeps_get_error_in_fn_only() {
        let program =
            "(let fn (lambda x (get x 's')))\n\n(integer x 1)\n(while (< (get x) 10) (do\n    (match? (Integer->String (get x)) \"10\")\n(&alter! x (+ (&get x) 1))))";
        let diagnostics_json = crate::wasm_api::lsp_diagnostics(program.to_string());
        let diagnostics: serde_json::Value = serde_json
            ::from_str(&diagnostics_json)
            .expect("diagnostics response should be valid JSON");
        let items = diagnostics.as_array().expect("diagnostics response should be an array");

        assert!(!items.is_empty(), "expected at least one diagnostic, got: {}", diagnostics_json);

        for item in items {
            let message = item
                .get("message")
                .and_then(|v| v.as_str())
                .expect("diagnostic should include message");
            assert!(
                message.contains("Cannot unify"),
                "expected type error diagnostic, got: {}",
                diagnostics_json
            );

            let start_line = item
                .get("range")
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(|v| v.as_u64())
                .expect("diagnostic should include range.start.line");
            let end_line = item
                .get("range")
                .and_then(|r| r.get("end"))
                .and_then(|e| e.get("line"))
                .and_then(|v| v.as_u64())
                .expect("diagnostic should include range.end.line");

            assert_eq!(
                start_line,
                0,
                "diagnostic should be scoped to fn form only, got: {}",
                diagnostics_json
            );
            assert_eq!(
                end_line,
                0,
                "diagnostic should be scoped to fn form only, got: {}",
                diagnostics_json
            );
        }
    }

    #[test]
    fn test_wasm_lsp_diagnostics_empty_application_scoped_to_its_top_form() {
        let program =
            "(let fn1 (lambda x x))\n(let fn2 (lambda x x))\n() ; Error!: Empty application";
        let diagnostics_json = crate::wasm_api::lsp_diagnostics(program.to_string());
        let diagnostics: serde_json::Value = serde_json
            ::from_str(&diagnostics_json)
            .expect("diagnostics response should be valid JSON");
        let items = diagnostics.as_array().expect("diagnostics response should be an array");

        assert_eq!(
            items.len(),
            1,
            "expected a single scoped diagnostic, got: {}",
            diagnostics_json
        );
        let item = &items[0];
        let start_line = item
            .get("range")
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("line"))
            .and_then(|v| v.as_u64())
            .expect("diagnostic should include range.start.line");
        let end_line = item
            .get("range")
            .and_then(|r| r.get("end"))
            .and_then(|e| e.get("line"))
            .and_then(|v| v.as_u64())
            .expect("diagnostic should include range.end.line");
        let message = item
            .get("message")
            .and_then(|v| v.as_str())
            .expect("diagnostic should include message");

        assert_eq!(
            start_line,
            2,
            "expected empty application error on third form, got: {}",
            diagnostics_json
        );
        assert_eq!(
            end_line,
            2,
            "expected empty application error on third form, got: {}",
            diagnostics_json
        );
        assert!(
            message.contains("Empty application"),
            "expected empty application diagnostic message, got: {}",
            diagnostics_json
        );
    }

    #[test]
    fn test_wasm_lsp_diagnostics_scope_mismatch_prefers_unique_snippet_location() {
        let program =
            "(let text (reduce (lambda acc file (cons acc (|> file read! (map lower)))) \"\" ARGV))\n\
(let arrow (lambda xs (unless (empty? xs) (cons \" <- \" xs) \"\")))\n\
(|> (range 0 25)\n\
    (map (lambda i (+# (Int->Char i) 'a'))) ; generate alphabet\n\
    (map (lambda ch { ch (count/char ch text) }))\n\
    (sort (lambda { a na } { b nb } (if (= na nb) (<# a b) (> na nb))))\n\
    (map (lambda { ch n } (cons [ch] \": \" (Integer->String n)\n\
                                (arrow\n\
                                    (cond\n\
                                        (> n 9000) \"It's over Nine Thousaaaaaand!\"\n\
                                        (> n 4000) \"You seem to like this letter\"\n\
                                        (= n 999) \"I have 999 problems and this letter ain't one\"\n\
                                        (or (= n 6) (= 7)) (if (= n 6) \"7\" \"6 7\")\n\
                                        (= n 42) \"Meaning of life\"\n\
                                        (= n 777) \"Lucky\"\n\
                                        (= n 666) \"Devil\"\n\
                                        (= n 1337) \"leet\"\n\
                                        (= n 69) \";)\"\n\
                                        \"\")))))\n\
    (Vector->String nl))";

        let diagnostics_json = crate::wasm_api::lsp_diagnostics(program.to_string());
        let diagnostics: serde_json::Value = serde_json
            ::from_str(&diagnostics_json)
            .expect("diagnostics response should be valid JSON");
        let items = diagnostics.as_array().expect("diagnostics response should be an array");
        assert!(!items.is_empty(), "expected at least one diagnostic, got: {}", diagnostics_json);

        let wrong_map_pos = (3u64, 10u64);
        let expected_or_pos = (12u64, 10u64);
        let mut hits_or = false;

        for item in items {
            let range = item.get("range").expect("diagnostic should include range");
            let start = range.get("start").expect("range should include start");
            let end = range.get("end").expect("range should include end");
            let sl = start
                .get("line")
                .and_then(|v| v.as_u64())
                .expect("start.line should be u64");
            let sc = start
                .get("character")
                .and_then(|v| v.as_u64())
                .expect("start.character should be u64");
            let el = end
                .get("line")
                .and_then(|v| v.as_u64())
                .expect("end.line should be u64");
            let ec = end
                .get("character")
                .and_then(|v| v.as_u64())
                .expect("end.character should be u64");

            let contains = |line: u64, ch: u64| {
                let after_start = line > sl || (line == sl && ch >= sc);
                let before_end = line < el || (line == el && ch < ec);
                after_start && before_end
            };

            assert!(
                !contains(wrong_map_pos.0, wrong_map_pos.1),
                "diagnostic should not include early map lambda site, got: {}",
                diagnostics_json
            );
            if contains(expected_or_pos.0, expected_or_pos.1) {
                hits_or = true;
            }
        }

        assert!(
            hits_or,
            "expected diagnostic to cover the (or (= n 6) (= 7)) site, got: {}",
            diagnostics_json
        );
    }

    #[test]
    #[cfg(feature = "io")]
    fn test_wat_host_print_releases_temporary_string_arg() {
        let expr = crate::parser
            ::build(r#"(do (print! "hello") 1)"#)
            .expect("program should build");
        let wat = crate::wat::compile_program_to_wat(&expr).expect("program should compile");

        let host_pos = wat.find("call $host_print").expect("expected host print call in wat");
        let release_pos = wat[host_pos..]
            .find("call $rc_release")
            .map(|p| host_pos + p)
            .expect("expected rc_release after host print call");

        assert!(
            release_pos > host_pos,
            "expected temporary [Char] arg to be released after print!, got wat:\n{}",
            wat
        );
    }
    #[test]
    fn test_big_iterations_ref_leak_crash() {
        let test_case =
            r#"
(let box (lambda value [value]))
(let int box)
(let dec box)
(let bool box)
(let set-box! (lambda vrbl x (set! vrbl 0 x)))
(let =! (lambda vrbl x (set! vrbl 0 x)))
(let &alter! (lambda vrbl x (set! vrbl 0 x)))
(let &mut! (lambda value [value]))
(let &get (lambda vrbl (get vrbl 0)))

(let boole-set! (lambda vrbl x (set! vrbl 0 (if x true false))))
(let boole-eqv (lambda a b (=? (get a) (get b))))
(let true? (lambda vrbl (if (get vrbl) true false)))
(let false? (lambda vrbl (if (get vrbl) false true)))
(let += (lambda vrbl n (&alter! vrbl (+ (get vrbl) n))))
(let -= (lambda vrbl n (&alter! vrbl (- (get vrbl) n))))
(let *= (lambda vrbl n (&alter! vrbl (* (get vrbl) n))))
(let /= (lambda vrbl n (&alter! vrbl (/ (get vrbl) n))))
(let ++ (lambda vrbl (&alter! vrbl (+ (get vrbl) 1))))
(let -- (lambda vrbl (&alter! vrbl (- (get vrbl) 1))))
(let ** (lambda vrbl (&alter! vrbl (* (get vrbl) (get vrbl)))))


(let +=. (lambda vrbl n (&alter! vrbl (+. (get vrbl) n))))
(let -=. (lambda vrbl n (&alter! vrbl (-. (get vrbl) n))))
(let *=. (lambda vrbl n (&alter! vrbl (*. (get vrbl) n))))
(let /=. (lambda vrbl n (&alter! vrbl (/. (get vrbl) n))))
(let ++. (lambda vrbl (&alter! vrbl (+. (get vrbl) 1.0))))
(let --. (lambda vrbl (&alter! vrbl (-. (get vrbl) 1.0))))
(let **. (lambda vrbl (&alter! vrbl (*. (get vrbl) (get vrbl)))))

    (let fn (lambda a b (do (let y [a b]) (set! y (length y) 10) y)))
    (let outer (lambda z (do (let g (fn 1 2)) (length g))))
    (mut bi0 0)
    (while (< bi0 500000) (do
      (outer bi0)
      (alter! bi0 (+ bi0 1))))
    1
    
    (let mk1 (lambda n (do
      (let v [])
      (mut i 0)
      (while (< i n) (do
        (set! v (length v) [i (+ i 1)])
        (alter! i (+ i 1))))
      v)))
    (mut bi1 0)
    (while (< bi1 200000) (do
      (let t (mk1 4))
      (length t)
      (alter! bi1 (+ bi1 1))))
    1
    
    (let mk2 (lambda n (do
      (let v [])
      (mut i 0)
      (while (< i n) (do
        (set! v (length v) i)
        (alter! i (+ i 1))))
      v)))
    (mut bi2 0)
    (while (< bi2 300000) (do
      (let a (mk2 3))
      (let b (mk2 40))
      (+ (length a) (length b))
      (alter! bi2 (+ bi2 1))))
    1
    
    ; closure captures a vector, closure dies each loop iteration
    (let mk-reader (lambda n (do
      (let xs [])
      (mut i 0)
      (while (< i n) (do
        (set! xs (length xs) i)
        (alter! i (+ i 1))))
      (lambda idx (get xs idx)))))
    
    (mut bi3 0)
    (while (< bi3 500000) (do
      (let f (mk-reader 8))
      (f 3)
      (alter! bi3 (+ bi3 1))))
    1
    ; closure captures another closure (which captures a vector)
    (let mk-pipeline (lambda n (do
      (let xs [n (+ n 1)])
      (let add-base (lambda x (+ x (get xs 0))))
      (lambda y (add-base (+ y (get xs 1)))))))
    
    (mut bi4 0)
    (while (< bi4 400000) (do
      (let g (mk-pipeline bi4))
      (g 1)
      (alter! bi4 (+ bi4 1))))
    1
    
    ; closure captures another closure (which captures a vector)
    (let mk-pipeline2 (lambda n (do
      (let xs [n (+ n 1)])
      (let add-base (lambda x (+ x (get xs 0))))
      (lambda y (add-base (+ y (get xs 1)))))))
    
    (mut bi5 0)
    (while (< bi5 400000) (do
      (let g (mk-pipeline2 bi5))
      (g 1)
      (alter! bi5 (+ bi5 1))))
    1
    ; closure captures another closure (which captures a vector)
    (let mk-pipeline3 (lambda n (do
      (let xs [n (+ n 1)])
      (let add-base (lambda x (+ x (get xs 0))))
      (lambda y (add-base (+ y (get xs 1)))))))
    
    (mut bi6 0)
    (while (< bi6 400000) (do
      (let g (mk-pipeline3 bi6))
      (g 1)
      (alter! bi6 (+ bi6 1))))
    1
    
    ; closure captures another closure (which captures a vector)
    (let mk-pipeline4 (lambda n (do
      (let xs [n (+ n 1)])
      (let add-base (lambda x (+ x (get xs 0))))
      (lambda y (add-base (+ y (get xs 1)))))))
    
    (mut bi7 0)
    (while (< bi7 400000) (do
      (let g (mk-pipeline4 bi7))
      (g 1)
      (alter! bi7 (+ bi7 1))))
    1
    
    ; closure captures another closure (which captures a vector)
    (let mk-pipeline5 (lambda n (do
      (let xs [n (+ n 1)])
      (let add-base (lambda x (+ x (get xs 0))))
      (lambda y (add-base (+ y (get xs 1)))))))
    
    (mut bi8 0)
    (while (< bi8 400000) (do
      (let g (mk-pipeline5 bi8))
      (g 1)
      (alter! bi8 (+ bi8 1))))
    1
    
    ; closure captures another closure (which captures a vector)
    (let mk-pipeline6 (lambda n (do
      (let xs [n (+ n 1)])
      (let add-base (lambda x (+ x (get xs 0))))
      (lambda y (add-base (+ y (get xs 1)))))))
    
    (mut bi9 0)
    (while (< bi9 400000) (do
      (let g (mk-pipeline6 bi9))
      (g 1)
      (alter! bi9 (+ bi9 1))))
    1
    
    ; closure captures nested vectors
    (let mk-grid-reader7 (lambda n (do
      (let rows [])
      (mut i 0)
      (while (< i n) (do
        (set! rows (length rows) [i (+ i 1) (+ i 2)])
        (alter! i (+ i 1))))
      (lambda j (+ (get (get rows 0) 0) (get (get rows 1) 1) j)))))
    
    (mut bi10 0)
    (while (< bi10 250000) (do
      (let r (mk-grid-reader7 4))
      (r 1)
      (alter! bi10 (+ bi10 1))))
    1
    ; same idea as your outer/fn pattern but closure-returning inner
    (let make2 (lambda a b (do
      (let y [a b])
      (set! y (length y) 10)
      (lambda k (+ (length y) k)))))
    
    (let outer2 (lambda z (do
      (let g (make2 1 2))
      (g z))))
    
    (mut bi11 0)
    (while (< bi11 500000) (do
      (outer2 bi11)
      (alter! bi11 (+ bi11 1))))
    1
    
    ; vector fn clean up
    (let build-fns (lambda n (do
      (let fs [])
      (mut i 0)
      (while (< i n) (do
        (let idx i)
        (set! fs (length fs) (lambda x (+ x idx)))
        (alter! i (+ i 1))))
      fs)))
    
    (let sum-call (lambda fs n (do
      (integer acc 0)
      (mut i 0)
      (while (< i n) (do
        (let f (get fs i))
        (&alter! acc (+ (&get acc) (f 1)))
        (alter! i (+ i 1))))
      (get acc))))
    
    (let once (sum-call (build-fns 5) 5))
    
    (mut bi12 0)
    (while (< bi12 50000) (do
      (let fs (build-fns 16))
      (sum-call fs 16)
      (alter! bi12 (+ bi12 1))))
    
    (= once 15)"#;

        let expr = crate::parser::build(test_case).expect("program should build");
        let wat = crate::wat::compile_program_to_wat(&expr).expect("program should compile");
        let argv: Vec<String> = Vec::new();
        #[cfg(feature = "io")]
        let store_data = crate::io::ShellStoreData
            ::new_with_security(None, crate::io::ShellPolicy::disabled())
            .map_err(|e| e.to_string())
            .unwrap();
        #[cfg(feature = "io")]
        let run_result = crate::runtime::run_wat_text(&wat, store_data, &argv, |linker| {
            crate::io::add_shell_to_linker(linker).map_err(|e| e.to_string())
        });
        #[cfg(not(feature = "io"))]
        let run_result = crate::runtime::run_wat_text(&wat, (), &argv, |_linker| Ok(()));
        assert_eq!(
            run_result.expect("program should run without trap"),
            "true",
            "expected stress loop to finish without memory trap"
        );
    }
    #[test]
    #[cfg(feature = "runtime")]
    fn test_set_ref_vector_stress_no_leak_crash() {
        let expr = crate::parser
            ::build(
                r#"(do
(let x [[]])
(mut i 0)
(while (< i 300000) (do
  (set! x 0 [])
  (alter! i (+ i 1))))
1)"#
            )
            .expect("program should build");
        let wat = crate::wat::compile_program_to_wat(&expr).expect("program should compile");
        let argv: Vec<String> = Vec::new();
        #[cfg(feature = "io")]
        let store_data = crate::io::ShellStoreData
            ::new_with_security(None, crate::io::ShellPolicy::disabled())
            .map_err(|e| e.to_string())
            .unwrap();
        #[cfg(feature = "io")]
        let run_result = crate::runtime::run_wat_text(&wat, store_data, &argv, |linker| {
            crate::io::add_shell_to_linker(linker).map_err(|e| e.to_string())
        });
        #[cfg(not(feature = "io"))]
        let run_result = crate::runtime::run_wat_text(&wat, (), &argv, |_linker| Ok(()));
        assert_eq!(
            run_result.expect("program should run without trap"),
            "1",
            "expected stress loop to finish without memory trap"
        );
    }

    #[test]
    #[cfg(feature = "runtime")]
    fn test_correctness() {
        let test_cases = [
            ("nil", "0"),
            ("(+ 1 2)", "3"),
            ("(std/vector/int/sum [ 1 2 ])", "3"),
            ("\"Hello world\"", "Hello world"),
            (
                r#"(let A [false (and (= 1 2) (> 3 3))]) ; => [false false] Correct
(let B [false (or (= 1 2) (> 3 3))]) ; => [true false] Wrong
(and (=? (get A 0) (get B 0)) (=? (get A 1) (get B 1)))"#,
                "true",
            ),
            (
                r#"(let samples [
        "(())"    ; result in floor 0.
        "()()"    ; result in floor 0.
        "((("     ; result in floor 3.
        "(()(()(" ; result in floor 3.
        "))(((((" ; also results in floor 3.
        "())"     ; result in floor -1 (the first basement level).
        "))("     ; result in floor -1 (the first basement level).
        ")))"     ; result in floor -3.
        ")())())" ; result in floor -3.
])
(let solve (lambda input (- (std/vector/char/count input std/char/left-brace) (std/vector/char/count input std/char/right-brace))))
(std/vector/map samples solve)"#,
                "[0 0 3 3 3 -1 -1 -3 -3]",
            ),
            (
                r#"(let last-stone-weight (lambda stones (do
  (let max-cmp (lambda a b (> a b)))
  (let heap (std/convert/vector->heap stones max-cmp))
  (letrec tail-call/smash (lambda t
    (if (> (length heap) 1)
      (do
        (let y (std/heap/peek heap))
        (std/heap/pop! heap max-cmp)
        (let x (std/heap/peek heap))
        (std/heap/pop! heap max-cmp)
        (if (!= x y)
          (std/heap/push! heap (- y x) max-cmp))
        (tail-call/smash t))
        false)))
  (tail-call/smash true)
  (if (> (length heap) 0) (std/heap/peek heap) Int))))

[(last-stone-weight [ 2 7 4 1 8 1 ]) (last-stone-weight [ 1 ])]"#,
                "[1 1]",
            ),
            (
                r#"(let has-groups (lambda deck
  (do
    (let chars (<| deck
                    (std/vector/map std/convert/integer->string)
                    (std/vector/hash/table/count)
                    (std/vector/hash/table/entries)
                    (std/vector/map snd)))
    
    (let counts (as chars [Int]))
    (> (std/vector/reduce counts std/int/gcd (std/vector/first counts)) 1)
    )))

[
    (has-groups [ 1 2 3 4 4 3 2 1 ]) ; Output/ true
    (has-groups [ 1 1 1 2 2 2 3 3 ]) ; Output/ false
]
"#,
                "[true false]",
            ),
            (
                r#"
            (let find-missing-numbers (lambda nums (<| 
    (std/vector/int/range 1 (length nums)) 
    (std/vector/map (lambda x (std/convert/integer->string-base x 10)))
    (std/convert/vector->set)
    (std/vector/hash/set/difference (<| nums (std/vector/map (lambda x (std/convert/integer->string-base x 10))) (std/convert/vector->set)))
    (std/vector/flat-one)
    (std/vector/map std/convert/chars->integer))))

[
    (find-missing-numbers [ 4 3 2 7 8 2 3 1 ]) ; Output/ [5 6]
    (find-missing-numbers [ 1 1 ])             ; Output/ [2]
]
            "#,
                "[[5 6] [2]]",
            ),
            (
                r#"(let has-trailing-zeros (lambda nums (>= (std/vector/count-of nums (lambda x (= (mod x 2) 0))) 2)))

[(has-trailing-zeros [ 1 2 3 4 5 ]) ; Should return true
 (has-trailing-zeros [ 2 4 8 16 ]) ; Should return true
 (has-trailing-zeros [ 1 3 5 7 9 ]) ; Should return false
 (has-trailing-zeros [ 1 2 ])]  ; Should return false
"#,
                "[true true false false]",
            ),
            (
                r#"(let pillow-holder (lambda n time (do
  (let cycle (- (* 2 n) 2))
  (let t (mod time cycle))
  (if (< t n)
    (+ 1 t)
    (- (+ n n -1) t)))))

[(pillow-holder 4 5) (pillow-holder 3 2)]
"#,
                "[2 3]",
            ),
            (
                r#"(let flood-fill! (lambda image sr sc color (do 
    (let old (get image sr sc))
    (unless (= old color) 
        (do 
            (let m (length image))
            (let n (length (std/vector/first image)))
            (let stack [[sr sc]])
            (while (std/vector/not-empty? stack) (do 
                (let t (std/vector/last stack))
                (pop! stack)
                (let i (std/vector/first t))
                (let j (std/vector/second t))
                (if (and (>= i 0) (< i m) (>= j 0) (< j n) (= (get image i j) old)) (do
                    (std/vector/3d/set! image i j color)
                    (std/vector/push! stack [(+ i 1) j])
                    (std/vector/push! stack [(- i 1) j])
                    (std/vector/push! stack [i (+ j 1)])
                    (std/vector/push! stack [i (- j 1)])
                    nil))))
        nil)))))


(let image [[1 1 1] [1 1 0] [1 0 1]])
(flood-fill! image 1 1 2)
image
; Output/ [[2 2 2] [2 2 0] [2 0 1]]"#,
                "[[2 2 2] [2 2 0] [2 0 1]]",
            ),
            (
                r#"(let flood-fill! (lambda image sr sc color (do 
    (let old (get image sr sc))
    (unless (= old color) 
        (do 
            (let m (length image))
            (let n (length (get image 0)))
            (let stack [[sr sc]])
            (while (not-empty? stack) (do 
                (let t (std/vector/last stack))
                (pop! stack)
                (let i (get t 0))
                (let j (get t 1))
                (if (and (>= i 0) (< i m) (>= j 0) (< j n) (= (get image i j) old)) (do
                    (set! image i j color)
                    (push! stack [(+ i 1) j])
                    (push! stack [(- i 1) j])
                    (push! stack [i (+ j 1)])
                    (push! stack [i (- j 1)])
                    nil))))
        nil)))))


(let image [[1 1 1] [1 1 0] [1 0 1]])
(flood-fill! image 1 1 2)
image
; Output/ [[2 2 2] [2 2 0] [2 0 1]]"#,
                "[[2 2 2] [2 2 0] [2 0 1]]",
            ),
            (
                r#"(let valid-path (lambda n edges source destination (do
  (if (= source destination) true
    (do
      (let graph (std/vector/map (std/vector/int/zeroes n) (lambda _ [])))
      (std/vector/for edges (lambda edge (do
        (let u (get edge 0))
        (let v (get edge 1))
        (std/vector/push! (get graph u) v)
        (std/vector/push! (get graph v) u))))
      (let visited (std/vector/int/zeroes n))
      (let queue [source])
      (std/vector/set! visited source 1)
      (boolean found false)
      (while (and (not (true? found)) (> (length queue) 0)) (do
        (let current (std/vector/last queue))
        (pop! queue)
        (if (= current destination)
          (&alter! found true)
          (std/vector/for (get graph current) (lambda neighbor (do
            (if (= (get visited neighbor) 0)
              (do
                (std/vector/set! visited neighbor 1)
                (std/vector/push! queue neighbor) 
                nil))))))))
      (true? found))))))

[(valid-path 3 [[ 0 1 ] [ 1 2 ] [ 2 0 ]] 0 2) ; Should return true
 (valid-path 6 [[ 0 1 ] [ 0 2 ] [ 3 5 ] [ 5 4 ] [ 4 3 ]] 0 5)] ; Should return false"#,
                "[true false]",
            ),
            (
                r#"(let INPUT 
"89010123
78121874
87430965
96549874
45678903
32019012
01329801
10456732")
(let yx->key (lambda y x (std/vector/concat/with (std/vector/map [ (as y Char) (as x Char) ] (lambda c [ c ])) std/char/dash)))
(let parse (lambda input (<| input (std/convert/string->vector std/char/new-line) (std/vector/map std/convert/chars->digits))))
(let part1 (lambda matrix (do
  (let coords (std/vector/3d/points matrix std/int/zero?))
  (std/vector/reduce coords (lambda a xs (do
        (integer score 0)
        (let y (std/vector/first xs))
        (let x (std/vector/second xs))
        (let visited (std/vector/buckets 8))
        (let queue (std/vector/queue/new [Int]))
        (let current (get matrix y x))
        (std/vector/hash/set/add! visited (yx->key y x))
        (std/vector/queue/enqueue! queue [ y x ])
        
        (while (std/vector/queue/not-empty? queue) (do
            (let element (std/vector/queue/peek queue))
            (std/vector/queue/dequeue! queue )
            (let y (std/vector/first element))
            (let x (std/vector/second element))  
            (std/vector/3d/adjacent matrix std/vector/3d/von-neumann-neighborhood y x (lambda cell dir dy dx (do
                 (let key (yx->key dy dx))
                 (if (and (= (- cell (get matrix y x)) 1) (not (std/vector/hash/set/has? visited key))) (do
                    (if (= cell 9) (do (&alter! score  (+ (&get score) 1)) nil) (do (std/vector/queue/enqueue! queue [ dy dx ]) nil))
                    (std/vector/hash/set/add! visited key)
                    nil)))))))

        (+ a (get score)))) 0))))

(let part2 (lambda matrix (do
  (let coords (std/vector/3d/points matrix std/int/zero?))
  (std/vector/reduce coords (lambda a xs (do
        (integer score 0)
        (let y (std/vector/first xs))
        (let x (std/vector/second xs))
        (let visited (std/vector/buckets 8))
        (let queue (std/vector/queue/new [Int]))
        (let current (get matrix y x))
        (let root-key (yx->key y x))
        (std/vector/hash/table/set! visited root-key 1)
        (std/vector/queue/enqueue! queue [ y x ])
        (while (std/vector/queue/not-empty? queue) (do
            (let element (std/vector/queue/peek queue))
            (let y (std/vector/first element))
            (let x (std/vector/second element))  
            (if (= (get matrix y x) 9) (&alter! score (+ (&get score) (snd (get (std/vector/hash/table/get visited root-key))))))
            (std/vector/queue/dequeue! queue)
            (std/vector/3d/adjacent matrix std/vector/3d/von-neumann-neighborhood y x (lambda cell dir dy dx (do
                 (let key (yx->key dy dx))
                 (if (= (- cell (get matrix y x)) 1) (do
                    (std/vector/queue/enqueue! queue [ dy dx ])
                    (if (std/vector/hash/table/has? visited key) 
                        (std/vector/hash/table/set! visited key (+ (snd (get (std/vector/hash/table/get visited root-key))) (snd (get (std/vector/hash/table/get visited key))))) 
                        (std/vector/hash/table/set! visited key (as (snd (get (std/vector/hash/table/get visited root-key))) Int)))
                      nil)))))))
        (+ a (get score)))) 0))))

(let PARSED (parse INPUT))

[(part1 PARSED) (part2 PARSED)]
"#,
                "[36 81]",
            ),
            (
                r#"(let INPUT
"3   4
4   3
2   5
1   3
3   9
3   3")

(let parse (lambda input (<|
                            input
                            (std/vector/char/lines)
                            (std/vector/map (lambda word (<|
                                                      word
                                                      (std/vector/char/words)
                                                      (std/vector/filter std/vector/not-empty?)
                                                      (std/vector/map std/convert/chars->integer)))))))

(let part1 (lambda input (<|
                          input
                          (std/vector/unzip)
                          (std/vector/map std/vector/sort/desc!)
                          (std/vector/zip)
                          (std/vector/map std/vector/int/pair/sub)
                          (std/vector/map std/int/abs)
                          (std/vector/int/sum))))
                        
(let part2 (lambda input (do
  (let unzipped (std/vector/unzip input))
  (let left (std/vector/first unzipped))
  (let right (std/vector/second unzipped))
  (<|
    left
    (std/vector/map (lambda l (* l (std/vector/count-of right (lambda r (= l r))))))
    (std/vector/int/sum)))))

(let PARSED (parse INPUT))
[(part1 PARSED) (part2 PARSED)]"#,
                "[11 31]",
            ),
            (
                r#"
(let parse (lambda input (<| input (std/vector/char/lines) (std/vector/map std/convert/chars->integer))))
(let part1 (lambda input (do 
    (let m (std/vector/int/minimum input))
    (<| input
        (std/vector/map (lambda x (- x m)))
        (std/vector/int/sum)))))

 (let part2 (lambda inp (do
    (let input (copy inp))
    (std/vector/sort/desc! input)
    (let m (std/vector/int/median input))
    (<| input
        (std/vector/map (lambda x (cond (> x m) (- x m) (< x m) (- m x) 0)))
        (std/vector/int/sum)))))

[(<| 
"3
4
7
8"
    (parse)
    (part1)
)
(<| 
"2
4
5
6
8"
    (parse)
    (part2)
)]"#,
                "[10 8]",
            ),
            (
                r#"(let xs [1 2 0 4 3 0 5 0])
(let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))
(let solve! (lambda xs (do 
    (integer c 0)
    (let len (length xs))
    (std/vector/for xs (lambda x (if (<> x 0) (do 
        (set! xs (get c) x)
        (&alter! c (+ (&get c) 1))))))
    (while (< (get c) len) (do 
        (set! xs (get c) 0)
        (&alter! c (+ (&get c) 1))))
    nil)))

(solve! xs)
xs"#,
                "[1 2 4 3 5 0 0 0]",
            ),
            (
                r#"(let naive-sub-array-sum (lambda xs (do 
    (let n (length xs))
    (integer out 0)
    (loop 0 n (lambda i (do 
        (integer temp 0)
        (loop i n (lambda j (do 
            (&alter! temp (+ (&get temp) (get xs j)))
            (&alter! out (+ (&get out) (get temp)))))))))
    (get out))))

(let expert-sub-array-sum (lambda xs (do 
    (let n (length xs))
    (integer out 0)
    (loop 0 n (lambda i (&alter! out (+ (&get out) (* (get xs i) (+ i 1) (- n i))))))
    (get out))))

(let xs [1 4 5 3 2])
[(naive-sub-array-sum xs) (expert-sub-array-sum xs)]
"#,
                "[116 116]",
            ),
            (
                r#"
; Input / [1, 2, 4]
; Output / 125
; Explanation/ 124 + 1 = 125 

; Input / [9, 9, 9]
; Output/ 1000
; Explanation/ 999 + 1 = 1000 

[
    (+ (std/convert/digits->integer [ 1 2 4 ]) 1)
    (+ (std/convert/digits->integer [ 9 9 9 ]) 1)
]
            "#,
                "[125 1000]",
            ),
            ("(std/convert/bits->integer [ 1 0 0 0 0 0 1 1 0 0 ])", "524"),
            (
                r#"(let xs [ 1 2 3 ])
(let copy (std/vector/copy xs))
(set! copy 0 1000)
[ xs copy ]"#,
                "[[1 2 3] [1000 2 3]]",
            ),
            (
                r#"(let sort-array-by-parity2 (lambda nums (if (std/vector/empty? nums) nums (do 
    (let odd [])
    (let even [])
    (let out [])
    (loop 0 (length nums) (lambda i (std/vector/push! (if (std/int/even? i) even odd) (get nums i))))
    (loop 0 (length even) (lambda i (do (std/vector/push! out (get even i)) (std/vector/push! out (get odd i)))))
    out))))

[
  (sort-array-by-parity2 [ 4 2 5 7 ])
  (sort-array-by-parity2 [ 2 3 ])
  (sort-array-by-parity2 [ 4 3 ])
]"#,
                "[[4 2 5 7] [2 3] [4 3]]",
            ),
            ("(std/int/collinear? [[ 3 8 ] [ 5 10 ] [ 7 12 ]])", "true"),
            (
                r#"(let fn (lambda [ a b c . r ] (+ a b c (std/vector/int/product r))))
(fn [ 1 2 3 4 5 6 ])"#,
                "126",
            ),
            (
                r#"(let input "A:+,-,=,=,+,-,=,=,+,-
B:+,=,-,+,+,=,-,+,+,=
C:=,-,+,+,=,-,+,+,=,-
D:=,=,=,+,=,=,=,+,=,=")

(let parse (lambda input (do 
(<| input (std/vector/char/lines) (std/vector/map (lambda x (do 
    (let y (std/vector/char/commas x))
    (set! y 0 (get (std/convert/string->vector (get y 0) std/char/colon) 1))
    (std/vector/flat-one y)))))
)))
    
(let app (lambda a x 
    (cond (=# x std/char/plus) (std/vector/cons a [(+ (std/vector/last a) 1)])
    (=# x std/char/minus) (std/vector/cons a [(- (std/vector/last a) 1)])
    (=# x std/char/equal) (std/vector/cons a [(std/vector/last a)])
    (std/vector/cons a [(std/vector/last a)]))))
(let part1! (lambda xs (do
    (let letters (<| input (std/vector/char/lines) (std/vector/map std/vector/first)))
    (<| xs (std/vector/map (lambda x (<| x (std/vector/reduce app [0])))) 
    (std/vector/map std/vector/int/sum)
    (std/vector/map/i (lambda x i [i (+ x 100)]))
    (std/vector/sort! (lambda a b (> (get a 1) (get b 1))))
    (std/vector/map (lambda [i] (get letters i)))))))
(<| input (parse) (part1!))"#,
                "BDCA",
            ),
            (
                r#"(let palindrome? (lambda str (do 
    (let q (std/vector/queue/new '0'))
    (let s (std/vector/stack/new '0'))
    
    (std/vector/for str (lambda x (do
        (std/vector/stack/push! s x)
        (std/vector/queue/enqueue! q x))))
    
    (let p? [true])

    (mut i 0)
    (while (< i (/ (length str) 2)) (do
      (if (not (=# (std/vector/stack/peek s) (std/vector/queue/peek q)))
           (&alter! p? false) 
           (do 
               (std/vector/stack/pop! s)
               (std/vector/queue/dequeue! q)
               nil))
      (alter! i (+ i 1))))
    (get p?))))
    
[(palindrome? "racecar") (palindrome? "yes")]"#,
                "[true false]",
            ),
            (
                r#"(let palindrome? (lambda str (do 
    (let p? [true])
    (loop 0 (/ (length str) 2) (lambda i (if (not (=# (get str i) (get str (- (length str) i 1)))) (&alter! p? false))))
    (true? p?))))
[(palindrome? "racecar") (palindrome? "yes")]"#,
                "[true false]",
            ),
            (
                r#"(let palindrome? (lambda str (std/vector/char/match? str (std/vector/reverse str))))
[(palindrome? "racecar") (palindrome? "yes")]"#,
                "[true false]",
            ),
            (
                r#"(letrec rev! (lambda ys xs 
    (if (std/vector/empty? xs) 
         ys 
        (rev! (do (std/vector/push! ys (std/vector/at xs -1)) ys) (std/vector/drop/last xs 1)))))
;
(rev! [] [ 1 2 3 4 5 ])"#,
                "[5 4 3 2 1]",
            ),
            (
                r#"
[

(std/int/big/div [ 1 0 ] [ 5 ])
(std/int/big/add [ 9 9 9 ] [ 1 2 ])
(std/int/big/sub [ 1 0 1 ] [ 1 1 ])
(std/int/big/mul [ 2 ] [ 9 9 5 ])

]
"#,
                "[[2] [1 0 1 1] [9 0] [1 9 9 0]]",
            ),
            (
                r#"(let fn (lambda xs (do 
    (integer max 0)
    (integer i 0)
    (integer j (- (length xs) 1))
    (while (<> (get i) (get j)) (do 
        (if (> (get xs (get i)) (get xs (get j))) (do 
            (&alter! max (std/int/max (* (- (get j) (get i)) (get xs (get j))) (get max)))
            (&alter! j (- (&get j) 1))) (do
            (&alter! max (std/int/max (* (- (get j) (get i)) (get xs (get i))) (get max)))
            (&alter! i (+ (&get i) 1))))))
    (get max))))

[
    (fn [ 1 8 6 2 5 4 8 3 7 ]) ; 49
    (fn [ 1 1 ]) ; 1
]"#,
                "[49 1]",
            ),
            (
                r#"
(letrec factorial (lambda n total
    (if (= (get n 0) 0)
        total
        (factorial (std/int/big/sub n [ 1 ]) (std/int/big/mul total n)))))

(let bionomial-coefficient (lambda a b
    (std/int/big/div (factorial a [ 1 ])
            (std/int/big/mul
                (factorial b [ 1 ])
                (factorial (std/int/big/sub a b) [ 1 ])))))

(let m [ 2 0 ])
(let n [ 2 0 ])
(bionomial-coefficient (std/int/big/add m n) m)
; [Int]
; [1 3 7 8 4 6 5 2 8 8 2 0]"#,
                "[1 3 7 8 4 6 5 2 8 8 2 0]",
            ),
            (
                r#"(letrec fibonacci (lambda n 
    (if (< n 2) n 
        (+ (fibonacci (- n 1)) (fibonacci (- n 2))))))

(fibonacci 10)"#,
                "55",
            ),
            (
                r#"(let str "
 1 + 2 = 3
 3 + 3 = 6
 8 + -1 = 7
 8 + 1 = 9
 8 + 1 = 10
")
(<|
 str
 (std/convert/string->vector std/char/new-line)
 (std/vector/filter std/vector/not-empty?) ; trim
 (std/vector/map (lambda xs
   (<| xs
     (std/convert/string->vector std/char/space)
     (std/vector/filter std/vector/not-empty?)
     (std/vector/filter/i (lambda _ i (std/int/even? i)))
     (std/vector/map std/convert/chars->integer))))
 (std/vector/map (lambda [ a b c ] (= (+ a b) c)))
 (std/vector/count-of (lambda x (eq x true))))"#,
                "4",
            ),
            (
                r#"(let num-rabbits (lambda answers
  (<| answers
      (std/vector/map std/convert/integer->string)
      (std/vector/hash/table/count)
      (std/vector/hash/table/entries)
                
      (std/vector/reduce (lambda acc { str cnt }
        (+ acc (* (std/int/ceil/div cnt (+ (std/convert/chars->integer str) 1))
                  (+ (std/convert/chars->integer str) 1))))
      0)
      
      )))
    
[
    (num-rabbits [ 1 1 2 ]) ; Output/ 5
    (num-rabbits [ 10 10 10 ]) ; Output/ 11
]
"#,
                "[5 11]",
            ),
            (
                r#"(let count-apples-and-oranges (lambda s t a b apples oranges (do
          (let helper (lambda xs m (<| xs (std/vector/map (lambda x (+ x m))) (std/vector/count-of (lambda x (and (>= x s) (<= x t)))))))
          [(helper apples a) (helper oranges b)])))
      
      (count-apples-and-oranges 7 11 5 15 [ -2 2 1 ] [ 5 -6 ])"#,
                "[1 1]",
            ),
            (
                r#"(let count-points (lambda rings (do
  (let rods (std/vector/map (std/vector/int/zeroes 10) (lambda _ [false false false]))) ; [R, G, B] for each rod
  (let len (length rings))
  (loop 0 len (lambda i (do
    (if (std/int/even? i)
      (do
        (let color (get rings i))
        (let rod-char (get rings (+ i 1)))
        (let rod (get rods (- (std/convert/char->digit rod-char) 0)))
        (cond
          (=# color 'R') (set! rod 0 true)
          (=# color 'G') (set! rod 1 true)
          (=# color 'B') (set! rod 2 true)
          nil))))))
  (std/vector/count-of rods (lambda rod (and (get rod 0) (get rod 1) (get rod 2)))))))

; Example usage
[(count-points "B0B6G0R6R0R6G9") ; Should return 1
 (count-points "B0R0G0R9R0B0G0") ; Should return 1
 (count-points "G4")] ; Should return 0"#,
                "[1 1 0]",
            ),
            (
                r#"(let part1 (lambda input (<| input 
    (std/vector/cons [(std/vector/first input)]) 
    (std/vector/sliding-window 2) 
    (std/vector/filter (lambda x (= (get x 0) (get x 1))))
    (std/vector/map std/vector/first)
    (std/vector/int/sum))))
(let part2 (lambda input (<| input
    (std/vector/cons (std/vector/slice input 0 (/ (length input) 2)))
    (std/vector/sliding-window (+ (/ (length input) 2) 1))
    (std/vector/filter (lambda x (= (std/vector/first x) (std/vector/last x))))
    (std/vector/map std/vector/first)
    (std/vector/int/sum))))
    
[
  (<| ["1122" "1111" "1234" "91212129"] (std/vector/map std/convert/chars->digits) (std/vector/map part1)) 
  (<| ["1212"  "1221" "123425" "123123" "12131415"] (std/vector/map std/convert/chars->digits) (std/vector/map part2))
]"#,
                "[[3 4 0 9] [6 0 4 12 4]]",
            ),
            (
                r#"(let INPUT "0
3
0
1
-3")
(let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))
(let parse (lambda input (<| input (std/convert/string->vector std/char/new-line) (std/vector/map std/convert/chars->integer))))
(let part1 (lambda ip (do
    (let input (copy ip))
    (integer pointer (get input))
    (integer steps 0)
    (integer index 0)
    (boolean escaped? false)
    (while (false? escaped?) (do
        (set! input (get index) (+ (get pointer) 1))
        (&alter! index (+ (&get index) (get pointer)))
        (if (std/vector/in-bounds? input (get index)) (&alter! pointer (get input (get index))) (&alter! escaped? true))
        (&alter! steps (+ (&get steps) 1))))
    (get steps))))

(let part2 (lambda ip (do 
    (let input (copy ip))
    (integer pointer (get input))
    (integer steps 0)
    (integer index 0)
    (boolean escaped? false)
    (while (false? escaped?) (do
        (set! input (get index) (+ (get pointer) (if (>= (get pointer) 3) -1 1)))
        (&alter! index (+ (&get index) (get pointer)))
        (if (std/vector/in-bounds? input (get index)) (&alter! pointer (get input (get index))) (&alter! escaped? true))
        (&alter! steps (+ (&get steps) 1))))
    (get steps))))
    
[(<| INPUT (parse) (part1)) (<| INPUT (parse) (part2))]"#,
                "[5 10]",
            ),
            (
                r#"
; Kadane's algorithm: returns maximum subarray sum for a vector of Ints
(let max-subarray (lambda xs (do 
  (let step (lambda acc x (do 
    (let current (std/int/max x (+ (get acc 0) x)))
    (let best (std/int/max (get acc 1) current))
    [ current best ])))

  (let init [ (get xs 0) (get xs 0) ]) ; start with first element as current and best
  (let rest (std/vector/drop xs 1))
  (let result (std/vector/reduce rest step init))
  (get result 1))))

; Examples
[
    (max-subarray [ -2 1 -3 4 -1 2 1 -5 4 ]) ; Int -> 6 (subarray [4 -1 2 1])
    (max-subarray [ 1 2 3 ]) ; Int -> 6
    (max-subarray [ -3 -2 -1 ]) ; Int -> -1
]"#,
                "[6 6 -1]",
            ),
            (
                r#"(let interleave (lambda a b (<| (std/vector/zipper a b) (std/vector/flat-one))))
(let ints (lambda xs (std/vector/map xs std/convert/integer->string)))
; examples
[
 (interleave [ "a" "b" "c" ] (ints [ 1 2 3 ])) ; [ "a" 1 "b" 2 "c" 3 ]
 (interleave (ints [ 1 2 ]) [ "x" "y" "z" ])  ; [ 1 "x" 2 "y" "z" ]
]"#,
                "[[a 1 b 2 c 3] [1 x 2 y]]",
            ),
            (
                r#"(let fn (lambda [ a b ] [ x y ] (std/int/manhattan-distance a b x y)))
(fn [ 1 2 ] [ 3 4 ])"#,
                "4",
            ),
            (
                r#"
(let N 9)
(let matrix (<| (std/vector/int/zeroes N) (std/vector/map (lambda x (std/vector/map (std/vector/int/zeroes N) (lambda _ 0))))))
(let add-glider! (lambda matrix y x (do 
  (set! (get matrix (+ y 2)) (+ x 1) 1)
  (set! (get matrix (+ y 2)) (+ x 2) 1)
  (set! (get matrix (+ y 2)) (+ x 3) 1)
  (set! (get matrix (+ y 1)) (+ x 3) 1)
  (set! (get matrix (+ y 0)) (+ x 2) 1)
  )))
(add-glider! matrix 0 0)

; (set! (get matrix 6) 2 1)
; (set! (get matrix 5) 4 1)
; (set! (get matrix 5) 3 1)
; (set! (get matrix 3) 3 1)

(let gof (lambda matrix (do
  (std/vector/map/i matrix (lambda arr y (do
    (std/vector/map/i arr (lambda cell x (do
      (let score (std/vector/3d/sliding-adjacent-sum matrix std/vector/3d/moore-neighborhood y x N +))
      (cond 
        (and (= cell 1) (or (< score 2) (> score 3))) 0
        (and (= cell 1) (or (= score 2) (= score 3))) 1
        (and (= cell 0) (= score 3)) 1
        0))))))))))
(let render (lambda matrix 
                  (do (<| matrix 
                      (std/vector/map (lambda y 
                        (std/vector/map y (lambda x (cond 
                                                (= x 0) "." 
                                                (= x 1) "*"
                                                ""))))) 
                              (std/convert/vector/3d->string std/char/new-line std/char/space)))))
(<| matrix (gof) (gof) (gof) (gof) (gof) (gof) (gof) (gof))"#,
                "[[0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0] [0 0 0 0 1 0 0 0 0] [0 0 0 0 0 1 0 0 0] [0 0 0 1 1 1 0 0 0] [0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0]]",
            ),
            (
                r#"(let *RES* 51)
(integer generation 0)
(variable cells (std/vector/int/zeroes *RES*))
(let ruleset [ 0 1 0 1 1 0 1 0 ])
(set! (get cells) (/ (length (get cells)) 2) 1)
(let out [])

(let rules (lambda a b c (do 
    (let index (std/convert/bits->integer [ a b c ]))
    (get ruleset (- 7 index)))))
(let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))
(while (< (get generation) (/ *RES* 2)) (do 
    (std/vector/push! out (get cells))
    (let nextgen (std/vector/copy (get cells)))
    (loop 1 (- (length (get cells)) 1) (lambda i (do 
        (let left (get cells 0 (- i 1)))
        (let me (get cells 0 i))
        (let right (get cells 0 (+ i 1)))
        (set! nextgen i (rules left me right)))))
    (&alter! cells nextgen)
    (&alter! generation (+ (&get generation) 1))))


(<| out 
        (std/vector/map (lambda y 
            (std/vector/map y (lambda x (cond 
                                    (= x 0) "." 
                                    (= x 1) "*"
                                    "")))))
                (std/convert/vector/3d->string std/char/new-line std/char/space))
out                
                
                "#,
                "[[0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0] [0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0] [0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0] [0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0] [0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0] [0 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 0] [0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0]]",
            ),
            (
                r#"; The Document indicates that you should start at the given coordinates (where you just landed) and face North. Then, follow the provided sequence: either turn left (L) or right (R) 90 degrees, then walk forward the given number of blocks, ending at a new intersection.
; Following R2, L3 leaves you 2 blocks East and 3 blocks North, or 5 blocks away.
; R2, R2, R2 leaves you 2 blocks due South of your starting position, which is 2 blocks away.
; R5, L5, R5, R3 leaves you 12 blocks away.

(let parse (lambda input 
    (|> 
        input 
        (Vector/cons [std/char/comma])
        (String->Vector std/char/space)
        (map (lambda x (drop/last 1 x)))
        (map (lambda [ D . M ] [ (Char->Int D) (std/convert/chars->integer M) ])))))
(let delta/pairs (lambda [ y x ] (+ (abs y) (abs x))))
(let part1 (lambda input (|> input
    (reduce (lambda [ y x a ] [ D M ] (do
                                (let F (mod (+ a (if (=# (Int->Char D) 'R') 1 3)) 4))
                                (cond 
                                    (= F 0) [y (+ x M) F]
                                    (= F 1) [(- y M) x F]
                                    (= F 2) [y (- x M) F]
                                    (= F 3) [(+ y M) x F]
                                    [ y x a ])))
                                [ 0 0 1 ])
    (delta/pairs))))

; Then, you notice the instructions continue on the back of the Recruiting Document. Easter Bunny HQ is actually at the first location you visit twice.
; For example, if your instructions are R8, R4, R4, R8, the first location you visit twice is 4 blocks away, due East.
; How many blocks away is the first location you visit twice?
(let turn
  (lambda facing D
    (mod (+ facing (if (=# (Int->Char D) 'R') 1 3)) 4)))
(let step
  (lambda y x facing
    (cond
      (= facing 0) [(+ y 1) x]
      (= facing 1) [y (+ x 1)]
      (= facing 2) [(- y 1) x]
      (= facing 3) [y (- x 1)]
      [])))
(let point->key
  (lambda y x
    (cons (Integer->String y) "," (Integer->String x))))
(let part2
  (lambda input
    (do
      (integer y 0)
      (integer x 0)
      (integer facing 0) ; North

      (let visited (buckets 128))
      (Table/set! visited (point->key (get y) (get x)) true)

      (integer result 0)

      (for (lambda [ D M ]
          (if (= (get result) 0)
              (do
                (&alter! facing (turn (get facing) D))
                (loop 0 M
                  (lambda _
                    (if (= (get result) 0)
                        (do
                          (let p (step (get y) (get x) (get facing)))
                          (&alter! y (get p 0))
                          (&alter! x (get p 1))

                          (let key (point->key (get y) (get x)))
                          (if (Table/has? key visited)
                              (&alter! result (+ (std/int/abs (get y))
                                             (std/int/abs (get x))))
                              (Table/set! visited key true))))))))) input)

      (get result))))

{
  (|> ["R2, L3"  "R2, R2, R2" "R5, L5, R5, R3"] (map parse) (map part1))
  (part2 (parse "R8, R4, R4, R8"))
}"#,
                "{ [5 2 12] 4 }",
            ),
            (
                r#"(let INPUT
"7 6 4 2 1
1 2 7 8 9
9 7 6 2 1
1 3 2 4 5
8 6 4 4 1
1 3 6 7 9")

(let parse (lambda input (<| input (std/vector/char/lines) (std/vector/map (lambda l (<| l (std/vector/char/words) (std/vector/map std/convert/chars->integer)))))))

(let part1 (lambda input (<| input 
    (std/vector/filter (lambda line (do
        (let slc (<| line 
                       (std/vector/drop/last 1)
                       (std/vector/zipper (std/vector/drop line 1))
                       (std/vector/map std/vector/int/pair/sub)))
        ; The levels are either all increasing or all decreasing.
        ; Any two adjacent levels differ by at least one and at most three.
        (or (std/vector/every? slc (lambda x (and (>= x 1) (<= x 3)))) 
            (std/vector/every? slc (lambda x (and (<= x -1) (>= x -3))))))))
    (length))))

(let part2 (lambda input (<| input
                            (std/vector/map
                              (lambda line (<| line
                                (std/vector/map/i (lambda _ i
                                  (<| line (std/vector/filter/i (lambda _ j (not (= i j))))))))))
                            (std/vector/count-of (lambda x (std/int/positive? (part1 x)))))))

(let PARSED (parse INPUT))

[(part1 PARSED) (part2 PARSED)]"#,
                "[2 4]",
            ),
            (
                r#"(let fn (lambda { a b } (do 
(if (= b 1) [ a  ] [ false ])
b
)))
(fn { true 3 })"#,
                "3",
            ),
            (
                r#"(std/vector/tuple/zip { [ 1 2 3 ] [ true false true ] })"#,
                "[{ 1 true } { 2 false } { 3 true }]",
            ),
            (
                r#"(std/vector/tuple/unzip 
    (std/vector/tuple/zip { [ 1 2 3 ] [ true false true ] })
)"#,
                "{ [1 2 3] [true false true] }",
            ),
            (
                r#"(letrec rec (lambda { x y } { _ b } (if (< (+ x y) b) (rec {(+ x 2) (+ y 3)} { true b } ) { false (+ x y) })))
(rec { 1 1 } { true 10 })"#,
                "{ false 12 }",
            ),
            (
                r#"(let factorial! (lambda N (snd (pull! (Rec { N 1 } (lambda { n acc }
                  (if (= n 0)
                      { Rec/return [ { n acc } ]}
                      { Rec/push [ { (- n 1) (* acc n) } ] })))))))

            (let rec-sum (lambda N (snd (get (Rec { N 0 } (lambda { n acc }
                (if (= 0 n)
                    { Rec/return [ { n acc } ] }
                    { Rec/push [ { (- n 1) (+ acc n) } ] })))))))

            (let factorialVec! (lambda N (first (pull! (Rec [ 1 N ] (lambda [ acc n ]
                  (if (= n 0)
                      { Rec/return [ [ acc n ] ]}
                      { Rec/push [ [ (* acc n) (- n 1) ] ] })))))))

            (factorialVec! 5)

            [(factorial! 5) (factorialVec! 5) (rec-sum 10)]"#,
                "[120 120 55]",
            ),
            (
                r#"(let INPUT "58:5,3,7,8,9,10,4,5,7,8,8")
(let parse (lambda input (do 
  (let parts (|> input (String->Vector ':')))
  (let head (|> (car parts) (Chars->Integer)))
  (let tail (|> (cdr parts) (car) (String->Vector ',') (map Chars->Integer)))
  { head tail })))

(let part1 (lambda { _ nums } (do 
(let sword [[-1 (get nums 0) -1]])
(loop 1 (length nums) (lambda i (do
  (let num (get nums i))
  (boolean placed false)
  (loop 0 (length sword) (lambda j (do 
    (let segment (get sword j))
    (cond
        (and (false? placed) (< num (get segment 1)) (= (get segment 0) -1)) (do (set! segment 0 num) (&alter! placed true))
        (and (false? placed) (> num (get segment 1)) (= (get segment 2) -1)) (do (set! segment 2 num) (&alter! placed true))
        nil))))
    (if (false? placed) (push! sword [-1 num -1])))))
sword)))
  
(part1 (parse INPUT))"#,
                "[[3 5 7] [4 8 9] [5 10 -1] [-1 7 8] [-1 8 -1]]",
            ),
            (
                r#"(let scan/sum! (lambda a b (push! a (+ (last a) b))))

(let left-right-sum-diff (lambda input 
  (|> (zip {
        (|> input (reduce (lambda a b (do (scan/sum! a b) a)) [0]) (drop/last 1))
        (|> input (reverse) (reduce (lambda a b (do (scan/sum! a b) a)) [0]) (drop/last 1) (reverse))
    }) 
    (map (lambda { a b } (abs (- a b)))))))

(left-right-sum-diff [ 10 4 8 3 ])"#,
                "[15 1 11 22]",
            ),
            (
                r#"(let maximum/count (lambda xs 
      (max 
        (count positive? xs) 
        (count negative? xs))))
        
(maximum/count [ -1 -1 1 1 1 -1 1 ])"#,
                "4",
            ),
            (
                r#";    halve :: [T] -> {[T] * [T]}
(let halve (lambda xs (do 
          (let half (/ (length xs) 2)) 
          { (take/first half xs) (drop/first half xs)})))

(halve (range 0 11))"#,
                "{ [0 1 2 3 4 5] [6 7 8 9 10 11] }",
            ),
            (
                r#"(let max-depth (lambda s (|> s 
  (filter (lambda x (or (=# x '(') (=# x ')'))))
  (map (lambda x (if (=# x '(') 1 -1)))
  (scan +)
  (maximum))))
(max-depth "(1 + (2 * 3) + ((8)/4))+1")"#,
                "3",
            ),
            (
                r#"(let A { 0 { 42 { false { "" nil } } } })
(let B { 1 { 0 { true { "" nil } } } })
(let C { 2 { 0 { false { "algebraic data types" nil } } } })

(let algebraic (lambda T (do 
  (let out [])
  (let kind (fst T))
  (cond 
    (= kind 0) (push! out (* (fst (snd T)) 10))
    (= kind 1) (if (fst (snd (snd T))) (push! out -1))
    (= kind 2) (push! out (count/char 'a' (fst (snd (snd (snd T)))))) 
  nil)
  out)))

(algebraic C)"#,
                "[4]",
            ),
            (
                r#"(let INPUT "L68
L30
R48
L5
R60
L55
L1
L99
R14
L82")
; (let INPUT "R1000")
(let parse (lambda xs 
  (|> xs 
    (String->Vector nl)
    (map (lambda [d . r] 
      [(if (=# d 'L') -1 1) (Chars->Integer r)])))))

(let part1 (lambda xs 
  (snd (reduce (lambda { dial counter } [d r] (do 
      (let res (emod (+ dial (* d r)) 100))
      { res (+ counter (Bool->Int (= res 0))) })) { 50 0 } xs))))

(let part2 (lambda xs 
  (snd (reduce (lambda { dial counter } [d r] (do 
      (let res (emod (+ dial (* d r)) 100))
      (let rng [dial])
      (loop 1 r (lambda i (push! rng (emod (+ (at rng -1) (* 1 d)) 100)))) 
      { res (+ counter (count/int 0 rng)) })) { 50 0 } xs))))

[ (part1 (parse INPUT)) (part2 (parse INPUT)) ]"#,
                "[3 6]",
            ),
            (
                r#"(let INPUT "987654321111111
811111111111119
234234234234278
818181911112111")

(let parse (String->Vector nl))
(let part1 (lambda parsed (do 
  (integer S 0)
  (|> parsed (for (lambda inp (do
    (integer M -infinity)
    (loop 0 (length inp) (lambda i 
      (loop i (length inp) (lambda j 
        (if (<> i j) 
          (&alter! M (max (get M) (Chars->Integer [(get inp i) (get inp j)]))))))))
    (&alter! S (+ (&get S) (get M))))))) 
    (get S))))

(let part2 (lambda parsed (do
  (variable S [0])
  (for (lambda line (do 
    (let N (length line))
    (let stack [])
    (loop 0 N (lambda i (do
      (while (and (not (empty? stack)) (<# (at stack -1) (get line i)) (> (+ (length stack) (- N i)) 12)) (pop! stack))
      (push! stack (get line i))
      (while (> (length stack) 12) (pop! stack)))))
    (&alter! S (BigInt/add (get S) (BigInt/new stack))))) parsed)
  (get S))))


{ (part1 (parse INPUT)) (part2 (parse INPUT)) }"#,
                "{ 357 [3 1 2 1 9 1 0 7 7 8 6 1 9] }",
            ),
            (
                r#"(let INPUT 
"..@@.@@@@.
@@@.@.@.@@
@@@@@.@.@@
@.@@@@..@.
@@.@@@@.@@
.@@@@@@@.@
.@.@.@.@@@
@.@@@.@@@@
.@@@@@@@@.
@.@.@@@.@.")
(let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))
(let parse (lambda input (|> input (String->Vector nl) (map (lambda x (map (lambda x (if (=# x '@') 1 0)) x))))))
(let part1 (lambda input (do
  (integer TOTAL 0)
  (loop 0 (length input) (lambda y (do 
    (loop 0 (length (get input 0)) (lambda x (if (= (get input y x) 1) (do 
      (integer SUM 0)
      (neighborhood neighborhood/moore y x (lambda cell dir y x (&alter! SUM (+ (&get SUM) cell))) input)
      (if (< (get SUM) 4) (&alter! TOTAL (+ (&get TOTAL) 1))))))))))
  (get TOTAL))))

  (let part2 (lambda input (do
    (letrec rec (lambda total (do
        (let rem [])
        (loop 0 (length input) (lambda y (do 
        (loop 0 (length (get input 0)) (lambda x (if (= (get input y x) 1) (do 
        (integer ACC 0)
        (neighborhood neighborhood/moore y x (lambda cell dir y x (&alter! ACC (+ (&get ACC) cell))) input)
        (if (< (get ACC) 4) (push! rem [ y x ])))))))))
        (if (empty? rem) total (do 
        (for (lambda [y x] (set! (get input y) x 0)) rem)
        (rec (+ total (length rem))))))))
    (rec 0))))

[(part1 (parse INPUT)) (part2 (parse INPUT))]"#,
                "[13 43]",
            ),
            (
                r#"(let INPUT "3-5
10-14
16-20
12-18
*
1
5
8
11
17
32")

(let parse (lambda input (do 
  (let [ p1 p2 ] (String->Vector '*' input))
  (let A (drop/last 1 p1))
  (let B (drop/first 1 p2))
  { (map (lambda x (map BigInt/new (String->Vector '-' x))) (String->Vector nl A)) (map BigInt/new (String->Vector nl B)) })))

(let part1 (lambda { ranges fruits } (length (filter (lambda fruit (some? (lambda [ low high ] (and (BigInt/gte? fruit low) (BigInt/lte? fruit high))) ranges)) fruits))))

(let part2 (lambda { rng _ } (do
  (let ranges (sort (lambda [ a ] [ b  ] (BigInt/lt? a b)) rng))
  (variable low (get ranges 0 0))
  (variable high (get ranges 0 1))
  (variable out [ 0 ])
  (loop 1 (length ranges) (lambda i (do 
    (let [ dlow dhigh ] (get ranges i))
    (if (BigInt/gte? (get high) dlow) (do 
      (&alter! low (if (BigInt/lt? (get low) dlow) (get low) dlow))
      (&alter! high (if (BigInt/gt? (get high) dhigh) (get high) dhigh))) (do 
      (&alter! out (BigInt/add (get out) (BigInt/add (BigInt/sub (get high) (get low)) [ 1 ])))
      (&alter! low (get ranges i 0))
      (&alter! high (get ranges i 1)))))))
  (&alter! out (BigInt/add (get out) (BigInt/add (BigInt/sub (get high) (get low)) [ 1 ])))
  (get out))))

(let PARSED (parse INPUT))

[[(part1 PARSED)] (part2 PARSED)]"#,
                "[[3] [1 4]]",
            ),
            (
                r#"(let INPUT 
"123 328  51 64 
 45 64  387 23 
  6 98  215 314
*   +   *   +  ")
(let parse (lambda input (do 
  (let groups (|> input (String->Vector nl) (map (lambda x (|> x (String->Vector ' ') (filter (lambda x (not (empty? x)))))))))
  (let op (map first (last groups)))
  (pop! groups)
  (let ints (map (lambda x (map BigInt/new x)) groups))
  (let numbers (reduce (lambda a i (do (push! a (map (lambda n (get n i)) ints)) a)) [] (range 0 (- (length (get ints 0)) 1))))
  {numbers op }
  )))

(let part1 (lambda { numbers op } (reduce (lambda a { op i } 
    (BigInt/add a 
      (cond 
        (=# op '*') (reduce BigInt/mul [1] (get numbers i))
        (=# op '+') (reduce BigInt/add [0] (get numbers i))
        []
      ))) [0] (zip { op (range 0 (length op)) }))))
(part1 (parse INPUT))"#,
                "[4 2 7 7 5 5 6]",
            ),
            (
                r#"
(let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))                
(let INPUT 
".......S.......
...............
.......^.......
...............
......^.^......
...............
.....^.^.^.....
...............
....^.^...^....
...............
...^.^...^.^...
...............
..^...^.....^..
...............
.^.^.^.^.^...^.
...............")
(let parse (lambda input (String->Vector nl input)))
(let part1 (lambda input (do
  (integer total 0)
  (let visited [[] [] [] [] [] [] [] [] []])
  (let queue (Que/new [Int]))
  (let start (points (lambda x (=# x 'S')) input))
  (Que/enque! queue (get start 0))
  (while (Que/not-empty? queue) (do 
    (let current (Que/peek queue))
    (Que/deque! queue)
    (let [ y x ] current)
    (let key (cons (Integer->String y) "-" (Integer->String x)))
    (if (and (std/vector/3d/in-bounds? input y x) (not (Set/has? key visited))) (do
        (Set/add! visited key)
        (if (=# (get input y x) '^') (do 
          (&alter! total (+ (&get total) 1))
          (Que/enque! queue [ y (+ x 1) ])
          (Que/enque! queue [ y (- x 1) ])) (Que/enque! queue [ (+ y 1) x ]))))))
  (get total))))
(let part2 (lambda input (do
  (integer total 0)
  (let queue (Que/new [Int]))
  (let start (first (points (lambda x (=# x 'S')) input)))
  (Que/enque! queue [(get start 0) (get start 1) 1])
  (while (Que/not-empty? queue) (do 
    (let current (Que/peek queue))
    (Que/deque! queue)
    (let [ y x c ] current)
    (if (std/vector/3d/in-bounds? input y x) (do
        (if (=# (get input y x) '^') (do 
            (Que/enque! queue [ y (+ x 1) c ])
            (Que/enque! queue [ y (- x 1) c ])) 
            (Que/enque! queue [ (+ y 1) x c ])))
        (&alter! total (+ (&get total) c)))))
  (get total))))
(let PARSED (parse INPUT))
[(part1 PARSED) (part2 PARSED)]"#,
                "[21 40]",
            ),
            (
                r#"(let INPUT 
".......S.......
...............
.......^.......
...............
......^.^......
...............
.....^.^.^.....
...............
....^.^...^....
...............
...^.^...^.^...
...............
..^...^.....^..
...............
.^.^.^.^.^...^.
...............")
(let parse (lambda input (String->Vector nl input)))
(let solution (lambda input (do
  (let data (map (lambda x (map identity x)) input))
  (variable beam [ 0 ])
  (let timeline (map (lambda _ [ 0 ]) (zeroes (length (get data 0)))))
  (loop 0 (length data) (lambda y (do 
    (let line (get data y))
    (loop 0 (length line) (lambda x (do 
      (let c (get line x))
      (cond 
        (=# c 'S') (do 
          (set! (get data (+ y 1)) x '|')
          (set! timeline x [ 1 ]))
        (=# c '^') (if (and (> (- y 1) 0) (=# (get data (- y 1) x) '|')) (do 
          (set! (get data y) (- x 1) '|')
          (set! (get data y) (+ x 1) '|')
          (&alter! beam (BigInt/add (get beam) [ 1 ]))
          (set! timeline (- x 1) (BigInt/add (get timeline (- x 1)) (get timeline x)))
          (set! timeline (+ x 1) (BigInt/add (get timeline (+ x 1)) (get timeline x)))
          (set! timeline x [ 0 ])
        ))
        (=# c '.') (if (and (> (- y 1) 0) (=# (get data (- y 1) x) '|')) (do 
          (set! (get data y) x '|'))))))))))
  [(get beam) (BigInt/sum timeline)])))

(solution (parse INPUT))"#,
                "[[2 1] [4 0]]",
            ),
            (r#"[ (floor 1.23) (ceil 14.235) (ceil -1.2) ]"#, "[1 15 -1]"),
            (
                r#"(let INPUT "162,817,812
57,618,57
906,360,560
592,479,940
352,342,300
466,668,158
542,29,236
431,825,988
739,650,466
52,470,668
216,146,977
819,987,18
117,168,530
805,96,715
346,949,466
970,615,88
941,993,340
862,61,35
984,92,344
425,690,689")
(let distance/3d (lambda [ x1 y1 z1 ] [ x2 y2 z2 ] (+ (square (- x2 x1)) (square (- y2 y1)) (square (- z2 z1)))))
(let parse (lambda input (|> input (String->Vector nl) (map (lambda x (|> x (String->Vector ',') (map Chars->Integer)))))))
(let part1 (lambda input (do
  (let len (length input))
  (let dist [])
  (loop 0 len (lambda i (do 
    (loop i len (lambda j (if (<> i j) 
      (push! dist { [ i j ] (abs (distance/3d (get input i) (get input j))) })))))))
  (sort! dist (lambda { _ d1 } { _ d2 } (< d1 d2)))
  (let edges (map fst dist))
  (let parent (range 0 (- (length input) 1)))
  (letrec root (lambda i (if (= (get parent i) i) i (root (get parent i)))))
  (let merge (lambda a b (set! parent (root a) (root b))))
  (for (lambda [ a b ] (merge a b)) (take/first 10 edges))
  (|> 
    (range 0 (- (length input) 1))
    (reduce (lambda a b (do 
      (let i (root b))
      (set! a i (+ (get a i) 1))
      a)) 
    (zeroes (length input)))
    (sort >)
    (take/first 3)
    (product)))))

(let part2 (lambda input (do
  (let len (length input))
  (let dist [])
  
  ; compute all pairwise distances
  (loop 0 len (lambda i
    (loop (+ i 1) len (lambda j
      (push! dist { [i j] (distance/3d (get input i) (get input j)) })))))

  ; sort edges by distance
  (sort! dist (lambda { _ d1 } { _ d2 } (< d1 d2))) 
  (let edges (|> dist (map fst)))

  ; initialize union-find
  (let parent (range 0 len))
  (let size (map (lambda i 1) parent))
  (integer components len)

  ; root with path compression
  (letrec root (lambda i
      (if (= (get parent i) i) i
          (do
            (set! parent i (root (get parent i)))
            (get parent i)))))

  ; merge with size tracking
  (let merge (lambda a b (do
        (let ra (root a))
        (let rb (root b))
        (if (<> ra rb) (do
              (if (< (get size ra) (get size rb))
                  (do
                    (set! parent ra rb)
                    (set! size rb (+ (get size rb) (get size ra))))
                  (do
                    (set! parent rb ra)
                    (set! size ra (+ (get size ra) (get size rb)))))
              (&alter! components (- (&get components) 1))
              true)
            false))))

  ; walk edges until all connected
  (integer answer 0)
  (for (lambda [ a b ]
      (if (and (= (get answer) 0) (merge a b) (= (get components) 1))
                (&alter! answer (* (get (get input a) 0) (get (get input b) 0))))) edges)
  (get answer))))

[(part1 (parse INPUT)) (part2 (parse INPUT))]"#,
                "[40 25272]",
            ),
            (
                r#"(let INPUT "7,1
11,1
11,7
9,7
9,5
2,5
2,3
7,3")
(let parse (lambda input (|> input (String->Vector nl) (map (lambda x (|> x (String->Vector ',') (map Chars->Integer)))))))
(let part1 (lambda input (do
  (let pairs (std/vector/unique-pairs input))
  (let rect (lambda [ x1 y1 ] [ x2 y2 ] (* (+ 1 (abs (- x1 x2))) (+ 1 (abs (- y1 y2))))))
 (|> pairs (map (lambda [ a b ] (rect a b))) (maximum)))))

[(part1 (parse INPUT))]"#,
                "[50]",
            ),
            (
                "{  (std/vector/dec/mean (range/dec 1 10)) (Dec->Int (std/vector/dec/mean (range/dec 1 10))) }",
                "{ 5.5 5 }",
            ),
            (
                "{ (map Dec->Int (range/dec 1 10)) (map Int->Dec (range/int 1 10)) }",
                "{ [1 2 3 4 5 6 7 8 9 10] [1 2 3 4 5 6 7 8 9 10] }",
            ),
            (
                "(=? 
    (=
        (Dec->Int 
        (sum/dec (range/dec 0 10)))
        (sum/int (range/int 0 10)))
    (=.
        (Int->Dec 
        (sum/int (range/int 0 10)))
        (sum/dec (range/dec 0 10))))",
                "true",
            ),
            (
                r#"(let INPUT "ULL
RRDDD
LURDL
UUUUD")

(let parse (String->Vector nl))
(let part1 (lambda input (do 
  (let pad [['1' '2' '3'] ['4' '5' '6'] ['7' '8' '9']])
  (let len (- (length pad) 1))
  (let start (first (points (lambda x (=# x '5')) pad)))
  (let L [ 0 -1 ])
  (let R [ 0 1 ])
  (let U [ -1 0 ])
  (let D [ 1 0 ])
  (let N [ 0 0 ])
  (|> input (map (lambda x 
        (|> x (map (lambda y 
            (cond (=# y 'U') U
                  (=# y 'D') D
                  (=# y 'L') L
                  (=# y 'R') R
                          N)))
            (reduce (lambda a dir (do 
              (set! start 0 (clamp-range 0 len (+ (get dir 0) (get start 0))))
              (set! start 1 (clamp-range 0 len (+ (get dir 1) (get start 1))))
              (get pad (get start 0) (get start 1)))) '0'))))))))
(let part2 (lambda input (do 
  (let pad [['*' '*' '1' '*' '*'] ['*' '2' '3' '4' '*'] ['5' '6' '7' '8' '9'] ['*' 'A' 'B' 'C' '*'] ['*' '*' 'D' '*' '*']])
  (let len (- (length pad) 1))
  (let start (first (points (lambda x (=# x '5')) pad)))
  (let L [ 0 -1 ])
  (let R [ 0 1 ])
  (let U [ -1 0 ])
  (let D [ 1 0 ])
  (let N [ 0 0 ])
  (|> input (map (lambda x 
        (|> x (map (lambda y 
            (cond (=# y 'U') U
                  (=# y 'D') D
                  (=# y 'L') L
                  (=# y 'R') R
                          N)))
            (reduce (lambda a dir (do 
              (let y (+ (get dir 0) (get start 0)))
              (let x (+ (get dir 1) (get start 1)))
              (unless (and (std/vector/3d/in-bounds? pad y x) (=# (get pad y x) '*')) (do 
                 (set! start 0 (clamp-range 0 len y))
                 (set! start 1 (clamp-range 0 len x))))
              (get pad (get start 0) (get start 1)))) '0'))))))))
[(part1 (parse INPUT)) (part2 (parse INPUT))]"#,
                "[1985 5DB3]",
            ),
            (
                r#"(let parse (lambda input (|> input (String->Vector nl) (map (lambda x (|> x (String->Vector ' ') (map (lambda x (filter digit? x))) (filter not-empty?) (map String->Integer)))))))
(let part1 (lambda input (|> input (count (lambda [a b c] (and (> (+ a b) c) (> (+ b c) a) (> (+ a c) b)))))))
(let part2 (lambda input (|> input 
  (reduce (lambda a [A B C] (do 
    (let [ pa pb pc ] (at a -1))
    (push! pa A)
    (push! pb B)
    (push! pc C)
    (if (= (length pa) 3) (push! a [[] [] []]))
   a)) [[[] [] []]])
  (filter (lambda x (not (empty? (get x 0)))))
  (flat)
  (part1))))

[
  (part1 (parse "5 10 25
    330  143  338
    769  547   83")) 
  (part2 (parse "  330  143  5
    769  547   10
    930  625  25"))
]"#,
                "[1 2]",
            ),
            (
                r#"(let A (Vector->Set ["Hello" "Darkness" "My" "Old" "Friend"]))
(let B (Vector->Set ["Hello" "Darkness" "My" "New" "Enemy"]))

(|> [
  (Set/intersection A B)
  (Set/difference A B)
  (Set/difference B A)
  (Set/xor A B)
  (Set/union A B)
] 
 (map Set/values)
 (reduce cons [])
 (reduce cons [])
 (map box)
 (Table/count)
 (Table/entries)
 (sort (lambda { _ a } { _ b } (> a b)))
 (car)
 (fst))"#,
                "e",
            ),
            (
                r#"(let flood-fill! (lambda image sr sc color (do 
    (let old (get image sr sc))
    (unless (= old color) 
        (do 
        (let m (length image))
        (let n (length (first image)))
        (letrec adj (lambda r c (if (and (>= r 0) (< r m) (>= c 0) (< c n) (= (get image r c) old)) (do 
                    (set! image r c color)
                    (adj (+ r 1) c)
                    (adj (- r 1) c)
                    (adj r (+ c 1))
                    (adj r (- c 1))
                nil))))
        (adj sr sc)
        nil)))))

(let image [[1 1 1] [1 1 0] [1 0 1]])
(flood-fill! image 1 1 2)
image
; Output/ [[2 2 2] [2 2 0] [2 0 1]]"#,
                "[[2 2 2] [2 2 0] [2 0 1]]",
            ),
            (
                r#"(letrec fibonacci (lambda n
    (if (< n 2) n (+ (fibonacci (- n 1)) (fibonacci (- n 2))))))
(fibonacci 10)"#,
                "55",
            ),
            (
                r#"(let memo [[] [] [] []])
(letrec fibonacci! (lambda n
    (do 
    (let key (Integer->String n))
    (if (< n 2) n (if (Table/has? key memo) (snd (first (Table/get key memo))) (do 
      (let res (+ (fibonacci! (- n 1)) (fibonacci! (- n 2))))
      (Table/set! memo key res)
      res))))))
(fibonacci! 10)"#,
                "55",
            ),
            (
                r#"(letrec ackermann (lambda m n 
    (cond 
        (and (< m 0) (< n 0)) -1 
        (= m 0) (+ n 1)
        (and (> m 0) (= n 0)) (ackermann (- m 1) 1)
        (and (> m 0) (> n 0)) (ackermann (- m 1) (ackermann m (- n 1)))
        0)))

(ackermann 2 3)"#,
                "9",
            ),
            (
                r#"(let INPUT "00100
11110
10110
10111
10101
01111
00111
11100
10000
11001
00010
01010")

(let parse
  (lambda input
    (|> input
        (String->Vector nl)
        (map (lambda row
               (map Char->Digit row))))))

(let PARSED (parse INPUT))

(let part1 (lambda input (do 
  (let Matrix->Count (lambda sig matrix (|> matrix (map (lambda xs (> (count/int (- 1 (- 1 sig)) xs) (count/int (- 1 sig) xs)))) (map Bool->Int) (std/convert/bits->integer))))
  (let matrix (std/vector/3d/rotate input))
  (let gamma (Matrix->Count 1 matrix))
  (let epsilon (Matrix->Count 0 matrix))
  (* gamma epsilon))))
  
  (let count-bit
  (lambda rows idx bit
    (count/int
      bit
      (map (lambda r (get r idx)) rows))))

(let find-rating
  (lambda rows prefer-most? (do
    (let width (length (get rows 0)))

    (letrec step
      (lambda idx remaining
        (if (or (= (length remaining) 1) (= idx width))
            remaining
            (do
              (let ones  (count-bit remaining idx 1))
              (let zeros (count-bit remaining idx 0))

              (let keep-bit
                (if prefer-most?
                    ;; oxygen
                    (if (>= ones zeros) 1 0)
                    ;; CO2
                    (if (<= zeros ones) 0 1)))

              (step
                (+ idx 1)
                (filter (lambda r (= (get r idx) keep-bit)) remaining)
                )))))

    (get (step 0 rows)))))

(let part2
  (lambda input (do
    (let oxygen-bits (find-rating input true))
    (let co2-bits    (find-rating input false))
    (* (std/convert/bits->integer oxygen-bits)
       (std/convert/bits->integer co2-bits)))))


[(part1 PARSED) (part2 PARSED)]"#,
                "[198 230]",
            ),
            (
                r#"(let INPUT 
"r, wr, b, g, bwu, rb, gb, br

brwrr
bggr
gbbr
rrbgbr
ubwu
bwurrg
brgr
bbrgwb")

(let parse (lambda input (do
    (let lines (|> input (String->Vector nl)))
    {
      (|> lines (first) (String->Vector ',') (map (lambda xs (filter (lambda x (not (=# x ' '))) xs))))
      (|> lines (drop/first 2))
    })))

(let part1 (lambda { patterns-input towels } (do 
  (let patterns (reduce (lambda a b (do (Set/add! a b) a)) [[] [] [] [] [] [] [] [] []] patterns-input))
  (letrec dp? (lambda str (loop/some-range? 1 (length str) (lambda i (do 
    (let a (slice 0 i str))
    (let b (slice i (length str) str))
    (or (and (Set/has? a patterns) (Set/has? b patterns)) (and (dp? a) (dp? b))))))))

  (count dp? towels))))

[(part1 (parse INPUT))]"#,
                "[6]",
            ),
            (
                r#"
(let res (lambda x y 
(std/vector/option/resolve [ 
    (std/int/div/option (+ 1 2 x) y)
    (std/true/option (* 4 5 x))
    (std/int/sqrt/option x)
  ] 
  (lambda [ a b c ] (+ a b c))
  -1)
))

[(res 234 25) (res -1 25) (res 234 0)]
"#,
                "[{ true 4704 } { false -1 } { false -1 }]",
            ),
            (
                r#"
(let INPUT "7,4,9,5,11,17,23,2,0,14,21,24,10,16,13,6,15,25,12,22,18,20,8,19,3,26,1

22 13 17 11  0
 8  2 23  4 24
21  9 14 16  7
 6 10  3 18  5
 1 12 20 15 19

 3 15  0  2 22
 9 18 13 17  5
19  8  7 25 23
20 11 10 24  4
14 21 16 12  6

14 21 17 24  4
10 16 15  9 19
18  8 23 26 20
22 11 13  6  5
 2  0 12  3  7")
; [Char] -> {[Int] * [[[Int]]]}
(let parse (lambda input (do  
  (let lines (|> input (String->Vector nl) (filter not-empty?)))
  (let numbers (|> (car lines) (String->Vector ',') (map String->Integer)))
  (let boards (|> (cdr lines) (map (lambda xs (|> xs (String->Vector ' ') (filter not-empty?) (map String->Integer)))) (partition 5)))
  { numbers boards })))
; [[T]] -> [[T]]
(let board-lines
  (lambda board
    (cons board (transpose board))))
; [Int] -> [[[Char]]] -> Bool
(let line-complete?
  (lambda line drawn
    (every? (lambda x (Set/has? (Integer->String x) drawn)) line)))
; [[[Int]]] -> [[[Char]]] -> Bool
(let board-wins?
  (lambda board drawn
    (|> (board-lines board)
        (some? (lambda line
          (line-complete? line drawn))))))
; [Int] -> [[[[Int]]]] -> {Int * [[[Int]]]}
(let first-winning-board
  (lambda numbers boards (do
    (let drawn (buckets 32))
    (letrec step
      (lambda i result
        (if (or
              (not (= (fst result) -1))
              (= i (length numbers)))
            result
            (do
              (Set/add! drawn (Integer->String (get numbers i)))
              (let winners
                (filter (lambda b (board-wins? b drawn)) boards))
              (step
                (+ i 1)
                (if (empty? winners)
                    result
                    { i (get winners 0) }))))))
    (step 0 { -1 [] }))))
; [[Int]] -> [[[Char]]] -> Int -> Int
(let score
  (lambda board drawn last-number (do
    (let unmarked-sum
      (|> board
          (flat)
          (filter
            (lambda x
              (not (Set/has? (Integer->String x) drawn))))
          (sum)))
    (* unmarked-sum last-number))))
; {[Int] * [[[Int]]]} -> Int
(let part1
  (lambda { numbers boards } (do
    (let { i board } (first-winning-board numbers boards))
    (let drawn (Vector->Set (map Integer->String (slice 0 (+ i 1) numbers))))
    (score board drawn (get numbers i)))))
(part1 (parse INPUT))"#,
                "4512",
            ),
            (
                r#"
(let Point   (Id!))
(let Segment (Id!))
(let Key     (Id!))
(let None    (Id!))

; Int -> Int -> { Point * { Int * Int } }
(let make-point
  (lambda x y
    { Point { x y } }))
; Point -> Point -> { Segment * { Point * Point } }
(let make-segment
  (lambda p q
    { Segment { p q } }))

; Point -> { Key * [Char] }
(let Point->Key
  (lambda { Tag { x y } }
    (if (= Tag Point ) { Key (cons (Integer->String x) "," (Integer->String y)) } { None [] })))

; [Char] -> [Int]
(let Point->Coords (lambda p (|> p (String->Vector ',') (map String->Integer))))

; [Char] -> [Segment]
(let parse
  (lambda input
    (|> input
        (String->Vector nl)
        (filter not-empty?)
        (map
          (lambda line (do
            (let parts (String->Vector ' ' line))
            (let a (Point->Coords (get parts 0)))
            (let b (Point->Coords (get parts 2)))
            (make-segment
              (make-point (get a 0) (get a 1))
              (make-point (get b 0) (get b 1)))))))))
; Int -> Int -> [Int]
(let range*
  (lambda a b
    (if (<= a b)
        (range a b)
        (reverse (range b a)))))
; Segment -> [Point]
(let Segment->Points
  (lambda { Tag { { _ { x1 y1 } } { _ { x2 y2 } } } } 
    (cond
        ; vertical
        (= x1 x2)
        (map (lambda y (make-point x1 y)) 
             (range (min y1 y2) (max y1 y2)))

        ; horizontal
        (= y1 y2)
        (map (lambda x (make-point x y1))
             (range (min x1 x2) (max x1 x2)))

        ; diagonal (45°)
        (= (abs (- x1 x2)) (abs (- y1 y2)))
        (map (lambda { x y } (make-point x y))
             (zip { (range* x1 x2) (range* y1 y2) }))

        [])))
; [Segment] -> Int
(let solve
  (lambda segments
    (|> segments
        (map Segment->Points) ; [ [Point] ]
        (flat)                ; [Point]
        (map Point->Key)      ; [Key]
        (map snd)
        (Table/count)
        (Table/values)
        (count (lambda n (>= n 2))))))

; [Char]
(let INPUT "0,9 -> 5,9
8,0 -> 0,8
9,4 -> 3,4
2,2 -> 2,1
7,0 -> 7,4
6,4 -> 2,0
0,9 -> 2,9
3,4 -> 1,4
0,0 -> 8,8
5,5 -> 8,2")

(solve (parse INPUT))"#,
                "12",
            ),
            (
                r#"
(let unpack (lambda { _ { _ { _ { y x }}}} (+ y x)))
(let unnest (lambda [[a b c ] [_ d]] [ a b c d ]))
(let both (lambda { [ y x z ] { a b } } (if (or a b) (* (+ y x) z) z)))
{ (unnest [[ 1 2 3 ] [ 4 5 6 ]]) { (unpack { 1 { 2 { 3 { 4 5 }}}}) (both { [ 2 3 -1 ] { false true } }) } }"#,
                "{ [1 2 3 5] { 9 -5 } }",
            ),
            (
                r#"; [Char]
(let INPUT "3,4,3,1,2")
; [Char] -> [Int]
(let parse (lambda input (do
    (let timers (|> input (String->Vector ',') (map String->Integer)))
    (let counts (zeroes 9))
      (for (lambda t (set! counts t (+ 1 (get counts t)))) timers)
      counts)))
; [Int] -> [Int] ; length 9
(let step (lambda [ c0 c1 c2 c3 c4 c5 c6 c7 c8 ] [ c1 c2 c3 c4 c5 c6 (+ c7 c0) c8 c0 ]))
; Int -> [Int] -> [Int]
(let simulate (lambda days state (do 
    (letrec rec (lambda i acc
        (if (= i days)
            acc
            (rec (+ i 1) (step acc)))))
    (rec 0 state))))
; [Int] -> Int
(let part1 (lambda input (sum (simulate 80 input))))
; Int
(part1 (parse INPUT))"#,
                "5934",
            ),
            (
                r#"; [Char]
(let INPUT "\#1 @ 1,3: 4x4
#2 @ 3,1: 4x4
#3 @ 5,5: 2x2")
; "\#123" -> 123
(let parse-id
  (lambda s
    (String->Integer (drop/first 1 s))))
; "3,2" -> [3 2]
(let parse-coords
  (lambda s
    (map String->Integer (String->Vector ',' (drop/last 1 s)))))
; "5x4" -> [5 4]
(let parse-size
  (lambda s
    (map String->Integer (String->Vector 'x' s))))
; [Char] -> {Int * {Int * Int * Int * Int}}
(let parse-claim
  (lambda line
    (do
      (let parts (String->Vector ' ' line))
      (let id (parse-id (get parts 0)))
      ; (let [ x y ] (parse-coords (get parts 2)))
      ; (let [ w h ] (parse-size (get parts 3)))
      (let coords (parse-coords (get parts 2)))
      (let size (parse-size (get parts 3)))
      
      (let x (get coords 0))
      (let y (get coords 1))
      (let w (get size 0))
      (let h (get size 1))

      { id { x { y { w h } } } })))
; {Int * {Int * Int * Int * Int}} -> [[Int]]
(let claim->points
  (lambda { _ { x { y { w h } } } }
    (flat
        (map (lambda i
            (map (lambda j [ i j ]) 
                (range y (- (+ y h) 1)))) 
            (range x (- (+ x w) 1))))))

; [Int Int] -> [Char]
(let point->key
  (lambda [ x y ]
    (cons (Integer->String x) "," (Integer->String y))))

; [Char] -> Int
(let part1
  (lambda input
    (do
      (let claims
        (|> input
            (String->Vector nl)
            (filter not-empty?)
            (map parse-claim)))

      (|> claims
          (map claim->points)   ; [ [[x y]] ]
          (flat)                ; [[x y]]
          (map point->key)      ; [String]
          (Table/count)         ; { key -> Int }
          (Table/values) 
          (count (lambda n (>= n 2))))))) 

(part1 INPUT)"#,
                "4",
            ),
            (
                r#"[
(apply (comp
    (map (String->Vector '-')) 
    (map (map String->Integer)) 
    flat 
    sum) ["1-2" "3-4" "5-6"])
(|> ["1-2" "3-4" "5-6"]
    (map (String->Vector '-')) 
    (map (map String->Integer)) 
    flat 
    sum)]"#,
                "[21 21]",
            ),
            (
                r#"(let puncts ['!' ',' '.' '?' ' ' sq nl]) 
(let punct? (lambda x (some? (apply =# x) puncts)))

(let palindrome? (comp (map lower) (exclude punct?) (S/comb match? reverse)))
(|> [
  "Was it a cat I saw?"
  "No lemon, no melon"
  "No one made killer apparel like Dame Noon."
  "Go hang a salami, I'm a lasagna hog"
  "Stab nail at ill, italian bats!"
] 
(map palindrome?) 
(every? identity))"#,
                "true",
            ),
            (
                r#"; SRM 727: Problem 1 - MakeTwoConsecutive
(let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))
(let solve (lambda s (and (> (length s) 2) (do
  (let n (length s))
  (integer c 0)
  (loop 0 (- n 1) (lambda i (if (=# (get s i) (get s (+ i 1))) (&alter! c (+ (&get c) 1)))))
  (loop 0 (- n 2) (lambda i (if (=# (get s i) (get s (+ i 2))) (&alter! c (+ (&get c) 1)))))
  (> (get c) 0)))))

[
  (solve "BCAB")
  (solve "BB")
  (solve "A")
  (solve "AABB")
  (solve "BAB")
  (solve "KEEP")
]"#,
                "[false false false true true true]",
            ),
            (
                r#"(let max-water (lambda input (do 
  (letrec max-water-rec (lambda l r out 
    (if (< l r) 
      (do 
        (let width (- r l))
        (let left (get input l))
        (let right (get input r))
        (let min-height (min left right))
        (let area (* width min-height))
        (let condition (< left right))
        (max-water-rec (if condition (+ l 1) l) (unless condition (- r 1) r) (max area out))) 
     out)))
  (max-water-rec 0 (- (length input) 1) 0))))

(max-water [ 3 9 4 8 2 6 1 ])"#,
                "24",
            ),
            (
                r#"
(let fn (lambda { x y } (do 
  (let vec [ 1 2 3 ])
  (let [ a b ] vec)
      [ a b  x y ]
  )))
(let { x { y z }} { 10 { [ 1 2 3 ] false} })
(let def { true { [ 1 2 3 4 5 ] (lambda [ a . b ] (+ (sum b) a)) } })
(let { a1 { b1 c1 } } def)
{ z  { (fn { 10 23 }) { a1 (c1 b1)} } }"#,
                "{ false { [1 2 10 23] { true 15 } } }",
            ),
            (r#"(let [ a b . rest ] [ 1 2 3 4 5 6 ])
{ a rest }"#, "{ 1 [3 4 5 6] }"),
            (
                r#"(let rev (lambda xs (do 
  (letrec rec/rev (lambda xs out 
                  (if (empty? xs) out 
                      (rec/rev (cdr xs) (cons [(car xs)] out)))))
  (rec/rev xs []))))

(let xs [ 1 2 3 4 ])

[
 (rev xs)
 (rev xs)
 (rev xs)
 (rev xs)
]"#,
                "[[4 3 2 1] [4 3 2 1] [4 3 2 1] [4 3 2 1]]",
            ),

            (
                r#"
(let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))
(let INPUT "2333133121414131402")
(let parse (comp (map Char->Digit)))
(let part1 (lambda input (do 
  (integer file-id -1)
  (let disk (|> input (reduce/i (lambda disk ch i (std/vector/concat! disk 
  [(if (even? i) (do 
    (&alter! file-id (+ (&get file-id) 1))
    (let id (get file-id))
    (Vector/new (lambda _ id) ch))
    (Vector/new (lambda _ -1) ch))])) [])))
  (let blanks (reduce/i (lambda a x i (do (if (= x -1) (push! a i)) a)) [] disk))
  (letrec fragment (lambda ind out (do 
    (let i (get blanks ind))
    (if (= (last disk) -1) (do (pop! disk) (fragment ind out))
      (if (not (<= (length disk) i)) (do 
        (set! disk i (last disk))
        (pop! disk)
        (fragment (+ ind 1) out)
      ) false)))))
  (fragment 0 true)
  (|> disk (reduce/i (lambda a b i (+ a (* b i))) 0)))))
(part1 (parse INPUT))"#,
                "1928",
            ),

            (
                r#"(let parse (comp (String->Vector nl) (map String->Integer)))
(let part1 (comp
  (map (lambda secret (do
    (integer SECRET secret)
    ; Each step of the above process involves mixing and pruning:

    ; To mix a value into the secret number, 
    ; calculate the bitwise XOR of the given value and the secret number.
    ;  Then, the secret number becomes the result of that operation. 
    ; (If the secret number is 42 and you were to mix 15 into the secret number, 
    ; the secret number would become 37.)
    (let mix (lambda value (do (&alter! SECRET (^ value (get SECRET))) (get SECRET))))

    ; To prune the secret number, 
    ; calculate the value of the secret number modulo 777216. 
    ; Then, the secret number becomes the result of that operation. 
    (let prune (lambda value (do (&alter! SECRET (emod value 777216)) (get SECRET))))

    (let random (lambda (|>
          (get SECRET)

          (fp/mul 64)        ; Calculate the result of multiplying the secret number by 64.
          
          mix        ; Then, mix this result into the secret number.
          prune       ; Finally, prune the secret number.

          (fp/div 32)        ; Calculate the result of dividing the secret number by 32.
                        ; Round the result down to the nearest integer.
          
         mix         ; Then, mix this result into the secret number.
         prune       ; Finally, prune the secret number.
                
          (fp/mul 2048)      ; Calculate the result of multiplying the secret number by 2048.

          mix         ; Then, mix this result into the secret number. 
          prune       ; Finally, prune the secret number.
          )))
          
        ; In a single day, buyers each have time to generate 2000 new secret numbers
        (loop/repeat 2000 random)
        (get SECRET))))
    ; What is the sum of the 2000th secret number generated by each buyer?
    sum))
(part1 (parse "1
10
100
2024"))"#,
                "1234258",
            ),
            (
                r#"; solve :: [[Char]] -> Int
(let solve (comp 
    (map (Vector/get-unsafe 1)) 
    (map (lambda x (if (=# x '-') -1 1))) 
    sum))

(map solve [["--X" "X++" "X++"] ["++X" "++X" "X++"] ["X++" "++X" "--X" "X--"]])
; [ 1 3 0 ]
"#,
                "[1 3 0]",
            ),
            (
                r#"(let group-anagrams (comp 
    (map (lambda w { w (sort ># w)}))
    (sort (lambda { _ a } { _ b } (String/gt? a b)))
    (reduce/i (lambda a b i (do 
      (let { bw bs } b)
      (let prev (last a))
      (cond (empty? prev) (set! a 0 [b])
            (not (match? (snd (last prev)) bs)) (push! a [b])
            (push! prev b)) 
      a)) [[]])
      (map (cond (map fst)))))

(group-anagrams ["eat" "tea" "tan" "ate" "nat" "bat"])"#,
                "[[nat tan] [eat tea ate] [bat]]",
            ),

            (
                r#"(let binary-search (lambda arr target
  (do
    (let L (box 0))
    (let R (box (- (length arr) 1)))
    (let result (box -1)) ; Store index here, -1 if not found

    (while (and (<= (get L 0) (get R 0)) (= (get result 0) -1))
        (do
          (let mid (/ (+ (get L 0) (get R 0)) 2))
          (let val (get arr mid))
          (cond
            (= val target) (set! result 0 mid)
            (< val target) (set! L 0 (+ mid 1))
            (set! R 0 (- mid 1)))))
    (get result 0))))

; Usage with your sorted result
(let sorted-vec [1 2 5 5 6 9])
(binary-search sorted-vec 6)"#,
                "4",
            ),

            (
                r#"
(let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))
(let correct (Vector->Set ["spelling" "bat" "cat"]))

(let generate-abc (lambda (do 
  (let offset (Char->Int 'a'))
  (let out [])
  (loop 0 26 (lambda i (push! out (Int->Char (+ i offset)))))
  out)))

(let abc (generate-abc))

(let autocorrect (lambda word (if (Set/has? word correct) word (do

    (let temp (copy word))
    (boolean loop? true)
    (integer i 0)
    (variable out word)

    (while (and (true? loop?) (< (get i) (length word))) (do

        (let filtered (filter/i (lambda _ j (<> j (get i))) word))
        (if (Set/has? filtered correct) (do 
            (&alter! out filtered)
            (&alter! loop? false)))

        (if (> (get i) 0) (do 
          (let prev (get temp (- (get i) 1)))
          (let next (get temp (get i)))
          (set! temp (- (get i) 1) next)
          (set! temp (get i) prev)

          (if (Set/has? temp correct) (do 
            (&alter! out temp)
            (&alter! loop? false))
            (do 
              (set! temp (- (get i) 1) prev)
              (set! temp (get i) next)))))

        (integer k 0)
        (while (and (true? loop?) (< (get k) (length abc))) (do
          (let a (get abc (get k)))
          (let t (get temp (get i)))
          (set! temp (get i) a)
          (if (Set/has? temp correct) (do
              (&alter! out temp)
              (&alter! loop? false))
              (set! temp (get i) t))
          (&alter! k (+ (&get k) 1))))

        (integer j 0)
        (while (and (true? loop?) (< (get j) (length abc))) (do
          (let a (get abc (get j)))
          (let added (cons (slice 0 (get i) word) [a] (slice (get i) (length word) word)))
          (if (Set/has? added correct) (do
              (&alter! out added) 
              (&alter! loop? false)))
          (&alter! j (+ (&get j) 1))))

        (&alter! i (+ (&get i) 1))))
    (get out)))))
(every? (lambda x (Set/has? x correct)) [

  (autocorrect "spellling")
  (autocorrect "speling")
  (autocorrect "spelljng")
  (autocorrect "seplling")

            ])"#,
                "true",
            ),

            (
                r#"(let .env "PORT = 8080
DB   = postgres
SECRET = SANTA")

(let parse/env (comp
                (String->Vector nl)
                (exclude empty?)
                (map (comp (String->Vector sp)
                (map (comp (exclude (apply =# '='))))
                (exclude empty?)))))

(let ENV (reduce
  (lambda a [ k v ] (do (Table/set! a k v) a))
  (buckets 32)
  (parse/env .env)))

(let option (Table/get* "PORT" ENV))
(if (fst option) (get (snd option)) "")"#,
                "8080",
            ),

            (
                r#"
  (let ++ (lambda vrbl (&alter! vrbl (+ (&get vrbl) 1))))

    (let solve (lambda s 
  (and (> (length s) 2) (do
    (let n (length s))
    (integer c 0)
    (loop 0 (- n 1) (lambda i (if (=# (get s i) (get s (+ i 1))) (&alter! c (+ (&get c) 1)))))
    (loop 0 (- n 2) (lambda i (if (=# (get s i) (get s (+ i 2))) (&alter! c (+ (&get c) 1)))))
    (> (get c) 0)))))

(map solve ["BCAB" "BB" "A" "AABB" "BAB" "KEEP"])"#,
                "[false false false true true true]",
            ),

            (
                r#"(let modulus 33554393)
(let multiplier 252533)
(let seed 20151125)

(let mod-add (lambda a b m (do
    (let s (+ a b))
    (if (>= s m) (- s m) s))))

(letrec mul-mod/loop (lambda a b m acc
    (if (= b 0)
        acc
        (mul-mod/loop
            (mod-add a a m)
            (/ b 2)
            m
            (if (= (mod b 2) 1)
                (mod-add acc a m)
                acc)))))

(let mul-mod (lambda a b m (mul-mod/loop a b m 0)))

(letrec pow-mod/loop (lambda base exp m acc
    (if (= exp 0)
        acc
        (pow-mod/loop
            (mul-mod base base m)
            (/ exp 2)
            m
            (if (= (mod exp 2) 1)
                (mul-mod acc base m)
                acc)))))

(let pow-mod (lambda base exp m (pow-mod/loop base exp m 1)))

(let code-index (lambda row col (do
    (let k (+ row col -2))
    (+ (/ (* k (+ k 1)) 2) col))))

(let code-at (lambda row col (do
    (let idx (code-index row col))
    (let factor (pow-mod multiplier (- idx 1) modulus))
    (mul-mod seed factor modulus))))

[(code-at 6 6) (code-at 2981 3075)]
"#,
                "[27995004 9132360]",
            ),

            (
                r#"(let h (lambda payload (do
(let [a b c] payload)
(letrec step (lambda i [i i i]))
(let final-state (step 1))
(do
(let [x y z] final-state)
(+ a b x)
))))
(h [1 2 3 4])"#,
                "4",
            ),

            (
                r#"(let secret "iwrupvqb")
(let secret-bytes [105 119 114 117 112 118 113 98])

(let K [
-680876936 -389564586 606105819 -1044525330
-176418897 1200080426 -1473231341 -45705983
1770035416 -1958414417 -42063 -1990404162
1804603682 -40341101 -1502002290 1236535329
-165796510 -1069501632 643717713 -373897302
-701558691 38016083 -660478335 -405537848
568446438 -1019803690 -187363961 1163531501
-1444681467 -51403784 1735328473 -1926607734
-378558 -2022574463 1839030562 -35309556
-1530992060 1272893353 -155497632 -1094730640
681279174 -358537222 -722521979 76029189
-640364487 -421815835 530742520 -995338651
-198630844 1126891415 -1416354905 -57434055
1700485571 -1894986606 -1051523 -2054922799
1873313359 -30611744 -1560198380 1309151649
-145523070 -1120210379 718787259 -343485551
])

(let left-rotate (lambda x c
(| (<< x c) (& (>> x (- 32 c)) (- (<< 1 c) 1)))))

(let add32 (lambda a b (do
(let s0 (+ (& a 255) (& b 255)))
(let r0 (& s0 255))
(let c0 (>> s0 8))

(let s1 (+ (& (>> a 8) 255) (& (>> b 8) 255) c0))
(let r1 (& s1 255))
(let c1 (>> s1 8))

(let s2 (+ (& (>> a 16) 255) (& (>> b 16) 255) c1))
(let r2 (& s2 255))
(let c2 (>> s2 8))

(let s3 (+ (& (>> a 24) 255) (& (>> b 24) 255) c2))
(let r3 (& s3 255))

(| r0 (| (<< r1 8) (| (<< r2 16) (<< r3 24)))))))

(let add32-4 (lambda a b c d
(add32 (add32 a b) (add32 c d))))

(let block [0])
(loop 1 64 (lambda i (push! block 0)))
(let words [0])
(loop 1 16 (lambda i (push! words 0)))

(let shift-by (lambda i
(do
(let r (mod i 4))
(if (< i 16)
(if (= r 0) 7 (if (= r 1) 12 (if (= r 2) 17 22)))
(if (< i 32)
(if (= r 0) 5 (if (= r 1) 9 (if (= r 2) 14 20)))
(if (< i 48)
(if (= r 0) 4 (if (= r 1) 11 (if (= r 2) 16 23)))
(if (= r 0) 6 (if (= r 1) 10 (if (= r 2) 15 21)))))))))

(let md5-a0! (lambda n (do
(loop 0 8 (lambda i (set! block i (get secret-bytes i))))

(let div-init
(if (>= n 1000000000) 1000000000
(if (>= n 100000000) 100000000
(if (>= n 10000000) 10000000
(if (>= n 1000000) 1000000
(if (>= n 100000) 100000
(if (>= n 10000) 10000
(if (>= n 1000) 1000
(if (>= n 100) 100
(if (>= n 10) 10 1))))))))))
(integer div div-init)

(integer x n)
(integer msg-len 8)
(loop 0 10 (lambda i
(if (> (get div) 0) (do
(let digit (/ (get x) (get div)))
(set! block (get msg-len) (+ 48 digit))
(&alter! x (mod (get x) (get div)))
(&alter! div (/ (get div) 10))
(&alter! msg-len (+ (get msg-len) 1)))
nil)))

(set! block (get msg-len) 128)
(loop 0 56 (lambda i
(if (> i (get msg-len))
(set! block i 0)
nil)))

(let bit-len (* (get msg-len) 8))
(set! block 56 (& bit-len 255))
(set! block 57 (& (>> bit-len 8) 255))
(set! block 58 (& (>> bit-len 16) 255))
(set! block 59 (& (>> bit-len 24) 255))
(set! block 60 0)
(set! block 61 0)
(set! block 62 0)
(set! block 63 0)

(loop 0 16 (lambda i (do
(let j (* i 4))
(set! words i
(| (get block j)
   (| (<< (get block (+ j 1)) 8)
      (| (<< (get block (+ j 2)) 16)
         (<< (get block (+ j 3)) 24))))))))

(let a0 1732584193)
(let b0 -271733879)
(let c0 -1732584194)
(let d0 271733878)

(integer A a0)
(integer B b0)
(integer C c0)
(integer D d0)
(integer f 0)
(integer g 0)

(loop 0 64 (lambda i (do
(if (< i 16)
(do
  (&alter! f (| (& (get B) (get C)) (& (~ (get B)) (get D))))
  (&alter! g i))
(if (< i 32)
(do
  (&alter! f (| (& (get D) (get B)) (& (~ (get D)) (get C))))
  (&alter! g (mod (+ (* 5 i) 1) 16)))
(if (< i 48)
(do
  (&alter! f (^ (get B) (^ (get C) (get D))))
  (&alter! g (mod (+ (* 3 i) 5) 16)))
(do
  (&alter! f (^ (get C) (| (get B) (~ (get D)))))
  (&alter! g (mod (* 7 i) 16))))))
(let x4 (add32-4 (get A) (get f) (get K i) (get words (get g))))
(let new-b (add32 (get B) (left-rotate x4 (shift-by i))))
(let old-d (get D))
(&alter! D (get C))
(&alter! C (get B))
(&alter! B new-b)
(&alter! A old-d))))

(add32 a0 (get A)))))

(let has-five-leading-zeroes! (lambda n (do
(let a (md5-a0! n))
(and (= (& a 255) 0)
 (= (& (>> a 8) 255) 0)
 (< (& (>> a 16) 255) 16)))))

(integer answer 0)
(loop 1 500001 (lambda candidate
(if (and (= (get answer) 0) (has-five-leading-zeroes! candidate))
(&alter! answer candidate)
nil)))
(get answer)
"#,
                "346386",
            ),
        ];
        let std_ast = crate::baked::load_ast();
        for (inp, out) in &test_cases {
            // Temporary xfail: loop-callback lowering regression affects this BFS case.
            // Keep the case in table for visibility, but skip until the bug is fixed.
            if let crate::parser::Expression::Apply(items) = &std_ast {
                match crate::parser::merge_std_and_program(&inp, items[1..].to_vec()) {
                    Ok(exprs) => {
                        match
                            crate::infer
                                ::infer_with_builtins_typed(
                                    &exprs,
                                    crate::types::create_builtin_environment(
                                        crate::types::TypeEnv::new()
                                    )
                                )
                                .map(|(typ, _)| typ)
                        {
                            Ok(_) => {
                                // match crate::vm::run(&exprs, crate::vm::VM::new()) {
                                match crate::wat::compile_program_to_wat(&exprs) {
                                    Ok(result) => {
                                        let argv: Vec<String> = Vec::new();
                                        #[cfg(feature = "io")]
                                        let store_data = crate::io::ShellStoreData
                                            ::new_with_security(
                                                None,
                                                crate::io::ShellPolicy::disabled()
                                            )
                                            .map_err(|e| e.to_string())
                                            .unwrap();
                                        #[cfg(feature = "io")]
                                        let run_result = crate::runtime::run_wat_text(
                                            &result,
                                            store_data,
                                            &argv,
                                            |linker| {
                                                crate::io
                                                    ::add_shell_to_linker(linker)
                                                    .map_err(|e| e.to_string())
                                            }
                                        );
                                        #[cfg(not(feature = "io"))]
                                        let run_result = crate::runtime::run_wat_text(
                                            &result,
                                            (),
                                            &argv,
                                            |_linker| Ok(())
                                        );
                                        match run_result {
                                            Ok(res) => {
                                                assert_eq!(format!("{}", res), *out, "Solution");
                                            }
                                            Err(e) => {
                                                println!("{:?}", inp);
                                                panic!("Failed tests because {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        // to figure out which test failed due to run time Error!
                                        // println!("{:?}", inp);
                                        panic!("Failed tests because {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                // println!("{:?}", inp);
                                panic!("Failed tests because {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        // println!("{:?}", inp);
                        panic!("Failed tests because {}", e);
                    }
                }
            }
        }
    }
}
