(let for (lambda fn xs (do
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do (fn (get xs i)) (alter! i (+ i 1)))))))

(let for/i (lambda fn xs (do
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do (fn (get xs i) i) (alter! i (+ i 1)))))))

(let filter/i (lambda fn? xs (if (std/vector/empty? xs) xs (do
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

(let map (lambda fn xs (if (std/vector/empty? xs) [] (do
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do (set! out (length out) (fn (get xs i))) (alter! i (+ i 1))))
     out))))

(let map/i (lambda fn xs (if (std/vector/empty? xs) [] (do
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
    (&mut root (std/dec/sqrt (&get b)))
    (&mut frac 0.5)

    ; Loop 22. times for precision (handles bits of the fraction)
    (&mut i 0.)
    (while (<. (&get i) 22.)
      (do
        (if (>=. (&get e) (&get frac))
            (do
              (&alter! res (*. (&get res) (&get root)))
              (&alter! e (-. (&get e) (&get frac)))))
        (&alter! root (std/dec/sqrt (&get root)))
        (&alter! frac (/. (&get frac) 2.))
        (&alter! i (+. (&get i) 1.))))
    (&get res))))

(let map/adjacent (lambda fn xs (if (std/vector/empty? xs) [] (do
  (let out [])
  (mut i 1)
  (let len (length xs))
  (while (< i len) (do
    (std/vector/push! out (fn (get xs (- i 1)) (get xs i)))
    (alter! i (+ i 1))))
  out))))

(let zip-with (lambda f a b (do
    (let out [])
    (mut i 0)
    (let len (length a))
    (while (< i len) (do (set! out (length out) (f (get a i) (get b i))) (alter! i (+ i 1))))
    out)))

(let slice (lambda start end xs (if (std/vector/empty? xs) xs (do
     (let bounds (- end start))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs (+ start i))) (alter! i (+ i 1))))
     out))))

(let drop/first (lambda start xs (if (std/vector/empty? xs) xs (do
     (let end (length xs))
     (let bounds (- end start))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs (+ start i))) (alter! i (+ i 1))))
     out))))

(let drop/last (lambda end xs (if (std/vector/empty? xs) xs (do
     (let bounds (- (length xs) end))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs i)) (alter! i (+ i 1))))
     out))))

(let take/first (lambda end xs (if (std/vector/empty? xs) xs (do
     (let out [])
     (mut i 0)
     (while (< i end) (do (set! out (length out) (get xs i)) (alter! i (+ i 1))))
     out))))

(let take/last (lambda start xs (if (std/vector/empty? xs) xs (do
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
        (set! (std/vector/at a -1) (length (std/vector/at a -1)) (get xs i)))
        (alter! i (+ i 1))))
     a))))

(let window (lambda size xs (cond
     (std/vector/empty? xs) []
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
      (let dy (+ (std/vector/first dir) y))
      (let dx (+ (std/vector/second dir) x))
      (if (std/vector/three-d/in-bounds? xs dy dx)
          (fn (get xs dy dx) dir dy dx))
      (alter! i (+ i 1))))
    nil)))

(let points (lambda fn? matrix (do
   (let coords [])
   (std/vector/three-d/for/i matrix (lambda cell y x (if (fn? cell) (do (std/vector/push! coords [ y x ]) nil))))
    coords)))

(let flat-map (lambda fn xs (std/vector/flat-one (map fn xs))))

(let intersperse (lambda x xs (if (std/vector/empty? xs) [] (do
  (let out [])
  (let len (- (length xs) 1))
  (mut i 0)
  (while (< i len) (do
    (std/vector/push! out (get xs i))
    (std/vector/push! out x)
    (alter! i (+ i 1))))
   (std/vector/push! out (get xs (- (length xs) 1)))
  out))))

(let scan (lambda fn xsi (do
  (let len (length xsi))
  (let xs (std/vector/copy xsi))
  (mut i 1)
  (while (< i len) (do
    (std/vector/update! xs i (fn (get xs (- i 1)) (get xs i)))
    (alter! i (+ i 1))))
  xs)))

(let map/tuple (lambda fn { a b } (fn a b)))

(let map/fst (lambda fn { a _ } (fn a)))

(let map/snd (lambda fn { _ b } (fn b)))

(let combination/n (lambda n xs (do
    (let out [])
    (letrec combinations (lambda arr size start temp
        (if (= (length temp) size)
            (set! out (length out) (std/vector/copy temp))
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
          (std/vector/push! values (snd option))
          nil)
        (alter! ok false))
    (alter! i (+ i 1))))
  (if ok { true (fn values) } { false df }))))

(let call (lambda fn xs (fn xs)))

(let group (lambda fn xs (do
  (let out (std/vector/hash/table 32))
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do
    (let item (get xs i))
    (let key (fn item))
    (let hit (std/vector/hash/table/get out key))
    (if (= (length hit) 0)
        (do (std/vector/hash/table/set! out key [item]) nil)
        (do (push! (snd (get hit 0)) item) nil))
    (alter! i (+ i 1))))
  out)))

(let autocorrect (lambda dictionary word (do
  (let f (get dictionary 0))
  (&mut best-word f)
  (mut best-dist (std/vector/char/damerau-levenshtein word f))
  (mut i 1)
  (while (< i (length dictionary)) (do
    (let candidate (get dictionary i))
    (let dist (std/vector/char/damerau-levenshtein word candidate))
    (if (< dist best-dist)
        (do
          (&alter! best-word candidate)
          (alter! best-dist dist)))
    (alter! i (+ i 1))))
  { (&get best-word) best-dist })))


(let const std/fn/const)
(let floor std/dec/floor)
(let ceil std/dec/ceil)

(let extreme std/vector/int/extreme)
(let filter (lambda fn? xs (if (std/vector/empty? xs) xs (do
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do
            (let x (get xs i))
            (if (fn? x) (set! out (length out) x))
            (alter! i (+ i 1))))
     out))))

(let each (lambda xs fn (do (for fn xs) xs)))
(let each/i (lambda xs fn (do (for/i fn xs) xs)))

(let transpose std/vector/three-d/rotate)
(let interleave std/vector/two-d/interleave)
(let empty? std/vector/empty?)
(let not-empty? std/vector/not-empty?)

(let exclude (lambda fn? xs (filter (lambda x (not (fn? x))) xs)))
(let select filter)

(let reverse std/vector/reverse)

(let range std/vector/int/range)
(let range/int std/vector/int/range)
(let range/inclusive std/vector/int/range/inclusive)
(let range/exclusive std/vector/int/range/exclusive)
(let range/dec std/vector/dec/range)
(let range. range/dec)
(let square std/int/square)
(let expt/euler (lambda b (expt b const/dec/e)))
(let expt/euler. expt/euler)
(let sqrt std/int/sqrt)
(let expt/int expt)
(let sqrt/int std/int/sqrt)
(let sqrt/dec std/dec/sqrt)
(let expt. expt/dec)
(let sqrt. sqrt/dec)
(let log std/dec/log)
(let odd? std/int/odd?)
(let even? std/int/even?)
(let odd/int? std/int/odd?)
(let even/int? std/int/even?)
(let odd/dec? std/dec/odd?)
(let even/dec? std/dec/even?)
(let odd.? odd/dec?)
(let even.? even/dec?)
(let one? std/int/one?)
(let zero? std/int/zero?)
(let one/int? std/int/one?)
(let zero/int? std/int/zero?)
(let one/dec? std/dec/one?)
(let zero/dec? std/dec/zero?)
(let one.? one/dec?)
(let zero.? zero/dec?)


(let space? std/char/space?)

(let each/until (lambda fn fn? xs (do (for/until fn fn? xs) xs)))
(let each/until/i (lambda fn fn? xs (do (for/until/i fn fn? xs) xs)))

(let ones std/vector/int/ones)
(let zeroes std/vector/int/zeroes)
(let ones/int std/vector/int/ones)
(let zeroes/int std/vector/int/zeroes)
(let ones/dec std/vector/dec/ones)
(let zeroes/dec std/vector/dec/zeroes)
(let ones. ones/dec)
(let zeroes. zeroes/dec)
(let truths std/vector/bool/true)
(let falses std/vector/bool/false)

(let positive? std/int/positive?)
(let negative? std/int/negative?)
(let invert std/int/invert)
(let negative-one? std/int/negative-one?)

(let positive/int? std/int/positive?)
(let negative/int? std/int/negative?)
(let invert/int std/int/invert)
(let negative-one/int? std/int/negative-one?)
(let divisible/int? divisible?)

(let positive/dec? std/dec/positive?)
(let negative/dec? std/dec/negative?)
(let invert/dec std/dec/invert)
(let negative-one/dec? std/dec/negative-one?)
(let positive.? positive/dec?)
(let negative.? negative/dec?)
(let invert. invert/dec)
(let negative-one.? negative-one/dec?)
(let divisible.? divisible/dec?)

(let upper std/char/upper)
(let lower std/char/lower)
(let match? std/vector/char/equal?)

(let digit? std/char/digit?)
(let fill std/vector/two-d/fill)

(let max std/int/max)
(let min std/int/min)
(let max. std/dec/max)
(let min. std/dec/min)
(let max/int std/int/max)
(let min/int std/int/min)
(let max/dec std/dec/max)
(let min/dec std/dec/min)

(let maximum std/vector/int/maximum)
(let minimum std/vector/int/minimum)
(let maximum/int std/vector/int/maximum)
(let minimum/int std/vector/int/minimum)
(let maximum/dec std/vector/dec/maximum)
(let minimum/dec std/vector/dec/minimum)
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

(let abs std/int/abs)
(let abs. std/dec/abs)
(let abs/int std/int/abs)
(let abs/dec std/dec/abs)

(let first std/vector/first)
(let last std/vector/last)
(let pair (lambda a b (tuple a b)))
(let product std/vector/int/product)
(let product/int std/vector/int/product)
(let product/dec std/vector/dec/product)
(let product. product/dec)
(let sum std/vector/int/sum)
(let sum/int std/vector/int/sum)
(let sum/dec std/vector/dec/sum)
(let sum/bool std/vector/bool/sum)
(let sum. sum/dec)
(let avg std/int/average)
(let avg/int std/int/average)
(let avg/dec std/dec/average)
(let avg. avg/dec)
(let mean std/vector/int/mean)
(let median std/vector/int/median)
(let mean/int std/vector/int/mean)
(let mean/dec std/vector/dec/mean)
(let median/int std/vector/int/median)
(let median/dec std/vector/dec/median)
(let mean. mean/dec)
(let median. median/dec)
(let zip std/vector/tuple/zip)
(let unzip std/vector/tuple/unzip)

(let flat std/vector/flat-one)
(let enumerate std/vector/enumerate)
(let clamp/int clamp)
(let clamp-range/int clamp-range)
(let clamp. clamp/dec)
(let clamp-range. clamp-range/dec)

(let at std/vector/at)
(let cycle std/vector/cycle)
(let replicate std/vector/replicate)
(let cartesian-product std/vector/cartesian-product)
(let lcm std/int/lcm)
(let gcd std/int/gcd)

(let delta std/int/delta)
(let delta/int std/int/delta)
(let delta/dec std/dec/delta)
(let delta. delta/dec)


(let buckets std/vector/buckets)

(let count. count/dec)


(let unique/int std/vector/int/unique)
(let unique/char std/vector/char/unique)

(let permutation std/vector/permutations)
(let combination/pairs std/vector/tuple/unique-pairs)
(let combination std/vector/combinations)
(let subset std/vector/subset)


(let in-bounds? std/vector/in-bounds?)


(let true/option std/true/option)
(let false/option std/false/option)

(let copy std/vector/copy)
(let sort (lambda fn xs (do
  (let out (std/vector/copy xs))
  (std/vector/sort! out fn)
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

(let neighborhood/moore std/vector/three-d/moore-neighborhood)
(let neighborhood/diagonal std/vector/three-d/diagonal-neighborhood)
(let neighborhood/kernel std/vector/three-d/kernel-neighborhood)
(let neighborhood/von-neumann std/vector/three-d/von-neumann-neighborhood)


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
(let split/lines std/vector/char/lines)
(let split/words std/vector/char/words)
(let split/commas std/vector/char/commas)

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
            (if (std/vector/char/lesser? (get nodes i) (get nodes best))
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

(let trim std/vector/char/trim)
(let trim/left std/vector/char/trim/left)
(let trim/right std/vector/char/trim/right)

(let shoelace std/int/shoelace)
(let sequence std/vector/int/sequence)
(let collinear? std/int/collinear?)

(let bit/set?
  (lambda (pos n)
    (std/int/bit/set? n pos)))

(let bit/set
  (lambda (pos n)
    (std/int/bit/set n pos)))

(let bit/clear
  (lambda (pos n)
    (std/int/bit/clear n pos)))

(let bit/power-of-two
  (lambda (n)
    (std/int/bit/power-of-two n)))

(let bit/odd?
  (lambda (n)
    (std/int/bit/odd? n)))

(let bit/even?
  (lambda (n)
    (std/int/bit/even? n)))

(let bit/average
  (lambda (a b)
    (std/int/bit/average a b)))

(let bit/flag-flip
  (lambda (x)
    (std/int/bit/flag-flip x)))

(let bit/toggle
  (lambda (a b n)
    (std/int/bit/toggle n a b)))

(let bit/same-sign?
  (lambda (a b)
    (std/int/bit/same-sign? a b)))

(let bit/max
  (lambda (a b)
    (std/int/bit/max a b)))

(let bit/min
  (lambda (a b)
    (std/int/bit/min a b)))

(let bit/equal?
  (lambda (a b)
    (std/int/bit/equal? a b)))

(let bit/modulo
  (lambda (divisor numerator)
    (std/int/bit/modulo numerator divisor)))

(let bit/n-one?
  (lambda (nth n)
    (std/int/bit/n-one? n nth)))

(let bit/largest-power
  (lambda (n)
    (std/int/bit/largest-power n)))
