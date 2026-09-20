(let Vector/swap! (lambda xs i j (do (let temp (get xs i)) (set! xs i (get xs j)) (set! xs j temp))))

(let Vector/sort-partition! (lambda arr start end fn (do
     (let pivot (get arr end))
     (mut i (- start 1))
     (mut j start)

     (while (< j end) (do
           (if (fn (get arr j) pivot) (do
          (alter! i (+ i 1))
          (Vector/swap! arr i j)
          nil))
          (alter! j (+ j 1))))

     (Vector/swap! arr (+ i 1) end)
     (+ i 1))))

(let Vector/sort! (lambda arr fn (do
     (let stack [])
     (push! stack 0)
     (push! stack (- (length arr) 1))
     (while (> (length stack) 0) (do
           (let end (get stack (- (length stack) 1)))
           (pop! stack)
           (let start (get stack (- (length stack) 1)))
           (pop! stack)
           (if (< start end) (do
                 (let pivot-index (Vector/sort-partition! arr start end fn))
                 (push! stack start)
                 (push! stack (- pivot-index 1))
                 (push! stack (+ pivot-index 1))
                 (push! stack end)
                 nil))))
     arr)))

(let Matrix/for/i (lambda matrix fn (do
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

(let Int/min/three (lambda a b c (min (min a b) c)))

(let String/damerau-levenshtein (lambda a b (do
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
      (let best (Int/min/three delete-cost insert-cost subst-cost))

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

(let floor (lambda n (-. n (%. n 1.0))))
(let ceil (lambda n (do
    (let sign (if (>=. n 0.0) 1 -1))
    (let absn (if (>=. n 0.0) n (-. n)))
    (let frac (%. absn 1.0))
    (let intpart (-. absn frac))
    (cond (=. n 0.0) n (if (= sign 1) (+. intpart 1.0) (-. intpart))))))

(let at (lambda xs i (if (< i 0) (get xs (+ (length xs) i)) (get xs i))))

(let first (lambda xs (get xs 0)))

(let last (lambda xs (get xs (- (length xs) 1))))

(let digit? (lambda ch (and (>=# ch '0') (<=# ch '9'))))

(let upper (lambda ch (if (and (>=# ch 'a') (<=# ch 'z')) (-# ch ' ') ch)))

(let lower (lambda ch (if (and (>=# ch 'A') (<=# ch 'Z')) (+# ch ' ') ch)))

(let const (lambda x _ x))

(let empty? (lambda xs (= (length xs) 0)))

(let not-empty? (lambda xs (not (= (length xs) 0))))

(let in-bounds? (lambda xs index (and (< index (length xs)) (>= index 0))))

(let range (lambda start end (do
     (let out [ start ])
     (mut i (+ start 1))
     (while (<= i end) (do
        (set! out (length out) i)
        (alter! i (+ i 1))))
     out)))

(let range/inclusive range)

(let range/exclusive (lambda start end (range start (- end 1))))

(let range/dec (lambda start end (do
     (let out [ (Int->Dec start) ])
     (mut i (+ start 1))
     (while (<= i end) (do
        (set! out (length out) (Int->Dec i))
        (alter! i (+ i 1))))
     out)))

 (let ones/dec (lambda n (do
     (let out [])
     (mut i 0)
     (while (< i n) (do
        (set! out (length out) 1.0)
        (alter! i (+ i 1))))
     out)))

 (let zeroes/dec (lambda n (do
     (let out [])
     (mut i 0)
     (while (< i n) (do
        (set! out (length out) 0.0)
        (alter! i (+ i 1))))
     out)))

 (let ones (lambda n (do
     (let out [])
     (mut i 0)
     (while (< i n) (do
        (set! out (length out) 1)
        (alter! i (+ i 1))))
     out)))

 (let zeroes (lambda n (do
     (let out [])
     (mut i 0)
     (while (< i n) (do
        (set! out (length out) 0)
        (alter! i (+ i 1))))
     out)))

 (let truths (lambda n (do
     (let out [])
     (mut i 0)
     (while (< i n) (do
        (set! out (length out) true)
        (alter! i (+ i 1))))
     out)))

 (let falses (lambda n (do
     (let out [])
     (mut i 0)
     (while (< i n) (do
        (set! out (length out) false)
        (alter! i (+ i 1))))
     out)))

(let cartesian-product (lambda a b (do
    (let out [])
    (let len-a (length a))
    (let len-b (length b))
    (mut i 0)
    (while (< i len-a) (do
      (let x (get a i))
      (mut j 0)
      (while (< j len-b) (do
        (set! out (length out) { x (get b j) })
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    out)))

(let gcd (lambda a b (do
    (mut A a)
    (mut B b)
    (while (> B 0) (do
        (let a A)
        (let b B)
        (alter! A b)
        (alter! B (% a b))))
    A)))

(let lcm (lambda a b (/ (* a b) (gcd  a b))))

(let abs (lambda n (- (^ n (>> n 31)) (>> n 31))))

(let abs. (lambda n (if (<. n 0.0) (*. n -1.0) n)))

(let positive? (lambda x (> x 0)))

(let negative? (lambda x (< x 0)))

(let invert (lambda x (- x)))

(let zero? (lambda x (= x 0)))

(let one? (lambda x (= x 1)))

(let negative-one? (lambda x (= x -1)))

(let positive/dec? (lambda x (>. x 0.)))

(let negative/dec? (lambda x (<. x 0.)))

(let invert/dec (lambda x (-. x)))

(let zero/dec? (lambda x (=. x 0.)))

(let one/dec? (lambda x (=. x 1.)))

(let negative-one/dec? (lambda x (=. x -1.)))

(let square (lambda x (* x x)))

(let even? (lambda x (= (% x 2) 0)))

(let odd? (lambda x (not (= (% x 2) 0))))

(let even/dec? (lambda x (=. (%. x 2.) 0.)))

(let odd/dec? (lambda x (not (=. (%. x 2.) 0.))))

(let sum (lambda xs (do
    (mut total 0)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (alter! total (+ total (get xs i)))
      (alter! i (+ i 1))))
    total)))

(let sum/dec (lambda xs (do
    (mut total 0.0)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (alter! total (+. total (get xs i)))
      (alter! i (+ i 1))))
    total)))

(let sum/bool (lambda xs (do
    (mut total 0)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (if (get xs i) (alter! total (+ total 1)))
      (alter! i (+ i 1))))
    total)))

(let product (lambda xs (do
    (mut total 1)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (alter! total (* total (get xs i)))
      (alter! i (+ i 1))))
    total)))

(let product/dec (lambda xs (do
    (mut total 1.0)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (alter! total (*. total (get xs i)))
      (alter! i (+ i 1))))
    total)))

(let max (lambda a b (if (> a b) a b)))

(let min (lambda a b (if (< a b) a b)))

(let max. (lambda a b (if (>. a b) a b)))

(let min. (lambda a b (if (<. a b) a b)))

(let maximum (lambda xs (cond
    (empty? xs) Int
    (= (length xs) 1) (get xs 0)
    (do
      (mut best (get xs 0))
      (let len (length xs))
      (mut i 1)
      (while (< i len) (do
        (let value (get xs i))
        (if (> value best) (alter! best value))
        (alter! i (+ i 1))))
      best))))

(let minimum (lambda xs (cond
    (empty? xs) Int
    (= (length xs) 1) (get xs 0)
    (do
      (mut best (get xs 0))
      (let len (length xs))
      (mut i 1)
      (while (< i len) (do
        (let value (get xs i))
        (if (< value best) (alter! best value))
        (alter! i (+ i 1))))
      best))))

(let maximum/dec (lambda xs (cond
    (empty? xs) Dec
    (= (length xs) 1) (get xs 0)
    (do
      (mut best (get xs 0))
      (let len (length xs))
      (mut i 1)
      (while (< i len) (do
        (let value (get xs i))
        (if (>. value best) (alter! best value))
        (alter! i (+ i 1))))
      best))))

(let minimum/dec (lambda xs (cond
    (empty? xs) Dec
    (= (length xs) 1) (get xs 0)
    (do
      (mut best (get xs 0))
      (let len (length xs))
      (mut i 1)
      (while (< i len) (do
        (let value (get xs i))
        (if (<. value best) (alter! best value))
        (alter! i (+ i 1))))
      best))))

(let avg/dec (lambda x y (/. (+. x y) 2.0)))

(let avg (lambda x y (/ (+ x y) 2)))

(let mean (lambda xs (/ (sum xs) (length xs))))

(let mean/dec (lambda xs (/. (sum/dec xs) (Int->Dec (length xs)))))

(let median (lambda xs (do
    (let len (length xs))
    (let half (/ len 2))
    (if (odd? len)
        (get xs half)
        (/ (+ (get xs (- half 1)) (get xs half)) 2)))))

(let median/dec (lambda xs (do
  (let len (Int->Dec (length xs)))
  (let half (/. len 2.))
  (let mid (Dec->Int half))
  (if (odd/dec? len)
      (get xs mid)
      (/. (+. (get xs (Dec->Int (-. half 1.))) (get xs mid)) 2.)))))

(let sqrt (lambda n
  (do
    (mut low 0)
    (mut high n)
    (mut mid 0)
    (mut res 0)
    (while (<= low high)
      (do
        (alter! mid (+ low (/ (- high low) 2)))
        (if (<= mid 0)
          (do
            (alter! res mid)
            (alter! low (+ mid 1)))
          (if (<= mid (/ n mid))
            (do
              (alter! res mid)
              (alter! low (+ mid 1)))
            (alter! high (- mid 1))))))
    res)))

(let sqrt/dec (lambda n
  (do
    (&mut low 0.)
    (&mut high n)
    (&mut mid 0.)
    (&mut i 0.)
    ; Loop 100 times for high precision
    (while (<. (&get i) 100.)
      (do
        (&alter! mid (/. (+. (&get low) (&get high)) 2.))
        (if (<=. (*. (&get mid) (&get mid)) n)
            (&alter! low (&get mid))
            (&alter! high (&get mid)))
        (&alter! i (+. (&get i) 1.))))
    (&get low))))

(let delta (lambda a b (abs (- a b))))

(let delta/dec (lambda a b (abs. (-. a b))))

(let zip (lambda xs (do
  (let a (fst xs))
  (let b (snd xs))
  (mut i 0)
  (let out [])
  (while (< i (length a)) (do
    (push! out { (get a i) (get b i) })
    (alter! i (+ i 1))))
  out)))

(let unzip (lambda xs (do
    (let left [])
    (let right [])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (let pair (get xs i))
      (set! left (length left) (fst pair))
      (set! right (length right) (snd pair))
      (alter! i (+ i 1))))
    { left right })))

(let reverse (lambda xs (if (empty? xs) xs (do
     (let out [])
     (let len (length xs))
     (mut i 0)
     (while (< i len) (do (set! out (length out) (get xs (- len i 1))) (alter! i (+ i 1))))
     out))))

(let buckets (lambda size (do
     (let out [[]])
     (mut i 1)
     (while (< i size) (do (set! out (length out) []) (alter! i (+ i 1))))
     out)))

(let match? (lambda a b (do
    (let len-a (length a))
    (let len-b (length b))
    (if (<> len-a len-b) false (do
        (mut i 0)
        (mut same true)
        (while (and same (< i len-a)) (do
            (if (not (=# (get a i) (get b i))) (alter! same false) nil)
            (alter! i (+ i 1))))
        same)))))

(let flat (lambda xs (cond
     (empty? xs) []
     (= (length xs) 1) (get xs)
     (do
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
       out))))

  (let combination/pairs (lambda xs (do
    (let pairs [])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
        (mut j (+ i 1))
        (while (< j len) (do
            (Vector/push! pairs { (get xs i) (get xs j) })
            (alter! j (+ j 1))))
        (alter! i (+ i 1))))
    pairs)))

(let unique/int (lambda xs
    (if (= (length xs) 0)
        [(+ (get xs 0) 0)]
        (<| xs (map (lambda x [(as x Char)])) (Vector->Set) (Set->Vector) (map (lambda x (as (get x 0) Int)))))))

(let unique/char (lambda xs
    (if (= (length xs) 0)
        xs
        (<| xs (map (lambda x [x])) (Vector->Set) (Set->Vector) (map (lambda x (get x 0)))))))

(let neighborhood/diagonal [ [ 1 -1 ] [ -1 -1 ] [ 1 1 ] [ -1 1 ] ])

(let neighborhood/kernel [ [ 0 0 ] [ 0 1 ] [ 1 0 ] [ -1 0 ] [ 0 -1 ] [ 1 -1 ] [ -1 -1 ] [ 1 1 ] [ -1 1 ]])

(let neighborhood/moore [ [ 0 1 ] [ 1 0 ] [ -1 0 ] [ 0 -1 ] [ 1 -1 ] [ -1 -1 ] [ 1 1 ] [ -1 1 ] ])

(let neighborhood/von-neumann [ [ 1 0 ] [ 0 -1 ] [ 0 1 ] [ -1 0 ] ])

(let split/lines (lambda xs (String/to-vector/raw xs '\n')))

(let split/words (lambda xs (String/to-vector/raw xs ' ')))

(let split/commas (lambda xs (String/to-vector/raw xs ',')))

(let subset (lambda xs (if (empty? xs) [ xs ] (do
    (let n (length xs))
    (let out [])
    (mut i 0)
    (let limit (expt n 2))
    (while (< i limit) (do
        ; generate bitmask, from 0..00 to 1..11
        (let bits (Integer->Bits i))
        (let subset [])
        (let bits-len (length bits))
        (mut j 0)
        (while (< j bits-len) (do
          (if (= (get bits j) 1)
              (Vector/append/raw! subset (get xs j)))
          (alter! j (+ j 1))))
        (Vector/push! out subset)
        (alter! i (+ i 1))))
    out))))

(let copy (lambda xs (do
    (let out [])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (set! out (length out) (get xs i))
      (alter! i (+ i 1))))
    out)))

(let fill (lambda n fn (do
  (let out [])
  (mut i n)
  (while (> i 0) (do
    (Vector/append/raw! out (fn i))
    (alter! i (- i 1))))
  out)))

(let transpose (lambda matrix (if (empty? matrix) matrix (do
    (let H (length matrix))
    (let W (length (get matrix 0)))
    (let out [])
    (mut i 0)
    (while (< i W) (do
        (mut j 0)
        (Vector/push! out [])
        (while (< j H) (do
          (Vector/push! (at out -1) (get matrix j i))
          (alter! j (+ j 1))))
        (alter! i (+ i 1))))
    out))))

(let interleave (lambda xs ys (do
  (let out [])
  (let len (min (length xs) (length ys)))
  (mut i 0)
  (while (< i len) (do
    (Vector/push! out (get xs i))
    (Vector/push! out (get ys i))
    (alter! i (+ i 1))))
  out)))

(let sequence (lambda xs (range 0 (- (length xs) 1))))

(let shoelace (lambda px (do
    (let len (length px))
    (mut a 0)
    (mut b 0)
    (mut i 0)
    (while (< i len) (do
      (let left (get px i))
      (let right (get px (% (+ i 1) len)))
      (let y1 (get left 0))
      (let x1 (get left 1))
      (let y2 (get right 0))
      (let x2 (get right 1))
      (alter! a (+ a (* y1 x2)))
      (alter! b (+ b (* y2 x1)))
      (alter! i (+ i 1))))
    (/ (abs (- a b)) 2))))

(let collinear? (lambda px (= (shoelace px) 0)))

(let cycle (lambda n xs (do
  (let out [])
  (let len (length xs))
  (mut i 0)
  (while (< i n) (do
    (Vector/push! out (get xs (% i len)))
    (alter! i (+ i 1))))
  out)))

(let replicate (lambda n x (do
  (let out [])
  (mut i 0)
  (while (< i n) (do
    (Vector/push! out x)
    (alter! i (+ i 1))))
  out)))

(let extreme (lambda xs { (minimum xs) (maximum xs) }))

(let enumerate (lambda xs (zip { (range 0 (- (length xs) 1)) xs })))

(let permutation (lambda arr (do
  (letrec permute (lambda arr (if (<= (length arr) 1)
        [arr]
        (do
          (let out [])
          (&mut i 0)
          (while (< (&get i) (length arr)) (do
              (let rest (filter/i (lambda y j (not (= j (&get i)))) arr))
              (let perms (permute rest))
              (let x (get arr (&get i)))
              (&mut j 0)
              (while (< (&get j) (length perms)) (do
                  (set! out (length out) (Vector/cons/raw! [x] (get perms (&get j))))
                  (&alter! j (+ (&get j) 1))))
              (&alter! i (+ (&get i) 1))))
          out))))
    (permute arr))))

(let combination (lambda xs (do
    (let out [])
    (letrec combinations (lambda arr size start temp
        (if (= (length temp) size)
            (set! out (length out) (copy temp))
            (do
              (mut i start)
              (let len (length arr))
              (while (< i len) (do
                    (set! temp (length temp) (get arr i))
                    (combinations arr size (+ i 1) temp)
                    (pop! temp)
                    (alter! i (+ i 1))))))))
   (mut i 1)
   (let max-size (+ 1 (length xs)))
   (while (< i max-size) (do
      (combinations xs i 0 [])
      (alter! i (+ i 1))))
    out)))

(let true/option (lambda x { true x }))

(let false/option (lambda x { false x }))

(let space? (lambda c
    (or (=# c ' ')
        (=# c '\n')
        (=# c '\r')
        (=# c '\t'))))

(let trim/left (lambda xs (do
      (let len (length xs))
      (mut start 0)
      (while (and (< start len) (space? (get xs start))) (do
        (++ start)))
      (slice start len xs))))

(let trim/right (lambda xs (do
      (let len (length xs))
      (mut end (- len 1))
      (while (and (>= end 0) (space? (get xs end))) (do
        (-- end)))
      (slice 0 (+ end 1) xs))))

(let trim (lambda xs (trim/right (trim/left xs))))


(let for (lambda fn xs (do
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do (fn (get xs i)) (alter! i (+ i 1)))))))

(let for/i (lambda fn xs (do
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do (fn (get xs i) i) (alter! i (+ i 1)))))))

(let filter/i (lambda fn? xs (if (empty? xs) xs (do
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do
            (let x (get xs i))
            (if (fn? x i) (set! out (length out) x))
            (alter! i (+ i 1))))
     out))))

(let reduce (lambda fn initial xs (do
     (mut out initial)
     (mut i 0)
     (while (< i (length xs)) (do (alter! out (fn out (get xs i))) (alter! i (+ i 1))))
     out)))

(let reduce/i (lambda fn initial xs (do
     (mut out initial)
     (mut i 0)
     (while (< i (length xs)) (do (alter! out (fn out (get xs i) i)) (alter! i (+ i 1))))
     out)))

(let map (lambda fn xs (if (empty? xs) [] (do
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do (set! out (length out) (fn (get xs i))) (alter! i (+ i 1))))
     out))))

(let map/i (lambda fn xs (if (empty? xs) [] (do
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do (set! out (length out) (fn (get xs i) i)) (alter! i (+ i 1))))
     out))))

(let reduce/until (lambda fn fn? initial xs (do
  (mut out initial)
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do
    (let x (get xs i))
    (let a out)
    (unless (fn? a x) (alter! out (fn a x)) (alter! placed true))
    (alter! i (+ i 1))))
out)))

(let reduce/until/i (lambda fn fn? initial xs (do
  (mut out initial)
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do
    (let idx i)
    (let x (get xs idx))
    (let a out)
    (unless (fn? a x idx) (alter! out (fn a x idx)) (alter! placed true))
    (alter! i (+ i 1))))
out)))

(let for/until (lambda fn fn? xs (do
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do
    (let x (get xs i))
    (unless (fn? x) (do (fn x) nil) (alter! placed true))
    (alter! i (+ i 1)))))))

(let for/until/i (lambda fn fn? xs (do
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do
    (let idx i)
    (let x (get xs idx))
    (unless (fn? x idx) (do (fn x idx) nil) (alter! placed true))
    (alter! i (+ i 1)))))))

(let map/until (lambda fn fn? xs (do
  (let out [])
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do
    (let x (get xs i))
    (unless (fn? x) (do (push! out (fn x)) nil) (alter! placed true))
    (alter! i (+ i 1))))
  out)))

(let map/until/i (lambda fn fn? xs (do
  (let out [])
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do
    (let idx i)
    (let x (get xs idx))
    (unless (fn? x idx) (do (push! out (fn x idx)) nil) (alter! placed true))
    (alter! i (+ i 1))))
  out)))

(let count (lambda fn? xs (length (filter fn? xs))))

(let count/int (lambda item xs (count (lambda x (= x item)) xs)))

(let count/dec (lambda item xs (count (lambda x (=. x item)) xs)))

(let count/char (lambda item xs (count (lambda x (=# x item)) xs)))

(let count/bool (lambda item xs (count (lambda x (=? x item)) xs)))

(let every? (lambda predicate? xs (do
           (mut i 0)
           (let len (length xs))
           (while (and (< i len) (predicate? (get xs i))) (alter! i (+ i 1)))
           (not (> len i)))))

(let some? (lambda predicate? xs (do
           (mut i 0)
           (let len (length xs))
           (while (and (< i len) (not (predicate? (get xs i)))) (alter! i (+ i 1)))
           (or (= len 0) (> len i)))))

(let every/i? (lambda predicate? xs (do
           (mut i 0)
           (let len (length xs))
           (while (and (< i len) (predicate? (get xs i) i)) (alter! i (+ i 1)))
           (not (> len i)))))

(let some/i? (lambda predicate? xs (do
           (mut i 0)
           (let len (length xs))
           (while (and (< i len) (not (predicate? (get xs i) i))) (alter! i (+ i 1)))
           (or (= len 0) (> len i)))))

(let divisible? (lambda b a (= (% a b) 0)))

(let divisible/dec? (lambda b a (=. (%. a b) 0.)))

(let clamp (lambda limit x (if (> x limit) limit x)))

(let clamp-range (lambda start end x (cond (> x end) end (< x start) start x)))

(let clamp/dec (lambda limit x (if (>. x limit) limit x)))

(let clamp-range/dec (lambda start end x (cond (>. x end) end (<. x start) start x)))

(let expt (lambda exp base (do
  (if (< exp 0) 0 (do
      (mut result 1)
      (mut b base)
      (mut e exp)
      (while (> e 0) (do
          (if (= (% e 2) 1)
            (alter! result (* result b)))
            (alter! b (* b b))
            (alter! e (/ e 2))))
      result)))))

(let expt/dec (lambda exp base
  (do
    (&mut res 1.)
    (&mut b base)
    (&mut e exp)

    ; 1. Handle the integer part of the exponent
    (while (>=. (&get e) 1.)
      (do
        (if (=. (%. (floor (&get e)) 2.) 1.)
            (&alter! res (*. (&get res) (&get b))))
        (&alter! b (*. (&get b) (&get b)))
        (&alter! e (/. (floor (&get e)) 2.))))

    ; 2. Handle the fractional part using square roots
    ; Refresh 'b' to original base and 'e' to the remaining fraction
    (&alter! b base)
    (&alter! e (-. exp (floor exp)))
    (&mut root (sqrt/dec (&get b)))
    (&mut frac 0.5)

    ; Loop 22. times for precision (handles bits of the fraction)
    (&mut i 0.)
    (while (<. (&get i) 22.)
      (do
        (if (>=. (&get e) (&get frac))
            (do
              (&alter! res (*. (&get res) (&get root)))
              (&alter! e (-. (&get e) (&get frac)))))
        (&alter! root (sqrt/dec (&get root)))
        (&alter! frac (/. (&get frac) 2.))
        (&alter! i (+. (&get i) 1.))))
    (&get res))))

(let map/adjacent (lambda fn xs (if (empty? xs) [] (do
  (let out [])
  (mut i 1)
  (let len (length xs))
  (while (< i len) (do
    (Vector/push! out (fn (get xs (- i 1)) (get xs i)))
    (alter! i (+ i 1))))
  out))))

(let zip-with (lambda f a b (do
    (let out [])
    (mut i 0)
    (let len (length a))
    (while (< i len) (do (set! out (length out) (f (get a i) (get b i))) (alter! i (+ i 1))))
    out)))

(let slice (lambda start end xs (if (empty? xs) xs (do
     (let bounds (- end start))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs (+ start i))) (alter! i (+ i 1))))
     out))))

(let drop/first (lambda start xs (if (empty? xs) xs (do
     (let end (length xs))
     (let bounds (- end start))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs (+ start i))) (alter! i (+ i 1))))
     out))))

(let drop/last (lambda end xs (if (empty? xs) xs (do
     (let bounds (- (length xs) end))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs i)) (alter! i (+ i 1))))
     out))))

(let take/first (lambda end xs (if (empty? xs) xs (do
     (let out [])
     (mut i 0)
     (while (< i end) (do (set! out (length out) (get xs i)) (alter! i (+ i 1))))
     out))))

(let take/last (lambda start xs (if (empty? xs) xs (do
     (let out [])
     (let len (length xs))
     (mut i (- len start))
     (while (< i len) (do (set! out (length out) (get xs i)) (alter! i (+ i 1))))
     out))))

(let take/while
  (lambda fn? xs
    (do
      (let out [])
      (mut i 0)
      (while (and (< i (length xs)) (fn? (get xs i))) (do
        (push! out (get xs i))
        (++ i)))
      out)))

(let drop/while
  (lambda fn? xs
    (do
      (mut i 0)
      (while (and (< i (length xs)) (fn? (get xs i))) (++ i))
      (cdr xs i))))

(let find (lambda fn? xs (do
     (mut i 0)
     (mut index -1)
     (let len (length xs))
     (while (and (< i len) (= index -1)) (if (fn? (get xs i))
              (alter! index i)
              (alter! i (+ i 1))))
     index)))

(let partition (lambda n xs (if (= n (length xs)) [xs] (do
    (let a [])
    (mut i 0)
    (let len (length xs))
    (while (< i len) (do (if (= (% i n) 0)
        (set! a (length a) [(get xs i)])
        (set! (at a -1) (length (at a -1)) (get xs i)))
        (alter! i (+ i 1))))
     a))))

(let window (lambda size xs (cond
     (empty? xs) []
     (= size (length xs)) [xs]
     (do
       (let out [])
       (let len (length xs))
       (mut i 0)
       (while (< i len) (do
         (if (<= (+ i size) len)
             (set! out (length out) (slice i (+ i size) xs)))
         (alter! i (+ i 1))))
       out))))

(let neighborhood (lambda directions y x fn xs (do
    (let len (length directions))
    (mut i 0)
    (while (< i len) (do
      (let dir (get directions i))
      (let dy (+ (first dir) y))
      (let dx (+ (get dir 1) x))
      (if (Matrix/in-bounds/raw? xs dy dx)
          (fn (get xs dy dx) dir dy dx))
      (alter! i (+ i 1))))
    nil)))

(let points (lambda fn? matrix (do
   (let coords [])
   (Matrix/for/i matrix (lambda cell y x (if (fn? cell) (do (Vector/push! coords [ y x ]) nil))))
    coords)))

(let flat-map (lambda fn xs (flat (map fn xs))))

(let intersperse (lambda x xs (if (empty? xs) [] (do
  (let out [])
  (let len (- (length xs) 1))
  (mut i 0)
  (while (< i len) (do
    (Vector/push! out (get xs i))
    (Vector/push! out x)
    (alter! i (+ i 1))))
   (Vector/push! out (get xs (- (length xs) 1)))
  out))))

(let scan (lambda fn xsi (do
  (let len (length xsi))
  (let xs (copy xsi))
  (mut i 1)
  (while (< i len) (do
    (set! xs i (fn (get xs (- i 1)) (get xs i)))
    (alter! i (+ i 1))))
  xs)))

(let map/tuple (lambda fn { a b } (fn a b)))

(let map/fst (lambda fn { a _ } (fn a)))

(let map/snd (lambda fn { _ b } (fn b)))

(let combination/n (lambda n xs (do
    (let out [])
    (letrec combinations (lambda arr size start temp
        (if (= (length temp) size)
            (set! out (length out) (copy temp))
            (do
              (mut i start)
              (let len (length arr))
              (while (< i len) (do
                    (set! temp (length temp) (get arr i))
                    (combinations arr size (+ i 1) temp)
                    (pop! temp)
                    (alter! i (+ i 1))))))))
    (combinations xs n 0 [])
    out)))

(let resolve/option (lambda fn df xs (do
  (let values [])
  (mut ok true)
  (let len (length xs))
  (mut i 0)
  (while (and ok (< i len)) (do
    (let option (get xs i))
    (if (fst option)
        (do
          (Vector/push! values (snd option))
          nil)
        (alter! ok false))
    (alter! i (+ i 1))))
  (if ok { true (fn values) } { false df }))))

(let call (lambda fn xs (fn xs)))

(let group (lambda fn xs (do
  (let out (Table/create 32))
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do
    (let item (get xs i))
    (let key (fn item))
    (let hit (Table/get/raw out key))
    (if (= (length hit) 0)
        (do (Table/set/raw! out key [item]) nil)
        (do (push! (snd (get hit 0)) item) nil))
    (alter! i (+ i 1))))
  out)))

(let autocorrect (lambda dictionary word (do
  (let f (get dictionary 0))
  (&mut best-word f)
  (mut best-dist (String/damerau-levenshtein word f))
  (mut i 1)
  (while (< i (length dictionary)) (do
    (let candidate (get dictionary i))
    (let dist (String/damerau-levenshtein word candidate))
    (if (< dist best-dist)
        (do
          (&alter! best-word candidate)
          (alter! best-dist dist)))
    (alter! i (+ i 1))))
  { (&get best-word) best-dist })))



(let filter (lambda fn? xs (if (empty? xs) xs (do
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do
            (let x (get xs i))
            (if (fn? x) (set! out (length out) x))
            (alter! i (+ i 1))))
     out))))

(let each (lambda xs fn (do (for fn xs) xs)))
(let each/i (lambda xs fn (do (for/i fn xs) xs)))


(let exclude (lambda fn? xs (filter (lambda x (not (fn? x))) xs)))
(let select filter)


(let range/int range)
(let range. range/dec)
(let expt/euler (lambda b (expt b const/dec/e)))
(let expt/euler. expt/euler)
(let expt/int expt)
(let sqrt/int sqrt)
(let expt. expt/dec)
(let sqrt. sqrt/dec)
(let odd/int? odd?)
(let even/int? even?)
(let odd.? odd/dec?)
(let even.? even/dec?)
(let one/int? one?)
(let zero/int? zero?)
(let one.? one/dec?)
(let zero.? zero/dec?)



(let each/until (lambda fn fn? xs (do (for/until fn fn? xs) xs)))
(let each/until/i (lambda fn fn? xs (do (for/until/i fn fn? xs) xs)))

(let ones/int ones)
(let zeroes/int zeroes)
(let ones. ones/dec)
(let zeroes. zeroes/dec)


(let positive/int? positive?)
(let negative/int? negative?)
(let invert/int invert)
(let negative-one/int? negative-one?)
(let divisible/int? divisible?)

(let positive.? positive/dec?)
(let negative.? negative/dec?)
(let invert. invert/dec)
(let negative-one.? negative-one/dec?)
(let divisible.? divisible/dec?)



(let max/int max)
(let min/int min)
(let max/dec max.)
(let min/dec min.)

(let maximum/int maximum)
(let minimum/int minimum)
(let maximum. maximum/dec)
(let minimum. minimum/dec)

(let gt/int? (lambda a b (> b a)))
(let lt/int? (lambda a b (< b a)))
(let gte/int? (lambda a b (>= b a)))
(let lte/int? (lambda a b (<= b a)))
(let gt/dec? (lambda a b (>. b a)))
(let lt/dec? (lambda a b (<. b a)))
(let gte/dec? (lambda a b (>=. b a)))
(let lte/dec? (lambda a b (<=. b a)))
(let gt.? gt/dec?)
(let lt.? lt/dec?)
(let gte.? gte/dec?)
(let lte.? lte/dec?)

(let gt/bool? (lambda a b (and (=? a true) (=? b false))))
(let lt/bool? (lambda a b (and (=? a false) (=? b true))))
(let and? (lambda a b (and (=? a true) (=? b true))))
(let or? (lambda a b (or (=? a true) (=? b true))))
(let not? (lambda x (not x)))

(let abs/int abs)
(let abs/dec abs.)

(let pair (lambda a b (tuple a b)))
(let product/int product)
(let product. product/dec)
(let sum/int sum)
(let sum. sum/dec)
(let avg/int avg)
(let avg. avg/dec)
(let mean/int mean)
(let median/int median)
(let mean. mean/dec)
(let median. median/dec)

(let clamp/int clamp)
(let clamp-range/int clamp-range)
(let clamp. clamp/dec)
(let clamp-range. clamp-range/dec)


(let delta/int delta)
(let delta. delta/dec)



(let count. count/dec)








(let sort (lambda fn xs (do
  (let out (copy xs))
  (Vector/sort! out fn)
  out)))

(let sort/bool/desc
  (lambda xs
    (do
      (let out [])
      (let trues (count (lambda x x) xs))
      (mut i 0)
      (while (< i trues)
        (push! out true)
        (++ i))
      (while (< i (length xs))
        (push! out false)
        (++ i))
      out)))

(let sort/bool/asc
  (lambda xs
    (do
      (let out [])
      (let falses (count (lambda x (not x)) xs))
      (mut i 0)
      (while (< i falses)
        (push! out false)
        (++ i))
      (while (< i (length xs))
        (push! out true)
        (++ i))
      out)))



(let tail (lambda xs (slice 1 (length xs) xs)))
(let head (lambda xs (slice 0 (- (length xs) 1) xs)))

(let fp/mul (lambda b a (* a b)))
(let fp/div (lambda b a (/ a b)))
(let fp/add (lambda b a (+ a b)))
(let fp/sub (lambda b a (- a b)))
(let fp/emod (lambda b a (emod a b)))
(let fp/mod (lambda b a (% a b)))

(let cond/dispatch (lambda fn? a b x (if (fn? x) a b)))
; experimental functions
(let split (lambda ys str (do
  (if (empty? ys)
      [str]
      (do
        (let out [])
        (mut i 0)
        (mut start 0)
        (let len-str (length str))
        (let len-ys (length ys))
        (while (< i len-str) (do
          (mut matched? false)
          (if (and (<= (+ i len-ys) len-str) (=# (get str i) (get ys 0)))
              (do
                (alter! matched? true)
                (mut j 1)
                (while (and matched? (< j len-ys)) (do
                  (if (not (=# (get str (+ i j)) (get ys j))) (alter! matched? false))
                  (alter! j (+ j 1))))))
          (if matched?
              (do
                (push! out (slice start i str))
                (alter! start (+ i len-ys))
                (alter! i (+ i len-ys)))
              (alter! i (+ i 1)))))
        (push! out (slice start len-str str))
        out)))))

(let join (lambda str xs (do
    (let out [])
    (mut i 0)
    (let len-xs (length xs))
    (while (< i len-xs) (do
      (let current (get xs i))
      (mut j 0)
      (while (< j (length current)) (do
        (push! out (get current j))
        (alter! j (+ j 1))))
      (if (< (+ i 1) len-xs)
          (do
            (mut k 0)
            (while (< k (length str)) (do
              (push! out (get str k))
              (alter! k (+ k 1))))))
      (alter! i (+ i 1))))
   out)))
(let join/lines (lambda xs (join ['\n'] xs)))
(let join/commas (lambda xs (join "," xs)))

(let replace (lambda a b xs (|> xs (split a) (join b))))

(let graph/project-path
  (lambda col path
    (map (lambda i (get col i)) path)))

(let graph/path->nodes
  (lambda from-col to-col path
    (if (empty? path)
        []
        (do
          (let out [(get from-col (get path 0))])
          (for (lambda i (push! out (get to-col i))) path)
          out))))

(let graph/rotate
  (lambda start xs
    (do
      (let out [])
      (mut i 0)
      (let len (length xs))
      (while (< i len) (do
        (push! out (get xs (% (+ start i) len)))
        (alter! i (+ i 1))))
      out)))

(let graph/cycle/min-rotation
  (lambda nodes
    (if (<= (length nodes) 1)
        0
        (do
          (let cycle-len (- (length nodes) 1))
          (mut best 0)
          (mut i 1)
          (while (< i cycle-len) (do
            (if (String/lt? (get nodes i) (get nodes best))
                (alter! best i)
                nil)
            (alter! i (+ i 1))))
          best))))

(let graph/normalize-cycle
  (lambda nodes
    (if (<= (length nodes) 1)
        nodes
        (do
          (let start (graph/cycle/min-rotation nodes))
          (let core (graph/rotate start (slice 0 (- (length nodes) 1) nodes)))
          (cons core [(get core 0)])))))

(let graph/normalize-path
  (lambda from-col to-col path
    (if (empty? path)
        []
        (graph/rotate (graph/cycle/min-rotation (graph/path->nodes from-col to-col path)) path))))

(let graph/cycle-key
  (lambda from-col to-col path
    (do
      (let normalized-nodes (graph/normalize-cycle (graph/path->nodes from-col to-col path)))
      (let normalized-path (graph/normalize-path from-col to-col path))
      (cons (join "->" normalized-nodes)
            "::"
            (join "," (map Integer->String normalized-path))))))

(let graph/outgoing-by
  (lambda from-col rows
    (reduce
      (lambda (a i)
        (do
          (let from (get from-col i))
          (if (Table/has? from a)
              (push! (Table/get-unsafe from a) i)
              (Table/set! a from [i]))
          a))
      (Table/new)
      rows)))

(let graph/simple-cycle?
  (lambda from-col to-col path
    (do
      (let nodes (graph/path->nodes from-col to-col path))
      (if (or (< (length nodes) 3)
              (not (match? (get nodes 0) (last nodes))))
          false
          (do
            (let seen (Set/new))
            (mut i 0)
            (mut ok true)
            (let stop (- (length nodes) 1))
            (while (and ok (< i stop)) (do
              (let node (get nodes i))
              (if (Set/has? node seen)
                  (alter! ok false)
                  (Set/add! seen node))
              (alter! i (+ i 1))))
            ok)))))

(let graph/find-cycles
  (lambda rows from-col to-col next-ok? cycle-ok?
    (do
      (let outgoing (graph/outgoing-by from-col rows))
      (let seen (Set/new))
      (let out [])
      (let contains-node?
        (lambda node nodes
          (some? (lambda x (match? x node)) nodes)))
      (letrec dfs
        (lambda origin current visited path
          (if (Table/has? current outgoing)
              (for
                (lambda next-i
                  (do
                    (let next-to (get to-col next-i))
                    (if (next-ok? path next-i)
                        (if (match? next-to origin)
                            (do
                              (let full-path (cons path [next-i]))
                              (if (and (graph/simple-cycle? from-col to-col full-path)
                                       (cycle-ok? full-path))
                                  (do
                                    (let key (graph/cycle-key from-col to-col full-path))
                                    (if (not (Set/has? key seen))
                                        (do
                                          (Set/add! seen key)
                                          (push! out full-path))
                                        nil))
                                  nil))
                            (if (not (contains-node? next-to visited))
                                (dfs origin
                                     next-to
                                     (cons visited [next-to])
                                     (cons path [next-i]))
                                nil))
                        nil)))
                (Table/get-unsafe current outgoing))
              nil)))
      (for
        (lambda start-i
          (dfs (get from-col start-i)
               (get to-col start-i)
               [(get from-col start-i) (get to-col start-i)]
               [start-i]))
        rows)
      out)))

(let graph/find-cycles/increasing-time
  (lambda rows from-col to-col time-col min-length max-span-seconds
    (graph/find-cycles
      rows
      from-col
      to-col
      (lambda (path next-i)
        (> (get time-col next-i) (get time-col (at path -1))))
      (lambda (path)
        (and (>= (length path) min-length)
             (<= (- (get time-col (at path -1))
                    (get time-col (get path 0)))
                 max-span-seconds))))))

(let graph/has-cycle?
  (lambda rows from-col to-col
    (not
      (empty?
        (graph/find-cycles
          rows
          from-col
          to-col
          (lambda (path next-i) true)
          (lambda (path) true))))))

(let floyd/cycle?
  (lambda next eq? start limit
    (do
      (&mut turtle start)
      (&mut hare start)
      (mut steps 0)
      (mut found? false)
      (while (and (not found?) (< steps limit))
        (&alter! turtle (next (&get turtle)))
        (&alter! hare (next (&get hare)))
        (&alter! hare (next (&get hare)))
        (alter! found? (eq? (&get hare) (&get turtle)))
        (++ steps))
      found?)))



(let bit/set?
  (lambda (pos n)
    (= (& n (<< 1 pos)) 0)))

(let bit/set
  (lambda (pos n)
    (| n (<< 1 pos))))

(let bit/clear
  (lambda (pos n)
    (& n (~ (<< 1 pos)))))

(let bit/power-of-two
  (lambda (n)
    (<< 2 (- n 1))))

(let bit/odd?
  (lambda (n)
    (= (& n 1) 1)))

(let bit/even?
  (lambda (n)
    (= (& n 1) 0)))

(let bit/average
  (lambda (a b)
    (>> (+ a b) 1)))

(let bit/flag-flip
  (lambda (x)
    (- 1 (* x x))))

(let bit/toggle
  (lambda (a b n)
    (^ (^ a b) n)))

(let bit/same-sign?
  (lambda (a b)
    (>= (^ a b) 0)))

(let bit/max
  (lambda (a b)
    (- a (& (- a b) (>> (- a b) 31)))))

(let bit/min
  (lambda (a b)
    (- a (& (- a b) (>> (- b a) 31)))))

(let bit/equal?
  (lambda (a b)
    (< (^ a b) 1)))

(let bit/modulo
  (lambda (divisor numerator)
    (& numerator (- divisor 1))))

(let bit/n-one?
  (lambda (nth n)
    (not (= (& n (<< 1 nth)) 0))))

(let bit/largest-power
  (lambda (n)
    (do
      (mut value n)
      (alter! value (| value (>> value 1)))
      (alter! value (| value (>> value 2)))
      (alter! value (| value (>> value 4)))
      (alter! value (| value (>> value 8)))
      (alter! value (| value (>> value 16)))
      (- value (>> value 1)))))



(let loop/repeat (lambda n fn (do
  (mut i 0)
  (while (< i n) (do
    (fn)
    (alter! i (+ i 1))))
  nil)))
(let loop/some-range? (lambda start end predicate? (do
  (mut i start)
  (mut found? false)
  (while (and (< i end) (not found?)) (do
    (if (predicate? i)
        (alter! found? true)
        (alter! i (+ i 1)))))
  found?)))

(let loop/some-n? (lambda n predicate? (loop/some-range? 0 n predicate?)))

(let swap! Vector/swap!)
(let scan! (lambda xs fn (do
  (let len (length xs))
  (if (> len 1) (do
    (mut i 1)
    (while (< i len) (do
      (set! xs i (fn (get xs (- i 1)) (get xs i)))
      (alter! i (+ i 1))))) nil)
  nil)))
(let empty! (lambda xs (do (Heap/empty! xs) nil)))
(let reverse! Vector/reverse!)
(let overwrite! Vector/overwrite!)

(let sort! Vector/sort!)

(let emod Int/euclidean-mod)
