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
