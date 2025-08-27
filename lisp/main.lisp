; (let f (lambda x [a . c] (* x (+ a c c))))

; ; (f 10 [ 1 2 3 ])

(let xs! [ 1 2 3 ])
(set! xs! 0 10)
(set! xs! 1 10)
(set! xs! 2 30)
(set! xs! 3 30)
(set! xs! 4 30)

xs!

; (let arr! [ 1 2 3 ])
; (pop! arr!)
; (let x (pop-and-get! arr!))
; x


; (map [ 1 2 3 4 ] (lambda x (* x 10)))