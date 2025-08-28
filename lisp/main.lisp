(let for (lambda xs cb (do
      (let out [])
      (loop 0 (length xs) (lambda i (push! out (cb (get xs i)))))
      out)))

(for [ 1 2 3 4 ] (lambda x (* x x)))
