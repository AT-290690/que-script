(let csv/char/cr (get (string 13) 0))

(let csv/header fst)
(let csv/rows snd)

(let csv/empty-cell? (lambda cell (= (length cell) 0)))

(let csv/trim-cr (lambda cell
  (if (and (not-empty? cell) (=# (last cell) csv/char/cr))
      (head cell)
      cell)))

(let csv/split-line (lambda cell-delim line
  (|> line
      (csv/trim-cr)
      (String->Vector cell-delim)
      (map csv/trim-cr))))

(let csv/read/simple
  (lambda text cell-delim
    (do
      (let raw-lines (String->Vector nl text))
      (let lines (filter not-empty? (map csv/trim-cr raw-lines)))
      (let header
        (if (empty? lines)
            []
            (csv/split-line cell-delim (get lines 0))))
      (let rows
        (if (< (length lines) 2)
            []
            (map (lambda line (csv/split-line cell-delim line)) (tail lines))))
      { header rows })))

(let csv/header-index
  (lambda header header-name
    (find (lambda cell (match? cell header-name)) header)))

(let csv/cell-at
  (lambda row idx
    (if (and (>= idx 0) (< idx (length row)))
        (get row idx)
        "")))

(let csv/as-string
  (lambda cell default
    (if (csv/empty-cell? cell)
        default
        cell)))

(let csv/as-int
  (lambda cell default
    (if (csv/empty-cell? cell)
        default
        (String->Integer cell))))

(let csv/as-decimal
  (lambda cell default
    (if (csv/empty-cell? cell)
        default
        (String->Dec cell))))

(let csv/as-bool
  (lambda cell default
    (if (csv/empty-cell? cell)
        default
        (do
          (let lowered (map lower cell))
          (cond
            (or (match? lowered "true") (match? lowered "1") (match? lowered "yes")) true
            (or (match? lowered "false") (match? lowered "0") (match? lowered "no")) false
            default)))))

(let csv/column/by
  (lambda parsed header-name reader default
    (do
      (let header (csv/header parsed))
      (let rows (csv/rows parsed))
      (let idx (csv/header-index header header-name))
      (if (< idx 0)
          []
          (map (lambda row (reader (csv/cell-at row idx) default)) rows)))))

(let csv/column/string
  (lambda parsed header-name default
    (csv/column/by parsed header-name csv/as-string default)))

(let csv/column/int
  (lambda parsed header-name default
    (csv/column/by parsed header-name csv/as-int default)))

(let csv/column/decimal
  (lambda parsed header-name default
    (csv/column/by parsed header-name csv/as-decimal default)))

(let csv/column/bool
  (lambda parsed header-name default
    (csv/column/by parsed header-name csv/as-bool default)))

(letmacro with-csv-columns/from
  ((parsed body)
    (qq (uq body)))
  ((parsed name header reader default . rest)
    (qq (block
          (let (uq name) ((uq reader) (uq parsed) (uq header) (uq default)))
          (with-csv-columns/from (uq parsed) (uqs rest))))))

(letmacro with-csv-columns
  (lambda text cell-delim . rest
    (do
      (let parsed (gensym))
      (qq ((lambda (uq parsed)
              (with-csv-columns/from (uq parsed) (uqs rest)))
            (csv/read/simple (uq text) (uq cell-delim)))))))
