(let Vector/append/raw! (lambda x xs (do (push! x xs) xs)))

(let Vector/cons/raw
  (lambda a b
    (do
      (let lena (length a))
      (let lenb (length b))
      (let out [])

      (mut i 0)
      (while (< i lena) (do
        (set! (length out) (get i a) out)
        (alter! i (+ i 1))))

      (mut j 0)
      (while (< j lenb) (do
        (set! (length out) (get j b) out)
        (alter! j (+ j 1))))

      out)))

(let Vector/cons/raw! (lambda b a (if (and (empty? a) (empty? b)) a (do
  (mut i 0)
  (let lenb (length b))
  (while (< i lenb) (do
    (set! (length a) (get i b) a)
    (alter! i (+ i 1))))
  a))))

(let Int/euclidean-mod (lambda a b (% (+ (% a b) b) b)))

(let Vector/to-string/raw (lambda delim xs (do
    (let out [])
    (let len-xs (length xs))
    (mut i 0)
    (while (< i len-xs) (do
      (if (> i 0)
          (set! (length out) delim out))
      (let current (get i xs))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! (length out) (get j current) out)
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    out)))

(let String/to-vector/raw (lambda ch str (do
    (let out [[]])
    (let len (length str))
    (mut i 0)
    (while (< i len) (do
      (let current (get i str))
      (if (=# current ch)
          (set! (length out) [] out)
          (do
            (let prev (get (- (length out) 1) out))
            (set! (length prev) current prev)))
      (alter! i (+ i 1))))
    out)))

(let String/chars->unsigned-dec (lambda xs (do
  (let parts (Chars->Digits/dec xs))
  (let pow (expt (length (get 1 parts)) 10))
  (/. (Int->Dec (+
    (* (Digits->Integer (get 0 parts)) pow)
    (Digits->Integer (get 1 parts)))) (Int->Dec pow)))))

(let Matrix/in-bounds/raw? (lambda y x matrix (and (in-bounds? y matrix) (in-bounds? x (get y matrix)))))

(let Matrix/set/raw! (lambda y x value matrix (do (set! x value (get y matrix)) 0)))

(let Dec/scaling 1000000.0)

(let Que/offset-left (lambda q (* (- (length (get 0 q)) 1) -1)))

(let Que/offset-right (lambda q (length (get 1 q))))

(let Que/empty/raw! (lambda q (do
    (set! 0 [(get 0 0 q)] q)
    (set! 1 [] q)
    q)))

(let Que/add-to-left! (lambda item q (do (let c (get 0 q)) (set! (length c) item c))))

(let Que/add-to-right! (lambda item q (do (let c (get 1 q)) (set! (length c) item c))))

(let Que/remove-from-left! (lambda q (do
  (let len (Que/length q))
  (if (> len 0)
     (cond
        (= len 1) (Que/empty/raw! q)
        (> (length (get 0 q)) 0) (do (pop! (get 0 q)) q)
        q) q))))

(let Que/remove-from-right! (lambda q (do
    (let len (Que/length q))
    (if (> len 0)
     (cond
        (= len 1) (Que/empty/raw! q)
        (> (length (get 1 q)) 0) (do (pop! (get 1 q)) q)
        q) q))))

(let Que/balance? (lambda q (= (+ (Que/offset-right q) (Que/offset-left q)) 0)))

(let Que/balance! (lambda q
    (if (Que/balance? q) q (do
      (let initial (Que->Vector q))
      (Que/empty/raw! q)
      (let half (/ (length initial) 2))
      (mut right half)
      (let len (length initial))
      (while (< right len) (do
        (Que/add-to-right! (get right initial) q)
        (alter! right (+ right 1))))
      (mut left (- half 1))
      (while (>= left 0) (do
        (Que/add-to-left! (get left initial) q)
        (alter! left (- left 1))))
    q))))

(let Que/append/raw! (lambda item q (do (Que/add-to-right! item q) q)))

(let Que/prepend/raw! (lambda item q (do (Que/add-to-left! item q) q)))

(let Que/head! (lambda q (do
    (if (= (Que/offset-right q) 0) (Que/balance! q) q)
    (Que/remove-from-right! q)
    q)))

(let Que/tail/raw! (lambda q (do
    (if (= (Que/offset-left q) 0) (Que/balance! q) q)
    (Que/remove-from-left! q)
q)))

(let Que/enqueue/raw! (lambda item queue (Que/append/raw! item queue)))

(let Vector/equal/raw? (lambda a b fn? (do
  (if (< (length a) (length b)) false
  (if (> (length a) (length b)) false
    (do
      (mut i 0)
      (mut result true)
      (let len (length a))
      (while (< i len) (do
        (let da (get i a))
        (let db (get i b))
        (if (not (fn? da db)) (do
          (alter! result false)
          (alter! i len)))
        (alter! i (+ i 1))))
      (if result true false)))))))

(let Vector/compare/raw (lambda a b step done? initial (do
    (mut state initial)
    (mut i 0)
    (let len-a (length a))
    (let len-b (length b))
    (let len (if (< len-a len-b) len-a len-b))
    (while (and (< i len) (not (done? state))) (do
      (alter! state (step state (get i a) (get i b) i))
      (alter! i (+ i 1))))
    { state len-a len-b })))

(let Matrix/fill/raw (lambda W H fn
  (cond
    (or (= W 0) (= H 0)) []
    (and (= W 1) (= H 1)) [[(fn 0 0)]] (do
      (let matrix [])
      (mut i 0)
      (while (< i W) (do
          (push! [] matrix)
          (mut j 0)
          (while (< j H) (do
            (Matrix/set/raw! i j (fn i j) matrix)
            (alter! j (+ j 1))))
          (alter! i (+ i 1))))
      matrix))))

(sig BigInt/remove-leading-zeroes (BigInt -> BigInt))
(let BigInt/remove-leading-zeroes
  (lambda value
    (do
      (let digits (BigInt/digits value))
      (mut i 0)
      (while (and (< i (length digits))
                  (zero? (get i digits))) (do
        (++ i)))
      (if (= i (length digits))
          (BigInt [0])
          (BigInt (slice i (length digits) digits))))))

(let StructBigDec/rescale-signed (lambda signed from-precision to-precision
  (if (= from-precision to-precision)
      signed
      (SignedBigInt/normalize (SignedBigInt
        (SignedBigInt/negative? signed)
        (BigInt/mul
          (SignedBigInt/digits signed)
          (StructBigDec/scale (- to-precision from-precision))))))))

(let StructBigDec/align-left (lambda a b
  (do
    (let precision (max (StructBigDec/precision a) (StructBigDec/precision b)))
    (StructBigDec/rescale-signed
      (StructBigDec/signed a)
      (StructBigDec/precision a)
      precision))))

(let StructBigDec/align-right (lambda a b
  (do
    (let precision (max (StructBigDec/precision a) (StructBigDec/precision b)))
    (StructBigDec/rescale-signed
      (StructBigDec/signed b)
      (StructBigDec/precision b)
      precision))))

(let Vector/to-tuple/raw (lambda fn1 fn2 xs { (fn1 xs) (fn2 xs) }))

(let Table/hash
 (lambda key table
   (do
     (let cap (length table))
     (__table-key-hash key cap))))

(let Table/create (lambda capacity (buckets (max 4 capacity))))

(let Table/max-capacity (lambda a b (Table/create (max (length a) (length b)))))

(let Table/find-index (lambda key bucket (do
  (mut i 0)
  (mut found -1)
  (let len (length bucket))
  (while (and (= found -1) (< i len)) (do
    (if (__table-key-equal? (fst (get i bucket)) key)
        (alter! found i)
        nil)
    (alter! i (+ i 1))))
  found)))

(let Table/for-each (lambda fn table (do
  (mut i 0)
  (let table-len (length table))
  (while (< i table-len) (do
    (let bucket (get i table))
    (mut j 0)
    (let bucket-len (length bucket))
    (while (< j bucket-len) (do
      (fn (get j bucket))
      (alter! j (+ j 1))))
    (alter! i (+ i 1)))))))

(let Table/set/unchecked! (lambda key value table (do
  (let idx (Table/hash key table))
  (let bucket (get idx table))
  (set! (length bucket) { key value } bucket)
  table)))

(let Table/resize/raw! (lambda new-capacity table (do
  (let target (max 4 new-capacity))
  (if (= target (length table))
      table
      (do
        (let entries [])
        (Table/for-each (lambda entry (set! (length entries) entry entries)) table)
        (Heap/empty! table)
        (mut i 0)
        (while (< i target) (do
          (set! (length table) [] table)
          (alter! i (+ i 1))))
        (mut j 0)
        (let entries-len (length entries))
        (while (< j entries-len) (do
          (let entry (get j entries))
          (Table/set/unchecked! (fst entry) (snd entry) table)
          (alter! j (+ j 1))))
        table)))))

(let Table/compact/raw! (lambda table (do
  (let used (Table/count-entries table))
  (let target (max 32 (* used 2)))
  (Table/resize/raw! target table))))

(let Table/has/raw? (lambda key table
  (if (= (length table) 0)
      false
      (do
        (let idx (Table/hash key table))
        (let bucket (get idx table))
        (>= (Table/find-index key bucket) 0)))))

(let Table/set/raw! (lambda key value table (do
  (if (= (length table) 0) (do (Table/resize/raw! 32 table) nil) nil)
  (let idx (Table/hash key table))
  (let bucket (get idx table))
  (let index (Table/find-index key bucket))
  (if (= index -1)
      (do
        (set! (length bucket) { key value } bucket)
        (if (> (length bucket) 8)
            (do (Table/resize/raw! (* (length table) 2) table) nil)
            nil))
      (set! index { key value } bucket))
  table)))

(let Table/remove/raw! (lambda key table (do
  (if (= (length table) 0)
      table
      (do
        (let idx (Table/hash key table))
        (let bucket (get idx table))
        (let index (Table/find-index key bucket))
        (if (>= index 0)
            (do
              (set! index (get (- (length bucket) 1) bucket) bucket)
              (pop! bucket))
            nil)
        (if (and (> (length table) 32) (= (length bucket) 0))
            (do
              (let used (Table/count-entries table))
              (if (< (* used 4) (length table))
                  (do (Table/resize/raw! (max 32 (/ (length table) 2)) table) nil)
                  nil))
            nil)
        table)))))

(let Table/get/raw (lambda key table
  (if (= (length table) 0)
      []
      (do
        (let idx (Table/hash key table))
        (let bucket (get idx table))
        (let index (Table/find-index key bucket))
        (if (>= index 0) [ (get index bucket) ] [])))))

(let Table/drop/raw! (lambda keys table (do
  (mut i 0)
  (let len (length keys))
  (while (< i len) (do
    (Table/remove/raw! (get i keys) table)
    (alter! i (+ i 1)))))))

(let Table/keep/raw (lambda keys table (do
  (let out (Table/create (max 32 (length keys))))
  (mut i 0)
  (let len (length keys))
  (while (< i len) (do
    (let key (get i keys))
    (let hit (Table/get/raw key table))
    (if (> (length hit) 0)
        (do (Table/set/raw! key (fst (get 0 hit)) out) nil)
        nil)
    (alter! i (+ i 1))))
  out)))

(let Table/merge/raw! (lambda b a (do
  (let entries (Table/entries b))
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (let entry (get i entries))
    (Table/set/raw! (fst entry) (snd entry) a)
    (alter! i (+ i 1))))
  a)))

(let Table/merge/raw (lambda b a (do
  (let out (Table/max-capacity a b))
  (Table/merge/raw! a out)
  (Table/merge/raw! b out)
  out)))

(let Table/omit/raw (lambda keys table (do
  (let out (Table/merge/raw table (Table/create 32)))
  (Table/drop/raw! keys out)
  out)))

(let String/prepend-zeroes (lambda (s target-len)
  (do
    (mut result s)
    (while (< (length result) target-len)
      (alter! result (cons "0" result)))
    result)))

(let Dec/to-chars/scale (lambda (scale x)
  (if (=. (floor x) x)
      (cons (Integer->Chars (Dec->Int x)) ".0")
      (do
        (let flip
          (if (<. x 0.0) -1.0 1.0))

        (let exponent
          (floor x))

        (let mantisa
          (-. x exponent))

        (let left
          (Integer->Chars (Dec->Int exponent)))

        (let right-unpadded
          (Integer->Chars
            (Dec->Int (*. mantisa Dec/scaling flip))))

        (let right
          (String/prepend-zeroes right-unpadded 6))

        (mut len (length right))
        (while (=# (get (- len 1) right) '0')
            (pop! right)
            (alter! len (- len 1)))

        (cons left ['.'] right)))))

(let String/matches-at?
  (lambda (data target offset)
    (let target-len (length target))
    (mut i 0)
    (mut same true)
    (while (and same (< i target-len))
      (do
        (if (not (=# (get (+ offset i) data)
                     (get i target)))
            (alter! same false))
        (alter! i (+ i 1))))
    same))

(let String/contains/raw?
  (lambda (data target)
    (let data-len (length data))
    (let target-len (length target))
    (if (= target-len 0)
        true
        (if (> target-len data-len)
            false
            (do
              (mut i 0)
              (mut found false)
              (while
                (and
                  (not found)
                  (<= i (- data-len target-len)))
                (do
                  (if
                    (String/matches-at?
                      data
                      target
                      i)
                    (alter! found true))
                  (alter! i (+ i 1))))
              found)))))

(let String/starts/raw?
  (lambda (data prefix)
    (let data-len (length data))
    (let prefix-len (length prefix))
    (if (> prefix-len data-len)
        false
        (String/matches-at?
          data
          prefix
          0))))

(let String/ends/raw?
  (lambda (data suffix)
    (let data-len (length data))
    (let suffix-len (length suffix))
    (if (> suffix-len data-len)
        false
        (String/matches-at?
          data
          suffix
          (- data-len suffix-len)))))

(let Digits->Integer (lambda digits-with-sign (do
    (let len (length digits-with-sign))
    (let negative? (and (> len 0) (< (get 0 digits-with-sign) 0)))
    (mut num 0)
    (mut i 0)
    (while (< i len) (do
      (let digit (get i digits-with-sign))
      (alter! num (+ (* num 10) (if negative? (abs digit) digit)))
      (alter! i (+ i 1))))
    (if negative? (- 0 num) num))))

(let Chars->Integer (lambda chars (do
    (let len (length chars))
    (mut sign 1)
    (mut i 0)
    (if (and (> len 0) (=# (get 0 chars) '-'))
        (do
          (alter! sign -1)
          (alter! i 1)))
    (mut num 0)
    (while (< i len) (do
      (alter! num (+ (* num 10) (Char->Digit (get i chars))))
      (alter! i (+ i 1))))
    (* num sign))))

(sig BigInt/equal? (BigInt -> BigInt -> Bool))
(let BigInt/equal? (lambda a b (do
  (let a-digits (BigInt/digits a))
  (let b-digits (BigInt/digits b))
  (if (< (length a-digits) (length b-digits)) false
  (if (> (length a-digits) (length b-digits)) false
    (do
      (mut i 0)
      (mut result true)
      (let len (length a-digits))
      (while (< i len) (do
        (let da (get i a-digits))
        (let db (get i b-digits))
        (if (not (= da db)) (do
          (alter! result false)
          (alter! i len)))
        (alter! i (+ i 1))))
      (if result true false)))))))

(let Vector/length (lambda xs (length xs)))

(let Vector/pop! (lambda xs (pop! xs)))

(let Vector/push! (lambda x xs (do (set! (length xs) x xs) xs)))

(let Heap/empty! (lambda xs (do (empty! xs) xs)))

(let Vector/overwrite! (lambda (ys xs) (Heap/empty! xs) (loop i (< i (length ys)) (set! i (get i ys) xs))))

(let StructDec/scale 1000)

(let StructDec/zero { false 0 0 })

(let StructDec/one { false 1 0 })

(let StructDec/pi { false 3 142 })

(let StructDec/e { false 2 718 })

(let StructDec/negative? (lambda n (if (fst n) true false)))

(let StructDec/whole (lambda n (fst (snd n))))

(let StructDec/fraction (lambda n (snd (snd n))))

(let StructDec/normalize (lambda n (do
  (let neg (fst n))
  (let whole (abs (fst (snd n))))
  (let frac-raw (abs (snd (snd n))))
  (let carry (/ frac-raw StructDec/scale))
  (let frac (% frac-raw StructDec/scale))
  (let normalized-whole (+ whole carry))
  (if (and (= normalized-whole 0) (= frac 0))
      StructDec/zero
      { neg normalized-whole frac }))))

(let StructDec/new (lambda neg whole fraction
  (StructDec/normalize { neg whole fraction })))

(let StructDec/from-int (lambda n
  (StructDec/new (< n 0) (abs n) 0)))

(let StructDec/scaled (lambda n
  (do
    (let mag (+ (* (StructDec/whole n) StructDec/scale)
                (StructDec/fraction n)))
    (if (StructDec/negative? n) (- mag) mag))))

(let StructDec/from-scaled (lambda n
  (do
    (let mag (abs n))
    (StructDec/new (< n 0)
                        (/ mag StructDec/scale)
                        (% mag StructDec/scale)))))

(let StructDec/negate (lambda n
  (if (= (StructDec/scaled n) 0)
      StructDec/zero
      { (not (StructDec/negative? n))
        (StructDec/whole n)
        (StructDec/fraction n) })))

(let StructDec/abs (lambda n
  { false (StructDec/whole n) (StructDec/fraction n) }))

(let StructDec/add (lambda a b
  (StructDec/from-scaled (+ (StructDec/scaled a)
                                 (StructDec/scaled b)))))

(let StructDec/sub (lambda a b
  (StructDec/from-scaled (- (StructDec/scaled a)
                                 (StructDec/scaled b)))))

(let StructDec/mul (lambda a b
  (StructDec/from-scaled (/ (* (StructDec/scaled a)
                                    (StructDec/scaled b))
                                 StructDec/scale))))

(let StructDec/div (lambda a b
  (StructDec/from-scaled (/ (* (StructDec/scaled a)
                                    StructDec/scale)
                                 (StructDec/scaled b)))))

(let StructDec/equal? (lambda a b (= (StructDec/scaled a) (StructDec/scaled b))))

(let StructDec/lt? (lambda a b (< (StructDec/scaled a) (StructDec/scaled b))))

(let StructDec/lte? (lambda a b (<= (StructDec/scaled a) (StructDec/scaled b))))

(let StructDec/gt? (lambda a b (> (StructDec/scaled a) (StructDec/scaled b))))

(let StructDec/gte? (lambda a b (>= (StructDec/scaled a) (StructDec/scaled b))))

(let StructDec/to-dec (lambda n
  (/. (Int->Dec (StructDec/scaled n)) (Int->Dec StructDec/scale))))

(let String/gt? (lambda a b (do
    (let len-a (length a))
    (let len-b (length b))
    (let min-len (if (< len-a len-b) len-a len-b))
    (mut i 0)
    (mut decided false)
    (mut out false)
    (while (and (not decided) (< i min-len)) (do
        (let ca (get i a))
        (let cb (get i b))
        (if (=# ca cb)
            (alter! i (+ i 1))
            (do
                (alter! out (># ca cb))
                (alter! decided true)))))
    (if decided out (> len-a len-b)))))

(let String/lt? (lambda a b (do
    (let len-a (length a))
    (let len-b (length b))
    (let min-len (if (< len-a len-b) len-a len-b))
    (mut i 0)
    (mut decided false)
    (mut out false)
    (while (and (not decided) (< i min-len)) (do
        (let ca (get i a))
        (let cb (get i b))
        (if (=# ca cb)
            (alter! i (+ i 1))
            (do
                (alter! out (<# ca cb))
                (alter! decided true)))))
    (if decided out (< len-a len-b)))))

(let String/gte? (lambda A B (or (match? A B) (String/gt? A B))))

(let String/lte? (lambda A B (or (match? A B) (String/lt? A B))))

(let Table/size (lambda matrix (length (flat matrix))))

(let Char->Digit (lambda digit (if (<# digit '0') 0 (- (as digit Int) (as '0' Int)))))

(let Chars->Digits (lambda digits (do
    (let out [])
    (let len (length digits))
    (mut i 0)
    (while (< i len) (do
      (set! (length out) (Char->Digit (get i digits)) out)
      (alter! i (+ i 1))))
    out)))

(let Chars->Digits/dec (lambda xs (do
    (let out [[]])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (let ch (get i xs))
      (if (=# ch '.')
          (push! [] out)
          (push! (Char->Digit ch) (at -1 out)))
      (alter! i (+ i 1))))
    out)))


(let Integer->Chars-base (lambda num base
    (if (= num 0) "0" (do
        (let neg? (< num 0))
        (mut n (if neg? (* num -1) num))
        (let str [])
        (while (> n 0) (do
            (let x (% n base))
            (push! (+# (Int->Char x) (char 48)) str)
            (alter! n (/ n base))))
        (if neg? (push! '-' str))
        (mut left 0)
        (mut right (- (length str) 1))
        (while (< left right) (do
            (let ch (get left str))
            (set! left (get right str) str)
            (set! right ch str)
            (alter! left (+ left 1))
            (alter! right (- right 1))))
        str))))

(let Digit->Char (lambda digit (if (< digit 0) '0' (+# (as digit Char) '0'))))

(let Digits->Chars (lambda digits (do
    (let out [])
    (let len (length digits))
    (mut i 0)
    (while (< i len) (do
      (set! (length out) (Digit->Char (get i digits)) out)
      (alter! i (+ i 1))))
    out)))



(let String->Dec (lambda xs
  (if (=# (get 0 xs) '-') (*. (String/chars->unsigned-dec (slice 1 (length xs) xs)) -1.0) (String/chars->unsigned-dec xs))))

(let Int->Alphabet
  (lambda x offset (Int->Char (+ x (Char->Int offset)))))

(let Heap/peek! (lambda heap (get 0 heap)))

(let Set->Vector (lambda xs (filter not-empty? (flat xs))))

(let Integer->Chars (lambda x (Integer->Chars-base x 10)))

(let Que/new (lambda def [[ def ] []]))

(let Que/length (lambda q (+ (length (get 0 q)) (length (get 1 q)) -1)))

(let Que/empty? (lambda q (= (Que/length q) 0)))

(let Que/get (lambda offset q (do
  (let offset-index (+ offset (Que/offset-left q)))
  (let index (if (< offset-index 0) (* offset-index -1) offset-index))
  (if (>= offset-index 0)
       (get index (get 1 q))
       (get index (get 0 q))))))

(let Vector->Que (lambda init initial (do
 (let q (Que/new init))
 (let half (/ (length initial) 2))
 (mut left (- half 1))
 (while (>= left 0) (do
    (Que/add-to-left! (get left initial) q)
    (alter! left (- left 1))))
 (mut right half)
 (let len (length initial))
 (while (< right len) (do
   (Que/add-to-right! (get right initial) q)
   (alter! right (+ right 1))))
    q)))

(let Que->Vector (lambda q (if (Que/empty? q) [(get 0 0 q)] (do
  (let out [])
  (mut index 0)
  (let len (Que/length q))
  (while (< index len) (do
      (set! (length out) (Que/get index q) out)
      (alter! index (+ index 1))))
    out))))

(let Que/peek (lambda q (Que/get 0 q)))

(let Que/last (lambda q (Que/get (- (Que/length q) 1) q)))

(let Que/not-empty? (lambda q (not (Que/empty? q))))

(let Integer->Bits (lambda num
    (if (= num 0) [ 0 ] (do
        (mut n num)
        (let out [])
        (while (> n 0) (do
            (push! (% n 2) out)
            (alter! n (/ n 2))))
        (reverse out)))))

(let Bits->Integer (lambda xs (do
  (let len (length xs))
  (mut index 0)
  (mut out 0)
  (while (< index len) (do
    (alter! out (+ out (* (at index xs) (expt (- len index 1) 2))))
    (alter! index (+ index 1))))
  out)))

(sig BigInt/new ([Char] -> BigInt))
(let BigInt/new (lambda text
  (BigInt/remove-leading-zeroes (BigInt (Chars->Digits text)))))

(sig BigInt/zero BigInt)
(let BigInt/zero (BigInt [0]))
(sig BigInt/one BigInt)
(let BigInt/one (BigInt [1]))

(sig BigInt/range (Int -> Int -> [BigInt]))
(let BigInt/range (lambda start end (do
  (let out [(BigInt/new (Integer->Chars start))])
  (mut i (+ start 1))
  (while (<= i end) (do
    (push! (BigInt/new (Integer->Chars i)) out)
    (++ i)))
  out)))

(sig BigInt/add (BigInt -> BigInt -> BigInt))
(let BigInt/add (lambda a1 b1 (do
  (let a (reverse (BigInt/digits a1)))
  (let b (reverse (BigInt/digits b1)))
  (let max-length (max (length a) (length b)))
  (let result [])
  (mut carry 0)
  (loop/range/exclusive i 0 max-length
    (let digit-a (if (< i (length a)) (get i a) 0))
    (let digit-b (if (< i (length b)) (get i b) 0))
    (let total (+ digit-a digit-b carry))
    (push! (% total 10) result)
    (alter! carry (/ total 10)))
  (while (> carry 0) (do
    (push! (% carry 10) result)
    (alter! carry (/ carry 10))))
  (BigInt (reverse result)))))

(sig BigInt/sub (BigInt -> BigInt -> BigInt))
(let BigInt/sub (lambda a1 b1
  ; BigInt is unsigned.  Do not silently wrap a negative result modulo 10.
  (if (BigInt/lt? a1 b1)
    (do (/ 1 0) BigInt/zero)
    (do
  (let a (reverse (BigInt/digits a1)))
  (let b (reverse (BigInt/digits b1)))
  (let result [])
  (mut borrow 0)
  (mut i 0)
  (let max-length (max (length a) (length b)))
  (while (< i max-length) (do
    (let digit-a (if (< i (length a)) (get i a) 0))
    (let digit-b (if (< i (length b)) (get i b) 0))
    (let difference (- (- digit-a digit-b) borrow))
    (if (< difference 0)
      (do (push! (+ difference 10) result) (alter! borrow 1))
      (do (push! difference result) (alter! borrow 0)))
    (++ i)))
  (mut last (- (length result) 1))
  (while (and (> last 0) (= (get last result) 0)) (do
    (pop! result)
    (alter! last (- last 1))))
  (BigInt (reverse result))))))

(sig BigInt/mul (BigInt -> BigInt -> BigInt))
(let BigInt/mul (lambda a1 b1 (do
  (let a (reverse (BigInt/digits a1)))
  (let b (reverse (BigInt/digits b1)))
  (let result [])
  (loop/range/exclusive _ 0 (+ (length a) (length b)) (push! 0 result))
  (loop/range/exclusive i 0 (length a)
    (mut carry 0)
    (let digit-a (get i a))
    (loop/range/exclusive j 0 (length b)
      (let index (+ i j))
      (let product (+ (* digit-a (get j b)) (get index result) carry))
      (set! index (% product 10) result)
      (alter! carry (/ product 10)))
    (mut index (+ i (length b)))
    (while (> carry 0) (do
      (if (not (< index (length result))) (push! 0 result))
      (let total (+ (get index result) carry))
      (set! index (% total 10) result)
      (alter! carry (/ total 10))
      (++ index))))
  (mut last (- (length result) 1))
  (while (and (> last 0) (= (get last result) 0)) (do
    (pop! result)
    (alter! last (- last 1))))
  (BigInt (reverse result)))))

(sig BigInt/lte? (BigInt -> BigInt -> Bool))
(let BigInt/lte? (lambda a b (do
  (let a-digits (BigInt/digits a))
  (let b-digits (BigInt/digits b))
  (if (< (length a-digits) (length b-digits)) true
  (if (> (length a-digits) (length b-digits)) false
    (do
      (mut i 0)
      (mut result true)
      (while (< i (length a-digits)) (do
        (let left (get i a-digits))
        (let right (get i b-digits))
        (if (< left right) (do (alter! result true) (alter! i (length a-digits))))
        (if (> left right) (do (alter! result false) (alter! i (length a-digits))))
        (++ i)))
      result))))))

(sig BigInt/gte? (BigInt -> BigInt -> Bool))
(let BigInt/gte? (lambda a b (BigInt/lte? b a)))

(sig BigInt/lt? (BigInt -> BigInt -> Bool))
(let BigInt/lt? (lambda a b (and (BigInt/lte? a b) (not (BigInt/equal? a b)))))

(sig BigInt/gt? (BigInt -> BigInt -> Bool))
(let BigInt/gt? (lambda a b (BigInt/lt? b a)))

(sig BigInt/div (BigInt -> BigInt -> BigInt))
(let BigInt/div (lambda dividend divisor
  ; Keep the partial arithmetic API aligned with Int division: zero divisors
  ; trap instead of manufacturing a digit-wise quotient.
  (if (BigInt/equal? divisor BigInt/zero)
    (do (/ 1 0) BigInt/zero)
    (do
  (let result [])
  (&mut current BigInt/zero)
  (let digits (BigInt/digits dividend))
  (mut i 0)
  (while (< i (length digits)) (do
    (let digit (get i digits))
    (let cur0 (&get current))
    (let cur1 (BigInt (cons (BigInt/digits cur0) [digit])))
    (&alter! current (BigInt/remove-leading-zeroes cur1))
    (mut low 0)
    (mut high 9)
    (mut quotient-digit 0)
    (while (<= low high) (do
      (let mid (/ (+ low high) 2))
      (if (BigInt/lte? (BigInt/mul divisor (BigInt [mid])) (&get current))
        (do (alter! quotient-digit mid) (alter! low (+ mid 1)))
        (alter! high (- mid 1)))))
    (push! quotient-digit result)
    (&alter! current (BigInt/sub (&get current)
                                  (BigInt/mul divisor (BigInt [quotient-digit]))))
    (++ i)))
  (BigInt/remove-leading-zeroes (BigInt result))))))

(sig BigInt/mod (BigInt -> BigInt -> BigInt))
(let BigInt/mod (lambda a b
  (BigInt/sub a (BigInt/mul b (BigInt/div a b)))))

(sig BigInt/square (BigInt -> BigInt))
(let BigInt/square (lambda x (BigInt/mul x x)))

(sig BigInt/div/floor (BigInt -> BigInt -> BigInt))
(let BigInt/div/floor (lambda a b (BigInt/div a b)))

(sig BigInt/div/ceal (BigInt -> BigInt -> BigInt))
(let BigInt/div/ceal (lambda a b
  (BigInt/div (BigInt/sub (BigInt/add a b) BigInt/one) b)))

(sig BigInt/sum ([BigInt] -> BigInt))
(let BigInt/sum (lambda xs (do
  (&mut total BigInt/zero)
  (loop i (< i (length xs))
    (&alter! total (BigInt/add (&get total) (get i xs))))
  (&get total))))

(sig BigInt/product ([BigInt] -> BigInt))
(let BigInt/product (lambda xs (do
  (&mut total BigInt/one)
  (loop i (< i (length xs))
    (&alter! total (BigInt/mul (&get total) (get i xs))))
  (&get total))))

(sig BigInt/pow (BigInt -> Int -> BigInt))
(let BigInt/pow (lambda a exponent
  (if (= exponent 0) BigInt/one (do
    (&mut out a)
    (loop i (< i (- exponent 1))
      (&alter! out (BigInt/mul (&get out) a)))
    (&get out)))))

(sig BigInt/expt (BigInt -> BigInt -> BigInt))
(let BigInt/expt (lambda a exponent
  (if (BigInt/equal? exponent BigInt/zero) BigInt/one (do
    (&mut out a)
    (&mut remaining (BigInt/sub exponent BigInt/one))
    (while (not (BigInt/equal? (&get remaining) BigInt/zero)) (do
      (&alter! out (BigInt/mul (&get out) a))
      (&alter! remaining (BigInt/sub (&get remaining) BigInt/one))))
    (&get out)))))

; Symbolic aliases keep arithmetic readable without losing the named API.
(sig BigInt/+ (BigInt -> BigInt -> BigInt))
(let BigInt/+ BigInt/add)
(sig BigInt/- (BigInt -> BigInt -> BigInt))
(let BigInt/- BigInt/sub)
(sig BigInt/* (BigInt -> BigInt -> BigInt))
(let BigInt/* BigInt/mul)
(sig BigInt// (BigInt -> BigInt -> BigInt))
(let BigInt// BigInt/div)

(sig SignedBigInt/zero SignedBigInt)
(let SignedBigInt/zero (SignedBigInt false BigInt/zero))
(sig SignedBigInt/one SignedBigInt)
(let SignedBigInt/one (SignedBigInt false BigInt/one))

(sig SignedBigInt/abs (SignedBigInt -> BigInt))
(let SignedBigInt/abs SignedBigInt/digits)

(sig SignedBigInt/zero? (SignedBigInt -> Bool))
(let SignedBigInt/zero? (lambda value
  (BigInt/equal? (SignedBigInt/digits value) BigInt/zero)))

(sig SignedBigInt/normalize (SignedBigInt -> SignedBigInt))
(let SignedBigInt/normalize (lambda value (do
  (let magnitude (BigInt/remove-leading-zeroes (SignedBigInt/digits value)))
  (if (BigInt/equal? magnitude BigInt/zero)
    SignedBigInt/zero
    (SignedBigInt (SignedBigInt/negative? value) magnitude)))))

(sig SignedBigInt/new ([Char] -> SignedBigInt))
(let SignedBigInt/new (lambda text
  (if (and (> (length text) 0) (=# (get 0 text) '-'))
    (SignedBigInt/normalize (SignedBigInt true (BigInt/new (slice 1 (length text) text))))
    (SignedBigInt/normalize (SignedBigInt false (BigInt/new text))))))

(sig SignedBigInt/negate (SignedBigInt -> SignedBigInt))
(let SignedBigInt/negate (lambda value
  (if (SignedBigInt/zero? value)
    SignedBigInt/zero
    (SignedBigInt (not (SignedBigInt/negative? value)) (SignedBigInt/digits value)))))

(sig SignedBigInt/equal? (SignedBigInt -> SignedBigInt -> Bool))
(let SignedBigInt/equal? (lambda a b
  (and (=? (SignedBigInt/negative? a) (SignedBigInt/negative? b))
       (BigInt/equal? (SignedBigInt/digits a) (SignedBigInt/digits b)))))

(sig SignedBigInt/lt? (SignedBigInt -> SignedBigInt -> Bool))
(let SignedBigInt/lt? (lambda a b
  (if (SignedBigInt/negative? a)
    (if (SignedBigInt/negative? b)
      (BigInt/gt? (SignedBigInt/digits a) (SignedBigInt/digits b))
      true)
    (if (SignedBigInt/negative? b)
      false
      (BigInt/lt? (SignedBigInt/digits a) (SignedBigInt/digits b))))))

(sig SignedBigInt/lte? (SignedBigInt -> SignedBigInt -> Bool))
(let SignedBigInt/lte? (lambda a b
  (or (SignedBigInt/equal? a b) (SignedBigInt/lt? a b))))

(sig SignedBigInt/gt? (SignedBigInt -> SignedBigInt -> Bool))
(let SignedBigInt/gt? (lambda a b (not (SignedBigInt/lte? a b))))

(sig SignedBigInt/gte? (SignedBigInt -> SignedBigInt -> Bool))
(let SignedBigInt/gte? (lambda a b (not (SignedBigInt/lt? a b))))

(sig SignedBigInt/add (SignedBigInt -> SignedBigInt -> SignedBigInt))
(let SignedBigInt/add (lambda a b
  (if (=? (SignedBigInt/negative? a) (SignedBigInt/negative? b))
    (SignedBigInt/normalize (SignedBigInt (SignedBigInt/negative? a)
                                           (BigInt/add (SignedBigInt/digits a) (SignedBigInt/digits b))))
    (if (BigInt/gt? (SignedBigInt/digits a) (SignedBigInt/digits b))
      (SignedBigInt/normalize (SignedBigInt (SignedBigInt/negative? a)
                                             (BigInt/sub (SignedBigInt/digits a) (SignedBigInt/digits b))))
      (if (BigInt/lt? (SignedBigInt/digits a) (SignedBigInt/digits b))
        (SignedBigInt/normalize (SignedBigInt (SignedBigInt/negative? b)
                                               (BigInt/sub (SignedBigInt/digits b) (SignedBigInt/digits a))))
        SignedBigInt/zero)))))

(sig SignedBigInt/sub (SignedBigInt -> SignedBigInt -> SignedBigInt))
(let SignedBigInt/sub (lambda a b (SignedBigInt/add a (SignedBigInt/negate b))))

(sig SignedBigInt/mul (SignedBigInt -> SignedBigInt -> SignedBigInt))
(let SignedBigInt/mul (lambda a b
  (SignedBigInt/normalize (SignedBigInt
    (not (=? (SignedBigInt/negative? a) (SignedBigInt/negative? b)))
    (BigInt/mul (SignedBigInt/digits a) (SignedBigInt/digits b))))))

(sig SignedBigInt/square (SignedBigInt -> SignedBigInt))
(let SignedBigInt/square (lambda x (SignedBigInt/mul x x)))

(sig SignedBigInt/pow (SignedBigInt -> Int -> SignedBigInt))
(let SignedBigInt/pow (lambda a exponent
  (if (= exponent 0)
    SignedBigInt/one
    (SignedBigInt/normalize (SignedBigInt
      (and (SignedBigInt/negative? a) (= (% exponent 2) 1))
      (BigInt/pow (SignedBigInt/digits a) exponent))))))

(sig SignedBigInt/expt (SignedBigInt -> BigInt -> SignedBigInt))
(let SignedBigInt/expt (lambda a exponent
  (if (BigInt/equal? exponent BigInt/zero)
    SignedBigInt/one
    (SignedBigInt/normalize (SignedBigInt
      (and (SignedBigInt/negative? a) (BigInt/equal? (BigInt/mod exponent (BigInt [2])) BigInt/one))
      (BigInt/expt (SignedBigInt/digits a) exponent))))))

(sig SignedBigInt/div (SignedBigInt -> SignedBigInt -> SignedBigInt))
(let SignedBigInt/div (lambda a b
  (SignedBigInt/normalize (SignedBigInt
    (not (=? (SignedBigInt/negative? a) (SignedBigInt/negative? b)))
    (BigInt/div (SignedBigInt/digits a) (SignedBigInt/digits b))))))

(sig SignedBigInt/mod (SignedBigInt -> SignedBigInt -> SignedBigInt))
(let SignedBigInt/mod (lambda a b
  (SignedBigInt/sub a (SignedBigInt/mul b (SignedBigInt/div a b)))))

(sig SignedBigInt/div/floor (SignedBigInt -> SignedBigInt -> SignedBigInt))
(let SignedBigInt/div/floor (lambda a b (do
  (let quotient (SignedBigInt/div a b))
  (let remainder (SignedBigInt/mod a b))
  (if (and (not (SignedBigInt/zero? remainder))
           (not (=? (SignedBigInt/negative? a) (SignedBigInt/negative? b))))
    (SignedBigInt/sub quotient SignedBigInt/one)
    quotient))))

(sig SignedBigInt/div/ceil (SignedBigInt -> SignedBigInt -> SignedBigInt))
(let SignedBigInt/div/ceil (lambda a b (do
  (let quotient (SignedBigInt/div a b))
  (let remainder (SignedBigInt/mod a b))
  (if (and (not (SignedBigInt/zero? remainder))
           (=? (SignedBigInt/negative? a) (SignedBigInt/negative? b)))
    (SignedBigInt/add quotient SignedBigInt/one)
    quotient))))

(sig SignedBigInt->String (SignedBigInt -> [Char]))
(let SignedBigInt->String (lambda value
  (if (SignedBigInt/negative? value)
    (cons "-" (Digits->Chars (BigInt/digits (SignedBigInt/digits value))))
    (Digits->Chars (BigInt/digits (SignedBigInt/digits value))))))

(sig SignedBigInt/range (SignedBigInt -> SignedBigInt -> [SignedBigInt]))
(let SignedBigInt/range (lambda start end
  (if (SignedBigInt/gt? start end) [] (do
    (let out [start])
    (&mut current start)
    (while (SignedBigInt/lt? (&get current) end) (do
      (&alter! current (SignedBigInt/add (&get current) SignedBigInt/one))
      (push! (&get current) out)))
    out))))

(sig SignedBigInt/sum ([SignedBigInt] -> SignedBigInt))
(let SignedBigInt/sum (lambda xs (do
  (&mut total SignedBigInt/zero)
  (loop i (< i (length xs))
    (&alter! total (SignedBigInt/add (&get total) (get i xs))))
  (&get total))))

(sig SignedBigInt/product ([SignedBigInt] -> SignedBigInt))
(let SignedBigInt/product (lambda xs (do
  (&mut total SignedBigInt/one)
  (loop i (< i (length xs))
    (&alter! total (SignedBigInt/mul (&get total) (get i xs))))
  (&get total))))

(sig StructBigDec/zero StructBigDec)
(let StructBigDec/zero (StructBigDec 0 SignedBigInt/zero))
(sig StructBigDec/one StructBigDec)
(let StructBigDec/one (StructBigDec 0 SignedBigInt/one))

(sig StructBigDec/negative? (StructBigDec -> Bool))
(let StructBigDec/negative? (lambda value
  (SignedBigInt/negative? (StructBigDec/signed value))))

(sig StructBigDec/scale (Int -> BigInt))
(let StructBigDec/scale (lambda precision (BigInt/pow (BigInt [1 0]) precision)))

(sig StructBigDec/from-scaled (Int -> SignedBigInt -> StructBigDec))
(let StructBigDec/from-scaled (lambda precision scaled
  (StructBigDec precision (SignedBigInt/normalize scaled))))

(sig StructBigDec/new (Int -> Bool -> BigInt -> BigInt -> StructBigDec))
(let StructBigDec/new (lambda precision negative whole fraction (do
  (let scale (StructBigDec/scale precision))
  (let scaled-whole (BigInt/mul whole scale))
  (StructBigDec/from-scaled precision
    (SignedBigInt negative (BigInt/add scaled-whole fraction))))))

(sig StructBigDec/new-digits (Bool -> BigInt -> BigInt -> StructBigDec))
(let StructBigDec/new-digits (lambda negative whole fraction
  (StructBigDec/new (length (BigInt/digits fraction)) negative whole fraction)))

(sig StructBigDec/pi StructBigDec)
(let StructBigDec/pi (StructBigDec/new-digits false (BigInt [3]) (BigInt [1 4 1 5 9 2 6 5 3 5 8 9 7 9 3])))
(sig StructBigDec/e StructBigDec)
(let StructBigDec/e (StructBigDec/new-digits false (BigInt [2]) (BigInt [7 1 8 2 8 1 8 2 8 4 5 9 0 4 5])))

(sig StructBigDec/from-int (Int -> StructBigDec))
(let StructBigDec/from-int (lambda value
  (StructBigDec 0 (SignedBigInt/new (Integer->Chars value)))))

(sig StructBigDec/from-digits (Int -> Bool -> BigInt -> StructBigDec))
(let StructBigDec/from-digits (lambda precision negative digits
  (StructBigDec/from-scaled precision (SignedBigInt negative digits))))

(sig StructBigDec/scaled (StructBigDec -> SignedBigInt))
(let StructBigDec/scaled StructBigDec/signed)

(sig StructBigDec/whole (StructBigDec -> BigInt))
(let StructBigDec/whole (lambda value
  (BigInt/div (SignedBigInt/digits (StructBigDec/signed value))
              (StructBigDec/scale (StructBigDec/precision value)))))

(sig StructBigDec/fraction (StructBigDec -> BigInt))
(let StructBigDec/fraction (lambda value
  (BigInt/mod (SignedBigInt/digits (StructBigDec/signed value))
              (StructBigDec/scale (StructBigDec/precision value)))))

(sig StructBigDec/add (StructBigDec -> StructBigDec -> StructBigDec))
(let StructBigDec/add (lambda a b (do
  (let precision (max (StructBigDec/precision a) (StructBigDec/precision b)))
  (StructBigDec/from-scaled precision
    (SignedBigInt/add (StructBigDec/align-left a b) (StructBigDec/align-right a b))))))

(sig StructBigDec/sub (StructBigDec -> StructBigDec -> StructBigDec))
(let StructBigDec/sub (lambda a b (do
  (let precision (max (StructBigDec/precision a) (StructBigDec/precision b)))
  (StructBigDec/from-scaled precision
    (SignedBigInt/sub (StructBigDec/align-left a b) (StructBigDec/align-right a b))))))

(sig StructBigDec/mul (StructBigDec -> StructBigDec -> StructBigDec))
(let StructBigDec/mul (lambda a b
  (StructBigDec/from-scaled
    (+ (StructBigDec/precision a) (StructBigDec/precision b))
    (SignedBigInt/mul (StructBigDec/signed a) (StructBigDec/signed b)))))

(sig StructBigDec/div/precision (Int -> StructBigDec -> StructBigDec -> StructBigDec))
(let StructBigDec/div/precision (lambda precision a b (do
  (let numerator (SignedBigInt/mul
    (StructBigDec/signed a)
    (SignedBigInt false (StructBigDec/scale (+ precision (StructBigDec/precision b))))))
  ; Multiplying the signed divisor by a positive scale preserves its sign.
  (let denominator (SignedBigInt/mul
    (StructBigDec/signed b)
    (SignedBigInt false (StructBigDec/scale (StructBigDec/precision a)))))
  (StructBigDec/from-scaled precision (SignedBigInt/div numerator denominator)))))

(sig StructBigDec/div (StructBigDec -> StructBigDec -> StructBigDec))
(let StructBigDec/div (lambda a b
  (StructBigDec/div/precision (max (StructBigDec/precision a) (StructBigDec/precision b)) a b)))

(sig StructBigDec/equal? (StructBigDec -> StructBigDec -> Bool))
(let StructBigDec/equal? (lambda a b
  (SignedBigInt/equal? (StructBigDec/align-left a b) (StructBigDec/align-right a b))))

(sig StructBigDec/lt? (StructBigDec -> StructBigDec -> Bool))
(let StructBigDec/lt? (lambda a b
  (SignedBigInt/lt? (StructBigDec/align-left a b) (StructBigDec/align-right a b))))

(sig StructBigDec/lte? (StructBigDec -> StructBigDec -> Bool))
(let StructBigDec/lte? (lambda a b
  (SignedBigInt/lte? (StructBigDec/align-left a b) (StructBigDec/align-right a b))))

(sig StructBigDec/gt? (StructBigDec -> StructBigDec -> Bool))
(let StructBigDec/gt? (lambda a b
  (SignedBigInt/gt? (StructBigDec/align-left a b) (StructBigDec/align-right a b))))

(sig StructBigDec/gte? (StructBigDec -> StructBigDec -> Bool))
(let StructBigDec/gte? (lambda a b
  (SignedBigInt/gte? (StructBigDec/align-left a b) (StructBigDec/align-right a b))))

(sig StructBigDec/negate (StructBigDec -> StructBigDec))
(let StructBigDec/negate (lambda value
  (StructBigDec/from-scaled (StructBigDec/precision value)
    (SignedBigInt/negate (StructBigDec/signed value)))))

(sig StructBigDec/abs (StructBigDec -> StructBigDec))
(let StructBigDec/abs (lambda value
  (StructBigDec/from-scaled (StructBigDec/precision value)
    (SignedBigInt false (SignedBigInt/digits (StructBigDec/signed value))))))

(sig StructBigDec->String (StructBigDec -> [Char]))
(let StructBigDec->String (lambda value (do
  (let precision (StructBigDec/precision value))
  (let whole (Digits->Chars (BigInt/digits (StructBigDec/whole value))))
  (let fraction (String/prepend-zeroes
    (Digits->Chars (BigInt/digits (StructBigDec/fraction value))) precision))
  (let sign (if (StructBigDec/negative? value) "-" ""))
  (if (= precision 0) (cons sign whole) (cons sign whole "." fraction)))))

(let Integer->Digits (lambda num (Integer->Digits-base num 10)))

(let Tuple/swap (lambda { a b } { b a }))

(let Tuple/int/add (lambda { a b } (+ a b)))

(let Tuple/int/sub (lambda { a b } (- a b)))

(let Tuple/int/mul (lambda { a b } (* a b)))

(let Tuple/int/div (lambda { a b } (* a b)))

(let Set/hash (lambda key table (do
  (let cap (length table))
  (if (= cap 0) 0 (do
    (mut i 0)
    (mut hash 0)
    (let len (length key))
    (while (< i len) (do
      (alter! hash (Int/euclidean-mod
        (+ (* hash 131) (as (get i key) Int))
        cap))
      (alter! i (+ i 1))))
    hash)))))

(let Set/key-equal? (lambda a b (do
  (let len (length a))
  (if (not (= len (length b))) false (do
    (mut i 0)
    (mut matches true)
    (while (and matches (< i len)) (do
      (if (not (=# (get i a) (get i b))) (alter! matches false) nil)
      (alter! i (+ i 1))))
    matches)))))

(let Set/find-index (lambda key bucket (do
  (mut i 0)
  (mut found -1)
  (let len (length bucket))
  (while (and (= found -1) (< i len)) (do
    (if (Set/key-equal? (get i bucket) key) (alter! found i) nil)
    (alter! i (+ i 1))))
  found)))

(let Set/count (lambda table (do
  (mut total 0)
  (mut i 0)
  (while (< i (length table)) (do
    (alter! total (+ total (length (get i table))))
    (alter! i (+ i 1))))
  total)))

(let Set/for-each (lambda fn table (do
  (mut i 0)
  (let table-len (length table))
  (while (< i table-len) (do
    (let bucket (get i table))
    (mut j 0)
    (let bucket-len (length bucket))
    (while (< j bucket-len) (do
      (fn (get j bucket))
      (alter! j (+ j 1))))
    (alter! i (+ i 1)))))))

(let Set/add/raw! (lambda key table (do
  (let idx (Set/hash key table))
  (let bucket (get idx table))
  (set! (length bucket) key bucket)
  table)))

(let Set/new (lambda (buckets 32)))
(let Set/new/capacity (lambda n (buckets (max 4 n))))
(let Set/max-capacity (lambda a b (Set/new/capacity (max (length a) (length b)))))

(let Set/resize! (lambda new-capacity table (do
  (let target (max 4 new-capacity))
  (if (not (= target (length table))) (do
    (let entries [])
    (Set/for-each (lambda key (set! (length entries) key entries)) table)
    (Heap/empty! table)
    (mut i 0)
    (while (< i target) (do (set! (length table) [] table) (alter! i (+ i 1))))
    (mut j 0)
    (while (< j (length entries)) (do
      (Set/add/raw! (get j entries) table)
      (alter! j (+ j 1))))) nil)
  nil)))

(let Set/compact! (lambda table (do
  (Set/resize! (max 32 (* (Set/count table) 2)) table)
  nil)))

(let Set/has? (lambda item table
  (if (= (length table) 0) false (do
    (let idx (Set/hash item table))
    (>= (Set/find-index item (get idx table)) 0)))))

(let Set/add! (lambda item table (do
  (if (= (length table) 0) (Set/resize! 32 table) nil)
  (let idx (Set/hash item table))
  (let bucket (get idx table))
  (if (= (Set/find-index item bucket) -1) (do
    (set! (length bucket) item bucket)
    (if (> (length bucket) 8) (Set/resize! (* (length table) 2) table) nil)) nil)
  nil)))

(let Set/remove! (lambda item table (do
  (if (not (= (length table) 0)) (do
    (let idx (Set/hash item table))
    (let bucket (get idx table))
    (let index (Set/find-index item bucket))
    (if (>= index 0) (do
      (set! index (get (- (length bucket) 1) bucket) bucket)
      (pop! bucket)) nil)
    (if (and (> (length table) 32) (= (length bucket) 0)
             (< (* (Set/count table) 4) (length table)))
        (Set/resize! (max 32 (/ (length table) 2)) table) nil)) nil)
  nil)))

(let Vector->Set (lambda xs (do
  (let out (Set/new/capacity (max 32 (length xs))))
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do
    (Set/add! (get i xs) out)
    (alter! i (+ i 1))))
  out)))

(let Set/intersection (lambda b a (do
  (let out (Set/max-capacity a b))
  (let a-count (Set/count a))
  (let b-count (Set/count b))
  (let src (if (< a-count b-count) a b))
  (let trg (if (< a-count b-count) b a))
  (Set/for-each (lambda key
    (if (and (not-empty? key) (Set/has? key trg))
        (do (Set/add! key out) nil)
        nil)) src)
  out)))

(let Set/difference (lambda b a (do
  (let out (Set/max-capacity a b))
  (Set/for-each (lambda key
    (if (and (not-empty? key) (not (Set/has? key b)))
        (do (Set/add! key out) nil)
        nil)) a)
  out)))

(let Set/xor (lambda b a (do
  (let out (Set/max-capacity a b))
  (Set/for-each (lambda key
    (if (and (not-empty? key) (not (Set/has? key b)))
        (do (Set/add! key out) nil)
        nil)) a)
  (Set/for-each (lambda key
    (if (and (not-empty? key) (not (Set/has? key a)))
        (do (Set/add! key out) nil)
        nil)) b)
  out)))

(let Set/union (lambda b a (do
  (let out (Set/max-capacity a b))
  (Set/for-each (lambda key
    (if (not-empty? key) (do (Set/add! key out) nil) nil)) a)
  (Set/for-each (lambda key
    (if (not-empty? key) (do (Set/add! key out) nil) nil)) b)
  out)))

(let Table/update! (lambda key init f table (do
  (if (= (length table) 0) (do (Table/resize/raw! 32 table) nil) nil)
  (let idx (Table/hash key table))
  (let bucket (get idx table))
  (let index (Table/find-index key bucket))
  (if (>= index 0)
      (set! index { (fst (get index bucket))
          (f (snd (get index bucket))) } bucket)
      (do
        (set! (length bucket) { key init } bucket)
        (if (> (length bucket) 8)
            (do (Table/resize/raw! (* (length table) 2) table) nil)
            nil)))
  nil)))

(let Table/update-or! (lambda key missing present table (do
  (if (= (length table) 0) (do (Table/resize/raw! 32 table) nil) nil)
  (let idx (Table/hash key table))
  (let bucket (get idx table))
  (let index (Table/find-index key bucket))
  (if (>= index 0)
      (set! index { (fst (get index bucket))
          (present (snd (get index bucket))) } bucket)
      (do
        (set! (length bucket) { key (missing) } bucket)
        (if (> (length bucket) 8)
            (do (Table/resize/raw! (* (length table) 2) table) nil)
            nil)))
  nil)))

(let Table/push-or! (lambda key value table (do
  (if (= (length table) 0) (do (Table/resize/raw! 32 table) nil) nil)
  (let idx (Table/hash key table))
  (let bucket (get idx table))
  (let index (Table/find-index key bucket))
  (if (>= index 0)
      (push! value (snd (get index bucket)))
      (do
        (set! (length bucket) { key [value] } bucket)
        (if (> (length bucket) 8)
            (do (Table/resize/raw! (* (length table) 2) table) nil)
            nil)))
  nil)))

(let Table/entries (lambda table (do
  (let out [])
  (Table/for-each (lambda entry (set! (length out) entry out)) table)
  out)))

(let Table/keys (lambda table (do
  (let entries (Table/entries table))
  (let out [])
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (set! (length out) (fst (get i entries)) out)
    (alter! i (+ i 1))))
  out)))

(let Table/values (lambda table (do
  (let entries (Table/entries table))
  (let out [])
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (set! (length out) (snd (get i entries)) out)
    (alter! i (+ i 1))))
  out)))

(let Table/count (lambda arr (do
  (let table (Table/create (max 64 (length arr))))
  (mut i 0)
  (let len (length arr))
  (while (< i len) (do
    (let key (get i arr))
    (let idx (Table/hash key table))
    (let bucket (get idx table))
    (let index (Table/find-index key bucket))
    (if (>= index 0)
        (set! index { (fst (get index bucket))
            (+ (snd (get index bucket)) 1) } bucket)
        (do
          (set! (length bucket) { key 1 } bucket)
          (if (> (length bucket) 8)
              (do (Table/resize/raw! (* (length table) 2) table) nil)
              nil)))
    (alter! i (+ i 1))))
  table)))

(let Table/frequency (lambda xs (do
  (let table (Table/create (max 64 (length xs))))
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do
    (let key [(get i xs)])
    (let idx (Table/hash key table))
    (let bucket (get idx table))
    (let index (Table/find-index key bucket))
    (if (>= index 0)
        (set! index { (fst (get index bucket))
            (+ (snd (get index bucket)) 1) } bucket)
        (do
          (set! (length bucket) { key 1 } bucket)
          (if (> (length bucket) 8)
              (do (Table/resize/raw! (* (length table) 2) table) nil)
              nil)))
    (alter! i (+ i 1))))
  table)))

(let Table->Vector Table/entries)
(let Vector->Table (lambda entries (do
  (let out (Table/create (max 32 (length entries))))
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (let entry (get i entries))
    (Table/set/raw! (fst entry) (snd entry) out)
    (alter! i (+ i 1))))
  out)))

(let String/find (lambda target xs (do
  (let len (length xs))
  (mut i 0)
  (mut result -1)
  (while (and (= result -1) (< i len)) (do
    (if (match? (get i xs) target)
        (alter! result i))
    (alter! i (+ i 1))))
  result)))

(let String/find/last (lambda target xs (do
  (let len (length xs))
  (mut i 0)
  (mut result -1)
  (while (< i len) (do
    (if (match? (get i xs) target)
        (alter! result i))
    (alter! i (+ i 1))))
  result)))

(let Dec->Chars (Dec/to-chars/scale 6))



(let box (lambda value [ value ]))
(let true? (lambda vrbl (if (get vrbl) true false)))
(let false? (lambda vrbl (if (get vrbl) false true)))

(let Bool->Int (lambda x (if (=? x true) 1 0)))
(let Bool->Char (lambda x (if (=? x true) '1' '0')))
(let Char->Int (lambda x (if (>=# x '') (as x Int) 0)))
(let Char->Bool (lambda x (if (or (=# x '') (=# x '0')) false true)))
(let Int->Bool (lambda x 
    (cond 
        (<= x 0) false
        (>= x 1) true
        false)))
(let Int->Char (lambda x (if (>= x 0) (as x Char) '')))

(let Tuple/new (lambda a b (tuple a b)))
(let Tuple/map map/tuple)
(let Tuple/map/fst map/fst)
(let Tuple/map/snd map/snd)

(let Vector->Tuple (lambda fn1 fn2 xs (Vector/to-tuple/raw fn1 fn2 xs)))

(let Tuple/bool/eq? (lambda { a b } (=? a b)))
(let Tuple/int/eq? (lambda { a b } (= a b)))
(let Tuple/char/eq? (lambda { a b } (=# a b)))
(let Tuple/dec/eq? (lambda { a b } (=. a b)))

(let Tuple/bool/not-eq? (lambda { a b } (not (=? a b))))
(let Tuple/int/not-eq? (lambda { a b } (not (= a b))))
(let Tuple/char/not-eq? (lambda { a b } (not (=# a b))))
(let Tuple/dec/not-eq? (lambda { a b } (not (=. a b))))


(let Integer->String Integer->Chars)
(let Dec->String Dec->Chars)

(let String->Vector (lambda ch xs (String/to-vector/raw ch xs)))
(let Vector->String (lambda ch xs (Vector/to-string/raw ch xs)))
(let String->Integer Chars->Integer)
(let Chars->Dec String->Dec)

(let Vector/equal? (lambda fn? xs ys (Vector/equal/raw? xs ys fn?)))

(let Que/empty! (lambda q (do (Que/empty/raw! q) nil)))
(let Que/enque! (lambda v xs (do (Que/enqueue/raw! v xs) nil)))
(let Que/deque! (lambda queue (do (Que/tail/raw! queue) nil)))
(let Que/push! (lambda v xs (do (Que/append/raw! v xs) nil)))
(let Que/pop! (lambda queue (do (Que/head! queue) nil)))
(let Que/prepend! (lambda v xs (do (Que/prepend/raw! v xs) nil)))
(let Que/first Que/peek)
(let Que/tail! (lambda queue (do (Que/tail/raw! queue) nil)))
(let Que/append! (lambda v xs (do (Que/append/raw! v xs) nil)))

(let Que/at (lambda i xs (if (< i 0) (Que/get (+ (length xs) i) xs) (Que/get i xs))))

(let Vector/pop-val! pop-val!)
(let Vector/last last)
(let Vector/first first)
(let Vector/pull! pop-val!)
(let Vector/get-unsafe (lambda idx xs (get idx xs)))
(let Vector/get* get*)
(let Vector/at-unsafe (lambda idx xs (at idx xs)))
(let Vector/at* at*)
(let Vector/find* find*)
(let Vector/set! (lambda i v xs (set! i v xs)))
(let Vector/append! (lambda x xs (Vector/append/raw! x xs)))
(let Vector/cons! (lambda x xs (Vector/cons/raw! x xs)))
(let Vector/cons (lambda x xs (Vector/cons/raw xs x)))
(let Vector/compare (lambda step done? initial b a (Vector/compare/raw a b step done? initial)))
(let Vector/new (lambda fn n (fill n fn)))
(let Vector/of (lambda (n x)
  (let out [])
  (loop i (< i n) (push! x out))
  out))
(let Matrix/new (lambda fn w h (Matrix/fill/raw w h fn)))
(let Vector/in-bounds? (lambda (index xs) (in-bounds? index xs)))
(let Matrix/in-bounds? (lambda (y x xs) (Matrix/in-bounds/raw? y x xs)))
(let String/equal? match?)
(let String/quote (lambda str (cons "'" str "'")))
(let String/dquote (lambda str (cons ['"'] str ['"'])))
(let String->Bool (lambda str (or (match? text "true")
      (match? text "1")
      (match? text "yes"))))
(let Char/eq? (lambda a b (=# a b)))
(let Int/eq? (lambda a b (= a b)))
(let Bool/eq? (lambda a b (=? a b)))

(let String/starts? (lambda (needle xs) (String/starts/raw? xs needle)))
(let String/ends? (lambda (needle xs) (String/ends/raw? xs needle)))
(let String/contains? (lambda (needle xs) (String/contains/raw? xs needle)))

(let Dec/eq? (lambda a b (=. a b)))

; data-last aliases mirroring Table/*
(let Table/new (lambda (buckets 32)))
(let Table/new/capacity (lambda n (buckets (max 4 n))))


(let Table/get (lambda key table (Table/get/raw key table)))
(let Table/get* (lambda (key table)
  (let hit (Table/get/raw key table))
  (if (empty? hit)
      None
      (Some (snd (get 0 hit))))))
(let Table/get-unsafe (lambda key table (snd (get 0 (Table/get/raw key table)))))

(let Table/has? (lambda key table (Table/has/raw? key table)))
(let Table/set! (lambda key value table (do (Table/set/raw! key value table) nil)))
(let Table/remove! (lambda key table (do (Table/remove/raw! key table) nil)))

(let Table/drop! (lambda keys table (Table/drop/raw! keys table)))
(let Table/keep (lambda keys table (Table/keep/raw keys table)))
(let Table/omit (lambda keys table (Table/omit/raw keys table)))

(let Table/merge! (lambda other table (do (Table/merge/raw! other table) nil)))
(let Table/merge (lambda other table (Table/merge/raw other table)))

(let Table/resize! (lambda n table (do (Table/resize/raw! n table) nil)))
(let Table/compact! (lambda table (do (Table/compact/raw! table) nil)))

(let Table/create/new Table/create)
(let Table/count-entries (lambda table (do
  (mut total 0)
  (mut i 0)
  (let len (length table))
  (while (< i len) (do
    (alter! total (+ total (length (get i table))))
    (alter! i (+ i 1))))
  total)))

(let Table/get/raw* (lambda i some none xs (if (Table/has/raw? i xs) (do (some (Table/get/raw i xs)) nil) (do (none) nil))))

(let Set/size Table/size)
(let Set/values flat)

(let Heap/parent (lambda i (- (>> (+ i 1) 1) 1)))
(let Heap/left (lambda i (+ (<< i 1) 1)))
(let Heap/right (lambda i (<< (+ i 1) 1)))
(let Heap/compare? (lambda i j fn? heap (=? (fn? (get i heap) (get j heap)) true)))

(let Heap/swap! (lambda i j heap (do
  (let value (get i heap))
  (set! i (get j heap) heap)
  (set! j value heap))))

(let Heap/sift-up! (lambda fn heap (do
  (mut node (- (length heap) 1))
  (while (and (> node 0) (Heap/compare? node (Heap/parent node) fn heap)) (do
    (Heap/swap! node (Heap/parent node) heap)
    (alter! node (Heap/parent node)))))))

(let Heap/sift-down! (lambda fn heap (do
  (mut node 0)
  (mut active true)
  (while active (do
    (let left (Heap/left node))
    (let right (Heap/right node))
    (if (or
          (and (< left (length heap)) (Heap/compare? left node fn heap))
          (and (< right (length heap)) (Heap/compare? right node fn heap)))
        (block
          (let child (if (and (< right (length heap)) (Heap/compare? right left fn heap)) right left))
          (Heap/swap! node child heap)
          (alter! node child))
        (alter! active false)))))))

(let Heap/push! (lambda value fn heap (do
  (set! (length heap) value heap)
  (Heap/sift-up! fn heap)
  nil)))

(let Heap/pop! (lambda fn heap (do
  (let bottom (- (length heap) 1))
  (if (> bottom 0) (Heap/swap! 0 bottom heap) nil)
  (pop! heap)
  (if (not (empty? heap)) (Heap/sift-down! fn heap) nil)
  nil)))

(let Heap/replace! (lambda value fn heap (do
  (set! 0 value heap)
  (Heap/sift-down! fn heap)
  heap)))
(let Heap/empty? empty?)
(let Heap/not-empty? not-empty?)
(let Vector->Heap (lambda fn xs (do
  (let heap [])
  (mut i 0)
  (while (< i (length xs)) (do
    (Heap/push! (get i xs) fn heap)
    (alter! i (+ i 1))))
  heap)))

(let Matrix->String (comp (map (Vector->String ' ')) (Vector->String '\n')))

(let Date/iso->Vector
  (lambda ts
    [
      (String->Integer (slice 0 4 ts))
      (String->Integer (slice 5 7 ts))
      (String->Integer (slice 8 10 ts))
    ]))

(let Date/iso-z->Vector
  (lambda ts
    [
      (String->Integer (slice 0 4 ts))
      (String->Integer (slice 5 7 ts))
      (String->Integer (slice 8 10 ts))
      (String->Integer (slice 11 13 ts))
      (String->Integer (slice 14 16 ts))
      (String->Integer (slice 17 19 ts))
    ]))

(let Date/iso->Vector
  (lambda ts
    (if (= (length ts) 10)
        [
          (String->Integer (slice 0 4 ts))
          (String->Integer (slice 5 7 ts))
          (String->Integer (slice 8 10 ts))
          0 0 0
        ]
        [
          (String->Integer (slice 0 4 ts))
          (String->Integer (slice 5 7 ts))
          (String->Integer (slice 8 10 ts))
          (String->Integer (slice 11 13 ts))
          (String->Integer (slice 14 16 ts))
          (String->Integer (slice 17 19 ts))
        ])))

(let Date/year (lambda dt (get 0 dt)))
(let Date/month (lambda dt (get 1 dt)))
(let Date/day (lambda dt (get 2 dt)))
(let Date/hour (lambda dt (get 3 dt)))

(let Date/ymd (lambda [a b c] [a b c]))
(let Date/ymdh (lambda [a b c d] [a b c d]))

(let Timestamp/iso-z->seconds
  (lambda (ts)
    (+ (* (- (String->Integer (slice 8 10 ts)) 1) 86400)
        (* (String->Integer (slice 11 13 ts)) 3600)
        (* (String->Integer (slice 14 16 ts)) 60)
        (String->Integer (slice 17 19 ts)))))


(let Integer->Digits-base (lambda num base
    (if (= num 0) [ 0 ] (do
        (mut n num)
        (let digits [])
        (while (> n 0) (do
            (Vector/push! (% n base) digits)
            (alter! n (/ n base))))
        (reverse digits)))))

  
(let Vector/reverse! (lambda xs (do
  (let len (length xs))
  (let half (/ len 2))
  (mut i 0)
  (while (< i half) (do
    (Vector/swap! i (- len i 1) xs)
    (alter! i (+ i 1))))
  xs)))

(let Vector/concat (lambda xs (do
    (let out [])
    (let len-xs (length xs))
    (mut i 0)
    (while (< i len-xs) (do
      (let current (get i xs))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! (length out) (get j current) out)
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    out)))
(let Vector/concat! (lambda os xs (do
    (let len-os (length os))
    (mut i 0)
    (while (< i len-os) (do
      (let current (get i os))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! (length xs) (get j current) xs)
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    xs)))
