(let &box (lambda value [ value ]))
(let &alter! (lambda vrbl x (set! vrbl 0 x)))
(let &get (lambda vrbl (get vrbl 0)))
(let push! (lambda xs x (set! xs (length xs) x)))

(let infinity 2147483647)
(let -infinity -2147483648)
(let identity (lambda x x))
(let Int 0)
(let Float 0.0)
(let Char 'a')
(let Bool false)
(let Nil nil)
(let as (lambda . t t))
(let cons (lambda a b (do 
  (let lena (length a))
  (let lenb (length b))
  (cond (= lena 0) b (= lenb 0) a (do 
  (let out []) 
  (mut i 0)
  (while (< i lena) (do (set! out (length out) (get a i)) (alter! i (+ i 1))))
  (alter! i 0)
  (while (< i lenb) (do (set! out (length out) (get b i)) (alter! i (+ i 1))))
  out)))))
(let Char/empty (get (string 0) 0))
(let Char/double-quote (get (string 34) 0))
(let Char/single-quote (get "'" 0))
(let Char/new-line (get (string 10) 0))

(let Bool->Int (lambda x (if (=? x true) 1 0)))
(let Bool->Char (lambda x (if (=? x true) '1' '0')))
(let Char->Int (lambda x (if (>=# x Char/empty) (as x Int) 0)))
(let Char->Bool (lambda x (if (or (=# x Char/empty) (=# x '0')) false true)))
(let Int->Bool (lambda x 
    (cond 
        (<= x 0) false
        (>= x 1) true
        false)))
(let Int->Char (lambda x (if (>= x 0) (as x Char) Char/empty)))

(let String/append! (lambda out xs (do
  (mut i 0)
  (while (< i (length xs)) (do
    (set! out (length out) (get xs i))
    (alter! i (+ i 1))))
  nil)))

(let String->Int (lambda s
  (if (= (length s) 0)
      0
      (do
        (mut i 0)
        (mut sign 1)
        (if (=# (get s 0) '-')
            (do
              (alter! sign -1)
              (alter! i 1))
            (if (=# (get s 0) '+')
                (alter! i 1)
                nil))
        (mut acc 0)
        (while (< i (length s)) (do
          (let ch (get s i))
          (if (and (>=# ch '0') (<=# ch '9'))
              (do
                (let d (- (Char->Int ch) 48))
                (let next (+ (* acc 10) d))
                (alter! acc next))
              nil)
          (alter! i (+ i 1))))
        (* sign acc)))))

(let Int->String (lambda n (do
  (if (= n 0)
      "0"
      (do
        (mut x n)
        (mut neg false)
        (if (< x 0)
            (do
              (alter! neg true)
              (alter! x (- 0 x)))
            nil)

        (let rev [])
        (while (> x 0) (do
          (let d (mod x 10))
          (set! rev (length rev) (+# (Int->Char d) '0'))
          (alter! x (/ x 10))))

        (let out [])
        (if neg (set! out (length out) '-') nil)

        (mut i (- (length rev) 1))
        (while (>= i 0) (do
          (set! out (length out) (get rev i))
          (alter! i (- i 1))))
        out)))))

(let String->Float (lambda s
  (if (= (length s) 0)
      0.0
      (do
        (mut i 0)
        (mut neg false)
        (if (=# (get s 0) '-')
            (do
              (alter! neg true)
              (alter! i 1))
            (if (=# (get s 0) '+')
                (alter! i 1)
                nil))

        (mut int-part 0)
        (while (and (< i (length s)) (not (=# (get s i) '.'))) (do
          (let ch (get s i))
          (if (and (>=# ch '0') (<=# ch '9'))
              (do
                (let d (- (Char->Int ch) 48))
                (let next (+ (* int-part 10) d))
                (alter! int-part next))
              nil)
          (alter! i (+ i 1))))

        (mut frac-part 0)
        (mut frac-base 1)
        (if (and (< i (length s)) (=# (get s i) '.'))
            (do
              (alter! i (+ i 1))
              (while (< i (length s)) (do
                (let ch (get s i))
                (if (and (>=# ch '0') (<=# ch '9'))
                    (do
                      (let d (- (Char->Int ch) 48))
                      (let next (+ (* frac-part 10) d))
                      (alter! frac-part next)
                      (alter! frac-base (* frac-base 10)))
                    nil)
                (alter! i (+ i 1)))))
            nil)

        (let ip int-part)
        (let fp frac-part)
        (let fb frac-base)
        (let value (+. (Int->Float ip) (/. (Int->Float fp) (Int->Float fb))))
        (if neg (-. 0.0 value) value)))))

(let Float->String (lambda x (do
  (mut neg false)
  (mut v x)
  (if (<. v 0.0)
      (do
        (alter! neg true)
        (alter! v (-. 0.0 v)))
      nil)

  (let whole (Float->Int v))
  (let frac (-. v (Int->Float whole)))

  (let body [])
  (String/append! body (Int->String whole))

  (let final-body
    (if (>. frac 0.0)
        (do
          (set! body (length body) '.')
          (mut f frac)
          (mut k 0)
          (while (< k 6) (do
            (alter! f (*. f 10.0))
            (let d (Float->Int f))
            (set! body (length body) (+# (Int->Char d) '0'))
            (alter! f (-. f (Int->Float d)))
            (alter! k (+ k 1))))

          (mut end (- (length body) 1))
          (while (and (> end 0) (=# (get body end) '0')) (do
            (alter! end (- end 1))))
          (if (=# (get body end) '.')
              (alter! end (- end 1))
              nil)

          (let trimmed [])
          (mut i 0)
          (while (<= i end) (do
            (set! trimmed (length trimmed) (get body i))
            (alter! i (+ i 1))))
          trimmed)
        body))

  (if neg
      (do
        (let out ['-'])
        (String/append! out final-body)
        out)
      final-body))))


(let map (lambda fn xs (if (= (length xs) 0) [] (do
    (let out [])
    (mut i 0)
    (while (< i (length xs)) (do
      (set! out (length out) (fn (get xs i)))
      (alter! i (+ i 1))))
    out))))

  (let map/i (lambda fn xs (if (= (length xs) 0) [] (do
    (let out [])
    (mut i 0)
    (while (< i (length xs)) (do
      (set! out (length out) (fn (get xs i) i))
      (alter! i (+ i 1))))
    out))))

  (let filter (lambda fn? xs (if (= (length xs) 0) xs (do
    (let out [])
    (mut i 0)
    (while (< i (length xs)) (do
      (let x (get xs i))
      (if (fn? x) (set! out (length out) x))
      (alter! i (+ i 1))))
    out))))

  (let filter/i (lambda fn? xs (if (= (length xs) 0) xs (do
    (let out [])
    (mut i 0)
    (while (< i (length xs)) (do
      (let x (get xs i))
      (if (fn? x i) (set! out (length out) x))
      (alter! i (+ i 1))))
    out))))

  (let select (lambda fn? xs (filter fn? xs)))
  (let exclude (lambda fn? xs (filter (lambda x (not (fn? x))) xs)))

  (let reduce (lambda fn init xs (do
    (mut out init)
    (mut i 0)
    (while (< i (length xs)) (do
      (alter! out (fn out (get xs i)))
      (alter! i (+ i 1))))
    out)))

  (let reduce/i (lambda fn init xs (do
    (mut out init)
    (mut i 0)
    (while (< i (length xs)) (do
      (alter! out (fn out (get xs i) i))
      (alter! i (+ i 1))))
    out)))

  (let reduce/until (lambda fn fn? init xs (do
    (mut out init)
    (mut placed false)
    (mut i 0)
    (let len (length xs))
    (while (and (not placed) (< i len)) (do
      (let x (get xs i))
      (let a out)
      (if (fn? a x)
          (alter! placed true)
          (alter! out (fn a x)))
      (alter! i (+ i 1))))
    out)))

  (let reduce/until/i (lambda fn fn? init xs (do
    (mut out init)
    (mut placed false)
    (mut i 0)
    (let len (length xs))
    (while (and (not placed) (< i len)) (do
      (let idx i)
      (let x (get xs idx))
      (let a out)
      (if (fn? a x idx)
          (alter! placed true)
          (alter! out (fn a x idx)))
      (alter! i (+ i 1))))
    out)))

  (let sum (lambda xs (reduce (lambda a b (+ a b)) 0 xs)))
  (let sum/int (lambda xs (sum xs)))
  (let sum/float (lambda xs (reduce (lambda a b (+. a b)) 0.0 xs)))

  (let product (lambda xs (reduce (lambda a b (* a b)) 1 xs)))
  (let product/int (lambda xs (product xs)))
  (let product/float (lambda xs (reduce (lambda a b (*. a b)) 1.0 xs)))

  (let mean (lambda xs (/ (sum/int xs) (length xs))))
  (let mean/int (lambda xs (mean xs)))
  (let mean/float (lambda xs (/. (sum/float xs) (Int->Float (length xs)))))

  (let every? (lambda fn? xs (do
    (mut i 0)
    (let len (length xs))
    (while (and (< i len) (fn? (get xs i))) (alter! i (+ i 1)))
    (not (> len i)))))

  (let some? (lambda fn? xs (do
    (mut i 0)
    (let len (length xs))
    (while (and (< i len) (not (fn? (get xs i)))) (alter! i (+ i 1)))
    (or (= len 0) (> len i)))))

  (let every/i? (lambda fn? xs (do
    (mut i 0)
    (let len (length xs))
    (while (and (< i len) (fn? (get xs i) i)) (alter! i (+ i 1)))
    (not (> len i)))))

  (let some/i? (lambda fn? xs (do
    (mut i 0)
    (let len (length xs))
    (while (and (< i len) (not (fn? (get xs i) i))) (alter! i (+ i 1)))
    (or (= len 0) (> len i)))))

  (let find (lambda fn? xs (do
    (mut i 0)
    (mut index -1)
    (let len (length xs))
    (while (and (< i len) (= index -1)) (if (fn? (get xs i))
      (alter! index i)
      (alter! i (+ i 1))))
    index)))

  (let range/int (lambda start end (do
    (let out [start])
    (mut i (+ start 1))
    (while (< i (+ end 1)) (do
      (set! out (length out) i)
      (alter! i (+ i 1))))
    out)))
  (let range (lambda start end (range/int start end)))
  (let range/float (lambda start end (do
    (let out [(Int->Float start)])
    (mut i (+ start 1))
    (while (< i (+ end 1)) (do
      (set! out (length out) (Int->Float i))
      (alter! i (+ i 1))))
    out)))

  (let slice (lambda start end xs (if (= (length xs) 0) xs (do
    (let bounds (- end start))
    (let out [])
    (mut i 0)
    (while (< i bounds) (do
      (set! out (length out) (get xs (+ start i)))
      (alter! i (+ i 1))))
    out))))

  (let take/first (lambda n xs (if (= (length xs) 0) xs (do
    (let out [])
    (mut i 0)
    (while (< i n) (do
      (set! out (length out) (get xs i))
      (alter! i (+ i 1))))
    out))))

  (let drop/first (lambda n xs (if (= (length xs) 0) xs (do
    (let end (length xs))
    (let bounds (- end n))
    (let out [])
    (mut i 0)
    (while (< i bounds) (do
      (set! out (length out) (get xs (+ n i)))
      (alter! i (+ i 1))))
    out))))

  (let drop/last (lambda n xs (if (= (length xs) 0) xs (do
    (let bounds (- (length xs) n))
    (let out [])
    (mut i 0)
    (while (< i bounds) (do
      (set! out (length out) (get xs i))
      (alter! i (+ i 1))))
    out))))

  (let take/last (lambda n xs (if (= (length xs) 0) xs (do
    (let out [])
    (let len (length xs))
    (mut i (- len n))
    (while (< i len) (do
      (set! out (length out) (get xs i))
      (alter! i (+ i 1))))
    out))))

  (let zip (lambda xs (do
    (let a (fst xs))
    (let b (snd xs))
    (if (= (length a) 0)
        []
        (do
          (mut i 1)
          (let len (length a))
          (let out [{ (get a 0) (get b 0) }])
          (while (< i len) (do
            (set! out (length out) { (get a i) (get b i) })
            (alter! i (+ i 1))))
          out)))))

  (let unzip (lambda xs { (map fst xs) (map snd xs) }))

  (let window (lambda n xs (cond
    (= (length xs) 0) []
    (= n (length xs)) [xs]
    (reduce/i (lambda a b i (if (> (+ i n) (length xs)) a (do
      (set! a (length a) (slice i (+ i n) xs))
      a))) [] xs))))

  (let flat (lambda xs (cond
    (= (length xs) 0) []
    (= (length xs) 1) (get xs 0)
    (reduce (lambda a b (do
      (mut i 0)
      (while (< i (length b)) (do
        (set! a (length a) (get b i))
        (alter! i (+ i 1))))
      a)) [] xs))))

(let flat-map (lambda fn xs (flat (map fn xs))))