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
                "(do (let xs (vector (vector 1))) (let x (get xs 0)) (let y (get x 0)) y)",
                "Int",
            ),
            (
                "(do (let xs (vector (vector 1))) (let x (get (get xs 0) 0)) x)",
                "Int",
            ),
            ("(lambda x (+ x 1))", "Int -> Int"),
            ("(lambda x (and x (or x x)))", "Bool -> Bool"),
            ("(let fn (lambda a b (and a b))) (fn 42 69)", "Int"),
            (
                "(let process (lambda xs (get xs 0))) (process (vector 1 2 3 ))",
                "Int",
            ),
            ("(do (let process (lambda xs (do (let x (get xs 0)) x))) (process (vector (= 1 1))))", "Bool"),
            ("(vector 1 2 3)", "[Int]"),
            ("(vector (vector (vector 1)))", "[[[Int]]]"),
            ("(do (let x 10) (let fn (lambda (do (let x 2) (* x x)))) (fn))", "Int")
        ];

        for (inp, out) in &test_cases {
            let exprs = crate::parser::parse(inp).unwrap();

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
            (
                "(do (let x 10) (let x 2))",
                "Variable 'x' already defined in this scope",
            ),
        ];

        for (inp, out) in &test_cases {
            let exprs = crate::parser::parse(inp).unwrap();

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

    #[test]
    fn test_correctness() {
        let test_cases = [
            ("(+ 1 2)", "3"),
            ("(std:vector:ints:sum [ 1 2 ])", "3"),
            (
                r#"(import filter map std:vector)
(import odd? square std:int)
(import sum std:vector:ints)

(let sum-odd-squares (lambda xs
    (|> xs
        (filter odd?)
        (map square)
        (sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])"#,
                "165",
            ),
            (
                "\"Hello world\"",
                "[72 101 108 108 111 32 119 111 114 108 100]",
            ),
            (
                r#"(import
    cons sliding-window filter map slice
    ints:sum ints:maximum ints:minimum ints:range
    first last
    push! concat! sort!
 std:vector)
 
 (import
   max min char:space char:tab char:new-line
 std:int)
 
 (import
    chars->digits string->vector chars->integer
 std:convert)
 
 ; tests
 (let k-mod (lambda n k (if (< k n) k (k-mod n (- k n)))))
 (let mod2 (k-mod 2))
 (let collatz (lambda n steps
               (if (= n 1) steps
                   (if (= (mod2 n) 0)
                       (collatz (/ n 2) (+ steps 1))
                       (collatz (+ (* n 3) 1) (+ steps 1))))))
 (let fact (lambda n total
   (if (= n 0)
       total
       (fact (- n 1) (* total n)))))
 
 (let Apart1 (lambda input (|>
  input
  (cons [(first input)])
  (sliding-window 2)
  (filter (lambda xs (= (. xs 0) (. xs 1))))
  (map first)
  (ints:sum))))
 
 (let Apart2 (lambda input (|>
  input
  (cons (slice input 0 (/ (length input) 2)))
  (sliding-window (+ (/ (length input) 2) 1))
  (filter (lambda xs (= (first xs) (last xs))))
  (map first)
  (ints:sum))))
 
 (let parse (lambda input (|>
 input
 (string->vector char:new-line)
 (map (lambda xs (|>
                  xs
                  (map (lambda x (if (= x char:tab) char:space x)))
                  (string->vector char:space)
                  (map chars->integer)))))))
 
 (let part1 (lambda input (|> input (map (lambda xs (- (ints:maximum xs) (ints:minimum xs)))) (ints:sum))))
 
 (let divisible-pair (lambda xs (do
 (let len (length xs))
 (let out [])
 (loop 0 len (lambda i
  (loop i len (lambda j (do
      (let a (. xs i))
      (let b (. xs j))
      (let l (max a b))
      (let r (min a b))
      (if (and (<> i j) (= (mod l r) 0)) (do
        (push! out l)
        (push! out r)
        nil)))))))
  out)))
 
 (let part2 (lambda input (|>
 input
 (map (lambda xs (do
  (let pair (divisible-pair xs))
  (/ (. pair 0) (. pair 1)))))
 (ints:sum))))
 
 (concat! [ (collatz 27 0) (fact 10 1) ]
 [
   (|> [ "1122" "1111" "1234" "91212129" ] (map chars->digits) (map Apart1))
   (|> [ "1212"  "1221" "123425" "123123" "12131415"] (map chars->digits) (map Apart2))
[(|> "5 1 9 5
7 5 3
2 4 6 8" (parse) (part1))
(|> "5 9 2 8
9 4 7 3
3 8 6 5" (parse) (part2))]
 (sort! (ints:range 1 10) >=)
 ]
 )"#,
                "[111 3628800 3 4 0 9 6 0 4 12 4 18 9 10 9 8 7 6 5 4 3 2 1]",
            ),
        ];
        let std_ast = crate::baked::load_ast();
        for (inp, out) in &test_cases {
            if let crate::parser::Expression::Apply(items) = &std_ast {
                let exprs = crate::parser::merge_std_and_program(&inp, items[1..].to_vec());
                let result = crate::vm::run(&exprs);
                assert_eq!(format!("{:?}", result), *out, "Solution");
            }
        }
    }
}
