#![allow(dead_code)]
#![allow(warnings)]

pub fn test_type_inference() {
    // Test type inference
    println!("\n=== Type Inference Examples ===");

    // Test simple expressions
    let simple_exprs = crate::lisp::parse("(+ 1 2)").unwrap();
    if let Some(simple_expr) = simple_exprs.first() {
        match crate::infer::infer_with_builtins(simple_expr) {
            Ok(typ) => println!("Type of (+ 1 2): {}", typ),
            Err(e) => println!("Type error in '(+ 1 2)': {}", e),
        }
    }

    let simple_exprs =
        crate::lisp::parse("(do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b)")
            .unwrap();
    if let Some(simple_expr) = simple_exprs.first() {
        match crate::infer::infer_with_builtins(simple_expr) {
            Ok(typ) => println!("Type of (do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b): {}", typ),
            Err(e) => println!("Type error in '(do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b)': {}", e),
        }
    }

    let simple_exprs =
        crate::lisp::parse("(do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b)")
            .unwrap();
    if let Some(simple_expr) = simple_exprs.first() {
        match crate::infer::infer_with_builtins(simple_expr) {
            Ok(typ) => println!("Type of (do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b): {}", typ),
            Err(e) => println!("Type error in '(do (let id (lambda x x)) (let a (id 10)) (let b (id (= 1 1))) b)': {}", e),
        }
    }
    let simple_exprs = crate::lisp::parse(
        "(do (let xs (array (array 1))) (let x (get xs 0)) (let y (get x 0)) y)",
    )
    .unwrap();
    if let Some(simple_expr) = simple_exprs.first() {
        match crate::infer::infer_with_builtins(simple_expr) {
            Ok(typ) => println!(
                "Type of (do (let xs (array (array 1))) (let x (get xs 0)) (let y (get x 0)) y): {}",
                typ
            ),
            Err(e) => println!(
                "Type error in '(do (let xs (array (array 1))) (let x (get xs 0)) (let y (get x 0)) y)': {}",
                e
            ),
        }
    }
    let simple_exprs =
        crate::lisp::parse("(do (let xs (array (array 1))) (let x (get (get xs 0) 0)) x)").unwrap();
    if let Some(simple_expr) = simple_exprs.first() {
        match crate::infer::infer_with_builtins(simple_expr) {
            Ok(typ) => println!(
                "Type of (do (let xs (array (array 1))) (let x (get (get xs 0) 0)) x): {}",
                typ
            ),
            Err(e) => println!(
                "Type error in '(do (let xs (array (array 1))) (let x (get (get xs 0) 0)) x)': {}",
                e
            ),
        }
    }

    // Test lambda
    let lambda_exprs = crate::lisp::parse("(lambda x (+ x 1))").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match crate::infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!("Type of (lambda x (+ x 1)): {}", typ),
            Err(e) => println!("Type error in '(lambda x (+ x 1))': {}", e),
        }
    }

    let lambda_exprs = crate::lisp::parse("(lambda x (and x 42))").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match crate::infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!("Type of (lambda x (and x 42)): {}", typ),
            Err(e) => println!("Type error in '(lambda x (and x 42))': {}", e),
        }
    }

    let lambda_exprs = crate::lisp::parse("(lambda x (and x (or x x)))").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match crate::infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!("Type of (lambda x (and x (or x x))): {}", typ),
            Err(e) => println!("Type error in '(lambda x (and x (or x x)))': {}", e),
        }
    }

    let lambda_exprs = crate::lisp::parse("(let fn (lambda a b (and a b))) (fn 42 69)").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match crate::infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!(
                "Type of (let fn (lambda a b (and a b))) (fn 42 69): {}",
                typ
            ),
            Err(e) => println!(
                "Type error in '(let fn (lambda a b (and a b))) (fn 42 69)': {}",
                e
            ),
        }
    }

    let lambda_exprs =
        crate::lisp::parse("(do (let process (lambda xs (get xs 0))) (process (array 1 2 3 )))")
            .unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match crate::infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!(
                "Type of (let process (lambda xs (get xs 0))) (process (array 1 2 3 )): {}",
                typ
            ),
            Err(e) => println!(
                "Type error in '(let process (lambda xs (get xs 0))) (process (array 1 2 3 ))': {}",
                e
            ),
        }
    }

    let lambda_exprs = crate::lisp::parse(
        "(do (let process (lambda xs (do (let x (get xs 0)) x))) (process (array (= 1 1) )))",
    )
    .unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match crate::infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!(
                "Type of (do (let process (lambda xs (do (let x (get xs 0)) x))) (process (array (= 1 1) ))): {}",
                typ
            ),
            Err(e) => println!(
                "Type error in '(do (let process (lambda xs (do (let x (get xs 0)) x))) (process (array (= 1 1) )))': {}",
                e
            ),
        }
    }
    // Test your range function (without pipe operator)
    let range_exprs = crate::lisp::parse("(summation (range 1 10))").unwrap();
    if let Some(range_expr) = range_exprs.first() {
        match crate::infer::infer_with_builtins(range_expr) {
            Ok(typ) => println!("Type of (summation (range 1 10)): {}", typ),
            Err(e) => println!("Type error in '(summation (range 1 10))': {}", e),
        }
    }

    // Test array creation
    let array_exprs = crate::lisp::parse("(array 1 2 3)").unwrap();
    if let Some(array_expr) = array_exprs.first() {
        match crate::infer::infer_with_builtins(array_expr) {
            Ok(typ) => println!("Type of (array 1 2 3): {}", typ),
            Err(e) => println!("Type error in '(array 1 2 3)': {}", e),
        }
    }

    // Test array creation
    let array_exprs = crate::lisp::parse("(array (array (array 1)))").unwrap();
    if let Some(array_expr) = array_exprs.first() {
        match crate::infer::infer_with_builtins(array_expr) {
            Ok(typ) => println!("Type of (array (array (array 1))): {}", typ),
            Err(e) => println!("Type error in '(array (array (array 1)))': {}", e),
        }
    }

    // Test if expression
    let if_expression = crate::lisp::parse("(if (= 1 2) 10 (= 0 1))").unwrap();
    if let Some(if_expr) = if_expression.first() {
        match crate::infer::infer_with_builtins(if_expr) {
            Ok(typ) => println!("Type of (if (= 1 2) 10 (= 0 1)): {}", typ),
            Err(e) => println!("Type error in '(if (= 1 2) 10 (= 0 1))': {}", e),
        }
    }
}
