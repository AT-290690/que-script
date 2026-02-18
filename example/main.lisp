(let fn (lambda (do 
        (+ 1 2)
    (|> (range 1 10) (map square) sum)
    (let str "1 2 3 4")
    (String->Vector ' ' str)
    (let* rev (lambda xs ys (if (empty? xs) ys (rev (cdr xs) (cons [(car xs)] ys)))))
    (rev [ 1 2 3 ] [])
    
    (let INPUT 
"r, wr, b, g, bwu, rb, gb, br

brwrr
bggr
gbbr
rrbgbr
ubwu
bwurrg
brgr
bbrgwb")

(let parse (lambda input (do
    (let lines (|> input (String->Vector nl)))
    {
      (|> lines (first) (String->Vector ',') (map (lambda xs (filter (lambda x (not (=# x ' '))) xs))))
      (|> lines (drop/first 2))
    })))
(parse INPUT)


(let part1 (lambda { patterns-input towels } (do 
  (let patterns (reduce (lambda a b (do (Set/add! b a) a)) [[] [] [] [] [] [] [] [] []] patterns-input))
  (let* dp? (lambda str (loop/some-range? 1 (length str) (lambda i (do 
    (let a (slice 0 i str))
    (let b (slice i (length str) str))
    (or (and (Set/has? a patterns) (Set/has? b patterns)) (and (dp? a) (dp? b))))))))

  (count dp? towels))))

; TCO recursion
(let~ k-mod (lambda n k (if (< k n) k (k-mod n (- k n)))))
; taking advantage of partial apply
(let mod2 (k-mod 2))
; TCO recursion
(let~ collatz (lambda n steps
               (if (= n 1)
                    steps
                    (collatz (if (= (mod2 n) 0)
                                 (/ n 2)
                                 (+ (* n 3) 1))
                                 (+ steps 1)))))

(collatz 27 0)
(/. (Int->Float 12332) 2.2)
(map Float->Int [1. 2. 3.])
(as 'a' Int)

(|> 
 (range 1 10)
(map (lambda x [ x x ]))
(map (reduce + 0))
)
[(part1 (parse INPUT))]

(range 1 10)

    )))
(fn)


