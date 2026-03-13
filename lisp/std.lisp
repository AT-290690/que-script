(let std/char/empty (get (string 0) 0))
(let std/char/double-quote (get (string 34) 0))
(let std/char/single-quote (get "'" 0))
(let std/char/new-line (get (string 10) 0))
(let std/char/space (get " " 0))
(let std/char/tab (get "  " 0))
(let std/char/comma (get "," 0))
(let std/char/dot (get "." 0))
(let std/char/semi-colon (get ";" 0))
(let std/char/colon (get ":" 0))
(let std/char/dash (get "-" 0))
(let std/char/lower-dash (get "_" 0))
(let std/char/left-brace (get "(" 0))
(let std/char/right-brace (get ")" 0))
(let std/char/curly-left-brace (get "{" 0))
(let std/char/curly-right-brace (get "}" 0))
(let std/char/left-bracket (get "[" 0))
(let std/char/right-bracket (get "]" 0))
(let std/char/pipe (get "|" 0))
(let std/char/hash (get "#" 0))
(let std/char/question-mark (get "?" 0))
(let std/char/exclamation-mark (get "!" 0))
(let std/char/minus (get "-" 0))
(let std/char/plus (get "+" 0))
(let std/char/equal (get "=" 0))
(let std/char/asterix (get "*" 0))
(let std/char/ampersand (get "&" 0))
(let std/char/at (get "@" 0))
(let std/char/backtick (get "`" 0))

(let nl std/char/new-line)
(let sp std/char/space)
(let ep std/char/empty)
(let dq std/char/double-quote)
(let sq std/char/single-quote)
(let bt std/char/backtick)

(let std/float/floor (lambda n (-. n (mod. n 1.0))))
(let std/float/ceil (lambda n (do 
    (let sign (if (>=. n 0.0) 1 -1))
    (let absn (if (>=. n 0.0) n (-. n)))
    (let frac (mod. absn 1.0))
    (let intpart (-. absn frac))
    (cond (=. n 0.0) n (if (= sign 1) (+. intpart 1.0) (-. intpart))))))

(let std/vector/length (lambda xs (length xs)))
(let std/vector/get (lambda xs i (get xs i)))
(let get/default (lambda xs i def (if (< i (length xs)) (get xs i) def)))
(let std/vector/2d/length std/vector/length)
(let std/vector/2d/get get)
(let std/vector/2d/get/default get/default)
(let std/vector/pop! (lambda xs (pop! xs)))
(let std/vector/set! (lambda xs i x (set! xs i x)))
(let std/vector/swap! (lambda xs i j (do (let temp (get xs i)) (set! xs i (get xs j)) (set! xs j temp))))
(let std/vector/push! (lambda xs x (do (set! xs (length xs) x) xs)))
(let std/vector/pop-and-get! (lambda xs (do 
      (let out (get xs (- (length xs) 1))) 
      (pop! xs)
      out)))
(let std/vector/push-and-get! (lambda xs x (do (set! xs (length xs) x) x)))
(let std/vector/update! (lambda xs i value (do (set! xs i value) xs)))
(let std/vector/tail! (lambda xs (do (pop! xs) xs)))
(let std/vector/append! (lambda xs x (do (std/vector/push! xs x) xs)))
(let std/vector/at (lambda xs i (if (< i 0) (get xs (+ (length xs) i)) (get xs i))))
(let std/vector/first (lambda xs (get xs 0)))
(let std/vector/second (lambda xs (get xs 1)))
(let std/vector/third (lambda xs (get xs 3)))
(let std/vector/last (lambda xs (get xs (- (length xs) 1))))

(let box (lambda value [ value ]))
(let set (lambda vrbl x (set! vrbl 0 x)))
(let =! (lambda vrbl x (set! vrbl 0 x)))
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
(let Bool->Char (lambda x (if (=? x true) '1' '0')))
(let Char->Int (lambda x (if (>=# x std/char/empty) (as x Int) 0)))
(let Char->Bool (lambda x (if (or (=# x std/char/empty) (=# x '0')) false true)))
(let Int->Bool (lambda x 
    (cond 
        (<= x 0) false
        (>= x 1) true
        false)))
(let Int->Char (lambda x (if (>= x 0) (as x Char) std/char/empty)))

(let std/char/digit? (lambda ch (and (>=# ch '0') (<=# ch '9'))))
(let std/char/upper (lambda ch (if (and (>=# ch 'a') (<=# ch 'z')) (-# ch std/char/space) ch)))
(let std/char/lower (lambda ch (if (and (>=# ch 'A') (<=# ch 'Z')) (+# ch std/char/space) ch)))

(let std/float/safe? (lambda value (and (>=. value const/float/min-safe) (<=. value const/float/max-safe))))
(let std/float/get-safe (lambda vrbl (if (std/float/safe? (get vrbl)) (get vrbl) Float)))

(let int (lambda value (if (std/int/safe? value) [ value ] [ 0 ])))
(let float (lambda value (if (std/float/safe? value) [ value ] [ 0.0 ])))
(let bool (lambda value [(=? value true)]))

(let std/int/safe? (lambda value (and (>= value const/int/min-safe) (<= value const/int/max-safe))))
(let std/int/get-safe (lambda vrbl (if (std/int/safe? (get vrbl)) (get vrbl) Int)))


; Extra keywords
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
(let std/fn/apply/first/7 (lambda fn a b c d e f g (fn a b c d e f g)))
(let std/fn/apply/first/8 (lambda fn a b c d e f g h (fn a b c d e f g h)))
(let std/fn/apply/first/9 (lambda fn a b c d e f g h i (fn a b c d e f g h i)))
(let std/fn/apply/first/10 (lambda fn a b c d e f g h i j (fn a b c d e f g h i j)))


(let std/fn/combinator/1 (lambda a x (a x)))
(let std/fn/combinator/2 (lambda a b x (a (b x))))
(let std/fn/combinator/3 (lambda a b c x (a (b (c x)))))
(let std/fn/combinator/4 (lambda a b c d x (a (b (c (d x))))))
(let std/fn/combinator/5 (lambda a b c d e x (a (b (c (d (e x)))))))
(let std/fn/combinator/6 (lambda a b c d e f x (a (b (c (d (e (f x))))))))
(let std/fn/combinator/7 (lambda a b c d e f g x (a (b (c (d (e (f (g x)))))))))
(let std/fn/combinator/8 (lambda a b c d e f g h x (a (b (c (d (e (f (g (h x))))))))))
(let std/fn/combinator/9 (lambda a b c d e f g h i x (a (b (c (d (e (f (g (h (i x)))))))))))

(let std/fn/rev/combinator/1 (lambda a x (a x)))
(let std/fn/rev/combinator/2 (lambda a b x (b (a x))))
(let std/fn/rev/combinator/3 (lambda a b c x (c (b (a x)))))
(let std/fn/rev/combinator/4 (lambda a b c d x (d (c (b (a x))))))
(let std/fn/rev/combinator/5 (lambda a b c d e x (e (d (c (b (a x)))))))
(let std/fn/rev/combinator/6 (lambda a b c d e f x (f (e (d (c (b (a x))))))))
(let std/fn/rev/combinator/7 (lambda a b c d e f g x (g (f (e (d (c (b (a x)))))))))
(let std/fn/rev/combinator/8 (lambda a b c d e f g h x (h (g (f (e (d (c (b (a x))))))))))
(let std/fn/rev/combinator/9 (lambda a b c d e f g h i x (i (h (g (f (e (d (c (b (a x)))))))))))

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

(let std/fn/const (lambda x . x))
(let std/fn/return 1)
(let std/fn/push 2)
(let std/fn/none 0)

(let std/fn/rec (lambda init-frame handler (do 
  (let stack [init-frame])
  (let result [[]])
  (while (not (std/vector/empty? stack)) (do 
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
    )))
  (get result))))


(let Rec/return std/fn/return)
(let Rec/push std/fn/push)
(let Rec/none std/fn/none)
(let Rec std/fn/rec)

(let std/vector/empty? (lambda xs (= (length xs) 0)))
(let std/vector/empty! (lambda xs (if (std/vector/empty? xs) xs (do 
     (loop 0 (length xs) (lambda . (pop! xs)))
     xs))))
(let std/vector/not-empty? (lambda xs (not (= (length xs) 0))))
(let std/vector/in-bounds? (lambda xs index (and (< index (length xs)) (>= index 0))))
(let std/vector/for (lambda xs fn (do
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do (fn (get xs i)) (alter! i (+ i 1)))))))
(let std/vector/for/i (lambda xs fn (do
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do (fn (get xs i) i) (alter! i (+ i 1)))))))
(let std/vector/filter (lambda xs fn? (if (std/vector/empty? xs) xs (do 
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do 
            (let x (get xs i))
            (if (fn? x) (set! out (length out) x))
            (alter! i (+ i 1))))
     out))))

    ;  (mut i 0)
    ;  (while (< i (length xs)) (do ... (alter! i (+ i 1))))
(let std/vector/filter/i (lambda xs fn? (if (std/vector/empty? xs) xs (do 
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do 
            (let x (get xs i))
            (if (fn? x i) (set! out (length out) x))
            (alter! i (+ i 1))))
     out))))

(let std/vector/reduce (lambda xs fn initial (do
     (mut out initial)
     (mut i 0)
     (while (< i (length xs)) (do (alter! out (fn out (get xs i))) (alter! i (+ i 1))))
     out)))

(let std/vector/reduce/i (lambda xs fn initial (do
     (mut out initial)
     (mut i 0)
     (while (< i (length xs)) (do (alter! out (fn out (get xs i) i)) (alter! i (+ i 1))))
     out)))

(let std/vector/map (lambda xs fn (if (std/vector/empty? xs) [] (do
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do (set! out (length out) (fn (get xs i))) (alter! i (+ i 1))))
     out))))

(let std/vector/map/i (lambda xs fn (if (std/vector/empty? xs) [] (do
     (let out [])
     (mut i 0)
     (while (< i (length xs)) (do (set! out (length out) (fn (get xs i) i)) (alter! i (+ i 1))))
     out))))

(let std/vector/reduce/until (lambda xs fn fn? initial (do 
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

(let std/vector/reduce/until/i (lambda xs fn fn? initial (do 
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

(let std/vector/for/until (lambda xs fn fn? (do 
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do 
    (let x (get xs i))
    (unless (fn? x) (do (fn x) nil) (alter! placed true))
    (alter! i (+ i 1)))))))

(let std/vector/for/until/i (lambda xs fn fn? (do 
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do
    (let idx i)
    (let x (get xs idx))
    (unless (fn? x idx) (do (fn x idx) nil) (alter! placed true))
    (alter! i (+ i 1)))))))

(let std/vector/map/until (lambda xs fn fn? (do
  (let out [])
  (mut placed false)
  (mut i 0)
  (let len (length xs))
  (while (and (not placed) (< i len)) (do 
    (let x (get xs i))
    (unless (fn? x) (do (push! out (fn x)) nil) (alter! placed true))
    (alter! i (+ i 1))))
  out)))

(let std/vector/map/until/i (lambda xs fn fn? (do
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

(let std/vector/int/range (lambda start end (do
     (let out [ start ])
     (loop (+ start 1) (+ end 1) (lambda i (set! out (length out) i)))
     out))) 


(let std/vector/float/range (lambda start end (do
     (let out [ (Int->Float start) ])
     (loop (+ start 1) (+ end 1) (lambda i (set! out (length out) (Int->Float i))))
     out))) 
    
 (let std/vector/float/ones (lambda n (do
     (let out [ 1.0 ])
     (loop 1 n (lambda i (set! out (length out) 1.0)))
     out))) 

 (let std/vector/float/zeroes (lambda n (do
     (let out [ 0.0 ])
     (loop 1 n (lambda i (set! out (length out) 0.0)))
     out)))

 (let std/vector/int/ones (lambda n (do
     (let out [ 1 ])
     (loop 1 n (lambda i (set! out (length out) 1)))
     out))) 

 (let std/vector/int/zeroes (lambda n (do
     (let out [ 0 ])
     (loop 1 n (lambda i (set! out (length out) 0)))
     out)))

(let std/vector/3d/int/range (lambda s w h (do 
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

(let std/vector/2d/int/range std/vector/int/range)
(let std/vector/2d/float/range std/vector/float/range)

(let std/vector/char/blanks (lambda n (do
    (let out [ std/char/empty ])
    (loop 1 n (lambda i (set! out (length out) std/char/empty)))
    out))) 

(let std/vector/int/all-equal? (lambda xs (do (let x (get xs 0)) (std/vector/every? xs (lambda y (= y x))))))
(let std/vector/float/all-equal? (lambda xs (do (let x (get xs 0)) (std/vector/every? xs (lambda y (=. y x))))))
(let std/vector/char/all-equal? (lambda xs (do (let x (get xs 0)) (std/vector/every? xs (lambda y (=# y x))))))
(let std/vector/bool/all-equal? (lambda xs (do (let x (get xs 0)) (std/vector/every? xs (lambda y (=? y x))))))

(let all-equal/int? std/vector/int/all-equal?)
(let all-equal/float? std/vector/float/all-equal?)
(let all-equal/char? std/vector/char/all-equal?)
(let all-equal/bool? std/vector/bool/all-equal?)

(let std/vector/count-of (lambda xs fn? (length (std/vector/filter xs fn?))))
(let std/vector/int/count (lambda xs item (std/vector/count-of xs (lambda x (= x item)))))
(let std/vector/float/count (lambda xs item (std/vector/count-of xs (lambda x (=. x item)))))
(let std/vector/char/count (lambda xs item (std/vector/count-of xs (lambda x (=# x item)))))
(let std/vector/bool/count (lambda xs item (std/vector/count-of xs (lambda x (=? x item)))))

(let std/vector/2d/count-of std/vector/count-of)
(let std/vector/2d/int/count std/vector/int/count)
(let std/vector/2d/char/count std/vector/char/count)
(let std/vector/2d/bool/count std/vector/bool/count)

(let std/vector/3d/count-of (lambda xs fn? (<| xs (std/vector/map (lambda ys (std/vector/2d/count-of ys fn?))) (std/vector/int/sum))))
(let std/vector/3d/int/count (lambda xs x (<| xs (std/vector/map (lambda ys (std/vector/2d/int/count ys x))) (std/vector/int/sum))))
(let std/vector/3d/char/count (lambda xs x (<| xs (std/vector/map (lambda ys (std/vector/2d/char/count ys x))) (std/vector/int/sum))))
(let std/vector/3d/bool/count (lambda xs x (<| xs (std/vector/map (lambda ys (std/vector/2d/bool/count ys x))) (std/vector/int/sum))))

(let std/vector/cons (lambda a b (cond (std/vector/empty? a) b (std/vector/empty? b) a (do 
  (let out []) 
  (loop 0 (length a) (lambda i (set! out (length out) (get a i)))) 
  (loop 0 (length b) (lambda i (set! out (length out) (get b i)))) 
  out))))

(let std/vector/cons! (lambda a b (if (and (std/vector/empty? a) (std/vector/empty? b)) a (do 
  (loop 0 (length b) (lambda i (set! a (length a) (get b i)))) 
  a))))

(let std/vector/concat (lambda xs (std/vector/reduce xs std/vector/cons [])))
(let std/vector/concat! (lambda xs os (std/vector/reduce os std/vector/cons! xs)))

(let std/vector/every? (lambda xs predicate? (do
           (mut i 0)
           (let len (length xs))
           (while (and (< i len) (predicate? (get xs i))) (alter! i (+ i 1)))
           (not (> len i)))))

(let std/vector/some? (lambda xs predicate? (do
           (mut i 0)
           (let len (length xs))
           (while (and (< i len) (not (predicate? (get xs i)))) (alter! i (+ i 1)))
           (or (= len 0) (> len i)))))

(let std/vector/every/i? (lambda xs predicate? (do
           (mut i 0)
           (let len (length xs))
           (while (and (< i len) (predicate? (get xs i) i)) (alter! i (+ i 1)))
           (not (> len i)))))

(let std/vector/some/i? (lambda xs predicate? (do
           (mut i 0)
           (let len (length xs))
           (while (and (< i len) (not (predicate? (get xs i) i))) (alter! i (+ i 1)))
           (or (= len 0) (> len i)))))

(let std/vector/cartesian-product (lambda a b (std/vector/reduce a (lambda p x (std/vector/cons p (std/vector/map b (lambda y { x y })))) [])))

(let std/int/gcd (lambda a b (do
    (mut A a)
    (mut B b)
    (while (> B 0) (do
        (let a A)
        (let b B)
        (alter! A b)
        (alter! B (mod a b))))
    A)))
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

(let std/float/positive? (lambda x (>. x 0.)))
(let std/float/negative? (lambda x (<. x 0.)))
(let std/float/invert (lambda x (-. x)))
(let std/float/zero? (lambda x (=. x 0.)))
(let std/float/one? (lambda x (=. x 1.)))
(let std/float/negative-one? (lambda x (=. x -1.)))
(let std/float/divisible? (lambda a b (=. (mod. a b) 0.)))


(let std/int/square (lambda x (* x x)))
(let std/int/even? (lambda x (= (mod x 2) 0)))
(let std/int/odd? (lambda x (not (= (mod x 2) 0))))
(let std/float/even? (lambda x (=. (mod. x 2.) 0.)))
(let std/float/odd? (lambda x (not (=. (mod. x 2.) 0.))))
(let std/vector/int/sum (lambda xs (std/vector/reduce xs (lambda a b (+ a b)) 0)))
(let std/vector/float/sum (lambda xs (std/vector/reduce xs (lambda a b (+. a b)) 0.0)))

(let std/vector/int/product (lambda xs (std/vector/reduce xs (lambda a b (* a b)) 1)))
(let std/vector/float/product (lambda xs (std/vector/reduce xs (lambda a b (*. a b)) 1.0)))
(let std/int/mul (lambda a b (* a b)))
(let std/int/add (lambda a b (+ a b)))
(let std/int/div (lambda a b (/ a b)))
(let std/int/sub (lambda a b (- a b)))
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
(let std/vector/float/median (lambda xs (do
  (let len (Int->Float (length xs)))
  (let half (/. len 2.))
  (let mid (Float->Int half))
  (if (std/float/odd? len)
      (get xs mid)
      (/. (+. (get xs (Float->Int (-. half 1.))) (get xs mid)) 2.)))))
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
(let std/int/expt (lambda base exp (do
  (if (< exp 0) 0 (do
      (mut result 1)
      (mut b base)
      (mut e exp)
      (while (> e 0) (do
          (if (= (mod e 2) 1)
            (alter! result (* result b)))
            (alter! b (* b b))
            (alter! e (/ e 2))))
      result)))))
; a helper for infix ^ power
; has to be data first
(let iexpt std/int/expt)
(let std/float/sqrt (lambda n
  (do
    (floating low 0.)
    (floating high n)
    (floating mid 0.)
    (floating i 0.)
    ; Loop 100 times for high precision
    (while (<. (get i) 100.)
      (do
        (set mid (/. (+. (get low) (get high)) 2.))
        (if (<=. (*. (get mid) (get mid)) n)
            (set low (get mid))
            (set high (get mid)))
        (set i (+. (get i) 1.))))
    (get low))))

(let std/float/expt (lambda base exp
  (do
    (floating res 1.)
    (floating b base)
    (floating e exp)
    
    ; 1. Handle the integer part of the exponent
    (while (>=. (get e) 1.)
      (do
        (if (=. (mod. (floor (get e)) 2.) 1.)
            (set res (*. (get res) (get b))))
        (set b (*. (get b) (get b)))
        (set e (/. (floor (get e)) 2.))))
    
    ; 2. Handle the fractional part using square roots
    ; Refresh 'b' to original base and 'e' to the remaining fraction
    (set b base)
    (set e (-. exp (floor exp)))
    (floating root (std/float/sqrt (get b)))
    (floating frac 0.5)
    
    ; Loop 22. times for precision (handles bits of the fraction)
    (floating i 0.)
    (while (<. (get i) 22.)
      (do
        (if (>=. (get e) (get frac))
            (do 
              (set res (*. (get res) (get root)))
              (set e (-. (get e) (get frac)))))
        (set root (std/float/sqrt (get root)))
        (set frac (/. (get frac) 2.))
        (set i (+. (get i) 1.))))
    (get res))))


(let std/int/delta (lambda a b (std/int/abs (- a b))))
(let std/float/delta (lambda a b (std/float/abs (-. a b))))

(let std/vector/map/adjacent (lambda xs fn (if (std/vector/empty? xs) [] (do 
  (do 
    (let out [])
    (loop 1 (length xs) (lambda i (std/vector/push! out (fn (get xs (- i 1)) (get xs i)))))
    out)))))

(let std/vector/zipper (lambda a b (do 
      (mut i 1)
      (let len (length a))
      (let out [[(get a 0) (get b 0)]])
      (while (< i len) (do (set! out (length out) [(get a i) (get b i)]) (alter! i (+ i 1))))
      out)))

(let std/vector/zip (lambda xs (std/vector/zipper (std/vector/first xs) (std/vector/second xs))))
(let std/vector/unzip (lambda xs [ (std/vector/map xs std/vector/first) (std/vector/map xs std/vector/second) ]))

(let std/vector/tuple/zipper (lambda a b (do
      (mut i 1)
      (let len (length a))
      (let out [{ (get a 0) (get b 0) }])
      (while (< i len) (do (set! out (length out) { (get a i) (get b i) }) (alter! i (+ i 1))))
      out)))

(let std/vector/tuple/zip (lambda xs (std/vector/tuple/zipper (fst xs) (snd xs))))
(let std/vector/tuple/unzip (lambda xs { (std/vector/map xs (lambda x (fst x))) (std/vector/map xs (lambda x (snd x))) }))
(let std/vector/tuple/zip-with (lambda a b f (do 
    (let out [])
    (mut i 0)
    (let len (length a))
    (while (< i len) (do (set! out (length out) (f (get a i) (get b i))) (alter! i (+ i 1))))
    out)))

(let std/vector/rest (lambda xs start (if (std/vector/empty? xs) xs (do
     (let end (length xs))
     (let bounds (- end start))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs (+ start i))) (alter! i (+ i 1))))
     out))))
     
(let std/vector/slice (lambda xs start end (if (std/vector/empty? xs) xs (do
     (let bounds (- end start))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs (+ start i))) (alter! i (+ i 1))))
     out))))

(let std/vector/drop (lambda xs start (if (std/vector/empty? xs) xs (do
     (let end (length xs))
     (let bounds (- end start))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs (+ start i))) (alter! i (+ i 1))))
     out))))

(let std/vector/drop/last (lambda xs end (if (std/vector/empty? xs) xs (do
     (let bounds (- (length xs) end))
     (let out [])
     (mut i 0)
     (while (< i bounds) (do (set! out (length out) (get xs i)) (alter! i (+ i 1))))
     out))))

(let std/vector/take (lambda xs end (if (std/vector/empty? xs) xs (do
     (let out [])
     (mut i 0)
     (while (< i end) (do (set! out (length out) (get xs i)) (alter! i (+ i 1))))
     out))))

(let std/vector/take/last (lambda xs start (if (std/vector/empty? xs) xs (do
     (let out [])
     (let len (length xs))
     (mut i (- len start))
     (while (< i len) (do (set! out (length out) (get xs i)) (alter! i (+ i 1))))
     out))))

(let std/vector/reverse (lambda xs (if (std/vector/empty? xs) xs (do
     (let out [])
     (let len (length xs))
     (mut i 0)
     (while (< i len) (do (set! out (length out) (get xs (- len i 1))) (alter! i (+ i 1))))
     out))))

(let std/vector/reverse! (lambda xs (loop 0 (/ (length xs) 2) (lambda i (std/vector/swap! xs i (- (length xs) i 1))))))

(let std/vector/find-index (lambda xs fn? (do
     (mut i 0)
     (mut index -1)
     (let len (length xs))
     (while (and (< i len) (= index -1)) (if (fn? (get xs i))
              (alter! index i)
              (alter! i (+ i 1))))
     index)))

(let std/vector/buckets (lambda size (do
     (let out [[]])
     (mut i 1)
     (while (< i size) (do (set! out (length out) []) (alter! i (+ i 1))))
     out)))

(let std/vector/char/equal? (lambda a b (do
    (let len-a (length a))
    (let len-b (length b))
    (if (<> len-a len-b) false (do
        (mut i 0)
        (mut same true)
        (while (and same (< i len-a)) (do
            (if (not (=# (get a i) (get b i))) (alter! same false) nil)
            (alter! i (+ i 1))))
        same)))))

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
    
(let std/vector/char/match? std/vector/char/equal?)
(let std/vector/char/greater-or-equal? (lambda A B (or (std/vector/char/equal? A B) (std/vector/char/greater? A B))))
(let std/vector/char/lesser-or-equal? (lambda A B (or (std/vector/char/equal? A B) (std/vector/char/lesser? A B))))
(let std/vector/char/negative? (lambda str (=# (std/vector/first str) std/char/minus)))

(let std/vector/partition (lambda xs n (if (= n (length xs)) [xs] (do 
    (let a [])
    (mut i 0)
    (let len (length xs))
    (while (< i len) (do (if (= (mod i n) 0)
        (set! a (length a) [(get xs i)])
        (set! (std/vector/at a -1) (length (std/vector/at a -1)) (get xs i)))
        (alter! i (+ i 1))))
     a))))

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
    (let handler (lambda frame
      (do
        (let rng (fst frame))
        (let vec (snd frame))
        (let low (fst rng))
        (let high (snd rng))
        (if (>= low high) {std/fn/none []} 
            (do
              (let pivot (get vec high))
              (let i (box low))
              (let j (box low))
              (while (< (get j 0) high) (do
                  (if (fn (get vec (get j 0)) pivot)
                      (do (std/vector/swap! vec (get i 0) (get j 0)) (++ i))
                      nil)
                  (++ j)))
              (std/vector/swap! vec (get i 0) high)
              (let p (get i 0))
              {std/fn/push [{{low (- p 1)} vec} {{(+ p 1) high} vec}]})))))
    (std/fn/rec init-frame handler)
    v)))

(let std/vector/sliding-window (lambda xs size (cond 
     (std/vector/empty? xs) []
     (= size (length xs)) [xs]
     (std/vector/reduce/i xs (lambda a b i (if (> (+ i size) (length xs)) a (std/vector/cons a [(std/vector/slice xs i (+ i size))]))) []))))

(let std/vector/flat-one (lambda xs (cond 
     (std/vector/empty? xs) []
     (= (length xs) 1) (get xs)
     (std/vector/reduce xs (lambda a b (std/vector/cons a b)) []))))
(let std/vector/flat/length (lambda matrix (length (std/vector/flat-one matrix))))

(let std/convert/char->digit (lambda digit (if (<# digit '0') 0 (- (as digit Int) (as '0' Int)))))
(let std/convert/chars->digits (lambda digits (std/vector/map digits std/convert/char->digit)))
(let std/convert/digit->char (lambda digit (if (< digit 0) '0' (+# (as digit Char) '0'))))
(let std/convert/digits->chars (lambda digits (std/vector/map digits std/convert/digit->char)))
(let std/convert/bool->int (lambda x (if (=? x true) 1 0)))
(let std/convert/int->bool (lambda x (if (= x 0) false true)))
(let std/convert/vector->string (lambda xs delim (std/vector/reduce/i xs (lambda a b i (if (> i 0) (std/vector/cons (std/vector/append! a delim) b) b)) "")))
(let std/convert/string->vector (lambda str ch (<| str
              (std/vector/reduce(lambda a b (do
              (let prev (std/vector/at a -1))
                (if (std/vector/char/equal? [b] [ch])
                    (set! a (length a) [])
                    (set! prev (length prev) b)) a))
              [[]])
              (std/vector/map (lambda x (std/convert/vector->string [ x ] std/char/empty))))))

(let std/convert/positive-or-negative-digits->integer (lambda digits-with-sign (do
    (let std/int/negative? (< (std/vector/first digits-with-sign) 0))
    (let digits (if std/int/negative? (std/vector/map digits-with-sign std/int/abs) digits-with-sign))
    (mut num 0)
    (mut base (/ (std/int/expt 10 (length digits)) 10))
    (mut i 0)
    (while (< i (length digits)) (do 
      (alter! num (+ num (* base (get digits i))))
      (alter! base (/ base 10))
      (alter! i (+ i 1))))
    (alter! num (* num (if std/int/negative? -1 1)))
    num)))

(let std/convert/chars->positive-or-negative-digits (lambda chars (do
    (integer current-sign 1)
    (<| chars 
        (std/vector/reduce (lambda a ch (do 
            (if (=# ch std/char/minus) 
                (set current-sign -1) 
                (do  
                    (std/vector/push! a (* (get current-sign) (std/convert/char->digit ch))) 
                    (set current-sign 1)))
                a)) [])))))
(let std/convert/digits->integer std/convert/positive-or-negative-digits->integer)
(let std/convert/positive-or-negative-chars->integer (lambda x (<| x (std/convert/chars->positive-or-negative-digits) (std/convert/positive-or-negative-digits->integer))))
(let std/convert/chars->integer std/convert/positive-or-negative-chars->integer)

(let std/convert/chars->digits/float (lambda xs
    (<| xs 
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

  (let std/vector/tuple/unique-pairs (lambda xs (do 
    (let pairs [])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do 
        (mut j (+ i 1))
        (while (< j len) (do 
            (std/vector/push! pairs { (get xs i) (get xs j) })
            (alter! j (+ j 1))))
        (alter! i (+ i 1))))
    pairs)))

(let std/vector/int/unique (lambda xs 
    (if (= (length xs) 0) 
        [(+ (get xs 0) 0)] 
        (<| xs (std/vector/map (lambda x [(as x Char)])) (std/convert/vector->set) (std/convert/set->vector) (std/vector/map (lambda x (as (get x 0) Int)))))))


(let std/vector/char/unique (lambda xs 
    (if (= (length xs) 0) 
        xs 
        (<| xs (std/vector/map (lambda x [x])) (std/convert/vector->set) (std/convert/set->vector) (std/vector/map (lambda x (get x 0)))))))

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


(let std/node/parent (lambda i (- (>> (+ i 1) 1) 1)))
(let std/node/left (lambda i (+ (<< i 1) 1)))
(let std/node/right (lambda i (<< (+ i 1) 1)))

(let std/heap/top 0)
(let std/heap/greater? (lambda heap i j fn? (=? (fn? (get heap i) (get heap j)) true)))
(let std/heap/sift-up! (lambda heap fn (do 
  (integer node (- (length heap) 1))
  (let* tail-call/std/heap/sift-up! (lambda heap
    (if (and (> (get node) std/heap/top) (std/heap/greater? heap (get node) (std/node/parent (get node)) fn))
      (do 
        (std/vector/swap! heap (get node) (std/node/parent (get node)))
        (set node (std/node/parent (get node)))
        (tail-call/std/heap/sift-up! heap)) heap)))
  (tail-call/std/heap/sift-up! heap))))

(let std/heap/sift-down! (lambda heap fn (do
  (integer node std/heap/top)
  (let* tail-call/std/heap/sift-down! (lambda heap
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
  (if (> bottom std/heap/top) (do (std/vector/swap! heap std/heap/top bottom) heap) heap)
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

(let std/convert/integer->string-base (lambda num base  
    (if (= num 0) "0" (do 
        (let neg? (< num 0))
        (integer n (if neg? (* num -1) num))
        (let* tail-call/while (lambda out
            (if (> (get n) 0) (do
                (let x (mod (get n) base))
                (std/vector/push! out x)
                (set n (/ (get n) base))
                (tail-call/while out)) out)))
        (let str (std/convert/digits->chars (tail-call/while [])))
        (std/vector/reverse (if neg? (std/vector/append! str std/char/dash) str))))))
(let std/convert/integer->string (lambda x (std/convert/integer->string-base x 10)))
(let std/convert/vector->set (lambda xs (std/vector/reduce xs (lambda s x (do (std/vector/hash/set/add! s x) s)) [ [] [] [] [] [] [] [] ])))

(let std/integer/decimal-scaling 1000000)
(let std/float/decimal-scaling 1000000.0)

(let std/convert/float->string (lambda x (if (=. (std/float/floor x) x) (cons (std/convert/integer->string (Float->Int x)) ".0") (do 
    (let flip (if (<. x 0.0) -1.0 1.0))
    (let exponent (std/float/floor x))
    (let mantisa (-. x exponent))
    (let left (std/convert/integer->string (Float->Int exponent)))
    (let right (std/convert/integer->string (Float->Int (*. mantisa std/float/decimal-scaling flip))))
    (let len (length right))
    (let* tail-call/while (lambda i 
        (if (=# (get right (- len i)) '0') (do 
            (pop! right)
            (tail-call/while (+ i 1))) 
        i)))
    (tail-call/while 1)
    (cons left [std/char/dot] right)))))

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
  (let* tail-call/std/vector/deque/iter (lambda index bounds (do
      (fn (std/vector/deque/get q index))
      (if (< index bounds) (tail-call/std/vector/deque/iter (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/iter 0 (std/vector/deque/length q)))))
(let std/vector/deque/map (lambda q fn (do
  (let result (std/vector/deque/new))
  (let len (std/vector/deque/length q))
  (let half (/ len 2))
  (let* tail-call/left/std/vector/deque/map (lambda index (do
    (std/vector/deque/add-to-left! result (fn (std/vector/deque/get q index)))
   (if (> index 0) (tail-call/left/std/vector/deque/map (- index 1)) Int))))
 (tail-call/left/std/vector/deque/map (- half 1))
(let* tail-call/right/std/vector/deque/map (lambda index bounds (do
   (std/vector/deque/add-to-right! result (fn (std/vector/deque/get q index)))
   (if (< index bounds) (tail-call/right/std/vector/deque/map (+ index 1) bounds) Int))))
 (tail-call/right/std/vector/deque/map half (- len 1))
 result)))
(let std/vector/deque/balance? (lambda q (= (+ (std/vector/deque/offset-right q) (std/vector/deque/offset-left q)) 0)))
(let std/convert/vector->deque (lambda initial (do
 (let q (std/vector/deque/new))
 (let half (/ (length initial) 2))
 (let* tail-call/left/from/vector->deque (lambda index (do
    (std/vector/deque/add-to-left! q (get initial index))
   (if (> index 0) (tail-call/left/from/vector->deque (- index 1)) Int))))
 (tail-call/left/from/vector->deque (- half 1))
(let* tail-call/right/from/vector->deque (lambda index bounds (do
   (std/vector/deque/add-to-right! q (get initial index))
   (if (< index bounds) (tail-call/right/from/vector->deque (+ index 1) bounds) Int))))
 (tail-call/right/from/vector->deque half (- (length initial) 1))
    q)))
(let std/convert/deque->vector (lambda q (if (std/vector/deque/empty? q) [(get q 0 0)] (do
  (let out [])
  (let* tail-call/from/deque->vector (lambda index bounds (do
      (set! out (length out) (std/vector/deque/get q index))
      (if (< index bounds) (tail-call/from/deque->vector (+ index 1) bounds) Int))))
    (tail-call/from/deque->vector 0 (- (std/vector/deque/length q) 1))
    out))))
(let std/vector/deque/balance! (lambda q
    (if (std/vector/deque/balance? q) q (do
      (let initial (std/convert/deque->vector q))
      (std/vector/deque/empty! q)
      (let half (/ (length initial) 2))
      (let* tail-call/left/std/vector/deque/balance! (lambda index (do
        (std/vector/deque/add-to-left! q (get initial index))
        (if (> index 0) (tail-call/left/std/vector/deque/balance! (- index 1)) Int))))
      (let* tail-call/right/std/vector/deque/balance! (lambda index bounds (do
        (std/vector/deque/add-to-right! q (get initial index))
        (if (< index bounds) (tail-call/right/std/vector/deque/balance! (+ index 1) bounds) Int))))
      (tail-call/right/std/vector/deque/balance! half (- (length initial) 1))
      (if (> (length initial) 1) (tail-call/left/std/vector/deque/balance! (- half 1)) Int)
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
  (let N (mod n (std/vector/deque/length q)))
  (let* tail-call/std/vector/deque/rotate-left! (lambda index bounds (do
      (if (= (std/vector/deque/offset-left q) 0) (std/vector/deque/balance! q) q)
      (std/vector/deque/add-to-right! q (std/vector/deque/first q))
      (std/vector/deque/remove-from-left! q)
      (if (< index bounds) (tail-call/std/vector/deque/rotate-left! (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/rotate-left! 0 N) q)))
(let std/vector/deque/rotate-right! (lambda q n (do
  (let N (mod n (std/vector/deque/length q)))
  (let* tail-call/std/vector/deque/rotate-left! (lambda index bounds (do
      (if (= (std/vector/deque/offset-right q) 0) (std/vector/deque/balance! q) q)
      (std/vector/deque/add-to-left! q (std/vector/deque/last q))
      (std/vector/deque/remove-from-right! q)
      (if (< index bounds) (tail-call/std/vector/deque/rotate-left! (+ index 1) bounds) Int))))
    (tail-call/std/vector/deque/rotate-left! 0 N) q)))
(let std/vector/deque/slice (lambda entity s e (do
  (let len (std/vector/deque/length entity))
  (let start (if (< s 0) (std/int/max (+ len s) 0) (std/int/min s len)))
  (let end (if (< e 0) (std/int/max (+ len e) 0) (std/int/min e len)))
  (let scl (std/vector/deque/new))
  (let slice-len (std/int/max (- end start) 0))
  (let half (/ slice-len 2))
  (let* tail-call/left/std/vector/deque/slice (lambda index (do
      (std/vector/deque/add-to-left! scl (std/vector/deque/get entity (+ start index)))
      (if (> index 0) (tail-call/left/std/vector/deque/slice (- index 1)) Int))))
  (tail-call/left/std/vector/deque/slice (- half 1))
  (let* tail-call/right/std/vector/deque/slice (lambda index bounds (do
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

(let std/vector/float/equal? (lambda a b (do
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
        (integer n num)
        (let* tail-call/while (lambda out
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
        (std/vector/push! out (<|
                i
                (std/convert/integer->bits)
                (std/vector/reduce/i (lambda a x i
                    (if (= x 1) (std/vector/append! a (get xs i)) a)) [])))))
    out))))

(let std/vector/flat-map (lambda xs (std/vector/map (std/vector/flat-one xs))))
    
; alternative implementation using bitwise operators
; (let std/convert/bits->integer (lambda bits (std/vector/reduce bits (lambda value bit (| (<< value 1) (& bit 1))) 0)))

(let std/convert/bits->integer (lambda xs (do
  (let* tail-call/bits->integer (lambda index out (if
                              (= index (length xs)) out
                              (tail-call/bits->integer (+ index 1) (+ out (* (std/vector/at xs index) (std/int/expt 2 (- (length xs) index 1))))))))
  (tail-call/bits->integer 0 0))))



(let std/vector/copy (lambda xs (std/vector/map xs identity)))

(let std/int/reduce (lambda n fn acc (do 
    (let* tail-call/fold-n (lambda i out (if (< i n) (tail-call/fold-n (+ i 1) (fn out i)) out)))
    (tail-call/fold-n 0 acc))))
(let std/vector/2d/fill (lambda n fn (do 
  (let* tail-call/std/vector/fill (lambda i xs (if (= i 0) xs (tail-call/std/vector/fill (- i 1) (std/vector/cons! xs (vector (fn i)))))))
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
  (let rowsA (get dimsA 0))
  (let colsA (get dimsA 1))
  (let rowsB (get dimsB 0))
  (let colsB (get dimsB 1))
  (if (= colsA rowsB) (std/vector/3d/fill rowsA colsB (lambda i j
      (std/int/reduce colsA (lambda sm k (+ sm (* (get A i k) (get B k j)))) 0))) []))))
(let std/vector/3d/dot-product (lambda a b (do
  (let lenA (length a))
  (let lenB (length b))
  (if (= lenA lenB)
    (std/int/reduce lenA (lambda sm i (+ sm (* (get a i) (get b i)))) 0) Int))))

(let std/vector/3d/rotate (lambda matrix (if (std/vector/empty? matrix) matrix (do 
    (let H (length matrix))
    (let W (length (get matrix 0)))
    (let out [])
    (mut i 0)
    (while (< i W) (do
        (mut j 0)
        (std/vector/push! out [])
        (while (< j H) (do
          (std/vector/push! (std/vector/at out -1) (get matrix j i))
          (alter! j (+ j 1))))
        (alter! i (+ i 1))))
    out))))

(let std/vector/2d/interleave (lambda xs ys (do 
  (let out [])
  (loop 0 (std/int/min (length xs) (length ys)) (lambda i (do 
    (std/vector/push! out (get xs i))
    (std/vector/push! out (get ys i)))))
  out)))

(let std/vector/intersperse (lambda xs x (if (std/vector/empty? xs) [] (do 
  (let out [])
  (loop 0 (- (length xs) 1) (lambda i (do
    (std/vector/push! out (get xs i)) 
    (std/vector/push! out x))))
   (std/vector/push! out (get xs (- (length xs) 1))) 
  out))))

(let std/vector/int/sequence (lambda xs (std/vector/int/range 0 (- (length xs) 1))))
(let std/int/shoelace (lambda px (do
    (let len (length px))
    (/ (<| (std/vector/int/sequence px)
        (std/vector/reduce (lambda ab i (do
            (let a (get ab 0))
            (let b (get ab 1))
            (let left (get px i))
            (let right (get px (mod (+ i 1) len)))
            (let y1 (get left 0))
            (let x1 (get left 1))
            (let y2 (get right 0))
            (let x2 (get right 1))
            [(+ a (* y1 x2)) (+ b (* y2 x1))])) 
        [0 0])
        (std/vector/int/pair/sub)
        (std/int/abs)) 2))))
(let std/int/collinear? (lambda px (= (std/int/shoelace px) 0)))


(let std/vector/int/big/range (lambda start end (do
     (let out [ (std/int/big/new (std/convert/integer->string start)) ])
     (loop (+ start 1) (+ end 1) (lambda i (set! out (length out) (std/int/big/new (std/convert/integer->string i)))))
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
    (let sm (+ digit-A digit-B (get carry)))
    (std/vector/push! result (mod sm 10))
    (set carry (/ sm 10)))
  ))
  ; Handle remaining carry
  (while (> (get carry) 0) (do
    (std/vector/push! result (mod (get carry) 10))
    (set carry (/ (get carry) 10))))
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
  (while (and (> (get i) 0) (= (get result (get i)) 0)) (do
    (pop! result)
    (set i (- (get i) 1))))
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
    (while (> (get carry) 0) (do
      (if (not (< (get k) (length result))) (do (std/vector/push! result 0) nil) nil)
      (let sm (+ (get result (get k)) (get carry)))
      (set! result (get k) (mod sm 10))
      (set carry (/ sm 10))
      (set k (+ (get k) 1)))))))
  ; Remove trailing zeros (from the most significant end), but keep at least one digit
  (integer i (- (length result) 1))
  (while (and (> (get i) 0) (= (get result (get i)) 0) (> (length result) 1)) (do
    (pop! result)
    (set i (- (get i) 1))))
  (std/vector/reverse result))))

(let std/vector/int/remove-leading-zeroes (lambda digits (do
  (boolean tr true)
  (<| digits (std/vector/reduce (lambda a b (if
  (and (true? tr) (std/int/zero? b)) a
    (do
      (if (true? tr) (set tr false))
      (std/vector/cons! a [b])))) [])))))

(let std/int/big/less-or-equal? (lambda a b (do
  (if (< (length a) (length b)) true
  (if (> (length a) (length b)) false
    ; Equal length, compare digit by digit
    (do
      (integer i 0)
      (boolean result true) ; assume a <= b
      (while (< (get i) (length a)) (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (< da db) (do
          (set result true)
          (set i (length a))))
        (if (> da db) (do
          (set result false)
          (set i (length a))))
        (set i (+ (get i) 1))))
      (if (true? result) true false)))))))

(let std/int/big/greater-or-equal? (lambda a b (do
  (if (> (length a) (length b)) true
  (if (< (length a) (length b)) false
    ; Equal length, compare digit by digit
    (do
      (integer i 0)
      (boolean result true) ; assume a >= b
      (while (< (get i) (length a)) (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (> da db) (do
          (set result true)
          (set i (length a))))
        (if (< da db) (do
          (set result false)
          (set i (length a))))
        (set i (+ (get i) 1))))
      (if (true? result) true false)))))))

(let std/int/big/less-than? (lambda a b (do
  (if (< (length a) (length b)) true
  (if (> (length a) (length b)) false
    ; Equal length, check for strict less (not equal)
    (do
      (integer i 0)
      (boolean found-less false) ; true if a < b at some digit
      (while (< (get i) (length a)) (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (< da db) (do
          (set found-less true)
          (set i (length a))))
        (if (> da db) (do
          (set i (length a)))) ; stop on a > b, keep found-less false
        (set i (+ (get i) 1))))
      (if (true? found-less) true false)))))))

(let std/int/big/greater-than? (lambda a b (do
  (if (> (length a) (length b)) true
  (if (< (length a) (length b)) false
    ; Equal length, check for strict greater (not equal)
    (do
      (integer i 0)
      (boolean found-greater false) ; true if a > b at some digit
      (while (< (get i) (length a)) (do
        (let da (get a (get i)))
        (let db (get b (get i)))
        (if (> da db) (do
          (set found-greater true)
          (set i (length a))))
        (if (< da db) (do
          (set i (length a)))) ; stop on a < b, keep found-greater false
        (set i (+ (get i) 1))))
      (if (true? found-greater) true false)))))))

(let std/int/big/equal? std/vector/int/equal?)

(let std/int/big/div (lambda dividend divisor (do
  (let result (as [] [Int]))
  (let current [[]])
  (let len (length dividend))
  (integer i 0)
  ; Main loop/ process each digit of the dividend
  (while (< (get i) len) (do
    (let digit (get dividend (get i)))
    (set current (std/vector/int/remove-leading-zeroes (std/vector/cons (get current) [ digit ])))
    ; Find max digit q such that (divisor * q) <= current
    (integer low 0)
    (integer high 9)
    (integer q 0)
    (while (<= (get low) (get high)) (do
      (let mid (/ (+ (get low) (get high)) 2))
      (let prod (std/int/big/mul divisor [ mid ]))
      (if (std/int/big/less-or-equal? prod (get current))
        (do
          (set q mid)
          (set low (+ mid 1)))
        (set high (- mid 1)))))

    (std/vector/push! result (get q))

    ; current /= current - (divisor * q)
    (let sub (std/int/big/mul divisor [ (get q) ]))
    (set current (std/int/big/sub (get current) sub))
    (++ i)))
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
  (while (< (get p) pow) (do
    (set carry 0) ; Reset carry to 0
    (loop 0 (length digits) (lambda exp (do
      (let prod (+ (* (get digits exp) n) (get carry)))
      (let new-carry (/ prod 10))
      (set! digits exp (mod prod 10))
      ; Update carry using variable helper
      (set carry new-carry))))
    ; Handle carry
    (while (> (get carry) 0) (do
      (std/vector/push! digits (mod (get carry) 10))
      ; Update carry using variable helper
      (/= carry 10)))
    ; Increment p using variable helper
    (++ p)))
  (std/vector/reverse digits))))

(let std/int/big/pow (lambda a b (if (= b 0) [ 1 ] (do 
    (variable out a)
    (loop 0 (- b 1) (lambda . (set out (std/int/big/mul (get out) a))))
    (get out)))))

(let std/int/big/expt (lambda a b (if (and (= (length b) 1) (= (get b 0) 0)) [ 1 ] (do 
    (variable out a)
    (variable expt (std/int/big/sub b [ 1 ]))
    (while (not (and (= (length (get expt)) 1) (= (get (get expt) 0) 0))) (do 
      (set out (std/int/big/mul (get out) a))
      (set expt (std/int/big/sub (get expt) [ 1 ]))))
    (get out)))))

(let std/convert/integer->digits-base (lambda num base  
    (if (= num 0) [ 0 ] (do 
        (integer n num)
        (let* tail-call/while (lambda out
            (if (> (get n) 0) (do
                (std/vector/push! out (mod (get n) base))
                (set n (/ (get n) base))
                (tail-call/while out)) out)))
        (let digits (tail-call/while []))
        (std/vector/reverse digits)))))

(let std/convert/integer->digits (lambda num (std/convert/integer->digits-base num 10)))
(let std/vector/adjacent-difference (lambda xsi fn (do
  (let len (length xsi))
  (let xs (std/vector/copy xsi))
  (mut i 1)
  (while (< i len) (do
    (std/vector/update! xs i (fn (get xs (- i 1)) (get xs i)))
    (alter! i (+ i 1))))
  xs)))

(let std/vector/adjacent-difference! (lambda xs fn (do
  (let len (length xs))
  (unless (= len 1)
    (do
      (mut i 1)
      (while (< i len) (do
        (std/vector/update! xs i (fn (get xs (- i 1)) (get xs i)))
        (alter! i (+ i 1))))
      nil)))))


(let std/convert/vector/3d->string (lambda xs a b (std/convert/vector->string (std/vector/map xs (lambda x (std/convert/vector->string x b))) a)))
(let std/vector/cycle (lambda n xs (do 
  (let out [])
  (let len (length xs))
  (loop 0 n (lambda i (std/vector/push! out (get xs (mod i len)))))
  out)))
(let std/vector/replicate (lambda n x (do 
  (let out [])
  (loop 0 n (lambda . (std/vector/push! out x)))
  out)))
(let std/vector/int/extreme (lambda xs { (std/vector/int/minimum xs) (std/vector/int/maximum xs) }))
(let std/tuple/map (lambda { a b } fn (fn a b)))
(let std/tuple/map/fst (lambda { a . } fn (fn a)))
(let std/tuple/map/snd (lambda { . b } fn (fn b)))
(let std/tuple/swap (lambda { a b } { b a }))

(let get* (lambda xs i some none (if (std/vector/in-bounds? xs i) (do (some (get xs i)) nil) (do (none) nil))))
(let get* (lambda xs i some none (if (std/vector/in-bounds? xs i) (do (some (get xs i)) nil) (do (none) nil))))
(let std/vector/2d/get* get*)
(let std/vector/3d/get* (lambda xs i j some none (if (std/vector/3d/in-bounds? xs i j) (do (some (get xs i j)) nil) (do (none) nil))))
(let std/vector/enumerate (lambda xs (std/vector/tuple/zip { (std/vector/int/range 0 (- (length xs) 1)) xs })))

(let std/int/factorial (lambda n (do 
  (let* fact (lambda n total
    (if (= n 0)
        total
        (fact (- n 1) (* total n)))))
  (fact n 1))))

(let std/float/factorial (lambda n (do 
  (let* fact (lambda n total
    (if (=. n 0.)
        total
        (fact (-. n 1.) (*. total n)))))
  (fact n 1.))))

(let std/vector/permutations (lambda arr (do 
  (let* permute (lambda arr (if (<= (length arr) 1)
        [arr]
        (do
          (let out [])
          (variable i 0)
          (while (< (get i) (length arr)) (do
              (let rest (std/vector/filter/i arr (lambda y j (not (= j (get i))))))
              (let perms (permute rest))
              (let x (get arr (get i)))
              (variable j 0)
              (while (< (get j) (length perms)) (do
                  (set! out (length out) (std/vector/cons! [x] (get perms (get j))))
                  (++ j)))
              (++ i)))
          out))))
    (permute arr))))

(let std/vector/combinations (lambda xs (do
    (let out [])
    (let* combinations (lambda arr size start temp
        (if (= (length temp) size)
            (set! out (length out) (std/vector/copy temp))
            (loop start (length arr) (lambda i (do
                    (set! temp (length temp) (get arr i))
                    (combinations arr size (+ i 1) temp)
                    (pop! temp)))))))
   (loop 1 (+ 1 (length xs)) (lambda i (combinations xs i 0 [])))
    out)))

(let std/vector/combinations/n (lambda xs n (do
    (let out [])
    (let* combinations (lambda arr size start temp
        (if (= (length temp) size)
            (set! out (length out) (std/vector/copy temp))
            (loop start (length arr) (lambda i (do
                    (set! temp (length temp) (get arr i))
                    (combinations arr size (+ i 1) temp)
                    (pop! temp)))))))
    (combinations xs n 0 [])
    out)))

(let std/int/div/option (lambda a b (if (= b 0) { false 0 } { true (/ a b) })))
(let std/int/expt/option (lambda a b (if (< a 0) { false 0 } { true (std/int/expt a b) })))
(let std/int/mod/option (lambda a b (if (= b 0) { false 0 } { true (mod a b) })))
(let std/int/sqrt/option (lambda n (if (< n 0) { false 0 } { true (std/int/sqrt n)})) )

(let std/float/div/option (lambda a b (if (=. b 0.) { false 0. } { true (/. a b) })))
(let std/float/expt/option (lambda a b (if (<. a 0.) { false 0. } { true (std/float/expt a b) })))
(let std/float/mod/option (lambda a b (if (=. b 0.) { false 0. } { true (mod. a b) })))
(let std/float/sqrt/option (lambda n (if (<. n 0.) { false 0. } { true (std/float/sqrt n)})) )

(let std/true/option (lambda x { true x }))
(let std/false/option (lambda x { false x }))

(let std/vector/option/resolve (lambda xs fn df 
  (if (std/vector/every? xs fst) { true (fn (std/vector/map xs snd)) } { false df })))
(let std/fn/exec (lambda xs fn (fn xs)))
(let std/convert/vector->tuple (lambda xs fn1 fn2 { (fn1 xs) (fn2 xs) }))
(let std/tuple/int/add (lambda { a b } (+ a b)))
(let std/tuple/int/sub (lambda { a b } (- a b)))
(let std/tuple/int/mul (lambda { a b } (* a b)))
(let std/tuple/int/div (lambda { a b } (* a b)))

(let loop/repeat (lambda n fn (loop 0 n (lambda . (fn)))))
(let loop/some-range? (lambda start end predicate? (do 
  (let* tail-call/loop/some-range? (lambda i out
                          (if (< i end)
                                (if (predicate? i) 
                                    true
                                    (tail-call/loop/some-range? (+ i 1) out)) 
                            out)))
                          (tail-call/loop/some-range? start false))))

(let loop/some-n? (lambda n predicate? (loop/some-range? 0 n predicate?)))

(let push! (lambda xs x (set! xs (length xs) x)))
(let pull! std/vector/pop-and-get!)
(let swap! std/vector/swap!)
(let scan! (lambda xs fn (std/vector/adjacent-difference! xs fn)))
(let empty! (lambda xs (do (std/vector/empty! xs) nil)))
(let reverse! std/vector/reverse!)
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
(let std/vector/hash/set (lambda capacity (std/vector/buckets (std/int/max 4 capacity))))
(let std/vector/hash/set/new std/vector/hash/set)
(let std/vector/hash/set/max-capacity (lambda a b (std/vector/hash/set (std/int/max (length a) (length b)))))
(let std/vector/hash/set/min-capacity (lambda a b (std/vector/hash/set (std/int/min (length a) (length b)))))

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
  (let target (std/int/max 4 new-capacity))
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
  (let target (std/int/max 32 (* used 2)))
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
                  (do (std/vector/hash/set/resize! table (std/int/max 32 (/ (length table) 2))) nil)
                  nil))
            nil)
        table)))))

(let std/convert/vector->set/dynamic (lambda xs (do
  (let out (std/vector/hash/set (std/int/max 32 (length xs))))
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
    (if (and (std/vector/not-empty? key) (std/vector/hash/set/has? trg key))
        (do (std/vector/hash/set/add! out key) nil)
        nil)))
  out)))

(let std/vector/hash/set/difference (lambda a b (do
  (let out (std/vector/hash/set/max-capacity a b))
  (std/vector/hash/set/for-each a (lambda key
    (if (and (std/vector/not-empty? key) (not (std/vector/hash/set/has? b key)))
        (do (std/vector/hash/set/add! out key) nil)
        nil)))
  out)))

(let std/vector/hash/set/xor (lambda a b (do
  (let out (std/vector/hash/set/max-capacity a b))
  (std/vector/hash/set/for-each a (lambda key
    (if (and (std/vector/not-empty? key) (not (std/vector/hash/set/has? b key)))
        (do (std/vector/hash/set/add! out key) nil)
        nil)))
  (std/vector/hash/set/for-each b (lambda key
    (if (and (std/vector/not-empty? key) (not (std/vector/hash/set/has? a key)))
        (do (std/vector/hash/set/add! out key) nil)
        nil)))
  out)))

(let std/vector/hash/set/union (lambda a b (do
  (let out (std/vector/hash/set/max-capacity a b))
  (std/vector/hash/set/for-each a (lambda key
    (if (std/vector/not-empty? key) (do (std/vector/hash/set/add! out key) nil) nil)))
  (std/vector/hash/set/for-each b (lambda key
    (if (std/vector/not-empty? key) (do (std/vector/hash/set/add! out key) nil) nil)))
  out)))

; -------------------------
; Fast Table implementation
; -------------------------
(let std/vector/hash/table (lambda capacity (std/vector/buckets (std/int/max 4 capacity))))
(let std/vector/hash/table/new std/vector/hash/table)
(let std/vector/hash/table/max-capacity (lambda a b (std/vector/hash/table (std/int/max (length a) (length b)))))

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
  (let target (std/int/max 4 new-capacity))
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
  (let target (std/int/max 32 (* used 2)))
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
                  (do (std/vector/hash/table/resize! table (std/int/max 32 (/ (length table) 2))) nil)
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
  (let table (std/vector/hash/table (std/int/max 64 (length arr))))
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

(let std/vector/hash/table/drop! (lambda table keys (do
  (mut i 0)
  (let len (length keys))
  (while (< i len) (do
    (std/vector/hash/table/remove! table (get keys i))
    (alter! i (+ i 1)))))))

(let std/vector/hash/table/keep (lambda table keys (do
  (let out (std/vector/hash/table (std/int/max 32 (length keys))))
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
  (let out (std/vector/hash/table (std/int/max 32 (length entries))))
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (let entry (get entries i))
    (std/vector/hash/table/set! out (fst entry) (snd entry))
    (alter! i (+ i 1))))
  out)))

(let std/vector/hash/table/group-by (lambda xs fn (do
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

(let std/int/min/3 (lambda a b c (std/int/min (std/int/min a b) c)))
(let std/int/min/4 (lambda a b c d (std/int/min (std/int/min a b) (std/int/min c d))))
(let std/int/min/2 std/int/min)

(let std/vector/char/damerau-levenshtein (lambda a b (do
  (let n (length a))
  (let m (length b))
  (let matrix (Matrix/new (lambda . . 0) (+ n 1) (+ m 1) ))

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
      (let best (std/int/min/3 delete-cost insert-cost subst-cost))

      (let with-transpose
        (if (and (> i 1)
                 (> j 1)
                 (=# a-char (get b (- j 2)))
                 (=# (get a (- i 2)) b-char))
            (std/int/min best (+ (get (get matrix (- i 2)) (- j 2)) 1))
            best))
      (set! current-row j with-transpose)
      (alter! j (+ j 1))))
    (alter! i (+ i 1))))

  (get (get matrix n) m))))

(let std/vector/char/autocorrect (lambda word dictionary (do
  (let f (get dictionary 0))
  (variable best-word f)
  (mut best-dist (std/vector/char/damerau-levenshtein word f))
  (mut i 1)
  (while (< i (length dictionary)) (do
    (let candidate (get dictionary i))
    (let dist (std/vector/char/damerau-levenshtein word candidate))
    (if (< dist best-dist)
        (do
          (set best-word candidate)
          (alter! best-dist dist)))
    (alter! i (+ i 1))))
  { (get best-word) best-dist })))
