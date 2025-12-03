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
            ("(do (let fn (lambda a b (and a b))) (fn (= 1 1) (= 1 2)))", "Bool"),
            (
                "(do (let process (lambda xs (get xs 0))) (process (vector 1 2 3 )))",
                "Int",
            ),
            ("(do (let process (lambda xs (do (let x (get xs 0)) x))) (process (vector (= 1 1))))", "Bool"),
            ("(vector 1 2 3)", "[Int]"),
            ("(vector (vector (vector 1)))", "[[[Int]]]"),
            ("(do (let x 10) (let fn (lambda (do (let x 2) (* x x)))) (fn))", "Int"),
            ("(do (let fn (lambda a b c d (do (set! d (length d) (if c (lambda x (> (+ a b) x)) (lambda . false))) (> (length d) 10)))) fn)", "Int -> Int -> Bool -> [Int -> Bool] -> Bool"),
            ("(do (let Int 0) (let as (lambda . t t)) (let xs (as (vector) (vector Int))) xs)", "[Int]"),
            ("(tuple 0 true)", "{Int * Bool}"),
            ("(vector (tuple 0 true) (tuple 1 false))", "[{Int * Bool}]")
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
                    // println!("{:?}", inp);
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
            (
                "(+ 1 (= 1 1))",
                r#"Error! Cannot unify Int with Bool
Error! (+ 1 (= 1 1))"#,
            ),
            ("(1 2)", "Error! Cannot apply non-function type: Int\n(1 2)"),
            (
                "(do (let t 10) (t))",
                "Error! Cannot apply non-function type: Int\n(t)",
            ),
            (
                "(let x (vector 1 2 (= 1 2)))",
                "Error! Cannot unify Int with Bool\nError! (vector 1 2 (= 1 2))",
            ),
            (
                "(vector 1 2 (> 1 2))",
                "Error! Cannot unify Int with Bool\nError! (vector 1 2 (> 1 2))",
            ),
            (
                "(lambda x (and x 42))",
                "Error! Cannot unify Bool with Int\nError! (and x 42)",
            ),
            (
                "(summation (range 1 10))",
                "Error! Undefined variable: summation",
            ),
            (
                "(if 1 10 20)",
                r#"Error! Cannot unify Int with Bool
Error! Condition must be Bool
(if 1 10 20)"#,
            ),
            (
                "(if (= 1 2) 10 (= 0 1))",
                r#"Error! Cannot unify Int with Bool
Error! Concequent and alternative must match types
(if (= 1 2) 10 (= 0 1))"#,
            ),
            (
                "(do (let x 10) (let x 2))",
                "Error! Variable 'x' already defined in this scope",
            ),
            (
                "(vector (tuple 0 true) (tuple true 0))",
                "Error! Cannot unify Int with Bool\nError! (vector (tuple 0 true) (tuple true 0))",
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
            ("(std/vector/int/sum [ 1 2 ])", "3"),
            (
                "\"Hello world\"",
                "[72 101 108 108 111 32 119 111 114 108 100]",
            ),
            (r#"(let A [false (and (= 1 2) (> 3 3))]) ; => [false false] Correct
(let B [false (or (= 1 2) (> 3 3))]) ; => [true false] Wrong
(and (=? (get A 0) (get B 0)) (=? (get A 1) (get B 1)))"#, "true"),
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
  (let tail-call/smash (lambda t
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
    (let chars (|> deck
                    (std/vector/map std/convert/integer->string)
                    (std/vector/hash/table/count)
                    (std/vector/hash/table/entries)
                    (std/vector/map std/vector/second)
                    (std/vector/flat-one)))
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
            (let find-missing-numbers (lambda nums (|> 
    (std/vector/int/range 1 (length nums)) 
    (std/vector/map (lambda x (std/convert/integer->string-base x 10)))
    (std/convert/vector->set)
    (std/vector/hash/set/difference (|> nums (std/vector/map (lambda x (std/convert/integer->string-base x 10))) (std/convert/vector->set)))
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
                r#"(let flood-fill (lambda image sr sc color (do 
    (let old (. image sr sc))
    (if (= old color) 
        image 
        (do 
            (let m (length image))
            (let n (length (std/vector/first image)))
            (let stack [[sr sc]])
            (loop (std/vector/not-empty? stack) (lambda (do 
                (let t (std/vector/pop-and-get! stack))
                (let i (std/vector/first t))
                (let j (std/vector/second t))
                (if (and (>= i 0) (< i m) (>= j 0) (< j n) (= (. image i j) old)) (do
                    (std/vector/3d/set! image i j color)
                    (std/vector/push! stack [(+ i 1) j])
                    (std/vector/push! stack [(- i 1) j])
                    (std/vector/push! stack [i (+ j 1)])
                    (std/vector/push! stack [i (- j 1)])
                    nil)))))
        image)))))


(let image [[1 1 1] [1 1 0] [1 0 1]])
(flood-fill image 1 1 2)
; Output/ [[2 2 2] [2 2 0] [2 0 1]]"#,
                "[[2 2 2] [2 2 0] [2 0 1]]",
            ),
            (
                r#"(let valid-path (lambda n edges source destination (do
  (if (= source destination) true
    (do
      (let graph (std/vector/map (std/vector/int/zeroes n) (lambda . [])))
      (std/vector/for edges (lambda edge (do
        (let u (get edge 0))
        (let v (get edge 1))
        (std/vector/push! (get graph u) v)
        (std/vector/push! (get graph v) u))))
      (let visited (std/vector/int/zeroes n))
      (let queue [source])
      (std/vector/set! visited source 1)
      (boolean found false)
      (loop (and (not (true? found)) (> (length queue) 0))
        (lambda (do
          (let current (std/vector/pop-and-get! queue))
          (if (= current destination)
            (boole-set found true)
            (std/vector/for (get graph current) (lambda neighbor (do
              (if (= (get visited neighbor) 0)
                (do
                  (std/vector/set! visited neighbor 1)
                  (std/vector/push! queue neighbor) 
                  nil)))))))))
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
(let parse (lambda input (|> input (std/convert/string->vector std/char/new-line) (std/vector/map std/convert/chars->digits))))
(let part1 (lambda matrix (do
  (let coords (std/vector/3d/points matrix std/int/zero?))
  (let default-queue-value [ 0 ])
  (std/vector/reduce coords (lambda a xs (do
        (integer score 0)
        (let y (std/vector/first xs))
        (let x (std/vector/second xs))
        (let visited (std/vector/buckets 8))
        (let queue (std/vector/queue/new default-queue-value))
        (let current (. matrix y x))
        (std/vector/hash/set/add! visited (yx->key y x))
        (std/vector/queue/enqueue! queue [ y x ])
        
        (loop (std/vector/queue/not-empty? queue) (lambda (do
            (let element (std/vector/queue/peek queue))
            (std/vector/queue/dequeue! queue  default-queue-value)
            (let y (std/vector/first element))
            (let x (std/vector/second element))  
            (std/vector/3d/adjacent matrix std/vector/3d/von-neumann-neighborhood y x (lambda cell dir dy dx (do
                 (let key (yx->key dy dx))
                 (if (and (= (- cell (. matrix y x)) 1) (not (std/vector/hash/set/has? visited key))) (do
                    (if (= cell 9) (do (++ score) nil) (do (std/vector/queue/enqueue! queue [ dy dx ]) nil))
                    (std/vector/hash/set/add! visited key)
                    nil))))))))

        (+ a (get score)))) 0))))

(let part2 (lambda matrix (do
  (let coords (std/vector/3d/points matrix std/int/zero?))
  (let default-queue-value [ 0 ])
  (std/vector/reduce coords (lambda a xs (do
        (integer score 0)
        (let y (std/vector/first xs))
        (let x (std/vector/second xs))
        (let visited (std/vector/buckets 8))
        (let queue (std/vector/queue/new default-queue-value))
        (let current (. matrix y x))
        (let root-key (yx->key y x))
        (std/vector/hash/table/set! visited root-key 1)
        (std/vector/queue/enqueue! queue [ y x ])
        (loop (std/vector/queue/not-empty? queue) (lambda (do
            (let element (std/vector/queue/peek queue))
            (let y (std/vector/first element))
            (let x (std/vector/second element))  
            (if (= (. matrix y x) 9) (+= score (as (std/vector/hash/table/get visited root-key) Int)))
            (std/vector/queue/dequeue! queue default-queue-value)
            (std/vector/3d/adjacent matrix std/vector/3d/von-neumann-neighborhood y x (lambda cell dir dy dx (do
                 (let key (yx->key dy dx))
                 (if (= (- cell (. matrix y x)) 1) (do
                    (std/vector/queue/enqueue! queue [ dy dx ])
                    (if (std/vector/hash/table/has? visited key) 
                        (std/vector/hash/table/set! visited key (as (+# (std/vector/hash/table/get visited root-key) (std/vector/hash/table/get visited key)) Int)) 
                        (std/vector/hash/table/set! visited key (as (std/vector/hash/table/get visited root-key) Int)))
                      nil))))))))
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

(let parse (lambda input (|>
                            input
                            (std/vector/char/lines)
                            (std/vector/map (lambda word (|>
                                                      word
                                                      (std/vector/char/words)
                                                      (std/vector/filter std/vector/not-empty?)
                                                      (std/vector/map std/convert/chars->integer)))))))

(let part1 (lambda input (|>
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
  (|>
    left
    (std/vector/map (lambda l (* l (std/vector/count-of right (lambda r (= l r))))))
    (std/vector/int/sum)))))

(let PARSED (parse INPUT))
[(part1 PARSED) (part2 PARSED)]"#,
                "[11 31]",
            ),
            (
                r#"
(let parse (lambda input (|> input (std/vector/char/lines) (std/vector/map std/convert/chars->integer))))
(let part1 (lambda input (do 
    (let min (std/vector/int/minimum input))
    (|> input
        (std/vector/map (lambda x (- x min)))
        (std/vector/int/sum)))))

 (let part2 (lambda input (do 
    (std/vector/sort/desc! input)
    (let median (std/vector/int/median input))
    (|> input
        (std/vector/map (lambda x (cond (> x median) (- x median) (< x median) (- median x) 0)))
        (std/vector/int/sum)))))

[(|> 
"3
4
7
8"
    (parse)
    (part1)
)
(|> 
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

(let solve! (lambda xs (do 
    (integer count 0)
    (let len (length xs))
    (std/vector/for xs (lambda x (if (<> x 0) (do 
        (set! xs (get count) x)
        (++ count)))))
    (loop (< (get count) len) (lambda (do 
        (set! xs (get count) 0)
        (++ count))))
    xs)))

(solve! xs)"#,
                "[1 2 4 3 5 0 0 0]",
            ),
            (
                r#"(let naive-sub-array-sum (lambda xs (do 
    (let n (length xs))
    (integer out 0)
    (loop 0 n (lambda i (do 
        (integer temp 0)
        (loop i n (lambda j (do 
            (+= temp (get xs j))
            (+= out (get temp))))))))
    (get out))))

(let expert-sub-array-sum (lambda xs (do 
    (let n (length xs))
    (integer out 0)
    (loop 0 n (lambda i (+= out (* (get xs i) (+ i 1) (- n i)))))
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
    (loop 0 (length nums) (lambda i (std/vector/push! (if (std/int/even? i) even odd) (. nums i))))
    (loop 0 (length even) (lambda i (do (std/vector/push! out (. even i)) (std/vector/push! out (. odd i)))))
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
                r#"(let fn (lambda [ a b c r ] (+ a b c (std/vector/int/product r))))
(fn [ 1 2 3 4 5 6 ])"#,
                "126",
            ),
            (
                r#"(let input "A:+,-,=,=,+,-,=,=,+,-
B:+,=,-,+,+,=,-,+,+,=
C:=,-,+,+,=,-,+,+,=,-
D:=,=,=,+,=,=,=,+,=,=")

(let parse (lambda input (do 
(|> input (std/vector/char/lines) (std/vector/map (lambda x (do 
    (let y (std/vector/char/commas x))
    (set! y 0 (get (std/convert/string->vector (get y 0) std/char/colon) 1))
    (std/vector/flat-one y)))))
)))
    
(let app (lambda a x 
    (cond (=# x std/char/plus) (std/vector/append! a (+ (std/vector/last a) 1))
    (=# x std/char/minus) (std/vector/append! a (- (std/vector/last a) 1))
    (=# x std/char/equal) (std/vector/append! a (std/vector/last a))
    (std/vector/append! a (std/vector/last a)))))
(let part1 (lambda xs (do
    (let letters (|> input (std/vector/char/lines) (std/vector/map std/vector/first)))
    (|> xs (std/vector/map (lambda x (|> x (std/vector/reduce app [0])))) 
    (std/vector/map std/vector/int/sum)
    (std/vector/map/i (lambda x i [i (+ x 100)]))
    (std/vector/sort! (lambda a b (> (. a 1) (. b 1))))
    (std/vector/map (lambda [i .] (. letters i)))))))
(|> input (parse) (part1))"#,
                "[66 68 67 65]",
            ),
            (
                r#"(let palindrome? (lambda str (do 
    (let q (std/vector/queue/new std/char/0))
    (let s (std/vector/stack/new std/char/0))
    
    (std/vector/for str (lambda x (do
        (std/vector/stack/push! s x)
        (std/vector/queue/enqueue! q x))))
    
    (let p? [true])
    
    (loop 0 (/ (length str) 2) (lambda . 
        (if (not (=# (std/vector/stack/peek s) (std/vector/queue/peek q)))
             (boole-set p? false) 
             (do 
                 (std/vector/stack/pop! s std/char/0)
                 (std/vector/queue/dequeue! q std/char/0)
                 nil))))
    (get p?))))
    
[(palindrome? "racecar") (palindrome? "yes")]"#,
                "[true false]",
            ),
            (
                r#"(let palindrome? (lambda str (do 
    (let p? [true])
    (loop 0 (/ (length str) 2) (lambda i (if (not (=# (get str i) (get str (- (length str) i 1)))) (boole-set p? false))))
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
                r#"(let rev (lambda xs rev 
    (if (std/vector/empty? xs) 
         rev 
        (rev (std/vector/drop/last xs 1) (std/vector/append! rev (std/vector/at xs -1))))))
;
(rev [ 1 2 3 4 5 ] [])"#,
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
    (loop (<> (get i) (get j)) (lambda (do 
        (if (> (get xs (get i)) (get xs (get j))) (do 
            (set max (std/int/max (* (- (get j) (get i)) (get xs (get j))) (get max)))
            (-- j)) (do
            (set max (std/int/max (* (- (get j) (get i)) (get xs (get i))) (get max)))
            (++ i))))))
    (get max))))

[
    (fn [ 1 8 6 2 5 4 8 3 7 ]) ; 49
    (fn [ 1 1 ]) ; 1
]"#,
                "[49 1]",
            ),
            (
                r#"
            (let factorial (lambda n total
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
            ; [1 3 7 8 4 6 5 2 8 8 2 0]
            
            "#,
                "[1 3 7 8 4 6 5 2 8 8 2 0]",
            ),
            (
                r#"(let str "
 1 + 2 = 3
 3 + 3 = 6
 8 + -1 = 7
 8 + 1 = 9
 8 + 1 = 10
")
(|>
 str
 (std/convert/string->vector std/char/new-line)
 (std/vector/filter std/vector/not-empty?) ; trim
 (std/vector/map (lambda xs
   (|> xs
     (std/convert/string->vector std/char/space)
     (std/vector/filter std/vector/not-empty?)
     (std/vector/filter/i (lambda . i (std/int/even? i)))
     (std/vector/map std/convert/chars->integer))))
 (std/vector/map (lambda [ a b c . ] (= (+ a b) c)))
 (std/vector/count-of (lambda x (eq x true))))"#,
                "4",
            ),
            (
                r#"
(let num-rabbits (lambda answers
  (|> answers
      (std/vector/map std/convert/integer->string)
      (std/vector/hash/table/count)
      (std/vector/hash/table/entries)
   
      (std/vector/reduce (lambda acc [str count .]
        (+ acc (* (std/int/ceil/div (as (get count) Int) (+ (std/convert/chars->integer (as str [Char])) 1))
                  (+ (std/convert/chars->integer (as str [Char])) 1))))
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
          (let helper (lambda xs m (|> xs (std/vector/map (lambda x (+ x m))) (std/vector/count-of (lambda x (and (>= x s) (<= x t)))))))
          [(helper apples a) (helper oranges b)])))
      
      (count-apples-and-oranges 7 11 5 15 [ -2 2 1 ] [ 5 -6 ])"#,
                "[1 1]",
            ),
            (
                r#"(let count-points (lambda rings (do
  (let rods (std/vector/map (std/vector/int/zeroes 10) (lambda . [false false false]))) ; [R, G, B] for each rod
  (let len (length rings))
  (loop 0 len (lambda i (do
    (if (std/int/even? i)
      (do
        (let color (get rings i))
        (let rod-char (get rings (+ i 1)))
        (let rod (get rods (- (std/convert/char->digit rod-char) 0)))
        (cond
          (=# color std/char/R) (set! rod 0 true)
          (=# color std/char/G) (set! rod 1 true)
          (=# color std/char/B) (set! rod 2 true)
          nil))))))
  (std/vector/count-of rods (lambda rod (and (get rod 0) (get rod 1) (get rod 2)))))))

; Example usage
[(count-points "B0B6G0R6R0R6G9") ; Should return 1
 (count-points "B0R0G0R9R0B0G0") ; Should return 1
 (count-points "G4")] ; Should return 0

"#,
                "[1 1 0]",
            ),
            (
                r#"(let part1 (lambda input (|> input 
    (std/vector/cons [(std/vector/first input)]) 
    (std/vector/sliding-window 2) 
    (std/vector/filter (lambda x (= (. x 0) (. x 1))))
    (std/vector/map std/vector/first)
    (std/vector/int/sum))))
(let part2 (lambda input (|> input
    (std/vector/cons (std/vector/slice input 0 (/ (length input) 2)))
    (std/vector/sliding-window (+ (/ (length input) 2) 1))
    (std/vector/filter (lambda x (= (std/vector/first x) (std/vector/last x))))
    (std/vector/map std/vector/first)
    (std/vector/int/sum))))
    
[
  (|> ["1122" "1111" "1234" "91212129"] (std/vector/map std/convert/chars->digits) (std/vector/map part1)) 
  (|> ["1212"  "1221" "123425" "123123" "12131415"] (std/vector/map std/convert/chars->digits) (std/vector/map part2))
]"#,
                "[[3 4 0 9] [6 0 4 12 4]]",
            ),
            (
                r#"(let INPUT "0
3
0
1
-3")

(let parse (lambda input (|> input (std/convert/string->vector std/char/new-line) (std/vector/map std/convert/chars->integer))))
(let part1 (lambda input (do 
    (integer pointer (get input))
    (integer steps 0)
    (integer index 0)
    (boolean escaped? false)
    (loop (false? escaped?) (lambda (do
        (set! input (get index) (+ (get pointer) 1))
        (+= index (get pointer))
        (if (std/vector/in-bounds? input (get index)) (set pointer (get input (get index))) (boole-set escaped? true))
        (++ steps))))
    (get steps))))

(let part2 (lambda input (do 
    (integer pointer (get input))
    (integer steps 0)
    (integer index 0)
    (boolean escaped? false)
    (loop (false? escaped?) (lambda (do
        (set! input (get index) (+ (get pointer) (if (>= (get pointer) 3) -1 1)))
        (+= index (get pointer))
        (if (std/vector/in-bounds? input (get index)) (set pointer (get input (get index))) (boole-set escaped? true))
        (++ steps))))
    (get steps))))
    
[(|> INPUT (parse) (part1)) (|> INPUT (parse) (part2))]"#,
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
                r#"(let interleave (lambda a b (|> (std/vector/zipper a b) (std/vector/flat-one))))
(let ints (lambda xs (std/vector/map xs std/convert/integer->string)))
; examples
[
 (interleave [ "a" "b" "c" ] (ints [ 1 2 3 ])) ; [ "a" 1 "b" 2 "c" 3 ]
 (interleave (ints [ 1 2 ]) [ "x" "y" "z" ])  ; [ 1 "x" 2 "y" "z" ]
]"#,
                "[[[97] [49] [98] [50] [99] [51]] [[49] [120] [50] [121]]]",
            ),
(r#"(let fn (lambda [ a b . ] [ x y . ] (std/int/manhattan-distance a b x y)))
(fn [ 1 2 ] [ 3 4 ])"#, "4"),

            (r#"
(let RAW "A=._
B=_...
C=_._.
D=_..
E=.
F=.._.
G=__.
H=....
I=..
J=.___
K=_._
L=._..
M=__
N=_.
O=___
P=.__.
Q=__._
R=._.
S=...
T=_
U=.._
V=..._
W=.__
X=_.._
Y=_.__
Z=__..
0=_____
1=.____
2=..___
3=...__
4=...._
5=.....
6=_....
7=__...
8=___..
9=____.
.=._._._
,=__..__")
(let parse (lambda xs (|> xs (std/convert/string->vector std/char/new-line) (std/vector/map (lambda x (std/convert/string->vector x std/char/equal))))))
(let PARSED (parse RAW))
(let *ABC->MORSE-IDEXES* (std/vector/reduce/i PARSED (lambda a [k .] i (std/vector/hash/table/set! a k i)) (std/vector/buckets 64)))
(let *ABC->MORSE-CODES* (std/vector/map PARSED (lambda [. v .] v)))

(let *MORSE->ABC-IDEXES* (std/vector/reduce/i PARSED (lambda a [. k .] i (std/vector/hash/table/set! a k i)) (std/vector/buckets 64)))
(let *MORSE->ABC-CODES* (std/vector/map PARSED (lambda [v .] v)))

(let abc->morse (lambda letter (get *ABC->MORSE-CODES* (as (std/vector/hash/table/get *ABC->MORSE-IDEXES* letter) Int))))
(let morse->abc (lambda letter (get *MORSE->ABC-CODES* (as (std/vector/hash/table/get *MORSE->ABC-IDEXES* letter) Int))))


[
(|> "Hello,World.1234" (std/vector/map std/char/upper) (std/vector/map (lambda xs (|> xs (vector) (abc->morse)))) (std/vector/flat-one))
(|> 
["...." "." "._.." "._.." "___" "__..__" ".__" "___" "._." "._.." "_.." "._._._" ".____" "..___" "...__" "...._"] 
(std/vector/map morse->abc)
(std/vector/flat-one)
)]"#, "[[46 46 46 46 46 46 95 46 46 46 95 46 46 95 95 95 95 95 46 46 95 95 46 95 95 95 95 95 46 95 46 46 95 46 46 95 46 46 46 95 46 95 46 95 46 95 95 95 95 46 46 95 95 95 46 46 46 95 95 46 46 46 46 95] [72 69 76 76 79 44 87 79 82 76 68 46 49 50 51 52]]"),
(r#"
(let N 9)
(let matrix (|> (std/vector/int/zeroes N) (std/vector/map (lambda x (std/vector/map (std/vector/int/zeroes N) (lambda . 0))))))
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
                  (do (|> matrix 
                      (std/vector/map (lambda y 
                        (std/vector/map y (lambda x (cond 
                                                (= x 0) "." 
                                                (= x 1) "*"
                                                ""))))) 
                              (std/convert/vector/3d->string std/char/new-line std/char/space)))))
(|> matrix (gof) (gof) (gof) (gof) (gof) (gof) (gof) (gof))"#, "[[0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0] [0 0 0 0 1 0 0 0 0] [0 0 0 0 0 1 0 0 0] [0 0 0 1 1 1 0 0 0] [0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0]]"),

(r#"(let *RES* 51)
(integer generation 0)
(variable cells (std/vector/int/zeroes *RES*))
(let ruleset [ 0 1 0 1 1 0 1 0 ])
(set! (get cells) (/ (length (get cells)) 2) 1)
(let out [])

(let rules (lambda a b c (do 
    (let index (std/convert/bits->integer [ a b c ]))
    (get ruleset (- 7 index)))))

(loop (< (get generation) (/ *RES* 2)) (lambda (do 
    (std/vector/push! out (get cells))
    (let nextgen (std/vector/copy (get cells)))
    (loop 1 (- (length (get cells)) 1) (lambda i (do 
        (let left (get cells 0 (- i 1)))
        (let me (get cells 0 i))
        (let right (get cells 0 (+ i 1)))
        (set! nextgen i (rules left me right)))))
    (set cells nextgen)
    (++ generation))))


(|> out 
        (std/vector/map (lambda y 
            (std/vector/map y (lambda x (cond 
                                    (= x 0) "." 
                                    (= x 1) "*"
                                    "")))))
                (std/convert/vector/3d->string std/char/new-line std/char/space))
out                
                
                "#, "[[0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0] [0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0] [0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 0 0 0 0 0] [0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 0 0 0 0 0] [0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 0 0 0 0 1 0 1 0 0 0 0] [0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0] [0 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 1 0 0] [0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0]]"),
        (r#"; The Document indicates that you should start at the given coordinates (where you just landed) and face North. Then, follow the provided sequence: either turn left (L) or right (R) 90 degrees, then walk forward the given number of blocks, ending at a new intersection.
; Following R2, L3 leaves you 2 blocks East and 3 blocks North, or 5 blocks away.
; R2, R2, R2 leaves you 2 blocks due South of your starting position, which is 2 blocks away.
; R5, L5, R5, R3 leaves you 12 blocks away.

(let parse (lambda input 
    (|> 
        input 
        (std/vector/append! std/char/comma)
        (std/convert/string->vector std/char/space)
        (std/vector/map (lambda x (std/vector/drop/last x 1)))
        (std/vector/map (lambda [ D M ] [ (Char->Int D) (std/convert/chars->integer M) ])))))
(let part1 (lambda input (|> input
    (std/vector/reduce (lambda [ y x a .] [ D M .] (do
                                (let F (mod (+ a (if (=# (Int->Char D) std/char/R) 1 3)) 4))
                                (cond 
                                    (= F 0) [y (+ x M) F]
                                    (= F 1) [(- y M) x F]
                                    (= F 2) [y (- x M) F]
                                    (= F 3) [(+ y M) x F]
                                    [ y x a ])))
                                [ 0 0 1 ])
    (std/fn/apply/1 (lambda [ y x . ] (+ (std/int/abs y) (std/int/abs x)))))))

(|> ["R2, L3"  "R2, R2, R2" "R5, L5, R5, R3"] (std/vector/map parse) (std/vector/map part1))

"#, "[5 2 12]"),

        (r#"(let INPUT
"7 6 4 2 1
1 2 7 8 9
9 7 6 2 1
1 3 2 4 5
8 6 4 4 1
1 3 6 7 9")

(let parse (lambda input (|> input (std/vector/char/lines) (std/vector/map (lambda l (|> l (std/vector/char/words) (std/vector/map std/convert/chars->integer)))))))

(let part1 (lambda input (|> input 
    (std/vector/filter (lambda line (do
        (let slice (|> line 
                       (std/vector/drop/last 1)
                       (std/vector/zipper (std/vector/drop line 1))
                       (std/vector/map std/vector/int/pair/sub)))
        ; The levels are either all increasing or all decreasing.
        ; Any two adjacent levels differ by at least one and at most three.
        (or (std/vector/every? slice (lambda x (and (>= x 1) (<= x 3)))) 
            (std/vector/every? slice (lambda x (and (<= x -1) (>= x -3))))))))
    (length))))

(let part2 (lambda input (|> input
                            (std/vector/map
                              (lambda line (|> line
                                (std/vector/map/i (lambda . i
                                  (|> line (std/vector/filter/i (lambda . j (not (= i j))))))))))
                            (std/vector/count-of (lambda x (std/int/positive? (part1 x)))))))

(let PARSED (parse INPUT))

[(part1 PARSED) (part2 PARSED)]"#, "[2 4]"),
        (r#"(let INPUT "12
14
1969
100756")

(let parse (lambda input 
    (|> input (std/vector/char/lines) (std/vector/map std/convert/chars->integer))))

(let PARSED (parse INPUT))

(let part1 (lambda input (|>    
    input 
    (std/vector/map (lambda x (- (/ x 3) 2)))
    (std/vector/int/sum))))

(let part2 (lambda input (do

(let retry (lambda x (do
    (let tail-call:retry! (lambda x out (do 
        (let result (- (/ x 3) 2))
        (if (<= result 0) out 
            (tail-call:retry! result (std/vector/append! out result))))))
     (tail-call:retry! x []))))
     
 (|>
    input 
    (std/vector/map retry)
    (std/vector/map std/vector/int/sum)
    (std/vector/int/sum)))))
    
[(part1 PARSED) (part2 PARSED)]"#, "[34241 51316]"


),
(r#"(let fn (lambda { a b  } (do 
(if (= b 1) [ a  ] [ false ])
b
)))
(fn { true 3 })"#, "3"),
(r#"(std/vector/tuple/zip { [ 1 2 3 ] [ true false true ] })"#, "[[1 true] [2 false] [3 true]]"),
(r#"(std/vector/tuple/unzip 
    (std/vector/tuple/zip { [ 1 2 3 ] [ true false true ] })
)"#, "[[1 2 3] [true false true]]"),

(r#"(let rec (lambda { x y } { . b } (if (< (+ x y) b) (rec {(+ x 2) (+ y 3)} { true b } ) { false (+ x y) })))
(rec { 1 1 } { true 10 })"#, "[false 12]"),
(r#"(let factorial (lambda N (snd (pull! (Rec { N 1 } (lambda { n acc } 
      (if (= n 0) 
          { Rec/return [ { n acc } ]} 
          { Rec/push [ { (- n 1) (* acc n) } ] })))))))

(let rec-sum (lambda N (snd (get (Rec { N 0 } (lambda { n acc } 
    (if (= 0 n) 
        { Rec/return [ { n acc } ] } 
        { Rec/push [ { (- n 1) (+ acc n) } ] })))))))

(let factorialVec (lambda N (first (pull! (Rec [ 1 N ] (lambda [ acc n . ] 
      (if (= n 0) 
          { Rec/return [ [ acc n ] ]} 
          { Rec/push [ [ (* acc n) (- n 1) ] ] })))))))

(factorialVec 5)

[(factorial 5) (factorialVec 5) (rec-sum 10)]"#, "[120 120 55]"),

    (r#"(let INPUT "58:5,3,7,8,9,10,4,5,7,8,8")
(let parse (lambda input (do 
  (let parts (|> input (String->Vector ':')))
  (let head (|> (car parts) (Chars->Integer)))
  (let tail (|> (cdr parts) (car) (String->Vector ',') (map Chars->Integer)))
  { head tail })))

;  5   3-5   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7 
;                      |       |       |       |       |       |       |       |
;                      8       8-9     8-9   4-8-9   4-8-9   4-8-9   4-8-9   4-8-9
;                                      |       |       |       |       |       |
;                                      10      10    5-10    5-10    5-10    5-10
;                                                              |       |       |
;                                                              7       7-8     7-8
;                                                                              |
;                                                                              8  
; Check all segments of the spine, starting from the top.
; If your number is less than the one on the spine segment and the left side of the segment is free - place it on the left.
; If your number is greater than the one on the spine segment and the right side of the segment is free - place it on the right.
; If no suitable place is found at any segment, create a new spine segment from your number and place it as the last one.


(let part1 (lambda { . nums } (do 
(let sword [[-1 (get nums 0) -1]])
(loop 1 (length nums) (lambda i (do
  (let num (get nums i))
  (boolean placed false)
  (loop 0 (length sword) (lambda j (do 
    (let segment (get sword j))
    (cond
        (and (false? placed) (< num (get segment 1)) (= (get segment 0) -1)) (do (set! segment 0 num) (boolean/set placed true))
        (and (false? placed) (> num (get segment 1)) (= (get segment 2) -1)) (do (set! segment 2 num) (boolean/set placed true))
        nil))))
    (if (false? placed) (push! sword [-1 num -1])))))
sword)))
  
(part1 (parse INPUT))"#, "[[3 5 7] [4 8 9] [5 10 -1] [-1 7 8] [-1 8 -1]]",
),
(r#"(let scan/sum (lambda a b (do (push! a (+ (last a) b)) a)))

(let left-right-sum-diff (lambda input 
  (|> (zip {
        (|> input (reduce scan/sum [0]) (drop/last 1))
        (|> input (reverse) (reduce scan/sum [0]) (drop/last 1) (reverse))
    }) 
    (map (lambda { a b } (abs (- a b)))))))

(left-right-sum-diff [ 10 4 8 3 ])"#, "[15 1 11 22]"),
(r#"(let maximum/count (lambda xs 
      (max 
        (count xs positive?) 
        (count xs negative?))))
        
(maximum/count [ -1 -1 1 1 1 -1 1 ])"#, "4"),
(r#"(let input [
    [ 1 2 3 ]
    [ 5 5 5 ]
    [ 3 1 4 ]
])

; long version - not prefered
(let maximumWealthLong (lambda xs (reduce (map xs (lambda ys (reduce ys + 0))) (lambda a b (max a b)) -infinity)))

; Both of these are prefered 

; Pipe composition - easy to follow
(let maximumWealthPipe (lambda xs (|> xs (map sum) (maximum))))

; Point free version - most conscise
(let maximumWealthFree (\ maximum (\map sum)))

(|> ; all of these functions are [[Int]] -> Int so they can exist in the same vector
    [maximumWealthLong maximumWealthPipe maximumWealthFree] 
    (map (lambda fn (fn input))))"#, "[15 15 15]"),
    (r#"(let \filterOdd (\filter odd?))
(let maximumWealthFree (\ maximum (\map sum)))
[(|> 
    [ 1 2 3 4 5 ] 
    (\filterOdd)
    (\map square)
    (sum))
 (|> [
    [ 1 2 3 ]
    [ 5 5 5 ]
    [ 3 1 4 ]
] (maximumWealthFree))]"#, "[35 15]"),
 (r#";    halve :: [T] -> {[T] * [T]}
(let halve (lambda xs (do 
          (let half (/ (length xs) 2)) 
          { (take/first xs half) (drop/first xs half)})))

(halve (range 0 11))"#, "[[0 1 2 3 4 5] [6 7 8 9 10 11]]"), (r#"(let max-depth (lambda s (|> s 
  (filter (lambda x (or (=# x '(') (=# x ')'))))
  (map (lambda x (if (=# x '(') 1 -1)))
  (scan +)
  (maximum))))
(max-depth "(1 + (2 * 3) + ((8)/4))+1")
"#, "3"),

(r#"(let a 10)
(let b 8)
(let c 23)
(let x 42)
(let y 69)
(let r 10)
`a + b * c + (x + y) / 2 + r^2 - x^2`"#, "-1415"),
(r#"(let A { 0 { 42 { false { "" nil } } } })
(let B { 1 { 0 { true { "" nil } } } })
(let C { 2 { 0 { false { "algebraic data types" nil } } } })

(let algebraic (lambda T (do 
  (let out [])
  (let kind (fst T))
  (cond 
    (= kind 0) (push! out (* (fst (snd T)) 10))
    (= kind 1) (if (fst (snd (snd T))) (push! out -1))
    (= kind 2) (push! out (Char/count (fst (snd (snd (snd T)))) 'a')) 
  nil)
  out)))

(algebraic C)"#, "[4]"),
(r#"
(let INPUT "L68
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
    (map (lambda [d r] 
      [(if (=# d 'L') -1 1) (Chars->Integer r)])))))

(let part1 (lambda xs 
  (snd (reduce xs (lambda { dial counter } [d r .] (do 
      (let res (emod (+ dial (* d r)) 100))
      { res (+ counter (Bool->Int (= res 0))) })) { 50 0 }))))

(let part2 (lambda xs 
  (snd (reduce xs (lambda { dial counter } [d r .] (do 
      (let res (emod (+ dial (* d r)) 100))
      (let rng [dial])
      (loop 1 r (lambda i (push! rng (emod (+ (at rng -1) (* 1 d)) 100)))) 
      { res (+ counter (Int/count rng 0)) })) { 50 0 }))))

[ (part1 (parse INPUT)) (part2 (parse INPUT)) ]"#, "[3 6]"),

 (r#"(let INPUT "987654321111111
811111111111119
234234234234278
818181911112111")

(let parse (lambda input (String->Vector input nl)))
(let part1 (lambda parsed (do 
  (integer S 0)
  (|> parsed (for (lambda inp (do
    (integer M -infinity)
    (loop 0 (length inp) (lambda i 
      (loop i (length inp) (lambda j 
        (if (<> i j) 
          (set M (max (get M) (Chars->Integer [(get inp i) (get inp j)]))))))))
    (+= S (get M)))))) 
    (get S))))

[(part1 (parse INPUT))]"#, "[357]")


        ];
        let std_ast = crate::baked::load_ast();
        for (inp, out) in &test_cases {
            if let crate::parser::Expression::Apply(items) = &std_ast {
                match crate::parser::merge_std_and_program(&inp, items[1..].to_vec()) {
                    Ok(exprs) => {
                        match crate::infer::infer_with_builtins(&exprs) {
                            Ok(_) => {
                                match crate::vm::run(&exprs) {
                                    Ok(result) => {
                                        // println!("{:?}", inp);
                                        assert_eq!(format!("{:?}", result), *out, "Solution")
                                    }
                                    Err(e) => {
                                        // to figure out which test failed due to run time Error!
                                        println!("{:?}", inp);
                                        panic!("Failed tests because {}", e)
                                    }
                                }
                            }
                            Err(e) => {
                                // println!("{:?}", inp);
                                panic!("Failed tests because {}", e)
                            }
                        }
                    }
                    Err(e) => panic!("Failed tests because {}", e),
                }
            }
        }
    }
}
