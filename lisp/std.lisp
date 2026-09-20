
(let std/vector/length (lambda xs (length xs)))
(let std/vector/get (lambda xs i (get xs i)))
(let get/default (lambda xs i def (if (< i (length xs)) (get xs i) def)))
(let std/vector/two-d/length std/vector/length)
(let std/vector/two-d/get get)
(let std/vector/two-d/get/default get/default)
(let std/vector/pop! (lambda xs (pop! xs)))
(let std/vector/set! (lambda xs i x (set! xs i x)))
(let std/vector/swap! (lambda xs i j (do (let temp (get xs i)) (set! xs i (get xs j)) (set! xs j temp))))
(let std/vector/push! (lambda xs x (do (set! xs (length xs) x) xs)))
(let std/vector/pop-val! pop-val!)
(let std/vector/pop-and-get! std/vector/pop-val!)
(let std/vector/push-and-get! (lambda xs x (do (set! xs (length xs) x) x)))
(let std/vector/update! (lambda xs i value (do (set! xs i value) xs)))
(let std/vector/tail! (lambda xs (do (pop! xs) xs)))
(let std/vector/append! (lambda xs x (do (std/vector/push! xs x) xs)))
(let std/vector/second (lambda xs (get xs 1)))
(let std/vector/third (lambda xs (get xs 3)))


(let std/dec/safe? (lambda value (and (>=. value const/dec/min-safe) (<=. value const/dec/max-safe))))
(let std/dec/get-safe (lambda vrbl (if (std/dec/safe? (&get vrbl)) (&get vrbl) Dec)))

(let int (lambda value (if (std/int/safe? value) [ value ] [ 0 ])))
(let dec (lambda value (if (std/dec/safe? value) [ value ] [ 0.0 ])))
(let bool (lambda value [(=? value true)]))

(let std/int/safe? (lambda value (and (>= value const/int/min-safe) (<= value const/int/max-safe))))
(let std/int/get-safe (lambda vrbl (if (std/int/safe? (&get vrbl)) (&get vrbl) Int)))

(let std/fn/combinator/i (lambda x x))
(let std/fn/combinator/k (lambda x y x))
(let std/fn/combinator/ki (lambda x y y))
(let std/fn/combinator/w (lambda f x (f x x)))
(let std/fn/combinator/b (lambda f g x (f (g x))))
(let std/fn/combinator/c (lambda f x y (f y x)))
(let std/fn/combinator/s (lambda f g x (f x (g x))))
(let std/fn/combinator/d (lambda f g x y (f x (g y))))
(let std/fn/combinator/b1 (lambda f g x y (f (g x y))))
(let std/fn/combinator/psi (lambda f g x y (f (g x) (g y))))
(let std/fn/combinator/phi (lambda f g h x (g (f x) (h x))))

(let I/comb std/fn/combinator/i)
(let K/comb std/fn/combinator/k)
(let KI/comb std/fn/combinator/ki)
(let W/comb std/fn/combinator/w)
(let B/comb std/fn/combinator/b)
(let C/comb std/fn/combinator/c)
(let S/comb std/fn/combinator/s)
(let D/comb std/fn/combinator/d)
(let B1/comb std/fn/combinator/b1)
(let PSI/comb std/fn/combinator/psi)
(let PHI/comb std/fn/combinator/phi)

(let std/fn/return 1)
(let std/fn/push 2)
(let std/fn/none 0)

(let std/fn/rec (lambda init-frame handler (do
  (let stack [init-frame])
  (let result [[]])
  (while (not (empty? stack)) (do
    (let frame (pull! stack))
    (let action (handler frame))
    ; Action grammar:
    ; { std/fn/return, [value] } return
    ; { std/fn/push, [...] } push
    ; { std/fn/none [] } none
    (cond
      (= (fst action) std/fn/return) (do (set! result 0 (snd action)) nil)
      (= (fst action) std/fn/push) (do
        (let values (snd action))
        (let len (length values))
        (mut i 0)
        (while (< i len) (do
          (push! stack (get values i))
          (alter! i (+ i 1))))
        nil)
      nil
    )))
  (get result))))


(let Rec/return std/fn/return)
(let Rec/push std/fn/push)
(let Rec/none std/fn/none)
(let Rec std/fn/rec)

(let std/vector/empty! (lambda xs (if (empty? xs) xs (do
     (while (not (empty? xs)) (pop! xs))
     xs))))
(let std/vector/overwrite! (lambda (xs ys) (std/vector/empty! xs) (loop i (< i (length ys)) (set! xs i (get ys i)))))





















(let std/vector/three-d/int/range (lambda s w h (do
  (mut i s)
  (let matrix [])
  (mut j 0)
  (while (< j w) (do
    (mut k 0)
    (let current [])
    (push! matrix current)
    (while (< k h) (do
        (push! current i)
        (alter! i (+ i 1))
        (alter! k (+ k 1))))
    (alter! j (+ j 1))))
    matrix)))

(let std/vector/two-d/int/range range)
(let std/vector/two-d/dec/range range/dec)

(let std/vector/char/blanks (lambda n (do
    (let out [ '' ])
    (mut i 1)
    (while (< i n) (do
        (set! out (length out) '')
        (alter! i (+ i 1))))
    out)))

(let std/vector/int/all-equal? (lambda xs (do (let x (get xs 0)) (every? (lambda y (= y x)) xs))))
(let std/vector/dec/all-equal? (lambda xs (do (let x (get xs 0)) (every? (lambda y (=. y x)) xs))))
(let std/vector/char/all-equal? (lambda xs (do (let x (get xs 0)) (every? (lambda y (=# y x)) xs))))
(let std/vector/bool/all-equal? (lambda xs (do (let x (get xs 0)) (every? (lambda y (=? y x)) xs))))

(let all-equal/int? std/vector/int/all-equal?)
(let all-equal/dec? std/vector/dec/all-equal?)
(let all-equal/char? std/vector/char/all-equal?)
(let all-equal/bool? std/vector/bool/all-equal?)


(let std/vector/two-d/count-of count)

(let std/vector/three-d/count-of (lambda xs fn? (do
    (mut total 0)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (alter! total (+ total (std/vector/two-d/count-of (get xs i) fn?)))
      (alter! i (+ i 1))))
    total)))
(let std/vector/three-d/int/count (lambda xs x (do
    (mut total 0)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (alter! total (+ total (count/int x (get xs i))))
      (alter! i (+ i 1))))
    total)))
(let std/vector/three-d/char/count (lambda xs x (do
    (mut total 0)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (alter! total (+ total (count/char x (get xs i))))
      (alter! i (+ i 1))))
    total)))
(let std/vector/three-d/bool/count (lambda xs x (do
    (mut total 0)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (alter! total (+ total (count/bool x (get xs i))))
      (alter! i (+ i 1))))
    total)))

(let std/vector/cons
  (lambda a b
    (do
      (let lena (length a))
      (let lenb (length b))
      (let out [])

      (mut i 0)
      (while (< i lena) (do
        (set! out (length out) (get a i))
        (alter! i (+ i 1))))

      (mut j 0)
      (while (< j lenb) (do
        (set! out (length out) (get b j))
        (alter! j (+ j 1))))

      out)))

(let std/vector/cons! (lambda a b (if (and (empty? a) (empty? b)) a (do
  (mut i 0)
  (let lenb (length b))
  (while (< i lenb) (do
    (set! a (length a) (get b i))
    (alter! i (+ i 1))))
  a))))

(let std/vector/concat (lambda xs (do
    (let out [])
    (let len-xs (length xs))
    (mut i 0)
    (while (< i len-xs) (do
      (let current (get xs i))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! out (length out) (get current j))
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    out)))
(let std/vector/concat! (lambda xs os (do
    (let len-os (length os))
    (mut i 0)
    (while (< i len-os) (do
      (let current (get os i))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! xs (length xs) (get current j))
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    xs)))







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

(let std/int/floor/div (lambda a b (/ a b)))
(let std/int/ceil/div (lambda a b (/ (+ a b -1) b)))




(let std/int/mul (lambda a b (* a b)))
(let std/int/add (lambda a b (+ a b)))
(let std/int/div (lambda a b (/ a b)))
(let std/int/sub (lambda a b (- a b)))
(let std/int/euclidean-mod (lambda a b (% (+ (% a b) b) b)))
(let std/int/euclidean-distance (lambda x1 y1 x2 y2 (do
  (let a (- x1 x2))
  (let b (- y1 y2))
  (sqrt (+ (* a a) (* b b))))))
(let std/int/manhattan-distance (lambda x1 y1 x2 y2 (+ (abs (- x2 x1)) (abs (- y2 y1)))))
(let std/int/chebyshev-distance (lambda x1 y1 x2 y2 (max (abs (- x2 x1)) (abs (- y2 y1)))))
(let std/int/normalize (lambda value min max (* (- value min) (/ (- max min)))))
(let std/int/linear-interpolation (lambda a b n (+ (* (- 1 n) a) (* n b))))
(let std/int/gauss-sum (lambda n (/ (* n (+ n 1)) 2)))
(let std/int/gauss-sum-sequance (lambda a b (/ (* (+ a b) (+ (- b a) 1)) 2)))
(let std/int/between? (lambda v min max (and (> v min) (< v max))))
(let std/int/overlap? (lambda v min max (and (>= v min) (<= v max))))

(let std/dec/between? (lambda v min max (and (>. v min) (<. v max))))
(let std/dec/overlap? (lambda v min max (and (>=. v min) (<=. v max))))

; a helper for infix ^ power
; has to be data first
(let iexpt (lambda base exp (expt exp base)))



; Structural fixed decimal: { negative? whole fraction } with fraction in [0, scale).
; This is useful when you want decimal-like values without depending on Que's Dec scale.
(let std/struct-dec/scale 1000)
(let std/struct-dec/zero { false 0 0 })
(let std/struct-dec/one { false 1 0 })
(let std/struct-dec/pi { false 3 142 })
(let std/struct-dec/e { false 2 718 })
(let std/struct-dec/negative? (lambda n (if (fst n) true false)))
(let std/struct-dec/whole (lambda n (fst (snd n))))
(let std/struct-dec/fraction (lambda n (snd (snd n))))
(let std/struct-dec/normalize (lambda n (do
  (let neg (fst n))
  (let whole (abs (fst (snd n))))
  (let frac-raw (abs (snd (snd n))))
  (let carry (/ frac-raw std/struct-dec/scale))
  (let frac (% frac-raw std/struct-dec/scale))
  (let normalized-whole (+ whole carry))
  (if (and (= normalized-whole 0) (= frac 0))
      std/struct-dec/zero
      { neg normalized-whole frac }))))
(let std/struct-dec/new (lambda neg whole fraction
  (std/struct-dec/normalize { neg whole fraction })))
(let std/struct-dec/from-int (lambda n
  (std/struct-dec/new (< n 0) (abs n) 0)))
(let std/struct-dec/scaled (lambda n
  (do
    (let mag (+ (* (std/struct-dec/whole n) std/struct-dec/scale)
                (std/struct-dec/fraction n)))
    (if (std/struct-dec/negative? n) (- mag) mag))))
(let std/struct-dec/from-scaled (lambda n
  (do
    (let mag (abs n))
    (std/struct-dec/new (< n 0)
                        (/ mag std/struct-dec/scale)
                        (% mag std/struct-dec/scale)))))
(let std/struct-dec/negate (lambda n
  (if (= (std/struct-dec/scaled n) 0)
      std/struct-dec/zero
      { (not (std/struct-dec/negative? n))
        (std/struct-dec/whole n)
        (std/struct-dec/fraction n) })))
(let std/struct-dec/abs (lambda n
  { false (std/struct-dec/whole n) (std/struct-dec/fraction n) }))
(let std/struct-dec/add (lambda a b
  (std/struct-dec/from-scaled (+ (std/struct-dec/scaled a)
                                 (std/struct-dec/scaled b)))))
(let std/struct-dec/sub (lambda a b
  (std/struct-dec/from-scaled (- (std/struct-dec/scaled a)
                                 (std/struct-dec/scaled b)))))
(let std/struct-dec/mul (lambda a b
  (std/struct-dec/from-scaled (/ (* (std/struct-dec/scaled a)
                                    (std/struct-dec/scaled b))
                                 std/struct-dec/scale))))
(let std/struct-dec/div (lambda a b
  (std/struct-dec/from-scaled (/ (* (std/struct-dec/scaled a)
                                    std/struct-dec/scale)
                                 (std/struct-dec/scaled b)))))
(let std/struct-dec/equal? (lambda a b (= (std/struct-dec/scaled a) (std/struct-dec/scaled b))))
(let std/struct-dec/lt? (lambda a b (< (std/struct-dec/scaled a) (std/struct-dec/scaled b))))
(let std/struct-dec/lte? (lambda a b (<= (std/struct-dec/scaled a) (std/struct-dec/scaled b))))
(let std/struct-dec/gt? (lambda a b (> (std/struct-dec/scaled a) (std/struct-dec/scaled b))))
(let std/struct-dec/gte? (lambda a b (>= (std/struct-dec/scaled a) (std/struct-dec/scaled b))))
(let std/struct-dec/to-dec (lambda n
  (/. (Int->Dec (std/struct-dec/scaled n)) (Int->Dec std/struct-dec/scale))))


(let std/vector/zipper (lambda a b (do
      (mut i 1)
      (let len (length a))
      (let out [[(get a 0) (get b 0)]])
      (while (< i len) (do (set! out (length out) [(get a i) (get b i)]) (alter! i (+ i 1))))
      out)))

(let std/vector/zip (lambda xs (std/vector/zipper (first xs) (std/vector/second xs))))
(let std/vector/unzip (lambda xs (do
    (let left [])
    (let right [])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (let pair (get xs i))
      (set! left (length left) (first pair))
      (set! right (length right) (std/vector/second pair))
      (alter! i (+ i 1))))
    [ left right ])))

(let std/vector/tuple/zipper (lambda a b (do
      (mut i 1)
      (let len (length a))
      (let out [{ (get a 0) (get b 0) }])
      (while (< i len) (do (set! out (length out) { (get a i) (get b i) }) (alter! i (+ i 1))))
      out)))


(let std/vector/rest (lambda xs start (if (empty? xs) xs (do
     (let end (length xs))
     (let bounds (- end start))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs (+ start i))) (alter! i (+ i 1))))
     out))))









(let std/vector/reverse! (lambda xs (do
  (let len (length xs))
  (let half (/ len 2))
  (mut i 0)
  (while (< i half) (do
    (std/vector/swap! xs i (- len i 1))
    (alter! i (+ i 1))))
  xs)))




(let std/vector/char/greater? (lambda a b (do
    (let len-a (length a))
    (let len-b (length b))
    (let min-len (if (< len-a len-b) len-a len-b))
    (mut i 0)
    (mut decided false)
    (mut out false)
    (while (and (not decided) (< i min-len)) (do
        (let ca (get a i))
        (let cb (get b i))
        (if (=# ca cb)
            (alter! i (+ i 1))
            (do
                (alter! out (># ca cb))
                (alter! decided true)))))
    (if decided out (> len-a len-b)))))

(let std/vector/char/lesser? (lambda a b (do
    (let len-a (length a))
    (let len-b (length b))
    (let min-len (if (< len-a len-b) len-a len-b))
    (mut i 0)
    (mut decided false)
    (mut out false)
    (while (and (not decided) (< i min-len)) (do
        (let ca (get a i))
        (let cb (get b i))
        (if (=# ca cb)
            (alter! i (+ i 1))
            (do
                (alter! out (<# ca cb))
                (alter! decided true)))))
    (if decided out (< len-a len-b)))))

(let std/vector/char/match? match?)
(let std/vector/char/greater-or-equal? (lambda A B (or (match? A B) (std/vector/char/greater? A B))))
(let std/vector/char/lesser-or-equal? (lambda A B (or (match? A B) (std/vector/char/lesser? A B))))
(let std/vector/char/negative? (lambda str (=# (first str) '-')))


(let std/vector/sort-partition! (lambda arr start end fn (do
     (let pivot (get arr end))
     (mut i (- start 1))
     (mut j start)

     (while (< j end) (do
           (if (fn (get arr j) pivot) (do
          (alter! i (+ i 1))
          (std/vector/swap! arr i j)
          nil))
          (alter! j (+ j 1))))

     (std/vector/swap! arr (+ i 1) end)
     (+ i 1))))

(let std/vector/sort! (lambda arr fn (do
     (let stack [])
     (push! stack 0)
     (push! stack (- (length arr) 1))
     (while (> (length stack) 0) (do
           (let end (get stack (- (length stack) 1)))
           (pop! stack)
           (let start (get stack (- (length stack) 1)))
           (pop! stack)
           (if (< start end) (do
                 (let pivot-index (std/vector/sort-partition! arr start end fn))
                 (push! stack start)
                 (push! stack (- pivot-index 1))
                 (push! stack (+ pivot-index 1))
                 (push! stack end)
                 nil))))
     arr)))

(let std/vector/safe-sort! (lambda v fn
  (do
    (let init-frame {{0 (- (length v) 1)} v})
    (let handler (lambda { { low high } vec }
      (if (>= low high) {std/fn/none []}
            (do
              (let pivot (get vec high))
              (&mut i low)
              (&mut j low)
              (while (< (&get j) high) (do
                  (if (fn (get vec (&get j)) pivot)
                      (do (std/vector/swap! vec (&get i) (&get j)) (&alter! i (+ (&get i) 1)))
                      nil)
                  (&alter! j (+ (&get j) 1))))
              (std/vector/swap! vec (&get i) high)
              (let p (&get i))
              {std/fn/push [{{ low (- p 1) } vec} {{ (+ p 1) high } vec}]}))))
    (std/fn/rec init-frame handler)
    v)))


(let std/vector/flat/length (lambda matrix (length (flat matrix))))

(let std/convert/char->digit (lambda digit (if (<# digit '0') 0 (- (as digit Int) (as '0' Int)))))
(let std/convert/chars->digits (lambda digits (do
    (let out [])
    (let len (length digits))
    (mut i 0)
    (while (< i len) (do
      (set! out (length out) (std/convert/char->digit (get digits i)))
      (alter! i (+ i 1))))
    out)))
(let std/convert/digit->char (lambda digit (if (< digit 0) '0' (+# (as digit Char) '0'))))
(let std/convert/digits->chars (lambda digits (do
    (let out [])
    (let len (length digits))
    (mut i 0)
    (while (< i len) (do
      (set! out (length out) (std/convert/digit->char (get digits i)))
      (alter! i (+ i 1))))
    out)))
(let std/convert/bool->int (lambda x (if (=? x true) 1 0)))
(let std/convert/int->bool (lambda x (if (= x 0) false true)))
(let std/convert/vector->string (lambda xs delim (do
    (let out [])
    (let len-xs (length xs))
    (mut i 0)
    (while (< i len-xs) (do
      (if (> i 0)
          (set! out (length out) delim))
      (let current (get xs i))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! out (length out) (get current j))
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    out)))
(let std/convert/string->vector (lambda str ch (do
    (let out [[]])
    (let len (length str))
    (mut i 0)
    (while (< i len) (do
      (let current (get str i))
      (if (=# current ch)
          (set! out (length out) [])
          (do
            (let prev (get out (- (length out) 1)))
            (set! prev (length prev) current)))
      (alter! i (+ i 1))))
    out)))

(let std/convert/positive-or-negative-digits->integer (lambda digits-with-sign (do
    (let len (length digits-with-sign))
    (let negative? (and (> len 0) (< (get digits-with-sign 0) 0)))
    (mut num 0)
    (mut i 0)
    (while (< i len) (do
      (let digit (get digits-with-sign i))
      (alter! num (+ (* num 10) (if negative? (abs digit) digit)))
      (alter! i (+ i 1))))
    (if negative? (- 0 num) num))))

(let std/convert/chars->positive-or-negative-digits (lambda chars (do
    (mut current-sign 1)
    (let out [])
    (let len (length chars))
    (mut i 0)
    (while (< i len) (do
      (let ch (get chars i))
      (if (=# ch '-')
          (alter! current-sign -1)
          (do
            (std/vector/push! out (* current-sign (std/convert/char->digit ch)))
            (alter! current-sign 1)))
      (alter! i (+ i 1))))
    out)))
(let std/convert/digits->integer std/convert/positive-or-negative-digits->integer)
(let std/convert/positive-or-negative-chars->integer (lambda chars (do
    (let len (length chars))
    (mut sign 1)
    (mut i 0)
    (if (and (> len 0) (=# (get chars 0) '-'))
        (do
          (alter! sign -1)
          (alter! i 1)))
    (mut num 0)
    (while (< i len) (do
      (alter! num (+ (* num 10) (std/convert/char->digit (get chars i))))
      (alter! i (+ i 1))))
    (* num sign))))
(let std/convert/chars->integer std/convert/positive-or-negative-chars->integer)

(let std/convert/chars->digits/dec (lambda xs (do
    (let out [[]])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (let ch (get xs i))
      (if (=# ch '.')
          (push! out [])
          (push! (at out -1) (std/convert/char->digit ch)))
      (alter! i (+ i 1))))
    out)))

(let std/convert/chars->ufloat (lambda xs (do
  (let parts (std/convert/chars->digits/dec xs))
  (let pow (expt (length (get parts 1)) 10))
  (/. (Int->Dec (+
    (* (std/convert/digits->integer (get parts 0)) pow)
    (std/convert/digits->integer (get parts 1)))) (Int->Dec pow)))))

(let std/convert/chars->dec (lambda xs
  (if (=# (get xs 0) '-') (*. (std/convert/chars->ufloat (slice 1 (length xs) xs)) -1.0) (std/convert/chars->ufloat xs))))

(let std/convert/int->char/alphabet
  (lambda x offset (Int->Char (+ x (Char->Int offset)))))

(let std/vector/unique-pairs (lambda xs (do
    (let pairs [])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
        (mut j (+ i 1))
        (while (< j len) (do
            (std/vector/push! pairs [(get xs i) (get xs j)])
            (alter! j (+ j 1))))
        (alter! i (+ i 1))))
    pairs)))





(let std/vector/three-d/dimensions (lambda matrix [ (length matrix) (length (get matrix 0)) ]))
(let std/vector/three-d/in-bounds? (lambda matrix y x (and (in-bounds? matrix y) (in-bounds? (get matrix y) x))))
(let std/vector/three-d/set! (lambda matrix y x value (do (set! (get matrix y) x value) 0)))


(let std/vector/three-d/sliding-adjacent-sum (lambda xs directions y x N fn
    (do
      (mut total 0)
      (let len (length directions))
      (mut i 0)
      (while (< i len) (do
        (let dir (get directions i))
        (let dy (+ (first dir) y))
        (let dx (+ (std/vector/second dir) x))
        (alter! total (fn total (get xs (std/int/euclidean-mod dy N) (std/int/euclidean-mod dx N))))
        (alter! i (+ i 1))))
      total)))


(let std/node/parent (lambda i (- (>> (+ i 1) 1) 1)))
(let std/node/left (lambda i (+ (<< i 1) 1)))
(let std/node/right (lambda i (<< (+ i 1) 1)))

(let std/heap/top 0)
(let std/heap/greater? (lambda heap i j fn? (=? (fn? (get heap i) (get heap j)) true)))
(let std/heap/sift-up! (lambda heap fn (do
  (&mut node (- (length heap) 1))
  (letrec tail-call/std/heap/sift-up! (lambda heap
    (if (and (> (&get node) std/heap/top) (std/heap/greater? heap (&get node) (std/node/parent (&get node)) fn))
      (do
        (std/vector/swap! heap (&get node) (std/node/parent (&get node)))
        (&alter! node (std/node/parent (&get node)))
        (tail-call/std/heap/sift-up! heap)) heap)))
  (tail-call/std/heap/sift-up! heap))))

(let std/heap/sift-down! (lambda heap fn (do
  (&mut node std/heap/top)
  (letrec tail-call/std/heap/sift-down! (lambda heap
    (if (or
          (and
            (< (std/node/left (&get node)) (length heap))
            (std/heap/greater? heap (std/node/left (&get node)) (&get node) fn))
          (and
            (< (std/node/right (&get node)) (length heap))
            (std/heap/greater? heap (std/node/right (&get node)) (&get node) fn)))
      (do
        (let max-child (if (and
                            (< (std/node/right (&get node)) (length heap))
                            (std/heap/greater? heap (std/node/right (&get node)) (std/node/left (&get node)) fn))
                            (std/node/right (&get node))
                            (std/node/left (&get node))))
        (std/vector/swap!  heap (&get node) max-child)
        (&alter! node max-child)
        (tail-call/std/heap/sift-down! heap)) heap)))
  (tail-call/std/heap/sift-down! heap))))

(let std/heap/peek (lambda heap (get heap std/heap/top)))

(let std/heap/push! (lambda heap value fn (do
    (set! heap (length heap) value)
    (std/heap/sift-up! heap fn)
    nil)))

(let std/heap/pop! (lambda heap fn (do
  (let bottom (- (length heap) 1))
  (if (> bottom std/heap/top) (do (std/vector/swap! heap std/heap/top bottom) heap) heap)
  (pop! heap)
  (std/heap/sift-down! heap fn)
  nil)))

(let std/heap/replace! (lambda heap value fn (do
(set! heap std/heap/top value)
(std/heap/sift-down! heap fn)
heap)))


(let std/heap/empty? empty?)
(let std/heap/not-empty? not-empty?)
(let std/heap/empty! std/vector/empty!)

(let std/convert/vector->heap (lambda xs fn (do
    (let heap [])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (std/heap/push! heap (get xs i) fn)
      (alter! i (+ i 1))))
    heap)))
(let std/convert/set->vector (lambda xs (filter not-empty? (flat xs))))

(let std/convert/integer->string-base (lambda num base
    (if (= num 0) "0" (do
        (let neg? (< num 0))
        (mut n (if neg? (* num -1) num))
        (let str [])
        (while (> n 0) (do
            (let x (% n base))
            (std/vector/push! str (+# (Int->Char x) (char 48)))
            (alter! n (/ n base))))
        (if neg? (do (std/vector/push! str '-') nil))
        (mut left 0)
        (mut right (- (length str) 1))
        (while (< left right) (do
            (let ch (get str left))
            (set! str left (get str right))
            (set! str right ch)
            (alter! left (+ left 1))
            (alter! right (- right 1))))
        str))))
(let std/convert/integer->string (lambda x (std/convert/integer->string-base x 10)))
(let std/convert/vector->set (lambda xs (do
    (let s [ [] [] [] [] [] [] [] ])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (std/vector/hash/set/add! s (get xs i))
      (alter! i (+ i 1))))
    s)))

(let std/integer/dec-scaling 1000000)
(let std/dec/dec-scaling 1000000.0)

(let std/convert/dec->string (lambda x (if (=. (floor x) x) (cons (std/convert/integer->string (Dec->Int x)) ".0") (do
    (let flip (if (<. x 0.0) -1.0 1.0))
    (let exponent (floor x))
    (let mantisa (-. x exponent))
    (let left (std/convert/integer->string (Dec->Int exponent)))
    (let right (std/convert/integer->string (Dec->Int (*. mantisa std/dec/dec-scaling flip))))
    (let len (length right))
    (letrec tail-call/while! (lambda i
        (if (=# (get right (- len i)) '0') (do
            (pop! right)
            (tail-call/while! (+ i 1)))
        i)))
    (tail-call/while! 1)
    (cons left ['.'] right)))))

; Experimental still
(let std/vector/deque/new (lambda def [[ def ] []]))
(let std/vector/deque/offset-left (lambda q (* (- (length (get q 0)) 1) -1)))
(let std/vector/deque/offset-right (lambda q (length (get q 1))))
(let std/vector/deque/length (lambda q (+ (length (get q 0)) (length (get q 1)) -1)))
(let std/vector/deque/empty? (lambda q (= (std/vector/deque/length q) 0)))
(let std/vector/deque/empty! (lambda q (do
    (set! q 0 [(get q 0 0)])
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
(let std/vector/deque/remove-from-left! (lambda q (do
  (let len (std/vector/deque/length q))
  (if (> len 0)
     (cond
        (= len 1) (std/vector/deque/empty! q)
        (> (length (get q 0)) 0) (do (pop! (get q 0)) q)
        q) q))))
(let std/vector/deque/remove-from-right! (lambda q (do
    (let len (std/vector/deque/length q))
    (if (> len 0)
     (cond
        (= len 1) (std/vector/deque/empty! q)
        (> (length (get q 1)) 0) (do (pop! (get q 1)) q)
        q) q))))
(let std/vector/deque/iter (lambda q fn (do
  (letrec tail-call/std/vector/deque/iter (lambda index bounds (do
      (fn (std/vector/deque/get q index))
      (if (< index bounds) (tail-call/std/vector/deque/iter (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/iter 0 (std/vector/deque/length q)))))
(let std/vector/deque/map (lambda q fn (do
  (let result (std/vector/deque/new))
  (let len (std/vector/deque/length q))
  (let half (/ len 2))
  (letrec tail-call/left/std/vector/deque/map (lambda index (do
    (std/vector/deque/add-to-left! result (fn (std/vector/deque/get q index)))
   (if (> index 0) (tail-call/left/std/vector/deque/map (- index 1)) Int))))
 (tail-call/left/std/vector/deque/map (- half 1))
(letrec tail-call/right/std/vector/deque/map (lambda index bounds (do
   (std/vector/deque/add-to-right! result (fn (std/vector/deque/get q index)))
   (if (< index bounds) (tail-call/right/std/vector/deque/map (+ index 1) bounds) Int))))
 (tail-call/right/std/vector/deque/map half (- len 1))
 result)))
(let std/vector/deque/balance? (lambda q (= (+ (std/vector/deque/offset-right q) (std/vector/deque/offset-left q)) 0)))
(let std/convert/vector->deque (lambda initial (do
 (let q (std/vector/deque/new))
 (let half (/ (length initial) 2))
 (mut left (- half 1))
 (while (>= left 0) (do
    (std/vector/deque/add-to-left! q (get initial left))
    (alter! left (- left 1))))
 (mut right half)
 (let len (length initial))
 (while (< right len) (do
   (std/vector/deque/add-to-right! q (get initial right))
   (alter! right (+ right 1))))
    q)))
(let std/convert/deque->vector (lambda q (if (std/vector/deque/empty? q) [(get q 0 0)] (do
  (let out [])
  (mut index 0)
  (let len (std/vector/deque/length q))
  (while (< index len) (do
      (set! out (length out) (std/vector/deque/get q index))
      (alter! index (+ index 1))))
    out))))
(let std/vector/deque/balance! (lambda q
    (if (std/vector/deque/balance? q) q (do
      (let initial (std/convert/deque->vector q))
      (std/vector/deque/empty! q)
      (let half (/ (length initial) 2))
      (mut right half)
      (let len (length initial))
      (while (< right len) (do
        (std/vector/deque/add-to-right! q (get initial right))
        (alter! right (+ right 1))))
      (mut left (- half 1))
      (while (>= left 0) (do
        (std/vector/deque/add-to-left! q (get initial left))
        (alter! left (- left 1))))
    q))))
(let std/vector/deque/append! (lambda q item (do (std/vector/deque/add-to-right! q item) q)))
(let std/vector/deque/prepend! (lambda q item (do (std/vector/deque/add-to-left! q item) q)))
(let std/vector/deque/head! (lambda q (do
    (if (= (std/vector/deque/offset-right q) 0) (std/vector/deque/balance! q) q)
    (std/vector/deque/remove-from-right! q)
    q)))
(let std/vector/deque/tail! (lambda q (do
    (if (= (std/vector/deque/offset-left q) 0) (std/vector/deque/balance! q) q)
    (std/vector/deque/remove-from-left! q)
q)))
(let std/vector/deque/first (lambda q (std/vector/deque/get q 0)))
(let std/vector/deque/last (lambda q (std/vector/deque/get q (- (std/vector/deque/length q) 1))))
(let std/vector/deque/pop-right! (lambda q (do
    (let last (std/vector/deque/last q))
    (std/vector/deque/head! q)
    last)))
(let std/vector/deque/pop-left! (lambda q (do
    (let f (std/vector/deque/first q))
    (std/vector/deque/tail! q)
    f)))
(let std/vector/deque/rotate-left! (lambda q n (do
  (let N (% n (std/vector/deque/length q)))
  (letrec tail-call/std/vector/deque/rotate-left! (lambda index bounds (do
      (if (= (std/vector/deque/offset-left q) 0) (std/vector/deque/balance! q) q)
      (std/vector/deque/add-to-right! q (std/vector/deque/first q))
      (std/vector/deque/remove-from-left! q)
      (if (< index bounds) (tail-call/std/vector/deque/rotate-left! (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/rotate-left! 0 N) q)))
(let std/vector/deque/rotate-right! (lambda q n (do
  (let N (% n (std/vector/deque/length q)))
  (letrec tail-call/std/vector/deque/rotate-left! (lambda index bounds (do
      (if (= (std/vector/deque/offset-right q) 0) (std/vector/deque/balance! q) q)
      (std/vector/deque/add-to-left! q (std/vector/deque/last q))
      (std/vector/deque/remove-from-right! q)
      (if (< index bounds) (tail-call/std/vector/deque/rotate-left! (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/rotate-left! 0 N) q)))
(let std/vector/deque/slice (lambda entity s e (do
  (let len (std/vector/deque/length entity))
  (let start (if (< s 0) (max (+ len s) 0) (min s len)))
  (let end (if (< e 0) (max (+ len e) 0) (min e len)))
  (let scl (std/vector/deque/new))
  (let slice-len (max (- end start) 0))
  (let half (/ slice-len 2))
  (letrec tail-call/left/std/vector/deque/slice (lambda index (do
      (std/vector/deque/add-to-left! scl (std/vector/deque/get entity (+ start index)))
      (if (> index 0) (tail-call/left/std/vector/deque/slice (- index 1)) Int))))
  (tail-call/left/std/vector/deque/slice (- half 1))
  (letrec tail-call/right/std/vector/deque/slice (lambda index bounds (do
      (std/vector/deque/add-to-right! scl (std/vector/deque/get entity (+ start index)))
      (if (< index bounds) (tail-call/right/std/vector/deque/slice (+ index 1) bounds) Int))))
  (tail-call/right/std/vector/deque/slice half (- slice-len 1))
  scl)))

(let std/vector/queue/new std/vector/deque/new)
(let std/vector/stack/new std/vector/deque/new)

(let std/vector/queue/empty? std/vector/deque/empty?)
(let std/vector/queue/not-empty? (lambda q (not (std/vector/deque/empty? q))))
(let std/vector/queue/empty! std/vector/deque/empty!)
(let std/vector/queue/enqueue! (lambda queue item (std/vector/deque/append! queue item)))
(let std/vector/queue/dequeue! (lambda queue (std/vector/deque/tail! queue)))
(let std/vector/queue/peek (lambda queue (std/vector/deque/first queue)))

(let std/vector/stack/empty? std/vector/deque/empty?)
(let std/vector/stack/not-empty? (lambda q (not (std/vector/deque/empty? q))))
(let std/vector/stack/empty! std/vector/deque/empty!)
(let std/vector/stack/push! (lambda stack item (std/vector/deque/append! stack item)))
(let std/vector/stack/pop! (lambda stack (std/vector/deque/head! stack)))
(let std/vector/stack/peek (lambda stack (std/vector/deque/last stack)))


(let std/vector/three-d/for (lambda matrix fn (do
  (let width (length (first matrix)))
  (let height (length matrix))
  (mut y 0)
  (while (< y height) (do
    (mut x 0)
    (while (< x width) (do
      (fn (get matrix y x))
      (alter! x (+ x 1))))
    (alter! y (+ y 1))))
   matrix)))

(let std/vector/three-d/for/i (lambda matrix fn (do
  (let width (length (first matrix)))
  (let height (length matrix))
  (mut y 0)
  (while (< y height) (do
    (mut x 0)
    (while (< x width) (do
      (fn (get matrix y x) y x)
      (alter! x (+ x 1))))
    (alter! y (+ y 1))))
   matrix)))


(let std/vector/concat/with (lambda xs ch (do
    (let out [])
    (let len-xs (length xs))
    (mut i 0)
    (while (< i len-xs) (do
      (if (> i 0) (set! out (length out) ch))
      (let current (get xs i))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! out (length out) (get current j))
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    out)))


(let std/vector/int/pair/sub (lambda xs (- (get xs 0) (get xs 1))))
(let std/vector/int/pair/add (lambda xs (+ (get xs 0) (get xs 1))))
(let std/vector/int/pair/mult (lambda xs (* (get xs 0) (get xs 1))))
(let std/vector/int/pair/div (lambda xs (/ (get xs 0) (get xs 1))))
(let std/vector/sort/asc! (lambda xs (std/vector/sort! xs <)))
(let std/vector/sort/desc! (lambda xs (std/vector/sort! xs >)))

(let std/vector/equal? (lambda a b fn? (do
  (if (< (length a) (length b)) false
  (if (> (length a) (length b)) false
    (do
      (mut i 0)
      (mut result true)
      (let len (length a))
      (while (< i len) (do
        (let da (get a i))
        (let db (get b i))
        (if (not (fn? da db)) (do
          (alter! result false)
          (alter! i len)))
        (alter! i (+ i 1))))
      (if result true false)))))))

 (let std/vector/char/lexicographic (lambda a b eq? gt? (do
    (mut i 0)
    (let len-a (length a))
    (let len-b (length b))
    (let len (if (< len-a len-b) len-a len-b))
    (mut out 0) ; 0 equal-so-far, 1 a>b, -1 a<b
    (while (and (< i len) (= out 0)) (do
      (let da (get a i))
      (let db (get b i))
      (if (not (eq? da db))
        (alter! out (if (gt? da db) 1 -1))
        nil)
      (alter! i (+ i 1))))
    (if (= out 0)
      (if (= len-a len-b) 0 (if (> len-a len-b) 1 -1))
      out))))

(let std/vector/compare (lambda a b step done? initial (do
    (mut state initial)
    (mut i 0)
    (let len-a (length a))
    (let len-b (length b))
    (let len (if (< len-a len-b) len-a len-b))
    (while (and (< i len) (not (done? state))) (do
      (alter! state (step state (get a i) (get b i) i))
      (alter! i (+ i 1))))
    { state len-a len-b })))

(let std/vector/int/equal? (lambda a b (do
  (if (< (length a) (length b)) false
  (if (> (length a) (length b)) false
    (do
      (mut i 0)
      (mut result true)
      (let len (length a))
      (while (< i len) (do
        (let da (get a i))
        (let db (get b i))
        (if (not (= da db)) (do
          (alter! result false)
          (alter! i len)))
        (alter! i (+ i 1))))
      (if result true false)))))))

(let std/vector/dec/equal? (lambda a b (do
  (if (< (length a) (length b)) false
  (if (> (length a) (length b)) false
    (do
      (mut i 0)
      (mut result true)
      (let len (length a))
      (while (< i len) (do
        (let da (get a i))
        (let db (get b i))
        (if (not (=. da db)) (do
          (alter! result false)
          (alter! i len)))
        (alter! i (+ i 1))))
      (if result true false)))))))

(let std/vector/bool/equal? (lambda a b (do
  (if (< (length a) (length b)) false
  (if (> (length a) (length b)) false
    (do
      (mut i 0)
      (mut result true)
      (let len (length a))
      (while (< i len) (do
        (let da (get a i))
        (let db (get b i))
        (if (not (=? da db)) (do
          (alter! result false)
          (alter! i len)))
        (alter! i (+ i 1))))
      (if result true false)))))))

(let std/convert/integer->bits (lambda num
    (if (= num 0) [ 0 ] (do
        (&mut n num)
        (letrec tail-call/while! (lambda out
            (if (> (&get n) 0) (do
                (std/vector/push! out (% (get n) 2))
                (&alter! n (/ (&get n) 2))
                (tail-call/while! out)) out)))
        (reverse (tail-call/while! []))))))



; alternative implementation using bitwise operators
; (let std/convert/bits->integer (lambda bits (std/vector/reduce bits (lambda value bit (| (<< value 1) (& bit 1))) 0)))

(let std/convert/bits->integer (lambda xs (do
  (letrec tail-call/bits->integer (lambda index out (if
                              (= index (length xs)) out
                              (tail-call/bits->integer (+ index 1) (+ out (* (at xs index) (expt (- (length xs) index 1) 2)))))))
  (tail-call/bits->integer 0 0))))


(let std/int/reduce (lambda n fn acc (do
    (letrec tail-call/fold-n (lambda i out (if (< i n) (tail-call/fold-n (+ i 1) (fn out i)) out)))
    (tail-call/fold-n 0 acc))))
(let std/vector/three-d/fill (lambda W H fn
  (cond
    (or (= W 0) (= H 0)) []
    (and (= W 1) (= H 1)) [[(fn 0 0)]] (do
      (let matrix [])
      (mut i 0)
      (while (< i W) (do
          (std/vector/push! matrix [])
          (mut j 0)
          (while (< j H) (do
            (std/vector/three-d/set! matrix i j (fn i j))
            (alter! j (+ j 1))))
          (alter! i (+ i 1))))
      matrix))))
(let std/vector/three-d/product (lambda A B (do
  (let dimsA (std/vector/three-d/dimensions A))
  (let dimsB (std/vector/three-d/dimensions B))
  (let rowsA (get dimsA 0))
  (let colsA (get dimsA 1))
  (let rowsB (get dimsB 0))
  (let colsB (get dimsB 1))
  (if (= colsA rowsB) (std/vector/three-d/fill rowsA colsB (lambda i j
      (std/int/reduce colsA (lambda sm k (+ sm (* (get A i k) (get B k j)))) 0))) []))))
(let std/vector/three-d/dot-product (lambda a b (do
  (let lenA (length a))
  (let lenB (length b))
  (if (= lenA lenB)
    (std/int/reduce lenA (lambda sm i (+ sm (* (get a i) (get b i)))) 0) Int))))






(let std/vector/int/big/range (lambda start end (do
     (let out [ (std/int/big/new (std/convert/integer->string start)) ])
     (mut i (+ start 1))
     (while (<= i end) (do
        (set! out (length out) (std/int/big/new (std/convert/integer->string i)))
        (alter! i (+ i 1))))
   out)))

(let std/int/big/add (lambda a1 b1 (do
  (let a (reverse a1))
  (let b (reverse b1))
  (let max-length (max (length a) (length b)))
  (let result [])
  (mut carry 0)
  (loop/range/exclusive i 0 max-length
    (let digit-A (if (< i (length a)) (get a i) 0))
    (let digit-B (if (< i (length b)) (get b i) 0))
    (let sm (+ digit-A digit-B carry))
    (std/vector/push! result (% sm 10))
    (alter! carry (/ sm 10)))
  ; Handle remaining carry
  (while (> carry 0) (do
    (std/vector/push! result (% carry 10))
    (alter! carry (/ carry 10))))
  (reverse result))))

(let std/int/big/sub
  (lambda a1 b1
    (do
      (let a (reverse a1))
      (let b (reverse b1))
      (let result [])
      (mut borrow 0)
      (mut j 0)
      (let max-length (max (length a) (length b)))

      (while (< j max-length) (do
        (let digit-a (if (< j (length a)) (get a j) 0))
        (let digit-b (if (< j (length b)) (get b j) 0))
        (let diff (- (- digit-a digit-b) borrow))

        (if (< diff 0)
            (do
              (push! result (+ diff 10))
              (alter! borrow 1))
            (do
              (push! result diff)
              (alter! borrow 0)))

        (alter! j (+ j 1))))

      (mut k (- (length result) 1))
      (while (and (> k 0) (= (get result k) 0)) (do
        (pop! result)
        (alter! k (- k 1))))

      (reverse result))))

(let std/int/big/mul (lambda a1 b1 (do
  (let a (reverse a1))
  (let b (reverse b1))
  (let result [])
  ; Initialize result array with zeros
  (loop/range/exclusive _ 0 (+ (length a) (length b)) (std/vector/push! result 0))
  (loop/range/exclusive i 0 (length a)
    (mut carry 0)
    (let digit-a (get a i))
    (loop/range/exclusive j 0 (length b)
      (let digit-B (get b j))
      (let idx (+ i j))
      (let prod (+ (* digit-a digit-B) (get result idx) carry))
      (set! result idx (% prod 10))
      (alter! carry (/ prod 10)))
    ; Handle carry for this digit-a
    (mut k (+ i (length b)))
    (while (> carry 0) (do
      (if (not (< k (length result))) (do (std/vector/push! result 0) nil) nil)
      (let sm (+ (get result k) carry))
      (set! result k (% sm 10))
      (alter! carry (/ sm 10))
      (alter! k (+ k 1)))))
  ; Remove trailing zeros (from the most significant end), but keep at least one digit
  (mut ii (- (length result) 1))
  (while (and (> ii 0) (= (get result ii) 0) (> (length result) 1)) (do
    (pop! result)
    (alter! ii (- ii 1))))
  (reverse result))))

(let std/vector/int/remove-leading-zeroes
  (lambda digits
    (do
      (mut i 0)
      (while (and (< i (length digits))
                  (zero? (get digits i))) (do
        (++ i)))
      (if (= i (length digits))
          [0]
          (slice i (length digits) digits)))))

(let std/int/big/less-or-equal? (lambda a b (do
  (if (< (length a) (length b)) true
  (if (> (length a) (length b)) false
    ; Equal length, compare digit by digit
    (do
      (mut i 0)
      (mut result true) ; assume a <= b
      (while (< i (length a)) (do
        (let da (get a i))
        (let db (get b i))
        (if (< da db) (do
          (alter! result true)
          (alter! i (length a))))
        (if (> da db) (do
          (alter! result false)
          (alter! i (length a))))
        (alter! i (+ i 1))))
      result))))))

(let std/int/big/greater-or-equal? (lambda a b (do
  (if (> (length a) (length b)) true
  (if (< (length a) (length b)) false
    ; Equal length, compare digit by digit
    (do
      (mut i 0)
      (mut result true) ; assume a >= b
      (while (< i (length a)) (do
        (let da (get a i))
        (let db (get b i))
        (if (> da db) (do
          (alter! result true)
          (alter! i (length a))))
        (if (< da db) (do
          (alter! result false)
          (alter! i (length a))))
        (alter! i (+ i 1))))
      result))))))

(let std/int/big/less-than? (lambda a b (do
  (if (< (length a) (length b)) true
  (if (> (length a) (length b)) false
    ; Equal length, check for strict less (not equal)
    (do
      (mut i 0)
      (mut found-less false) ; true if a < b at some digit
      (while (< i (length a)) (do
        (let da (get a i))
        (let db (get b i))
        (if (< da db) (do
          (alter! found-less true)
          (alter! i (length a))))
        (if (> da db) (do
          (alter! i (length a)))) ; stop on a > b, keep found-less false
        (alter! i (+ i 1))))
      found-less))))))

(let std/int/big/greater-than? (lambda a b (do
  (if (> (length a) (length b)) true
  (if (< (length a) (length b)) false
    ; Equal length, check for strict greater (not equal)
    (do
      (mut i 0)
      (mut found-greater false) ; true if a > b at some digit
      (while (< i (length a)) (do
        (let da (get a i))
        (let db (get b i))
        (if (> da db) (do
          (alter! found-greater true)
          (alter! i (length a))))
        (if (< da db) (do
          (alter! i (length a)))) ; stop on a < b, keep found-greater false
        (alter! i (+ i 1))))
      found-greater))))))

(let std/int/big/equal? std/vector/int/equal?)


(let std/int/big/div
  (lambda dividend divisor
    (do
      (let result [])
      (&mut current [])
      (let len (length dividend))
      (mut i 0)

      (while (< i len) (do
        (let digit (get dividend i))

        ;; current = trim(cons(current, [digit]))
        (let cur0 (&get current))
        (let cur1 (cons cur0 [digit]))
        (let cur2 (std/vector/int/remove-leading-zeroes cur1))
        (&alter! current cur2)

        ;; binary search q in [0,9]
        (mut low 0)
        (mut high 9)
        (mut q 0)

        (while (<= low high) (do
          (let mid (/ (+ low high) 2))
          (let prod (std/int/big/mul divisor [mid]))
          (let cur (&get current))

          (if (std/int/big/less-or-equal? prod cur)
              (do
                (alter! q mid)
                (alter! low (+ mid 1)))
              (alter! high (- mid 1)))))

        (std/vector/push! result q)

        ;; current = current - divisor*q
        (let prod2 (std/int/big/mul divisor [q]))
        (let cur3 (&get current))
        (let cur4 (std/int/big/sub cur3 prod2))
        (&alter! current cur4)

        (alter! i (+ i 1))))

      (let out (std/vector/int/remove-leading-zeroes result))
      (if (empty? out) [0] out))))

(let std/int/big/mod
  (lambda a b
    (do
      (let q (std/int/big/div a b))
      (let p (std/int/big/mul b q))
      (let r (std/int/big/sub a p))
      r)))
(let std/int/big/square (lambda x (std/int/big/mul x x)))
(let std/int/big/floor/div (lambda a b (std/int/big/div a b)))
(let std/int/big/ceil/div (lambda a b (std/int/big/div
    (std/int/big/sub (std/int/big/add a b) [ 1 ]) b)))
(let std/vector/int/big/sum (lambda xs (do
    (&mut total [ 0 ])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (&alter! total (std/int/big/add (&get total) (get xs i)))
      (alter! i (+ i 1))))
    (&get total))))
(let std/vector/int/big/product (lambda xs (do
    (&mut total [ 1 ])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (&alter! total (std/int/big/mul (&get total) (get xs i)))
      (alter! i (+ i 1))))
    (&get total))))
(let std/int/big/new (lambda str (std/convert/chars->digits str)))
(let std/int/pow/big (lambda n pow (do
  ; Initialize digits array with the first digit
  (let digits [ n ])
  (mut p 1) ; Use numeric variable for p
  (mut carry 0) ; Use numeric variable for carry
  ; Loop to calculate n^pow
  (while (< p pow) (do
    (alter! carry 0) ; Reset carry to 0
    (mut exp 0)
    (let len-digits (length digits))
    (while (< exp len-digits) (do
      (let prod (+ (* (get digits exp) n) carry))
      (let new-carry (/ prod 10))
      (set! digits exp (% prod 10))
      ; Update carry using variable helper
      (alter! carry new-carry)
      (alter! exp (+ exp 1))))
    ; Handle carry
    (while (> carry 0) (do
      (std/vector/push! digits (% carry 10))
      ; Update carry using variable helper
      (alter! carry (/ carry 10))))
    ; Increment p using variable helper
    (alter! p (+ p 1))))
  (reverse digits))))

(let std/int/big/pow (lambda a b (if (= b 0) [ 1 ] (do
    (&mut out a)
    (mut i 0)
    (while (< i (- b 1)) (do (&alter! out (std/int/big/mul (&get out) a)) (alter! i (+ i 1))))
    (&get out)))))

(let std/int/big/expt (lambda a b (if (and (= (length b) 1) (= (get b 0) 0)) [ 1 ] (do
    (&mut out a)
    (&mut exp (std/int/big/sub b [ 1 ]))
    (while (not (and (= (length (&get exp)) 1) (= (get (&get exp) 0) 0))) (do
      (&alter! out (std/int/big/mul (&get out) a))
      (&alter! exp (std/int/big/sub (&get exp) [ 1 ]))))
    (&get out)))))

(let std/int/big/signed/zero { false [0] })
(let std/int/big/signed/one { false [1] })
(let std/int/big/signed/negative? (lambda n (if (fst n) true false)))
(let std/int/big/signed/digits snd)
(let std/int/big/signed/abs snd)
(let std/int/big/signed/zero? (lambda n (std/int/big/equal? (snd n) [0])))
(let std/int/big/signed/normalize (lambda { neg digits } (do
  (let mag (std/vector/int/remove-leading-zeroes digits))
  (if (or (empty? mag) (std/int/big/equal? mag [0]))
      std/int/big/signed/zero
      { neg mag }))))
(let std/int/big/signed/new (lambda str
  (if (and (> (length str) 0) (=# (get str 0) '-'))
      (std/int/big/signed/normalize { true (std/int/big/new (slice 1 (length str) str)) })
      (std/int/big/signed/normalize { false (std/int/big/new str) }))))
(let std/int/big/signed/negate (lambda { neg digits }
  (if (std/int/big/equal? digits [0])
      std/int/big/signed/zero
      { (not neg) digits })))
(let std/int/big/signed/equal? (lambda a b
  (and (=? (fst a) (fst b)) (std/int/big/equal? (snd a) (snd b)))))
(let std/int/big/signed/lt? (lambda a b
  (if (fst a)
      (if (fst b)
          (std/int/big/greater-than? (snd a) (snd b))
          true)
      (if (fst b)
          false
          (std/int/big/less-than? (snd a) (snd b))))))
(let std/int/big/signed/lte? (lambda a b
  (or (std/int/big/signed/equal? a b) (std/int/big/signed/lt? a b))))
(let std/int/big/signed/gt? (lambda a b
  (not (std/int/big/signed/lte? a b))))
(let std/int/big/signed/gte? (lambda a b
  (not (std/int/big/signed/lt? a b))))
(let std/int/big/signed/add (lambda a b
  (if (=? (fst a) (fst b))
      (std/int/big/signed/normalize { (fst a) (std/int/big/add (snd a) (snd b)) })
      (if (std/int/big/greater-than? (snd a) (snd b))
          (std/int/big/signed/normalize { (fst a) (std/int/big/sub (snd a) (snd b)) })
          (if (std/int/big/less-than? (snd a) (snd b))
              (std/int/big/signed/normalize { (fst b) (std/int/big/sub (snd b) (snd a)) })
              std/int/big/signed/zero)))))
(let std/int/big/signed/sub (lambda a b
  (std/int/big/signed/add a (std/int/big/signed/negate b))))
(let std/int/big/signed/mul (lambda a b
  (std/int/big/signed/normalize { (not (=? (fst a) (fst b))) (std/int/big/mul (snd a) (snd b)) })))
(let std/int/big/signed/square (lambda x (std/int/big/signed/mul x x)))
(let std/int/big/signed/pow (lambda a b
  (if (= b 0)
      std/int/big/signed/one
      (std/int/big/signed/normalize {
        (and (fst a) (= (% b 2) 1))
        (std/int/big/pow (snd a) b)
      }))))
(let std/int/big/signed/expt (lambda a b
  (if (std/int/big/equal? b [0])
      std/int/big/signed/one
      (std/int/big/signed/normalize {
        (and (fst a) (std/int/big/equal? (std/int/big/mod b [2]) [1]))
        (std/int/big/expt (snd a) b)
      }))))
(let std/int/big/signed/div (lambda a b (do
  (let q (std/int/big/div (snd a) (snd b)))
  (std/int/big/signed/normalize { (not (=? (fst a) (fst b))) q }))))
(let std/int/big/signed/mod (lambda a b
  (std/int/big/signed/sub a (std/int/big/signed/mul b (std/int/big/signed/div a b)))))
(let std/int/big/signed/floor/div (lambda a b (do
  (let q (std/int/big/signed/div a b))
  (let r (std/int/big/signed/mod a b))
  (if (and (not (std/int/big/signed/zero? r)) (not (=? (fst a) (fst b))))
      (std/int/big/signed/sub q std/int/big/signed/one)
      q))))
(let std/int/big/signed/ceil/div (lambda a b (do
  (let q (std/int/big/signed/div a b))
  (let r (std/int/big/signed/mod a b))
  (if (and (not (std/int/big/signed/zero? r)) (=? (fst a) (fst b)))
      (std/int/big/signed/add q std/int/big/signed/one)
      q))))
(let std/int/big/signed/to-string (lambda n
  (if (fst n)
      (cons "-" (std/convert/digits->chars (snd n)))
      (std/convert/digits->chars (snd n)))))
(let std/vector/int/big/signed/range (lambda start end
  (if (std/int/big/signed/gt? start end)
      []
      (do
        (let out [start])
        (&mut current start)
        (while (std/int/big/signed/lt? (&get current) end) (do
          (&alter! current (std/int/big/signed/add (&get current) std/int/big/signed/one))
          (set! out (length out) (&get current))))
        out))))
(let std/vector/int/big/signed/sum (lambda xs (do
    (&mut total std/int/big/signed/zero)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (&alter! total (std/int/big/signed/add (&get total) (get xs i)))
      (alter! i (+ i 1))))
    (&get total))))
(let std/vector/int/big/signed/product (lambda xs (do
    (&mut total std/int/big/signed/one)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (&alter! total (std/int/big/signed/mul (&get total) (get xs i)))
      (alter! i (+ i 1))))
    (&get total))))

; Arbitrary-size structural fixed decimal.
; Representation: { precision signed-scaled }
; - precision is the count of fractional decimal digits.
; - signed-scaled is a SignedBigInt containing value * 10^precision.
(let std/struct-big-dec/zero { 0 std/int/big/signed/zero })
(let std/struct-big-dec/one { 0 std/int/big/signed/one })
(let std/struct-big-dec/precision fst)
(let std/struct-big-dec/signed snd)
(let std/struct-big-dec/negative? (lambda n (std/int/big/signed/negative? (std/struct-big-dec/signed n))))
(let std/struct-big-dec/scale (lambda precision (std/int/big/pow [1 0] precision)))
(let std/struct-big-dec/from-scaled (lambda precision scaled
  { precision (std/int/big/signed/normalize scaled) }))
(let std/struct-big-dec/new (lambda precision neg whole fraction
  (do
    (let scale (std/struct-big-dec/scale precision))
    (let whole-scaled (std/int/big/mul whole scale))
    (let mag (std/int/big/add whole-scaled fraction))
    (std/struct-big-dec/from-scaled precision { neg mag }))))
(let std/struct-big-dec/new-digits (lambda neg whole fraction
  (std/struct-big-dec/new (length fraction) neg whole fraction)))
(let std/struct-big-dec/pi (std/struct-big-dec/new-digits false [3] [1 4 1 5 9 2 6 5 3 5 8 9 7 9 3]))
(let std/struct-big-dec/e (std/struct-big-dec/new-digits false [2] [7 1 8 2 8 1 8 2 8 4 5 9 0 4 5]))
(let std/struct-big-dec/from-int (lambda n
  { 0 (std/int/big/signed/new (std/convert/integer->string n)) }))
(let std/struct-big-dec/from-digits (lambda precision neg scaled-digits
  (std/struct-big-dec/from-scaled precision { neg scaled-digits })))
(let std/struct-big-dec/scaled (lambda n (std/struct-big-dec/signed n)))
(let std/struct-big-dec/whole (lambda n
  (std/int/big/div
    (std/int/big/signed/digits (std/struct-big-dec/signed n))
    (std/struct-big-dec/scale (std/struct-big-dec/precision n)))))
(let std/struct-big-dec/fraction (lambda n
  (std/int/big/mod
    (std/int/big/signed/digits (std/struct-big-dec/signed n))
    (std/struct-big-dec/scale (std/struct-big-dec/precision n)))))
(let std/struct-big-dec/rescale-signed (lambda signed from-precision to-precision
  (if (= from-precision to-precision)
      signed
      (std/int/big/signed/normalize {
        (std/int/big/signed/negative? signed)
        (std/int/big/mul
          (std/int/big/signed/digits signed)
          (std/struct-big-dec/scale (- to-precision from-precision)))
      }))))
(let std/struct-big-dec/align-left (lambda a b
  (do
    (let precision (max (std/struct-big-dec/precision a) (std/struct-big-dec/precision b)))
    (std/struct-big-dec/rescale-signed
      (std/struct-big-dec/signed a)
      (std/struct-big-dec/precision a)
      precision))))
(let std/struct-big-dec/align-right (lambda a b
  (do
    (let precision (max (std/struct-big-dec/precision a) (std/struct-big-dec/precision b)))
    (std/struct-big-dec/rescale-signed
      (std/struct-big-dec/signed b)
      (std/struct-big-dec/precision b)
      precision))))
(let std/struct-big-dec/add (lambda a b
  (do
    (let precision (max (std/struct-big-dec/precision a) (std/struct-big-dec/precision b)))
    (std/struct-big-dec/from-scaled
      precision
      (std/int/big/signed/add
        (std/struct-big-dec/align-left a b)
        (std/struct-big-dec/align-right a b))))))
(let std/struct-big-dec/sub (lambda a b
  (do
    (let precision (max (std/struct-big-dec/precision a) (std/struct-big-dec/precision b)))
    (std/struct-big-dec/from-scaled
      precision
      (std/int/big/signed/sub
        (std/struct-big-dec/align-left a b)
        (std/struct-big-dec/align-right a b))))))
(let std/struct-big-dec/mul (lambda a b
  (std/struct-big-dec/from-scaled
    (+ (std/struct-big-dec/precision a) (std/struct-big-dec/precision b))
    (std/int/big/signed/mul (std/struct-big-dec/signed a) (std/struct-big-dec/signed b)))))
(let std/struct-big-dec/div/precision (lambda precision a b
  (do
    (let numerator
      (std/int/big/signed/mul
        (std/struct-big-dec/signed a)
        { false (std/struct-big-dec/scale (+ precision (std/struct-big-dec/precision b))) }))
    (let denominator
      { false
        (std/int/big/mul
          (std/int/big/signed/digits (std/struct-big-dec/signed b))
          (std/struct-big-dec/scale (std/struct-big-dec/precision a))) })
    (std/struct-big-dec/from-scaled precision (std/int/big/signed/div numerator denominator)))))
(let std/struct-big-dec/div (lambda a b
  (std/struct-big-dec/div/precision
    (max (std/struct-big-dec/precision a) (std/struct-big-dec/precision b))
    a
    b)))
(let std/struct-big-dec/equal? (lambda a b
  (std/int/big/signed/equal?
    (std/struct-big-dec/align-left a b)
    (std/struct-big-dec/align-right a b))))
(let std/struct-big-dec/lt? (lambda a b
  (std/int/big/signed/lt?
    (std/struct-big-dec/align-left a b)
    (std/struct-big-dec/align-right a b))))
(let std/struct-big-dec/lte? (lambda a b
  (std/int/big/signed/lte?
    (std/struct-big-dec/align-left a b)
    (std/struct-big-dec/align-right a b))))
(let std/struct-big-dec/gt? (lambda a b
  (std/int/big/signed/gt?
    (std/struct-big-dec/align-left a b)
    (std/struct-big-dec/align-right a b))))
(let std/struct-big-dec/gte? (lambda a b
  (std/int/big/signed/gte?
    (std/struct-big-dec/align-left a b)
    (std/struct-big-dec/align-right a b))))
(let std/struct-big-dec/negate (lambda n
  (std/struct-big-dec/from-scaled
    (std/struct-big-dec/precision n)
    (std/int/big/signed/negate (std/struct-big-dec/signed n)))))
(let std/struct-big-dec/abs (lambda n
  (std/struct-big-dec/from-scaled
    (std/struct-big-dec/precision n)
    { false (std/int/big/signed/digits (std/struct-big-dec/signed n)) })))
(let std/struct-big-dec/to-string (lambda n
  (do
    (let precision (std/struct-big-dec/precision n))
    (let whole (std/convert/digits->chars (std/struct-big-dec/whole n)))
    (let fraction (std/string/prepend-zeroes
      (std/convert/digits->chars (std/struct-big-dec/fraction n))
      precision))
    (let sign (if (std/struct-big-dec/negative? n) "-" ""))
    (if (= precision 0)
        (cons sign whole)
        (cons sign whole "." fraction)))))

(let std/convert/integer->digits-base (lambda num base
    (if (= num 0) [ 0 ] (do
        (&mut n num)
        (letrec tail-call/while! (lambda out
            (if (> (&get n) 0) (do
                (std/vector/push! out (% (&get n) base))
                (&alter! n (/ (&get n) base))
                (tail-call/while! out)) out)))
        (let digits (tail-call/while! []))
        (reverse digits)))))

(let std/convert/integer->digits (lambda num (std/convert/integer->digits-base num 10)))

(let std/vector/adjacent-difference! (lambda xs fn (do
  (let len (length xs))
  (unless (= len 1)
    (do
      (mut i 1)
      (while (< i len) (do
        (std/vector/update! xs i (fn (get xs (- i 1)) (get xs i)))
        (alter! i (+ i 1))))
      nil)))))


(let std/convert/vector/three-d->string (lambda xs a b (do
    (let out [])
    (let rows (length xs))
    (mut y 0)
    (while (< y rows) (do
      (if (> y 0) (set! out (length out) a))
      (let row (get xs y))
      (let cols (length row))
      (mut x 0)
      (while (< x cols) (do
        (if (> x 0) (set! out (length out) b))
        (let cell (get row x))
        (let cell-len (length cell))
        (mut i 0)
        (while (< i cell-len) (do
          (set! out (length out) (get cell i))
          (alter! i (+ i 1))))
        (alter! x (+ x 1))))
      (alter! y (+ y 1))))
    out)))
(let std/tuple/swap (lambda { a b } { b a }))

(let get* (lambda xs i some none (if (in-bounds? xs i) (do (some (get xs i)) nil) (do (none) nil))))
(let get* (lambda xs i some none (if (in-bounds? xs i) (do (some (get xs i)) nil) (do (none) nil))))
(let std/vector/two-d/get* get*)
(let std/vector/three-d/get* (lambda xs i j some none (if (std/vector/three-d/in-bounds? xs i j) (do (some (get xs i j)) nil) (do (none) nil))))

(let std/int/factorial (lambda n (do
  (letrec fact (lambda n total
    (if (= n 0)
        total
        (fact (- n 1) (* total n)))))
  (fact n 1))))

(let std/dec/factorial (lambda n (do
  (letrec fact (lambda n total
    (if (=. n 0.)
        total
        (fact (-. n 1.) (*. total n)))))
  (fact n 1.))))




(let std/int/div/option (lambda a b (if (= b 0) { false 0 } { true (/ a b) })))
(let std/int/expt/option (lambda a b (if (< a 0) { false 0 } { true (expt b a) })))
(let std/int/mod/option (lambda a b (if (= b 0) { false 0 } { true (% a b) })))
(let std/int/sqrt/option (lambda n (if (< n 0) { false 0 } { true (sqrt n)})) )

(let std/dec/div/option (lambda a b (if (=. b 0.) { false 0. } { true (/. a b) })))
(let std/dec/expt/option (lambda a b (if (<. a 0.) { false 0. } { true (expt/dec b a) })))
(let std/dec/log/option (lambda x (if (<=. x 0.0) { false 0. } { true (log x) })))
(let std/dec/mod/option (lambda a b (if (=. b 0.) { false 0. } { true (%. a b) })))
(let std/dec/sqrt/option (lambda n (if (<. n 0.) { false 0. } { true (sqrt/dec n)})) )


(let std/convert/vector->tuple (lambda xs fn1 fn2 { (fn1 xs) (fn2 xs) }))
(let std/tuple/int/add (lambda { a b } (+ a b)))
(let std/tuple/int/sub (lambda { a b } (- a b)))
(let std/tuple/int/mul (lambda { a b } (* a b)))
(let std/tuple/int/div (lambda { a b } (* a b)))

(let loop/repeat (lambda n fn (do
  (mut i 0)
  (while (< i n) (do
    (fn)
    (alter! i (+ i 1))))
  nil)))
(let loop/some-range? (lambda start end predicate? (do
  (letrec tail-call/loop/some-range? (lambda i out
                          (if (< i end)
                                (if (predicate? i)
                                    true
                                    (tail-call/loop/some-range? (+ i 1) out))
                            out)))
                          (tail-call/loop/some-range? start false))))

(let loop/some-n? (lambda n predicate? (loop/some-range? 0 n predicate?)))

(let push! (lambda xs x (set! xs (length xs) x)))
(let pull! pop-val!)
(let swap! std/vector/swap!)
(let scan! (lambda xs fn (std/vector/adjacent-difference! xs fn)))
(let empty! (lambda xs (do (std/vector/empty! xs) nil)))
(let reverse! std/vector/reverse!)
(let overwrite! std/vector/overwrite!)

(let sort! std/vector/sort!)

(let emod std/int/euclidean-mod)
(let mul std/int/mul)
(let div std/int/div)
(let add std/int/add)
(let sub std/int/sub)

; -------------------------
; Fast Set/Table implementation
; -------------------------
(let std/int/hash
 (lambda table key
   (do
     (let cap (length table))
     (if (= cap 0)
         0
         (do
           (mut i 0)
           (mut hash 0)
           (let len (length key))
           (while (< i len) (do
             (alter! hash (std/int/euclidean-mod (+ (* hash 131) (as (get key i) Int)) cap))
             (alter! i (+ i 1))))
           hash)))))
; -------------------------
; Fast Set implementation
; -------------------------
(let std/vector/hash/set (lambda capacity (buckets (max 4 capacity))))
(let std/vector/hash/set/new std/vector/hash/set)
(let std/vector/hash/set/max-capacity (lambda a b (std/vector/hash/set (max (length a) (length b)))))
(let std/vector/hash/set/min-capacity (lambda a b (std/vector/hash/set (min (length a) (length b)))))

(let std/vector/hash/set/key-equal? (lambda a b (do
  (let len (length a))
  (if (not (= len (length b)))
      false
      (do
        (mut i 0)
        (mut matches true)
        (while (and matches (< i len)) (do
          (if (not (=# (get a i) (get b i)))
              (alter! matches false)
              nil)
          (alter! i (+ i 1))))
        matches)))))

(let std/vector/hash/set/find-index (lambda bucket key (do
  (mut i 0)
  (mut found -1)
  (let len (length bucket))
  (while (and (= found -1) (< i len)) (do
    (if (std/vector/hash/set/key-equal? (get bucket i) key)
        (alter! found i)
        nil)
    (alter! i (+ i 1))))
  found)))

(let std/vector/hash/set/count (lambda table (do
  (mut total 0)
  (let len (length table))
  (mut i 0)
  (while (< i (length table)) (do
    (alter! total (+ total (length (get table i))))
    (alter! i (+ i 1))))
  total)))

(let std/vector/hash/set/for-each (lambda table fn (do
  (mut i 0)
  (let table-len (length table))
  (while (< i table-len) (do
    (let bucket (get table i))
    (mut j 0)
    (let bucket-len (length bucket))
    (while (< j bucket-len) (do
      (fn (get bucket j))
      (alter! j (+ j 1))))
    (alter! i (+ i 1)))))))

(let std/vector/hash/set/add!/raw (lambda table key (do
  (let idx (std/int/hash table key))
  (let bucket (get table idx))
  (set! bucket (length bucket) key)
  table)))

(let std/vector/hash/set/resize! (lambda table new-capacity (do
  (let target (max 4 new-capacity))
  (if (= target (length table))
      table
      (do
        (let entries [])
        (std/vector/hash/set/for-each table (lambda key (set! entries (length entries) key)))
        (std/vector/empty! table)
        (mut i 0)
        (while (< i target) (do
          (set! table (length table) [])
          (alter! i (+ i 1))))
        (mut j 0)
        (let entries-len (length entries))
        (while (< j entries-len) (do
          (std/vector/hash/set/add!/raw table (get entries j))
          (alter! j (+ j 1))))
        table)))))

(let std/vector/hash/set/compact! (lambda table (do
  (let used (std/vector/hash/set/count table))
  (let target (max 32 (* used 2)))
  (std/vector/hash/set/resize! table target))))

(let std/vector/hash/set/has? (lambda table key
  (if (= (length table) 0)
      false
      (do
        (let idx (std/int/hash table key))
        (let bucket (get table idx))
        (>= (std/vector/hash/set/find-index bucket key) 0)))))

(let std/vector/hash/set/add! (lambda table key (do
  (if (= (length table) 0) (do (std/vector/hash/set/resize! table 32) nil) nil)
  (let idx (std/int/hash table key))
  (let bucket (get table idx))
  (if (= (std/vector/hash/set/find-index bucket key) -1)
        (do
        (set! bucket (length bucket) key)
        (if (> (length bucket) 8)
            (do (std/vector/hash/set/resize! table (* (length table) 2)) nil)
            nil))
      nil)
  table)))

(let std/vector/hash/set/remove! (lambda table key (do
  (if (= (length table) 0)
      table
      (do
        (let idx (std/int/hash table key))
        (let bucket (get table idx))
        (let index (std/vector/hash/set/find-index bucket key))
        (if (>= index 0)
            (do
              (set! bucket index (get bucket (- (length bucket) 1)))
              (pop! bucket))
            nil)
        (if (and (> (length table) 32) (= (length bucket) 0))
            (do
              (let used (std/vector/hash/set/count table))
              (if (< (* used 4) (length table))
                  (do (std/vector/hash/set/resize! table (max 32 (/ (length table) 2))) nil)
                  nil))
            nil)
        table)))))

(let std/convert/vector->set/dynamic (lambda xs (do
  (let out (std/vector/hash/set (max 32 (length xs))))
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do
    (std/vector/hash/set/add! out (get xs i))
    (alter! i (+ i 1))))
  out)))

(let std/vector/hash/set/intersection (lambda a b (do
  (let out (std/vector/hash/set/max-capacity a b))
  (let a-count (std/vector/hash/set/count a))
  (let b-count (std/vector/hash/set/count b))
  (let src (if (< a-count b-count) a b))
  (let trg (if (< a-count b-count) b a))
  (std/vector/hash/set/for-each src (lambda key
    (if (and (not-empty? key) (std/vector/hash/set/has? trg key))
        (do (std/vector/hash/set/add! out key) nil)
        nil)))
  out)))

(let std/vector/hash/set/difference (lambda a b (do
  (let out (std/vector/hash/set/max-capacity a b))
  (std/vector/hash/set/for-each a (lambda key
    (if (and (not-empty? key) (not (std/vector/hash/set/has? b key)))
        (do (std/vector/hash/set/add! out key) nil)
        nil)))
  out)))

(let std/vector/hash/set/xor (lambda a b (do
  (let out (std/vector/hash/set/max-capacity a b))
  (std/vector/hash/set/for-each a (lambda key
    (if (and (not-empty? key) (not (std/vector/hash/set/has? b key)))
        (do (std/vector/hash/set/add! out key) nil)
        nil)))
  (std/vector/hash/set/for-each b (lambda key
    (if (and (not-empty? key) (not (std/vector/hash/set/has? a key)))
        (do (std/vector/hash/set/add! out key) nil)
        nil)))
  out)))

(let std/vector/hash/set/union (lambda a b (do
  (let out (std/vector/hash/set/max-capacity a b))
  (std/vector/hash/set/for-each a (lambda key
    (if (not-empty? key) (do (std/vector/hash/set/add! out key) nil) nil)))
  (std/vector/hash/set/for-each b (lambda key
    (if (not-empty? key) (do (std/vector/hash/set/add! out key) nil) nil)))
  out)))

; -------------------------
; Fast Table implementation
; -------------------------
(let std/vector/hash/table (lambda capacity (buckets (max 4 capacity))))
(let std/vector/hash/table/new std/vector/hash/table)
(let std/vector/hash/table/max-capacity (lambda a b (std/vector/hash/table (max (length a) (length b)))))

(let std/vector/hash/table/find-index (lambda bucket key (do
  (mut i 0)
  (mut found -1)
  (let len (length bucket))
  (while (and (= found -1) (< i len)) (do
    (if (std/vector/hash/set/key-equal? (fst (get bucket i)) key)
        (alter! found i)
        nil)
    (alter! i (+ i 1))))
  found)))

(let std/vector/hash/table/for-each (lambda table fn (do
  (mut i 0)
  (let table-len (length table))
  (while (< i table-len) (do
    (let bucket (get table i))
    (mut j 0)
    (let bucket-len (length bucket))
    (while (< j bucket-len) (do
      (fn (get bucket j))
      (alter! j (+ j 1))))
    (alter! i (+ i 1)))))))

(let std/vector/hash/table/count-entries (lambda table (do
  (mut total 0)
  (mut i 0)
  (let len (length table))
  (while (< i len) (do
    (alter! total (+ total (length (get table i))))
    (alter! i (+ i 1))))
  total)))

(let std/vector/hash/table/set!/raw (lambda table key value (do
  (let idx (std/int/hash table key))
  (let bucket (get table idx))
  (set! bucket (length bucket) { key value })
  table)))

(let std/vector/hash/table/resize! (lambda table new-capacity (do
  (let target (max 4 new-capacity))
  (if (= target (length table))
      table
      (do
        (let entries [])
        (std/vector/hash/table/for-each table (lambda entry (set! entries (length entries) entry)))
        (std/vector/empty! table)
        (mut i 0)
        (while (< i target) (do
          (set! table (length table) [])
          (alter! i (+ i 1))))
        (mut j 0)
        (let entries-len (length entries))
        (while (< j entries-len) (do
          (let entry (get entries j))
          (std/vector/hash/table/set!/raw table (fst entry) (snd entry))
          (alter! j (+ j 1))))
        table)))))

(let std/vector/hash/table/compact! (lambda table (do
  (let used (std/vector/hash/table/count-entries table))
  (let target (max 32 (* used 2)))
  (std/vector/hash/table/resize! table target))))

(let std/vector/hash/table/has? (lambda table key
  (if (= (length table) 0)
      false
      (do
        (let idx (std/int/hash table key))
        (let bucket (get table idx))
        (>= (std/vector/hash/table/find-index bucket key) 0)))))

(let std/vector/hash/table/set! (lambda table key value (do
  (if (= (length table) 0) (do (std/vector/hash/table/resize! table 32) nil) nil)
  (let idx (std/int/hash table key))
  (let bucket (get table idx))
  (let index (std/vector/hash/table/find-index bucket key))
  (if (= index -1)
      (do
        (set! bucket (length bucket) { key value })
        (if (> (length bucket) 8)
            (do (std/vector/hash/table/resize! table (* (length table) 2)) nil)
            nil))
      (set! bucket index { key value }))
  table)))

(let std/vector/hash/table/remove! (lambda table key (do
  (if (= (length table) 0)
      table
      (do
        (let idx (std/int/hash table key))
        (let bucket (get table idx))
        (let index (std/vector/hash/table/find-index bucket key))
        (if (>= index 0)
            (do
              (set! bucket index (get bucket (- (length bucket) 1)))
              (pop! bucket))
            nil)
        (if (and (> (length table) 32) (= (length bucket) 0))
            (do
              (let used (std/vector/hash/table/count-entries table))
              (if (< (* used 4) (length table))
                  (do (std/vector/hash/table/resize! table (max 32 (/ (length table) 2))) nil)
                  nil))
            nil)
        table)))))


(let std/vector/hash/table/get* (lambda xs i some none (if (std/vector/hash/table/has? xs i) (do (some (std/vector/hash/table/get xs i)) nil) (do (none) nil))))

(let std/vector/hash/table/get (lambda table key
  (if (= (length table) 0)
      []
      (do
        (let idx (std/int/hash table key))
        (let bucket (get table idx))
        (let index (std/vector/hash/table/find-index bucket key))
        (if (>= index 0) [ (get bucket index) ] [])))))

(let std/vector/hash/table/update! (lambda table key init f (do
  (if (std/vector/hash/table/has? table key)
      (std/vector/hash/table/set! table key (f (snd (get (std/vector/hash/table/get table key) 0))))
      (std/vector/hash/table/set! table key init))
  nil)))
(let std/vector/hash/table/update-or! (lambda table key missing present (do
  (if (std/vector/hash/table/has? table key)
      (std/vector/hash/table/set! table key (present (snd (get (std/vector/hash/table/get table key) 0))))
      (std/vector/hash/table/set! table key (missing)))
  nil)))
(let std/vector/hash/table/push-or! (lambda table key value (do
  (if (std/vector/hash/table/has? table key)
      (do (push! (snd (get (std/vector/hash/table/get table key) 0)) value) nil)
      (do (std/vector/hash/table/set! table key [value]) nil))
  nil)))

(let std/vector/hash/table/entries (lambda table (do
  (let out [])
  (std/vector/hash/table/for-each table (lambda entry (set! out (length out) entry)))
  out)))

(let std/vector/hash/table/keys (lambda table (do
  (let entries (std/vector/hash/table/entries table))
  (let out [])
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (set! out (length out) (fst (get entries i)))
    (alter! i (+ i 1))))
  out)))

(let std/vector/hash/table/values (lambda table (do
  (let entries (std/vector/hash/table/entries table))
  (let out [])
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (set! out (length out) (snd (get entries i)))
    (alter! i (+ i 1))))
  out)))

(let std/vector/hash/table/count (lambda arr (do
  (let table (std/vector/hash/table (max 64 (length arr))))
  (mut i 0)
  (let len (length arr))
  (while (< i len) (do
    (let key (get arr i))
    (let hit (std/vector/hash/table/get table key))
    (if (= (length hit) 0)
        (std/vector/hash/table/set! table key 1)
        (std/vector/hash/table/set! table key (+ (snd (get hit 0)) 1)))
    (alter! i (+ i 1))))
  table)))

(let std/vector/hash/table/frequency (lambda xs (do
  (let table (std/vector/hash/table (max 64 (length xs))))
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do
    (let key [(get xs i)])
    (let hit (std/vector/hash/table/get table key))
    (if (= (length hit) 0)
        (std/vector/hash/table/set! table key 1)
        (std/vector/hash/table/set! table key (+ (snd (get hit 0)) 1)))
    (alter! i (+ i 1))))
  table)))

(let std/vector/hash/table/drop! (lambda table keys (do
  (mut i 0)
  (let len (length keys))
  (while (< i len) (do
    (std/vector/hash/table/remove! table (get keys i))
    (alter! i (+ i 1)))))))

(let std/vector/hash/table/keep (lambda table keys (do
  (let out (std/vector/hash/table (max 32 (length keys))))
  (mut i 0)
  (let len (length keys))
  (while (< i len) (do
    (let key (get keys i))
    (let hit (std/vector/hash/table/get table key))
    (if (> (length hit) 0)
        (do (std/vector/hash/table/set! out key (fst (get hit 0))) nil)
        nil)
    (alter! i (+ i 1))))
  out)))

(let std/vector/hash/table/merge! (lambda a b (do
  (let entries (std/vector/hash/table/entries b))
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (let entry (get entries i))
    (std/vector/hash/table/set! a (fst entry) (snd entry))
    (alter! i (+ i 1))))
  a)))

(let std/vector/hash/table/merge (lambda a b (do
  (let out (std/vector/hash/table/max-capacity a b))
  (std/vector/hash/table/merge! out a)
  (std/vector/hash/table/merge! out b)
  out)))

(let std/vector/hash/table/omit (lambda table keys (do
  (let out (std/vector/hash/table/merge (std/vector/hash/table 32) table))
  (std/vector/hash/table/drop! out keys)
  out)))

(let std/convert/vector->table (lambda entries (do
  (let out (std/vector/hash/table (max 32 (length entries))))
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (let entry (get entries i))
    (std/vector/hash/table/set! out (fst entry) (snd entry))
    (alter! i (+ i 1))))
  out)))


(let std/int/min/three (lambda a b c (min (min a b) c)))
(let std/int/min/four (lambda a b c d (min (min a b) (min c d))))
(let std/int/min/two min)

(let std/vector/char/damerau-levenshtein (lambda a b (do
  (let n (length a))
  (let m (length b))
  (let matrix (Matrix/new (lambda _ _ 0) (+ n 1) (+ m 1) ))

  (mut i0 0)
  (while (<= i0 n) (do
    (let row (get matrix i0))
    (set! row 0 i0)
    (alter! i0 (+ i0 1))))

  (let first-row (get matrix 0))
  (mut j0 0)
  (while (<= j0 m) (do
    (set! first-row j0 j0)
    (alter! j0 (+ j0 1))))

  (mut i 1)
  (while (<= i n) (do
    (let current-row (get matrix i))
    (let prev-row (get matrix (- i 1)))
    (mut j 1)
    (while (<= j m) (do
      (let a-char (get a (- i 1)))
      (let b-char (get b (- j 1)))
      (let replace-cost (if (=# a-char b-char) 0 1))

      (let delete-cost (+ (get prev-row j) 1))
      (let insert-cost (+ (get current-row (- j 1)) 1))
      (let subst-cost (+ (get prev-row (- j 1)) replace-cost))
      (let best (std/int/min/three delete-cost insert-cost subst-cost))

      (let with-transpose
        (if (and (> i 1)
                 (> j 1)
                 (=# a-char (get b (- j 2)))
                 (=# (get a (- i 2)) b-char))
            (min best (+ (get (get matrix (- i 2)) (- j 2)) 1))
            best))
      (set! current-row j with-transpose)
      (alter! j (+ j 1))))
    (alter! i (+ i 1))))

  (get (get matrix n) m))))


(let std/vector/char/join2 (lambda a b (do
  (let out [])
  (mut i 0)
  (while (< i (length a)) (do
    (push! out (get a i))
    (alter! i (+ i 1))))
  (mut j 0)
  (while (< j (length b)) (do
    (push! out (get b j))
    (alter! j (+ j 1))))
  out)))

(let std/vector/char/join3 (lambda a b c
  (std/vector/char/join2
    (std/vector/char/join2 a b)
    c)))

; Patch format:
; {pos delete-count insert-text}
;
; Example:
; {5 0 " world"} means insert " world" at index 5
; {5 6 ""} means delete 6 chars from index 5
; {5 5 "Que"} means replace 5 chars with "Que"

(let std/vector/char/diff/simple (lambda old new (do
  (let old-len (length old))
  (let new-len (length new))

  ; common prefix
  (mut prefix 0)
  (while
    (and
      (< prefix old-len)
      (< prefix new-len)
      (=# (get old prefix) (get new prefix)))
    (alter! prefix (+ prefix 1)))

  ; common suffix
  (mut old-suffix old-len)
  (mut new-suffix new-len)

  (while
    (and
      (> old-suffix prefix)
      (> new-suffix prefix)
      (=#
        (get old (- old-suffix 1))
        (get new (- new-suffix 1))))
    (do
      (alter! old-suffix (- old-suffix 1))
      (alter! new-suffix (- new-suffix 1))))

  (let delete-count (- old-suffix prefix))
  (let insert-text (slice prefix new-suffix new))

  {prefix delete-count insert-text})))

(let std/vector/char/apply-patch (lambda source patch (do
  (let pos (fst patch))
  (let rest (snd patch))
  (let delete-count (fst rest))
  (let insert-text (snd rest))

  (let before (slice 0 pos source))
  (let after
    (slice
      (+ pos delete-count)
      (length source)
      source))

  (std/vector/char/join3 before insert-text after))))

(let std/vector/char/apply-patches (lambda source patches (do
  (mut result source)
  (mut i 0)
  (while (< i (length patches)) (do
    (alter! result (std/vector/char/apply-patch result (get patches i)))
    (alter! i (+ i 1))))
  result)))

(let std/text/history/new (lambda _ {"" []}))

(let std/text/history/source (lambda history
  (fst history)))

(let std/text/history/patches (lambda history
  (snd history)))

(let std/text/history/add! (lambda history new-source (do
  (let old-source (fst history))
  (let patches (snd history))
  (let patch (std/vector/char/diff/simple old-source new-source))

  (push! patches patch)

  ; return updated history
  {new-source patches})))

(let std/text/history/reconstruct (lambda history (do
  (let patches (snd history))
  (std/vector/char/apply-patches "" patches))))




(let std/string/find (lambda target xs (do
  (let len (length xs))
  (mut i 0)
  (mut result -1)
  (while (and (= result -1) (< i len)) (do
    (if (match? (get xs i) target)
        (alter! result i))
    (alter! i (+ i 1))))
  result)))
(let std/string/find/last (lambda target xs (do
  (let len (length xs))
  (mut i 0)
  (mut result -1)
  (while (< i len) (do
    (if (match? (get xs i) target)
        (alter! result i))
    (alter! i (+ i 1))))
  result)))
(let std/string/prepend-zeroes (lambda (s target-len)
  (do
    (mut result s)
    (while (< (length result) target-len)
      (alter! result (cons "0" result)))
    result)))
(let std/convert/dec->string-scale (lambda (scale x)
  (if (=. (floor x) x)
      (cons (std/convert/integer->string (Dec->Int x)) ".0")
      (do
        (let flip
          (if (<. x 0.0) -1.0 1.0))

        (let exponent
          (floor x))

        (let mantisa
          (-. x exponent))

        (let left
          (std/convert/integer->string (Dec->Int exponent)))

        (let right-unpadded
          (std/convert/integer->string
            (Dec->Int (*. mantisa std/dec/dec-scaling flip))))

        (let right
          (std/string/prepend-zeroes right-unpadded 6))

        (mut len (length right))
        (while (=# (get right (- len 1)) '0')
            (pop! right)
            (alter! len (- len 1)))

        (cons left ['.'] right)))))
(let std/convert/dec->string-6 (std/convert/dec->string-scale 6))

(let std/io/println! (lambda text (do
  (print! text)
  (print! ['\n']))))
(let println! std/io/println!)

(let std/vector/char/matches-at?
  (lambda (data target offset)
    (let target-len (length target))
    (mut i 0)
    (mut same true)
    (while (and same (< i target-len))
      (do
        (if (not (=# (get data (+ offset i))
                     (get target i)))
            (alter! same false))
        (alter! i (+ i 1))))
    same))

(let std/vector/char/contains?
  (lambda (data target)
    (let data-len (length data))
    (let target-len (length target))
    (if (= target-len 0)
        true
        (if (> target-len data-len)
            false
            (do
              (mut i 0)
              (mut found false)
              (while
                (and
                  (not found)
                  (<= i (- data-len target-len)))
                (do
                  (if
                    (std/vector/char/matches-at?
                      data
                      target
                      i)
                    (alter! found true))
                  (alter! i (+ i 1))))
              found)))))

(let std/vector/char/starts?
  (lambda (data prefix)
    (let data-len (length data))
    (let prefix-len (length prefix))
    (if (> prefix-len data-len)
        false
        (std/vector/char/matches-at?
          data
          prefix
          0))))

(let std/vector/char/ends?
  (lambda (data suffix)
    (let data-len (length data))
    (let suffix-len (length suffix))
    (if (> suffix-len data-len)
        false
        (std/vector/char/matches-at?
          data
          suffix
          (- data-len suffix-len)))))
