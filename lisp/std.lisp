(let std/int/char/A 65)
(let std/int/char/B 66)
(let std/int/char/C 67)
(let std/int/char/D 68)
(let std/int/char/E 69)
(let std/int/char/F 70)
(let std/int/char/G 71)
(let std/int/char/H 72)
(let std/int/char/I 73)
(let std/int/char/J 74)
(let std/int/char/K 75)
(let std/int/char/L 76)
(let std/int/char/M 77)
(let std/int/char/N 78)
(let std/int/char/O 79)
(let std/int/char/P 80)
(let std/int/char/Q 81)
(let std/int/char/R 82)
(let std/int/char/S 83)
(let std/int/char/T 84)
(let std/int/char/U 85)
(let std/int/char/V 86)
(let std/int/char/W 87)
(let std/int/char/X 88)
(let std/int/char/Y 89)
(let std/int/char/Z 90)
(let std/int/char/a 97)
(let std/int/char/b 98)
(let std/int/char/c 99)
(let std/int/char/d 100)
(let std/int/char/e 101)
(let std/int/char/f 102)
(let std/int/char/g 103)
(let std/int/char/h 104)
(let std/int/char/i 105)
(let std/int/char/j 106)
(let std/int/char/k 107)
(let std/int/char/l 108)
(let std/int/char/m 109)
(let std/int/char/n 110)
(let std/int/char/o 111)
(let std/int/char/p 112)
(let std/int/char/q 113)
(let std/int/char/r 114)
(let std/int/char/s 115)
(let std/int/char/t 116)
(let std/int/char/u 117)
(let std/int/char/v 118)
(let std/int/char/w 119)
(let std/int/char/x 120)
(let std/int/char/y 121)
(let std/int/char/z 122)
(let std/int/char/0 48)
(let std/int/char/1 49)
(let std/int/char/2 50)
(let std/int/char/3 51)
(let std/int/char/4 52)
(let std/int/char/5 53)
(let std/int/char/6 54)
(let std/int/char/7 55)
(let std/int/char/8 56)
(let std/int/char/9 57)
(let std/int/char/empty 0)
(let std/int/char/double-quote 34)
(let std/int/char/new-line 10)
(let std/int/char/space 32)
(let std/int/char/tab 9)
(let std/int/char/comma 44)
(let std/int/char/dot 46)
(let std/int/char/semi-colon 59)
(let std/int/char/colon 58)
(let std/int/char/dash 45)
(let std/int/char/left-brace 40)
(let std/int/char/right-brace 41)
(let std/int/char/curly-left-brace 123)
(let std/int/char/curly-right-brace 125)
(let std/int/char/left-bracket 91)
(let std/int/char/right-bracket 93)
(let std/int/char/pipe 124)
(let std/int/char/hash 35)
(let std/int/char/question-mark 63)
(let std/int/char/exclamation-mark 33)
(let std/int/char/minus 45)
(let std/int/char/plus 43)
(let std/int/char/equal 61)
(let std/int/char/asterix 42)
(let std/int/char/ampersand 38)
(let std/int/char/at 64)
(let std/int/char/backtick 96)
(let std/int/char/digit? (lambda ch (and (>= ch std/int/char/0) (<= ch std/int/char/9))))
(let std/vector/string/upper (lambda char (if (and (>= char std/int/char/a) (<= char std/int/char/z)) (- char std/int/char/space) char)))
(let std/vector/string/lower (lambda char (if (and (>= char std/int/char/A) (<= char std/int/char/Z)) (+ char std/int/char/space) char)))
(let std/vector/length (lambda xs (length xs)))
(let std/vector/get (lambda xs i (get xs i)))
(let std/vector/get/default (lambda xs i def (if (< i (length xs)) (get xs i) def)))
(let std/vector/pop! (lambda xs (pop! xs)))
(let std/vector/set! (lambda xs i x (set! xs i x)))
(let std/vector/swap! (lambda xs i j (do (let temp (get xs i)) (std/vector/set! xs i (get xs j)) (std/vector/set! xs j temp) xs)))
(let std/vector/push! (lambda xs x (do (std/vector/set! xs (length xs) x) xs)))
(let std/vector/pop-and-get! (lambda xs (do 
      (let out (get xs (- (length xs) 1))) 
      (std/vector/pop! xs)
      out)))
(let std/vector/push-and-get! (lambda xs x (do (std/vector/set! xs (length xs) x) x)))
(let std/vector/update! (lambda xs i value (do (set! xs i value) xs)))
(let std/vector/tail! (lambda xs (do (std/vector/pop! xs) xs)))
(let std/vector/append! (lambda xs x (do (std/vector/push! xs x) xs)))
(let std/vector/at (lambda xs i (if (< i 0) (get xs (+ (length xs) i)) (get xs i))))
(let std/vector/first (lambda xs (get xs 0)))
(let std/vector/second (lambda xs (get xs 1)))
(let std/vector/third (lambda xs (get xs 3)))
(let std/vector/last (lambda xs (get xs (- (length xs) 1))))
(let std/int/max-safe 2147383647)
(let std/int/min-safe -2147483648)
(let std/int/safe? (lambda value (and (>= value std/int/min-safe) (<= value std/int/max-safe))))
(let std/int/get-safe (lambda vrbl (if (std/int/safe? (get vrbl)) (get vrbl) Int)))

; Extra "keywords" 
(let identity (lambda x x))
(let Int 0)
(let Bool false)
(let as (lambda . t t))
(let true (= 1 1))
(let false (= 0 1))
(let nil (loop 0 0 (lambda . 0)))
(let eq (lambda a b (cond 
          (and a b) true 
          (and (not a) (not b)) true
          false)))
(let +. (lambda xs index (get xs index)))
(let -. (lambda xs index (get xs (- (length xs) index))))
(let int (lambda value (if (std/int/safe? value) [ value ] [ 0 ])))
(let box (lambda value [ value ]))
(let set (lambda vrbl x (std/vector/set! vrbl 0 x)))
(let =! (lambda vrbl x (std/vector/set! vrbl 0 x)))
(let boole-set (lambda vrbl x (std/vector/set! vrbl 0 (if x true false))))
(let boole-eqv (lambda a b (eq (get a) (get b))))
(let true? (lambda vrbl (if (get vrbl) true false)))
(let false? (lambda vrbl (if (get vrbl) false true)))
(let += (lambda vrbl n (=! vrbl (+ (get vrbl) n))))
(let -= (lambda vrbl n (=! vrbl (- (get vrbl) n))))
(let *= (lambda vrbl n (=! vrbl (* (get vrbl) n))))
(let /= (lambda vrbl n (=! vrbl (/ (get vrbl) n))))
(let ++ (lambda vrbl (=! vrbl (+ (get vrbl) 1))))
(let -- (lambda vrbl (=! vrbl (- (get vrbl) 1))))
(let ** (lambda vrbl (=! vrbl (* (get vrbl) (get vrbl)))))

(let std/fn/apply-0 (lambda fn (fn)))
(let std/fn/apply-1 (lambda x fn (fn x)))
(let std/fn/apply-2 (lambda x y fn (fn x y)))
(let std/fn/apply-3 (lambda x y z fn (fn x y z)))
(let std/fn/const (lambda x . x))

(let std/vector/empty? (lambda xs (= (length xs) 0)))
(let std/vector/empty! (lambda xs (if (std/vector/empty? xs) xs (do 
     (loop 0 (length xs) (lambda . (std/vector/pop! xs)))
     xs))))
(let std/vector/not-empty? (lambda xs (not (= (length xs) 0))))
(let std/vector/in-bounds? (lambda xs index (and (< index (length xs)) (>= index 0))))
(let std/vector/for (lambda xs fn (loop 0 (length xs) (lambda i (fn (get xs i))))))
(let std/vector/filter (lambda xs fn? (if (std/vector/empty? xs) xs (do 
     (let out [])
     (let process (lambda i (do
      (let x (get xs i))
      (if (fn? x) (std/vector/set! out (length out) x)))))
     (loop 0 (length xs) process)
     out))))

(let std/vector/filter/i (lambda xs fn? (if (std/vector/empty? xs) xs (do 
     (let out [])
     (let process (lambda i (do
      (let x (get xs i))
      (if (fn? x i) (std/vector/set! out (length out) x)))))
     (loop 0 (length xs) process)
     out))))

(let std/vector/reduce (lambda xs fn initial (do
     (let out [ initial ])
     (let process (lambda i (std/vector/set! out 0 (fn (get out 0) (get xs i)))))
     (loop 0 (length xs) process)
     (get out))))

(let std/vector/reduce/i (lambda xs fn initial (do
     (let out [ initial ])
     (let process (lambda i (std/vector/set! out 0 (fn (get out 0) (get xs i) i))))
     (loop 0 (length xs) process)
     (get out))))

(let std/vector/map (lambda xs fn (if (std/vector/empty? xs) [] (do
     (let out [(fn (get xs 0))])
     (let process (lambda i (std/vector/set! out (length out) (fn (get xs i)))))
     (loop 1 (length xs) process)
     out))))

(let std/vector/map/i (lambda xs fn (if (std/vector/empty? xs) [] (do
     (let out [(fn (get xs 0) 0)])
     (let process (lambda i (std/vector/set! out (length out) (fn (get xs i) i))))
     (loop 1 (length xs) process)
     out))))

(let std/vector/int/range (lambda start end (do
     (let out [ start ])
     (let process (lambda i (std/vector/set! out (length out) i)))
     (loop (+ start 1) (+ end 1) process)
     out))) 

 (let std/vector/int/ones (lambda n (do
     (let out [ 1 ])
     (let process (lambda i (std/vector/set! out (length out) 1)))
     (loop 0 n process)
     out))) 

 (let std/vector/int/zeroes (lambda n (do
     (let out [ 0 ])
     (let process (lambda i (std/vector/set! out (length out) 0)))
     (loop 0 n process)
     out))) 

(let std/vector/count-of (lambda xs fn? (length (std/vector/filter xs fn?))))
(let std/vector/int/count (lambda input item (std/vector/count-of input (lambda x (= x item)))))

(let std/vector/cons (lambda a b (cond (std/vector/empty? a) b (std/vector/empty? b) a (do 
  (let out []) 
  (loop 0 (length a) (lambda i (std/vector/set! out (length out) (get a i)))) 
  (loop 0 (length b) (lambda i (std/vector/set! out (length out) (get b i)))) 
  out))))

(let std/vector/cons! (lambda a b (if (and (std/vector/empty? a) (std/vector/empty? b)) a (do 
  (loop 0 (length b) (lambda i (std/vector/set! a (length a) (get b i)))) 
  a))))

(let std/vector/concat (lambda xs (std/vector/reduce xs std/vector/cons [])))
(let std/vector/concat! (lambda xs os (std/vector/reduce os std/vector/cons! xs)))

(let std/vector/every? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop (and (< (get i) len) (predicate? (get xs (get i)))) (lambda (std/vector/set! i 0 (+ (get i) 1))))
           (not (> len (get i))))))

(let std/vector/some? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop (and (< (get i) len) (not (predicate? (get xs (get i))))) (lambda (std/vector/set! i 0 (+ (get i) 1))))
           (or (= len 0) (> len (get i))))))

(let std/vector/every/i? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop (and (< (get i) len) (predicate? (get xs (get i)) (get i))) (lambda (std/vector/set! i 0 (+ (get i) 1))))
           (not (> len (get i))))))

(let std/vector/some/i? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop (and (< (get i) len) (not (predicate? (get xs (get i)) (get i)))) (lambda (std/vector/set! i 0 (+ (get i) 1))))
           (or (= len 0) (> len (get i))))))

(let std/vector/cartesian-product (lambda a b (std/vector/reduce a (lambda p x (std/vector/cons p (std/vector/map b (lambda y [ x y ])))) [])))

(let std/int/gcd (lambda a b (do
    (integer A a)
    (integer B b)
    (loop (> (get B) 0) (lambda (do
        (let a (get A))
        (let b (get B))
        (set A b)
        (set B (mod a b)))))
    (get A))))
(let std/int/lcm (lambda a b (/ (* a b) (std/int/gcd  a b))))

(let std/int/bit/set? (lambda n pos (= (& n (<< 1 pos)) 0)))
(let std/int/bit/set (lambda n pos (| n (<< 1 pos))))
(let std/int/bit/clear (lambda n pos (& n (~ (<< 1 pos)))))
(let std/int/bit/power-of-two (lambda n (<< 2 (- n 1))))
(let std/int/bit/odd? (lambda n (= (& n 1) 1)))
(let std/int/bit/even? (lambda n (= (& n 1) 0)))
(let std/int/bit/average (lambda a b (>> (+ a b) 1)))
(let std/int/bit/flag-flip (lambda x (- 1 (* x x))))
(let std/int/bit/toggle (lambda n a b (^ (^ a b) n)))
(let std/int/bit/same-sign? (lambda a b (>= (^ a b) 0)))
(let std/int/bit/max (lambda a b (- a (& (- a b) (>> (- a b) 31)))))
(let std/int/bit/min (lambda a b (- a (& (- a b) (>> (- b a) 31)))))
(let std/int/bit/equal? (lambda a b (< (^ a b) 1)))
(let std/int/bit/modulo (lambda numerator divisor (& numerator (- divisor 1))))
(let std/int/bit/n-one? (lambda N nth (not (= (& N (<< 1 nth)) 0))))
(let std/int/bit/largest-power (lambda N (do
  ; changing all right side bits to 1.
  (let N1 (| N (>> N 1)))
  (let N2 (| N1 (>> N1 2)))
  (let N3 (| N2 (>> N2 4)))
  (let N4 (| N3 (>> N3 8)))
  ; as now the number is 2 * x - 1,
  ; where x is required answer,
  ; so adding 1 and dividing it by
  (>> (+ N4 1) 1))))
(let std/int/abs (lambda n (- (^ n (>> n 31)) (>> n 31))))
(let std/int/positive? (lambda x (> x 0)))
(let std/int/negative? (lambda x (< x 0)))
(let std/int/invert (lambda x (- x)))
(let std/int/zero? (lambda x (= x 0)))
(let std/int/one? (lambda x (= x 1)))
(let std/int/negative-one? (lambda x (= x -1)))
(let std/int/divisible? (lambda a b (= (mod a b) 0)))
(let std/int/floor/div (lambda a b (/ a b)))
(let std/int/ceil/div (lambda a b (/ (+ a b -1) b)))

(let std/int/square (lambda x (* x x)))
(let std/int/even? (lambda x (= (mod x 2) 0)))
(let std/int/odd? (lambda x (not (= (mod x 2) 0))))
(let std/vector/int/sum (lambda xs (std/vector/reduce xs (lambda a b (+ a b)) 0)))
(let std/vector/int/product (lambda xs (std/vector/reduce xs (lambda a b (* a b)) 1)))
(let std/int/euclidean-mod (lambda a b (mod (+ (mod a b) b) b)))
(let std/int/euclidean-distance (lambda x1 y1 x2 y2 (do
  (let a (- x1 x2))
  (let b (- y1 y2))
  (std/int/sqrt (+ (* a a) (* b b))))))
(let std/int/manhattan-distance (lambda x1 y1 x2 y2 (+ (std/int/abs (- x2 x1)) (std/int/abs (- y2 y1)))))
(let std/int/chebyshev-distance (lambda x1 y1 x2 y2 (std/int/max (std/int/abs (- x2 x1)) (std/int/abs (- y2 y1)))))
(let std/int/max (lambda a b (if (> a b) a b)))
(let std/int/min (lambda a b (if (< a b) a b)))
(let std/vector/int/maximum (lambda xs (cond (std/vector/empty? xs) Int (= (length xs) 1) (get xs 0) (std/vector/reduce xs std/int/max (get xs 0)))))
(let std/vector/int/minimum (lambda xs (cond (std/vector/empty? xs) Int (= (length xs) 1) (get xs 0) (std/vector/reduce xs std/int/min (get xs 0)))))
(let std/int/average (lambda x y (/ (+ x y) 2)))
(let std/vector/int/mean (lambda xs (/ (std/vector/int/sum xs) (length xs))))
(let std/vector/int/median (lambda xs (do
    (let len (length xs))
    (let half (/ len 2))
    (if (std/int/odd? len)
        (get xs half)
        (/ (+ (get xs (- half 1)) (get xs half)) 2)))))
(let std/int/normalize (lambda value min max (* (- value min) (/ (- max min)))))
(let std/int/linear-interpolation (lambda a b n (+ (* (- 1 n) a) (* n b))))
(let std/int/gauss-sum (lambda n (/ (* n (+ n 1)) 2)))
(let std/int/gauss-sum-sequance (lambda a b (/ (* (+ a b) (+ (- b a) 1)) 2)))
(let std/int/clamp (lambda x limit (if (> x limit) limit x)))
(let std/int/clamp-range (lambda x start end (cond (> x end) end (< x start) start x)))
(let std/int/between? (lambda v min max (and (> v min) (< v max))))
(let std/int/overlap? (lambda v min max (and (>= v min) (<= v max))))
(let std/int/sqrt (lambda n
  (do
    (integer x n)
    (integer prev 0)
    (loop (> (std/int/abs (- (get x) (get prev))) 0)
      (lambda (do
        (set prev (get x))
        (set x (/ (+ (get x) (/ n (get x))) 2)))))
    (get x))))
(let std/int/expt (lambda base exp (do
  (if (< exp 0) 0 (do
      (integer result 1)
      (integer b base)
      (integer e exp)
      (loop (> (get e) 0)
        (lambda (do
          (if (= (mod (get e) 2) 1)
            (set result (* (get result) (get b))))
          (set b (* (get b) (get b)))
          (set e (/ (get e) 2)))))
      (get result))))))

(let std/vector/zipper (lambda a b (do 
      (let out [[(get a 0) (get b 0)]])
      (let process (lambda i (std/vector/set! out (length out) [(get a i) (get b i)])))
      (loop 1 (length a) process)
      out)))

(let std/vector/zip (lambda xs (std/vector/zipper (std/vector/first xs) (std/vector/second xs))))
(let std/vector/unzip (lambda xs [ (std/vector/map xs std/vector/first) (std/vector/map xs std/vector/second) ]))

(let std/vector/slice (lambda xs start end (if (std/vector/empty? xs) xs (do
     (let bounds (- end start))
     (let out [])
     (let process (lambda i (std/vector/set! out (length out) (get xs (+ start i)))))
     (loop 0 bounds process)
     out))))

(let std/vector/drop (lambda xs start (if (std/vector/empty? xs) xs (do
     (let end (length xs))
     (let bounds (- end start))
     (let out [])
     (let process (lambda i (std/vector/set! out (length out) (get xs (+ start i)))))
     (loop 0 bounds process)
     out))))

(let std/vector/drop/last (lambda xs end (if (std/vector/empty? xs) xs (do
     (let bounds (- (length xs) end))
     (let out [])
     (let process (lambda i (std/vector/set! out (length out) (get xs i))))
     (loop 0 bounds process)
     out))))

(let std/vector/take (lambda xs end (if (std/vector/empty? xs) xs (do
     (let out [])
     (let process (lambda i (std/vector/set! out (length out) (get xs i))))
     (loop 0 end process)
     out))))

(let std/vector/take/last (lambda xs start (if (std/vector/empty? xs) xs (do
     (let out [])
     (let process (lambda i (std/vector/set! out (length out) (get xs i))))
     (loop (- (length xs) start) (length xs) process)
     out))))

(let std/vector/reverse (lambda xs (if (std/vector/empty? xs) xs (do
     (let out [])
     (let len (length xs))
     (let process (lambda i (std/vector/set! out (length out) (get xs (- len i 1)))))
     (loop 0 len process)
     out))))

(let std/vector/find-index (lambda xs fn? (do
     (let i [ 0 ])
     (let index [ -1 ])
     (let len (length xs))
     (let process (lambda
           (if (fn? (get xs (get i)))
              (std/vector/set! index 0 (get i))
              (std/vector/set! i 0 (+ (get i) 1)))))
     (loop (and (< (get i) len) (= (get index 0) -1)) process)
     (get index 0))))

(let std/vector/buckets (lambda size (do
     (let out [[]])
     (loop 1 size (lambda . (std/vector/set! out (length out) [])))
     out)))

(let std/vector/string/match? (lambda a b (and (= (length a) (length b)) (|>
  a
  (std/vector/zipper b)
  (std/vector/every? (lambda x (= (get x 0) (get x 1))))))))

(let std/vector/partition (lambda xs n (if (= n (length xs)) [xs] (do 
    (let a [])
    (loop 0 (length xs) (lambda i (if (= (mod i n) 0)
        (std/vector/set! a (length a) [(get xs i)])
        (std/vector/set! (std/vector/at a -1) (length (std/vector/at a -1)) (get xs i)))))
     a))))

(let std/vector/sort-partition! (lambda arr start end fn (do
     (let pivot (get arr end))
     (let i [(- start 1)])
     (let j [ start ])

     (let helper (lambda i j (do
          (std/vector/set! i 0 (+ (get i) 1))
          (std/vector/swap! arr (get i) (get j))
          nil)))

     (let process (lambda (do
           (if (fn (get arr (get j)) pivot) (helper i j))
           (std/vector/set! j 0 (+ (get j) 1)))))
     (loop (< (get j) end) process)

     (std/vector/swap! arr (+ (get i) 1) end)
     (+ (get i) 1))))

(let std/vector/sort! (lambda arr fn (do
     (let stack [])
     (std/vector/push! stack 0)
     (std/vector/push! stack (- (length arr) 1))
     (let process (lambda (do
           (let end (get stack (- (length stack) 1)))
           (std/vector/pop! stack)
           (let start (get stack (- (length stack) 1)))
           (std/vector/pop! stack)
           (let helper (lambda (do
                 (let pivot-index (std/vector/sort-partition! arr start end fn))
                 (std/vector/push! stack start)
                 (std/vector/push! stack (- pivot-index 1))
                 (std/vector/push! stack (+ pivot-index 1))
                 (std/vector/push! stack end)
                 nil)))
           (if (< start end) (helper)))))
     (loop (> (length stack) 0) process)
     arr)))

(let std/int/hash
 (lambda table key (do
     (let prime-num 31)
     (let total [ 0 ])
     (let i [ 0 ])
     (let bounds (if (< (- (length key) 1) 100) (- (length key) 1) 100))

     (let process (lambda (do
           (let letter (get key (get i)))
           (std/vector/set! total 0 (std/int/euclidean-mod (+ (* (get total 0 ) prime-num) letter) (length table)))
           (std/vector/set! i 0 (+ (get i) 1)))))

     (loop (< (get i) bounds) process)
     (get total 0))))

(let std/vector/hash/set/has? (lambda table key (do
     (let idx (std/int/hash table key))
     (let current (get table idx))
     (and (std/vector/in-bounds? table idx)
                  (and (> (length current) 0)
                       (>= (std/vector/find-index current (lambda x (std/vector/string/match? x key))) 0))))))

(let std/vector/hash/table/has? (lambda table key (do
         (let idx (std/int/hash table key))
         (let current (std/vector/map (get table idx) (lambda x (get x 0))))
         (and (std/vector/in-bounds? table idx)
         (and (> (length current) 0)
           (>= (std/vector/find-index current
             (lambda x
               (std/vector/string/match? x key))) 0))))))

(let std/vector/hash/set/add!
     (lambda table key
       (do
         (let idx (std/int/hash table key))
         (if (not (std/vector/in-bounds? table idx)) (std/vector/set! table idx []) nil)
         (let current (get table idx))
         (let len (length current))
         (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/string/match? x key))) -1))
         (let entry key)
         (if (= index -1)
           (std/vector/set! current (length current) entry)
           (std/vector/set! current index entry)) table)))

(let std/vector/hash/set/remove!
 (lambda table key
   (do
     (let idx (std/int/hash table key))
     (if (not (std/vector/in-bounds? table idx)) (std/vector/set! table idx []) nil)
     (let current (get table idx))
     (let len (length current))
     (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/string/match? x key))) -1))
     (let entry key)
     (if (not (= index -1)) (do (std/vector/set! current index (std/vector/at current -1)) (std/vector/pop! current)) nil)
     table)))

(let std/vector/hash/table/set! (lambda table key value
       (do
         (let idx (std/int/hash table key))
         (if (not (std/vector/in-bounds? table idx)) (std/vector/set! table idx []) nil)
         (let current (get table idx))
         (let len (length current))
         (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/string/match? (get x 0) key))) -1))
         (let entry [ key [value] ])
         (if (= index -1)
           (std/vector/set! current (length current) entry)
           (std/vector/set! current index entry))
         table)))
        
(let std/vector/hash/table/delete! (lambda table key
     (do
       (let idx (std/int/hash table key))
       (if (not (std/vector/in-bounds? table idx)) (std/vector/set! table idx []) nil)
       (let current (get table idx))
       (let len (length current))
       (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/string/match? (get x 0) key))) -1))
       (if (not (= index -1)) (do (std/vector/set! current index (std/vector/at current -1)) (std/vector/pop! current)) nil)
       table)))

(let std/vector/hash/clear! (lambda table (do 
     (loop 0 (length table) (lambda i (std/vector/empty! (get table i))))
     table)))

(let std/vector/hash/table/get-helper (lambda table idx key (do
   (let current (get table idx))
   (let found-index (std/vector/find-index current (lambda x (std/vector/string/match? key (get x 0)))))
   (unless (= found-index -1) (get current found-index 1) []))))

(let std/vector/hash/table/get (lambda table key (do
     (let idx (std/int/hash table key))
     (if (std/vector/in-bounds? table idx) (get (std/vector/hash/table/get-helper table idx key)) -1))))

(let std/vector/hash/table/count (lambda arr 
    (|> arr (std/vector/reduce (lambda table key (do 
        (if (std/vector/hash/table/has? table key) 
            (std/vector/hash/table/set! table key (+ (std/vector/hash/table/get table key) 1))
            (std/vector/hash/table/set! table key 1)))) (std/vector/buckets 64)))))

(let std/vector/sliding-window (lambda xs size (cond 
     (std/vector/empty? xs) []
     (= size (length xs)) [xs]
     (std/vector/reduce/i xs (lambda a b i (if (> (+ i size) (length xs)) a (std/vector/cons a [(std/vector/slice xs i (+ i size))]))) []))))

(let std/vector/flat-one (lambda xs (cond 
     (std/vector/empty? xs) []
     (= (length xs) 1) (get xs)
     (std/vector/reduce xs (lambda a b (std/vector/cons a b)) []))))

(let std/vector/hash/table/keys (lambda table (|> table (std/vector/flat-one) (std/vector/map std/vector/first))))
(let std/vector/hash/table/values (lambda table (|> table (std/vector/flat-one) (std/vector/map std/vector/second))))
(let std/vector/hash/table/entries (lambda table (|> table (std/vector/flat-one))))

(let std/convert/char->digit (lambda ch (- ch std/int/char/0)))
(let std/convert/chars->digits (lambda digits (std/vector/map digits std/convert/char->digit)))
(let std/convert/digit->char (lambda digit (+ digit std/int/char/0)))
(let std/convert/digits->chars (lambda digits (std/vector/map digits std/convert/digit->char)))
(let std/convert/bool->int (lambda x (if (eq x true) 1 0)))
(let std/convert/int->bool (lambda x (if (= x 0) false true)))
(let std/convert/vector->string (lambda xs delim (std/vector/reduce/i xs (lambda a b i (if (> i 0) (std/vector/cons (std/vector/append! a delim) b) b)) [])))
(let std/convert/string->vector (lambda str char (|> str
              (std/vector/reduce(lambda a b (do
              (let prev (std/vector/at a -1))
                (if (std/vector/string/match? [b] [char])
                    (std/vector/set! a (length a) [])
                    (std/vector/set! prev (length prev) b)) a))
              [[]])
              (std/vector/map (lambda x (std/convert/vector->string [ x ] std/int/char/empty))))))

(let std/convert/positive-or-negative-digits->integer (lambda digits-with-sign (do
    (let std/int/negative? (< (std/vector/first digits-with-sign) 0))
    (let digits (if std/int/negative? (std/vector/map digits-with-sign std/int/abs) digits-with-sign))
    (integer num 0)
    (integer base (/ (std/int/expt 10 (length digits)) 10))
    (loop 0 (length digits) (lambda i (do 
      (+= num (* (get base) (. digits i)))
      (/= base 10)
    )))
    (*= num (if std/int/negative? -1 1))
    (get num))))

(let std/convert/chars->positive-or-negative-digits (lambda chars (do
    (integer current-sign 1)
    (|> chars 
        (std/vector/reduce (lambda a ch (do 
            (if (= ch std/int/char/minus) 
                (set current-sign -1) 
                (do  
                    (std/vector/push! a (* (get current-sign) (std/convert/char->digit ch))) 
                    (set current-sign 1)))
                a)) [])))))
(let std/convert/digits->integer std/convert/positive-or-negative-digits->integer)
(let std/convert/positive-or-negative-chars->integer (lambda x (|> x (std/convert/chars->positive-or-negative-digits) (std/convert/positive-or-negative-digits->integer))))
(let std/convert/chars->integer std/convert/positive-or-negative-chars->integer)

; (let buffer [])
; (let fn (ring-buffer buffer 5))
; (let buffer/get (get fn 0))
; (let buffer/std/vector/push! (get fn 1))
; (loop 0 6 (lambda i (buffer/std/vector/push! i)))
; buffer
(let ring-buffer (lambda buffer len (do 
    (integer pointer 0)
    [(lambda index (get buffer index)) 
    (lambda item (do 
        (let pt (get pointer))
        (std/vector/set! buffer pt item)
        (set pointer (mod (+ len pt 1) len))
        item))])))

(let std/vector/unique-pairs (lambda xs (do 
    (let pairs [])
    (let len (length xs))
    (integer i 0)
    (loop (< (get i) len) (lambda (do 
        (integer j (+ (get i) 1))
        (loop (< (get j) len) (lambda (do 
            (std/vector/push! pairs [(get xs (get i)) (get xs (get j))])
            (++ j))))
        (++ i))))
    pairs)))

(let std/vector/3d/dimensions (lambda matrix [ (length matrix) (length (get matrix 0)) ]))
(let std/vector/3d/in-bounds? (lambda matrix y x (and (std/vector/in-bounds? matrix y) (std/vector/in-bounds? (get matrix y) x))))
(let std/vector/3d/set! (lambda matrix y x value (do (set! (get matrix y) x value) 0)))
(let std/vector/3d/diagonal-neighborhood [ [ 1 -1 ] [ -1 -1 ] [ 1 1 ] [ -1 1 ] ])
(let std/vector/3d/kernel-neighborhood [ [ 0 0 ] [ 0 1 ] [ 1 0 ] [ -1 0 ] [ 0 -1 ] [ 1 -1 ] [ -1 -1 ] [ 1 1 ] [ -1 1 ]])
(let std/vector/3d/moore-neighborhood [ [ 0 1 ] [ 1 0 ] [ -1 0 ] [ 0 -1 ] [ 1 -1 ] [ -1 -1 ] [ 1 1 ] [ -1 1 ] ])
(let std/vector/3d/von-neumann-neighborhood [ [ 1 0 ] [ 0 -1 ] [ 0 1 ] [ -1 0 ] ])

(let std/vector/3d/adjacent (lambda xs directions y x fn
      (std/vector/for directions (lambda dir (do
          (let dy (+ (std/vector/first dir) y))
          (let dx (+ (std/vector/second dir) x))
          (if (std/vector/3d/in-bounds? xs dy dx)
              (fn (get xs dy dx) dir dy dx)))))))

(let std/vector/3d/sliding-adjacent-sum (lambda xs directions y x N fn
      (std/vector/reduce directions (lambda a dir (do
          (let dy (+ (std/vector/first dir) y))
          (let dx (+ (std/vector/second dir) x))
          (fn a (get xs (std/int/euclidean-mod dy N) (std/int/euclidean-mod dx N))))) 0)))

(let std/node/parent (lambda i (- (>> (+ i 1) 1) 1)))
(let std/node/left (lambda i (+ (<< i 1) 1)))
(let std/node/right (lambda i (<< (+ i 1) 1)))

(let std/heap/top 0)
(let std/heap/greater? (lambda heap i j fn? (fn? (get heap i) (get heap j))))
(let std/heap/sift-up! (lambda heap fn (do 
  (integer node (- (length heap) 1))
  (let tail-call/std/heap/sift-up! (lambda heap
    (if (and (> (get node) std/heap/top) (std/heap/greater? heap (get node) (std/node/parent (get node)) fn))
      (do 
        (std/vector/swap! heap (get node) (std/node/parent (get node)))
        (set node (std/node/parent (get node)))
        (tail-call/std/heap/sift-up! heap)) heap)))
  (tail-call/std/heap/sift-up! heap))))

(let std/heap/sift-down! (lambda heap fn (do
  (integer node std/heap/top)
  (let tail-call/std/heap/sift-down! (lambda heap
    (if (or 
          (and 
            (< (std/node/left (get node)) (length heap))
            (std/heap/greater? heap (std/node/left (get node)) (get node) fn))
          (and 
            (< (std/node/right (get node)) (length heap))
            (std/heap/greater? heap (std/node/right (get node)) (get node) fn)))
      (do 
        (let max-child (if (and 
                            (< (std/node/right (get node)) (length heap))
                            (std/heap/greater? heap (std/node/right (get node)) (std/node/left (get node)) fn))
                            (std/node/right (get node))
                            (std/node/left (get node))))
        (std/vector/swap!  heap (get node) max-child)
        (set node max-child)
        (tail-call/std/heap/sift-down! heap)) heap)))
  (tail-call/std/heap/sift-down! heap))))

(let std/heap/peek (lambda heap (get heap std/heap/top)))

(let std/heap/push! (lambda heap value fn (do 
    (set! heap (length heap) value)
    (std/heap/sift-up! heap fn)
    nil)))

(let std/heap/pop! (lambda heap fn (do 
  (let bottom (- (length heap) 1))
  (if (> bottom std/heap/top) (std/vector/swap! heap std/heap/top bottom) heap)
  (pop! heap)
  (std/heap/sift-down! heap fn)
  nil)))

(let std/heap/replace! (lambda heap value fn (do 
(set! heap std/heap/top value)
(std/heap/sift-down! heap fn)
heap)))


(let std/heap/empty? std/vector/empty?)
(let std/heap/not-empty? std/vector/not-empty?)
(let std/heap/empty! std/vector/empty!)

(let std/convert/vector->heap (lambda xs fn (std/vector/reduce xs (lambda heap x (do (std/heap/push! heap x fn) heap)) [])))
(let std/convert/set->vector (lambda xs (std/vector/filter (std/vector/flat-one xs) std/vector/not-empty?)))

(let std/vector/hash/set/max-capacity (lambda a b (std/vector/buckets (std/int/max (length a) (length b)))))
(let std/vector/hash/set/min-capacity (lambda a b (std/vector/buckets (std/int/min (length a) (length b)))))
(let std/convert/integer->string-base (lambda num base  
    (if (= num 0) [ std/int/char/0 ] (do 
        (let neg? (< num 0))
        (integer n (if neg? (* num -1) num))
        (let tail-call/while (lambda out
            (if (> (get n) 0) (do
                (std/vector/push! out (mod (get n) base))
                (set n (/ (get n) base))
                (tail-call/while out)) out)))
        (let str (std/convert/digits->chars (tail-call/while [])))
        (std/vector/reverse (if neg? (std/vector/append! str std/int/char/dash) str))))))
(let std/convert/integer->string (lambda x (std/convert/integer->string-base x 10)))
(let std/convert/vector->set (lambda xs (std/vector/reduce xs (lambda s x (do (std/vector/hash/set/add! s x) s)) [ [] [] [] [] ])))
(let std/vector/hash/set/intersection (lambda a b
        (|> b
          (std/convert/set->vector)
          (std/vector/reduce (lambda out element
          (do (if (std/vector/hash/set/has? a element)
                    (std/vector/hash/set/add! out element) out) out)) (std/vector/hash/set/max-capacity a b)))))
(let std/vector/hash/set/difference (lambda a b
      (|> a
        (std/convert/set->vector)
        (std/vector/reduce (lambda out element
                        (do (if (not (std/vector/hash/set/has? b element))
                                        (std/vector/hash/set/add! out element) out) out)) (std/vector/hash/set/max-capacity a b)))))
(let std/vector/hash/set/xor (lambda a b (do
        (let out (std/vector/hash/set/max-capacity a b))
        (|> a (std/convert/set->vector) (std/vector/for (lambda element (if (not (std/vector/hash/set/has? b element)) (std/vector/hash/set/add! out element) out))))
        (|> b (std/convert/set->vector) (std/vector/for (lambda element (if (not (std/vector/hash/set/has? a element)) (std/vector/hash/set/add! out element) out))))
        out)))
(let std/vector/hash/set/union (lambda a b (do
        (let out (std/vector/hash/set/max-capacity a b))
        (|> a (std/convert/set->vector) (std/vector/for (lambda element (std/vector/hash/set/add! out element))))
        (|> b (std/convert/set->vector) (std/vector/for (lambda element (std/vector/hash/set/add! out element))))
        out)))

; Experimental still

(let std/vector/deque/new (lambda def [[ def ] []]))
(let std/vector/deque/offset-left (lambda q (* (- (length (get q 0)) 1) -1)))
(let std/vector/deque/offset-right (lambda q (length (get q 1))))
(let std/vector/deque/length (lambda q (+ (length (get q 0)) (length (get q 1)) -1)))
(let std/vector/deque/empty? (lambda q (= (std/vector/deque/length q) 0)))
(let std/vector/deque/empty! (lambda q def (do
    (set! q 0 [def])
    (set! q 1 [])
    q)))

(let std/vector/deque/get (lambda q offset (do
  (let offset-index (+ offset (std/vector/deque/offset-left q)))
  (let index (if (< offset-index 0) (* offset-index -1) offset-index))
  (if (>= offset-index 0)
       (get (get q 1) index)
       (get (get q 0) index)))))

(let std/vector/deque/set! (lambda q index value (do
    (let offset (+ index (std/vector/deque/offset-left q)))
    (if (>= offset 0)
        (set! (get q 1) offset value)
        (set! (get q 0) (* offset -1) value))
  q)))
(let std/vector/deque/add-to-left! (lambda q item (do (let c (get q 0)) (set! c (length c) item))))
(let std/vector/deque/add-to-right! (lambda q item (do (let c (get q 1)) (set! c (length c) item))))
(let std/vector/deque/remove-from-left! (lambda q def (do
  (let len (std/vector/deque/length q))
  (if (> len 0)
     (cond
        (= len 1) (std/vector/deque/empty! q def)
        (> (length (get q 0)) 0) (do (pop! (get q 0)) q)
        q) q))))
(let std/vector/deque/remove-from-right! (lambda q def (do
    (let len (std/vector/deque/length q))
    (if (> len 0)
     (cond
        (= len 1) (std/vector/deque/empty! q def)
        (> (length (get q 1)) 0) (do (pop! (get q 1)) q)
        q) q))))
(let std/vector/deque/iter (lambda q fn (do
  (let tail-call/std/vector/deque/iter (lambda index bounds (do
      (fn (std/vector/deque/get q index))
      (if (< index bounds) (tail-call/std/vector/deque/iter (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/iter 0 (std/vector/deque/length q)))))
(let std/vector/deque/map (lambda q fn def (do
  (let result (std/vector/deque/new def))
  (let len (std/vector/deque/length q))
  (let half (/ len 2))
  (let tail-call/left/std/vector/deque/map (lambda index (do
    (std/vector/deque/add-to-left! result (fn (std/vector/deque/get q index)))
   (if (> index 0) (tail-call/left/std/vector/deque/map (- index 1)) Int))))
 (tail-call/left/std/vector/deque/map (- half 1))
(let tail-call/right/std/vector/deque/map (lambda index bounds (do
   (std/vector/deque/add-to-right! result (fn (std/vector/deque/get q index)))
   (if (< index bounds) (tail-call/right/std/vector/deque/map (+ index 1) bounds) Int))))
 (tail-call/right/std/vector/deque/map half (- len 1))
 result)))
(let std/vector/deque/balance? (lambda q (= (+ (std/vector/deque/offset-right q) (std/vector/deque/offset-left q)) 0)))
(let std/convert/vector->deque (lambda initial def (do
 (let q (std/vector/deque/new def))
 (let half  (/ (length initial) 2))
 (let tail-call/left/from/vector->deque (lambda index (do
    (std/vector/deque/add-to-left! q (get initial index))
   (if (> index 0) (tail-call/left/from/vector->deque (- index 1)) Int))))
 (tail-call/left/from/vector->deque (- half 1))
(let tail-call/right/from/vector->deque (lambda index bounds (do
   (std/vector/deque/add-to-right! q (get initial index))
   (if (< index bounds) (tail-call/right/from/vector->deque (+ index 1) bounds) Int))))
 (tail-call/right/from/vector->deque half (- (length initial) 1))
    q)))
(let std/convert/deque->vector (lambda q (if (std/vector/deque/empty? q) [(. q 0 0)] (do
  (let out [])
  (let tail-call/from/deque->vector (lambda index bounds (do
      (set! out (length out) (std/vector/deque/get q index))
      (if (< index bounds) (tail-call/from/deque->vector (+ index 1) bounds) Int))))
    (tail-call/from/deque->vector 0 (- (std/vector/deque/length q) 1))
    out))))
(let std/vector/deque/balance! (lambda q def
    (if (std/vector/deque/balance? q) q (do
      (let initial (std/convert/deque->vector q))
      (std/vector/deque/empty! q def)
      (let half (/ (length initial) 2))
      (let tail-call/left/std/vector/deque/balance! (lambda index (do
        (std/vector/deque/add-to-left! q (get initial index))
        (if (> index 0) (tail-call/left/std/vector/deque/balance! (- index 1)) Int))))
      (let tail-call/right/std/vector/deque/balance! (lambda index bounds (do
        (std/vector/deque/add-to-right! q (get initial index))
        (if (< index bounds) (tail-call/right/std/vector/deque/balance! (+ index 1) bounds) Int))))
      (tail-call/right/std/vector/deque/balance! half (- (length initial) 1))
      (if (> (length initial) 1) (tail-call/left/std/vector/deque/balance! (- half 1)) Int)
    q))))
(let std/vector/deque/append! (lambda q item (do (std/vector/deque/add-to-right! q item) q)))
(let std/vector/deque/prepend! (lambda q item (do (std/vector/deque/add-to-left! q item) q)))
(let std/vector/deque/head! (lambda q def (do
    (if (= (std/vector/deque/offset-right q) 0) (std/vector/deque/balance! q def) q)
    (std/vector/deque/remove-from-right! q def)
    q)))
(let std/vector/deque/tail! (lambda q def (do
    (if (= (std/vector/deque/offset-left q) 0) (std/vector/deque/balance! q def) q)
    (std/vector/deque/remove-from-left! q def)
q)))
(let std/vector/deque/first (lambda q (std/vector/deque/get q 0)))
(let std/vector/deque/last (lambda q (std/vector/deque/get q (- (std/vector/deque/length q) 1))))
(let std/vector/deque/pop-right! (lambda q def (do
    (let last (std/vector/deque/last q))
    (std/vector/deque/head! q def)
    last)))
(let std/vector/deque/pop-left! (lambda q def (do
    (let first (std/vector/deque/first q))
    (std/vector/deque/tail! q def)
    first)))
(let std/vector/deque/rotate-left! (lambda q n def (do
  (let N (mod n (std/vector/deque/length q)))
  (let tail-call/std/vector/deque/rotate-left! (lambda index bounds (do
      (if (= (std/vector/deque/offset-left q) 0) (std/vector/deque/balance! q def) q)
      (std/vector/deque/add-to-right! q (std/vector/deque/first q))
      (std/vector/deque/remove-from-left! q def)
      (if (< index bounds) (tail-call/std/vector/deque/rotate-left! (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/rotate-left! 0 N) q)))
(let std/vector/deque/rotate-right! (lambda q n def (do
  (let N (mod n (std/vector/deque/length q)))
  (let tail-call/std/vector/deque/rotate-left! (lambda index bounds (do
      (if (= (std/vector/deque/offset-right q) 0) (std/vector/deque/balance! q def) q)
      (std/vector/deque/add-to-left! q (std/vector/deque/last q))
      (std/vector/deque/remove-from-right! q def)
      (if (< index bounds) (tail-call/std/vector/deque/rotate-left! (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/rotate-left! 0 N) q)))
(let std/vector/deque/slice (lambda entity s e def (do
  (let len (std/vector/deque/length entity))
  (let start (if (< s 0) (std/int/max (+ len s) 0) (std/int/min s len)))
  (let end (if (< e 0) (std/int/max (+ len e) 0) (std/int/min e len)))
  (let slice (std/vector/deque/new def))
  (let slice-len (std/int/max (- end start) 0))
  (let half (/ slice-len 2))
  (let tail-call/left/std/vector/deque/slice (lambda index (do
      (std/vector/deque/add-to-left! slice (std/vector/deque/get entity (+ start index)))
      (if (> index 0) (tail-call/left/std/vector/deque/slice (- index 1)) Int))))
  (tail-call/left/std/vector/deque/slice (- half 1))
  (let tail-call/right/std/vector/deque/slice (lambda index bounds (do
      (std/vector/deque/add-to-right! slice (std/vector/deque/get entity (+ start index)))
      (if (< index bounds) (tail-call/right/std/vector/deque/slice (+ index 1) bounds) Int))))
  (tail-call/right/std/vector/deque/slice half (- slice-len 1))
  slice)))

(let std/vector/queue/new std/vector/deque/new)
(let std/vector/stack/new std/vector/deque/new)

(let std/vector/queue/empty? std/vector/deque/empty?)
(let std/vector/queue/not-empty? (lambda q (not (std/vector/deque/empty? q))))
(let std/vector/queue/empty! std/vector/deque/empty!)
(let std/vector/queue/enqueue! (lambda queue item (std/vector/deque/append! queue item)))
(let std/vector/queue/dequeue! (lambda queue def (std/vector/deque/tail! queue def)))
(let std/vector/queue/peek (lambda queue (std/vector/deque/first queue)))

(let std/vector/stack/empty? std/vector/deque/empty?)
(let std/vector/stack/not-empty? (lambda q (not (std/vector/deque/empty? q))))
(let std/vector/stack/empty! std/vector/deque/empty!)
(let std/vector/stack/push! (lambda stack item (std/vector/deque/append! stack item)))
(let std/vector/stack/pop! (lambda stack (std/vector/deque/head! stack)))
(let std/vector/stack/peek (lambda stack (std/vector/deque/last stack)))


(let std/vector/3d/for (lambda matrix fn (do
  (let width (length (std/vector/first matrix)))
  (let height (length matrix))
  (loop 0 height (lambda y 
    (loop 0 width (lambda x
      (fn (get matrix y x))))))
   matrix)))

(let std/vector/3d/for/i (lambda matrix fn (do
  (let width (length (std/vector/first matrix)))
  (let height (length matrix))
  (loop 0 height (lambda y 
    (loop 0 width (lambda x
      (fn (get matrix y x) y x)))))
   matrix)))

(let std/vector/3d/points (lambda matrix fn? (do 
   (let coords [])
   (std/vector/3d/for/i matrix (lambda cell y x (if (fn? cell) (do (std/vector/push! coords [ y x ]) nil)))) 
    coords)))

(let std/vector/concat-with (lambda xs ch (std/vector/reduce/i xs (lambda a b i (if (and (> i 0) (< i (length xs))) (std/vector/cons (std/vector/cons a [ ch ]) b) (std/vector/cons a b))) [])))

(let std/vector/string/lines (lambda xs (std/convert/string->vector xs std/int/char/new-line)))
(let std/vector/string/words (lambda xs (std/convert/string->vector xs std/int/char/space)))
(let std/vector/string/commas (lambda xs (std/convert/string->vector xs std/int/char/comma)))
(let std/vector/int/pair/sub (lambda xs (- (. xs 0) (. xs 1))))
(let std/vector/int/pair/add (lambda xs (+ (. xs 0) (. xs 1))))
(let std/vector/int/pair/mult (lambda xs (* (. xs 0) (. xs 1))))
(let std/vector/int/pair/div (lambda xs (/ (. xs 0) (. xs 1))))
(let std/vector/sort/asc! (lambda xs (std/vector/sort! xs <)))
(let std/vector/sort/desc! (lambda xs (std/vector/sort! xs >)))

(let std/convert/integer->bits (lambda num  
    (if (= num 0) [ 0 ] (do 
        (integer n num)
        (let tail-call/while (lambda out
            (if (> (get n) 0) (do
                (std/vector/push! out (mod (get n) 2))
                (set n (/ (get n) 2))
                (tail-call/while out)) out)))
        (std/vector/reverse (tail-call/while []))))))

(let std/vector/subset (lambda xs (if (std/vector/empty? xs) [ xs ] (do
    (let n (length xs))
    (let out [])
    (loop 0 (std/int/expt 2 n) (lambda i 
       ; generate bitmask, from 0..00 to 1..11
        (std/vector/push! out (|>
                i
                (std/convert/integer->bits)
                (std/vector/reduce/i (lambda a x i
                    (if (= x 1) (std/vector/append! a (get xs i)) a)) [])))))
    out))))

; alternative implementation using bitwise operators
; (let std/convert/bits->integer (lambda bits (std/vector/reduce bits (lambda value bit (| (<< value 1) (& bit 1))) 0)))

(let std/convert/bits->integer (lambda xs (do
  (let bits->integer (lambda index out (if
                              (= index (length xs)) out
                              (bits->integer (+ index 1) (+ out (* (std/vector/at xs index) (std/int/expt 2 (- (length xs) index 1))))))))
  (bits->integer 0 0))))

(let std/vector/copy (lambda xs (std/vector/map xs identity)))

(let std/int/reduce (lambda n fn acc (do 
    (let tail-call/fold-n (lambda i out (if (< i n) (tail-call/fold-n (+ i 1) (fn out i)) out)))
    (tail-call/fold-n 0 acc))))
(let std/vector/2d/fill (lambda n fn (do 
  (let tail-call/std/vector/fill (lambda i xs (if (= i 0) xs (tail-call/std/vector/fill (- i 1) (std/vector/cons! xs (vector (fn i)))))))
  (tail-call/std/vector/fill n []))))
(let std/vector/3d/fill (lambda W H fn 
  (cond 
    (or (= W 0) (= H 0)) [] 
    (and (= W 1) (= H 1)) [[(fn 0 0)]] (do
      (let matrix [])
      (loop 0 W (lambda i (do 
          (std/vector/push! matrix [])
          (loop 0 H (lambda j (std/vector/3d/set! matrix i j (fn i j)))))))
      matrix))))
(let std/vector/3d/product (lambda A B (do
  (let dimsA (std/vector/3d/dimensions A))
  (let dimsB (std/vector/3d/dimensions B))
  (let rowsA (. dimsA 0))
  (let colsA (. dimsA 1))
  (let rowsB (. dimsB 0))
  (let colsB (. dimsB 1))
  (if (= colsA rowsB) (std/vector/3d/fill rowsA colsB (lambda i j
      (std/int/reduce colsA (lambda sum k (+ sum (* (. A i k) (. B k j)))) 0))) []))))
(let std/vector/3d/dot-product (lambda a b (do
  (let lenA (length a))
  (let lenB (length b))
  (if (= lenA lenB)
    (std/int/reduce lenA (lambda sum i (+ sum (* (. a i) (. b i)))) 0) Int))))

(let std/vector/3d/rotate (lambda matrix (if (std/vector/empty? matrix) matrix (do 
    (let H (length matrix))
    (let W (length (. matrix 0)))
    (let out [])
    (loop 0 W (lambda i (do
        (std/vector/push! out [])
        (loop 0 H (lambda j 
            (std/vector/push! (std/vector/at out -1) (. matrix j i)))))))
    out))))

(let std/vector/int/sequence (lambda xs (std/vector/int/range 0 (- (length xs) 1))))
(let std/int/shoelace (lambda points (do
    (let len (length points))
    (/ (|> (std/vector/int/sequence points)
        (std/vector/reduce (lambda ab i (do
            (let a (. ab 0))
            (let b (. ab 1))
            (let left (. points i))
            (let right (. points (mod (+ i 1) len)))
            (let y1 (. left 0))
            (let x1 (. left 1))
            (let y2 (. right 0))
            (let x2 (. right 1))
            [(+ a (* y1 x2)) (+ b (* y2 x1))])) 
        [0 0])
        (std/vector/int/pair/sub)
        (std/int/abs)) 2))))
(let std/int/collinear? (lambda points (= (std/int/shoelace points) 0)))


(let std/int/big/add (lambda a1 b1 (do
  (let a (std/vector/reverse a1))
  (let b (std/vector/reverse b1))
  (let max-length (std/int/max (length a) (length b)))
  (let result (as [] [Int]))
  (integer carry 0)
  (loop 0 max-length (lambda i (do
    (let digit-A (if (< i (length a)) (get a i) 0))
    (let digit-B (if (< i (length b)) (get b i) 0))
    (let sum (+ digit-A digit-B (get carry)))
    (std/vector/push! result (mod sum 10))
    (set carry (/ sum 10)))
  ))
  ; Handle remaining carry
  (loop (> (get carry) 0) (lambda (do
    (std/vector/push! result (mod (get carry) 10))
    (set carry (/ (get carry) 10)))))
  (std/vector/reverse result))))
  
(let std/int/big/sub (lambda a1 b1 (do
  (let a (std/vector/reverse a1))
  (let b (std/vector/reverse b1))
  (let max-length (std/int/max (length a) (length b)))
  (let result (as [] [Int]))
  (integer borrow 0)
  (loop 0 max-length (lambda i (do
    (let digit-A (if (< i (length a)) (get a i) 0))
    (let digit-B (if (< i (length b)) (get b i) 0))
    (let sub (- digit-A digit-B (get borrow)))
    (if (< sub 0)
      (do
        (std/vector/push! result (+ sub 10))
        (set borrow 1))
      (do
        (std/vector/push! result sub)
        (set borrow 0))))))
  ; Remove trailing zeros (from the most significant end)
  (integer i (- (length result) 1))
  (loop (and (> (get i) 0) (= (get result (get i)) 0)) (lambda (do
    (std/vector/pop! result)
    (set i (- (get i) 1)))))
  (std/vector/reverse result))))

(let std/int/big/mul (lambda a1 b1 (do
  (let a (std/vector/reverse a1))
  (let b (std/vector/reverse b1))
  (let result (as [] [Int]))
  ; Initialize result array with zeros
  (loop 0 (+ (length a) (length b)) (lambda . (std/vector/push! result 0)))
  (loop 0 (length a) (lambda i (do
    (integer carry 0)
    (let digit-a (get a i))
    (loop 0 (length b) (lambda j (do
      (let digit-B (get b j))
      (let idx (+ i j))
      (let prod (+ (* digit-a digit-B) (get result idx) (get carry)))
      (set! result idx (mod prod 10))
      (set carry (/ prod 10)))))
    ; Handle carry for this digit-a
    (integer k (+ i (length b)))
    (loop (> (get carry) 0) (lambda (do
      (if (not (< (get k) (length result))) (do (std/vector/push! result 0) nil) nil)
      (let sum (+ (get result (get k)) (get carry)))
      (set! result (get k) (mod sum 10))
      (set carry (/ sum 10))
      (set k (+ (get k) 1))))))))
  ; Remove trailing zeros (from the most significant end), but keep at least one digit
  (integer i (- (length result) 1))
  (loop (and (> (get i) 0) (= (get result (get i)) 0) (> (length result) 1)) (lambda (do
    (std/vector/pop! result)
    (set i (- (get i) 1)))))
  (std/vector/reverse result))))

(let std/vector/int/remove-leading-zeroes (lambda digits (do
  (boolean tr true)
  (|> digits (std/vector/reduce (lambda a b (if
  (and (true? tr) (std/int/zero? b)) a
    (do
      (if (true? tr) (boole-set tr false))
      (std/vector/cons! a [b])))) [])))))

(let std/int/big/less-or-equal? (lambda a b (do
  (if (< (length a) (length b)) true
  (if (> (length a) (length b)) false
    ; Equal length, compare digit by digit
    (do
      (integer i 0)
      (boolean result true) ; assume a <= b
      (loop (< (get i) (length a)) (lambda (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (< da db) (do
          (boole-set result true)
          (set i (length a))))
        (if (> da db) (do
          (boole-set result false)
          (set i (length a))))
        (set i (+ (get i) 1)))))
      (if (true? result) true false)))))))

(let std/int/big/div (lambda dividend divisor (do
  (let result (as [] [Int]))
  (let current [[]])
  (let len (length dividend))
  (integer i 0)
  ; Main loop/ process each digit of the dividend
  (loop (< (get i) len) (lambda (do
    (let digit (get dividend (get i)))
    (set current (std/vector/int/remove-leading-zeroes (std/vector/cons (get current) [ digit ])))
    ; Find max digit q such that (divisor * q) <= current
    (integer low 0)
    (integer high 9)
    (integer q 0)
    (loop (<= (get low) (get high)) (lambda (do
      (let mid (/ (+ (get low) (get high)) 2))
      (let prod (std/int/big/mul divisor [ mid ]))
      (if (std/int/big/less-or-equal? prod (get current))
        (do
          (set q mid)
          (set low (+ mid 1)))
        (set high (- mid 1))))))

    (std/vector/push! result (get q))

    ; current /= current - (divisor * q)
    (let sub (std/int/big/mul divisor [ (get q) ]))
    (set current (std/int/big/sub (get current) sub))
    (++ i))))
  (let out (std/vector/int/remove-leading-zeroes result))
  (if (std/vector/empty? out) [ 0 ] out))))
(let std/int/big/square (lambda x (std/int/big/mul x x)))
(let std/int/big/floor/div (lambda a b (std/int/big/div a b)))
(let std/int/big/ceil/div (lambda a b (std/int/big/div 
    (std/int/big/sub (std/int/big/add a b) [ 1 ]) b)))
(let std/vector/int/big/sum (lambda xs (std/vector/reduce xs (lambda a b (std/int/big/add a b)) [ 0 ] )))
(let std/int/big/new (lambda str (std/convert/chars->digits str)))
(let std/int/pow/big (lambda n pow (do
  ; Initialize digits array with the first digit
  (let digits [ n ])
  (integer p 1) ; Use numeric variable for p
  (integer carry 0) ; Use numeric variable for carry
  ; Loop to calculate n^pow
  (loop (< (get p) pow) (lambda (do
    (set carry 0) ; Reset carry to 0
    (loop 0 (length digits) (lambda exp (do
      (let prod (+ (* (get digits exp) n) (get carry)))
      (let new-carry (/ prod 10))
      (set! digits exp (mod prod 10))
      ; Update carry using variable helper
      (set carry new-carry))))
    ; Handle carry
    (loop (> (get carry) 0) (lambda (do
      (std/vector/push! digits (mod (get carry) 10))
      ; Update carry using variable helper
      (/= carry 10))))
    ; Increment p using variable helper
    (++ p))))
  (std/vector/reverse digits))))

(let std/int/big/pow (lambda a b (if (= b 0) [ 1 ] (do 
    (variable out a)
    (loop 0 (- b 1) (lambda . (set out (std/int/big/mul (get out) a))))
    (get out)))))

(let std/int/big/expt (lambda a b (if (and (= (length b) 1) (= (get b 0) 0)) [ 1 ] (do 
    (variable out a)
    (variable expt (std/int/big/sub b [ 1 ]))
    (loop (not (and (= (length (get expt)) 1) (= (get (get expt) 0) 0))) (lambda (do 
      (set out (std/int/big/mul (get out) a))
      (set expt (std/int/big/sub (get expt) [ 1 ])))))
    (get out)))))

(let std/convert/integer->digits-base (lambda num base  
    (if (= num 0) [ 0 ] (do 
        (integer n num)
        (let tail-call/while (lambda out
            (if (> (get n) 0) (do
                (std/vector/push! out (mod (get n) base))
                (set n (/ (get n) base))
                (tail-call/while out)) out)))
        (let digits (tail-call/while []))
        (std/vector/reverse digits)))))

(let std/convert/integer->digits (lambda num (std/convert/integer->digits-base num 10)))
(let std/vector/adjacent-difference (lambda xs cb (do
  (let len (length xs))
  (if (= len 1) xs
    (do
      (vector (std/vector/first xs))
      (let tail-call/vector/adjacent-difference (lambda i result (if (< i len) (do
        (tail-call/vector/adjacent-difference (+ i 1) (std/vector/update! result i (cb (get xs (- i 1)) (get xs i))))) result)))
        (tail-call/vector/adjacent-difference 1 xs))))))

(let std/convert/vector/3d->string (lambda xs a b (std/convert/vector->string (std/vector/map xs (lambda x (std/convert/vector->string x b))) a)))
