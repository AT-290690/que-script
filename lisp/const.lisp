(let *Id* [0])
(let Id! (lambda (do 
  (let id (&get *Id*))
  (&alter! *Id* (+ (&get *Id*) 1))
  id)))
(let const/int/max-safe 2147483647)
(let const/int/min-safe -2147483648)
(let const/dec/max-safe 2147483.647)
(let const/dec/min-safe -2147483.648)
(let const/dec/pi 3.142)
(let infinity 2147483647)
(let -infinity -2147483648)
(let Int 0)
(let Dec 0.0)
(let Char (&get "a"))
(let Bool false)
(let Nil nil)
(let as (lambda . t t))
(let : (lambda t x (do [ t x ] x)))
(let eq (lambda a b (cond 
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