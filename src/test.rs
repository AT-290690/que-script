#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer;
    use crate::lisp;

    #[test]
    fn test_addition_type() {
        // Parse the expression
        let exprs = lisp::parse("(+ 1 2)").expect("Failed to parse expression");

        // We just take the first expression
        let expr = exprs.first().expect("No expression found");

        // Run type inference
        let result = infer::infer_with_builtins(expr);

        // It should succeed
        assert!(result.is_ok(), "Type inference failed: {:?}", result.err());

        // Optionally check the exact type
        let typ = result.unwrap();
        assert_eq!(format!("{}", typ), "Int");
    }

    #[test]
    fn test_addition_type_error() {
        // Example that should fail: adding booleans
        let exprs = lisp::parse("(+ true false)").expect("Failed to parse expression");
        let expr = exprs.first().expect("No expression found");
        let result = infer::infer_with_builtins(expr);

        // It should fail
        assert!(result.is_err(), "Type inference unexpectedly succeeded");
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Assuming lisp and infer modules are in the same scope

    #[test]
    fn test_type_inference() {
        // Test simple expressions
        let simple_exprs = lisp::parse("(+ 1 2)").unwrap();
        if let Some(simple_expr) = simple_exprs.first() {
            match infer::infer_with_builtins(simple_expr) {
                Ok(typ) => println!("Type of (+ 1 2): {}", typ),
                Err(e) => panic!("Type error in '(+ 1 2)': {}", e),
            }
        }

        // Test lambda
        let lambda_exprs = lisp::parse("(lambda x (+ x 1))").unwrap();
        if let Some(lambda_expr) = lambda_exprs.first() {
            match infer::infer_with_builtins(lambda_expr) {
                Ok(typ) => println!("Type of (lambda x (+ x 1)): {}", typ),
                Err(e) => panic!("Type error in '(lambda x (+ x 1))': {}", e),
            }
        }

        let lambda_exprs = lisp::parse("(lambda x (and x 42))").unwrap();
        if let Some(lambda_expr) = lambda_exprs.first() {
            match infer::infer_with_builtins(lambda_expr) {
                Ok(typ) => println!("Type of (lambda x (and x 42)): {}", typ),
                Err(e) => panic!("Type error in '(lambda x (and x 42))': {}", e),
            }
        }

        let lambda_exprs = lisp::parse("(lambda x (and x (or x x)))").unwrap();
        if let Some(lambda_expr) = lambda_exprs.first() {
            match infer::infer_with_builtins(lambda_expr) {
                Ok(typ) => println!("Type of (lambda x (and x (or x x))): {}", typ),
                Err(e) => panic!("Type error in '(lambda x (and x (or x x)))': {}", e),
            }
        }

        let lambda_exprs = lisp::parse("(let fn (lambda a b (and a b))) (fn 42 69)").unwrap();
        if let Some(lambda_expr) = lambda_exprs.first() {
            match infer::infer_with_builtins(lambda_expr) {
                Ok(typ) => println!(
                    "Type of (let fn (lambda a b (and a b))) (fn 42 69): {}",
                    typ
                ),
                Err(e) => panic!(
                    "Type error in '(let fn (lambda a b (and a b))) (fn 42 69)': {}",
                    e
                ),
            }
        }

        // Test your range function (without pipe operator)
        let range_exprs = lisp::parse("(summation (range 1 10))").unwrap();
        if let Some(range_expr) = range_exprs.first() {
            match infer::infer_with_builtins(range_expr) {
                Ok(typ) => println!("Type of (summation (range 1 10)): {}", typ),
                Err(e) => panic!("Type error in '(summation (range 1 10))': {}", e),
            }
        }

        // Test array creation
        let array_exprs = lisp::parse("(array 1 2 3)").unwrap();
        if let Some(array_expr) = array_exprs.first() {
            match infer::infer_with_builtins(array_expr) {
                Ok(typ) => println!("Type of (array 1 2 3): {}", typ),
                Err(e) => panic!("Type error in '(array 1 2 3)': {}", e),
            }
        }

        // Test nested array creation
        let array_exprs = lisp::parse("(array (array (array 1)))").unwrap();
        if let Some(array_expr) = array_exprs.first() {
            match infer::infer_with_builtins(array_expr) {
                Ok(typ) => println!("Type of (array (array (array 1))): {}", typ),
                Err(e) => panic!("Type error in '(array (array (array 1)))': {}", e),
            }
        }

        // Test if expression
        let if_expression = lisp::parse("(if (= 1 2) 10 (> 1 0))").unwrap();
        if let Some(if_expr) = if_expression.first() {
            match infer::infer_with_builtins(if_expr) {
                Ok(typ) => println!("Type of (if (= 1 2) 10 false): {}", typ),

                Err(e) => panic!("Type error in '(if (= 1 2) 10 false)': {}", e),
            }
        }
    }
}
