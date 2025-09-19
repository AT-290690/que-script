(let std:int:char:A 65)
(let std:int:char:B 66)
(let std:int:char:C 67)
(let std:int:char:D 68)
(let std:int:char:E 69)
(let std:int:char:F 70)
(let std:int:char:G 71)
(let std:int:char:H 72)
(let std:int:char:I 73)
(let std:int:char:J 74)
(let std:int:char:K 75)
(let std:int:char:L 76)
(let std:int:char:M 77)
(let std:int:char:N 78)
(let std:int:char:O 79)
(let std:int:char:P 80)
(let std:int:char:Q 81)
(let std:int:char:R 82)
(let std:int:char:S 83)
(let std:int:char:T 84)
(let std:int:char:U 85)
(let std:int:char:V 86)
(let std:int:char:W 87)
(let std:int:char:X 88)
(let std:int:char:Y 89)
(let std:int:char:Z 90)
(let std:int:char:a 97)
(let std:int:char:b 98)
(let std:int:char:c 99)
(let std:int:char:d 100)
(let std:int:char:e 101)
(let std:int:char:f 102)
(let std:int:char:g 103)
(let std:int:char:h 104)
(let std:int:char:i 105)
(let std:int:char:j 106)
(let std:int:char:k 107)
(let std:int:char:l 108)
(let std:int:char:m 109)
(let std:int:char:n 110)
(let std:int:char:o 111)
(let std:int:char:p 112)
(let std:int:char:q 113)
(let std:int:char:r 114)
(let std:int:char:s 115)
(let std:int:char:t 116)
(let std:int:char:u 117)
(let std:int:char:v 118)
(let std:int:char:w 119)
(let std:int:char:x 120)
(let std:int:char:y 121)
(let std:int:char:z 122)
(let std:int:char:0 48)
(let std:int:char:1 49)
(let std:int:char:2 50)
(let std:int:char:3 51)
(let std:int:char:4 52)
(let std:int:char:5 53)
(let std:int:char:6 54)
(let std:int:char:7 55)
(let std:int:char:8 56)
(let std:int:char:9 57)
(let std:int:char:empty 0)
(let std:int:char:double-quote 34)
(let std:int:char:new-line 10)
(let std:int:char:space 32)
(let std:int:char:tab 9)
(let std:int:char:comma 44)
(let std:int:char:dot 46)
(let std:int:char:semi-colon 59)
(let std:int:char:colon 58)
(let std:int:char:dash 45)
(let std:int:char:left-brace 40)
(let std:int:char:right-brace 41)
(let std:int:char:curly-left-brace 123)
(let std:int:char:curly-right-brace 125)
(let std:int:char:left-bracket 91)
(let std:int:char:right-bracket 93)
(let std:int:char:pipe 124)
(let std:int:char:hash 35)
(let std:int:char:question-mark 63)
(let std:int:char:exclamation-mark 33)
(let std:int:char:minus 45)
(let std:int:char:plus 43)
(let std:int:char:equal 61)
(let std:int:char:asterix 42)
(let std:int:char:ampersand 38)
(let std:int:char:std:vector:at 64)
(let std:int:char:backtick 96)
(let std:int:char:digit? (lambda ch (and (>= ch std:int:char:0) (<= ch std:int:char:9))))
(let std:vector:string:upper (lambda char (if (and (>= char std:int:char:a) (<= char std:int:char:z)) (- char std:int:char:space) char)))
(let std:vector:string:lower (lambda char (if (and (>= char std:int:char:A) (<= char std:int:char:Z)) (+ char std:int:char:space) char)))
(let identity (lambda x x))


(let true (= 1 1))
(let false (= 0 1))
(let nil 0)
(let eq (lambda a b (cond 
          (and a b) true 
          (and (not a) (not b)) true
          false)))
(let std:vector:length (lambda xs (length xs)))
(let std:vector:get (lambda xs i (get xs i)))
(let std:vector:pop! (lambda xs (pop! xs)))
(let std:vector:set! (lambda xs i x (set! xs i x)))
(let std:vector:swap! (lambda xs i j (do (let temp (get xs i)) (std:vector:set! xs i (get xs j)) (std:vector:set! xs j temp) xs)))
(let std:vector:push! (lambda xs x (do (std:vector:set! xs (length xs) x) xs)))
(let std:vector:pop-and-get! (lambda xs (do 
      (let out (get xs (- (length xs) 1))) 
      (std:vector:pop! xs)
      out)))
(let std:vector:tail! (lambda xs (do (std:vector:pop! xs) xs)))
(let std:vector:append! (lambda xs x (do (std:vector:push! xs x) xs)))
(let std:vector:at (lambda xs i (if (< i 0) (get xs (+ (length xs) i)) (get xs i))))
(let std:vector:first (lambda xs (get xs 0)))
(let std:vector:second (lambda xs (get xs 1)))
(let std:vector:third (lambda xs (get xs 3)))
(let std:vector:last (lambda xs (get xs (- (length xs) 1))))
(let +. (lambda xs index (get xs index)))
(let -. (lambda xs index (get xs (- (length xs) index))))
(let std:int:min-safe -2147483648)
(let std:int:max-safe 2147383647)
(let std:int:safe? (lambda value (and (>= value std:int:min-safe) (<= value std:int:max-safe))))
(let std:int:get-safe (lambda var (if (std:int:safe? (get var)) var 0)))
(let int (lambda value (if (std:int:safe? value) [ value ] [ 0 ])))
(let box (lambda value [ value ]))
(let set (lambda var x (std:vector:set! var 0 x)))
(let =! (lambda var x (std:vector:set! var 0 x)))

(let boole-set (lambda var x (std:vector:set! var 0 (if x true false))))
(let boole-eqv (lambda a b (eq (get a) (get b))))

(let true? (lambda var (if (get var) true false)))
(let false? (lambda var (if (get var) false true)))


(let += (lambda var n (=! var (+ (get var) n))))
(let -= (lambda var n (=! var (- (get var) n))))
(let *= (lambda var n (=! var (* (get var) n))))
(let /= (lambda var n (=! var (/ (get var) n))))
(let ++ (lambda var (=! var (+ (get var) 1))))
(let -- (lambda var (=! var (- (get var) 1))))
(let ** (lambda var (=! var (* (get var) (get var)))))

(let std:vector:empty? (lambda xs (= (length xs) 0)))
(let std:vector:empty! (lambda xs (if (std:vector:empty? xs) xs (do 
     (loop 0 (length xs) (lambda . (std:vector:pop! xs)))
     xs))))
(let std:vector:not-empty? (lambda xs (not (= (length xs) 0))))
(let std:vector:in-bounds? (lambda xs index (and (< index (length xs)) (>= index 0))))

(let std:vector:filter (lambda xs cb? (if (std:vector:empty? xs) xs (do 
     (let out [])
     (let process (lambda i (do
      (let x (get xs i))
      (if (cb? x) (std:vector:set! out (length out) x)))))
     (loop 0 (length xs) process)
     out))))

(let std:vector:filter:i (lambda xs cb? (if (std:vector:empty? xs) xs (do 
     (let out [])
     (let process (lambda i (do
      (let x (get xs i))
      (if (cb? x i) (std:vector:set! out (length out) x)))))
     (loop 0 (length xs) process)
     out))))

(let std:vector:reduce (lambda xs cb initial (do
     (let out [ initial ])
     (let process (lambda i (std:vector:set! out 0 (cb (get out 0) (get xs i)))))
     (loop 0 (length xs) process)
     (get out))))

(let std:vector:reduce:i (lambda xs cb initial (do
     (let out [ initial ])
     (let process (lambda i (std:vector:set! out 0 (cb (get out 0) (get xs i) i))))
     (loop 0 (length xs) process)
     (get out))))

(let std:vector:map (lambda xs cb (if (std:vector:empty? xs) [] (do
     (let out [(cb (get xs 0))])
     (let process (lambda i (std:vector:set! out (length out) (cb (get xs i)))))
     (loop 1 (length xs) process)
     out))))

(let std:vector:map:i (lambda xs cb (if (std:vector:empty? xs) [] (do
     (let out [(cb (get xs 0) 0)])
     (let process (lambda i (std:vector:set! out (length out) (cb (get xs i) i))))
     (loop 1 (length xs) process)
     out))))

(let std:vector:ints:range (lambda start end (do
     (let out [ start ])
     (let process (lambda i (std:vector:set! out (length out) i)))
     (loop (+ start 1) (+ end 1) process)
     out))) 

 (let std:vector:ints:ones (lambda n (do
     (let out [ 1 ])
     (let process (lambda i (std:vector:set! out (length out) 1)))
     (loop 0 n process)
     out))) 

 (let std:vector:ints:zeroes (lambda n (do
     (let out [ 0 ])
     (let process (lambda i (std:vector:set! out (length out) 0)))
     (loop 0 n process)
     out))) 

(let std:vector:count-of (lambda xs cb? (length (std:vector:filter xs cb?))))
(let std:vector:ints:count (lambda input item (std:vector:count-of input (lambda x (= x item)))))

(let std:vector:cons (lambda a b (if (and (std:vector:empty? a) (std:vector:empty? b)) a (do 
  (let out []) 
  (loop 0 (length a) (lambda i (std:vector:set! out (length out) (get a i)))) 
  (loop 0 (length b) (lambda i (std:vector:set! out (length out) (get b i)))) 
  out))))

(let std:vector:cons! (lambda a b (if (and (std:vector:empty? a) (std:vector:empty? b)) a (do 
  (loop 0 (length b) (lambda i (std:vector:set! a (length a) (get b i)))) 
  a))))

(let std:vector:concat (lambda xs (std:vector:reduce xs std:vector:cons [])))
(let std:vector:concat! (lambda xs os (std:vector:reduce os std:vector:cons! xs)))

(let std:vector:every? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop (and (< (get i) len) (predicate? (get xs (get i)))) (lambda (std:vector:set! i 0 (+ (get i) 1))))
           (not (> len (get i))))))

(let std:vector:some? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop (and (< (get i) len) (not (predicate? (get xs (get i))))) (lambda (std:vector:set! i 0 (+ (get i) 1))))
           (or (= len 0) (> len (get i))))))

(let std:vector:every:i? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop (and (< (get i) len) (predicate? (get xs (get i)) (get i))) (lambda (std:vector:set! i 0 (+ (get i) 1))))
           (not (> len (get i))))))

(let std:vector:some:i? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop (and (< (get i) len) (not (predicate? (get xs (get i)) (get i)))) (lambda (std:vector:set! i 0 (+ (get i) 1))))
           (or (= len 0) (> len (get i))))))

(let std:vector:cartesian-product (lambda a b (std:vector:reduce a (lambda p x (std:vector:cons p (std:vector:map b (lambda y [ x y ])))) [])))

(let std:int:gcd (lambda a b (do
    (integer A a)
    (integer B b)
    (loop (> (get B) 0) (lambda (do
        (let a (get A))
        (let b (get B))
        (set A b)
        (set B (mod a b)))))
    (get A))))
(let std:int:lcm (lambda a b (/ (* a b) (std:int:gcd  a b))))

(let std:int:bit:set? (lambda n pos (= (& n (<< 1 pos)) 0)))
(let std:int:bit:set (lambda n pos (| n (<< 1 pos))))
(let std:int:bit:clear (lambda n pos (& n (~ (<< 1 pos)))))
(let std:int:bit:power-of-two (lambda n (<< 2 (- n 1))))
(let std:int:bit:odd? (lambda n (= (& n 1) 1)))
(let std:int:bit:even? (lambda n (= (& n 1) 0)))
(let std:int:bit:average (lambda a b (>> (+ a b) 1)))
(let std:int:bit:flag-flip (lambda x (- 1 (* x x))))
(let std:int:bit:toggle (lambda n a b (^ (^ a b) n)))
(let std:int:bit:same-sign? (lambda a b (>= (^ a b) 0)))
(let std:int:bit:max (lambda a b (- a (& (- a b) (>> (- a b) 31)))))
(let std:int:bit:min (lambda a b (- a (& (- a b) (>> (- b a) 31)))))
(let std:int:bit:equal? (lambda a b (< (^ a b) 1)))
(let std:int:bit:modulo (lambda numerator divisor (& numerator (- divisor 1))))
(let std:int:bit:n-one? (lambda N nth (not (= (& N (<< 1 nth)) 0))))
(let std:int:bit:largest-power (lambda N (do
  ; changing all right side bits to 1.
  (let N1 (| N (>> N 1)))
  (let N2 (| N1 (>> N1 2)))
  (let N3 (| N2 (>> N2 4)))
  (let N4 (| N3 (>> N3 8)))
  ; as now the number is 2 * x - 1,
  ; where x is required answer,
  ; so adding 1 and dividing it by
  (>> (+ N4 1) 1))))
(let std:int:abs (lambda n (- (^ n (>> n 31)) (>> n 31))))
(let std:int:positive? (lambda x (> x 0)))
(let std:int:negative? (lambda x (< x 0)))
(let std:int:invert (lambda x (- x)))
(let std:int:zero? (lambda x (= x 0)))
(let std:int:negative-one? (lambda x (= x -1)))
(let std:int:divisible? (lambda a b (= (mod a b) 0)))

(let std:int:square (lambda x (* x x)))
(let std:int:even? (lambda x (= (mod x 2) 0)))
(let std:int:odd? (lambda x (not (= (mod x 2) 0))))
(let std:vector:ints:sum (lambda xs (std:vector:reduce xs (lambda a b (+ a b)) 0)))
(let std:vector:ints:product (lambda xs (std:vector:reduce xs (lambda a b (* a b)) 1)))
(let std:int:euclidean-mod (lambda a b (mod (+ (mod a b) b) b)))
(let std:int:max (lambda a b (if (> a b) a b)))
(let std:int:min (lambda a b (if (< a b) a b)))
(let std:vector:ints:maximum (lambda xs (cond (std:vector:empty? xs) nil (= (length xs) 1) (get xs 0) (std:vector:reduce xs std:int:max (get xs 0)))))
(let std:vector:ints:minimum (lambda xs (cond (std:vector:empty? xs) nil (= (length xs) 1) (get xs 0) (std:vector:reduce xs std:int:min (get xs 0)))))
(let std:int:normalize (lambda value min max (* (- value min) (/ (- max min)))))
(let std:int:linear-interpolation (lambda a b n (+ (* (- 1 n) a) (* n b))))
(let std:int:gauss-sum (lambda n (/ (* n (+ n 1)) 2)))
(let std:int:gauss-sum-sequance (lambda a b (/ (* (+ a b) (+ (- b a) 1)) 2)))
(let std:int:clamp (lambda x limit (if (> x limit) limit x)))
(let std:int:clamp-range (lambda x start end (cond (> x end) end (< x start) start x)))
(let std:int:between? (lambda v min max (and (> v min) (< v max))))
(let std:int:overlap? (lambda v min max (and (>= v min) (<= v max))))
(let std:int:sqrt (lambda n
  (do
    (integer x n)
    (integer prev 0)
    (loop (> (std:int:abs (- (get x) (get prev))) 0)
      (lambda (do
        (set prev (get x))
        (set x (/ (+ (get x) (/ n (get x))) 2)))))
    (get x))))
(let std:int:expt (lambda base exp (do
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

(let std:vector:zipper (lambda a b (do 
      (let out [[(get a 0) (get b 0)]])
      (let process (lambda i (std:vector:set! out (length out) [(get a i) (get b i)])))
      (loop 1 (length a) process)
      out)))

(let std:vector:zip (lambda xs (std:vector:zipper (std:vector:first xs) (std:vector:second xs))))
(let std:vector:unzip (lambda xs [ (std:vector:map xs std:vector:first) (std:vector:map xs std:vector:second) ]))

(let std:vector:slice (lambda xs start end (if (std:vector:empty? xs) xs (do
     (let bounds (- end start))
     (let out [])
     (let process (lambda i (std:vector:set! out (length out) (get xs (+ start i)))))
     (loop 0 bounds process)
     out))))

(let std:vector:reverse (lambda xs (if (std:vector:empty? xs) xs (do
     (let out [])
     (let len (length xs))
     (let process (lambda i (std:vector:set! out (length out) (get xs (- len i 1)))))
     (loop 0 len process)
     out))))

(let std:vector:find-index (lambda xs cb? (do
     (let i [ 0 ])
     (let index [ -1 ])
     (let len (length xs))
     (let process (lambda
           (if (cb? (get xs (get i)))
              (std:vector:set! index 0 (get i))
              (std:vector:set! i 0 (+ (get i) 1)))))
     (loop (and (< (get i) len) (= (get index 0) -1)) process)
     (get index 0))))

(let std:vector:buckets (lambda size (do
     (let out [[]])
     (loop 1 size (lambda . (std:vector:set! out (length out) [])))
     out)))

(let std:vector:string:match? (lambda a b (and (= (length a) (length b)) (|>
  a
  (std:vector:zipper b)
  (std:vector:every? (lambda x (= (get x 0) (get x 1))))))))

(let std:vector:partition (lambda xs n (if (= n (length xs)) [xs] (do 
    (let a [])
    (loop 0 (length xs) (lambda i (if (= (mod i n) 0)
        (std:vector:set! a (length a) [(get xs i)])
        (std:vector:set! (std:vector:at a -1) (length (std:vector:at a -1)) (get xs i)))))
     a))))

(let std:vector:sort-partition! (lambda arr start end cb (do
     (let pivot (get arr end))
     (let i [(- start 1)])
     (let j [ start ])

     (let helper (lambda i j (do
          (std:vector:set! i 0 (+ (get i) 1))
          (std:vector:swap! arr (get i) (get j))
          nil)))

     (let process (lambda (do
           (if (cb (get arr (get j)) pivot) (helper i j))
           (std:vector:set! j 0 (+ (get j) 1)))))
     (loop (< (get j) end) process)

     (std:vector:swap! arr (+ (get i) 1) end)
     (+ (get i) 1))))

(let std:vector:sort! (lambda arr cb (do
     (let stack [])
     (std:vector:push! stack 0)
     (std:vector:push! stack (- (length arr) 1))
     (let process (lambda (do
           (let end (get stack (- (length stack) 1)))
           (std:vector:pop! stack)
           (let start (get stack (- (length stack) 1)))
           (std:vector:pop! stack)
           (let helper (lambda (do
                 (let pivot-index (std:vector:sort-partition! arr start end cb))
                 (std:vector:push! stack start)
                 (std:vector:push! stack (- pivot-index 1))
                 (std:vector:push! stack (+ pivot-index 1))
                 (std:vector:push! stack end)
                 nil)))
           (if (< start end) (helper)))))
     (loop (> (length stack) 0) process)
     arr)))

(let std:int:hash
 (lambda table key (do
     (let prime-num 31)
     (let total [ 0 ])
     (let i [ 0 ])
     (let bounds (if (< (- (length key) 1) 100) (- (length key) 1) 100))

     (let process (lambda (do
           (let letter (get key (get i)))
           (std:vector:set! total 0 (euclidean-mod (+ (* (get total 0 ) prime-num) letter) (length table)))
           (std:vector:set! i 0 (+ (get i) 1)))))

     (loop (< (get i) bounds) process)
     (get total 0))))

(let std:vector:hash:set:has? (lambda table key (do
     (let idx (std:int:hash table key))
     (let current (get table idx))
     (and (std:vector:in-bounds? table idx)
                  (and (> (length current) 0)
                       (>= (std:vector:find-index current (lambda x (std:vector:string:match? x key))) 0))))))

(let std:vector:hash:table:has? (lambda table key (do
         (let idx (std:int:hash table key))
         (let current (std:vector:map (get table idx) (lambda x (get x 0))))
         (and (std:vector:in-bounds? table idx)
         (and (> (length current) 0)
           (>= (std:vector:find-index current
             (lambda x
               (std:vector:string:match? x key))) 0))))))

(let std:vector:hash:set:add!
     (lambda table key
       (do
         (let idx (std:int:hash table key))
         (if (not (std:vector:in-bounds? table idx)) (std:vector:set! table idx []) nil)
         (let current (get table idx))
         (let len (length current))
         (let index (if (> len 0) (std:vector:find-index current (lambda x (std:vector:string:match? x key))) -1))
         (let entry key)
         (if (= index -1)
           (std:vector:set! current (length current) entry)
           (std:vector:set! current index entry)) table)))

(let std:vector:hash:set:remove!
 (lambda table key
   (do
     (let idx (std:int:hash table key))
     (if (not (std:vector:in-bounds? table idx)) (std:vector:set! table idx []) nil)
     (let current (get table idx))
     (let len (length current))
     (let index (if (> len 0) (std:vector:find-index current (lambda x (std:vector:string:match? x key))) -1))
     (let entry key)
     (if (not (= index -1)) (do (std:vector:set! current index (std:vector:at current -1)) (std:vector:pop! current)) nil)
     table)))

(let std:vector:hash:table:set! (lambda table key value
       (do
         (let idx (std:int:hash table key))
         (if (not (std:vector:in-bounds? table idx)) (std:vector:set! table idx []) nil)
         (let current (get table idx))
         (let len (length current))
         (let index (if (> len 0) (std:vector:find-index current (lambda x (std:vector:string:match? (get x 0) key))) -1))
         (let entry [ key [value] ])
         (if (= index -1)
           (std:vector:set! current (length current) entry)
           (std:vector:set! current index entry))
         table)))
        
(let std:vector:hash:table:delete! (lambda table key
     (do
       (let idx (std:int:hash table key))
       (if (not (std:vector:in-bounds? table idx)) (std:vector:set! table idx []) nil)
       (let current (get table idx))
       (let len (length current))
       (let index (if (> len 0) (std:vector:find-index current (lambda x (std:vector:string:match? (get x 0) key))) -1))
       (if (not (= index -1)) (do (std:vector:set! current index (std:vector:at current -1)) (std:vector:pop! current)) nil)
       table)))

(let std:vector:hash:clear! (lambda table (do 
     (loop 0 (length table) (lambda i (std:vector:empty! (get table i))))
     table)))

(let std:vector:hash:table:keys (lambda table (|> table (std:vector:flat-one) (std:vector:map std:vector:first))))
(let std:vector:hash:table:values (lambda table (|> table (std:vector:flat-one) (std:vector:map std:vector:second))))
(let std:vector:hash:table:entries (lambda table (|> table (std:vector:flat-one))))

(let std:vector:hash:table:get-helper (lambda table idx key (do
   (let current (get table idx))
   (let found-index (std:vector:find-index current (lambda x (std:vector:string:match? key (get x 0)))))
   (unless (= found-index -1) (get current found-index 1) []))))

(let std:vector:hash:table:get (lambda table key (do
     (let idx (std:int:hash table key))
     (if (std:vector:in-bounds? table idx) (get (std:vector:hash:table:get-helper table idx key)) -1))))

(let table-count (lambda arr 
    (|> arr (std:vector:reduce (lambda table key (do 
        (if (std:vector:hash:table:has? table key) 
            (std:vector:hash:table:set! table key (+ (std:vector:hash:table:get table key) 1))
            (std:vector:hash:table:set! table key 1)))) (std:vector:buckets 64)))))

(let std:vector:sliding-window (lambda xs size (cond 
     (std:vector:empty? xs) []
     (= size (length xs)) [xs]
     (std:vector:reduce:i xs (lambda a b i (if (> (+ i size) (length xs)) a (std:vector:cons a [(std:vector:slice xs i (+ i size))]))) []))))

(let std:vector:flat-one (lambda xs (cond 
     (std:vector:empty? xs) []
     (= (length xs) 1) (get xs)
     (std:vector:reduce xs (lambda a b (std:vector:cons a b)) []))))

(let std:convert:char->digit (lambda ch (- ch std:int:char:0)))
(let std:convert:chars->digits (lambda digits (std:vector:map digits std:convert:char->digit)))
(let std:convert:digit->char (lambda digit (+ digit std:int:char:0)))
(let std:convert:digits->chars (lambda digits (std:vector:map digits std:convert:digit->char)))
(let std:convert:bool->int (lambda x (if (eq x true) 1 0)))
(let std:convert:int->bool (lambda x (if (= x 0) false true)))
(let std:convert:vector->string (lambda xs delim (std:vector:reduce:i xs (lambda a b i (if (> i 0) (std:vector:cons (std:vector:append! a delim) b) b)) [])))
(let std:convert:string->vector (lambda str char (|> str
              (std:vector:reduce(lambda a b (do
              (let prev (std:vector:at a -1))
                (if (std:vector:string:match? [b] [char])
                    (std:vector:set! a (length a) [])
                    (std:vector:set! prev (length prev) b)) a))
              [[]])
              (std:vector:map (lambda x (std:convert:vector->string [ x ] std:int:char:empty))))))

(let std:convert:positive-or-negative-digits->integer (lambda digits-with-sign (do
    (let std:int:negative? (< (std:vector:first digits-with-sign) 0))
    (let digits (if std:int:negative? (std:vector:map digits-with-sign std:int:abs) digits-with-sign))
    (integer num 0)
    (integer base (/ (std:int:expt 10 (length digits)) 10))
    (loop 0 (length digits) (lambda i (do 
      (+= num (* (get base) (. digits i)))
      (/= base 10)
    )))
    (*= num (if std:int:negative? -1 1))
    (get num))))

(let std:convert:chars->positive-or-negative-digits (lambda chars (do
    (integer current-sign 1)
    (|> chars 
        (std:vector:reduce (lambda a ch (do 
            (if (= ch std:int:char:minus) 
                (set current-sign -1) 
                (do  
                    (std:vector:push! a (* (get current-sign) (std:convert:char->digit ch))) 
                    (set current-sign 1)))
                a)) [])))))
(let std:convert:positive-or-negative-chars->integer (lambda x (|> x (std:convert:chars->positive-or-negative-digits) (std:convert:positive-or-negative-digits->integer))))
(let std:convert:chars->integer (lambda chars (std:convert:positive-or-negative-chars->integer chars)))

; (let buffer [])
; (let fn (ring-buffer buffer 5))
; (let buffer:get (get fn 0))
; (let buffer:std:vector:push! (get fn 1))
; (loop 0 6 (lambda i (buffer:std:vector:push! i)))
; buffer
(let ring-buffer (lambda buffer len (do 
    (integer pointer 0)
    [(lambda index (get buffer index)) 
    (lambda item (do 
        (let pt (get pointer))
        (std:vector:set! buffer pt item)
        (set pointer (mod (+ len pt 1) len))
        item))])))

(let std:vector:unique-pairs (lambda xs (do 
    (let pairs [])
    (let len (length xs))
    (integer i 0)
    (loop (< (get i) len) (lambda (do 
        (integer j (+ (get i) 1))
        (loop (< (get j) len) (lambda (do 
            (std:vector:push! pairs [(get xs (get i)) (get xs (get j))])
            (++ j))))
        (++ i))))
    pairs)))

(let apply-0 (lambda x fn (fn x)))
(let apply-1 (lambda x y fn (fn x y)))
(let apply-2 (lambda x y z fn (fn x y z)))

(let std:vector:dimensions (lambda matrix [ (length matrix) (length (get matrix 0)) ]))
(let std:vector:in-bounds? (lambda matrix y x (and (std:vector:in-bounds? matrix y) (std:vector:in-bounds? (get matrix y) x))))