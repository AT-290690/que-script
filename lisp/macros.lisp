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


(let __type/vec1
  (lambda a x
    (do
      [a x]
      x)))

(let __type/fn1
  (lambda in out f
    (do
      (let probe
        (lambda x
          (__type/vec1 out (f (__type/vec1 in x)))))
      f)))

(let __type/fn2
  (lambda a b out f
    (do
      (let probe
        (lambda x y
          (__type/vec1 out
            (f (__type/vec1 a x)
               (__type/vec1 b y)))))
      f)))

(let __type/fn3
  (lambda a b c out f
    (do
      (let probe
        (lambda x y z
          (__type/vec1 out
            (f (__type/vec1 a x)
               (__type/vec1 b y)
               (__type/vec1 c z)))))
      f)))


(let __type/fn4
  (lambda a b c d out f
    (do
      (let probe
        (lambda x y z w
          (__type/vec1 out
            (f (__type/vec1 a x)
               (__type/vec1 b y)
               (__type/vec1 c z)
               (__type/vec1 d w)))))
      f)))

(let __type/fn5
  (lambda a b c d e out f
    (do
      (let probe
        (lambda x y z w v
          (__type/vec1 out
            (f (__type/vec1 a x)
               (__type/vec1 b y)
               (__type/vec1 c z)
               (__type/vec1 d w)
               (__type/vec1 e v)))))
      f)))

(let __type/fn6
  (lambda a b c d e g out f
    (do
      (let probe
        (lambda x y z w v u
          (__type/vec1 out
            (f (__type/vec1 a x)
               (__type/vec1 b y)
               (__type/vec1 c z)
               (__type/vec1 d w)
               (__type/vec1 e v)
               (__type/vec1 g u)))))
      f)))

(letmacro Fn1
  (lambda A R
    (qq
      (lambda f
        (__type/fn1 (uq A) (uq R) f)))))

(letmacro Fn2
  (lambda A B R
    (qq
      (lambda f
        (__type/fn2 (uq A) (uq B) (uq R) f)))))

(letmacro Fn3
  (lambda A B C R
    (qq
      (lambda f
        (__type/fn3 (uq A) (uq B) (uq C) (uq R) f)))))

(letmacro Fn4
  (lambda A B C D R
    (qq
      (lambda f
        (__type/fn4 (uq A) (uq B) (uq C) (uq D) (uq R) f)))))

(letmacro Fn5
  (lambda A B C D E R
    (qq
      (lambda f
        (__type/fn5 (uq A) (uq B) (uq C) (uq D) (uq E) (uq R) f)))))

(letmacro Fn6
  (lambda A B C D E G R
    (qq
      (lambda f
        (__type/fn6 (uq A) (uq B) (uq C) (uq D) (uq E) (uq G) (uq R) f)))))

(letmacro letype
  (lambda Name Ty
    (qq
      (let (uq Name)
        (lambda x
          (__type/vec1 (uq Ty) x))))))

(letmacro sig
  (lambda Ty value
    (qq
      ((uq Ty) (uq value)))))

(letmacro letype/fn1
  (lambda Name A R
    (qq
      (let (uq Name)
        (Fn1 (uq A) (uq R))))))

(letmacro letype/fn2
  (lambda Name A B R
    (qq
      (let (uq Name)
        (Fn2 (uq A) (uq B) (uq R))))))

(letmacro letype/fn3
  (lambda Name A B C R
    (qq
      (let (uq Name)
        (Fn3 (uq A) (uq B) (uq C) (uq R))))))

(letmacro letype/fn4
  (lambda Name A B C D R
    (qq
      (let (uq Name)
        (Fn4 (uq A) (uq B) (uq C) (uq D) (uq R))))))

(letmacro letype/fn5
  (lambda Name A B C D E R
    (qq
      (let (uq Name)
        (Fn5 (uq A) (uq B) (uq C) (uq D) (uq E) (uq R))))))

(letmacro letype/fn6
  (lambda Name A B C D E G R
    (qq
      (let (uq Name)
        (Fn6 (uq A) (uq B) (uq C) (uq D) (uq E) (uq G) (uq R))))))
