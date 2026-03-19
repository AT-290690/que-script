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

(letmacro +=
  (lambda x n
    (qq (alter! (uq x) (+ (uq x) (uq n))))))

(letmacro -=
  (lambda x n
    (qq (alter! (uq x) (- (uq x) (uq n))))))

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

(letmacro loop
  (lambda start end fn
    (do
      (let i (gensym))
      (let loop-end (gensym))
      (let loop-cb (gensym))
      (qq (do
            (mut (uq i) (uq start))
            (let (uq loop-end) (uq end))
            (let (uq loop-cb) (uq fn))
            (while (< (uq i) (uq loop-end))
              (do
                ((uq loop-cb) (uq i))
                (alter! (uq i) (+ (uq i) 1))
                nil)))))))

(letmacro block
    (lambda . body
      (qq ((lambda
              (do (uqs body)))))))