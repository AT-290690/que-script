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
(let +/safe? (lambda a b
  (if (> b 0)
      (<= a (- const/int/max-safe b))
      (if (< b 0)
          (>= a (- const/int/min-safe b))
          true))))

(let -/safe? (lambda a b
  (if (> b 0)
      (>= a (+ const/int/min-safe b))
      (if (< b 0)
          (<= a (+ const/int/max-safe b))
          true))))

(let */safe? (lambda a b
  (cond
    (= a 0) true
    (= b 0) true
    (> a 0) (if (> b 0)
                (<= a (/ const/int/max-safe b))
                (>= b (/ const/int/min-safe a)))
    (> b 0) (>= a (/ const/int/min-safe b))
    (>= a (/ const/int/max-safe b)))))

(let /safe? (lambda a b
  (and (!= b 0)
       (not (and (= a const/int/min-safe) (= b -1))))))

(let %/safe? (lambda a b (and (= a a) (!= b 0))))

(let +./safe? (lambda a b
  (if (>. b 0.0)
      (<=. a (-. const/dec/max-safe b))
      (if (<. b 0.0)
          (>=. a (-. const/dec/min-safe b))
          true))))

(let -./safe? (lambda a b
  (if (>. b 0.0)
      (>=. a (+. const/dec/min-safe b))
      (if (<. b 0.0)
          (<=. a (+. const/dec/max-safe b))
          true))))

(let *./safe? (lambda a b
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

(let /./safe? (lambda a b
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

; Mulberry32 implemented in Calque with explicit 32-bit unsigned arithmetic.
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

  (let Payload/get (lambda (i) (if (> (length ARGV) i) (get i ARGV) "")))

  (let Meta/token->Pair
    (lambda (token)
        (let key [])
        (let value [])
        (mut seen? false)
        (mut i 0)
        (while (< i (length token))
            (let ch (get i token))
            (if (and (not seen?) (=# ch '='))
                (alter! seen? true)
                (if seen?
                    (push! ch value)
                    (push! ch key)))
            (++ i))
        { key value }))

  (let Meta/parse
    (lambda (text)
        (let out [])
        (for
          (lambda (token)
            (if (not-empty? token)
                (push! (Meta/token->Pair token) out)))
          (split " " text))
        out))

  (let Meta/table
    (lambda (text)
      (reduce
        (lambda (acc pair)
            (Table/set! (fst pair) (snd pair) acc)
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
    (let x (get i xs))
    (if (=# x prev)
      (++ counter)
      (do
        (push! { counter prev } temp)
        (alter! prev x)
        (alter! counter 1)))
    (alter! i (+ i 1)))
  (push! { counter prev } temp)
  temp))

(let decode/rl (lambda (xs)
  (let out [])
  (let len (length xs))
  (mut i 0)
  (while (< i len)
    (let { counter x } (get i xs))
    (mut n 0)
    (while (< n counter)
      (push! x out)
      (alter! n (+ n 1)))
    (alter! i (+ i 1)))
  out))

(let UTF8/continuation? (lambda (b) (= (& b 192) 128)))
(let UTF8/start-2? (lambda (b) (= (& b 224) 192)))
(let UTF8/start-3? (lambda (b) (= (& b 240) 224)))
(let UTF8/start-4? (lambda (b) (= (& b 248) 240)))

(let get* (lambda i some none xs (if (in-bounds? i xs) (do (some (get i xs)) nil) (do (none) nil))))

(let println! (lambda text (do
  (print! text)
  (print! ['\n']))))

(let int (lambda value
  (if (and (>= value const/int/min-safe) (<= value const/int/max-safe)) [value] [0])))

(let dec (lambda value
  (if (and (>=. value const/dec/min-safe) (<=. value const/dec/max-safe)) [value] [0.0])))

(let bool (lambda value [(=? value true)]))

(let string/hash
  (lambda (s)
    (mut h 0)
    (loop i (< i (length s))
      (alter! h
        (% (+ (* h 31)
              (Char->Int (get i s)))
           65521)))
    h))


; Pure Calque Thompson-NFA regular expression compiler and matcher.
; ASCII-oriented classes; literal characters accept Unicode code points.
; Operators: (), |, *, +, ?, ., ^, $, [...], [^...], ranges and escapes.
; \d \D \w \W \s \S, \n \r \t; \ escapes a metacharacter.
; A compile failure is represented by program[0] = [0,-1].
;
; Program = [[valid,start], kinds, args, out1, out2, classOffsets, classData].
; Class records in classData: [negated, numberOfRanges, lo, hi, ...].
; State kinds: 0 split/epsilon, 1 literal, 2 any, 3 class,
;              4 accept, 5 begin-anchor, 6 end-anchor.

(let String/regex (lambda pattern (do
  (let post [])                 ; postfix tokens [kind, argument]
  (let ops [])                  ; 11='(', 6=concat, 7=alternation
  (let offsets [])
  (let class-data [])
  (mut i 0)
  (mut valid 1)
  (let can-end [0])
  (let quantifier [0])

  (let add-op (lambda op (do
    (let precedence (if (= op 6) 2 1))
    (mut running 1)
    (while (and (= running 1) (> (length ops) 0))
      (let top (get (- (length ops) 1) ops))
      (if (and (not (= top 11)) (>= (if (= top 6) 2 1) precedence))
        (do (do
          (push! [(Vector/pop-val! ops) 0] post)
          0) 0)
        (do (do (alter! running 0) 0) 0)))
    (push! op ops)
    0)))

  (let before-atom (lambda unused (do
    (if (= (get 0 can-end) 1) (do (add-op 6) 0) (do 0 0))
    (set! 0 1 can-end)
    (set! 0 0 quantifier)
    0)))

  (let append-range (lambda lo hi (do
    (push! lo class-data)
    (push! hi class-data)
    0)))

  (let make-shorthand (lambda code (do
    (let index (length offsets))
    (push! (length class-data) offsets)
    (push! (if (or (= code 68) (= code 87) (= code 83)) 1 0) class-data)
    (let count-cell (length class-data))
    (push! 0 class-data)
    (mut cnt 0)
    (if (or (= code 100) (= code 68) (= code 119) (= code 87))
      (do (do (append-range 48 57) (alter! cnt 1) 0) 0) (do 0 0))
    (if (or (= code 119) (= code 87))
      (do (do
        (append-range 65 90)
        (append-range 95 95)
        (append-range 97 122)
        (alter! cnt (+ cnt 3)) 0) 0) (do 0 0))
    (if (or (= code 115) (= code 83))
      (do (do
        (append-range 9 13)
        (append-range 32 32)
        (alter! cnt 2) 0) 0) (do 0 0))
    (set! count-cell cnt class-data)
    index)))

  (while (and (= valid 1) (< i (length pattern)))
    (let c (Char->Int (get i pattern)))
    (if (= c 40) ; (
      (do (do
        (if (= (get 0 can-end) 1) (do (add-op 6) 0) (do 0 0))
        (push! 11 ops)
        (set! 0 0 can-end)
        (set! 0 0 quantifier)
        0) 0)
      (do (if (= c 41) ; )
        (do (do
          (if (= (get 0 can-end) 0) (do (do (alter! valid 0) 0) 0) (do 0 0))
          (mut found 0)
          (while (and (> (length ops) 0) (= found 0))
            (let item (Vector/pop-val! ops))
            (if (= item 11)
              (do (do (alter! found 1) 0) 0)
              (do (do (push! [item 0] post) 0) 0)))
          (if (= found 0) (do (do (alter! valid 0) 0) 0) (do 0 0))
          (set! 0 1 can-end)
          (set! 0 0 quantifier)
          0) 0)
        (do (if (= c 124) ; |
          (do (do
            (if (= (get 0 can-end) 0) (do (do (alter! valid 0) 0) 0) (do (add-op 7) 0))
            (set! 0 0 can-end)
            (set! 0 0 quantifier)
            0) 0)
          (do (if (or (= c 42) (= c 43) (= c 63)) ; *, +, ?
            (do (do
              (if (or (= (get 0 can-end) 0) (= (get 0 quantifier) 1))
                (do (alter! valid 0) 0)
                (do (push! [(if (= c 42) 8 (if (= c 43) 9 10)) 0] post) 0))
              (set! 0 1 quantifier)
              0) 0)
            (do (if (= c 91) ; character class
              (do (do
                (before-atom 0)
                (let cls (length offsets))
                (push! (length class-data) offsets)
                (alter! i (+ i 1))
                (mut negated 0)
                (if (< i (length pattern))
                  (do (if (= (Char->Int (get i pattern)) 94)
                    (do (do (alter! negated 1) (alter! i (+ i 1)) 0) 0) (do 0 0)) 0)
                  (do (do (alter! valid 0) 0) 0))
                (push! negated class-data)
                (let count-cell (length class-data))
                (push! 0 class-data)
                (mut count 0)
                (mut pending -1)
                (mut dash 0)
                (mut closed 0)
                (mut at-start 1)
                (while (and (= valid 1) (= closed 0) (< i (length pattern)))
                  (mut code (Char->Int (get i pattern)))
                  (mut escaped 0)
                  (mut shorthand 0)
                  (if (= code 92)
                    (do (do
                      (alter! escaped 1)
                      (alter! i (+ i 1))
                      (if (< i (length pattern))
                        (do (do
                          (alter! code (Char->Int (get i pattern)))
                          (if (and (or (and (>= code 65) (<= code 90))
                                       (and (>= code 97) (<= code 122)))
                                   (not (or (= code 100) (= code 119) (= code 115)
                                            (= code 110) (= code 116) (= code 114)
                                            (= code 102) (= code 118))))
                            (do (alter! valid 0) 0) 0)
                          (if (or (= code 100) (= code 119) (= code 115))
                            (do (alter! shorthand code) 0) 0)
                          (if (or (= code 68) (= code 87) (= code 83))
                            (do (alter! valid 0) 0) 0)
                          (if (= code 110) (do (do (alter! code 10) 0) 0) (do 0 0))
                          (if (= code 116) (do (do (alter! code 9) 0) 0) (do 0 0))
                          (if (= code 114) (do (do (alter! code 13) 0) 0) (do 0 0))
                          (if (= code 102) (do (alter! code 12) 0) 0)
                          (if (= code 118) (do (alter! code 11) 0) 0)) 0)
                        (do (do (alter! valid 0) 0) 0))) 0) (do 0 0))
                  (if (= valid 1)
                    (do (do (if (> shorthand 0)
                      (do
                        (if (= dash 1) (do (alter! valid 0) 0) 0)
                        (if (>= pending 0)
                          (do (append-range pending pending)
                              (alter! count (+ count 1)) 0) 0)
                        (if (= shorthand 100)
                          (do (append-range 48 57) (alter! count (+ count 1)) 0) 0)
                        (if (= shorthand 119)
                          (do
                            (append-range 48 57)
                            (append-range 65 90)
                            (append-range 95 95)
                            (append-range 97 122)
                            (alter! count (+ count 4)) 0) 0)
                        (if (= shorthand 115)
                          (do (append-range 9 13)
                              (append-range 32 32)
                              (alter! count (+ count 2)) 0) 0)
                        (alter! pending -1)
                        (alter! at-start 0)
                        0)
                      (if (and (= code 93) (= escaped 0) (= at-start 0))
                      (do (alter! closed 1) 0)
                      (do (if (and (= code 45) (= escaped 0) (>= pending 0)
                               (= dash 0) (< (+ i 1) (length pattern))
                               (not (= (Char->Int (get (+ i 1) pattern)) 93)))
                        (do (alter! dash 1) 0)
                        (do (do
                          (if (= dash 1)
                            (do (do
                              (if (< code pending) (do (alter! valid 0) 0)
                                (do (do (append-range pending code)
                                    (alter! count (+ count 1))) 0))
                              (alter! dash 0)
                              (alter! pending -1)) 0)
                            (do (do
                              (if (>= pending 0)
                                (do (do (append-range pending pending)
                                    (alter! count (+ count 1)) 0) 0) (do 0 0))
                              (alter! pending code)) 0))
                          (alter! at-start 0)) 0)) 0))) 0) 0) (do 0 0))
                  (if (= closed 0) (do (do (alter! i (+ i 1)) 0) 0) (do 0 0)))
                (if (= closed 0) (do (do (alter! valid 0) 0) 0) (do 0 0))
                (if (= dash 1) (do (do (alter! valid 0) 0) 0) (do 0 0))
                (if (>= pending 0)
                  (do (do (append-range pending pending) (alter! count (+ count 1)) 0) 0) (do 0 0))
                (if (= count 0) (do (do (alter! valid 0) 0) 0) (do 0 0))
                (set! count-cell count class-data)
                (push! [3 cls] post)
                0) 0)
              (do (if (= c 92) ; escape
                (do (do
                  (alter! i (+ i 1))
                  (if (< i (length pattern))
                    (do (do
                      (let escaped (Char->Int (get i pattern)))
                      (if (and (or (and (>= escaped 65) (<= escaped 90))
                                   (and (>= escaped 97) (<= escaped 122)))
                               (not (or (= escaped 100) (= escaped 68)
                                        (= escaped 119) (= escaped 87)
                                        (= escaped 115) (= escaped 83)
                                        (= escaped 110) (= escaped 116) (= escaped 114)
                                        (= escaped 102) (= escaped 118))))
                        (do (alter! valid 0) 0) 0)
                      (before-atom 0)
                      (if (or (= escaped 100) (= escaped 68) (= escaped 119)
                              (= escaped 87) (= escaped 115) (= escaped 83))
                        (do (push! [3 (make-shorthand escaped)] post) 0)
                        (do (push! [1 (if (= escaped 110) 10
                                   (if (= escaped 116) 9
                                     (if (= escaped 114) 13
                                       (if (= escaped 102) 12 (if (= escaped 118) 11 escaped)))))] post) 0))) 0)
                    (do (alter! valid 0) 0))
                  0) 0)
                (do (if (or (= c 123) (= c 125) (= c 93)) ; unsupported special
                  (do (do (alter! valid 0) 0) 0)
                  (do (do
                    (before-atom 0)
                    (push! [(if (= c 46) 2 (if (= c 94) 5 (if (= c 36) 13 1)))
                            (if (or (= c 46) (= c 94) (= c 36)) 0 c)] post)
                    0) 0)) 0)) 0)) 0)) 0)) 0)) 0))
    (alter! i (+ i 1)))

  (if (and (= valid 1) (= (get 0 can-end) 0) (> (length pattern) 0))
    (do (alter! valid 0) 0) (do 0 0))
  (while (> (length ops) 0)
    (let op (Vector/pop-val! ops))
    (if (= op 11) (do (alter! valid 0) 0) (do (push! [op 0] post) 0)))
  (if (= (length pattern) 0) (do (push! [12 0] post) 0) (do 0 0))

  ; Thompson construction
  (let kinds [])
  (let args [])
  (let out1 [])
  (let out2 [])
  (let starts [])
  (let holes [])
  (let new-state (lambda kind argument (do
    (let id (length kinds))
    (push! kind kinds)
    (push! argument args)
    (push! -1 out1)
    (push! -1 out2)
    id)))
  (let patch (lambda links target (do
    (loop j (< j (length links))
      (let edge (get j links))
      (let id (/ edge 2))
      (if (= (% edge 2) 0)
        (do (set! id target out1) 0)
        (do (set! id target out2) 0)))
    0)))
  (let merge-holes (lambda first second (do
    (let result [])
    (loop j (< j (length first)) (push! (get j first) result))
    (loop j (< j (length second)) (push! (get j second) result))
    result)))

  (mut t 0)
  (while (and (= valid 1) (< t (length post)))
    (let token (get t post))
    (let kind (get 0 token))
    (let arg (get 1 token))
    (if (or (<= kind 5) (= kind 12) (= kind 13))
      (do (do
        (let id (new-state (if (= kind 12) 0 (if (= kind 13) 6 kind)) arg))
        ; input 5=BOL, 13=EOL; postfix 6 is concatenation.
        (push! id starts)
        (push! [(* id 2)] holes)) 0)
      (do (if (= kind 6)
        (do (if (< (length starts) 2)
          (do (alter! valid 0) 0)
          (do (do
            (let right-start (Vector/pop-val! starts))
            (let right-holes (Vector/pop-val! holes))
            (let left-start (Vector/pop-val! starts))
            (let left-holes (Vector/pop-val! holes))
            (patch left-holes right-start)
            (push! left-start starts)
            (push! right-holes holes)) 0)) 0)
        (do (if (= kind 7)
          (do (if (< (length starts) 2)
            (do (alter! valid 0) 0)
            (do (do
              (let alt-right-start (Vector/pop-val! starts))
              (let alt-right-holes (Vector/pop-val! holes))
              (let alt-left-start (Vector/pop-val! starts))
              (let alt-left-holes (Vector/pop-val! holes))
              (let alt-split (new-state 0 0))
              (set! alt-split alt-left-start out1)
              (set! alt-split alt-right-start out2)
              (push! alt-split starts)
              (push! (merge-holes alt-left-holes alt-right-holes) holes)) 0)) 0)
          (do (if (< (length starts) 1)
            (do (alter! valid 0) 0)
            (do (do
              (let child-start (Vector/pop-val! starts))
              (let child-holes (Vector/pop-val! holes))
              (let quant-split (new-state 0 0))
              (set! quant-split child-start out1)
              (if (= kind 8) ; *
                (do (do
                  (patch child-holes quant-split)
                  (push! quant-split starts)
                  (push! [(+ (* quant-split 2) 1)] holes)) 0)
                (do (if (= kind 9) ; +
                  (do (do
                    (patch child-holes quant-split)
                    (push! child-start starts)
                    (push! [(+ (* quant-split 2) 1)] holes)) 0)
                  (do (do ; ?
                    (push! quant-split starts)
                    (push! (merge-holes child-holes [(+ (* quant-split 2) 1)]) holes)) 0)) 0))) 0)) 0)) 0)) 0))
    (alter! t (+ t 1)))

  (mut start -1)
  (if (= valid 1)
    (do (do (if (= (length starts) 1)
      (do (do
        (alter! start (get 0 starts))
        (let accept (new-state 4 0))
        (patch (get 0 holes) accept)) 0)
      (do (alter! valid 0) 0)) 0) 0) (do 0 0))
  [[valid start] kinds args out1 out2 offsets class-data])))

(let const/regex/class-matches? (lambda cls character offsets data (do
  (let base (get cls offsets))
  (let neg (get base data))
  (let count (get (+ base 1) data))
  (mut found 0)
  (loop j (< j count)
    (let lo (get (+ base 2 (* j 2)) data))
    (let hi (get (+ base 3 (* j 2)) data))
    (if (and (>= character lo) (<= character hi)) (do (do (alter! found 1) 0) 0) (do 0 0)))
  (if (= neg 1) (= found 0) (= found 1)))))

; Add epsilon closure at a given character offset. Seen vector is mutated.
(let const/regex/add-closure! (lambda start offset text-length ends-newline kinds out1 out2 seen (do
  (let pending [start])
  (while (> (length pending) 0)
    (let id (Vector/pop-val! pending))
    (if (and (>= id 0) (= (get id seen) 0))
      (do (do
        (set! id 1 seen)
        (let kind (get id kinds))
        (if (= kind 0)
          (do (do
            (if (>= (get id out1) 0) (do (do (push! (get id out1) pending) 0) 0) (do 0 0))
            (if (>= (get id out2) 0) (do (do (push! (get id out2) pending) 0) 0) (do 0 0))) 0)
          (do (if (and (= kind 5) (= offset 0))
            (do (if (>= (get id out1) 0) (do (do (push! (get id out1) pending) 0) 0) (do 0 0)) 0)
            (do (if (and (= kind 6) (or (= offset text-length)
                                   (and (= ends-newline 1) (= offset (- text-length 1)))))
              (do (if (>= (get id out1) 0) (do (do (push! (get id out1) pending) 0) 0) (do 0 0)) 0)
              (do 0 0)) 0)) 0))) 0)
      (do 0 0)))
  0)))

; Internal matcher. mode 1=full match, mode 0=search for any substring.
; Return -1 for a malformed regex.
(let const/regex/run (lambda program text mode (do
  (let meta (get 0 program))
  (if (= (get 0 meta) 0)
    -1
    (do
      (let start (get 1 meta))
      (let kinds (get 1 program))
      (let args (get 2 program))
      (let out1 (get 3 program))
      (let out2 (get 4 program))
      (let offsets (get 5 program))
      (let data (get 6 program))
      (let n (length kinds))
      (let text-length (length text))
      (let ends-newline (if (> text-length 0)
        (if (= (Char->Int (get (- text-length 1) text)) 10) 1 0) 0))
      (let current (Vector/of n 0))
      (const/regex/add-closure! start 0 text-length ends-newline kinds out1 out2 current)
      (mut result 0)
      (mut pos 0)
      (while (and (= result 0) (<= pos text-length))
        (if (or (= mode 0) (= pos text-length))
          (do (do (loop id (< id n)
            (if (and (= (get id current) 1) (= (get id kinds) 4))
              (do (do (alter! result 1) 0) 0) (do 0 0))) 0) 0) (do 0 0))
        (if (and (= result 0) (< pos text-length))
          (do (do
            (let character (Char->Int (get pos text)))
            (let next (Vector/of n 0))
            (loop id (< id n)
              (if (= (get id current) 1)
                (do (do
                  (let kind (get id kinds))
                  (if (or (and (= kind 1) (= character (get id args)))
                          (and (= kind 2) (not (= character 10)))
                          (and (= kind 3) (const/regex/class-matches? (get id args) character offsets data)))
                    (do (const/regex/add-closure! (get id out1) (+ pos 1) text-length ends-newline kinds out1 out2 next) 0)
                    (do 0 0))) 0) (do 0 0)))
            (if (= mode 0)
              (do (const/regex/add-closure! start (+ pos 1) text-length ends-newline kinds out1 out2 next) 0) (do 0 0))
            (loop node (< node n)
              (set! node (get node next) current))
            0) 0) (do 0 0))
        (alter! pos (+ pos 1)))
      result)))))

(let String/regex/valid?
  (lambda regex
    (if (in-bounds? 0 regex)
      (block
        (let meta (get 0 regex))
        (if (in-bounds? 0 meta)
          (= (get 0 meta) 1)
          false))
      false)))

; `text` is final so compiled regexes compose naturally with string pipelines.
; `matches?` requires the whole text to match; `contains?` searches within it.
(let String/regex/matches?
  (lambda regex text
    (if (String/regex/valid? regex)
      (= (const/regex/run regex text 1) 1)
      false)))

(let String/regex/contains?
  (lambda regex text
    (if (String/regex/valid? regex)
      (= (const/regex/run regex text 0) 1)
      false)))

; Convenience forms when the pattern is not reused.
(let String/matches-regex?
  (lambda pattern text
    (String/regex/matches? (String/regex pattern) text)))

(let String/contains-regex?
  (lambda pattern text
    (String/regex/contains? (String/regex pattern) text)))
