; How Que Script works

; This is an anonymous function wrapping the whole example
; It has a (do) block which means the function will a body
; it will retun it's last expression
; The type signature of this function is () -> [Int]
; Unit to a Vector of Ints or a function that has no arguments
; that returns a list of integers

(lambda (do

; Hindley–Milner Type Inference

; - No type annotations required: the compiler figures everything out.
; - Supports polymorphism and higher-order functions.
; - Only 5 types - functions, booleans, integers, characters and vectors.
; - Guarantees soundness: if your program compiles, it won’t have type errors at runtime.
; - Example:

(let sum-odd-squares (lambda xs
(|> xs
(std/vector/filter std/int/odd?)
(std/vector/map std/int/square)
(std/vector/int/sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])
; Int
; 165

; - filter, map and sum will be tree shaked from std.
; - Pipe (|> ... ) will be desuggered to:

(std/vector/int/sum (std/vector/map (std/vector/filter [ 1 2 3 4 5 6 7 8 9 10 ] std/int/odd?) std/int/square))

; - Argument type of the function will be [Int].
; - Return type of the function will be Int.
; - filter will only work with [Int] and callback of type Int -> Bool
; - map will only work with [Int] and callback of type Int -> Int
; - sum will only work with [Int]

; For quick scripting some of these functions are aliased

(|> (range 0 10)
(filter (lambda x (= (mod x 2) 1)))
(map square)
(sum))

; full list of aliased functions:
sum
nl ; new-line
map
filter
reduce
every?
some?
empty?
reverse
slice
cons ; merge two lists
sort! ; inplace quick sort
range
square
expt ; exponent
max
min
push!
pull! ; pop! and get
abs ; absolute
first
last
car ; first
cdr ; rest
count

; Tail Call Optimization
; A call is said to be in tail position if it is the last instruction executed before returning from the current function.
; Compilers can optimize such calls by discarding the caller frame and replacing the call with a jump.
; This is especially useful for recursive functions. For instance, take this function that sums the elements of a vector:

(let sum (lambda xs acc
(if (= (length xs) 0) acc
(sum (std/vector/drop xs 1) (+ acc (get xs 0))))))

(sum [ 1 2 3 4 5 ] 0)
; Int
; 15

; With a regular call, this consumes 𝒪(n) stack space: each element
; of the vector adds a new frame on the call stack.
; With a long enough vector, this could very quickly overflow the stack.
; By replacing the call with a jump, tail call optimization
; effectively turns this recursive function into a loop which uses 𝒪(1) stack space:

(let tco-sum (lambda xs acc (do
(let \_acc [ acc ])
(let \_xs [ xs ])
(let \_new_xs [])
(let \_new_acc [])
(loop (not (= (length (get \_xs 0)) 0)) (lambda (do
(set! \_new_xs 0 (std/vector/drop (get \_xs 0) 1))
(set! \_new_acc 0 (+ (get \_acc 0) (get (get \_xs 0) 0)))
(set! \_xs 0 (get \_new_xs 0))
(set! \_acc 0 (get \_new_acc 0)))))
(get \_acc 0))))

; This optimization is particularly important for functional languages.
; They rely heavily on recursive functions,
; and pure ones like Haskell don’t even provide loop control structures.
; Any kind of custom iteration typically uses recursion one way or another.
; Without tail call optimization,
; this would very quickly run into a stack overflow for any non-trivial program:

; TCO recursion
(let k-mod (lambda n k (if (< k n) k (k-mod n (- k n)))))
; taking advantage of partial apply
(let mod2 (k-mod 2))
; TCO recursion
(let collatz (lambda n steps
(if (= n 1)
steps
(collatz (if (= (mod2 n) 0)
(/ n 2)
(+ (* n 3) 1))
(+ steps 1)))))

(collatz 27 0)
; Int
; 111

; Tail Call Optimization Convention

; When a function is transformed for tail-call optimization,
; the last parameter is treated as the accumulator/result parameter.
; On entry, the compiler initializes the result register from this last parameter.
; On each tail-call, the accumulator is updated before the next iteration.
; This guarantees that type inference can resolve the result type statically.
; Users are encouraged to design tail-recursive functions
; so that the accumulator is the last argument.

; Cast
; An empty vector has a polymorphic type (it can contain anything):

(let xs [])
xs
; [t7]
; []

; To enforce a type we can use as:

(let ys (as [] [Int]))
ys
; [Int]
; []

; Now the vector can only have Ints and will error out if anything else is pushed to it.

; Loop Limit
; Loops are capped at 5,000,000 (five million) total iterations for the entire program.
; To ensure programs remain safe when running locally or on shared servers,
; loops must be "safe" and unable to hang or block the main thread.
; This limit also applies to tail-call optimized recursion.

; Starting in the top left corner of a 2x2 grid,
; and only being able to move to the right and down,
; there are exactly 6 routes to the bottom right corner:

(let factorial (lambda n total
(if (= n 0)
total
(factorial (- n 1) (* total n)))))

(let bionomial-coefficient (lambda a b
(/ (factorial a 1)
(*
(factorial b 1)
(factorial (- a b) 1)))))

(let m 2)
(let n 2)
(bionomial-coefficient (+ m n) m)
; Int
; 6

; How many such routes are there through a 20x20 grid?
; Unfortunately, we can't fit that number in 32 big integers.
; Instead we have to use Big integers (or numbers as a vectors with arbitrary precision):

(let int/big/factorial (lambda n total
(if (= (get n 0) 0)
total
(int/big/factorial (std/int/big/sub n [ 1 ]) (std/int/big/mul total n)))))

(let int/big/bionomial-coefficient (lambda a b
(std/int/big/div (int/big/factorial a [ 1 ])
(std/int/big/mul
(int/big/factorial b [ 1 ])
(int/big/factorial (std/int/big/sub a b) [ 1 ])))))

(let int/big/m [ 2 0 ])
(let int/big/n [ 2 0 ])
(int/big/bionomial-coefficient (std/int/big/add int/big/m int/big/n) int/big/m)
; [Int]
; [1 3 7 8 4 6 5 2 8 8 2 0]

; Advent of Code 2015

; --- Day 1: Not Quite Lisp ---

; Santa is trying to deliver presents in a large apartment building,
; but he can't find the right floor - the directions he got are a little confusing.
; He starts on the ground floor (floor 0) and then follows the instructions one character at a time.
; An opening parenthesis, (, means he should go up one floor,
; and a closing parenthesis, ), means he should go down one floor.
; The apartment building is very tall, and the basement is very deep;
; he will never find the top or bottom floors.

; For example:

; (()) and ()() both result in floor 0.
; ((( and (()(()( both result in floor 3.
; ))((((( also results in floor 3.
; ()) and ))( both result in floor -1 (the first basement level).
; ))) and )())()) both result in floor -3.
; To what floor do the instructions take Santa?

(let samples [
"(())" ; result in floor 0.
"()()" ; result in floor 0.
"(((" ; result in floor 3.
"(()(()(" ; result in floor 3.
"))(((((" ; also results in floor 3.
"())" ; result in floor -1 (the first basement level).
"))(" ; result in floor -1 (the first basement level).
")))" ; result in floor -3.
")())())" ; result in floor -3.
])
(let solve (lambda input (- (std/vector/char/count input std/char/left-brace) (std/vector/char/count input std/char/right-brace))))
(std/vector/map samples solve)
; [Int]
; [0 0 3 3 3 -1 -1 -3 -3]
))

