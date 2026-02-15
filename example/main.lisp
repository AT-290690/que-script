(let sum-vector (lambda xs (do 
    (integer out 0)
    (loop 1 (length xs) (lambda i (+= out (get xs 0))))
    (get out)
)))
(sum-vector [ 1 2 3 4 5 6 ])
; sum-vector