(+ 1 2)

;   (let sum/n std/int/gauss-sum)

; (type String [Char])
; ; (:: sum/n (Lambda (: Int x) Bool))
;   (:: solve (Lambda (: Int x1) (: String x2) (: Char x3) Bool))
;   (let solve (lambda n s c false))
; (let la (lambda (do
; (do 
;   (:: solve (Lambda (: Int x1) (: String x2) (: Char x3) Bool))
;   (let solve (lambda n s c (- (sum/n n) (|> s (String->Vector c) (map (comp length sum/n)) sum))))
;   [
;     (solve 3 "abb" 'b') ; 5
;     (solve 6 "abcabc" 'c') ; 15
;   ]
; )
; )))