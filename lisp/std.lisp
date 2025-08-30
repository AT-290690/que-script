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

(let append! (lambda q item (set! q (length q) item)))
(let set-and-get! (lambda q index item (do (set! q index item) item)))
(let tail! (lambda q (pop! q)))
(let pop-and-get! (lambda xs! (do 
      (let last (get xs! (- (length xs!) 1))) 
      (pop! xs!) 
      last)))
(let push! (lambda q item (do (set! q (length q) item) item)))
(let at (lambda xs i (if (< i 0) (get xs (+ (length xs) i)) (get xs i))))
(let first (lambda xs (get xs 0)))
(let second (lambda xs (get xs 1)))
(let third (lambda xs (get xs 3)))
(let last (lambda xs (get xs (- (length xs) 1))))
(let variable (lambda value [ value ]))
(let set (lambda var x (set! var 0 x)))
(let += (lambda var n (set var (+ (get var) n))))
(let -= (lambda var n (set var (- (get var) n))))
(let ++ (lambda var (set var (+ (get var) 1))))
(let -- (lambda var (set var (- (get var) 1))))

(let of (lambda xs cb (do
      (let process (lambda . (cb)))
      (dotimes 0 (length xs) process)
      xs)))

(let iterate (lambda xs cb (do
      (let process (lambda i (cb i)))
      (dotimes 0 (length xs) process)
      xs)))

(let each (lambda xs cb (do
      (let process (lambda i (cb (get xs i))))
      (dotimes 0 (length xs) process)
      xs)))

(let map (lambda xs cb (do
      (let out! [])
      (each xs (lambda x (push! out! (cb x))))
      out!)))

(let filter (lambda xs cb? (do
      (let out [])
      (each xs (lambda x (if (cb? x) (push! out x))))
      out)))

(let reduce (lambda xs cb initial (do
      (let out [ initial ])
      (each xs (lambda x (set! out 0 (cb (get out 0) x))))
      (get out))))


(let every? (lambda xs predicate? (do
            (let i [ 0 ])
            (let len (length xs))
            (loop (and (< (get i) len) (predicate? (get xs (get i)))) (set! i 0 (+ (get i) 1)))
            (not (> len (get i))))))

(let some? (lambda xs predicate? (do
            (let i [ 0 ])
            (let len (length xs))
            (loop (and (< (get i) len) (not (predicate? (get xs (get i))))) (set! i 0 (+ (get i) 1)))
            (not (= (> len (get i)) 0)))))

(let find-index (lambda xs cb? (do
      (let i [ 0 ])
      (let index [ -1 ])
      (let len (length xs))
      (let process (lambda 
            (if (cb? (get xs (get i)))
               (set! index 0 (get i))
               (set! i 0 (+ (get i) 1)))))
      (loop (and (< (get i) len) (= (get index 0) -1)) (process))
      (get index 0))))

(let merge! (lambda a b (do (each b (lambda x (push! a x))) a)))
(let merge (lambda a b (do (let out []) (each a (lambda x (push! out x))) (each b (lambda x (push! out x))) out)))
(let concat (lambda xs (reduce xs merge [])))

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

(let range (lambda start end (do
      (let out [])
      (dotimes start (+ end 1) (lambda i (push! out i)))
      out)))

(let sequence (lambda arr (do
      (let out [])
      (let end (length arr))
      (dotimes 0 end (lambda i (push! out i)))
      out)))

(let buckets (lambda size (do
      (let out [])
      (dotimes 0 size (lambda . (push! out [])))
      out)))

(let zeroes (lambda start end (do
      (let out [])
      (dotimes start (+ end 1) (lambda . (push! out 0)))
      out)))

(let ones (lambda start end (do
      (let out [])
      (dotimes start (+ end 1) (lambda . (push! out 1)))
      out)))

(let match? (lambda a b (and (= (length a) (length b)) (|>
   a
   (ziper b)
   (every? (lambda x (= (get x 0) (get x 1))))))))

(let ziper (lambda a b (do 
      (let out [])
      (iterate a (lambda i (push! out [(get a i)])))
      (iterate b (lambda i (push! (get out i) (get b i))))
      out)))

(let zip (lambda xs (ziper (first xs) (second xs))))
(let unzip (lambda xs (array (map xs first) (map xs second))))
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

      (loop (< (get i) bounds) (process))
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
      (if (not (= index -1)) (apply (lambda (do (set! current index (at current -1)) (pop! current)))) nil)
      table)))

(let set-property! (lambda table key value
        (do
          (let idx (hash table key))
          (if (not (in-bounds? table idx)) (set! table idx []) [])
          (let current (get table idx))
          (let len (length current))
          (let index (if (> len 0) (find-index current (lambda x (match? (get x 0) key))) -1))
          (let entry [ key value ])
          (if (= index -1)
            (set! current (length current) entry)
            (set! current index entry))
          table)))
          
(let delete-property! (lambda table key
      (do
        (let idx (hash table key))
        (if (not (in-bounds? table idx)) (set! table idx []) [])
        (let current (get table idx))
        (let len (length current))
        (let index (if (> len 0) (find-index current (lambda x (match? (get x 0) key))) -1))
        (if (not (= index -1)) (do (set! current index (at current -1)) (pop! current)) nil)
        table)))

(let access-property-helper (lambda table idx key (do 
    (let current (get table idx))
    (let found-index (find-index current (lambda x (match? key (get x 0)))))
    (unless (= found-index -1) (get (get current found-index) 1)))))

(let access-property (lambda table key (do
      (let idx (hash table key))
      (if (in-bounds? table idx) (access-property-helper table idx key)))))

(let swap! (lambda xs i j (do (let temp (get xs i)) (set! xs i (get xs j)) (set! xs j temp))))

(let sort-partition! (lambda arr start end cb (do 
      (let pivot (get arr end))
      (let i [(- start 1)])
      (let j [ start ])

      (let helper (lambda i j (do 
           (set! i 0 (+ (get i) 1))
           (swap! arr (get i) (get j)))))

      (let process (lambda (do 
            (if (cb (get arr (get j)) pivot) (helper i j) [])
            (set! j 0 (+ (get j) 1)))))
      (loop (< (get j) end) (process))

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
                  (push! stack end))))
            (if (< start end) (helper) []))))
      (loop (> (length stack) 0) (process))
      arr)))

(let char->digit (lambda ch (- ch char:0)))
(let chars->digits (lambda digits (map digits char->digit)))
(let digit->char (lambda digit (+ digit char:0)))
(let digits->chars (lambda digits (map digits digit->char)))

(let array->string (lambda xs delim (reduce (ziper xs (sequence xs)) (lambda a b (if (> (second b) 0) (merge! (append! a delim) (first b)) (first b))) [])))

(let string->array (lambda str char (|> str
              (reduce (lambda a b (do
              (let prev (at a -1))
                (if (match? (array b) (array char))
                    (push! a [])
                    (push! prev b)) a))
              (array []))
              (map (lambda x (array->string (array x) char:empty))))))

(let slice (lambda xs start end (do
      (let bounds (- end start))
      (let out [])
      (let process (lambda i (push! out (get xs (+ start i)))))
      (dotimes 0 bounds process)
      out)))

(let true? (lambda x (= (get x) true)))
(let false? (lambda x (= (get x) false)))
(let true! (lambda x (set! x 0 true)))
(let false! (lambda x (set! x 0 false)))

(let reverse (lambda xs (do 
      (let out [])
      (let len (length xs))
      (let process (lambda i (push! out (get xs (- len i 1)))))
      (dotimes 0 len process)
      out)))

(let trim-left (lambda str (do
  (let tr (variable true))
  (|> str (reduce (lambda a b (if
  (and (true? tr) (or (= b char:space) (= b char:new-line))) a
    (apply (lambda (do
      (if (true? tr) (false! tr) nil)
      (merge a (array b))))))) [])))))

(let trim-right (lambda str (do
  (let tr (variable true))
  (|> str (reverse) (reduce (lambda a b 
    (if (and (true? tr) (or (= b char:space) (= b char:new-line))) a
    (apply (lambda (do
      (if (true? tr) (false! tr) nil)
      (merge (array b) a)))))) [])))))

(let trim (lambda str (|> str (trim-left) (trim-right))))

