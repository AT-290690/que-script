(letmacro unless
  ((con)
    (qq (if (uq con) nil nil)))
  ((con body)
    (qq (if (uq con) nil (uq body))))
  ((con then else)
    (qq (if (uq con) (uq else) (uq then)))))

(letmacro !=
  (lambda a b
    (qq (not (= (uq a) (uq b))))))

(letmacro <>
  (lambda a b
    (qq (not (= (uq a) (uq b))))))

(letmacro cond
  (()
    (qq 0))
  ((default)
    (qq (uq default)))
  ((test branch)
    (qq (if (uq test) (uq branch) nil)))
  ((test branch . rest)
    (qq (if (uq test) (uq branch) (cond (uqs rest))))))

(letmacro when
  (lambda con . body
    (qq (if (uq con) (do (uqs body)) nil))))

(letmacro when-not
  (lambda con . body
    (qq (unless (uq con) (do (uqs body))))))

(letmacro test
  (lambda name expr
    (qq { (uq name) (uq expr) })))

(letmacro +=
  (lambda x n
    (qq (alter! (uq x) (+ (uq x) (uq n))))))

(letmacro -=
  (lambda x n
    (qq (alter! (uq x) (- (uq x) (uq n))))))

(letmacro *=
  (lambda x n
    (qq (alter! (uq x) (* (uq x) (uq n))))))

(letmacro /=
  (lambda x n
    (qq (alter! (uq x) (/ (uq x) (uq n))))))

(letmacro ++
  (lambda x
    (qq (alter! (uq x) (+ (uq x) 1)))))

(letmacro --
  (lambda x
    (qq (alter! (uq x) (- (uq x) 1)))))


(letmacro loop/for
  (lambda name init con step . body
    (qq (do
          (mut (uq name) (uq init))
          (while (uq con)
            (do
              (uqs body)
              (uq step)
              nil))))))

(letmacro loop/range/exclusive
  (lambda name start end . body
    (qq (do
          (mut (uq name) (uq start))
          (while (< (uq name) (uq end))
            (do
              (uqs body)
              (++ (uq name))
              nil))))))

(letmacro loop/range/inclusive
  (lambda name start end . body
    (qq (do
          (mut (uq name) (uq start))
          (while (<= (uq name) (uq end))
            (do
              (uqs body)
              (++ (uq name))
              nil))))))

(letmacro loop/range
  (lambda name start end . body
    (qq (loop/range/inclusive (uq name) (uq start) (uq end)
          (uqs body)))))

(letmacro loop/range/inclusive/by
  (lambda name start end step . body
    (qq (do
          (mut (uq name) (uq start))
          (let step# (uq step))
          (if (> step# 0)
              (while (<= (uq name) (uq end))
                (do
                  (uqs body)
                  (+= (uq name) step#)
                  nil))
              (while (>= (uq name) (uq end))
                (do
                  (uqs body)
                  (+= (uq name) step#)
                  nil)))))))

(letmacro loop/range/exclusive/by
  (lambda name start end step . body
    (qq (do
          (mut (uq name) (uq start))
          (let step# (uq step))
          (if (> step# 0)
              (while (< (uq name) (uq end))
                (do
                  (uqs body)
                  (+= (uq name) step#)
                  nil))
              (while (> (uq name) (uq end))
                (do
                  (uqs body)
                  (+= (uq name) step#)
                  nil)))))))

(letmacro loop/range/by
  (lambda name start end step . body
    (qq (loop/range/inclusive/by (uq name) (uq start) (uq end) (uq step)
          (uqs body)))))

(letmacro loop/in/vector
  (lambda item items . body
    (do
      (let i (gensym))
      (let xs (gensym))
      (let len (gensym))
      (qq (do
            (let (uq xs) (uq items))
            (let (uq len) (length (uq xs)))
            (mut (uq i) 0)
            (while (< (uq i) (uq len))
              (do
                (let (uq item) (get (uq xs) (uq i)))
                (uqs body)
                (++ (uq i))
                nil)))))))

(letmacro loop/in
  (lambda item items . body
    (qq (loop/in/vector (uq item) (uq items)
          (uqs body)))))

(letmacro loop/in/matrix
  (lambda item items . body
    (do
      (let y (gensym))
      (let x (gensym))
      (let rows (gensym))
      (let row (gensym))
      (let height (gensym))
      (let width (gensym))
      (qq (do
            (let (uq rows) (uq items))
            (let (uq height) (length (uq rows)))
            (mut (uq y) 0)
            (while (< (uq y) (uq height))
              (do
                (let (uq row) (get (uq rows) (uq y)))
                (let (uq width) (length (uq row)))
                (mut (uq x) 0)
                (while (< (uq x) (uq width))
                  (do
                    (let (uq item) (get (uq row) (uq x)))
                    (uqs body)
                    (++ (uq x))
                    nil))
                (++ (uq y))
                nil)))))))

(letmacro times
  (lambda n . body
    (do
      (let i (gensym))
      (qq (loop/range/exclusive (uq i) 0 (uq n)
            (uqs body))))))

(letmacro repeat
  (lambda n body
    (do
      (let i (gensym))
      (qq (loop/range/exclusive (uq i) 0 (uq n)
            (uq body))))))


(letmacro loop
  (lambda name condition . body
    (qq (do
          (mut (uq name) 0)
          (while (uq condition)
            (do
              (uqs body)
              (++ (uq name))
              nil))))))

(letmacro let*
  ((name value body)
    (qq (block
          (let (uq name) (uq value))
          (uq body))))
  ((name value . rest)
    (qq (block
          (let (uq name) (uq value))
          (let* (uqs rest))))))

(letmacro block
    (lambda . body
      (qq ((lambda
              (do (uqs body)))))))

(letmacro vector/default/items/static
  (lambda n x
    (if (= n 0)
        (quote ())
        (qq ((uq x) (uqs (vector/default/items/static (- n 1) x)))))))

(letmacro vector/default/static
  (lambda n x
    (qq [(uqs (vector/default/items/static n x))])))

(letmacro zeros/static
  (lambda n
    (qq (vector/default/static (uq n) 0))))

(letmacro ones/static
  (lambda n
    (qq (vector/default/static (uq n) 1))))

(letmacro truths/static
  (lambda n
    (qq (vector/default/static (uq n) true))))

(letmacro falses/static
  (lambda n
    (qq (vector/default/static (uq n) false))))

(letmacro chars/static
  (lambda n ch
    (qq (vector/default/static (uq n) (uq ch)))))

(letmacro range/items/static
  (lambda start end
    (if (> start end)
        (quote ())
        (qq ((uq start) (uqs (range/items/static (+ start 1) end)))))))

(letmacro range/static
  (lambda start end
    (qq [(uqs (range/items/static start end))])))

(letmacro repeat/items/static
  (lambda n body
    (if (= n 0)
        (quote ())
        (qq ((do (uqs body)) (uqs (repeat/items/static (- n 1) body)))))))

(letmacro repeat/static
  (lambda n . body
    (qq (do
          (uqs (repeat/items/static n body))
          nil))))

(letmacro unroll/items/static
  (lambda n name i body
    (if (= i n)
        (quote ())
        (qq (((lambda (uq name)
                (do (uqs body)))
              (uq i))
             (uqs (unroll/items/static n name (+ i 1) body)))))))

(letmacro unroll/static
  (lambda n name . body
    (qq (do
          (uqs (unroll/items/static n name 0 body))
          nil))))

(letmacro assert/static
  (lambda condition
    (if condition
        (qq nil)
        (error "assert/static failed"))))

(letmacro test-suite/static
  (lambda name . tests
    (qq [(uqs tests)])))

(letmacro json/parse
  ((k1 r1)
    (do
      (let source (gensym))
      (qq (lambda (uq source)
            ((uq r1) (std/json/field (uq source) (uq k1)))))))
  ((k1 r1 k2 r2)
    (do
      (let source (gensym))
      (qq (lambda (uq source)
            (tuple
              ((uq r1) (std/json/field (uq source) (uq k1)))
              ((uq r2) (std/json/field (uq source) (uq k2))))))))
  ((k1 r1 k2 r2 k3 r3)
    (do
      (let source (gensym))
      (qq (lambda (uq source)
            (tuple
              ((uq r1) (std/json/field (uq source) (uq k1)))
              ((uq r2) (std/json/field (uq source) (uq k2)))
              ((uq r3) (std/json/field (uq source) (uq k3))))))))
  ((k1 r1 k2 r2 k3 r3 k4 r4)
    (do
      (let source (gensym))
      (qq (lambda (uq source)
            (tuple
              ((uq r1) (std/json/field (uq source) (uq k1)))
              ((uq r2) (std/json/field (uq source) (uq k2)))
              ((uq r3) (std/json/field (uq source) (uq k3)))
              ((uq r4) (std/json/field (uq source) (uq k4))))))))
  ((k1 r1 k2 r2 k3 r3 k4 r4 k5 r5)
    (do
      (let source (gensym))
      (qq (lambda (uq source)
            (tuple
              ((uq r1) (std/json/field (uq source) (uq k1)))
              ((uq r2) (std/json/field (uq source) (uq k2)))
              ((uq r3) (std/json/field (uq source) (uq k3)))
              ((uq r4) (std/json/field (uq source) (uq k4)))
              ((uq r5) (std/json/field (uq source) (uq k5))))))))
  ((k1 r1 k2 r2 k3 r3 k4 r4 k5 r5 k6 r6)
    (do
      (let source (gensym))
      (qq (lambda (uq source)
            (tuple
              ((uq r1) (std/json/field (uq source) (uq k1)))
              ((uq r2) (std/json/field (uq source) (uq k2)))
              ((uq r3) (std/json/field (uq source) (uq k3)))
              ((uq r4) (std/json/field (uq source) (uq k4)))
              ((uq r5) (std/json/field (uq source) (uq k5)))
              ((uq r6) (std/json/field (uq source) (uq k6)))))))))
