(let *Id* [0])
(let Id! (lambda (do 
  (let id (get *Id*))
  (++ *Id*)
  id)))
(let const/int/max-safe 2147483647)
(let const/int/min-safe -2147483648)
(let const/float/max-safe 16777216.0)
(let const/float/min-safe -16777216.0)
(let const/float/pi 3.1415927)
(let infinity 2147483647)
(let -infinity -2147483648)
(let Int 0)
(let Float 0.0)
(let Char (get "a"))
(let Bool false)
(let Nil nil)
(let as (lambda . t t))
(let : (lambda t x (do [ t x ] x)))
(let eq (lambda a b (cond 
          (and a b) true 
          (and (not a) (not b)) true
          false)))
(let identity (lambda x x))
(let Char/nil (Int->Char 0))
(let Char/start (Int->Char 2))
(let Char/end (Int->Char 3))


(let const/float/two-pi (+. const/float/pi const/float/pi))

(let const/float/wrap-pi (lambda x (do
  (mut y x)
  (while (>. y const/float/pi) (do
    (alter! y (-. y const/float/two-pi))))
  (while (<. y (-. const/float/pi)) (do
    (alter! y (+. y const/float/two-pi))))
  y)))

(let const/float/sin/terms (lambda x terms (do
  (let y (const/float/wrap-pi x))
  (let x2 (*. y y))
  (mut term y)
  (mut sm y)
  (mut n 0)
  (while (< n terms) (do
    (let a (+ (* 2 n) 2))
    (let b (+ (* 2 n) 3))
    (let denom (Int->Float (* a b)))
    (alter! term (/. (-. (*. term x2)) denom))
    (alter! sm (+. sm term))
    (alter! n (+ n 1))))
  sm)))

(let const/float/cos/terms (lambda x terms (do
  (let y (const/float/wrap-pi x))
  (let x2 (*. y y))
  (mut term 1.0)
  (mut sm 1.0)
  (mut n 0)
  (while (< n terms) (do
    (let a (+ (* 2 n) 1))
    (let b (+ (* 2 n) 2))
    (let denom (Int->Float (* a b)))
    (alter! term (/. (-. (*. term x2)) denom))
    (alter! sm (+. sm term))
    (alter! n (+ n 1))))
  sm)))

; Public helpers (good default precision)
(let sin (lambda x (const/float/sin/terms x 8)))
(let cos (lambda x (const/float/cos/terms x 8)))