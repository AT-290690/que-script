(let char:A 65)
(let char:B 66)
(let char:C 67)
(let char:D 68)
(let char:E 69)
(let char:F 70)
(let char:G 71)
(let char:H 72)
(let char:I 73)
(let char:J 74)
(let char:K 75)
(let char:L 76)
(let char:M 77)
(let char:N 78)
(let char:O 79)
(let char:P 80)
(let char:Q 81)
(let char:R 82)
(let char:S 83)
(let char:T 84)
(let char:U 85)
(let char:V 86)
(let char:W 87)
(let char:X 88)
(let char:Y 89)
(let char:Z 90)
(let char:a 97)
(let char:b 98)
(let char:c 99)
(let char:d 100)
(let char:e 101)
(let char:f 102)
(let char:g 103)
(let char:h 104)
(let char:i 105)
(let char:j 106)
(let char:k 107)
(let char:l 108)
(let char:m 109)
(let char:n 110)
(let char:o 111)
(let char:p 112)
(let char:q 113)
(let char:r 114)
(let char:s 115)
(let char:t 116)
(let char:u 117)
(let char:v 118)
(let char:w 119)
(let char:x 120)
(let char:y 121)
(let char:z 122)
(let char:0 48)
(let char:1 49)
(let char:2 50)
(let char:3 51)
(let char:4 52)
(let char:5 53)
(let char:6 54)
(let char:7 55)
(let char:8 56)
(let char:9 57)
(let char:empty 0)
(let char:double-quote 34)
(let char:new-line 10)
(let char:space 32)
(let char:comma 44)
(let char:dot 46)
(let char:semi-colon 59)
(let char:colon 58)
(let char:dash 45)
(let char:left-brace 40)
(let char:right-brace 41)
(let char:curly-left-brace 123)
(let char:curly-right-brace 125)
(let char:left-bracket 91)
(let char:right-bracket 93)
(let char:pipe 124)
(let char:hash 35)
(let char:question-mark 63)
(let char:exclamation-mark 33)
(let char:minus 45)
(let char:plus 43)
(let char:equal 61)
(let char:asterix 42)
(let char:ampersand 38)
(let char:at 64)
(let char:backtick 96)
(let digit? (lambda ch (and (>= ch char:0) (<= ch char:9))))
(let upper (lambda char (if (and (>= char char:a) (<= char char:z)) (- char char:space) char)))
(let lower (lambda char (if (and (>= char char:A) (<= char char:Z)) (+ char char:space) char)))
(let identity (lambda x x))


(let true (= 1 1))
(let false (= 0 1))
(let nil 0)
(let eq (lambda a b (cond 
          (and a b) true 
          (and (not a) (not b)) true
          false)))

(let swap! (lambda xs i j (do (let temp (get xs i)) (set! xs i (get xs j)) (set! xs j temp) xs)))
(let push! (lambda xs x (do (set! xs (length xs) x) xs)))
(let pop-and-get! (lambda xs (do 
      (let out (get xs (- (length xs) 1))) 
      (pop! xs)
      out)))
(let tail! (lambda xs (do (pop! xs) xs)))
(let append! (lambda xs x (do (push! xs x) xs)))
(let at (lambda xs i (if (< i 0) (get xs (+ (length xs) i)) (get xs i))))
(let first (lambda xs (get xs 0)))
(let second (lambda xs (get xs 1)))
(let third (lambda xs (get xs 3)))
(let last (lambda xs (get xs (- (length xs) 1))))
(let +. (lambda xs index (get xs index)))
(let -. (lambda xs index (get xs (- (length xs) index))))
(let min-safe-int -2147483648)
(let max-safe-int 2147383647)
(let safe-int? (lambda value (and (>= value min-safe-int) (<= value max-safe-int))))
(let get-safe-int (lambda var (if (safe-int? (get var)) var 0)))
(let int (lambda value (if (safe-int? value) [ value ] [ 0 ])))
(let box (lambda value [ value ]))
(let set (lambda var x (set! var 0 x)))
(let =! (lambda var x (set! var 0 x)))

(let boole-set (lambda var x (set! var 0 (if x true false))))
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

(let empty? (lambda xs (= (length xs) 0)))
(let not-empty? (lambda xs (not (= (length xs) 0))))
(let in-bounds? (lambda xs index (and (< index (length xs)) (>= index 0))))

(let filter (lambda xs cb? (if (empty? xs) xs (do 
     (let out [])
     (let process (lambda i (do
      (let x (get xs i))
      (if (cb? x) (set! out (length out) x)))))
     (loop 0 (length xs) process)
     out))))

(let ifilter (lambda xs cb? (if (empty? xs) xs (do 
     (let out [])
     (let process (lambda i (do
      (let x (get xs i))
      (if (cb? x i) (set! out (length out) x)))))
     (loop 0 (length xs) process)
     out))))

(let reduce (lambda xs cb initial (do
     (let out [ initial ])
     (let process (lambda i (set! out 0 (cb (get out 0) (get xs i)))))
     (loop 0 (length xs) process)
     (get out))))

(let ireduce (lambda xs cb initial (do
     (let out [ initial ])
     (let process (lambda i (set! out 0 (cb (get out 0) (get xs i) i))))
     (loop 0 (length xs) process)
     (get out))))

(let map (lambda xs cb (if (empty? xs) [] (do
     (let out [(cb (get xs 0))])
     (let process (lambda i (set! out (length out) (cb (get xs i)))))
     (loop 1 (length xs) process)
     out))))

(let imap (lambda xs cb (if (empty? xs) [] (do
     (let out [(cb (get xs 0) 0)])
     (let process (lambda i (set! out (length out) (cb (get xs i) i))))
     (loop 1 (length xs) process)
     out))))

(let range (lambda start end (do
     (let out [ start ])
     (let process (lambda i (set! out (length out) i)))
     (loop (+ start 1) (+ end 1) process)
     out))) 

 (let ones (lambda n (do
     (let out [ 1 ])
     (let process (lambda i (set! out (length out) 1)))
     (loop 0 n process)
     out))) 

 (let zeroes (lambda n (do
     (let out [ 0 ])
     (let process (lambda i (set! out (length out) 0)))
     (loop 0 n process)
     out))) 

(let count-of (lambda xs cb? (length (filter xs cb?))))
(let count (lambda input item (count-of input (lambda x (= x item)))))

(let merge (lambda a b (if (and (empty? a) (empty? b)) a (do 
  (let out []) 
  (loop 0 (length a) (lambda i (set! out (length out) (get a i)))) 
  (loop 0 (length b) (lambda i (set! out (length out) (get b i)))) 
  out))))
(let concat (lambda xs (reduce xs merge [])))

(let every? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop-finish (and (< (get i) len) (predicate? (get xs (get i)))) (lambda (set! i 0 (+ (get i) 1))))
           (not (> len (get i))))))

(let some? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop-finish (and (< (get i) len) (not (predicate? (get xs (get i))))) (lambda (set! i 0 (+ (get i) 1))))
           (or (= len 0) (> len (get i))))))

(let ievery? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop-finish (and (< (get i) len) (predicate? (get xs (get i)) (get i))) (lambda (set! i 0 (+ (get i) 1))))
           (not (> len (get i))))))

(let isome? (lambda xs predicate? (do
           (let i [ 0 ])
           (let len (length xs))
           (loop-finish (and (< (get i) len) (not (predicate? (get xs (get i)) (get i)))) (lambda (set! i 0 (+ (get i) 1))))
           (or (= len 0) (> len (get i))))))

(let cartesian-product (lambda a b (reduce a (lambda p x (merge p (map b (lambda y [ x y ])))) [])))

(let gcd (lambda a b (do
    (integer A a)
    (integer B b)
    (loop-finish (> (get B) 0) (lambda (do
        (let a (get A))
        (let b (get B))
        (set A b)
        (set B (mod a b)))))
    (get A))))
(let lcm (lambda a b (/ (* a b) (gcd a b))))


(let bit-set? (lambda n pos (= (& n (<< 1 pos)) 0)))
(let bit-set (lambda n pos (| n (<< 1 pos))))
(let bit-clear (lambda n pos (& n (~ (<< 1 pos)))))
(let power-of-two-bits (lambda n (<< 2 (- n 1))))
(let odd-bit? (lambda n (= (& n 1) 1)))
(let even-bit? (lambda n (= (& n 1) 0)))
(let average-bit (lambda a b (>> (+ a b) 1)))
(let flag-flip (lambda x (- 1 (* x x))))
(let toggle-bit (lambda n a b (^ (^ a b) n)))
(let same-sign-bit? (lambda a b (>= (^ a b) 0)))
(let max-bit (lambda a b (- a (& (- a b) (>> (- a b) 31)))))
(let min-bit (lambda a b (- a (& (- a b) (>> (- b a) 31)))))
(let bit-equal? (lambda a b (< (^ a b) 1)))
(let modulo-bit (lambda numerator divisor (& numerator (- divisor 1))))
(let n-one-bit? (lambda N nth (not (= (& N (<< 1 nth)) 0))))
(let largest-power (lambda N (do
  ; changing all right side bits to 1.
  (let N1 (| N (>> N 1)))
  (let N2 (| N1 (>> N1 2)))
  (let N3 (| N2 (>> N2 4)))
  (let N4 (| N3 (>> N3 8)))
  ; as now the number is 2 * x - 1,
  ; where x is required answer,
  ; so adding 1 and dividing it by
  (>> (+ N4 1) 1))))
(let abs (lambda n (- (^ n (>> n 31)) (>> n 31))))
(let positive? (lambda x (> x 0)))
(let negative? (lambda x (< x 0)))
(let invert (lambda x (- x)))
(let zero? (lambda x (= x 0)))
(let negative-one? (lambda x (= x -1)))
(let divisible? (lambda a b (= (mod a b) 0)))

(let square (lambda x (* x x)))
(let even? (lambda x (= (mod x 2) 0)))
(let odd? (lambda x (not (= (mod x 2) 0))))
(let sum (lambda xs (reduce xs (lambda a b (+ a b)) 0)))
(let product (lambda xs (reduce xs (lambda a b (* a b)) 1)))
(let euclidean-mod (lambda a b (mod (+ (mod a b) b) b)))
(let max (lambda a b (if (> a b) a b)))
(let min (lambda a b (if (< a b) a b)))
(let maximum (lambda xs (cond (empty? xs) nil (= (length xs) 1) (get xs 0) (reduce xs max (get xs 0)))))
(let minimum (lambda xs (cond (empty? xs) nil (= (length xs) 1) (get xs 0) (reduce xs min (get xs 0)))))
(let normalize (lambda value min max (* (- value min) (/ (- max min)))))
(let linear-interpolation (lambda a b n (+ (* (- 1 n) a) (* n b))))
(let gauss-sum (lambda n (/ (* n (+ n 1)) 2)))
(let gauss-sum-sequance (lambda a b (/ (* (+ a b) (+ (- b a) 1)) 2)))
(let clamp (lambda x limit (if (> x limit) limit x)))
(let clamp-range (lambda x start end (cond (> x end) end (< x start) start x)))
(let between? (lambda v min max (and (> v min) (< v max))))
(let overlap? (lambda v min max (and (>= v min) (<= v max))))
(let sqrt (lambda n
  (do
    (integer x n)
    (integer prev 0)
    (loop-finish (> (abs (- (get x) (get prev))) 0)
      (lambda (do
        (set prev (get x))
        (set x (/ (+ (get x) (/ n (get x))) 2)))))
    (get x))))
(let expt (lambda base exp (do
  (if (< exp 0) 0 (do
      (integer result 1)
      (integer b base)
      (integer e exp)
      (loop-finish (> (get e) 0)
        (lambda (do
          (if (= (mod (get e) 2) 1)
            (set result (* (get result) (get b))))
          (set b (* (get b) (get b)))
          (set e (/ (get e) 2)))))
      (get result))))))

(let zipper (lambda a b (do 
      (let out [[(get a 0) (get b 0)]])
      (let process (lambda i (set! out (length out) [(get a i) (get b i)])))
      (loop 1 (length a) process)
      out)))

(let zip (lambda xs (zipper (first xs) (second xs))))
(let unzip (lambda xs (array (map xs first) (map xs second))))

(let slice (lambda xs start end (if (empty? xs) xs (do
     (let bounds (- end start))
     (let out [])
     (let process (lambda i (set! out (length out) (get xs (+ start i)))))
     (loop 0 bounds process)
     out))))

(let reverse (lambda xs (if (empty? xs) xs (do
     (let out [])
     (let len (length xs))
     (let process (lambda i (set! out (length out) (get xs (- len i 1)))))
     (loop 0 len process)
     out))))

(let find-index (lambda xs cb? (do
     (let i [ 0 ])
     (let index [ -1 ])
     (let len (length xs))
     (let process (lambda
           (if (cb? (get xs (get i)))
              (set! index 0 (get i))
              (set! i 0 (+ (get i) 1)))))
     (loop-finish (and (< (get i) len) (= (get index 0) -1)) process)
     (get index 0))))

(let buckets (lambda size (do
     (let out [[]])
     (loop 1 size (lambda . (set! out (length out) [])))
     out)))

(let match? (lambda a b (and (= (length a) (length b)) (|>
  a
  (zipper b)
  (every? (lambda x (= (get x 0) (get x 1))))))))

(let partition (lambda xs n (if (= n (length xs)) [xs] (do 
    (let a [])
    (loop 0 (length xs) (lambda i (if (= (mod i n) 0)
        (set! a (length a) [(get xs i)])
        (set! (at a -1) (length (at a -1)) (get xs i)))))
     a))))

(let sort-partition! (lambda arr start end cb (do
     (let pivot (get arr end))
     (let i [(- start 1)])
     (let j [ start ])

     (let helper (lambda i j (do
          (set! i 0 (+ (get i) 1))
          (swap! arr (get i) (get j))
          nil)))

     (let process (lambda (do
           (if (cb (get arr (get j)) pivot) (helper i j))
           (set! j 0 (+ (get j) 1)))))
     (loop-finish (< (get j) end) process)

     (swap! arr (+ (get i) 1) end)
     (+ (get i) 1))))

(let sort! (lambda arr cb (do
     (let stack [])
     (push! stack 0)
     (push! stack (- (length arr) 1))
     (let process (lambda (do
           (let end (get stack (- (length stack) 1)))
           (pop! stack)
           (let start (get stack (- (length stack) 1)))
           (pop! stack)
           (let helper (lambda (do
                 (let pivot-index (sort-partition! arr start end cb))
                 (push! stack start)
                 (push! stack (- pivot-index 1))
                 (push! stack (+ pivot-index 1))
                 (push! stack end)
                 nil)))
           (if (< start end) (helper)))))
     (loop-finish (> (length stack) 0) process)
     arr)))

(let hash
 (lambda table key (do
     (let prime-num 31)
     (let total [ 0 ])
     (let i [ 0 ])
     (let bounds (if (< (- (length key) 1) 100) (- (length key) 1) 100))

     (let process (lambda (do
           (let letter (get key (get i)))
           (set! total 0 (euclidean-mod (+ (* (get total 0 ) prime-num) letter) (length table)))
           (set! i 0 (+ (get i) 1)))))

     (loop-finish (< (get i) bounds) process)
     (get total 0))))

(let has-element? (lambda table key (do
     (let idx (hash table key))
     (let current (get table idx))
     (and (in-bounds? table idx)
                  (and (> (length current) 0)
                       (>= (find-index current (lambda x (match? x key))) 0))))))

(let has-property? (lambda table key (do
         (let idx (hash table key))
         (let current (map (get table idx) (lambda x (get x 0))))
         (and (in-bounds? table idx)
         (and (> (length current) 0)
           (>= (find-index current
             (lambda x
               (match? x key))) 0))))))

(let add-element!
     (lambda table key
       (do
         (let idx (hash table key))
         (if (not (in-bounds? table idx)) (set! table idx (array)) nil)
         (let current (get table idx))
         (let len (length current))
         (let index (if (> len 0) (find-index current (lambda x (match? x key))) -1))
         (let entry key)
         (if (= index -1)
           (set! current (length current) entry)
           (set! current index entry)) table)))

(let remove-element!
 (lambda table key
   (do
     (let idx (hash table key))
     (if (not (in-bounds? table idx)) (set! table idx (array)) nil)
     (let current (get table idx))
     (let len (length current))
     (let index (if (> len 0) (find-index current (lambda x (match? x key))) -1))
     (let entry key)
     (if (not (= index -1)) (do (set! current index (at current -1)) (pop! current)) nil)
     table)))

(let set-property! (lambda table key value
       (do
         (let idx (hash table key))
         (if (not (in-bounds? table idx)) (set! table idx []) nil)
         (let current (get table idx))
         (let len (length current))
         (let index (if (> len 0) (find-index current (lambda x (match? (get x 0) key))) -1))
         (let entry [ key [value] ])
         (if (= index -1)
           (set! current (length current) entry)
           (set! current index entry))
         table)))
        
(let delete-property! (lambda table key
     (do
       (let idx (hash table key))
       (if (not (in-bounds? table idx)) (set! table idx []) nil)
       (let current (get table idx))
       (let len (length current))
       (let index (if (> len 0) (find-index current (lambda x (match? (get x 0) key))) -1))
       (if (not (= index -1)) (do (set! current index (at current -1)) (pop! current)) nil)
       table)))

(let access-property-helper (lambda table idx key (do
   (let current (get table idx))
   (let found-index (find-index current (lambda x (match? key (get x 0)))))
   (unless (= found-index -1) (get current found-index 1) []))))

(let access-property (lambda table key (do
     (let idx (hash table key))
     (if (in-bounds? table idx) (get (access-property-helper table idx key)) -1))))

(let table-count (lambda arr 
    (|> arr (reduce (lambda table key (do 
        (if (has-property? table key) 
            (set-property! table key (+ (access-property table key) 1))
            (set-property! table key 1)))) (buckets 64)))))

(let sliding-window-array (lambda arr size (cond 
     (empty? arr) []
     (= size (length arr)) [arr]
     (do
          (let out [])
          (let i (box 0))
          (loop-finish (<= (+ (get i) size) (length arr)) (lambda (do
               (let window [])
               (let j (box 0))
               (loop-finish (< (get j) size) (lambda (do
                    (push! window (get arr (+ (get i) (get j))))
                    (++ j))))
               (push! out window)
               (++ i))))
          out))))
(let flat-one (lambda xs (cond 
     (empty? xs) []
     (= (length xs) 1) (get xs)
     (reduce xs (lambda a b (merge a b)) []))))

(let char->digit (lambda ch (- ch char:0)))
(let chars->digits (lambda digits (map digits char->digit)))
(let digit->char (lambda digit (+ digit char:0)))
(let digits->chars (lambda digits (map digits digit->char)))
(let bool->int (lambda x (if (eq x true) 1 0)))
(let int->bool (lambda x (if (= x 0) false true)))
(let array->string (lambda xs delim (ireduce xs (lambda a b i (if (> i 0) (merge (append! a delim) b) b)) [])))
(let string->array (lambda str char (|> str
              (reduce(lambda a b (do
              (let prev (at a -1))
                (if (match? [b] [char])
                    (set! a (length a) [])
                    (set! prev (length prev) b)) a))
              [[]])
              (map (lambda x (array->string [ x ] char:empty))))))

(let positive-or-negative-digits->integer (lambda digits-with-sign (do
    (let negative? (< (first digits-with-sign) 0))
    (let digits (if negative? (map digits-with-sign abs) digits-with-sign))
    (integer num 0)
    (integer base (/ (expt 10 (length digits)) 10))
    (loop 0 (length digits) (lambda i (do 
      (+= num (* (get base) (. digits i)))
      (/= base 10)
    )))
    (*= num (if negative? -1 1))
    (get num)
    )))
(let chars->positive-or-negative-digits (lambda chars (do
    (integer current-sign 1)
    (|> chars 
        (reduce (lambda a ch (do 
            (if (= ch char:minus) 
                (set current-sign -1) 
                (do  
                    (push! a (* (get current-sign) (char->digit ch))) 
                    (set current-sign 1)))
                a)) [])))))
(let positive-or-negative-chars->integer (lambda x (|> x (chars->positive-or-negative-digits) (positive-or-negative-digits->integer))))
(let chars->integer (lambda chars (positive-or-negative-chars->integer chars)))

; (let buffer [])
; (let fn (ring-buffer buffer 5))
; (let buffer:get (get fn 0))
; (let buffer:push! (get fn 1))
; (loop 0 6 (lambda i (buffer:push! i)))
; buffer
(let ring-buffer (lambda buffer len (do 
    (integer pointer 0)
    [(lambda index (get buffer index)) 
    (lambda item (do 
        (let pt (get pointer))
        (set! buffer pt item)
        (set pointer (mod (+ len pt 1) len))
        item))])))