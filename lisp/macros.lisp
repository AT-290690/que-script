(letmacro unless
  ((con)
    (qq (if (uq con) nil nil)))
  ((con body)
    (qq (if (uq con) nil (uq body))))
  ((con then else)
    (qq (if (uq con) (uq else) (uq then)))))

(letmacro when
  (lambda con . body
    (qq (if (uq con) (do (uqs body)) nil))))

(letmacro when-not
  (lambda con . body
    (qq (unless (uq con) (do (uqs body))))))
