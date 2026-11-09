; Foundational algebraic data types shared by the standard library.
; Keep this file dependency-free so collection modules can build on it.

(enum Option T
  None
  (Some value T))

; `Result` carries either a successful value or a typed error.  It is useful
; for operations where `None` would not explain why an operation failed.
(enum Result T E
  (Result/ok value T)
  (Result/err error E))

; `Ord` gives a name to an ordering for values of one type.  The type
; parameter keeps its comparison function connected to the values it orders.
; It is a record rather than an enum: an order has one piece of data, its
; `before?` predicate, and no alternatives to match.
(record Ord T before? (T -> T -> Bool))

; Arbitrary-precision values use nominal records rather than exposing their
; digit vectors and implementation tuples as their public types.
(record BigInt digits [Int])
(record SignedBigInt negative? Bool digits BigInt)
(record StructBigDec precision Int signed SignedBigInt)
