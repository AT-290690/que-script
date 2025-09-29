(let INPUT 
"89010123
78121874
87430965
96549874
45678903
32019012
01329801
10456732")
(let yx->key (lambda y x (std:vector:concat-with (std:vector:map [ y x ] (lambda c [ c ])) std:int:char:dash)))
(let parse (lambda input (|> input (std:convert:string->vector std:int:char:new-line) (std:vector:map std:convert:chars->digits))))
(let part1 (lambda matrix (do
  (let coords (std:vector:3d:points matrix std:int:zero?))
  (let default-queue-value [ 0 ])
  (std:vector:reduce coords (lambda a xs (do
        (integer score 0)
        (let y (std:vector:first xs))
        (let x (std:vector:second xs))
        (let visited (std:vector:buckets 8))
        (let queue (std:vector:queue:new default-queue-value))
        (let current (. matrix y x))
        (std:vector:hash:set:add! visited (yx->key y x))
        (std:vector:queue:enqueue! queue [ y x ])
        
        (loop (std:vector:queue:not-empty? queue) (lambda (do
            (let element (std:vector:queue:peek queue))
            (std:vector:queue:dequeue! queue  default-queue-value)
            (let y (std:vector:first element))
            (let x (std:vector:second element))  
            (std:vector:3d:adjacent matrix std:vector:3d:von-neumann-neighborhood y x (lambda cell dir dy dx (do
                 (let key (yx->key dy dx))
                 (if (and (= (- cell (. matrix y x)) 1) (not (std:vector:hash:set:has? visited key))) (do
                    (if (= cell 9) (do (++ score) nil) (do (std:vector:queue:enqueue! queue [ dy dx ]) nil))
                    (std:vector:hash:set:add! visited key)
                    nil))))))))

        (+ a (get score)))) 0))))

(let part2 (lambda matrix (do
  (let coords (std:vector:3d:points matrix std:int:zero?))
  (let default-queue-value [ 0 ])
  (std:vector:reduce coords (lambda a xs (do
        (integer score 0)
        (let y (std:vector:first xs))
        (let x (std:vector:second xs))
        (let visited (std:vector:buckets 8))
        (let queue (std:vector:queue:new default-queue-value))
        (let current (. matrix y x))
        (let root-key (yx->key y x))
        (std:vector:hash:table:set! visited root-key 1)
        (std:vector:queue:enqueue! queue [ y x ])
        (loop (std:vector:queue:not-empty? queue) (lambda (do
            (let element (std:vector:queue:peek queue))
            (let y (std:vector:first element))
            (let x (std:vector:second element))  
            (if (= (. matrix y x) 9) (+= score (std:vector:hash:table:get visited root-key)))
            (std:vector:queue:dequeue! queue default-queue-value)
            (std:vector:3d:adjacent matrix std:vector:3d:von-neumann-neighborhood y x (lambda cell dir dy dx (do
                 (let key (yx->key dy dx))
                 (if (= (- cell (. matrix y x)) 1) (do
                    (std:vector:queue:enqueue! queue [ dy dx ])
                    (if (std:vector:hash:table:has? visited key) 
                        (std:vector:hash:table:set! visited key (+ (std:vector:hash:table:get visited root-key) (std:vector:hash:table:get visited key))) 
                        (std:vector:hash:table:set! visited key (std:vector:hash:table:get visited root-key)))
                      nil))))))))
        (+ a (get score)))) 0))))

(let PARSED (parse INPUT))

[(part1 PARSED) (part2 PARSED)]