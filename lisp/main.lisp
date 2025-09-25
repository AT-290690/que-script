; int sum(List* list, int acc) {
;   if (list == nullptr) return acc;
;   return sum(list->next, acc + list->val);
; }

(let sum (lambda xs acc 
    (if (= (length xs) 0) acc 
        (sum (std:vector:drop xs 1) (+ acc (get xs 0))))))

(let sum (lambda xs acc (do
    (let _acc [ acc ])
    (let _xs [ xs ])
    (let _new_xs [])
    (let _new_acc [])
    (loop (not (= (length (get _xs 0)) 0)) (lambda (do
        (set! _new_xs 0 (std:vector:drop (get _xs 0) 1))
        (set! _new_acc 0 (+ (get _acc 0) (get (get _xs 0) 0)))
        (set! _xs 0  (get _new_xs 0))
        (set! _acc 0 (get _new_acc 0)))))
    (get _acc 0))))

(sum [ 1 2 3 4 5 ] 0)