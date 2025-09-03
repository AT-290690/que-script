#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_inference_passing_cases() {
        let test_cases = [
            ("(+ 1 2)", "Int"),
            ("(and (> 2 1) (= 1 1))", "Bool"),
            (
                "(do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b)",
                "Bool",
            ),
            (
                "(do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b)",
                "Bool",
            ),
            (
                "(do (let xs (array (array 1))) (let x (get xs 0)) (let y (get x 0)) y)",
                "Int",
            ),
            (
                "(do (let xs (array (array 1))) (let x (get (get xs 0) 0)) x)",
                "Int",
            ),
            ("(lambda x (+ x 1))", "Int -> Int"),
            ("(lambda x (and x (or x x)))", "Bool -> Bool"),
            ("(let fn (lambda a b (and a b))) (fn 42 69)", "Int"),
            (
                "(let process (lambda xs (get xs 0))) (process (array 1 2 3 ))",
                "Int",
            ),
            ("(do (let process (lambda xs (do (let x (get xs 0)) x))) (process (array (= 1 1))))", "Bool"),
            ("(array 1 2 3)", "[Int]"),
            ("(array (array (array 1)))", "[[[Int]]]")
        ];

        for (inp, out) in &test_cases {
            let exprs = crate::lisp::parse(inp).unwrap();

            if let Some(expr) = exprs.first() {
                let result = crate::infer::infer_with_builtins(expr);

                // Assert that the result is Ok
                assert!(
                    result.is_ok(),
                    "Type inference should succeed for expression: {}",
                    inp
                );

                // Optionally, check that the type is Int
                if let Ok(typ) = result {
                    assert_eq!(
                        typ.to_string(),
                        *out,
                        "Type of expression should match expected",
                    );
                }
            } else {
                panic!("No expressions found in parsed result for: {}", inp);
            }
        }
    }

    #[test]
    fn test_type_inference_failure() {
        // Test cases that should result in type inference errors
        let test_cases = [
            ("(+ 1 (= 1 1))", "Cannot unify Int with Bool"),
            ("(lambda x (and x 42))", "Cannot unify Bool with Int"),
            ("(summation (range 1 10))", "Undefined variable: summation"),
            ("(if (= 1 2) 10 (= 0 1))", "Cannot unify Int with Bool"),
        ];

        for (inp, out) in &test_cases {
            let exprs = crate::lisp::parse(inp).unwrap();

            if let Some(expr) = exprs.first() {
                // Check that type inference returns an Err
                let result = crate::infer::infer_with_builtins(expr);

                // Assert that the result is an Err
                assert!(
                    result.is_err(),
                    "Expected type inference error for expression: {}",
                    inp
                );

                // Optionally, you can check the error message
                if let Err(error_msg) = result {
                    assert_eq!(
                        error_msg.to_string(),
                        *out,
                        "Type error should match expected",
                    );
                }
            } else {
                panic!("No expressions found in parsed result");
            }
        }
    }
}
