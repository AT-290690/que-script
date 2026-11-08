; Foundational algebraic data types shared by the standard library.
; Keep this file dependency-free so collection modules can build on it.

(enum Option T
  None
  (Some value T))
