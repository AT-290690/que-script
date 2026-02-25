  (let h (lambda payload (do
    (let [a b c .] payload)
    (let* step (lambda i [i i i]))
    (let final-state (step 1))
    (do
      (let [x y z .] final-state)
      (+ a b x)
    ))))
  (h [1 2 3 4])