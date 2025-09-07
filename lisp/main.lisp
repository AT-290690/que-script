; (let max-water (lambda xs (do
;     (integer m 0)
;     (integer i 0)
;     (integer j (- (length xs) 1))
;     (loop-finish (<> (. i) (. j)) (lambda (do
;         (if (> (. xs (. i)) (. xs (. j))) 
;             (do
;                 (=! m (max (* (- (. j) (. i)) (. xs (. j))) (. m)))
;                 (-- j)) 
;             (do
;                 (=! m (max (* (- (. j) (. i) (. xs (. i)))) (. m)))
;                 (++ i))))))
;     (. m))))

; (max-water [ 1 8 6 2 5 4 8 3 7 ])


(integer i 0)
(let function (lambda cb var (lambda . (cb var))))
(loop 0 10 (function ++ i))
i