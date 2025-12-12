(let std/char/A (get "A"))
(let std/char/B (get "B"))
(let std/char/C (get "C"))
(let std/char/D (get "D"))
(let std/char/E (get "E"))
(let std/char/F (get "F"))
(let std/char/G (get "G"))
(let std/char/H (get "H"))
(let std/char/I (get "I"))
(let std/char/J (get "J"))
(let std/char/K (get "K"))
(let std/char/L (get "L"))
(let std/char/M (get "M"))
(let std/char/N (get "N"))
(let std/char/O (get "O"))
(let std/char/P (get "P"))
(let std/char/Q (get "Q"))
(let std/char/R (get "R"))
(let std/char/S (get "S"))
(let std/char/T (get "T"))
(let std/char/U (get "U"))
(let std/char/V (get "V"))
(let std/char/W (get "W"))
(let std/char/X (get "X"))
(let std/char/Y (get "Y"))
(let std/char/Z (get "Z"))
(let std/char/a (get "a"))
(let std/char/b (get "b"))
(let std/char/c (get "c"))
(let std/char/d (get "d"))
(let std/char/e (get "e"))
(let std/char/f (get "f"))
(let std/char/g (get "g"))
(let std/char/h (get "h"))
(let std/char/i (get "i"))
(let std/char/j (get "j"))
(let std/char/k (get "k"))
(let std/char/l (get "l"))
(let std/char/m (get "m"))
(let std/char/n (get "n"))
(let std/char/o (get "o"))
(let std/char/p (get "p"))
(let std/char/q (get "q"))
(let std/char/r (get "r"))
(let std/char/s (get "s"))
(let std/char/t (get "t"))
(let std/char/u (get "u"))
(let std/char/v (get "v"))
(let std/char/w (get "w"))
(let std/char/x (get "x"))
(let std/char/y (get "y"))
(let std/char/z (get "z"))
(let std/char/0 (get "0"))
(let std/char/1 (get "1"))
(let std/char/2 (get "2"))
(let std/char/3 (get "3"))
(let std/char/4 (get "4"))
(let std/char/5 (get "5"))
(let std/char/6 (get "6"))
(let std/char/7 (get "7"))
(let std/char/8 (get "8"))
(let std/char/9 (get "9"))
(let std/char/empty (get (string 0)))
(let std/char/double-quote (get (string 34)))
(let std/char/new-line (get (string 10)))
(let std/char/space (get " "))
(let std/char/tab (get "  "))
(let std/char/comma (get ","))
(let std/char/dot (get "."))
(let std/char/semi-colon (get ";"))
(let std/char/colon (get ":"))
(let std/char/dash (get "-"))
(let std/char/lower-dash (get "_"))
(let std/char/left-brace (get "("))
(let std/char/right-brace (get ")"))
(let std/char/curly-left-brace (get "{"))
(let std/char/curly-right-brace (get "}"))
(let std/char/left-bracket (get "["))
(let std/char/right-bracket (get "]"))
(let std/char/pipe (get "|"))
(let std/char/hash (get "#"))
(let std/char/question-mark (get "?"))
(let std/char/exclamation-mark (get "!"))
(let std/char/minus (get "-"))
(let std/char/plus (get "+"))
(let std/char/equal (get "="))
(let std/char/asterix (get "*"))
(let std/char/ampersand (get "&"))
(let std/char/at (get "@"))
(let std/char/backtick (get "`"))
(let std/char/digit? (lambda ch (and (>=# ch std/char/0) (<=# ch std/char/9))))
(let std/char/upper (lambda char (if (and (>=# char std/char/a) (<=# char std/char/z)) (-# char std/char/space) char)))
(let std/char/lower (lambda char (if (and (>=# char std/char/A) (<=# char std/char/Z)) (+# char std/char/space) char)))
(let std/vector/length (lambda xs (length xs)))
(let std/vector/get (lambda xs i (get xs i)))
(let std/vector/get/default (lambda xs i def (if (< i (length xs)) (get xs i) def)))
(let std/vector/2d/length std/vector/length)
(let std/vector/2d/get std/vector/get)
(let std/vector/2d/get/default std/vector/get/default)
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
(let std/float/max-safe 16777216.0)
(let std/float/min-safe -16777216.0)
(let std/float/pi 3.1415927)

(let std/float/safe? (lambda value (and (>=. value std/float/min-safe) (<=. value std/float/max-safe))))
(let std/float/get-safe (lambda vrbl (if (std/float/safe? (get vrbl)) (get vrbl) Float)))

(let std/int/safe? (lambda value (and (>= value std/int/min-safe) (<= value std/int/max-safe))))
(let std/int/get-safe (lambda vrbl (if (std/int/safe? (get vrbl)) (get vrbl) Int)))

(let std/float/floor (lambda n (-. n (mod. n 1.0))))
(let std/float/ceil (lambda n (do 
    (let sign (if (>=. n 0.0) 1 -1))
    (let absn (if (>=. n 0.0) n (-. n)))
    (let frac (mod. absn 1.0))
    (let intpart (-. absn frac))
    (cond (=. n 0.0) n (if (= sign 1) (+. intpart 1.0) (-. intpart))))))

; Extra "keywords" 
(let infinity 2147383647)
(let -infinity -2147483648)
(let identity (lambda x x))
(let Int 0)
(let Float 0.0)
(let Char (get "a"))
(let Bool false)
(let as (lambda . t t))
(let nil (loop 0 0 (lambda . 0)))
(let eq (lambda a b (cond 
          (and a b) true 
          (and (not a) (not b)) true
          false)))

(let int (lambda value (if (std/int/safe? value) [ value ] [ 0 ])))
(let float (lambda value (if (std/float/safe? value) [ value ] [ 0.0 ])))

(let box (lambda value [ value ]))
(let set (lambda vrbl x (std/vector/set! vrbl 0 x)))
(let =! (lambda vrbl x (std/vector/set! vrbl 0 x)))
(let boole-set (lambda vrbl x (std/vector/set! vrbl 0 (if x true false))))
(let boole-eqv (lambda a b (=? (get a) (get b))))
(let boolean/set (lambda vrbl x (std/vector/set! vrbl 0 (if x true false))))
(let true? (lambda vrbl (if (get vrbl) true false)))
(let false? (lambda vrbl (if (get vrbl) false true)))
(let += (lambda vrbl n (=! vrbl (+ (get vrbl) n))))
(let -= (lambda vrbl n (=! vrbl (- (get vrbl) n))))
(let *= (lambda vrbl n (=! vrbl (* (get vrbl) n))))
(let /= (lambda vrbl n (=! vrbl (/ (get vrbl) n))))
(let ++ (lambda vrbl (=! vrbl (+ (get vrbl) 1))))
(let -- (lambda vrbl (=! vrbl (- (get vrbl) 1))))
(let ** (lambda vrbl (=! vrbl (* (get vrbl) (get vrbl)))))


(let +=. (lambda vrbl n (=! vrbl (+. (get vrbl) n))))
(let -=. (lambda vrbl n (=! vrbl (-. (get vrbl) n))))
(let *=. (lambda vrbl n (=! vrbl (*. (get vrbl) n))))
(let /=. (lambda vrbl n (=! vrbl (/. (get vrbl) n))))
(let ++. (lambda vrbl (=! vrbl (+. (get vrbl) 1.0))))
(let --. (lambda vrbl (=! vrbl (-. (get vrbl) 1.0))))
(let **. (lambda vrbl (=! vrbl (*. (get vrbl) (get vrbl)))))

(let Bool->Int (lambda x (if (=? x true) 1 0)))
(let Bool->Char (lambda x (if (=? x true) std/char/1 std/char/0)))
(let Char->Int (lambda x (if (>=# x std/char/empty) (as x Int) 0)))
(let Char->Bool (lambda x (if (or (=# x std/char/empty) (=# x std/char/0)) false true)))
(let Int->Bool (lambda x 
    (cond 
        (<= x 0) false
        (>= x 1) true
        false)))
(let Int->Char (lambda x (if (>= x 0) (as x Char) std/char/empty)))

(let std/fn/apply/0 (lambda fn (fn)))
(let std/fn/apply/1 (lambda x fn (fn x)))
(let std/fn/apply/2 (lambda x y fn (fn x y)))
(let std/fn/apply/3 (lambda x y z fn (fn x y z)))
(let std/fn/apply/4 (lambda a b c d fn (fn a b c d)))
(let std/fn/apply/5 (lambda a b c d e fn (fn a b c d e)))
(let std/fn/apply/6 (lambda a b c d e f fn (fn a b c d e f)))


(let std/fn/apply/first/0 (lambda fn (fn)))
(let std/fn/apply/first/1 (lambda fn x (fn x)))
(let std/fn/apply/first/2 (lambda fn x y (fn x y)))
(let std/fn/apply/first/3 (lambda fn x y z (fn x y z)))
(let std/fn/apply/first/4 (lambda fn a b c d (fn a b c d)))
(let std/fn/apply/first/5 (lambda fn a b c d e (fn a b c d e)))
(let std/fn/apply/first/6 (lambda fn a b c d e f (fn a b c d e f)))

(let std/fn/combinator/2 (lambda a b x (a (b x))))
(let std/fn/combinator/3 (lambda a b c x (a (b (c x)))))
(let std/fn/combinator/4 (lambda a b c d x (a (b (c (d x))))))
(let std/fn/combinator/5 (lambda a b c d e x (a (b (c (d (e x)))))))
(let std/fn/combinator/6 (lambda a b c d e f x (a (b (c (d (e (f x))))))))

(let std/fn/const (lambda x . x))
(let std/fn/return 1)
(let std/fn/push 2)
(let std/fn/none 0)

(let std/fn/rec (lambda init-frame handler (do 
  (let stack [init-frame])
  (let result [])
  (loop (not (empty? stack)) (lambda (do 
    (let frame (pull! stack))
    (let action (handler frame))
    ; Action grammar:
    ; { std/fn/return, [value] } return
    ; { std/fn/push, [...] } push 
    ; { std/fn/none [] } none
    (cond 
      (= (fst action) std/fn/return) (do (set! result 0 (snd action)) nil)
      (= (fst action) std/fn/push) (do (loop 0 (length (snd action)) (lambda i (push! stack (get (snd action) i)))) nil)
      nil
    ))))
  (get result))))


(let Rec/return std/fn/return)
(let Rec/push std/fn/push)
(let Rec/none std/fn/none)
(let Rec std/fn/rec)

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


(let std/vector/float/range (lambda start end (do
     (let out [ (Int->Float start) ])
     (let process (lambda i (std/vector/set! out (length out) (Int->Float i))))
     (loop (+ start 1) (+ end 1) process)
     out))) 
    
 (let std/vector/float/ones (lambda n (do
     (let out [ 1.0 ])
     (let process (lambda i (std/vector/set! out (length out) 1.0)))
     (loop 1 n process)
     out))) 

 (let std/vector/float/zeroes (lambda n (do
     (let out [ 0.0 ])
     (let process (lambda i (std/vector/set! out (length out) 0.0)))
     (loop 1 n process)
     out)))


 (let std/vector/int/ones (lambda n (do
     (let out [ 1 ])
     (let process (lambda i (std/vector/set! out (length out) 1)))
     (loop 1 n process)
     out))) 

 (let std/vector/int/zeroes (lambda n (do
     (let out [ 0 ])
     (let process (lambda i (std/vector/set! out (length out) 0)))
     (loop 1 n process)
     out)))

(let std/vector/char/blanks (lambda n (do
    (let out [ std/char/empty ])
    (let process (lambda i (std/vector/set! out (length out) std/char/empty)))
    (loop 1 n process)
    out))) 

(let std/vector/count-of (lambda xs fn? (length (std/vector/filter xs fn?))))
(let std/vector/int/count (lambda input item (std/vector/count-of input (lambda x (= x item)))))
(let std/vector/char/count (lambda input item (std/vector/count-of input (lambda x (=# x item)))))
(let std/vector/bool/count (lambda input item (std/vector/count-of input (lambda x (=? x item)))))

(let std/vector/2d/count-of std/vector/count-of)
(let std/vector/2d/int/count std/vector/int/count)
(let std/vector/2d/char/count std/vector/char/count)
(let std/vector/2d/bool/count std/vector/bool/count)

(let std/vector/3d/count-of (lambda xs fn? (|> xs (std/vector/map (lambda ys (std/vector/2d/count-of ys fn?))) (std/vector/int/sum))))
(let std/vector/3d/int/count (lambda xs x (|> xs (std/vector/map (lambda ys (std/vector/2d/int/count ys x))) (std/vector/int/sum))))
(let std/vector/3d/char/count (lambda xs x (|> xs (std/vector/map (lambda ys (std/vector/2d/char/count ys x))) (std/vector/int/sum))))
(let std/vector/3d/bool/count (lambda xs x (|> xs (std/vector/map (lambda ys (std/vector/2d/bool/count ys x))) (std/vector/int/sum))))

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
(let std/float/abs (lambda n (if (<. n 0.0) (*. n -1.0) n)))

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
(let std/vector/float/sum (lambda xs (std/vector/reduce xs (lambda a b (+. a b)) 0.0)))

(let std/vector/int/product (lambda xs (std/vector/reduce xs (lambda a b (* a b)) 1)))
(let std/vector/float/product (lambda xs (std/vector/reduce xs (lambda a b (*. a b)) 1.0)))

(let std/int/euclidean-mod (lambda a b (mod (+ (mod a b) b) b)))
(let std/int/euclidean-distance (lambda x1 y1 x2 y2 (do
  (let a (- x1 x2))
  (let b (- y1 y2))
  (std/int/sqrt (+ (* a a) (* b b))))))
(let std/int/manhattan-distance (lambda x1 y1 x2 y2 (+ (std/int/abs (- x2 x1)) (std/int/abs (- y2 y1)))))
(let std/int/chebyshev-distance (lambda x1 y1 x2 y2 (std/int/max (std/int/abs (- x2 x1)) (std/int/abs (- y2 y1)))))
(let std/int/max (lambda a b (if (> a b) a b)))
(let std/int/min (lambda a b (if (< a b) a b)))
(let std/float/max (lambda a b (if (>. a b) a b)))
(let std/float/min (lambda a b (if (<. a b) a b)))
(let std/vector/int/maximum (lambda xs (cond (std/vector/empty? xs) Int (= (length xs) 1) (get xs 0) (std/vector/reduce xs std/int/max (get xs 0)))))
(let std/vector/int/minimum (lambda xs (cond (std/vector/empty? xs) Int (= (length xs) 1) (get xs 0) (std/vector/reduce xs std/int/min (get xs 0)))))
(let std/vector/float/maximum (lambda xs (cond (std/vector/empty? xs) Float (= (length xs) 1) (get xs 0) (std/vector/reduce xs std/float/max (get xs 0)))))
(let std/vector/float/minimum (lambda xs (cond (std/vector/empty? xs) Float (= (length xs) 1) (get xs 0) (std/vector/reduce xs std/float/min (get xs 0)))))
(let std/float/average (lambda x y (/. (+. x y) 2.0)))
(let std/int/average (lambda x y (/ (+ x y) 2)))
(let std/vector/int/mean (lambda xs (/ (std/vector/int/sum xs) (length xs))))
(let std/vector/float/mean (lambda xs (/. (std/vector/float/sum xs) (Int->Float (length xs)))))
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

(let std/float/clamp (lambda x limit (if (>. x limit) limit x)))
(let std/float/clamp-range (lambda x start end (cond (>. x end) end (<. x start) start x)))
(let std/float/between? (lambda v min max (and (>. v min) (<. v max))))
(let std/float/overlap? (lambda v min max (and (>=. v min) (<=. v max))))

(let std/int/sqrt (lambda n
  (do
    (integer low 0)
    (integer high n)
    (integer mid 0)
    (integer res 0)
    (loop (<= (get low) (get high))
      (lambda (do
        (set mid (+ (get low) (/ (- (get high) (get low)) 2)))
        (if (<= (get mid) 0)
          (do
            (set res (get mid))
            (set low (+ (get mid) 1)))
          (if (<= (get mid) (/ n (get mid)))
            (do
              (set res (get mid))
              (set low (+ (get mid) 1)))
            (set high (- (get mid) 1)))))))
    (get res))))
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
; 2d Vectors
(let std/vector/2d/int/set! (lambda vec [ x y . ] (do (set! vec 0 x) (set! vec 1 y) nil)))
(let std/vector/2d/int/add (lambda [ x1 y1 . ] [ x2 y2 . ] [ (+ x1 x2) (+ y1 y2) ]))
(let std/vector/2d/int/sub (lambda [ x1 y1 . ] [ x2 y2 . ] [ (- x1 x2) (- y1 y2) ]))
(let std/vector/2d/int/mul (lambda [ x1 y1 . ] [ x2 y2 . ] [ (* x1 x2) (* y1 y2) ]))
(let std/vector/2d/int/div (lambda [ x1 y1 . ] [ x2 y2 . ] [ (/ x1 x2) (/ y1 y2) ]))
(let std/vector/2d/int/mag/sqrt (lambda [ x y . ] (+ (* x x) (* y y))))
(let std/vector/2d/int/mag (lambda v (std/int/sqrt (std/vector/2d/int/mag/sqrt v))))
(let std/vector/2d/int/dot (lambda [ x1 y1 .] [ x2 y2 .] (+ (* x1 x2) (* y1 y2))))
(let std/vector/2d/int/cross (lambda [ x1 y1 . ] [ x2 y2 . ]  [ 0 0 (- (* x1 y2) (* y1 x2)) ]))
(let std/vector/2d/int/dist (lambda [ x1 y1 . ] [ x2 y2 . ] (std/vector/2d/int/mag (std/vector/2d/int/sub [ x2 y2 ] [ x1 y1 ]))))
(let std/vector/2d/int/normalize (lambda v (do 
  (let len (std/vector/2d/int/mag v))
  (std/vector/2d/int/div v [ len len ]))))

(let std/vector/2d/int/limit (lambda v m (do
            (let mSq (std/vector/2d/int/mag/sqrt v))
            (if (> mSq (* m m)) (do 
              (let sq (std/int/sqrt mSq))
              (std/vector/2d/int/mul (std/vector/2d/int/div v [ sq sq ]) [ m m ]))
              v))))
              
(let std/vector/2d/int/add! (lambda vec [ x2 y2 . ] (do (let x1 (get vec 0)) (let y1 (get vec 1)) (set! vec 0 (+ x1 x2)) (set! vec 1 (+ y1 y2)))))
(let std/vector/2d/int/sub! (lambda vec [ x2 y2 . ] (do (let x1 (get vec 0)) (let y1 (get vec 1)) (set! vec 0 (- x1 x2)) (set! vec 1 (- y1 y2)))))
(let std/vector/2d/int/mul! (lambda vec [ x2 y2 . ] (do (let x1 (get vec 0)) (let y1 (get vec 1)) (set! vec 0 (* x1 x2)) (set! vec 1 (* y1 y2)))))
(let std/vector/2d/int/div! (lambda vec [ x2 y2 . ] (do (let x1 (get vec 0)) (let y1 (get vec 1)) (set! vec 0 (/ x1 x2)) (set! vec 1 (/ y1 y2)))))
(let std/vector/2d/int/normalize! (lambda v (do 
  (let len (std/vector/2d/int/mag v))
  (std/vector/2d/int/div! v [ len len ]))))

(let std/vector/2d/int/limit! (lambda v m (do 
            (let mSq (std/vector/2d/int/mag/sqrt v)) 
            (if (> mSq (* m m)) (do 
              (let sq (std/int/sqrt mSq))
              (std/vector/2d/int/div! v [ sq sq ])
              (std/vector/2d/int/mul! v [ m m ]) 
              nil) 
              nil)
            nil)))
(let std/vector/2d/int/mag! (lambda v n (do
  (std/vector/2d/int/normalize! v) 
  (std/vector/2d/int/mul! v n)
  nil)))

; 3d Vectors
(let std/vector/3d/int/set! (lambda vec [ x y z . ] (do
  (set! vec 0 x) 
  (set! vec 1 y) 
  (set! vec 2 z) 
nil)))
(let std/vector/3d/int/add (lambda [ x1 y1 z1 . ] [ x2 y2 z2 . ] [ (+ x1 x2) (+ y1 y2) (+ z1 z2) ]))
(let std/vector/3d/int/sub (lambda [ x1 y1 z1 . ] [ x2 y2 z2 . ] [ (- x1 x2) (- y1 y2) (- z1 z2) ]))
(let std/vector/3d/int/mul (lambda [ x1 y1 z1 . ] [ x2 y2 z2 . ] [ (* x1 x2) (* y1 y2) (* z1 z2) ]))
(let std/vector/3d/int/div (lambda [ x1 y1 z1 . ] [ x2 y2 z2 . ] [ (/ x1 x2) (/ y1 y2) (/ z1 z2) ]))
(let std/vector/3d/int/mag/sqrt (lambda [ x y z . ] (+ (* x x) (* y y) (* z z))))
(let std/vector/3d/int/mag (lambda v (std/int/sqrt (std/vector/3d/int/mag/sqrt v))))
(let std/vector/3d/int/dot (lambda [ x1 y1 z1 .] [ x2 y2 z2 .] (+ (* x1 x2) (* y1 y2) (* z1 z2))))
(let std/vector/3d/int/cross (lambda [ x1 y1 z1 . ] [ x2 y2 z2 . ]  [ (- (* y1 z2) (* z1 y2)) (- (* z1 x2) (* x1 z2)) (- (* x1 y2) (* y1 x2)) ]))
(let std/vector/3d/int/dist (lambda [ x1 y1 z1 . ] [ x2 y2 z2 . ] (std/vector/3d/int/mag (std/vector/3d/int/sub [ x2 y2 z2 ] [ x1 y1 z1 ]))))
(let std/vector/3d/int/normalize (lambda v (do 
  (let len (std/vector/3d/int/mag v))
  (std/vector/3d/int/div v [ len len len ]))))

(let std/vector/3d/int/limit (lambda v m (do
            (let mSq (std/vector/3d/int/mag/sqrt v))
            (if (> mSq (* m m)) (do 
              (let sq (std/int/sqrt mSq))
              (std/vector/3d/int/mul (std/vector/3d/int/div v [ sq sq sq ]) [ m m m ]))
              v))))  
(let std/vector/3d/int/add! (lambda vec [ x2 y2 z2 . ] (do (let x1 (get vec 0)) (let y1 (get vec 1)) (let z1 (get vec 2)) (set! vec 0 (+ x1 x2)) (set! vec 1 (+ y1 y2)) (set! vec 2 (+ z1 z2)))))
(let std/vector/3d/int/sub! (lambda vec [ x2 y2 z2 . ] (do (let x1 (get vec 0)) (let y1 (get vec 1)) (let z1 (get vec 2)) (set! vec 0 (- x1 x2)) (set! vec 1 (- y1 y2)) (set! vec 2 (- z1 z2)))))
(let std/vector/3d/int/mul! (lambda vec [ x2 y2 z2 . ] (do (let x1 (get vec 0)) (let y1 (get vec 1)) (let z1 (get vec 2)) (set! vec 0 (* x1 x2)) (set! vec 1 (* y1 y2)) (set! vec 2 (* z1 z2)))))
(let std/vector/3d/int/div! (lambda vec [ x2 y2 z2 . ] (do (let x1 (get vec 0)) (let y1 (get vec 1)) (let z1 (get vec 2)) (set! vec 0 (/ x1 x2)) (set! vec 1 (/ y1 y2)) (set! vec 2 (/ z1 z2)))))
(let std/vector/3d/int/normalize! (lambda v (do 
  (let len (std/vector/3d/int/mag v))
  (std/vector/3d/int/div! v [ len len len ]))))

(let std/vector/3d/int/limit! (lambda v m (do 
            (let mSq (std/vector/3d/int/mag/sqrt v)) 
            (if (> mSq (* m m)) (do 
              (let sq (std/int/sqrt mSq))
              (std/vector/3d/int/div! v [ sq sq sq ])
              (std/vector/3d/int/mul! v [ m m m ]) 
              nil) 
              nil)
            nil)))
(let std/vector/3d/int/mag! (lambda v n (do
  (std/vector/3d/int/normalize! v) 
  (std/vector/3d/int/mul! v n)
  nil)))



(let std/vector/zipper (lambda a b (do 
      (let out [[(get a 0) (get b 0)]])
      (let process (lambda i (std/vector/set! out (length out) [(get a i) (get b i)])))
      (loop 1 (length a) process)
      out)))

(let std/vector/zip (lambda xs (std/vector/zipper (std/vector/first xs) (std/vector/second xs))))
(let std/vector/unzip (lambda xs [ (std/vector/map xs std/vector/first) (std/vector/map xs std/vector/second) ]))


(let std/vector/tuple/zipper (lambda a b (do
      (let out [{ (get a 0) (get b 0) }])
      (let process (lambda i (std/vector/set! out (length out) { (get a i) (get b i) })))
      (loop 1 (length a) process)
      out)))

(let std/vector/tuple/zip (lambda xs (std/vector/tuple/zipper (fst xs) (snd xs))))
(let std/vector/tuple/unzip (lambda xs { (std/vector/map xs (lambda x (fst x))) (std/vector/map xs (lambda x (snd x))) }))


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

(let std/vector/char/equal? (lambda a b (and (= (length a) (length b)) (|>
  a
  (std/vector/zipper b)
  (std/vector/every? (lambda x (=# (get x 0) (get x 1))))))))

(let std/vector/char/greater? (lambda A B (and (not (std/vector/char/equal? A B)) (do
    (let a (if (< (length A) (length B)) (std/vector/cons! A (std/vector/char/blanks (- (length B) (length A)))) A))
    (let b (if (> (length A) (length B)) (std/vector/cons! B (std/vector/char/blanks (- (length A) (length B)))) B))
    (let pairs (std/vector/reverse (std/vector/zipper a b)))
    (let tail-call/string/greater (lambda is (unless (std/vector/empty? pairs) (do 
        (let current (std/vector/pop-and-get! pairs))
        (if (=# (std/vector/first current) (std/vector/second current)) (tail-call/string/greater is) (># (std/vector/first current) (std/vector/second current))))
        is)))
    (tail-call/string/greater false)))))

(let std/vector/char/lesser? (lambda A B (and (not (std/vector/char/equal? A B)) (do
    (let a (if (< (length A) (length B)) (std/vector/cons! A (std/vector/char/blanks (- (length B) (length A)))) A))
    (let b (if (> (length A) (length B)) (std/vector/cons! B (std/vector/char/blanks (- (length A) (length B)))) B))
    (let pairs (std/vector/reverse (std/vector/zipper a b)))
    (let tail-call/string/lesser (lambda is (unless (std/vector/empty? pairs) (do 
        (let current (std/vector/pop-and-get! pairs))
        (if (=# (std/vector/first current) (std/vector/second current)) (tail-call/string/lesser is) (<# (std/vector/first current) (std/vector/second current))))
        is)))
    (tail-call/string/lesser false)))))
    
(let std/vector/char/match? std/vector/char/equal?)
(let std/vector/char/greater-or-equal? (lambda A B (or (std/vector/char/equal? A B) (std/vector/char/greater? A B))))
(let std/vector/char/lesser-or-equal? (lambda A B (or (std/vector/char/equal? A B) (std/vector/char/lesser? A B))))
(let std/vector/char/negative? (lambda str (=# (std/vector/first str) std/char/minus)))

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
           (std/vector/set! total 0 (std/int/euclidean-mod (+ (* (get total 0 ) prime-num) (as letter Int)) (length table)))
           (std/vector/set! i 0 (+ (get i) 1)))))

     (loop (< (get i) bounds) process)
     (get total 0))))

(let std/vector/hash/set/has? (lambda table key (do
     (let idx (std/int/hash table key))
     (let current (get table idx))
     (and (std/vector/in-bounds? table idx)
                  (and (> (length current) 0)
                       (>= (std/vector/find-index current (lambda x (std/vector/char/equal? x key))) 0))))))

(let std/vector/hash/table/has? (lambda table key (do
         (let idx (std/int/hash table key))
         (let current (std/vector/map (get table idx) (lambda x (get x 0))))
         (and (std/vector/in-bounds? table idx)
         (and (> (length current) 0)
           (>= (std/vector/find-index current
             (lambda x
               (std/vector/char/equal? x key))) 0))))))

(let std/vector/hash/set/add!
     (lambda table key
       (do
         (let idx (std/int/hash table key))
         (if (not (std/vector/in-bounds? table idx)) (std/vector/set! table idx (as [] [[Char]])) nil)
         (let item (get table idx))
         (let current (as item [[Char]]))
         (let len (length current))
         (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/char/equal? x key))) -1))
         (let entry key)
         (if (= index -1)
           (std/vector/set! current (length current) entry)
           (std/vector/set! current index entry)) table)))

(let std/vector/hash/set/remove!
 (lambda table key
   (do
     (let idx (std/int/hash table key))
     (if (not (std/vector/in-bounds? table idx)) (std/vector/set! table idx (as [] [[Char]])) nil)
     (let item (get table idx))
     (let current (as item [[Char]]))
     (let len (length current))
     (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/char/equal? x key))) -1))
     (let entry key)
     (if (not (= index -1)) (do (std/vector/set! current index (std/vector/at current -1)) (std/vector/pop! current)) nil)
     table)))

(let std/vector/hash/table/set! (lambda table key value
       (do
         (let idx (std/int/hash table key))
         (if (not (std/vector/in-bounds? table idx)) (std/vector/set! table idx (as [] [[[Char]]])) nil)
         (let item (get table idx))
         (let current (as item [[[Char]]]))
         (let len (length current))
         (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/char/equal? (as (get x 0) [Char]) key))) -1))
         (let entry [ key [(Int->Char value)] ])
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
       (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/char/equal? (get x 0) key))) -1))
       (if (not (= index -1)) (do (std/vector/set! current index (std/vector/at current -1)) (std/vector/pop! current)) nil)
       table)))

(let std/vector/hash/clear! (lambda table (do 
     (loop 0 (length table) (lambda i (std/vector/empty! (get table i))))
     table)))

(let std/vector/hash/table/get-helper (lambda table idx key (do
   (let current (get table idx))
   (let found-index (std/vector/find-index current (lambda x (std/vector/char/equal? key (as (get x 0) [Char])))))
   (unless (= found-index -1) (get current found-index 1) []))))

(let std/vector/hash/table/get (lambda table key (do
     (let idx (std/int/hash table key))
     (if (std/vector/in-bounds? table idx) (get (std/vector/hash/table/get-helper table idx key)) (as -1 Char)))))

(let std/vector/hash/table/count (lambda arr 
    (|> arr (std/vector/reduce (lambda table key (do 
        (if (std/vector/hash/table/has? table key) 
            (do 
            (let v (std/vector/hash/table/get table key))
            (std/vector/hash/table/set! table key (+ (as v Int) 1)))
            (std/vector/hash/table/set! table key 1)))) (std/vector/buckets 64)))))

(let std/vector/tuple/hash/table/set! (lambda table key value
       (do
         (let idx (std/int/hash table key))
         (if (not (std/vector/in-bounds? table idx)) (set! table idx []) nil)
         (let current (get table idx))
         (let len (length current))
         (let index (if (> len 0) (std/vector/find-index  current (lambda x (std/vector/char/equal? (fst (get x 0)) key))) -1))
         (let entry [{ key value }])
         (if (= index -1)
           (set! current (length current) entry)
           (set! current index entry))
         table)))


(let std/vector/tuple/hash/table/has? (lambda table key (do
         (let idx (std/int/hash table key))
         (let current (std/vector/map (get table idx) (lambda x (fst (get x 0)))))
         (and (std/vector/in-bounds? table idx)
         (and (> (length current) 0)
           (>= (std/vector/find-index current
             (lambda x
               (std/vector/char/equal? x key))) 0))))))

(let std/vector/tuple/hash/table/remove!
 (lambda table key
   (do
     (let idx (std/int/hash table key))
     (if (not (std/vector/in-bounds? table idx)) (set! table idx []) nil)
     (let current (get table idx))
     (let len (length current))
     (let index (if (> len 0) (std/vector/find-index current (lambda x (std/vector/char/equal? (fst (get x 0)) key))) -1))
     (let entry key)
     (if (not (= index -1)) (do (std/vector/set! current index (std/vector/at current -1)) (std/vector/pop! current)) nil)
     table)))


(let std/vector/tuple/hash/table/get-helper (lambda table idx key (do
   (let current (get table idx))
   (let found-index (std/vector/find-index current (lambda x (std/vector/char/equal? key (fst (get x 0))))))
   (unless (= found-index -1) (get current found-index) []))))

(let std/vector/tuple/hash/table/get (lambda table key (do
     (let idx (std/int/hash table key))
     (if (std/vector/in-bounds? table idx) (std/vector/tuple/hash/table/get-helper table idx key) []))))

(let std/vector/tuple/hash/table/count (lambda arr 
    (|> arr (std/vector/reduce (lambda table key (do 
        (if (std/vector/tuple/hash/table/has? table key) 
            (do 
            (let v (snd (get (std/vector/tuple/hash/table/get table key))))
            (std/vector/tuple/hash/table/set! table key (+ v 1)))
            (std/vector/tuple/hash/table/set! table key 1)))) (std/vector/buckets 64)))))

(let std/vector/tuple/hash/table/entries (lambda table (map (flat table) (lambda x { (fst (get x)) (snd (get x))}))))
(let std/vector/tuple/hash/table/keys (lambda table (map (flat table) (lambda x (fst (get x))))))
(let std/vector/tuple/hash/table/values (lambda table (map (flat table) (lambda x (snd (get x))))))

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
(let std/convert/char->digit (lambda digit (if (<# digit std/char/0) 0 (- (as digit Int) (as std/char/0 Int)))))
(let std/convert/chars->digits (lambda digits (std/vector/map digits std/convert/char->digit)))
(let std/convert/digit->char (lambda digit (if (< digit 0) std/char/0 (+# (as digit Char) std/char/0))))
(let std/convert/digits->chars (lambda digits (std/vector/map digits std/convert/digit->char)))
(let std/convert/bool->int (lambda x (if (=? x true) 1 0)))
(let std/convert/int->bool (lambda x (if (= x 0) false true)))
(let std/convert/vector->string (lambda xs delim (std/vector/reduce/i xs (lambda a b i (if (> i 0) (std/vector/cons (std/vector/append! a delim) b) b)) "")))
(let std/convert/string->vector (lambda str char (|> str
              (std/vector/reduce(lambda a b (do
              (let prev (std/vector/at a -1))
                (if (std/vector/char/equal? [b] [char])
                    (std/vector/set! a (length a) [])
                    (std/vector/set! prev (length prev) b)) a))
              [[]])
              (std/vector/map (lambda x (std/convert/vector->string [ x ] std/char/empty))))))

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
            (if (=# ch std/char/minus) 
                (set current-sign -1) 
                (do  
                    (std/vector/push! a (* (get current-sign) (std/convert/char->digit ch))) 
                    (set current-sign 1)))
                a)) [])))))
(let std/convert/digits->integer std/convert/positive-or-negative-digits->integer)
(let std/convert/positive-or-negative-chars->integer (lambda x (|> x (std/convert/chars->positive-or-negative-digits) (std/convert/positive-or-negative-digits->integer))))
(let std/convert/chars->integer std/convert/positive-or-negative-chars->integer)

(let std/convert/chars->digits/float (lambda xs
    (|> xs 
        (std/vector/reduce (lambda a ch (do 
              (if (=# ch '.') (push! a []) (push! (std/vector/at a -1) (std/convert/char->digit ch)))
                a)) [[]]))))

(let std/convert/chars->ufloat (lambda xs (do
  (let parts (std/convert/chars->digits/float xs))
  (let pow (std/int/expt 10 (length (get parts 1))))
  (/. (Int->Float (+ 
    (* (std/convert/digits->integer (get parts 0)) pow)
    (std/convert/digits->integer (get parts 1)))) (Int->Float pow)))))

(let std/convert/chars->float (lambda xs 
  (if (=# (get xs 0) std/char/minus) (*. (std/convert/chars->ufloat (std/vector/slice xs 1 (length xs))) -1.0) (std/convert/chars->ufloat xs))))

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


(let std/vector/int/unique (lambda xs 
    (if (= (length xs) 0) 
        [(+ (get xs 0) 0)] 
        (|> xs (std/vector/map (lambda x [(as x Char)])) (std/convert/vector->set) (std/convert/set->vector) (std/vector/map (lambda x (as (get x 0) Int)))))))


(let std/vector/char/unique (lambda xs 
    (if (= (length xs) 0) 
        xs 
        (|> xs (std/vector/map (lambda x [x])) (std/convert/vector->set) (std/convert/set->vector) (std/vector/map (lambda x (get x 0)))))))

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

(let neighborhood std/vector/3d/adjacent)
(let neighborhood/moore std/vector/3d/moore-neighborhood)
(let neighborhood/diagonal std/vector/3d/diagonal-neighborhood)
(let neighborhood/kernel std/vector/3d/kernel-neighborhood)
(let neighborhood/von-neumann std/vector/3d/von-neumann-neighborhood)

(let std/node/parent (lambda i (- (>> (+ i 1) 1) 1)))
(let std/node/left (lambda i (+ (<< i 1) 1)))
(let std/node/right (lambda i (<< (+ i 1) 1)))

(let std/heap/top 0)
(let std/heap/greater? (lambda heap i j fn? (=? (fn? (get heap i) (get heap j)) true)))
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
    (if (= num 0) "0" (do 
        (let neg? (< num 0))
        (integer n (if neg? (* num -1) num))
        (let tail-call/while (lambda out
            (if (> (get n) 0) (do
                (let x (mod (get n) base))
                (std/vector/push! out x)
                (set n (/ (get n) base))
                (tail-call/while out)) out)))
        (let str (std/convert/digits->chars (tail-call/while [])))
        (std/vector/reverse (if neg? (std/vector/append! str std/char/dash) str))))))
(let std/convert/integer->string (lambda x (std/convert/integer->string-base x 10)))
(let std/convert/vector->set (lambda xs (std/vector/reduce xs (lambda s x (do (std/vector/hash/set/add! s x) s)) [ [] [] [] [] [] [] [] ])))
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
        (as out [[[Char]]]))))
(let std/vector/hash/set/union (lambda a b (do
        (let out (std/vector/hash/set/max-capacity a b))
        (|> a (std/convert/set->vector) (std/vector/for (lambda element (std/vector/hash/set/add! out element))))
        (|> b (std/convert/set->vector) (std/vector/for (lambda element (std/vector/hash/set/add! out element))))
        (as out [[[Char]]]))))

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
 (let half (/ (length initial) 2))
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
(let std/vector/stack/pop! (lambda stack def (std/vector/deque/head! stack def)))
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

(let std/vector/concat/with (lambda xs ch (std/vector/reduce/i xs (lambda a b i (if (and (> i 0) (< i (length xs))) (std/vector/cons (std/vector/cons a [ ch ]) b) (std/vector/cons a b))) [])))

(let std/vector/char/lines (lambda xs (std/convert/string->vector xs std/char/new-line)))
(let std/vector/char/words (lambda xs (std/convert/string->vector xs std/char/space)))
(let std/vector/char/commas (lambda xs (std/convert/string->vector xs std/char/comma)))
(let std/vector/int/pair/sub (lambda xs (- (. xs 0) (. xs 1))))
(let std/vector/int/pair/add (lambda xs (+ (. xs 0) (. xs 1))))
(let std/vector/int/pair/mult (lambda xs (* (. xs 0) (. xs 1))))
(let std/vector/int/pair/div (lambda xs (/ (. xs 0) (. xs 1))))
(let std/vector/sort/asc! (lambda xs (std/vector/sort! xs <)))
(let std/vector/sort/desc! (lambda xs (std/vector/sort! xs >)))

(let std/vector/equal? (lambda a b (do
  (if (< (length a) (length b)) false
  (if (> (length a) (length b)) false
    (do
      (integer i 0)
      (boolean result true)
      (loop (< (get i) (length a)) (lambda (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (not (= da db)) (do
          (boolean/set result false)
          (set i (length a))))
        (set i (+ (get i) 1)))))
      (if (true? result) true false)))))))

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

(let std/vector/flat-map (lambda xs (std/vector/map (std/vector/flat-one xs))))
    
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


(let std/vector/int/big/range (lambda start end (do
     (let out [ (std/int/big/new (std/convert/integer->string start)) ])
     (let process (lambda i (std/vector/set! out (length out) (std/int/big/new (std/convert/integer->string i)))))
     (loop (+ start 1) (+ end 1) process)
   out))) 

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
      (if (true? tr) (boolean/set tr false))
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
          (boolean/set result true)
          (set i (length a))))
        (if (> da db) (do
          (boolean/set result false)
          (set i (length a))))
        (set i (+ (get i) 1)))))
      (if (true? result) true false)))))))

(let std/int/big/greater-or-equal? (lambda a b (do
  (if (> (length a) (length b)) true
  (if (< (length a) (length b)) false
    ; Equal length, compare digit by digit
    (do
      (integer i 0)
      (boolean result true) ; assume a >= b
      (loop (< (get i) (length a)) (lambda (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (> da db) (do
          (boolean/set result true)
          (set i (length a))))
        (if (< da db) (do
          (boolean/set result false)
          (set i (length a))))
        (set i (+ (get i) 1)))))
      (if (true? result) true false)))))))

(let std/int/big/less-than? (lambda a b (do
  (if (< (length a) (length b)) true
  (if (> (length a) (length b)) false
    ; Equal length, check for strict less (not equal)
    (do
      (integer i 0)
      (boolean found-less false) ; true if a < b at some digit
      (loop (< (get i) (length a)) (lambda (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (< da db) (do
          (boolean/set found-less true)
          (set i (length a))))
        (if (> da db) (do
          (set i (length a)))) ; stop on a > b, keep found-less false
        (set i (+ (get i) 1)))))
      (if (true? found-less) true false)))))))

(let std/int/big/greater-than? (lambda a b (do
  (if (> (length a) (length b)) true
  (if (< (length a) (length b)) false
    ; Equal length, check for strict greater (not equal)
    (do
      (integer i 0)
      (boolean found-greater false) ; true if a > b at some digit
      (loop (< (get i) (length a)) (lambda (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (> da db) (do
          (boolean/set found-greater true)
          (set i (length a))))
        (if (< da db) (do
          (set i (length a)))) ; stop on a < b, keep found-greater false
        (set i (+ (get i) 1)))))
      (if (true? found-greater) true false)))))))

(let std/int/big/equal? std/vector/equal?)

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
(let std/vector/int/big/product (lambda xs (std/vector/reduce xs (lambda a b (std/int/big/mul a b)) [ 1 ] )))
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

(let std/vector/int/extreme (lambda xs { (std/vector/int/minimum xs) (std/vector/int/maximum xs) }))
(let std/tuple/map (lambda { a b } fn (fn a b)))
(let std/tuple/map/fst (lambda { a } fn (fn a)))
(let std/tuple/map/snd (lambda { . b } fn (fn b)))

(let get* (lambda xs i some none (if (std/vector/in-bounds? xs i) (do (some (get xs i)) nil) (do (none) nil))))
(let std/vector/get* (lambda xs i some none (if (std/vector/in-bounds? xs i) (do (some (get xs i)) nil) (do (none) nil))))
(let std/vector/2d/get* std/vector/get*)
(let std/vector/3d/get* (lambda xs i j some none (if (std/vector/3d/in-bounds? xs i j) (do (some (get xs i j)) nil) (do (none) nil))))
(let std/vector/hash/table/get* (lambda xs i some none (if (std/vector/hash/table/has? xs i) (do (some (std/vector/hash/table/get xs i)) nil) (do (none) nil))))
(let std/vector/enumerate (lambda xs (std/vector/tuple/zip { (std/vector/int/range 0 (- (length xs) 1)) xs })))

; Start of more fake keywords
(let /flat-map std/vector/flat-map)
(let /reduce std/vector/reduce)
(let /map std/vector/map)
(let /filter std/vector/filter)
(let /every? std/vector/every?)
(let /some? std/vector/some?)
(let /sort! std/vector/sort!)
(let /find std/vector/find-index)
(let /count std/vector/count-of)
(let /for std/vector/for)
(let /scan std/vector/adjacent-difference)
(let /map/tuple std/tuple/map)
(let /map/fst std/tuple/map/fst)
(let /map/snd std/tuple/map/snd)
(let /partition std/vector/partition)

(let \reduce (lambda fn a xs (std/vector/reduce xs fn a)))
(let \map (lambda fn xs (std/vector/map xs fn)))
(let \filter (lambda fn? xs (std/vector/filter xs fn?)))
(let \every? (lambda fn? xs (std/vector/every? xs fn?)))
(let \some? (lambda fn? xs (std/vector/some? xs fn?)))
(let \sort! (lambda fn xs (std/vector/sort! xs fn)))
(let \find (lambda fn? xs (std/vector/find-index xs fn?)))
(let \count (lambda fn? xs (std/vector/count-of xs fn?)))
(let \flat-map (lambda fn xs (std/vector/flat-map xs fn)))
(let \for (lambda fn xs (std/vector/for xs fn)))
(let \scan (lambda fn xs (std/vector/adjacent-difference xs fn)))
(let \map/tuple (lambda fn xs (std/tuple/map xs fn)))
(let \map/fst (lambda fn xs (std/tuple/map/fst xs fn)))
(let \map/snd (lambda fn xs (std/tuple/map/snd xs fn)))
(let \partition (lambda n xs (std/vector/partition xs n)))

(let floor std/float/floor)
(let ceil std/float/ceil)

(let extreme std/vector/int/extreme)
(let map/tuple std/tuple/map)
(let map/fst std/tuple/map/fst)
(let map/snd std/tuple/map/snd)
(let subset std/vector/subset)
(let flat-map std/vector/flat-map)
(let map std/vector/map)
(let for std/vector/for)
(let filter std/vector/filter)
(let reduce std/vector/reduce)
(let every? std/vector/every?)
(let some? std/vector/some?)
(let empty? std/vector/empty?)
(let not-empty? std/vector/not-empty?)
(let find std/vector/find-index)
(let partition std/vector/partition)
(let reverse std/vector/reverse)
(let slice std/vector/slice)
(let cons std/vector/cons)
(let sort! std/vector/sort!)
(let range std/vector/int/range)
(let range/int std/vector/int/range)
(let range/float std/vector/float/range)
(let square std/int/square)
(let expt std/int/expt)
(let sqrt std/int/sqrt)
(let odd? std/int/odd?)
(let even? std/int/even?)
(let one? std/int/one?)
(let zero? std/int/zero?)
(let ones std/vector/int/ones)
(let zeroes std/vector/int/zeroes)
(let ones/int std/vector/int/ones)
(let zeroes/int std/vector/int/zeroes)
(let ones/float std/vector/float/ones)
(let zeroes/float std/vector/float/zeroes)

(let positive? std/int/positive?)
(let negative? std/int/negative?)
(let invert std/int/invert)
(let negative-one? std/int/negative-one?)
(let divisible? std/int/divisible?)
(let upper std/char/upper)
(let lower std/char/lower)
(let match? std/vector/char/equal?)
(let String/equal? std/vector/char/equal?)
(let String/lte? std/vector/char/lesser-or-equal?)
(let String/gte? std/vector/char/greater-or-equal?)
(let String/lt? std/vector/char/lesser?)
(let String/gt? std/vector/char/greater?)

(let digit? std/char/digit?)
(let fill std/vector/2d/fill)
(let max std/int/max)
(let min std/int/min)
(let max/int std/int/max)
(let min/int std/int/min)
(let max/float std/float/max)
(let min/float std/float/min)

(let maximum std/vector/int/maximum)
(let minimum std/vector/int/minimum)
(let maximum/int std/vector/int/maximum)
(let minimum/int std/vector/int/minimum)
(let maximum/float std/vector/float/maximum)
(let minimum/float std/vector/float/minimum)

(let push! (lambda xs x (set! xs (length xs) x)))

(let abs std/int/abs)
(let abs/int std/int/abs)
(let abs/float std/float/abs)

(let pull! std/vector/pop-and-get!)
(let first std/vector/first)
(let last std/vector/last)
(let car std/vector/first)
(let cdr (lambda xs (std/vector/slice xs 1 (length xs))))
(let pair (lambda a b (tuple a b)))
(let product std/vector/int/product)
(let product/int std/vector/int/product)
(let product/float std/vector/float/product)
(let sum std/vector/int/sum)
(let sum/int std/vector/int/sum)
(let sum/float std/vector/float/sum)
(let avg std/int/average)
(let zip std/vector/tuple/zip)
(let unzip std/vector/tuple/unzip)
(let nl std/char/new-line)
(let window std/vector/sliding-window)
(let flat std/vector/flat-one)
(let enumerate std/vector/enumerate)
(let clamp std/int/clamp)
(let clamp-range std/int/clamp-range)

(let clamp/int std/int/clamp)
(let clamp-range/int std/int/clamp-range)
(let clamp/float std/float/clamp)
(let clamp-range/float std/float/clamp-range)

(let at std/vector/at)
(let emod std/int/euclidean-mod)
(let swap! std/vector/swap!)
(let scan std/vector/adjacent-difference)

(let cartesian-product std/vector/cartesian-product)
(let lcm std/int/lcm)
(let gcd std/int/gcd)

(let Integer->Chars std/convert/integer->string)
(let Integer->String std/convert/integer->string)
(let String->Vector std/convert/string->vector)
(let Vector->String std/convert/vector->string)
(let Chars->Integer std/convert/chars->integer)
(let String->Integer std/convert/chars->integer)
(let String->Float std/convert/chars->float)
(let Chars->Float std/convert/chars->float)
(let Integer->Bits std/convert/integer->bits)

(let Integer->Digits std/convert/integer->digits)
(let Digits->Chars std/convert/digits->chars)
(let Set->Vector std/convert/set->vector)
(let Vector->Set std/convert/vector->set)

(let Char/count std/vector/char/count)
(let Int/count std/vector/int/count)
(let Bool/count std/vector/bool/count)
(let count std/vector/count-of)

(let unique/int std/vector/int/unique)
(let unique/char std/vector/char/unique)

(let Vector/equal? std/vector/equal?)

(let Set/intersection std/vector/hash/set/intersection)
(let Set/difference std/vector/hash/set/difference)
(let Set/xor std/vector/hash/set/xor)
(let Set/union std/vector/hash/set/union)
(let Set/add! (lambda table item (do (std/vector/hash/set/add! table item) nil)))
(let Set/remove! (lambda table item (do (std/vector/hash/set/remove! table item) nil)))
(let Set/has? std/vector/hash/set/has?)

(let Table/entries std/vector/tuple/hash/table/entries)
(let Table/keys std/vector/tuple/hash/table/keys)
(let Table/values std/vector/tuple/hash/table/values)

(let Set/values std/vector/tuple/hash/table/keys)

(let Table/get std/vector/tuple/hash/table/get)

(let Table/has? std/vector/tuple/hash/table/has?)
(let Table/set! (lambda table key value (do (std/vector/tuple/hash/table/set! table key value) nil)))
(let Table/remove! (lambda table key (do (std/vector/tuple/hash/table/remove! table key) nil)))
(let Table/count std/vector/tuple/hash/table/count)

(let in-bounds? std/vector/in-bounds?)

(let take/first std/vector/take)
(let drop/first std/vector/drop)

(let take/last std/vector/take/last)
(let drop/last std/vector/drop/last)

(let Que/new std/vector/deque/new)
(let Que/empty? std/vector/deque/empty?)
(let Que/not-empty? std/vector/queue/not-empty?)
(let Que/empty-default! (lambda def q (do (std/vector/deque/empty! q def) nil)))
(let Que/enque! (lambda xs v (do (std/vector/queue/enqueue! xs v) nil)))
(let Que/deque-default! (lambda def queue (do (std/vector/deque/tail! queue def) nil)))
(let Que/peek std/vector/deque/first)
(let Que/push! (lambda xs v (do (std/vector/deque/append! xs v) nil)))
(let Que/pop-default! (lambda def queue (do (std/vector/deque/head! queue def) nil)))
(let Que/prepend! (lambda xs v (do (std/vector/deque/prepend! xs v) nil))) 
(let Que/first std/vector/deque/first)
(let Que/last std/vector/deque/last)
(let Que/tail-default! (lambda def queue (do (std/vector/deque/tail! queue def) nil)))
(let Que/append! (lambda xs v (do (std/vector/deque/append! xs v) nil)))

(let Vector->Que (lambda def xs (std/convert/vector->deque xs def)))
(let Que->Vector std/convert/deque->vector)
(let Que/get std/vector/deque/get)
(let Que/length std/vector/deque/length)
(let Que/at (lambda xs i (if (< i 0) (std/vector/deque/get xs (+ (length xs) i)) (std/vector/deque/get xs i))))


(let BigInt/add std/int/big/add)
(let BigInt/sub std/int/big/sub)
(let BigInt/mul std/int/big/mul)
(let BigInt/lte? std/int/big/less-or-equal?)
(let BigInt/gte? std/int/big/greater-or-equal?)
(let BigInt/lt? std/int/big/less-than?)
(let BigInt/gt? std/int/big/greater-than?)
(let BigInt/equal? std/int/big/equal?)
(let BigInt/div std/int/big/div)
(let BigInt/square std/int/big/square)
(let BigInt/div/floor std/int/big/floor/div)
(let BigInt/div/ceal std/int/big/ceil/div)
(let BigInt/sum std/vector/int/big/sum)
(let BigInt/product std/vector/int/big/product)
(let BigInt/new std/int/big/new)
(let BigInt/pow std/int/big/pow)
(let BigInt/expt std/int/big/expt)
(let BigInt/range std/vector/int/big/range)

(let Vector/2d/set! std/vector/2d/int/set!)
(let Vector/2d/add std/vector/2d/int/add)
(let Vector/2d/sub std/vector/2d/int/sub)
(let Vector/2d/mul std/vector/2d/int/mul)
(let Vector/2d/div std/vector/2d/int/div)
(let Vector/2d/mag/sqrt std/vector/2d/int/mag/sqrt)
(let Vector/2d/mag std/vector/2d/int/mag)
(let Vector/2d/dot std/vector/2d/int/dot)
(let Vector/2d/cross std/vector/2d/int/cross)
(let Vector/2d/dist std/vector/2d/int/dist)
(let Vector/2d/norm std/vector/2d/int/normalize)
(let Vector/2d/limit std/vector/2d/int/limit)
(let Vector/2d/add! std/vector/2d/int/add!)
(let Vector/2d/sub! std/vector/2d/int/sub!)
(let Vector/2d/mul! std/vector/2d/int/mul!)
(let Vector/2d/div! std/vector/2d/int/div!)
(let Vector/2d/norm! std/vector/2d/int/normalize!)
(let Vector/2d/limit! std/vector/2d/int/limit!)
(let Vector/2d/mag! std/vector/2d/int/mag!)
(let Vector/3d/set! std/vector/3d/int/set!)
(let Vector/3d/add std/vector/3d/int/add)
(let Vector/3d/sub std/vector/3d/int/sub)
(let Vector/3d/mul std/vector/3d/int/mul)
(let Vector/3d/div std/vector/3d/int/div)
(let Vector/3d/mag/sqrt std/vector/3d/int/mag/sqrt)
(let Vector/3d/mag std/vector/3d/int/mag)
(let Vector/3d/dot std/vector/3d/int/dot)
(let Vector/3d/cross std/vector/3d/int/cross) 
(let Vector/3d/dist std/vector/3d/int/dist)
(let Vector/3d/norm std/vector/3d/int/normalize) 
(let Vector/3d/limit std/vector/3d/int/limit)
(let Vector/3d/add! std/vector/3d/int/add!)
(let Vector/3d/sub! std/vector/3d/int/sub!)
(let Vector/3d/mul! std/vector/3d/int/mul!)
(let Vector/3d/div! std/vector/3d/int/div!)
(let Vector/3d/norm! std/vector/3d/int/normalize!)
(let Vector/3d/limit! std/vector/3d/int/limit!)
(let Vector/3d/mag! std/vector/3d/int/mag!)

; End of more fake words

; Unsafe code 
(let !std/list/pair (lambda a b (vector a b)))
(let !std/list/car (lambda pair (get pair 0)))
(let !std/list/cdr (lambda pair (get pair 1)))
(let !std/list/head (lambda pair (get pair 0)))
(let !std/list/tail (lambda pair (get pair 1)))
(let !std/list/nil? (lambda pair (= (length pair) 0)))
(let !std/list/map (lambda xs f (if (!std/list/nil? xs) [] (!std/list/pair (f (!std/list/head xs)) (!std/list/map (!std/list/tail xs) f)))))
(let !std/list/filter (lambda xs f? (if (!std/list/nil? xs) [] (if (f? (!std/list/head xs)) (!std/list/pair (!std/list/head xs) (!std/list/filter (!std/list/tail xs) f?)) (!std/list/filter (!std/list/tail xs) f?)))))
(let !std/list/fold (lambda xs f out (if (!std/list/nil? xs) out (!std/list/fold (!std/list/tail xs) f (f out (!std/list/head xs))))))
(let !std/list/transform (lambda xs f out (if (!std/list/nil? xs) out (!std/list/transform (!std/list/tail xs) f (f out (!std/list/head xs))))))
(let !std/list/zip (lambda a b (if (!std/list/nil? a) [] (!std/list/pair (!std/list/pair (!std/list/head a) (!std/list/pair (!std/list/head b) [])) (!std/list/zip (!std/list/tail a) (!std/list/tail b))))))
(let !std/list/unzip (lambda xs (list (!std/list/map xs (lambda x (!std/list/head x))) (!std/list/map xs (lambda x (!std/list/head (!std/list/tail x)))))))
(let !std/list/length (lambda list (!std/list/fold list (lambda a . (+ a 1)) 0)))
(let !std/list/reverse (lambda list (!std/list/transform list (lambda a b (!std/list/pair b a)) [])))
(let !std/list/some? (lambda xs f? (cond 
                                (!std/list/nil? xs) false
                                (f? (!std/list/head xs)) true
                                (!std/list/some? (!std/list/tail xs) f?))))
(let !std/list/every? (lambda xs f? (cond 
                                  (!std/list/nil? xs) true
                                  (not (f? (!std/list/head xs))) false
                                  (!std/list/every? (!std/list/tail xs) f?))))

(let !std/vector/permutations (lambda arr
  (if (<= (length arr) 1)
      [arr]
      (do
        (let out [])
        (variable i 0)
        (loop (< (get i) (length arr)) (lambda (do
            (let x (get arr (get i)))
            (let rest (std/vector/filter arr (lambda y (!= y x))))
            (let perms (!std/vector/permutations rest))
            (variable j 0)
            (loop (< (get j) (length perms)) (lambda (do
                (set! out (length out) (std/vector/cons! [x] (get perms (get j))))
                (++ j))))
            (++ i))))
        out))))
