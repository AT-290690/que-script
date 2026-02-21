; (+ 1 2)

; (sum/float [ 1. 2. 3.5 6.2 ])


; (let xs [ 1 2 3 4 5 ])
; ; (let sm (lambda init xs (do 
; ;     (integer acc init)
; ;     (loop 0 (length xs) (lambda i (+= acc (get xs 0))))
; ;     (get acc))))
; (let rdc (lambda fn initial xs (do
;      (let out [ initial ])
;      (loop 0 (length xs) (lambda i (set! out 0 (fn (get out 0) (get xs i)))))
;      (get out))))
; ; ; (sm 0 xs)
; (let mp (lambda fn xs (do
;      (let out [])
;      (loop 0 (length xs) (lambda i (set! out (length out) (fn (get xs i)))))
;      out)))

; ; (rdc + 0 (mp (lambda x (* x x)) xs))\
; (let rng (lambda start end (do
;      (let out [])
;      (let process (lambda i (set! out (length out) i)))
;      (loop start (+ end 1) process)
;      out)))

; (let sqr (lambda x (* x x)))


; (rdc + 0 (mp length [[1] [2 3]]))


; (let f (mp (lambda x (and x true)) [ true false ]))
; (let ct (lambda f (do 
;     (integer c 0)
; (loop 0 (length f) (lambda i (+= c (if (=? (get f i) true) 1 0))))
; (get c)
; )))

; (let ls { 1 [ "Hello" "Wolrd" ]})
; (cons (get (snd ls) 0) " " (get (snd ls) 1))
; (ct f)

; ; TCO recursion
; (let~ k-mod (lambda n k (if (< k n) k (k-mod n (- k n)))))
; ; taking advantage of partial apply
; (let mod2 (k-mod 2))
; ; TCO recursion
; (let~ collatz (lambda n steps
;                (if (= n 1)
;                     steps
;                     (collatz (if (= (mod2 n) 0)
;                                  (/ n 2)
;                                  (+ (* n 3) 1))
;                                  (+ steps 1)))))

; (collatz 27 0)

; (let* rev (lambda xs ys (if (empty? xs) ys (rev (cdr xs) (cons [(car xs)] ys)))))
; (get (rev [ 1 2 3 ] []) 2)
; (rev [ 1 2 3 ] [])

; (get 
; (mp Integer->String (range 1 9))
; 0
; 0)

; (mp Integer->String (range 1 9))


; (let fn (lambda a b (do (let y [a b]) (set! y (length y) 10) y)))
; (let outer (lambda z (do (let g (fn 1 2)) (length g))))
; (loop 0 5000000 (lambda i (outer i)))
; (integer inc 0)
; (loop 0 (* 1000000000 2) (lambda i (set inc i)))
; (get inc)

; (let fn (lambda a b (do (let y [a b]) (set! y (length y) 10) y)))
; (let outer (lambda z (do (let g (fn 1 2)) (length g))))
; (loop 0 500000 (lambda i (outer i)))
; 1

; (let mk (lambda n (do
;   (let v [])
;   (loop 0 n (lambda i (set! v (length v) [i (+ i 1)])))
;   v)))
; (loop 0 200000 (lambda i (do (let t (mk 4)) (length t))))
; 1

; (let mk (lambda n (do
;   (let v [])
;   (loop 0 n (lambda i (set! v (length v) i)))
;   v)))
; (loop 0 300000 (lambda i (do
;   (let a (mk 3))
;   (let b (mk 40))
;   (+ (length a) (length b)))))
; 1


; (let fn (lambda x y (do 
;     { x y }
; )))

; (fn 1.23 4 )

; (let t { 1.2 { 10 [ 1 2 3 ] } })

; (get (snd (snd t)) 1)

; (let ARGS [])
; ARGS


(* 5 2)