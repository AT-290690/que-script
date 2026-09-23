(let Vector/append/raw! (lambda xs x (do (push! xs x) xs)))

(let Vector/cons/raw
  (lambda a b
    (do
      (let lena (length a))
      (let lenb (length b))
      (let out [])

      (mut i 0)
      (while (< i lena) (do
        (set! out (length out) (get a i))
        (alter! i (+ i 1))))

      (mut j 0)
      (while (< j lenb) (do
        (set! out (length out) (get b j))
        (alter! j (+ j 1))))

      out)))

(let Vector/cons/raw! (lambda a b (if (and (empty? a) (empty? b)) a (do
  (mut i 0)
  (let lenb (length b))
  (while (< i lenb) (do
    (set! a (length a) (get b i))
    (alter! i (+ i 1))))
  a))))

(let Int/euclidean-mod (lambda a b (% (+ (% a b) b) b)))

(let Vector/to-string/raw (lambda xs delim (do
    (let out [])
    (let len-xs (length xs))
    (mut i 0)
    (while (< i len-xs) (do
      (if (> i 0)
          (set! out (length out) delim))
      (let current (get xs i))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! out (length out) (get current j))
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    out)))

(let String/to-vector/raw (lambda str ch (do
    (let out [[]])
    (let len (length str))
    (mut i 0)
    (while (< i len) (do
      (let current (get str i))
      (if (=# current ch)
          (set! out (length out) [])
          (do
            (let prev (get out (- (length out) 1)))
            (set! prev (length prev) current)))
      (alter! i (+ i 1))))
    out)))

(let String/chars->unsigned-dec (lambda xs (do
  (let parts (Chars->Digits/dec xs))
  (let pow (expt (length (get parts 1)) 10))
  (/. (Int->Dec (+
    (* (Digits->Integer (get parts 0)) pow)
    (Digits->Integer (get parts 1)))) (Int->Dec pow)))))

(let Matrix/in-bounds/raw? (lambda matrix y x (and (in-bounds? matrix y) (in-bounds? (get matrix y) x))))

(let Matrix/set/raw! (lambda matrix y x value (do (set! (get matrix y) x value) 0)))

(let Dec/scaling 1000000.0)

(let Que/offset-left (lambda q (* (- (length (get q 0)) 1) -1)))

(let Que/offset-right (lambda q (length (get q 1))))

(let Que/empty/raw! (lambda q (do
    (set! q 0 [(get q 0 0)])
    (set! q 1 [])
    q)))

(let Que/add-to-left! (lambda q item (do (let c (get q 0)) (set! c (length c) item))))

(let Que/add-to-right! (lambda q item (do (let c (get q 1)) (set! c (length c) item))))

(let Que/remove-from-left! (lambda q (do
  (let len (Que/length q))
  (if (> len 0)
     (cond
        (= len 1) (Que/empty/raw! q)
        (> (length (get q 0)) 0) (do (pop! (get q 0)) q)
        q) q))))

(let Que/remove-from-right! (lambda q (do
    (let len (Que/length q))
    (if (> len 0)
     (cond
        (= len 1) (Que/empty/raw! q)
        (> (length (get q 1)) 0) (do (pop! (get q 1)) q)
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
        (Que/add-to-right! q (get initial right))
        (alter! right (+ right 1))))
      (mut left (- half 1))
      (while (>= left 0) (do
        (Que/add-to-left! q (get initial left))
        (alter! left (- left 1))))
    q))))

(let Que/append/raw! (lambda q item (do (Que/add-to-right! q item) q)))

(let Que/prepend/raw! (lambda q item (do (Que/add-to-left! q item) q)))

(let Que/head! (lambda q (do
    (if (= (Que/offset-right q) 0) (Que/balance! q) q)
    (Que/remove-from-right! q)
    q)))

(let Que/tail/raw! (lambda q (do
    (if (= (Que/offset-left q) 0) (Que/balance! q) q)
    (Que/remove-from-left! q)
q)))

(let Que/enqueue/raw! (lambda queue item (Que/append/raw! queue item)))

(let Vector/equal/raw? (lambda a b fn? (do
  (if (< (length a) (length b)) false
  (if (> (length a) (length b)) false
    (do
      (mut i 0)
      (mut result true)
      (let len (length a))
      (while (< i len) (do
        (let da (get a i))
        (let db (get b i))
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
      (alter! state (step state (get a i) (get b i) i))
      (alter! i (+ i 1))))
    { state len-a len-b })))

(let Matrix/fill/raw (lambda W H fn
  (cond
    (or (= W 0) (= H 0)) []
    (and (= W 1) (= H 1)) [[(fn 0 0)]] (do
      (let matrix [])
      (mut i 0)
      (while (< i W) (do
          (push! matrix [])
          (mut j 0)
          (while (< j H) (do
            (Matrix/set/raw! matrix i j (fn i j))
            (alter! j (+ j 1))))
          (alter! i (+ i 1))))
      matrix))))

(let BigInt/remove-leading-zeroes
  (lambda digits
    (do
      (mut i 0)
      (while (and (< i (length digits))
                  (zero? (get digits i))) (do
        (++ i)))
      (if (= i (length digits))
          [0]
          (slice i (length digits) digits)))))

(let StructBigDec/rescale-signed (lambda signed from-precision to-precision
  (if (= from-precision to-precision)
      signed
      (SignedBigInt/normalize {
        (SignedBigInt/negative? signed)
        (BigInt/mul
          (SignedBigInt/digits signed)
          (StructBigDec/scale (- to-precision from-precision)))
      }))))

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

(let Vector/to-tuple/raw (lambda xs fn1 fn2 { (fn1 xs) (fn2 xs) }))

(let Table/hash
 (lambda table key
   (do
     (let cap (length table))
     (if (= cap 0)
         0
         (do
           (mut i 0)
           (mut hash 0)
           (let len (length key))
           (while (< i len) (do
             (alter! hash (Int/euclidean-mod (+ (* hash 131) (as (get key i) Int)) cap))
             (alter! i (+ i 1))))
           hash)))))

(let Table/create (lambda capacity (buckets (max 4 capacity))))

(let Table/max-capacity (lambda a b (Table/create (max (length a) (length b)))))

(let Table/find-index (lambda bucket key (do
  (mut i 0)
  (mut found -1)
  (let len (length bucket))
  (while (and (= found -1) (< i len)) (do
    (if (Set/key-equal? (fst (get bucket i)) key)
        (alter! found i)
        nil)
    (alter! i (+ i 1))))
  found)))

(let Table/for-each (lambda table fn (do
  (mut i 0)
  (let table-len (length table))
  (while (< i table-len) (do
    (let bucket (get table i))
    (mut j 0)
    (let bucket-len (length bucket))
    (while (< j bucket-len) (do
      (fn (get bucket j))
      (alter! j (+ j 1))))
    (alter! i (+ i 1)))))))

(let Table/set/unchecked! (lambda table key value (do
  (let idx (Table/hash table key))
  (let bucket (get table idx))
  (set! bucket (length bucket) { key value })
  table)))

(let Table/resize/raw! (lambda table new-capacity (do
  (let target (max 4 new-capacity))
  (if (= target (length table))
      table
      (do
        (let entries [])
        (Table/for-each table (lambda entry (set! entries (length entries) entry)))
        (Heap/empty! table)
        (mut i 0)
        (while (< i target) (do
          (set! table (length table) [])
          (alter! i (+ i 1))))
        (mut j 0)
        (let entries-len (length entries))
        (while (< j entries-len) (do
          (let entry (get entries j))
          (Table/set/unchecked! table (fst entry) (snd entry))
          (alter! j (+ j 1))))
        table)))))

(let Table/compact/raw! (lambda table (do
  (let used (Table/count-entries table))
  (let target (max 32 (* used 2)))
  (Table/resize/raw! table target))))

(let Table/has/raw? (lambda table key
  (if (= (length table) 0)
      false
      (do
        (let idx (Table/hash table key))
        (let bucket (get table idx))
        (>= (Table/find-index bucket key) 0)))))

(let Table/set/raw! (lambda table key value (do
  (if (= (length table) 0) (do (Table/resize/raw! table 32) nil) nil)
  (let idx (Table/hash table key))
  (let bucket (get table idx))
  (let index (Table/find-index bucket key))
  (if (= index -1)
      (do
        (set! bucket (length bucket) { key value })
        (if (> (length bucket) 8)
            (do (Table/resize/raw! table (* (length table) 2)) nil)
            nil))
      (set! bucket index { key value }))
  table)))

(let Table/remove/raw! (lambda table key (do
  (if (= (length table) 0)
      table
      (do
        (let idx (Table/hash table key))
        (let bucket (get table idx))
        (let index (Table/find-index bucket key))
        (if (>= index 0)
            (do
              (set! bucket index (get bucket (- (length bucket) 1)))
              (pop! bucket))
            nil)
        (if (and (> (length table) 32) (= (length bucket) 0))
            (do
              (let used (Table/count-entries table))
              (if (< (* used 4) (length table))
                  (do (Table/resize/raw! table (max 32 (/ (length table) 2))) nil)
                  nil))
            nil)
        table)))))

(let Table/get/raw (lambda table key
  (if (= (length table) 0)
      []
      (do
        (let idx (Table/hash table key))
        (let bucket (get table idx))
        (let index (Table/find-index bucket key))
        (if (>= index 0) [ (get bucket index) ] [])))))

(let Table/drop/raw! (lambda table keys (do
  (mut i 0)
  (let len (length keys))
  (while (< i len) (do
    (Table/remove/raw! table (get keys i))
    (alter! i (+ i 1)))))))

(let Table/keep/raw (lambda table keys (do
  (let out (Table/create (max 32 (length keys))))
  (mut i 0)
  (let len (length keys))
  (while (< i len) (do
    (let key (get keys i))
    (let hit (Table/get/raw table key))
    (if (> (length hit) 0)
        (do (Table/set/raw! out key (fst (get hit 0))) nil)
        nil)
    (alter! i (+ i 1))))
  out)))

(let Table/merge/raw! (lambda a b (do
  (let entries (Table/entries b))
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (let entry (get entries i))
    (Table/set/raw! a (fst entry) (snd entry))
    (alter! i (+ i 1))))
  a)))

(let Table/merge/raw (lambda a b (do
  (let out (Table/max-capacity a b))
  (Table/merge/raw! out a)
  (Table/merge/raw! out b)
  out)))

(let Table/omit/raw (lambda table keys (do
  (let out (Table/merge/raw (Table/create 32) table))
  (Table/drop/raw! out keys)
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
        (while (=# (get right (- len 1)) '0')
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
        (if (not (=# (get data (+ offset i))
                     (get target i)))
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
    (let negative? (and (> len 0) (< (get digits-with-sign 0) 0)))
    (mut num 0)
    (mut i 0)
    (while (< i len) (do
      (let digit (get digits-with-sign i))
      (alter! num (+ (* num 10) (if negative? (abs digit) digit)))
      (alter! i (+ i 1))))
    (if negative? (- 0 num) num))))

(let Chars->Integer (lambda chars (do
    (let len (length chars))
    (mut sign 1)
    (mut i 0)
    (if (and (> len 0) (=# (get chars 0) '-'))
        (do
          (alter! sign -1)
          (alter! i 1)))
    (mut num 0)
    (while (< i len) (do
      (alter! num (+ (* num 10) (Char->Digit (get chars i))))
      (alter! i (+ i 1))))
    (* num sign))))

(let BigInt/equal? (lambda a b (do
  (if (< (length a) (length b)) false
  (if (> (length a) (length b)) false
    (do
      (mut i 0)
      (mut result true)
      (let len (length a))
      (while (< i len) (do
        (let da (get a i))
        (let db (get b i))
        (if (not (= da db)) (do
          (alter! result false)
          (alter! i len)))
        (alter! i (+ i 1))))
      (if result true false)))))))

(let Vector/length (lambda xs (length xs)))

(let Vector/pop! (lambda xs (pop! xs)))

(let Vector/push! (lambda xs x (do (set! xs (length xs) x) xs)))

(let Heap/empty! (lambda xs (if (empty? xs) xs (do
     (while (not (empty? xs)) (pop! xs))
     xs))))

(let Vector/overwrite! (lambda (xs ys) (Heap/empty! xs) (loop i (< i (length ys)) (set! xs i (get ys i)))))

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
        (let ca (get a i))
        (let cb (get b i))
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
        (let ca (get a i))
        (let cb (get b i))
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
      (set! out (length out) (Char->Digit (get digits i)))
      (alter! i (+ i 1))))
    out)))

(let Chars->Digits/dec (lambda xs (do
    (let out [[]])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (let ch (get xs i))
      (if (=# ch '.')
          (push! out [])
          (push! (at out -1) (Char->Digit ch)))
      (alter! i (+ i 1))))
    out)))


(let Integer->Chars-base (lambda num base
    (if (= num 0) "0" (do
        (let neg? (< num 0))
        (mut n (if neg? (* num -1) num))
        (let str [])
        (while (> n 0) (do
            (let x (% n base))
            (push! str (+# (Int->Char x) (char 48)))
            (alter! n (/ n base))))
        (if neg? (push! str '-'))
        (mut left 0)
        (mut right (- (length str) 1))
        (while (< left right) (do
            (let ch (get str left))
            (set! str left (get str right))
            (set! str right ch)
            (alter! left (+ left 1))
            (alter! right (- right 1))))
        str))))

(let Digit->Char (lambda digit (if (< digit 0) '0' (+# (as digit Char) '0'))))

(let Digits->Chars (lambda digits (do
    (let out [])
    (let len (length digits))
    (mut i 0)
    (while (< i len) (do
      (set! out (length out) (Digit->Char (get digits i)))
      (alter! i (+ i 1))))
    out)))



(let String->Dec (lambda xs
  (if (=# (get xs 0) '-') (*. (String/chars->unsigned-dec (slice 1 (length xs) xs)) -1.0) (String/chars->unsigned-dec xs))))

(let Int->Alphabet
  (lambda x offset (Int->Char (+ x (Char->Int offset)))))

(let Heap/peek! (lambda heap (get heap 0)))

(let Set->Vector (lambda xs (filter not-empty? (flat xs))))

(let Integer->Chars (lambda x (Integer->Chars-base x 10)))

(let Que/new (lambda def [[ def ] []]))

(let Que/length (lambda q (+ (length (get q 0)) (length (get q 1)) -1)))

(let Que/empty? (lambda q (= (Que/length q) 0)))

(let Que/get (lambda q offset (do
  (let offset-index (+ offset (Que/offset-left q)))
  (let index (if (< offset-index 0) (* offset-index -1) offset-index))
  (if (>= offset-index 0)
       (get (get q 1) index)
       (get (get q 0) index)))))

(let Vector->Que (lambda initial (do
 (let q (Que/new))
 (let half (/ (length initial) 2))
 (mut left (- half 1))
 (while (>= left 0) (do
    (Que/add-to-left! q (get initial left))
    (alter! left (- left 1))))
 (mut right half)
 (let len (length initial))
 (while (< right len) (do
   (Que/add-to-right! q (get initial right))
   (alter! right (+ right 1))))
    q)))

(let Que->Vector (lambda q (if (Que/empty? q) [(get q 0 0)] (do
  (let out [])
  (mut index 0)
  (let len (Que/length q))
  (while (< index len) (do
      (set! out (length out) (Que/get q index))
      (alter! index (+ index 1))))
    out))))

(let Que/peek (lambda q (Que/get q 0)))

(let Que/last (lambda q (Que/get q (- (Que/length q) 1))))

(let Que/not-empty? (lambda q (not (Que/empty? q))))

(let Integer->Bits (lambda num
    (if (= num 0) [ 0 ] (do
        (mut n num)
        (let out [])
        (while (> n 0) (do
            (push! out (% n 2))
            (alter! n (/ n 2))))
        (reverse out)))))

(let Bits->Integer (lambda xs (do
  (let len (length xs))
  (mut index 0)
  (mut out 0)
  (while (< index len) (do
    (alter! out (+ out (* (at xs index) (expt (- len index 1) 2))))
    (alter! index (+ index 1))))
  out)))

(let BigInt/range (lambda start end (do
     (let out [ (BigInt/new (Integer->Chars start)) ])
     (mut i (+ start 1))
     (while (<= i end) (do
        (set! out (length out) (BigInt/new (Integer->Chars i)))
        (alter! i (+ i 1))))
   out)))

(let BigInt/add (lambda a1 b1 (do
  (let a (reverse a1))
  (let b (reverse b1))
  (let max-length (max (length a) (length b)))
  (let result [])
  (mut carry 0)
  (loop/range/exclusive i 0 max-length
    (let digit-A (if (< i (length a)) (get a i) 0))
    (let digit-B (if (< i (length b)) (get b i) 0))
    (let sm (+ digit-A digit-B carry))
    (push! result (% sm 10))
    (alter! carry (/ sm 10)))
  ; Handle remaining carry
  (while (> carry 0) (do
    (push! result (% carry 10))
    (alter! carry (/ carry 10))))
  (reverse result))))

(let BigInt/sub
  (lambda a1 b1
    (do
      (let a (reverse a1))
      (let b (reverse b1))
      (let result [])
      (mut borrow 0)
      (mut j 0)
      (let max-length (max (length a) (length b)))

      (while (< j max-length) (do
        (let digit-a (if (< j (length a)) (get a j) 0))
        (let digit-b (if (< j (length b)) (get b j) 0))
        (let diff (- (- digit-a digit-b) borrow))

        (if (< diff 0)
            (do
              (push! result (+ diff 10))
              (alter! borrow 1))
            (do
              (push! result diff)
              (alter! borrow 0)))

        (alter! j (+ j 1))))

      (mut k (- (length result) 1))
      (while (and (> k 0) (= (get result k) 0)) (do
        (pop! result)
        (alter! k (- k 1))))

      (reverse result))))

(let BigInt/mul (lambda a1 b1 (do
  (let a (reverse a1))
  (let b (reverse b1))
  (let result [])
  ; Initialize result array with zeros
  (loop/range/exclusive _ 0 (+ (length a) (length b)) (push! result 0))
  (loop/range/exclusive i 0 (length a)
    (mut carry 0)
    (let digit-a (get a i))
    (loop/range/exclusive j 0 (length b)
      (let digit-B (get b j))
      (let idx (+ i j))
      (let prod (+ (* digit-a digit-B) (get result idx) carry))
      (set! result idx (% prod 10))
      (alter! carry (/ prod 10)))
    ; Handle carry for this digit-a
    (mut k (+ i (length b)))
    (while (> carry 0) (do
      (if (not (< k (length result))) (push! result 0))
      (let sm (+ (get result k) carry))
      (set! result k (% sm 10))
      (alter! carry (/ sm 10))
      (alter! k (+ k 1)))))
  ; Remove trailing zeros (from the most significant end), but keep at least one digit
  (mut ii (- (length result) 1))
  (while (and (> ii 0) (= (get result ii) 0) (> (length result) 1)) (do
    (pop! result)
    (alter! ii (- ii 1))))
  (reverse result))))

(let BigInt/lte? (lambda a b (do
  (if (< (length a) (length b)) true
  (if (> (length a) (length b)) false
    ; Equal length, compare digit by digit
    (do
      (mut i 0)
      (mut result true) ; assume a <= b
      (while (< i (length a)) (do
        (let da (get a i))
        (let db (get b i))
        (if (< da db) (do
          (alter! result true)
          (alter! i (length a))))
        (if (> da db) (do
          (alter! result false)
          (alter! i (length a))))
        (alter! i (+ i 1))))
      result))))))

(let BigInt/gte? (lambda a b (do
  (if (> (length a) (length b)) true
  (if (< (length a) (length b)) false
    ; Equal length, compare digit by digit
    (do
      (mut i 0)
      (mut result true) ; assume a >= b
      (while (< i (length a)) (do
        (let da (get a i))
        (let db (get b i))
        (if (> da db) (do
          (alter! result true)
          (alter! i (length a))))
        (if (< da db) (do
          (alter! result false)
          (alter! i (length a))))
        (alter! i (+ i 1))))
      result))))))

(let BigInt/lt? (lambda a b (do
  (if (< (length a) (length b)) true
  (if (> (length a) (length b)) false
    ; Equal length, check for strict less (not equal)
    (do
      (mut i 0)
      (mut found-less false) ; true if a < b at some digit
      (while (< i (length a)) (do
        (let da (get a i))
        (let db (get b i))
        (if (< da db) (do
          (alter! found-less true)
          (alter! i (length a))))
        (if (> da db) (do
          (alter! i (length a)))) ; stop on a > b, keep found-less false
        (alter! i (+ i 1))))
      found-less))))))

(let BigInt/gt? (lambda a b (do
  (if (> (length a) (length b)) true
  (if (< (length a) (length b)) false
    ; Equal length, check for strict greater (not equal)
    (do
      (mut i 0)
      (mut found-greater false) ; true if a > b at some digit
      (while (< i (length a)) (do
        (let da (get a i))
        (let db (get b i))
        (if (> da db) (do
          (alter! found-greater true)
          (alter! i (length a))))
        (if (< da db) (do
          (alter! i (length a)))) ; stop on a < b, keep found-greater false
        (alter! i (+ i 1))))
      found-greater))))))


(let BigInt/div
  (lambda dividend divisor
    (do
      (let result [])
      (&mut current [])
      (let len (length dividend))
      (mut i 0)

      (while (< i len) (do
        (let digit (get dividend i))

        ;; current = trim(cons(current, [digit]))
        (let cur0 (&get current))
        (let cur1 (cons cur0 [digit]))
        (let cur2 (BigInt/remove-leading-zeroes cur1))
        (&alter! current cur2)

        ;; binary search q in [0,9]
        (mut low 0)
        (mut high 9)
        (mut q 0)

        (while (<= low high) (do
          (let mid (/ (+ low high) 2))
          (let prod (BigInt/mul divisor [mid]))
          (let cur (&get current))

          (if (BigInt/lte? prod cur)
              (do
                (alter! q mid)
                (alter! low (+ mid 1)))
              (alter! high (- mid 1)))))

        (push! result q)

        ;; current = current - divisor*q
        (let prod2 (BigInt/mul divisor [q]))
        (let cur3 (&get current))
        (let cur4 (BigInt/sub cur3 prod2))
        (&alter! current cur4)

        (alter! i (+ i 1))))

      (let out (BigInt/remove-leading-zeroes result))
      (if (empty? out) [0] out))))

(let BigInt/mod
  (lambda a b
    (do
      (let q (BigInt/div a b))
      (let p (BigInt/mul b q))
      (let r (BigInt/sub a p))
      r)))

(let BigInt/square (lambda x (BigInt/mul x x)))

(let BigInt/div/floor (lambda a b (BigInt/div a b)))

(let BigInt/div/ceal (lambda a b (BigInt/div
    (BigInt/sub (BigInt/add a b) [ 1 ]) b)))

(let BigInt/sum (lambda xs (do
    (&mut total [ 0 ])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (&alter! total (BigInt/add (&get total) (get xs i)))
      (alter! i (+ i 1))))
    (&get total))))

(let BigInt/product (lambda xs (do
    (&mut total [ 1 ])
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (&alter! total (BigInt/mul (&get total) (get xs i)))
      (alter! i (+ i 1))))
    (&get total))))

(let BigInt/new (lambda str (Chars->Digits str)))

(let BigInt/pow (lambda a b (if (= b 0) [ 1 ] (do
    (&mut out a)
    (mut i 0)
    (while (< i (- b 1)) (do (&alter! out (BigInt/mul (&get out) a)) (alter! i (+ i 1))))
    (&get out)))))

(let BigInt/expt (lambda a b (if (and (= (length b) 1) (= (get b 0) 0)) [ 1 ] (do
    (&mut out a)
    (&mut exp (BigInt/sub b [ 1 ]))
    (while (not (and (= (length (&get exp)) 1) (= (get (&get exp) 0) 0))) (do
      (&alter! out (BigInt/mul (&get out) a))
      (&alter! exp (BigInt/sub (&get exp) [ 1 ]))))
    (&get out)))))

(let SignedBigInt/zero { false [0] })

(let SignedBigInt/one { false [1] })

(let SignedBigInt/negative? (lambda n (if (fst n) true false)))

(let SignedBigInt/digits snd)

(let SignedBigInt/abs snd)

(let SignedBigInt/zero? (lambda n (BigInt/equal? (snd n) [0])))

(let SignedBigInt/normalize (lambda { neg digits } (do
  (let mag (BigInt/remove-leading-zeroes digits))
  (if (or (empty? mag) (BigInt/equal? mag [0]))
      SignedBigInt/zero
      { neg mag }))))

(let SignedBigInt/new (lambda str
  (if (and (> (length str) 0) (=# (get str 0) '-'))
      (SignedBigInt/normalize { true (BigInt/new (slice 1 (length str) str)) })
      (SignedBigInt/normalize { false (BigInt/new str) }))))

(let SignedBigInt/negate (lambda { neg digits }
  (if (BigInt/equal? digits [0])
      SignedBigInt/zero
      { (not neg) digits })))

(let SignedBigInt/equal? (lambda a b
  (and (=? (fst a) (fst b)) (BigInt/equal? (snd a) (snd b)))))

(let SignedBigInt/lt? (lambda a b
  (if (fst a)
      (if (fst b)
          (BigInt/gt? (snd a) (snd b))
          true)
      (if (fst b)
          false
          (BigInt/lt? (snd a) (snd b))))))

(let SignedBigInt/lte? (lambda a b
  (or (SignedBigInt/equal? a b) (SignedBigInt/lt? a b))))

(let SignedBigInt/gt? (lambda a b
  (not (SignedBigInt/lte? a b))))

(let SignedBigInt/gte? (lambda a b
  (not (SignedBigInt/lt? a b))))

(let SignedBigInt/add (lambda a b
  (if (=? (fst a) (fst b))
      (SignedBigInt/normalize { (fst a) (BigInt/add (snd a) (snd b)) })
      (if (BigInt/gt? (snd a) (snd b))
          (SignedBigInt/normalize { (fst a) (BigInt/sub (snd a) (snd b)) })
          (if (BigInt/lt? (snd a) (snd b))
              (SignedBigInt/normalize { (fst b) (BigInt/sub (snd b) (snd a)) })
              SignedBigInt/zero)))))

(let SignedBigInt/sub (lambda a b
  (SignedBigInt/add a (SignedBigInt/negate b))))

(let SignedBigInt/mul (lambda a b
  (SignedBigInt/normalize { (not (=? (fst a) (fst b))) (BigInt/mul (snd a) (snd b)) })))

(let SignedBigInt/square (lambda x (SignedBigInt/mul x x)))

(let SignedBigInt/pow (lambda a b
  (if (= b 0)
      SignedBigInt/one
      (SignedBigInt/normalize {
        (and (fst a) (= (% b 2) 1))
        (BigInt/pow (snd a) b)
      }))))

(let SignedBigInt/expt (lambda a b
  (if (BigInt/equal? b [0])
      SignedBigInt/one
      (SignedBigInt/normalize {
        (and (fst a) (BigInt/equal? (BigInt/mod b [2]) [1]))
        (BigInt/expt (snd a) b)
      }))))

(let SignedBigInt/div (lambda a b (do
  (let q (BigInt/div (snd a) (snd b)))
  (SignedBigInt/normalize { (not (=? (fst a) (fst b))) q }))))

(let SignedBigInt/mod (lambda a b
  (SignedBigInt/sub a (SignedBigInt/mul b (SignedBigInt/div a b)))))

(let SignedBigInt/div/floor (lambda a b (do
  (let q (SignedBigInt/div a b))
  (let r (SignedBigInt/mod a b))
  (if (and (not (SignedBigInt/zero? r)) (not (=? (fst a) (fst b))))
      (SignedBigInt/sub q SignedBigInt/one)
      q))))

(let SignedBigInt/div/ceil (lambda a b (do
  (let q (SignedBigInt/div a b))
  (let r (SignedBigInt/mod a b))
  (if (and (not (SignedBigInt/zero? r)) (=? (fst a) (fst b)))
      (SignedBigInt/add q SignedBigInt/one)
      q))))

(let SignedBigInt->String (lambda n
  (if (fst n)
      (cons "-" (Digits->Chars (snd n)))
      (Digits->Chars (snd n)))))

(let SignedBigInt/range (lambda start end
  (if (SignedBigInt/gt? start end)
      []
      (do
        (let out [start])
        (&mut current start)
        (while (SignedBigInt/lt? (&get current) end) (do
          (&alter! current (SignedBigInt/add (&get current) SignedBigInt/one))
          (set! out (length out) (&get current))))
        out))))

(let SignedBigInt/sum (lambda xs (do
    (&mut total SignedBigInt/zero)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (&alter! total (SignedBigInt/add (&get total) (get xs i)))
      (alter! i (+ i 1))))
    (&get total))))

(let SignedBigInt/product (lambda xs (do
    (&mut total SignedBigInt/one)
    (let len (length xs))
    (mut i 0)
    (while (< i len) (do
      (&alter! total (SignedBigInt/mul (&get total) (get xs i)))
      (alter! i (+ i 1))))
    (&get total))))

(let StructBigDec/zero { 0 SignedBigInt/zero })

(let StructBigDec/one { 0 SignedBigInt/one })

(let StructBigDec/precision fst)

(let StructBigDec/signed snd)

(let StructBigDec/negative? (lambda n (SignedBigInt/negative? (StructBigDec/signed n))))

(let StructBigDec/scale (lambda precision (BigInt/pow [1 0] precision)))

(let StructBigDec/from-scaled (lambda precision scaled
  { precision (SignedBigInt/normalize scaled) }))

(let StructBigDec/new (lambda precision neg whole fraction
  (do
    (let scale (StructBigDec/scale precision))
    (let whole-scaled (BigInt/mul whole scale))
    (let mag (BigInt/add whole-scaled fraction))
    (StructBigDec/from-scaled precision { neg mag }))))

(let StructBigDec/new-digits (lambda neg whole fraction
  (StructBigDec/new (length fraction) neg whole fraction)))

(let StructBigDec/pi (StructBigDec/new-digits false [3] [1 4 1 5 9 2 6 5 3 5 8 9 7 9 3]))

(let StructBigDec/e (StructBigDec/new-digits false [2] [7 1 8 2 8 1 8 2 8 4 5 9 0 4 5]))

(let StructBigDec/from-int (lambda n
  { 0 (SignedBigInt/new (Integer->Chars n)) }))

(let StructBigDec/from-digits (lambda precision neg scaled-digits
  (StructBigDec/from-scaled precision { neg scaled-digits })))

(let StructBigDec/scaled (lambda n (StructBigDec/signed n)))

(let StructBigDec/whole (lambda n
  (BigInt/div
    (SignedBigInt/digits (StructBigDec/signed n))
    (StructBigDec/scale (StructBigDec/precision n)))))

(let StructBigDec/fraction (lambda n
  (BigInt/mod
    (SignedBigInt/digits (StructBigDec/signed n))
    (StructBigDec/scale (StructBigDec/precision n)))))

(let StructBigDec/add (lambda a b
  (do
    (let precision (max (StructBigDec/precision a) (StructBigDec/precision b)))
    (StructBigDec/from-scaled
      precision
      (SignedBigInt/add
        (StructBigDec/align-left a b)
        (StructBigDec/align-right a b))))))

(let StructBigDec/sub (lambda a b
  (do
    (let precision (max (StructBigDec/precision a) (StructBigDec/precision b)))
    (StructBigDec/from-scaled
      precision
      (SignedBigInt/sub
        (StructBigDec/align-left a b)
        (StructBigDec/align-right a b))))))

(let StructBigDec/mul (lambda a b
  (StructBigDec/from-scaled
    (+ (StructBigDec/precision a) (StructBigDec/precision b))
    (SignedBigInt/mul (StructBigDec/signed a) (StructBigDec/signed b)))))

(let StructBigDec/div/precision (lambda precision a b
  (do
    (let numerator
      (SignedBigInt/mul
        (StructBigDec/signed a)
        { false (StructBigDec/scale (+ precision (StructBigDec/precision b))) }))
    (let denominator
      { false
        (BigInt/mul
          (SignedBigInt/digits (StructBigDec/signed b))
          (StructBigDec/scale (StructBigDec/precision a))) })
    (StructBigDec/from-scaled precision (SignedBigInt/div numerator denominator)))))

(let StructBigDec/div (lambda a b
  (StructBigDec/div/precision
    (max (StructBigDec/precision a) (StructBigDec/precision b))
    a
    b)))

(let StructBigDec/equal? (lambda a b
  (SignedBigInt/equal?
    (StructBigDec/align-left a b)
    (StructBigDec/align-right a b))))

(let StructBigDec/lt? (lambda a b
  (SignedBigInt/lt?
    (StructBigDec/align-left a b)
    (StructBigDec/align-right a b))))

(let StructBigDec/lte? (lambda a b
  (SignedBigInt/lte?
    (StructBigDec/align-left a b)
    (StructBigDec/align-right a b))))

(let StructBigDec/gt? (lambda a b
  (SignedBigInt/gt?
    (StructBigDec/align-left a b)
    (StructBigDec/align-right a b))))

(let StructBigDec/gte? (lambda a b
  (SignedBigInt/gte?
    (StructBigDec/align-left a b)
    (StructBigDec/align-right a b))))

(let StructBigDec/negate (lambda n
  (StructBigDec/from-scaled
    (StructBigDec/precision n)
    (SignedBigInt/negate (StructBigDec/signed n)))))

(let StructBigDec/abs (lambda n
  (StructBigDec/from-scaled
    (StructBigDec/precision n)
    { false (SignedBigInt/digits (StructBigDec/signed n)) })))

(let StructBigDec->String (lambda n
  (do
    (let precision (StructBigDec/precision n))
    (let whole (Digits->Chars (StructBigDec/whole n)))
    (let fraction (String/prepend-zeroes
      (Digits->Chars (StructBigDec/fraction n))
      precision))
    (let sign (if (StructBigDec/negative? n) "-" ""))
    (if (= precision 0)
        (cons sign whole)
        (cons sign whole "." fraction)))))

(let Integer->Digits (lambda num (Integer->Digits-base num 10)))

(let Tuple/swap (lambda { a b } { b a }))

(let Tuple/int/add (lambda { a b } (+ a b)))

(let Tuple/int/sub (lambda { a b } (- a b)))

(let Tuple/int/mul (lambda { a b } (* a b)))

(let Tuple/int/div (lambda { a b } (* a b)))

(let Set/hash (lambda table key (do
  (let cap (length table))
  (if (= cap 0) 0 (do
    (mut i 0)
    (mut hash 0)
    (let len (length key))
    (while (< i len) (do
      (alter! hash (% (+ (% (+ (* hash 131) (as (get key i) Int)) cap) cap) cap))
      (alter! i (+ i 1))))
    hash)))))

(let Set/key-equal? (lambda a b (do
  (let len (length a))
  (if (not (= len (length b))) false (do
    (mut i 0)
    (mut matches true)
    (while (and matches (< i len)) (do
      (if (not (=# (get a i) (get b i))) (alter! matches false) nil)
      (alter! i (+ i 1))))
    matches)))))

(let Set/find-index (lambda bucket key (do
  (mut i 0)
  (mut found -1)
  (let len (length bucket))
  (while (and (= found -1) (< i len)) (do
    (if (Set/key-equal? (get bucket i) key) (alter! found i) nil)
    (alter! i (+ i 1))))
  found)))

(let Set/count (lambda table (do
  (mut total 0)
  (mut i 0)
  (while (< i (length table)) (do
    (alter! total (+ total (length (get table i))))
    (alter! i (+ i 1))))
  total)))

(let Set/for-each (lambda table fn (do
  (mut i 0)
  (let table-len (length table))
  (while (< i table-len) (do
    (let bucket (get table i))
    (mut j 0)
    (let bucket-len (length bucket))
    (while (< j bucket-len) (do
      (fn (get bucket j))
      (alter! j (+ j 1))))
    (alter! i (+ i 1)))))))

(let Set/add/raw! (lambda table key (do
  (let idx (Set/hash table key))
  (let bucket (get table idx))
  (set! bucket (length bucket) key)
  table)))

(let Set/new (lambda (buckets 32)))
(let Set/new/capacity (lambda n (buckets (max 4 n))))
(let Set/max-capacity (lambda a b (Set/new/capacity (max (length a) (length b)))))

(let Set/resize! (lambda table new-capacity (do
  (let target (max 4 new-capacity))
  (if (not (= target (length table))) (do
    (let entries [])
    (Set/for-each table (lambda key (set! entries (length entries) key)))
    (Heap/empty! table)
    (mut i 0)
    (while (< i target) (do (set! table (length table) []) (alter! i (+ i 1))))
    (mut j 0)
    (while (< j (length entries)) (do
      (Set/add/raw! table (get entries j))
      (alter! j (+ j 1))))) nil)
  nil)))

(let Set/compact! (lambda table (do
  (Set/resize! table (max 32 (* (Set/count table) 2)))
  nil)))

(let Set/has? (lambda item table
  (if (= (length table) 0) false (do
    (let idx (Set/hash table item))
    (>= (Set/find-index (get table idx) item) 0)))))

(let Set/add! (lambda table item (do
  (if (= (length table) 0) (Set/resize! table 32) nil)
  (let idx (Set/hash table item))
  (let bucket (get table idx))
  (if (= (Set/find-index bucket item) -1) (do
    (set! bucket (length bucket) item)
    (if (> (length bucket) 8) (Set/resize! table (* (length table) 2)) nil)) nil)
  nil)))

(let Set/remove! (lambda table item (do
  (if (not (= (length table) 0)) (do
    (let idx (Set/hash table item))
    (let bucket (get table idx))
    (let index (Set/find-index bucket item))
    (if (>= index 0) (do
      (set! bucket index (get bucket (- (length bucket) 1)))
      (pop! bucket)) nil)
    (if (and (> (length table) 32) (= (length bucket) 0)
             (< (* (Set/count table) 4) (length table)))
        (Set/resize! table (max 32 (/ (length table) 2))) nil)) nil)
  nil)))

(let Vector->Set (lambda xs (do
  (let out (Set/new/capacity (max 32 (length xs))))
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do
    (Set/add! out (get xs i))
    (alter! i (+ i 1))))
  out)))

(let Set/intersection (lambda a b (do
  (let out (Set/max-capacity a b))
  (let a-count (Set/count a))
  (let b-count (Set/count b))
  (let src (if (< a-count b-count) a b))
  (let trg (if (< a-count b-count) b a))
  (Set/for-each src (lambda key
    (if (and (not-empty? key) (Set/has? key trg))
        (do (Set/add! out key) nil)
        nil)))
  out)))

(let Set/difference (lambda a b (do
  (let out (Set/max-capacity a b))
  (Set/for-each a (lambda key
    (if (and (not-empty? key) (not (Set/has? key b)))
        (do (Set/add! out key) nil)
        nil)))
  out)))

(let Set/xor (lambda a b (do
  (let out (Set/max-capacity a b))
  (Set/for-each a (lambda key
    (if (and (not-empty? key) (not (Set/has? key b)))
        (do (Set/add! out key) nil)
        nil)))
  (Set/for-each b (lambda key
    (if (and (not-empty? key) (not (Set/has? key a)))
        (do (Set/add! out key) nil)
        nil)))
  out)))

(let Set/union (lambda a b (do
  (let out (Set/max-capacity a b))
  (Set/for-each a (lambda key
    (if (not-empty? key) (do (Set/add! out key) nil) nil)))
  (Set/for-each b (lambda key
    (if (not-empty? key) (do (Set/add! out key) nil) nil)))
  out)))

(let Table/update! (lambda table key init f (do
  (if (Table/has/raw? table key)
      (Table/set/raw! table key (f (snd (get (Table/get/raw table key) 0))))
      (Table/set/raw! table key init))
  nil)))

(let Table/update-or! (lambda table key missing present (do
  (if (Table/has/raw? table key)
      (Table/set/raw! table key (present (snd (get (Table/get/raw table key) 0))))
      (Table/set/raw! table key (missing)))
  nil)))

(let Table/push-or! (lambda table key value (do
  (if (Table/has/raw? table key)
      (do (push! (snd (get (Table/get/raw table key) 0)) value) nil)
      (do (Table/set/raw! table key [value]) nil))
  nil)))

(let Table/entries (lambda table (do
  (let out [])
  (Table/for-each table (lambda entry (set! out (length out) entry)))
  out)))

(let Table/keys (lambda table (do
  (let entries (Table/entries table))
  (let out [])
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (set! out (length out) (fst (get entries i)))
    (alter! i (+ i 1))))
  out)))

(let Table/values (lambda table (do
  (let entries (Table/entries table))
  (let out [])
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (set! out (length out) (snd (get entries i)))
    (alter! i (+ i 1))))
  out)))

(let Table/count (lambda arr (do
  (let table (Table/create (max 64 (length arr))))
  (mut i 0)
  (let len (length arr))
  (while (< i len) (do
    (let key (get arr i))
    (let hit (Table/get/raw table key))
    (if (= (length hit) 0)
        (Table/set/raw! table key 1)
        (Table/set/raw! table key (+ (snd (get hit 0)) 1)))
    (alter! i (+ i 1))))
  table)))

(let Table/frequency (lambda xs (do
  (let table (Table/create (max 64 (length xs))))
  (mut i 0)
  (let len (length xs))
  (while (< i len) (do
    (let key [(get xs i)])
    (let hit (Table/get/raw table key))
    (if (= (length hit) 0)
        (Table/set/raw! table key 1)
        (Table/set/raw! table key (+ (snd (get hit 0)) 1)))
    (alter! i (+ i 1))))
  table)))

(let Vector->Table (lambda entries (do
  (let out (Table/create (max 32 (length entries))))
  (mut i 0)
  (let len (length entries))
  (while (< i len) (do
    (let entry (get entries i))
    (Table/set/raw! out (fst entry) (snd entry))
    (alter! i (+ i 1))))
  out)))

(let String/find (lambda target xs (do
  (let len (length xs))
  (mut i 0)
  (mut result -1)
  (while (and (= result -1) (< i len)) (do
    (if (match? (get xs i) target)
        (alter! result i))
    (alter! i (+ i 1))))
  result)))

(let String/find/last (lambda target xs (do
  (let len (length xs))
  (mut i 0)
  (mut result -1)
  (while (< i len) (do
    (if (match? (get xs i) target)
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

(let Vector->Tuple (lambda fn1 fn2 xs (Vector/to-tuple/raw xs fn1 fn2)))

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

(let String->Vector (lambda ch xs (String/to-vector/raw xs ch)))
(let Vector->String (lambda ch xs (Vector/to-string/raw xs ch)))
(let String->Integer Chars->Integer)
(let Chars->Dec String->Dec)


(let Vector/equal? (lambda fn? xs ys (Vector/equal/raw? xs ys fn?)))

(let Que/empty! (lambda q (do (Que/empty/raw! q) nil)))
(let Que/enque! (lambda xs v(do (Que/enqueue/raw! xs v) nil)))
(let Que/deque! (lambda queue (do (Que/tail/raw! queue) nil)))
(let Que/push! (lambda xs v (do (Que/append/raw! xs v) nil)))
(let Que/pop! (lambda queue (do (Que/head! queue) nil)))
(let Que/prepend! (lambda xs v (do (Que/prepend/raw! xs v) nil))) 
(let Que/first Que/peek)
(let Que/tail! (lambda queue (do (Que/tail/raw! queue) nil)))
(let Que/append! (lambda xs v (do (Que/append/raw! xs v) nil)))

(let Que/at (lambda i xs (if (< i 0) (Que/get xs (+ (length xs) i)) (Que/get xs i))))





(let Vector/pop-val! pop-val!)
(let Vector/last last)
(let Vector/first first)
(let Vector/pull! pop-val!)
(let Vector/get-unsafe (lambda idx xs (get xs idx)))
(let Vector/get* (lambda idx xs (if (and (>= idx 0) (< idx (length xs))) { true [(get xs idx)] } { false [] })))
(let Vector/at! (lambda idx xs (at xs idx)))
(let Vector/at* (lambda idx xs (if (< idx (length xs)) { true [(at xs idx)] } { false [] })))
(let Vector/set! (lambda xs i v (set! xs i v)))
(let Vector/append! (lambda xs x (Vector/append/raw! xs x)))
(let Vector/cons! (lambda xs x (Vector/cons/raw! xs x)))
(let Vector/cons (lambda x xs (Vector/cons/raw xs x)))
(let Vector/compare (lambda step done? initial b a (Vector/compare/raw a b step done? initial)))
(let Vector/new (lambda fn n (fill n fn)))
(let Vector/of (lambda (n x)
  (let out [])
  (loop i (< i n) (push! out x))
  out))
(let Matrix/new (lambda fn w h (Matrix/fill/raw w h fn)))
(let Vector/in-bounds? (lambda (index xs) (in-bounds? xs index)))
(let Matrix/in-bounds? (lambda (y x xs) (Matrix/in-bounds/raw? xs y x)))
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


(let Table/get (lambda key table (Table/get/raw table key)))
(let Table/get* (lambda key table
  (if (Table/has/raw? table key)
      { true [ (snd (get (Table/get/raw table key) 0)) ] }
      { false [] })))
(let Table/get-unsafe (lambda key table (snd (get (Table/get/raw table key) 0))))

(let Table/has? (lambda key table (Table/has/raw? table key)))
(let Table/set! (lambda table key value (do (Table/set/raw! table key value) nil)))
(let Table/remove! (lambda table key (do (Table/remove/raw! table key) nil)))

(let Table/drop! (lambda table keys (Table/drop/raw! table keys)))
(let Table/keep (lambda keys table (Table/keep/raw table keys)))
(let Table/omit (lambda keys table (Table/omit/raw table keys)))

(let Table/merge! (lambda table other (do (Table/merge/raw! table other) nil)))
(let Table/merge (lambda other table (Table/merge/raw table other)))

(let Table/resize! (lambda table n (do (Table/resize/raw! table n) nil)))
(let Table/compact! (lambda table (do (Table/compact/raw! table) nil)))

(let Table/create/new Table/create)
(let Table/count-entries (lambda table (do
  (mut total 0)
  (mut i 0)
  (let len (length table))
  (while (< i len) (do
    (alter! total (+ total (length (get table i))))
    (alter! i (+ i 1))))
  total)))

(let Table/get/raw* (lambda xs i some none (if (Table/has/raw? xs i) (do (some (Table/get/raw xs i)) nil) (do (none) nil))))

(let Set/size Table/size)
(let Set/values flat)

(let Heap/parent (lambda i (- (>> (+ i 1) 1) 1)))
(let Heap/left (lambda i (+ (<< i 1) 1)))
(let Heap/right (lambda i (<< (+ i 1) 1)))
(let Heap/compare? (lambda i j fn? heap (=? (fn? (get heap i) (get heap j)) true)))

(let Heap/swap! (lambda heap i j (do
  (let value (get heap i))
  (set! heap i (get heap j))
  (set! heap j value))))

(let Heap/sift-up! (lambda heap fn (do
  (mut node (- (length heap) 1))
  (while (and (> node 0) (Heap/compare? node (Heap/parent node) fn heap)) (do
    (Heap/swap! heap node (Heap/parent node))
    (alter! node (Heap/parent node)))))))

(let Heap/sift-down! (lambda heap fn (do
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
          (Heap/swap! heap node child)
          (alter! node child))
        (alter! active false)))))))

(let Heap/push! (lambda heap value fn (do
  (set! heap (length heap) value)
  (Heap/sift-up! heap fn)
  nil)))

(let Heap/pop! (lambda heap fn (do
  (let bottom (- (length heap) 1))
  (if (> bottom 0) (Heap/swap! heap 0 bottom) nil)
  (pop! heap)
  (if (not (empty? heap)) (Heap/sift-down! heap fn) nil)
  nil)))

(let Heap/replace! (lambda heap value fn (do
  (set! heap 0 value)
  (Heap/sift-down! heap fn)
  heap)))
(let Heap/empty? empty?)
(let Heap/not-empty? not-empty?)
(let Vector->Heap (lambda fn xs (do
  (let heap [])
  (mut i 0)
  (while (< i (length xs)) (do
    (Heap/push! heap (get xs i) fn)
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

(let Date/year (lambda dt (get dt 0)))
(let Date/month (lambda dt (get dt 1)))
(let Date/day (lambda dt (get dt 2)))
(let Date/hour (lambda dt (get dt 3)))

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
            (Vector/push! digits (% n base))
            (alter! n (/ n base))))
        (reverse digits)))))

  
(let Vector/reverse! (lambda xs (do
  (let len (length xs))
  (let half (/ len 2))
  (mut i 0)
  (while (< i half) (do
    (Vector/swap! xs i (- len i 1))
    (alter! i (+ i 1))))
  xs)))

(let Vector/concat (lambda xs (do
    (let out [])
    (let len-xs (length xs))
    (mut i 0)
    (while (< i len-xs) (do
      (let current (get xs i))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! out (length out) (get current j))
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    out)))
(let Vector/concat! (lambda xs os (do
    (let len-os (length os))
    (mut i 0)
    (while (< i len-os) (do
      (let current (get os i))
      (let len-current (length current))
      (mut j 0)
      (while (< j len-current) (do
        (set! xs (length xs) (get current j))
        (alter! j (+ j 1))))
      (alter! i (+ i 1))))
    xs)))
