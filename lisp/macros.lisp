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
