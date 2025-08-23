(let true (not 0))
(let false (not 1))
(let array:get get)
(let array:set! set!)
(let array:select (lambda xs cb? (do
                  (let tail-call:array:select (lambda i out
                        (if (> (length xs) i)
                            (tail-call:array:select (+ i 1)
                                      (if (cb? (array:get xs i))
                                            (array:set! out (length out) (array:get xs i))
                                            out))
                            out)))
                      (tail-call:array:select 0 []))))
(let array:map (lambda xs cb (do
                  (let tail-call:array:map (lambda i out
                        (if (> (length xs) i)
                              (tail-call:array:map (+ i 1)
                                (array:set! out (length out) (cb (array:get xs i))))
                              out)))
                      (tail-call:array:map 0 []))))
(let array:fold (lambda xs cb initial (do
                  (let tail-call:array:fold (lambda i out
                        (if (> (length xs) i)
                            (tail-call:array:fold (+ i 1) (cb out (array:get xs i)))
                            out)))
                      (tail-call:array:fold 0 initial))))

(let math:square (lambda x (* x x)))
(let math:even? (lambda x (= (mod x 2) 0)))
(let math:odd? (lambda x (not (= (mod x 2) 0))))
(let math:summation (lambda xs (array:fold xs (lambda a b (+ a b)) 0)))
(let math:product (lambda xs (array:fold xs (lambda a b (* a b)) 1)))

(|> [ 1 2 3 4 ] (array:select math:even?) (array:map math:square) (math:summation))
