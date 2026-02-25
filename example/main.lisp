(let fn (lambda { x y } (do 
  (let vec [ 1 2 3 ])
  (let [ a b . ] vec)
      [ a b  x y ]
  )))
(let { x { y z }} { 10 { [ 1 2 3 ] false} })
(let def { true { [ 1 2 3 4 5 ] (lambda [ a b ] (+ (sum b) a)) } })
(let { a1 { b1 c1 } } def)
; c1 is a function (lambda [ a b ] (+ (sum b) a)) that destrucutres and sums these 1 + (2 + 3 + 4 + 5) = 15
; but we get 1 in wasm
; this is a bug in wasm 
; the result should be { true 15 }
{ a1 (c1 b1) }  
{ z  { (fn { 10 23 }) { a1 (c1 b1)} } }
