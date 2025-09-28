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
            ("(do (let x 10) (let fn (lambda (do (let x 2) (* x x)))) (fn))", "Int"),
            ("(do (let true (= 1 1)) (let false (= 1 0)) (let fn (lambda a b c d (do (set! d (length d) (if c (lambda x (> (+ a b) x)) (lambda . false))) (> (length d) 10)))) fn)", "Int -> Int -> Bool -> [Int -> Bool] -> Bool"),
            ("(do (let T (lambda x x)) (let xs (vector (T 0))) xs)", "[Int]")
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
            ("(std:vector:int:sum [ 1 2 ])", "3"),
            (
                r#"(import filter map std:vector)
(import odd? square std:int)
(import sum std:vector:int)

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
    int:sum int:maximum int:minimum int:range
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
  (int:sum))))
 
 (let Apart2 (lambda input (|>
  input
  (cons (slice input 0 (/ (length input) 2)))
  (sliding-window (+ (/ (length input) 2) 1))
  (filter (lambda xs (= (first xs) (last xs))))
  (map first)
  (int:sum))))
 
 (let parse (lambda input (|>
 input
 (string->vector char:new-line)
 (map (lambda xs (|>
                  xs
                  (map (lambda x (if (= x char:tab) char:space x)))
                  (string->vector char:space)
                  (map chars->integer)))))))
 
 (let part1 (lambda input (|> input (map (lambda xs (- (int:maximum xs) (int:minimum xs)))) (int:sum))))
 
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
 (int:sum))))
 
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
 (sort! (int:range 1 10) >=)
 ]
 )"#,
                "[111 3628800 3 4 0 9 6 0 4 12 4 18 9 10 9 8 7 6 5 4 3 2 1]",
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
(let solve (lambda input (- (std:vector:int:count input std:int:char:left-brace) (std:vector:int:count input std:int:char:right-brace))))
(std:vector:map samples solve)"#,
                "[0 0 3 3 3 -1 -1 -3 -3]",
            ),
            (
                r#"(let last-stone-weight (lambda stones (do
  (let max-cmp (lambda a b (> a b)))
  (let heap (std:convert:vector->heap stones max-cmp))
  (let tail-call:smash (lambda t
    (if (> (length heap) 1)
      (do
        (let y (std:heap:peek heap))
        (std:heap:pop! heap max-cmp)
        (let x (std:heap:peek heap))
        (std:heap:pop! heap max-cmp)
        (if (!= x y)
          (std:heap:push! heap (- y x) max-cmp))
        (tail-call:smash t))
        false)))
  (tail-call:smash true)
  (if (> (length heap) 0) (std:heap:peek heap)))))

[(last-stone-weight [ 2 7 4 1 8 1 ]) (last-stone-weight [ 1 ])]"#,
                "[1 1]",
            ),
            (
                r#"(let has-groups (lambda deck
  (do
    (let counts (|> deck
                    ; TODO:
                    ; delete next 2 lines
                    ; replace them with std:convert:integer->string
                    ; once that gets impelemnted
                    (std:vector:map std:convert:digit->char)
                    (std:vector:map int)
                    ; ^ to be replaced with std:convert:integer->string
                    (std:vector:hash:table:count)
                    (std:vector:hash:table:entries)
                    (std:vector:map std:vector:second)
                    (std:vector:flat-one)))
    (> (std:vector:reduce counts std:int:gcd (std:vector:first counts)) 1))))

[
    (has-groups [ 1 2 3 4 4 3 2 1 ]) ; Output: true
    (has-groups [ 1 1 1 2 2 2 3 3 ]) ; Output: false
]"#,
                "[1 0]",
            ),
            (
                r#"
            (let find-missing-numbers (lambda nums (|> 
    (std:vector:int:range 1 (length nums)) 
    (std:vector:map (lambda x (std:convert:integer->string-base x 10)))
    (std:convert:vector->set)
    (std:vector:hash:set:difference (|> nums (std:vector:map (lambda x (std:convert:integer->string-base x 10))) (std:convert:vector->set)))
    (std:vector:flat-one)
    (std:vector:map std:convert:chars->integer))))

[
    (find-missing-numbers [ 4 3 2 7 8 2 3 1 ]) ; Output: [5 6]
    (find-missing-numbers [ 1 1 ])             ; Output: [2]
]
            "#,
                "[[5 6] [2]]",
            ),
            (
                r#"(let has-trailing-zeros (lambda nums (>= (std:vector:count-of nums (lambda x (= (mod x 2) 0))) 2)))

[(has-trailing-zeros [ 1 2 3 4 5 ]) ; Should return true
 (has-trailing-zeros [ 2 4 8 16 ]) ; Should return true
 (has-trailing-zeros [ 1 3 5 7 9 ]) ; Should return false
 (has-trailing-zeros [ 1 2 ])]  ; Should return false
"#,
                "[1 1 0 0]",
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
            (let n (length (std:vector:first image)))
            (let stack [[sr sc]])
            (loop (std:vector:not-empty? stack) (lambda (do 
                (let t (std:vector:pop-and-get! stack))
                (let i (std:vector:first t))
                (let j (std:vector:second t))
                (if (and (>= i 0) (< i m) (>= j 0) (< j n) (= (. image i j) old)) (do
                    (std:vector:3d:set! image i j color)
                    (std:vector:push! stack [(+ i 1) j])
                    (std:vector:push! stack [(- i 1) j])
                    (std:vector:push! stack [i (+ j 1)])
                    (std:vector:push! stack [i (- j 1)])
                    nil)))))
        image)))))


(let image [[1 1 1] [1 1 0] [1 0 1]])
(flood-fill image 1 1 2)
; Output: [[2 2 2] [2 2 0] [2 0 1]]"#,
                "[[2 2 2] [2 2 0] [2 0 1]]",
            ),
            (
                r#"(let valid-path (lambda n edges source destination (do
  (if (= source destination) true
    (do
      (let graph (std:vector:map (std:vector:int:zeroes n) (lambda . [])))
      (std:vector:for edges (lambda edge (do
        (let u (get edge 0))
        (let v (get edge 1))
        (std:vector:push! (get graph u) v)
        (std:vector:push! (get graph v) u))))
      (let visited (std:vector:int:zeroes n))
      (let queue [source])
      (std:vector:set! visited source 1)
      (boolean found false)
      (loop (and (not (true? found)) (> (length queue) 0))
        (lambda (do
          (let current (std:vector:pop-and-get! queue))
          (if (= current destination)
            (boole-set found true)
            (std:vector:for (get graph current) (lambda neighbor (do
              (if (= (get visited neighbor) 0)
                (do
                  (std:vector:set! visited neighbor 1)
                  (std:vector:push! queue neighbor) 
                  nil)))))))))
      (true? found))))))

[(valid-path 3 [[ 0 1 ] [ 1 2 ] [ 2 0 ]] 0 2) ; Should return true
 (valid-path 6 [[ 0 1 ] [ 0 2 ] [ 3 5 ] [ 5 4 ] [ 4 3 ]] 0 5)] ; Should return false"#,
                "[1 0]",
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
(let yx->key (lambda y x (std:vector:concat-with (std:vector:map [ y x ] (lambda c [ c ])) std:int:char:dash)))
(let parse (lambda input (|> input (std:convert:string->vector std:int:char:new-line) (std:vector:map std:convert:chars->digits))))
(let part1 (lambda matrix (do
  (let coords (std:vector:3d:points matrix std:int:zero?))
  (let default-queue-value [ 0 ])
  (std:vector:reduce coords (lambda a xs (do
        (integer score 0)
        (let y (std:vector:first xs))
        (let x (std:vector:second xs))
        (let visited (std:vector:buckets 8))
        (let queue (std:vector:queue:new default-queue-value))
        (let current (. matrix y x))
        (std:vector:hash:set:add! visited (yx->key y x))
        (std:vector:queue:enqueue! queue [ y x ])
        
        (loop (std:vector:queue:not-empty? queue) (lambda (do
            (let element (std:vector:queue:peek queue))
            (std:vector:queue:dequeue! queue  default-queue-value)
            (let y (std:vector:first element))
            (let x (std:vector:second element))  
            (std:vector:3d:adjacent matrix std:vector:3d:von-neumann-neighborhood y x (lambda cell dir dy dx (do
                 (let key (yx->key dy dx))
                 (if (and (= (- cell (. matrix y x)) 1) (not (std:vector:hash:set:has? visited key))) (do
                    (if (= cell 9) (do (++ score) nil) (do (std:vector:queue:enqueue! queue [ dy dx ]) nil))
                    (std:vector:hash:set:add! visited key)
                    nil))))))))

        (+ a (get score)))) 0))))

(let part2 (lambda matrix (do
  (let coords (std:vector:3d:points matrix std:int:zero?))
  (let default-queue-value [ 0 ])
  (std:vector:reduce coords (lambda a xs (do
        (integer score 0)
        (let y (std:vector:first xs))
        (let x (std:vector:second xs))
        (let visited (std:vector:buckets 8))
        (let queue (std:vector:queue:new default-queue-value))
        (let current (. matrix y x))
        (let root-key (yx->key y x))
        (std:vector:hash:table:set! visited root-key 1)
        (std:vector:queue:enqueue! queue [ y x ])
        (loop (std:vector:queue:not-empty? queue) (lambda (do
            (let element (std:vector:queue:peek queue))
            (let y (std:vector:first element))
            (let x (std:vector:second element))  
            (if (= (. matrix y x) 9) (+= score (std:vector:hash:table:get visited root-key)))
            (std:vector:queue:dequeue! queue default-queue-value)
            (std:vector:3d:adjacent matrix std:vector:3d:von-neumann-neighborhood y x (lambda cell dir dy dx (do
                 (let key (yx->key dy dx))
                 (if (= (- cell (. matrix y x)) 1) (do
                    (std:vector:queue:enqueue! queue [ dy dx ])
                    (if (std:vector:hash:table:has? visited key) 
                        (std:vector:hash:table:set! visited key (+ (std:vector:hash:table:get visited root-key) (std:vector:hash:table:get visited key))) 
                        (std:vector:hash:table:set! visited key (std:vector:hash:table:get visited root-key)))
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
                            (std:string:lines)
                            (std:vector:map (lambda word (|>
                                                      word
                                                      (std:string:words)
                                                      (std:vector:filter std:vector:not-empty?)
                                                      (std:vector:map std:convert:chars->integer)))))))

(let part1 (lambda input (|>
                          input
                          (std:vector:unzip)
                          (std:vector:map std:vector:sort:desc!)
                          (std:vector:zip)
                          (std:vector:map std:vector:int:pair:sub)
                          (std:vector:map std:int:abs)
                          (std:vector:int:sum))))
                        
(let part2 (lambda input (do
  (let unzipped (std:vector:unzip input))
  (let left (std:vector:first unzipped))
  (let right (std:vector:second unzipped))
  (|>
    left
    (std:vector:map (lambda l (* l (std:vector:count-of right (lambda r (= l r))))))
    (std:vector:int:sum)))))

(let PARSED (parse INPUT))
[(part1 PARSED) (part2 PARSED)]"#,
                "[11 31]",
            ),
            (
                r#"
(let parse (lambda input (|> input (std:string:lines) (std:vector:map std:convert:chars->integer))))
(let part1 (lambda input (do 
    (let min (std:vector:int:minimum input))
    (|> input
        (std:vector:map (lambda x (- x min)))
        (std:vector:int:sum)))))

 (let part2 (lambda input (do 
    (std:vector:sort:desc! input)
    (let median (std:vector:int:median input))
    (|> input
        (std:vector:map (lambda x (cond (> x median) (- x median) (< x median) (- median x) 0)))
        (std:vector:int:sum)))))

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
    (std:vector:for xs (lambda x (if (<> x 0) (do 
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
; Input : [1, 2, 4]
; Output : 125
; Explanation: 124 + 1 = 125 

; Input : [9, 9, 9]
; Output: 1000
; Explanation: 999 + 1 = 1000 

[
    (+ (std:convert:digits->integer [ 1 2 4 ]) 1)
    (+ (std:convert:digits->integer [ 9 9 9 ]) 1)
]
            "#,
                "[125 1000]",
            ),
            ("(std:convert:bits->integer [ 1 0 0 0 0 0 1 1 0 0 ])", "524"),
            (
                r#"(let xs [ 1 2 3 ])
(let copy (std:vector:copy xs))
(set! copy 0 1000)
[ xs copy ]"#,
                "[[1 2 3] [1000 2 3]]",
            ),
            (
                r#"(let sort-array-by-parity2 (lambda nums (if (std:vector:empty? nums) nums (do 
    (let odd [])
    (let even [])
    (let out [])
    (loop 0 (length nums) (lambda i (std:vector:push! (if (std:int:even? i) even odd) (. nums i))))
    (loop 0 (length even) (lambda i (do (std:vector:push! out (. even i)) (std:vector:push! out (. odd i)))))
    out))))

[
  (sort-array-by-parity2 [ 4 2 5 7 ])
  (sort-array-by-parity2 [ 2 3 ])
  (sort-array-by-parity2 [ 4 3 ])
]"#,
                "[[4 2 5 7] [2 3] [4 3]]",
            ),
            ("(std:int:collinear? [[ 3 8 ] [ 5 10 ] [ 7 12 ]])", "1"),
            (
                r#"(let fn (lambda [ a b c r ] (+ a b c (std:vector:int:product r))))
(fn [ 1 2 3 4 5 6 ])"#,
                "126",
            ),
            (
                r#"(let input "A:+,-,=,=,+,-,=,=,+,-
B:+,=,-,+,+,=,-,+,+,=
C:=,-,+,+,=,-,+,+,=,-
D:=,=,=,+,=,=,=,+,=,=")

(let parse (lambda input (do 
(|> input (std:string:lines) (std:vector:map (lambda x (do 
    (let y (std:string:commas x))
    (set! y 0 (get (std:convert:string->vector (get y 0) std:int:char:colon) 1))
    (std:vector:flat-one y)))))
)))
    
(let app (lambda a x 
    (cond (= x std:int:char:plus) (std:vector:append! a (+ (std:vector:last a) 1))
    (= x std:int:char:minus) (std:vector:append! a (- (std:vector:last a) 1))
    (= x std:int:char:equal) (std:vector:append! a (std:vector:last a))
    (std:vector:append! a (std:vector:last a)))))
(let part1 (lambda xs (do
    (let letters (|> input (std:string:lines) (std:vector:map std:vector:first)))
    (|> xs (std:vector:map (lambda x (|> x (std:vector:reduce app [0])))) 
    (std:vector:map std:vector:int:sum)
    (std:vector:map:i (lambda x i [i (+ x 100)]))
    (std:vector:sort! (lambda a b (> (. a 1) (. b 1))))
    (std:vector:map (lambda [i .] (. letters i)))))))
(|> input (parse) (part1))"#,
                "[66 68 67 65]",
            ),
            (
                r#"(let palindrome? (lambda str (do 
    (let q (std:vector:queue:new 0))
    (let s (std:vector:stack:new 0))
    
    (std:vector:for str (lambda x (do
        (std:vector:stack:push! s x)
        (std:vector:queue:enqueue! q x))))
    
    (let p? [true])
    
    (loop 0 (/ (length str) 2) (lambda . 
        (if (!= (std:vector:stack:peek s) (std:vector:queue:peek q)) 
             (boole-set p? false) 
             (do 
                 (std:vector:stack:pop! s 0)
                 (std:vector:queue:dequeue! q 0)
                 nil))))
    (get p?))))
    
[(palindrome? "racecar") (palindrome? "yes")]"#,
                "[1 0]",
            ),
            (
                r#"(let palindrome? (lambda str (do 
    (let p? [true])
    (loop 0 (/ (length str) 2) (lambda i (if (<> (get str i) (get str (- (length str) i 1))) (boole-set p? false))))
    (true? p?))))
[(palindrome? "racecar") (palindrome? "yes")]"#,
                "[1 0]",
            ),
            (
                r#"(let palindrome? (lambda str (std:vector:string:match? str (std:vector:reverse str))))
[(palindrome? "racecar") (palindrome? "yes")]"#,
                "[1 0]",
            ),
            (
                r#"(let reverse (lambda xs rev 
    (if (std:vector:empty? xs) 
         rev 
        (reverse (std:vector:drop:last xs 1) (std:vector:append! rev (-. xs 1))))))
;
(reverse [ 1 2 3 4 5 ] [])"#,
                "[5 4 3 2 1]",
            ),
            (
                r#"
[

(std:int:big:div [ 1 0 ] [ 5 ])
(std:int:big:add [ 9 9 9 ] [ 1 2 ])
(std:int:big:sub [ 1 0 1 ] [ 1 1 ])
(std:int:big:mul [ 2 ] [ 9 9 5 ])

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
            (set max (std:int:max (* (- (get j) (get i)) (get xs (get j))) (get max)))
            (-- j)) (do
            (set max (std:int:max (* (- (get j) (get i)) (get xs (get i))) (get max)))
            (++ i))))))
    (get max))))

[
    (fn [ 1 8 6 2 5 4 8 3 7 ]) ; 49
    (fn [ 1 1 ]) ; 1
]"#,
                "[49 1]",
            ),
        ];
        let std_ast = crate::baked::load_ast();
        for (inp, out) in &test_cases {
            if let crate::parser::Expression::Apply(items) = &std_ast {
                match crate::parser::merge_std_and_program(&inp, items[1..].to_vec()) {
                    Ok(exprs) => {
                        let result = crate::vm::run(&exprs);
                        assert_eq!(format!("{:?}", result), *out, "Solution");
                    }
                    Err(e) => panic!("Failed tests because {}", e),
                }
            }
        }
    }
}
