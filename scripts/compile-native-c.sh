#!/bin/sh
set -eu

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "usage: $0 <input.que> [output-dir]" >&2
  exit 1
fi

input_file="$1"
output_dir="${2:-build}"
module_name="main"
wasm_file="$output_dir/$module_name.wasm"
c_file="$output_dir/$module_name.c"
exe_file="$output_dir/$module_name"

wasm2c_bin="$(command -v wasm2c || true)"
if [ -z "$wasm2c_bin" ]; then
  echo "error: wasm2c not found in PATH" >&2
  exit 1
fi

wasm2c_prefix="$(CDPATH= cd -- "$(dirname "$wasm2c_bin")/.." && pwd)"
wasm2c_runtime_dir="$wasm2c_prefix/share/wabt/wasm2c"
wasm2c_include_dir="$wasm2c_prefix/include"

if [ ! -f "$wasm2c_runtime_dir/wasm-rt-impl.c" ]; then
  echo "error: wasm2c runtime not found at $wasm2c_runtime_dir" >&2
  exit 1
fi

if command -v clang >/dev/null 2>&1; then
  cc_bin="clang"
else
  cc_bin="cc"
fi

if command -v quec >/dev/null 2>&1; then
  quec_bin="quec"
elif [ -x "./target/release/quec" ]; then
  quec_bin="./target/release/quec"
elif [ -x "./target/debug/quec" ]; then
  quec_bin="./target/debug/quec"
else
  echo "error: quec not found in PATH or ./target/{release,debug}/quec" >&2
  exit 1
fi

mkdir -p "$output_dir"

type_probe_bin="que"
if ! command -v "$type_probe_bin" >/dev/null 2>&1; then
  type_probe_bin="$quec_bin"
fi

result_type="$("$type_probe_bin" "$input_file" --emit types | sed -n 's/^result : //p' | tail -n 1)"
case "$result_type" in
  "()" | "Int" | "{Int * Int}" | "{Bool * [Int]}")
    ;;
  *)
    echo "error: native C runner currently supports result (), Int, {Int * Int}, or {Bool * [Int]}, got: $result_type" >&2
    exit 1
    ;;
esac

QUE_WASM_OPT="${QUE_WASM_OPT:-speed}" \
QUE_DEVIRTUALIZE="${QUE_DEVIRTUALIZE:-aggressive}" \
QUE_TCO="${QUE_TCO:-off}" \
QUE_SMALL_SCALAR_INLINE_COST="${QUE_SMALL_SCALAR_INLINE_COST:-512}" \
QUE_LOOP_UNROLL_MAX="${QUE_LOOP_UNROLL_MAX:-16}" \
QUE_LOOP_UNROLL_COST="${QUE_LOOP_UNROLL_COST:-2000}" \
QUE_BOUNDS_CHECK="${QUE_BOUNDS_CHECK:-0}" \
QUE_INT_OVERFLOW_CHECK="${QUE_INT_OVERFLOW_CHECK:-0}" \
QUE_DEC_OVERFLOW_CHECK="${QUE_DEC_OVERFLOW_CHECK:-0}" \
QUE_DIV_ZERO_CHECK="${QUE_DIV_ZERO_CHECK:-0}" \
QUE_VEC_MIN_CAP="${QUE_VEC_MIN_CAP:-8}" \
"$quec_bin" "$input_file" > "$wasm_file"
wasm2c "$wasm_file" -n "$module_name" -o "$c_file"

host_file="$output_dir/$module_name.host.c"

cat > "$host_file" <<'EOF'
#include <stdint.h>
#include <stdio.h>

#include "main.h"
#include "wasm-rt.h"

extern wasm_rt_jmp_buf g_wasm_rt_jmp_buf;

static uint32_t load_u32(w2c_main* instance, uint32_t addr) {
    uint8_t* p = instance->w2c_memory.data + addr;
    return ((uint32_t)p[0]) |
           ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) |
           ((uint32_t)p[3] << 24);
}

static void print_int_tuple2(w2c_main* instance, uint32_t tuple_ptr) {
    uint32_t data_ptr = load_u32(instance, tuple_ptr + 16);
    int32_t first = (int32_t)load_u32(instance, data_ptr);
    int32_t second = (int32_t)load_u32(instance, data_ptr + 4);
    printf("{ %d %d }\n", first, second);
}

static void print_int_vector(w2c_main* instance, uint32_t vec_ptr) {
    uint32_t len = load_u32(instance, vec_ptr);
    uint32_t data_ptr = load_u32(instance, vec_ptr + 16);
    putchar('[');
    for (uint32_t i = 0; i < len; i++) {
        if (i != 0) {
            putchar(' ');
        }
        printf("%d", (int32_t)load_u32(instance, data_ptr + i * 4));
    }
    putchar(']');
}

static void print_bool_int_vector_tuple(w2c_main* instance, uint32_t tuple_ptr) {
    uint32_t data_ptr = load_u32(instance, tuple_ptr + 16);
    uint32_t ok = load_u32(instance, data_ptr);
    uint32_t vec_ptr = load_u32(instance, data_ptr + 4);
    printf("{ %s ", ok ? "true" : "false");
    print_int_vector(instance, vec_ptr);
    printf(" }\n");
}

int main(void) {
    wasm_rt_init();

    w2c_main instance;
    wasm2c_main_instantiate(&instance);
    wasm_rt_trap_t trap = (wasm_rt_trap_t)wasm_rt_try(g_wasm_rt_jmp_buf);
    if (trap != WASM_RT_TRAP_NONE) {
        fprintf(stderr, "wasm trap: %s\n", wasm_rt_strerror(trap));
        wasm2c_main_free(&instance);
        wasm_rt_free();
        return 134;
    }
    uint32_t result = w2c_main_main(&instance);
EOF

case "$result_type" in
  "()")
    cat >> "$host_file" <<'EOF'
    (void)result;
EOF
    ;;
  "Int")
    cat >> "$host_file" <<'EOF'
    printf("%d\n", (int32_t)result);
EOF
    ;;
  "{Int * Int}")
    cat >> "$host_file" <<'EOF'
    print_int_tuple2(&instance, result);
EOF
    ;;
  "{Bool * [Int]}")
    cat >> "$host_file" <<'EOF'
    print_bool_int_vector_tuple(&instance, result);
EOF
    ;;
esac

cat >> "$host_file" <<'EOF'

    wasm2c_main_free(&instance);
    wasm_rt_free();
    return 0;
}
EOF

"$cc_bin" -O3 -DNDEBUG -flto -march=native -fno-math-errno -fno-trapping-math \
  -I "$output_dir" \
  -I "$wasm2c_include_dir" \
  -I "$wasm2c_runtime_dir" \
  "$host_file" \
  "$c_file" \
  "$wasm2c_runtime_dir/wasm-rt-impl.c" \
  "$wasm2c_runtime_dir/wasm-rt-mem-impl.c" \
  -o "$exe_file"

echo "$exe_file"
