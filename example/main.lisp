; (+ 1 2)
; (let fn (::
;   (lambda x y (do (: x Int) (: y Int) Int))
;   (lambda a b (> a b))
;   )
; )


; (fn 1 2)


; (let fn (:: Int -> [Bool] -> (Int -> Bool) -> {Int Bool}
;         (lambda a b fn (do 
;             (push! b (> a 1))
;             { a (get b) }
;         ))))
; (fn 1 [])


; (let fn (:: (lambda x0 x1 (do (: x0 Int) (: x1 [Int]) {Int Bool}))
;      (lambda a b { Int Bool })))

; (fn 1 false)

; Apply([Word("::"), Apply(
;         [Word("lambda"), Word("x0"), Word("x1"), Apply([Word("do"), 
;             Apply([Word(":"), Word("x0"), Word("Int")]), 
;             Apply([Word(":"), Word("x1"), Apply([Word("vector"), Word("Int")])]), 
;             Apply([Word("tuple"), Word("Int"), Word("Bool")])])]), 
            
;             Apply([Word("lambda"), Word("a"), Word("b"), Apply([Word("tuple"), Word("a"), Word("b")])])])






; (let -> (lambda x x))

; (lambda a b fn (do (: a Int) (: b [Bool]) (: fn (lambda x (do (: x Int) (-> false)))) (-> [{ Int Bool }])))

; : is type assertions
; -> is lambda type definition
; last thing in lambda definition (-> ) is the return type of that lambda 
; generics are (: name name) 
; names don't need to match with lambda args
; but the number of args must match

; Here are some examples 

; (let f (lambda a b fn (do (push! b [false]) (or (fn a) (get b 0 0)))))
; (: f (-> (: a Int) (: b [[Bool]]) (: fn (-> (: x Int) Bool)) Bool))

; (let xs [])
; (push! xs 1)
; (: xs [Int])

; ; (let fn (lambda a b c  { (+ a b) c }))
; ; (: fn (-> (: a Int) (: b Int) (: c c) { Int c }))

; (let rev (lambda xs (do 
;         (let~ rev* (lambda xs ys (if (empty? xs) ys (rev* (cdr xs) (cons [(car xs)] ys )))))
;         (rev* xs []))))
; (: rev (-> (: [] a) a))
; (let fn (: (-> (: Bool a) (: Bool b) b) (lambda a b (do (+ a 1) (or b true)))))
; fn

; (let xs (: [] [ 1 2 3 ]))
; xs