#!/bin/sh
set -eu

here="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
repo="$(CDPATH= cd -- "$here/../.." && pwd)"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/que-native-c.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT INT TERM

cat > "$tmp/pure.que" <<'EOF'
{true [1 2 3] "hello"}
EOF
"$repo/scripts/compile-native-c.sh" "$tmp/pure.que" "$tmp/pure" >/dev/null
test "$("$tmp/pure/main")" = '{ true { [1 2 3] "hello" } }'

cat > "$tmp/argv.que" <<'EOF'
ARGV
EOF
"$repo/scripts/compile-native-c.sh" "$tmp/argv.que" "$tmp/argv" >/dev/null
test "$("$tmp/argv/main" one two)" = '["one" "two"]'

cat > "$tmp/io.que" <<'EOF'
(block
  (write! "roundtrip.txt" "native C IO")
  (print! (read! "roundtrip.txt")))
EOF
"$repo/scripts/compile-native-c.sh" "$tmp/io.que" "$tmp/io" >/dev/null
test "$(cd "$tmp" && "$tmp/io/main" --allow read write print)" = 'native C IO'

cat > "$tmp/serde.que" <<'EOF'
(sig value {Int {[Bool] [Char]}})
(let value (deserialize "{ 42 { [true false] \"hello\" } }"))
(serialize value)
EOF
"$repo/scripts/compile-native-c.sh" "$tmp/serde.que" "$tmp/serde" >/dev/null
test "$("$tmp/serde/main")" = '"{ 42 { [true false] \"hello\" } }"'

cat > "$tmp/stream.txt" <<'EOF'
abcdef
EOF
cat > "$tmp/stream.que" <<'EOF'
(read/chunks! "stream.txt" 2 (lambda (chunk) (print! chunk) false))
EOF
"$repo/scripts/compile-native-c.sh" "$tmp/stream.que" "$tmp/stream" >/dev/null
test "$(cd "$tmp" && "$tmp/stream/main" --allow read print)" = "abcdef
false"

echo "native C host tests passed"
