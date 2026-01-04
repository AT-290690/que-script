(let *Id* [0])
(let Id (lambda (do 
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
(let nil (loop 0 0 (lambda . 0)))
(let as (lambda . t t))
(let : (lambda t x (do [ t x ] x)))
(let eq (lambda a b (cond 
          (and a b) true 
          (and (not a) (not b)) true
          false)))
(let identity (lambda x x))
