(let const std/fn/const)
(let floor std/dec/floor)
(let ceil std/dec/ceil)

(let extreme std/vector/int/extreme)
(let map/tuple (lambda fn xs (std/tuple/map xs fn)))
(let map/fst (lambda fn xs (std/tuple/map/fst xs fn)))
(let map/snd (lambda fn xs (std/tuple/map/snd xs fn)))
(let flat-map (lambda fn xs (std/vector/flat-map xs fn)))
(let map (lambda fn xs (std/vector/map xs fn)))
(let filter (lambda fn? xs (std/vector/filter xs fn?)))
(let reduce (lambda fn init xs (std/vector/reduce xs fn init)))

(let for (lambda fn xs (std/vector/for xs fn)))
(let for/i (lambda fn xs (std/vector/for/i xs fn)))

(let each (lambda xs fn (do (std/vector/for xs fn) xs)))
(let each/i (lambda xs fn (do (std/vector/for/i xs fn) xs)))

(let transpose std/vector/3d/rotate)
(let interleave std/vector/2d/interleave)
(let intersperse (lambda xs x (std/vector/intersperse x xs)))
(let every? (lambda fn? xs (std/vector/every? xs fn?)))
(let some? (lambda fn? xs (std/vector/some? xs fn?)))
(let empty? std/vector/empty?)
(let not-empty? std/vector/not-empty?)

(let exclude (lambda fn? xs (std/vector/filter xs (lambda x (not (fn? x))))))
(let select (lambda fn? xs (std/vector/filter xs fn?)))

(let find (lambda fn? xs (std/vector/find-index xs fn?)))
(let partition (lambda n xs (std/vector/partition xs n)))
(let reverse std/vector/reverse)

(let slice (lambda start end xs (std/vector/slice xs start end)))
(let range std/vector/int/range)
(let range/int std/vector/int/range)
(let range/dec std/vector/dec/range)
(let square std/int/square)
(let expt (lambda b x (std/int/expt x b)))
(let sqrt std/int/sqrt)
(let expt/int (lambda b x (std/int/expt x b)))
(let sqrt/int std/int/sqrt)
(let expt/dec (lambda b x (std/dec/expt x b)))
(let sqrt/dec std/dec/sqrt)
(let log std/dec/log)
(let odd? std/int/odd?)
(let even? std/int/even?)
(let odd/int? std/int/odd?)
(let even/int? std/int/even?)
(let odd/dec? std/dec/odd?)
(let even/dec? std/dec/even?)
(let one? std/int/one?)
(let zero? std/int/zero?)
(let one/int? std/int/one?)
(let zero/int? std/int/zero?)
(let one/dec? std/dec/one?)
(let zero/dec? std/dec/zero?)

(let map/until (lambda fn fn? xs (std/vector/map/until xs fn fn?)))
(let map/until/i (lambda fn fn? xs (std/vector/map/until/i xs fn fn?)))
(let reduce/until (lambda fn fn? init xs (std/vector/reduce/until xs fn fn? init)))
(let reduce/until/i (lambda fn fn? init xs (std/vector/reduce/until/i xs fn fn? init)))
(let for/until (lambda fn fn? xs (std/vector/for/until xs fn fn?)))
(let for/until/i (lambda fn fn? xs (std/vector/for/until/i xs fn fn?)))

(let each/until (lambda fn fn? xs (do (std/vector/for/until xs fn fn?) xs)))
(let each/until/i (lambda fn fn? xs (do (std/vector/for/until/i xs fn fn?) xs)))

(let map/i (lambda fn xs (std/vector/map/i xs fn)))
(let reduce/i (lambda fn init xs (std/vector/reduce/i xs fn init)))
(let filter/i (lambda fn? xs (std/vector/filter/i xs fn?)))
(let some/i? (lambda fn? xs (std/vector/some/i? xs fn?)))
(let every/i? (lambda fn? xs (std/vector/every/i? xs fn?)))

(let ones std/vector/int/ones)
(let zeroes std/vector/int/zeroes)
(let ones/int std/vector/int/ones)
(let zeroes/int std/vector/int/zeroes)
(let ones/dec std/vector/dec/ones)
(let zeroes/dec std/vector/dec/zeroes)

(let positive? std/int/positive?)
(let negative? std/int/negative?)
(let invert std/int/invert)
(let negative-one? std/int/negative-one?)
(let divisible? (lambda y x (std/int/divisible? x y)))

(let positive/int? std/int/positive?)
(let negative/int? std/int/negative?)
(let invert/int std/int/invert)
(let negative-one/int? std/int/negative-one?)
(let divisible/int? (lambda y x (std/int/divisible? x y)))

(let positive/dec? std/dec/positive?)
(let negative/dec? std/dec/negative?)
(let invert/dec std/dec/invert)
(let negative-one/dec? std/dec/negative-one?)
(let divisible/dec? (lambda y x (std/dec/divisible? x y)))

(let upper std/char/upper)
(let lower std/char/lower)
(let match? std/vector/char/equal?)

(let digit? std/char/digit?)
(let fill std/vector/2d/fill)

(let max std/int/max)
(let min std/int/min)
(let max/int std/int/max)
(let min/int std/int/min)
(let max/dec std/dec/max)
(let min/dec std/dec/min)

(let maximum std/vector/int/maximum)
(let minimum std/vector/int/minimum)
(let maximum/int std/vector/int/maximum)
(let minimum/int std/vector/int/minimum)
(let maximum/dec std/vector/dec/maximum)
(let minimum/dec std/vector/dec/minimum)

(let gt/int? (lambda a b (> b a)))
(let lt/int? (lambda a b (< b a)))
(let gte/int? (lambda a b (>= b a)))
(let lte/int? (lambda a b (<= b a)))
(let gt/dec? (lambda a b (>. b a)))
(let lt/dec? (lambda a b (<. b a)))
(let gte/dec? (lambda a b (>=. b a)))
(let lte/dec? (lambda a b (<=. b a)))

(let gt/bool? (lambda a b (and (=? a true) (=? b false))))
(let lt/bool? (lambda a b (and (=? a false) (=? b true))))
(let and? (lambda a b (and (=? a true) (=? b true))))
(let or? (lambda a b (or (=? a true) (=? b true))))
(let not? (lambda x (not x)))

(let abs std/int/abs)
(let abs/int std/int/abs)
(let abs/dec std/dec/abs)

(let first std/vector/first)
(let last std/vector/last)
(let pair (lambda a b (tuple a b)))
(let product std/vector/int/product)
(let product/int std/vector/int/product)
(let product/dec std/vector/dec/product)
(let sum std/vector/int/sum)
(let sum/int std/vector/int/sum)
(let sum/dec std/vector/dec/sum)
(let avg std/int/average)
(let avg/int std/int/average)
(let avg/dec std/dec/average)
(let mean std/vector/int/mean)
(let median std/vector/int/median)
(let mean/int std/vector/int/mean)
(let mean/dec std/vector/dec/mean)
(let median/int std/vector/int/median)
(let median/dec std/vector/dec/median)
(let zip std/vector/tuple/zip)
(let unzip std/vector/tuple/unzip)
(let zip-with (lambda f xs ys (std/vector/tuple/zip-with xs ys f)))

(let window (lambda n xs (std/vector/sliding-window xs n)))
(let flat std/vector/flat-one)
(let enumerate std/vector/enumerate)
(let clamp (lambda limit x (std/int/clamp x limit)))
(let clamp-range (lambda start end x (std/int/clamp-range x start end)))

(let clamp/int (lambda limit x (std/int/clamp x limit)))
(let clamp-range/int (lambda start end x (std/int/clamp-range x start end)))
(let clamp/dec (lambda limit x (std/dec/clamp x limit)))
(let clamp-range/dec (lambda start end x (std/dec/clamp-range x start end)))

(let at std/vector/at)
(let scan (lambda fn xs (std/vector/adjacent-difference xs fn)))
(let cycle std/vector/cycle)
(let replicate std/vector/replicate)
(let cartesian-product std/vector/cartesian-product)
(let lcm std/int/lcm)
(let gcd std/int/gcd)

(let delta std/int/delta)
(let delta/int std/int/delta)
(let delta/dec std/dec/delta)

(let map/adjacent (lambda fn xs (std/vector/map/adjacent xs fn)))

(let buckets std/vector/buckets)

(let count/char (lambda x xs (std/vector/char/count xs x)))
(let count/int (lambda x xs (std/vector/int/count xs x)))
(let count/dec (lambda x xs (std/vector/dec/count xs x)))
(let count/bool (lambda x xs (std/vector/bool/count xs x)))

(let count (lambda fn? xs (std/vector/count-of xs fn?)))

(let points (lambda fn? xs (std/vector/3d/points xs fn?)))

(let unique/int std/vector/int/unique)
(let unique/char std/vector/char/unique)

(let permutation std/vector/permutations)
(let combination/pairs std/vector/tuple/unique-pairs)
(let combination std/vector/combinations)
(let combination/n (lambda n xs (std/vector/combinations/n xs n)))
(let subset std/vector/subset)


(let in-bounds? std/vector/in-bounds?)

(let take/first (lambda n xs (std/vector/take xs n)))
(let drop/first (lambda n xs (std/vector/drop xs n)))

(let take/last (lambda n xs (std/vector/take/last xs n)))
(let drop/last (lambda n xs (std/vector/drop/last xs n)))

(let true/option std/true/option)
(let false/option std/false/option)
(let resolve/option (lambda fn df xs (std/vector/option/resolve xs fn df)))

(let call (lambda fn xs (std/fn/exec xs fn)))

(let copy std/vector/copy)
(let sort (lambda fn xs (do 
  (let out (std/vector/copy xs))
  (std/vector/sort! out fn)
  out)))

(let neighborhood (lambda directions y x fn xs (std/vector/3d/adjacent xs directions y x fn)))
(let neighborhood/moore std/vector/3d/moore-neighborhood)
(let neighborhood/diagonal std/vector/3d/diagonal-neighborhood)
(let neighborhood/kernel std/vector/3d/kernel-neighborhood)
(let neighborhood/von-neumann std/vector/3d/von-neumann-neighborhood)

(let group (lambda fn xs (std/vector/hash/table/group-by xs fn)))

(let tail (lambda xs (std/vector/slice xs 1 (length xs))))
(let head (lambda xs (std/vector/slice xs 0 (- (length xs) 1))))

(let fp/mul (lambda b a (* a b)))
(let fp/div (lambda b a (/ a b)))
(let fp/add (lambda b a (+ a b)))
(let fp/sub (lambda b a (- a b)))
(let fp/emod (lambda b a (emod a b)))
(let fp/mod (lambda b a (mod a b)))

(let cond/dispatch (lambda fn? a b x (if (fn? x) a b)))
; experimental functions
(let split (lambda ys str (do
  (if (empty? ys)
      [str]
      (do
        (let out [])
        (mut i 0)
        (mut start 0)
        (let len-str (length str))
        (let len-ys (length ys))
        (while (< i len-str) (do
          (mut matched? false)
          (if (and (<= (+ i len-ys) len-str) (=# (get str i) (get ys 0)))
              (do
                (alter! matched? true)
                (mut j 1)
                (while (and matched? (< j len-ys)) (do
                  (if (not (=# (get str (+ i j)) (get ys j))) (alter! matched? false))
                  (alter! j (+ j 1))))))
          (if matched?
              (do
                (push! out (slice start i str))
                (alter! start (+ i len-ys))
                (alter! i (+ i len-ys)))
              (alter! i (+ i 1)))))
        (push! out (slice start len-str str))
        out)))))

(let join (lambda str xs (do
    (let out [])
    (mut i 0)
    (let len-xs (length xs))
    (while (< i len-xs) (do
      (let current (get xs i))
      (mut j 0)
      (while (< j (length current)) (do
        (push! out (get current j))
        (alter! j (+ j 1))))
      (if (< (+ i 1) len-xs)
          (do
            (mut k 0)
            (while (< k (length str)) (do
              (push! out (get str k))
              (alter! k (+ k 1))))))
      (alter! i (+ i 1))))
   out)))
(let join/lines (lambda xs (join [nl] xs)))
(let join/commas (lambda xs (join "," xs)))

(let replace (lambda a b xs (|> xs (split a) (join b))))

(let graph/project-path
  (lambda col path
    (map (lambda i (get col i)) path)))

(let graph/path->nodes
  (lambda from-col to-col path
    (if (empty? path)
        []
        (do
          (let out [(get from-col (get path 0))])
          (for (lambda i (push! out (get to-col i))) path)
          out))))

(let graph/rotate
  (lambda start xs
    (do
      (let out [])
      (mut i 0)
      (let len (length xs))
      (while (< i len) (do
        (push! out (get xs (mod (+ start i) len)))
        (alter! i (+ i 1))))
      out)))

(let graph/cycle/min-rotation
  (lambda nodes
    (if (<= (length nodes) 1)
        0
        (do
          (let cycle-len (- (length nodes) 1))
          (mut best 0)
          (mut i 1)
          (while (< i cycle-len) (do
            (if (std/vector/char/lesser? (get nodes i) (get nodes best))
                (alter! best i)
                nil)
            (alter! i (+ i 1))))
          best))))

(let graph/normalize-cycle
  (lambda nodes
    (if (<= (length nodes) 1)
        nodes
        (do
          (let start (graph/cycle/min-rotation nodes))
          (let core (graph/rotate start (slice 0 (- (length nodes) 1) nodes)))
          (cons core [(get core 0)])))))

(let graph/normalize-path
  (lambda from-col to-col path
    (if (empty? path)
        []
        (graph/rotate (graph/cycle/min-rotation (graph/path->nodes from-col to-col path)) path))))

(let graph/cycle-key
  (lambda from-col to-col path
    (do
      (let normalized-nodes (graph/normalize-cycle (graph/path->nodes from-col to-col path)))
      (let normalized-path (graph/normalize-path from-col to-col path))
      (cons (join "->" normalized-nodes)
            "::"
            (join "," (map Integer->String normalized-path))))))

(let graph/outgoing-by
  (lambda from-col rows
    (reduce
      (lambda (a i)
        (do
          (let from (get from-col i))
          (if (Table/has? from a)
              (push! (Table/get-unsafe from a) i)
              (Table/set! a from [i]))
          a))
      (Table/new)
      rows)))

(let graph/simple-cycle?
  (lambda from-col to-col path
    (do
      (let nodes (graph/path->nodes from-col to-col path))
      (if (or (< (length nodes) 3)
              (not (match? (get nodes 0) (last nodes))))
          false
          (do
            (let seen (Set/new))
            (mut i 0)
            (mut ok true)
            (let stop (- (length nodes) 1))
            (while (and ok (< i stop)) (do
              (let node (get nodes i))
              (if (Set/has? node seen)
                  (alter! ok false)
                  (Set/add! seen node))
              (alter! i (+ i 1))))
            ok)))))

(let graph/find-cycles
  (lambda rows from-col to-col next-ok? cycle-ok?
    (do
      (let outgoing (graph/outgoing-by from-col rows))
      (let seen (Set/new))
      (let out [])
      (let contains-node?
        (lambda node nodes
          (some? (lambda x (match? x node)) nodes)))
      (letrec dfs
        (lambda origin current visited path
          (if (Table/has? current outgoing)
              (for
                (lambda next-i
                  (do
                    (let next-to (get to-col next-i))
                    (if (next-ok? path next-i)
                        (if (match? next-to origin)
                            (do
                              (let full-path (cons path [next-i]))
                              (if (and (graph/simple-cycle? from-col to-col full-path)
                                       (cycle-ok? full-path))
                                  (do
                                    (let key (graph/cycle-key from-col to-col full-path))
                                    (if (not (Set/has? key seen))
                                        (do
                                          (Set/add! seen key)
                                          (push! out full-path))
                                        nil))
                                  nil))
                            (if (not (contains-node? next-to visited))
                                (dfs origin
                                     next-to
                                     (cons visited [next-to])
                                     (cons path [next-i]))
                                nil))
                        nil)))
                (Table/get-unsafe current outgoing))
              nil)))
      (for
        (lambda start-i
          (dfs (get from-col start-i)
               (get to-col start-i)
               [(get from-col start-i) (get to-col start-i)]
               [start-i]))
        rows)
      out)))

(let graph/find-cycles/increasing-time
  (lambda rows from-col to-col time-col min-length max-span-seconds
    (graph/find-cycles
      rows
      from-col
      to-col
      (lambda (path next-i)
        (> (get time-col next-i) (get time-col (at path -1))))
      (lambda (path)
        (and (>= (length path) min-length)
             (<= (- (get time-col (at path -1))
                    (get time-col (get path 0)))
                 max-span-seconds))))))

(let graph/has-cycle?
  (lambda rows from-col to-col
    (not
      (empty?
        (graph/find-cycles
          rows
          from-col
          to-col
          (lambda (path next-i) true)
          (lambda (path) true))))))

(let floyd/cycle?
  (lambda next eq? start limit
    (do
      (&mut turtle start)
      (&mut hare start)
      (mut steps 0)
      (mut found? false)
      (while (and (not found?) (< steps limit))
        (&alter! turtle (next (&get turtle)))
        (&alter! hare (next (&get hare)))
        (&alter! hare (next (&get hare)))
        (alter! found? (eq? (&get hare) (&get turtle)))
        (++ steps))
      found?)))

(let autocorrect (lambda dict word (std/vector/char/autocorrect word dict)))
