#!/bin/bash

OUTPUT_DIR="build"
mkdir -p "$OUTPUT_DIR"

# 1. Generate files
QUE_WASM_OPT=speed QUE_DEVIRTUALIZE=aggressive QUE_TCO=aggressive QUE_BOUNDS_CHECK=0 QUE_DIV_ZERO_CHECK=0 QUE_INT_OVERFLOW_CHECK=0 QUE_FLOAT_OVERFLOW_CHECK=0 QUE_VEC_MIN_CAP=8 QUE_VEC_GROWTH_NUM=2 QUE_VEC_GROWTH_DEN=1 QUE_DECIMAL_SCALE=1000 que main.que --emit wasm --allow write > "$OUTPUT_DIR/main.wasm"
wasm2c "$OUTPUT_DIR/main.wasm" -n main -o "$OUTPUT_DIR/main.c"

# 2. Automatically find the allocator
ALLOC_FUNC=$(grep -oE "w2c_main_.*alloc" "$OUTPUT_DIR/main.h" | head -n 1)

# 3. Create the host.c
cat <<EOF > "$OUTPUT_DIR/host.c"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include "main.h"

w2c_main instance;
struct w2c_host {};

// --- Helper: Extract String ---
void get_string_from_vector(uint32_t vector_ptr, char* out_buf, size_t max_len) {
    uint32_t len = *(uint32_t*)(&instance.w2c_memory.data[vector_ptr]);
    uint32_t data_ptr = *(uint32_t*)(&instance.w2c_memory.data[vector_ptr + 16]);
    size_t i;
    for (i = 0; i < len && i < (max_len - 1); i++) {
        uint32_t char_code;
        memcpy(&char_code, &instance.w2c_memory.data[data_ptr + (i * 4)], 4);
        out_buf[i] = (char)char_code;
    }
    out_buf[i] = '\0';
}

// --- PRINT ---
uint32_t w2c_host_print(struct w2c_host* host, uint32_t ptr) {
    uint32_t len = *(uint32_t*)(&instance.w2c_memory.data[ptr]);
    uint32_t data_ptr = *(uint32_t*)(&instance.w2c_memory.data[ptr + 16]);
    for (uint32_t i = 0; i < len; i++) {
        uint32_t char_code;
        memcpy(&char_code, &instance.w2c_memory.data[data_ptr + (i * 4)], 4);
        putchar((char)char_code);
    }
    fflush(stdout);
    return 0;
}

// --- READ FILE ---
uint32_t w2c_host_read_file(struct w2c_host* host, uint32_t path_vec_ptr) {
    char filepath[256];
    get_string_from_vector(path_vec_ptr, filepath, sizeof(filepath));
    FILE* f = fopen(filepath, "rb");
    if (!f) return 0;
    fseek(f, 0, SEEK_END);
    long fsize = ftell(f);
    fseek(f, 0, SEEK_SET);
    uint32_t header_ptr = $ALLOC_FUNC(&instance, 32 + (fsize * 4));
    uint32_t data_ptr = header_ptr + 32;
    memcpy(&instance.w2c_memory.data[header_ptr], &fsize, 4);
    memcpy(&instance.w2c_memory.data[header_ptr + 16], &data_ptr, 4);
    for (long i = 0; i < fsize; i++) {
        uint32_t ch = (uint32_t)fgetc(f);
        memcpy(&instance.w2c_memory.data[data_ptr + (i * 4)], &ch, 4);
    }
    fclose(f);
    return header_ptr;
}

// --- CLEAR ---
uint32_t w2c_host_clear(struct w2c_host* host) {
    // ANSI escape code: clear screen and move cursor to top-left
    printf("\033[2J\033[H");
    fflush(stdout);
    return 0;
}

// --- SLEEP ---
uint32_t w2c_host_sleep(struct w2c_host* host, uint32_t ms) {
    usleep(ms * 1000); 
    return 0;
}

int main(int argc, char** argv) {
    wasm_rt_init();
    struct w2c_host host_instance; 
    wasm2c_main_instantiate(&instance, &host_instance);
    w2c_main_main(&instance);
    wasm2c_main_free(&instance);
    wasm_rt_free();
    return 0;
}
EOF

# 4. Compile
gcc -O3 -I/opt/homebrew/Cellar/wabt/1.0.40/include -I"$OUTPUT_DIR" \
    "$OUTPUT_DIR/host.c" "$OUTPUT_DIR/main.c" \
    /opt/homebrew/share/wabt/wasm2c/wasm-rt-impl.c \
    /opt/homebrew/share/wabt/wasm2c/wasm-rt-mem-impl.c \
    -o "$OUTPUT_DIR/my_program"

# 5. Run
if [ $? -eq 0 ]; then
    "./$OUTPUT_DIR/my_program"
fi
