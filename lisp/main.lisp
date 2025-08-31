; ; this is infered ok 
; (let fn (lambda xs (do 
;  (get xs 0)
; )))

; (fn [ false ]) ; Bool

; ; this is infered ok
; (let xs [ false ])
; (let x (get xs 0))
; x  ; Bool

; this is not
(let fn (lambda xs (do 
 (let x (get xs 0))
 x
)))

(fn [ [false] ]) ; t1 (generic type)
