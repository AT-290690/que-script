(let xs [ 1 2 3 ])
(let copy (std:vector:copy xs))
(set! copy 0 1000)
[ xs copy ]