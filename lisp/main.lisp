(let true 0)
(let false 1)

(let append! (lambda q item (set! q (length q) item)))
(let set-and-get! (lambda q index item (do (set! q index item) item)))
(let tail! (lambda q (del! q)))
(let push! (lambda q item (do (set! q (length q) item) item)))
(let pop! (lambda q (do (let l (at q -1)) (del! q) l)))

(let of (lambda xs cb (do
      (let i [ 0 ])
      (let len (length xs))
      (let process (lambda (do (cb) (set! i 0 (+ (get i 0) 1)))))
      (loop (< (get i 0) len) (process))
      xs)))

(let iterate (lambda xs cb (do
      (let i [ 0 ])
      (let len (length xs))
      (let process (lambda (do (cb (get i 0)) (set! i 0 (+ (get i 0) 1)))))
      (loop (< (get i 0) len) (process))
      xs)))

(let for (lambda xs cb (do
      (let i [ 0 ])
      (let len (length xs))
      (let process (lambda (do (cb (get xs (get i 0))) (set! i 0 (+ (get i 0) 1)))))
      (loop (< (get i 0) len) (process))
      xs)))

(let map (lambda xs cb (do
      (let out [])
      (for xs (lambda x (push! out (cb x))))
      out)))

(let filter (lambda xs cb? (do
      (let out [])
      (for xs (lambda x (if (cb? x) (push! out x))))
      out)))

(let reduce (lambda xs cb initial (do
      (let out [ initial ])
      (for xs (lambda x (set! out 0 (cb (get out 0) x))))
      (get out 0))))


(let every? (lambda xs predicate? (do
            (let i [ 0 ])
            (let len (length xs))
            (loop (and (< (get i 0) len) (predicate? (get xs (get i 0)))) (set! i 0 (+ (get i 0) 1)))
            (not (> len (get i 0))))))

(let some? (lambda xs predicate? (do
            (let i [ 0 ])
            (let len (length xs))
            (loop (and (< (get i 0) len) (not (predicate? (get xs (get i 0))))) (set! i 0 (+ (get i 0) 1)))
            (not (= (> len (get i 0)) 0)))))

(let find-index (lambda xs cb? (do
      (let i [ 0 ])
      (let index [ -1 ])
      (let len (length xs))
      (let process (lambda 
            (if (cb? (get xs (get i 0)))
               (set! index 0 (get i 0))
               (set! i 0 (+ (get i 0) 1)))))
      (loop (and (< (get i 0) len) (= (get index 0) -1)) (process))
      (get index 0))))

(let square (lambda x (* x x)))
(let even? (lambda x (= (mod x 2) 0)))
(let odd? (lambda x (not (= (mod x 2) 0))))
(let summation (lambda xs (reduce xs (lambda a b (+ a b)) 0)))
(let product (lambda xs (reduce xs (lambda a b (* a b)) 1)))
(let euclidean-mod (lambda a b (mod (+ (mod a b) b) b)))
(let max (lambda a b (if (> a b) a b)))
(let min (lambda a b (if (< a b) a b)))
(let maximum (lambda xs (reduce xs max (get xs 0))))
(let minimum (lambda xs (reduce xs min (get xs 0))))
(let normalize (lambda value min max (* (- value min) (/ (- max min)))))
(let linear-interpolation (lambda a b n (+ (* (- 1 n) a) (* n b))))
(let gauss-sum (lambda n (/ (* n (+ n 1)) 2)))
(let gauss-sum-sequance (lambda a b (/ (* (+ a b) (+ (- b a) 1)) 2)))
(let clamp (lambda x limit (if (> x limit) limit x)))
(let clamp-range (lambda x start end (cond (> x end) end (< x start) start (*) x)))

(let empty? (lambda xs (= (length xs) 0)))
(let not-empty? (lambda xs (not (= (length xs) 0))))
(let count-of (lambda xs cb? (length (filter xs cb?))))
(let count (lambda input item (count-of input (lambda x (= x item)))))
(let empty! (lambda xs (of xs (lambda (pop! xs)))))
(let in-bounds? (lambda xs index (and (< index (length xs)) (>= index 0))))

(|> [ 1 2 3 4 ] (filter even?) (map square) (summation))