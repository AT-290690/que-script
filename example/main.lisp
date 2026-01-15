(+ 1 2)
(let fn (lambda { x y } (do 
(let vec [ 1 2 3 ])

(let [ a b . ] vec)
    [ a b  x y ]

)))

(fn { 10 23 })