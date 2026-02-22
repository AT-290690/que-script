(let build-fns (lambda n (do
  (let fs [])
  (loop 0 n (lambda i (set! fs (length fs) (lambda x (+ x i)))))
  fs)))

(let sum-call (lambda fs n (do
  (integer acc 0)
  (loop 0 n (lambda i (do
    (let f (get fs i))
    (+= acc (f 1)))))
  (get acc))))

(let once (sum-call (build-fns 5) 5))

(loop 0 5000 (lambda t (do
  (let fs (build-fns 16))
  (sum-call fs 16))))

(= once 15)
