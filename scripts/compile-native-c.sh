#!/bin/sh
set -eu

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "usage: $0 <input.que> [output-dir]" >&2
  exit 1
fi

input_file="$1"
output_dir="${2:-build}"
script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
repo_dir="$(CDPATH= cd -- "$script_dir/.." && pwd)"
native_host_dir="${QUEC_NATIVE_HOST_DIR:-$repo_dir/miscs/native-c}"
if [ -z "${QUEC_NATIVE_HOST_DIR:-}" ] && [ -d "$script_dir/native-c" ]; then
  native_host_dir="$script_dir/native-c"
fi
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

if [ -n "${QUE_COMPILER:-}" ]; then
  que_bin="$QUE_COMPILER"
elif [ -n "${QUEC_COMPILER:-}" ]; then
  que_bin="$QUEC_COMPILER"
elif command -v que >/dev/null 2>&1; then
  que_bin="que"
elif [ -x "$repo_dir/target/release/que" ]; then
  que_bin="$repo_dir/target/release/que"
elif [ -x "$repo_dir/target/debug/que" ]; then
  que_bin="$repo_dir/target/debug/que"
else
  echo "error: que not found in PATH or $repo_dir/target/{release,debug}/que" >&2
  exit 1
fi

mkdir -p "$output_dir"

result_type="$("$que_bin" "$input_file" --emit types | sed -n 's/^result : //p' | tail -n 1)"
if [ -z "$result_type" ]; then
  echo "error: could not determine Que result type" >&2
  exit 1
fi

export QUE_WASM_OPT="${QUE_WASM_OPT:-speed}"
export QUE_DEVIRTUALIZE="${QUE_DEVIRTUALIZE:-aggressive}"
export QUE_TCO="${QUE_TCO:-off}"
export QUE_SMALL_SCALAR_INLINE_COST="${QUE_SMALL_SCALAR_INLINE_COST:-512}"
export QUE_LOOP_UNROLL_MAX="${QUE_LOOP_UNROLL_MAX:-16}"
export QUE_LOOP_UNROLL_COST="${QUE_LOOP_UNROLL_COST:-2000}"
export QUE_BOUNDS_CHECK="${QUE_BOUNDS_CHECK:-0}"
export QUE_INT_OVERFLOW_CHECK="${QUE_INT_OVERFLOW_CHECK:-0}"
export QUE_DEC_OVERFLOW_CHECK="${QUE_DEC_OVERFLOW_CHECK:-0}"
export QUE_DIV_ZERO_CHECK="${QUE_DIV_ZERO_CHECK:-0}"
export QUE_VEC_MIN_CAP="${QUE_VEC_MIN_CAP:-8}"
"$que_bin" compile "$input_file" --out "$wasm_file"
wasm2c "$wasm_file" -n "$module_name" -o "$c_file"

if grep -q 'struct w2c_host' "$output_dir/$module_name.h"; then
  has_host_imports=1
else
  has_host_imports=0
fi

host_file="$output_dir/$module_name.host.c"

cat > "$host_file" <<'EOF'
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "main.h"
#include "que_host.h"
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

static void print_char_vector(w2c_main* instance, uint32_t vec_ptr) {
    uint32_t len = load_u32(instance, vec_ptr);
    uint32_t data_ptr = load_u32(instance, vec_ptr + 16);
    for (uint32_t i = 0; i < len; i++) {
        uint32_t c = load_u32(instance, data_ptr + i * 4);
        if (c <= 0x7f) putchar((int)c);
        else if (c <= 0x7ff) {
            putchar((int)(0xc0 | (c >> 6)));
            putchar((int)(0x80 | (c & 0x3f)));
        } else if (c <= 0xffff) {
            putchar((int)(0xe0 | (c >> 12)));
            putchar((int)(0x80 | ((c >> 6) & 0x3f)));
            putchar((int)(0x80 | (c & 0x3f)));
        } else {
            putchar((int)(0xf0 | (c >> 18)));
            putchar((int)(0x80 | ((c >> 12) & 0x3f)));
            putchar((int)(0x80 | ((c >> 6) & 0x3f)));
            putchar((int)(0x80 | (c & 0x3f)));
        }
    }
    putchar('\n');
}

int main(int argc, char** argv) {
    wasm_rt_init();

    w2c_main instance;
EOF

if [ "$has_host_imports" -eq 1 ]; then
  cat >> "$host_file" <<'EOF'
    struct w2c_host host;
    que_host_init(&host, &instance, que_host_parse_permissions(getenv("QUE_ALLOW")));
    wasm2c_main_instantiate(&instance, &host);
EOF
else
  cat >> "$host_file" <<'EOF'
    wasm2c_main_instantiate(&instance);
    struct w2c_host host;
    que_host_init(&host, &instance, que_host_parse_permissions(getenv("QUE_ALLOW")));
EOF
fi

cat >> "$host_file" <<'EOF'
    wasm_rt_trap_t trap = (wasm_rt_trap_t)wasm_rt_try(g_wasm_rt_jmp_buf);
    if (trap != WASM_RT_TRAP_NONE) {
        fprintf(stderr, "wasm trap: %s\n", wasm_rt_strerror(trap));
        wasm2c_main_free(&instance);
        wasm_rt_free();
        return 134;
    }
EOF

cat >> "$host_file" <<'EOF'
    if (que_host_configure_argv(&host, argc, argv) != 0) {
        wasm2c_main_free(&instance);
        wasm_rt_free();
        return 2;
    }
    uint32_t result = w2c_main_main(&instance);
EOF

if [ "$result_type" != "()" ]; then
  cat >> "$host_file" <<EOF
    que_host_print_result(&host, result, "$result_type");
EOF
fi

cat >> "$host_file" <<'EOF'

    wasm2c_main_free(&instance);
    wasm_rt_free();
    return 0;
}
EOF

"$cc_bin" -O3 -DNDEBUG -flto -march=native -fno-math-errno -fno-trapping-math \
  -I "$output_dir" \
  -I "$native_host_dir" \
  -I "$wasm2c_include_dir" \
  -I "$wasm2c_runtime_dir" \
  "$host_file" \
  "$native_host_dir/que_host.c" \
  "$c_file" \
  "$wasm2c_runtime_dir/wasm-rt-impl.c" \
  "$wasm2c_runtime_dir/wasm-rt-mem-impl.c" \
  -o "$exe_file"

echo "$exe_file"
