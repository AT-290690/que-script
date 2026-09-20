(let *Id* [0])
(let Id! (lambda (do 
  (let id (&get *Id*))
  (&alter! *Id* (+ (&get *Id*) 1))
  id)))


(let const/int/max-safe 2147483647)
(let const/int/min-safe -2147483648)
(let const/dec/max-safe (+. 2147483.0 0.647))
(let const/dec/min-safe (-. -2147483.0 0.648))

; Arithmetic guards. These predicates perform only operations that are safe
; in the branch where they are evaluated, so they can also be used while
; debug overflow and divide-by-zero traps are enabled.
(let add/safe? (lambda a b
  (if (> b 0)
      (<= a (- const/int/max-safe b))
      (if (< b 0)
          (>= a (- const/int/min-safe b))
          true))))

(let sub/safe? (lambda a b
  (if (> b 0)
      (>= a (+ const/int/min-safe b))
      (if (< b 0)
          (<= a (+ const/int/max-safe b))
          true))))

(let mul/safe? (lambda a b
  (cond
    (= a 0) true
    (= b 0) true
    (> a 0) (if (> b 0)
                (<= a (/ const/int/max-safe b))
                (>= b (/ const/int/min-safe a)))
    (> b 0) (>= a (/ const/int/min-safe b))
    (>= a (/ const/int/max-safe b)))))

(let div/safe? (lambda a b
  (and (!= b 0)
       (not (and (= a const/int/min-safe) (= b -1))))))

(let mod/safe? (lambda a b (and (= a a) (!= b 0))))

(let add./safe? (lambda a b
  (if (>. b 0.0)
      (<=. a (-. const/dec/max-safe b))
      (if (<. b 0.0)
          (>=. a (-. const/dec/min-safe b))
          true))))

(let sub./safe? (lambda a b
  (if (>. b 0.0)
      (>=. a (+. const/dec/min-safe b))
      (if (<. b 0.0)
          (<=. a (+. const/dec/max-safe b))
          true))))

(let mul./safe? (lambda a b
  (cond
    (=. b 0.0) true
    (=. b 1.0) true
    (=. b -1.0) (not (=. a const/dec/min-safe))
    (>. b 1.0) (if (>. a 0.0)
                   (<=. a (/. const/dec/max-safe b))
                   (>=. a (/. const/dec/min-safe b)))
    (<. b -1.0) (if (>. a 0.0)
                    (<=. a (/. const/dec/min-safe b))
                    (>=. a (/. const/dec/max-safe b)))
    true)))

(let div./safe? (lambda a b
  (if (=. b 0.0)
      false
      (if (>=. b 1.0)
          true
          (if (<=. b -1.0)
              (not (and (=. a const/dec/min-safe) (=. b -1.0)))
              (if (>. b 0.0)
                  (and (>=. a (*. const/dec/min-safe b))
                       (<=. a (*. const/dec/max-safe b)))
                  (and (>=. a (*. const/dec/max-safe b))
                       (<=. a (*. const/dec/min-safe b)))))))))

(let %./safe? (lambda a b (and (=. a a) (not (=. b 0.0)))))

(let const/dec/pi 3.142)
(let const/dec/e 2.718)
(let const/dec/euler const/dec/e)
(let const/dec/ln2 0.693)
(let infinity 2147483647)
(let -infinity -2147483648)
(let Int 0)
(let Dec 0.0)
(let Char (&get "a"))
(let Bool false)
(let Nil nil)
(let as (lambda _ t t))
(let eq? (lambda a b (cond 
          (and a b) true 
          (and (not a) (not b)) true
          false)))
(let identity (lambda x x))
(let Char/nil (Int->Char 0))
(let Char/start (Int->Char 2))
(let Char/end (Int->Char 3))


(let const/dec/two-pi (+. const/dec/pi const/dec/pi))

(let const/dec/wrap-pi (lambda x (do
  (mut y x)
  (while (>. y const/dec/pi) (do
    (alter! y (-. y const/dec/two-pi))))
  (while (<. y (-. const/dec/pi)) (do
    (alter! y (+. y const/dec/two-pi))))
  y)))

(let const/dec/sin/terms (lambda x terms (do
  (let y (const/dec/wrap-pi x))
  (let x2 (*. y y))
  (mut term y)
  (mut sm y)
  (mut n 0)
  (while (< n terms) (do
    (let a (+ (* 2 n) 2))
    (let b (+ (* 2 n) 3))
    (let denom (Int->Dec (* a b)))
    (alter! term (/. (-. (*. term x2)) denom))
    (alter! sm (+. sm term))
    (alter! n (+ n 1))))
  sm)))

(let const/dec/cos/terms (lambda x terms (do
  (let y (const/dec/wrap-pi x))
  (let x2 (*. y y))
  (mut term 1.0)
  (mut sm 1.0)
  (mut n 0)
  (while (< n terms) (do
    (let a (+ (* 2 n) 1))
    (let b (+ (* 2 n) 2))
    (let denom (Int->Dec (* a b)))
    (alter! term (/. (-. (*. term x2)) denom))
    (alter! sm (+. sm term))
    (alter! n (+ n 1))))
  sm)))

; Public helpers (good default precision)
(let sin (lambda x (const/dec/sin/terms x 8)))
(let cos (lambda x (const/dec/cos/terms x 8)))

(let &box (lambda value [ value ]))
(let &alter! (lambda vrbl x (set! vrbl 0 x)))
(let &get (lambda vrbl (get vrbl 0)))


; Mulberry32 implemented in Que with explicit 32-bit unsigned arithmetic.
; Run:
;   que scripts/mulberry32.que
;   que scripts/mulberry32.que 1 5
;
; The generator is pure:
;   const/int/mulberry32/raw  : Int -> { Int * Int }
;   mulberry32/next : Int -> { Int * Dec }
;
; The first tuple element is the next seed/state.

(let const/int/byte-off (lambda x shift
  (& (>> x shift) 255)))

(let const/int/pack-u32 (lambda b0 b1 b2 b3
  (| b0 (| (<< b1 8) (| (<< b2 16) (<< b3 24))))))

(let const/int/u32/add (lambda a b (do
  (let s0 (+ (const/int/byte-off a 0) (const/int/byte-off b 0)))
  (let r0 (& s0 255))
  (let c0 (>> s0 8))

  (let s1 (+ (const/int/byte-off a 8) (const/int/byte-off b 8) c0))
  (let r1 (& s1 255))
  (let c1 (>> s1 8))

  (let s2 (+ (const/int/byte-off a 16) (const/int/byte-off b 16) c1))
  (let r2 (& s2 255))
  (let c2 (>> s2 8))

  (let s3 (+ (const/int/byte-off a 24) (const/int/byte-off b 24) c2))
  (let r3 (& s3 255))

  (const/int/pack-u32 r0 r1 r2 r3))))

(let const/int/u32/urshift (lambda x n
  (if (= n 0)
      x
      (& (>> x n) (- (<< 1 (- 32 n)) 1)))))

(let const/int/u32/mul (lambda a b (do
  (let a0 (const/int/byte-off a 0))
  (let a1 (const/int/byte-off a 8))
  (let a2 (const/int/byte-off a 16))
  (let a3 (const/int/byte-off a 24))
  (let b0 (const/int/byte-off b 0))
  (let b1 (const/int/byte-off b 8))
  (let b2 (const/int/byte-off b 16))
  (let b3 (const/int/byte-off b 24))

  (let s0 (* a0 b0))
  (let r0 (& s0 255))
  (let c0 (>> s0 8))

  (let s1 (+ (* a0 b1) (* a1 b0) c0))
  (let r1 (& s1 255))
  (let c1 (>> s1 8))

  (let s2 (+ (* a0 b2) (* a1 b1) (* a2 b0) c1))
  (let r2 (& s2 255))
  (let c2 (>> s2 8))

  (let s3 (+ (* a0 b3) (* a1 b2) (* a2 b1) (* a3 b0) c2))
  (let r3 (& s3 255))

  (const/int/pack-u32 r0 r1 r2 r3))))

(let const/dec/u32 (lambda x (do
  (mut acc (Int->Dec (const/int/byte-off x 0)))
  (alter! acc (+. (/. acc 256.0) (Int->Dec (const/int/byte-off x 8))))
  (alter! acc (+. (/. acc 256.0) (Int->Dec (const/int/byte-off x 16))))
  (alter! acc (+. (/. acc 256.0) (Int->Dec (const/int/byte-off x 24))))
  (/. acc 256.0))))

(let const/int/mulberry32/raw (lambda seed (do
  (let next-seed (const/int/u32/add seed 1831565813))
  (let z1 (const/int/u32/mul (^ next-seed (const/int/u32/urshift next-seed 15))
                   (| next-seed 1)))
  (let z2 (^ z1
              (const/int/u32/add z1
                       (const/int/u32/mul (^ z1 (const/int/u32/urshift z1 7))
                                (| z1 61)))))
  { next-seed (^ z2 (const/int/u32/urshift z2 14)) })))

(let const/dec/mulberry32/next (lambda seed (do
  (let step (const/int/mulberry32/raw seed))
  { (fst step) (const/dec/u32 (snd step)) })))

(let random/int const/int/mulberry32/raw)
(let random/dec const/dec/mulberry32/next)

(let random const/int/mulberry32/raw)
(let random. const/dec/mulberry32/next)

(let log (lambda x
  (if (<=. x 0.0)
      0.0
      (do
        (mut y x)
        (mut shifts 0)
        (while (>. y 2.0)
          (do
            (alter! y (/. y 2.0))
            (alter! shifts (+ shifts 1))))
        (while (<. y 1.0)
          (do
            (alter! y (*. y 2.0))
            (alter! shifts (- shifts 1))))
        (let z (/. (-. y 1.0) (+. y 1.0)))
        (let z2 (*. z z))
        (mut term z)
        (mut sm z)
        (mut n 1)
        (while (< n 12)
          (do
            (alter! term (*. term z2))
            (alter! sm (+. sm (/. term (Int->Dec (+ (* 2 n) 1)))))
            (alter! n (+ n 1))))
        (+. (*. 2.0 sm) (*. (Int->Dec shifts) const/dec/ln2))))))

  (let Payload/get (lambda (i) (if (> (length ARGV) i) (get ARGV i) "")))

  (let Meta/token->Pair
    (lambda (token)
        (let key [])
        (let value [])
        (mut seen? false)
        (mut i 0)
        (while (< i (length token))
            (let ch (get token i))
            (if (and (not seen?) (=# ch '='))
                (alter! seen? true)
                (if seen?
                    (push! value ch)
                    (push! key ch)))
            (++ i))
        { key value }))

  (let Meta/parse
    (lambda (text)
        (let out [])
        (for
          (lambda (token)
            (if (not-empty? token)
                (push! out (Meta/token->Pair token))))
          (split " " text))
        out))

  (let Meta/table
    (lambda (text)
      (reduce
        (lambda (acc pair)
            (Table/set! acc (fst pair) (snd pair))
            acc)
        (Table/new)
        (Meta/parse text))))

  (let Meta/has?
    (lambda (table key)
      (Table/has? key table)))

  (let Meta/get
    (lambda (table key)
      (if (Table/has? key table)
          (Table/get-unsafe key table)
          "")))

  (let Meta/get/text
    (lambda (text key)
      (Meta/get (Meta/table text) key)))

  (let prime? (lambda n
    (cond
      (< n 2) false
      (= n 2) true
      (= n 3) true
      (= (% n 2) 0) false
      (= (% n 3) 0) false

      (do
        (mut i 5)
        (mut ok true)

        (while (and ok (<= (* i i) n)) (do
          (if (or (= (% n i) 0)
                  (= (% n (+ i 2)) 0))
              (alter! ok false))

          (alter! i (+ i 6))))

        ok))))

(let encode/rl (lambda (xs)
  (mut prev (car xs))
  (mut counter 0)
  (let temp [])
  (let len (length xs))
  (mut i 0)
  (while (< i len)
    (let x (get xs i))
    (if (=# x prev)
      (++ counter)
      (do
        (push! temp { counter prev })
        (alter! prev x)
        (alter! counter 1)))
    (alter! i (+ i 1)))
  (push! temp { counter prev })
  temp))

(let decode/rl (lambda (xs)
  (let out [])
  (let len (length xs))
  (mut i 0)
  (while (< i len)
    (let { counter x } (get xs i))
    (mut n 0)
    (while (< n counter)
      (push! out x)
      (alter! n (+ n 1)))
    (alter! i (+ i 1)))
  out))

(let UTF8/continuation? (lambda (b) (= (& b 192) 128)))
(let UTF8/start-2? (lambda (b) (= (& b 224) 192)))
(let UTF8/start-3? (lambda (b) (= (& b 240) 224)))
(let UTF8/start-4? (lambda (b) (= (& b 248) 240)))

(let get* (lambda xs i some none (if (in-bounds? xs i) (do (some (get xs i)) nil) (do (none) nil))))

(let println! (lambda text (do
  (print! text)
  (print! ['\n']))))

(let int (lambda value
  (if (and (>= value const/int/min-safe) (<= value const/int/max-safe)) [value] [0])))

(let dec (lambda value
  (if (and (>=. value const/dec/min-safe) (<=. value const/dec/max-safe)) [value] [0.0])))

(let bool (lambda value [(=? value true)]))
