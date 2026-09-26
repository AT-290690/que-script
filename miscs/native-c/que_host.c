#if !defined(_WIN32)
#define _POSIX_C_SOURCE 200809L
#else
#define _CRT_RAND_S
#endif

#include "que_host.h"

#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#if defined(_WIN32)
#include <direct.h>
#include <io.h>
#include <windows.h>
#define QUE_MKDIR(path) _mkdir(path)
#define QUE_RMDIR(path) _rmdir(path)
#else
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#define QUE_MKDIR(path) mkdir((path), 0777)
#define QUE_RMDIR(path) rmdir(path)
#endif

#include "main.h"
#include "wasm-rt.h"

static void que_host_fail(const char* operation, const char* detail) {
    fprintf(stderr, "que native host: %s: %s\n", operation, detail);
    wasm_rt_trap(WASM_RT_TRAP_UNREACHABLE);
}

static void require_permission(struct w2c_host* host, uint32_t permission,
                               const char* operation) {
    if ((host->permissions & permission) == 0) {
        char message[160];
        snprintf(message, sizeof(message),
                 "permission denied (set QUE_ALLOW to include the required permission)");
        que_host_fail(operation, message);
    }
}

static uint32_t load_u32(struct w2c_host* host, uint32_t address) {
    wasm_rt_memory_t* memory = &host->instance->w2c_memory;
    if ((uint64_t)address + 4u > memory->size) {
        que_host_fail("memory", "invalid Que vector address");
    }
    const uint8_t* p = memory->data + address;
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static char* read_que_string(struct w2c_host* host, uint32_t vector) {
    uint32_t length = load_u32(host, vector);
    uint32_t data = load_u32(host, vector + 16u);
    if ((uint64_t)data + (uint64_t)length * 4u > host->instance->w2c_memory.size) {
        que_host_fail("string", "invalid Que string data");
    }

    size_t capacity = (size_t)length * 4u + 1u;
    char* out = (char*)malloc(capacity);
    if (out == NULL) {
        que_host_fail("string", "out of host memory");
    }

    size_t used = 0;
    for (uint32_t i = 0; i < length; ++i) {
        uint32_t c = load_u32(host, data + i * 4u);
        if (c <= 0x7fu) {
            out[used++] = (char)c;
        } else if (c <= 0x7ffu) {
            out[used++] = (char)(0xc0u | (c >> 6));
            out[used++] = (char)(0x80u | (c & 0x3fu));
        } else if (c <= 0xffffu && !(c >= 0xd800u && c <= 0xdfffu)) {
            out[used++] = (char)(0xe0u | (c >> 12));
            out[used++] = (char)(0x80u | ((c >> 6) & 0x3fu));
            out[used++] = (char)(0x80u | (c & 0x3fu));
        } else if (c <= 0x10ffffu) {
            out[used++] = (char)(0xf0u | (c >> 18));
            out[used++] = (char)(0x80u | ((c >> 12) & 0x3fu));
            out[used++] = (char)(0x80u | ((c >> 6) & 0x3fu));
            out[used++] = (char)(0x80u | (c & 0x3fu));
        } else {
            out[used++] = '?';
        }
    }
    out[used] = '\0';
    return out;
}

static int write_codepoint(FILE* stream, uint32_t c) {
    if (c <= 0x7fu) return fputc((int)c, stream) == EOF ? -1 : 0;
    if (c <= 0x7ffu) {
        if (fputc((int)(0xc0u | (c >> 6)), stream) == EOF) return -1;
        return fputc((int)(0x80u | (c & 0x3fu)), stream) == EOF ? -1 : 0;
    }
    if (c <= 0xffffu && !(c >= 0xd800u && c <= 0xdfffu)) {
        if (fputc((int)(0xe0u | (c >> 12)), stream) == EOF) return -1;
        if (fputc((int)(0x80u | ((c >> 6) & 0x3fu)), stream) == EOF) return -1;
        return fputc((int)(0x80u | (c & 0x3fu)), stream) == EOF ? -1 : 0;
    }
    if (c <= 0x10ffffu) {
        if (fputc((int)(0xf0u | (c >> 18)), stream) == EOF) return -1;
        if (fputc((int)(0x80u | ((c >> 12) & 0x3fu)), stream) == EOF) return -1;
        if (fputc((int)(0x80u | ((c >> 6) & 0x3fu)), stream) == EOF) return -1;
        return fputc((int)(0x80u | (c & 0x3fu)), stream) == EOF ? -1 : 0;
    }
    return fputc('?', stream) == EOF ? -1 : 0;
}

static int write_que_string_to_stream(struct w2c_host* host, uint32_t vector,
                                      FILE* stream) {
    uint32_t length = load_u32(host, vector);
    uint32_t data = load_u32(host, vector + 16u);
    if ((uint64_t)data + (uint64_t)length * 4u > host->instance->w2c_memory.size) {
        que_host_fail("string", "invalid Que string data");
    }
    for (uint32_t i = 0; i < length; ++i) {
        if (write_codepoint(stream, load_u32(host, data + i * 4u)) != 0) return -1;
    }
    return 0;
}

static uint32_t next_utf8(const unsigned char* bytes, size_t length, size_t* at) {
    unsigned char c = bytes[(*at)++];
    if (c < 0x80u) return c;
    if ((c & 0xe0u) == 0xc0u && *at < length) {
        uint32_t value = ((uint32_t)(c & 0x1fu) << 6) | (bytes[(*at)++] & 0x3fu);
        return value >= 0x80u ? value : 0xfffdu;
    }
    if ((c & 0xf0u) == 0xe0u && *at + 1u < length) {
        uint32_t value = ((uint32_t)(c & 0x0fu) << 12) |
                         ((uint32_t)(bytes[(*at)++] & 0x3fu) << 6) |
                         (bytes[(*at)++] & 0x3fu);
        return value >= 0x800u ? value : 0xfffdu;
    }
    if ((c & 0xf8u) == 0xf0u && *at + 2u < length) {
        uint32_t value = ((uint32_t)(c & 0x07u) << 18) |
                         ((uint32_t)(bytes[(*at)++] & 0x3fu) << 12) |
                         ((uint32_t)(bytes[(*at)++] & 0x3fu) << 6) |
                         (bytes[(*at)++] & 0x3fu);
        return value >= 0x10000u && value <= 0x10ffffu ? value : 0xfffdu;
    }
    return 0xfffdu;
}

static uint32_t write_que_bytes(struct w2c_host* host,
                                const unsigned char* bytes, size_t length) {
    uint32_t vector = w2c_main_make_vec(host->instance, 0);
    size_t at = 0;
    while (at < length) {
        uint32_t codepoint = next_utf8(bytes, length, &at);
        (void)w2c_main_vec_push(host->instance, vector, codepoint);
    }
    return vector;
}

static uint32_t write_que_raw_bytes(struct w2c_host* host,
                                    const unsigned char* bytes, size_t length) {
    uint32_t vector = w2c_main_make_vec(host->instance, 0);
    for (size_t i = 0; i < length; ++i) {
        (void)w2c_main_vec_push(host->instance, vector, bytes[i]);
    }
    return vector;
}

static uint32_t write_que_string(struct w2c_host* host, const char* text) {
    return write_que_bytes(host, (const unsigned char*)text, strlen(text));
}

static int unsafe_path(const char* path) {
    if (path[0] == '/' || path[0] == '\\') return 1;
#if defined(_WIN32)
    if (isalpha((unsigned char)path[0]) && path[1] == ':') return 1;
#endif
    const char* p = path;
    while (*p != '\0') {
        while (*p == '/' || *p == '\\') ++p;
        const char* start = p;
        while (*p != '\0' && *p != '/' && *p != '\\') ++p;
        if ((p - start) == 2 && start[0] == '.' && start[1] == '.') return 1;
    }
    return 0;
}

static char* checked_path(struct w2c_host* host, uint32_t path_vector,
                          const char* operation) {
    char* path = read_que_string(host, path_vector);
    if (unsafe_path(path)) {
        free(path);
        que_host_fail(operation, "path must stay beneath the process working directory");
    }
#if !defined(_WIN32)
    char prefix[PATH_MAX];
    size_t used = 0;
    const char* p = path;
    while (*p != '\0') {
        while (*p == '/') ++p;
        const char* start = p;
        while (*p != '\0' && *p != '/') ++p;
        size_t part = (size_t)(p - start);
        if (part == 0) break;
        if (used != 0) prefix[used++] = '/';
        if (used + part >= sizeof(prefix)) {
            free(path);
            que_host_fail(operation, "path is too long");
        }
        memcpy(prefix + used, start, part);
        used += part;
        prefix[used] = '\0';
        struct stat info;
        if (lstat(prefix, &info) == 0 && S_ISLNK(info.st_mode)) {
            free(path);
            que_host_fail(operation, "symbolic links are not allowed in sandboxed paths");
        }
    }
#endif
    return path;
}

static uint32_t permission_for_name(const char* name, size_t length) {
#define IS_PERMISSION(text, value) \
    if (length == sizeof(text) - 1u && strncmp(name, text, length) == 0) return value
    IS_PERMISSION("read", QUE_HOST_READ);
    IS_PERMISSION("stdin", QUE_HOST_STDIN);
    IS_PERMISSION("write", QUE_HOST_WRITE);
    IS_PERMISSION("print", QUE_HOST_PRINT);
    IS_PERMISSION("clock", QUE_HOST_CLOCK);
    IS_PERMISSION("delete", QUE_HOST_DELETE);
    IS_PERMISSION("all", QUE_HOST_ALL);
#undef IS_PERMISSION
    return 0;
}

uint32_t que_host_parse_permissions(const char* value) {
    uint32_t permissions = 0;
    if (value == NULL) return permissions;
    while (*value != '\0') {
        while (*value == ',' || isspace((unsigned char)*value)) ++value;
        const char* start = value;
        while (*value != '\0' && *value != ',' && !isspace((unsigned char)*value)) ++value;
        if (value != start) permissions |= permission_for_name(start, (size_t)(value - start));
    }
    return permissions;
}

void que_host_init(struct w2c_host* host, struct w2c_main* instance,
                   uint32_t permissions) {
    host->instance = instance;
    host->permissions = permissions;
}

int que_host_configure_argv(struct w2c_host* host, int argc, char** argv) {
    (void)w2c_main_argv_clear(host->instance);
    int literal_args = 0;
    for (int i = 1; i < argc; ++i) {
        const char* arg = argv[i];
        if (!literal_args && strcmp(arg, "--") == 0) {
            literal_args = 1;
            continue;
        }
        if (!literal_args && strncmp(arg, "--allow=", 8) == 0) {
            host->permissions |= que_host_parse_permissions(arg + 8);
            continue;
        }
        if (!literal_args && strcmp(arg, "--allow") == 0) {
            int found = 0;
            while (i + 1 < argc) {
                const char* candidate = argv[i + 1];
                uint32_t permission = permission_for_name(candidate, strlen(candidate));
                if (permission == 0) break;
                host->permissions |= permission;
                ++i;
                found = 1;
            }
            if (!found) {
                fprintf(stderr, "que native host: --allow requires one or more permissions\n");
                return -1;
            }
            continue;
        }
        uint32_t value = write_que_string(host, arg);
        (void)w2c_main_argv_push(host->instance, value);
        (void)w2c_main_rc_release(host->instance, value);
    }
    return 0;
}

uint32_t w2c_host_print(struct w2c_host* host, uint32_t value) {
    require_permission(host, QUE_HOST_PRINT, "print!");
    if (write_que_string_to_stream(host, value, stdout) != 0 || fflush(stdout) != 0) {
        que_host_fail("print!", "failed while writing stdout");
    }
    return 0;
}

uint32_t w2c_host_read_file(struct w2c_host* host, uint32_t path_vector) {
    require_permission(host, QUE_HOST_READ, "read!");
    char* path = checked_path(host, path_vector, "read!");
    FILE* file = fopen(path, "rb");
    if (file == NULL) {
        char message[256];
        snprintf(message, sizeof(message), "cannot open '%s': %s", path, strerror(errno));
        free(path);
        que_host_fail("read!", message);
    }
    free(path);
    if (fseek(file, 0, SEEK_END) != 0) que_host_fail("read!", "cannot seek file");
    long file_size = ftell(file);
    if (file_size < 0 || fseek(file, 0, SEEK_SET) != 0) que_host_fail("read!", "cannot size file");
    unsigned char* bytes = (unsigned char*)malloc((size_t)file_size + 1u);
    if (bytes == NULL) que_host_fail("read!", "out of host memory");
    size_t count = fread(bytes, 1, (size_t)file_size, file);
    if (ferror(file)) que_host_fail("read!", "failed while reading file");
    fclose(file);
    uint32_t result = write_que_bytes(host, bytes, count);
    free(bytes);
    return result;
}

uint32_t w2c_host_read_stdin(struct w2c_host* host) {
    require_permission(host, QUE_HOST_STDIN, "stdin!");
    size_t length = 0, capacity = 4096;
    unsigned char* bytes = (unsigned char*)malloc(capacity);
    if (bytes == NULL) que_host_fail("stdin!", "out of host memory");
    for (;;) {
        if (length == capacity) {
            capacity *= 2u;
            unsigned char* grown = (unsigned char*)realloc(bytes, capacity);
            if (grown == NULL) que_host_fail("stdin!", "out of host memory");
            bytes = grown;
        }
        size_t count = fread(bytes + length, 1, capacity - length, stdin);
        length += count;
        if (count == 0) break;
    }
    uint32_t result = write_que_bytes(host, bytes, length);
    free(bytes);
    return result;
}

uint32_t w2c_host_write_file(struct w2c_host* host, uint32_t path_vector,
                             uint32_t data_vector) {
    require_permission(host, QUE_HOST_WRITE, "write!");
    char* path = checked_path(host, path_vector, "write!");
    FILE* file = fopen(path, "wb");
    if (file == NULL) {
        char message[256];
        snprintf(message, sizeof(message), "cannot open '%s': %s", path, strerror(errno));
        free(path);
        que_host_fail("write!", message);
    }
    int write_failed = write_que_string_to_stream(host, data_vector, file) != 0;
    int close_failed = fclose(file) != 0;
    if (write_failed || close_failed) {
        free(path);
        que_host_fail("write!", "failed while writing file");
    }
    free(path);
    return 0;
}

uint32_t w2c_host_mkdir_p(struct w2c_host* host, uint32_t path_vector) {
    require_permission(host, QUE_HOST_WRITE, "mkdir!");
    char* path = checked_path(host, path_vector, "mkdir!");
    char* p = path;
    for (; *p != '\0'; ++p) {
        if ((*p == '/' || *p == '\\') && p != path) {
            char saved = *p;
            *p = '\0';
            if (QUE_MKDIR(path) != 0 && errno != EEXIST) que_host_fail("mkdir!", strerror(errno));
            *p = saved;
        }
    }
    if (QUE_MKDIR(path) != 0 && errno != EEXIST) que_host_fail("mkdir!", strerror(errno));
    free(path);
    return 0;
}

uint32_t w2c_host_delete(struct w2c_host* host, uint32_t path_vector) {
    require_permission(host, QUE_HOST_DELETE, "delete!");
    char* path = checked_path(host, path_vector, "delete!");
    if (remove(path) != 0 && QUE_RMDIR(path) != 0) que_host_fail("delete!", strerror(errno));
    free(path);
    return 0;
}

uint32_t w2c_host_move(struct w2c_host* host, uint32_t from_vector, uint32_t to_vector) {
    require_permission(host, QUE_HOST_WRITE, "move!");
    char* from = checked_path(host, from_vector, "move!");
    char* to = checked_path(host, to_vector, "move!");
    if (rename(from, to) != 0) que_host_fail("move!", strerror(errno));
    free(from);
    free(to);
    return 0;
}

uint32_t w2c_host_sleep(struct w2c_host* host, uint32_t milliseconds) {
    require_permission(host, QUE_HOST_CLOCK, "sleep!");
#if defined(_WIN32)
    Sleep(milliseconds);
#else
    struct timespec duration = {(time_t)(milliseconds / 1000u),
                                (long)(milliseconds % 1000u) * 1000000L};
    while (nanosleep(&duration, &duration) != 0 && errno == EINTR) {}
#endif
    return 0;
}

uint32_t w2c_host_time(struct w2c_host* host) {
    require_permission(host, QUE_HOST_CLOCK, "time!");
    struct timespec now;
#if defined(_WIN32)
    timespec_get(&now, TIME_UTC);
#else
    clock_gettime(CLOCK_REALTIME, &now);
#endif
    uint64_t milliseconds = (uint64_t)now.tv_sec * 1000u + (uint64_t)now.tv_nsec / 1000000u;
    return (uint32_t)milliseconds;
}

uint32_t w2c_host_random(struct w2c_host* host) {
    require_permission(host, QUE_HOST_CLOCK, "random!");
    uint32_t value = 0;
#if defined(_WIN32)
    if (rand_s(&value) != 0) que_host_fail("random!", "OS random source failed");
#else
    FILE* source = fopen("/dev/urandom", "rb");
    if (source == NULL || fread(&value, sizeof(value), 1, source) != 1) {
        if (source != NULL) fclose(source);
        que_host_fail("random!", "OS random source failed");
    }
    fclose(source);
#endif
    return value;
}

uint32_t w2c_host_clear(struct w2c_host* host) {
    require_permission(host, QUE_HOST_PRINT, "clear!");
    fputs("\x1b[2J\x1b[H", stdout);
    fflush(stdout);
    return 0;
}

static int compare_names(const void* left, const void* right) {
    const char* const* a = (const char* const*)left;
    const char* const* b = (const char* const*)right;
    return strcmp(*a, *b);
}

static char* copy_name(const char* value) {
    size_t length = strlen(value) + 1u;
    char* copy = (char*)malloc(length);
    if (copy != NULL) memcpy(copy, value, length);
    return copy;
}

uint32_t w2c_host_list_dir(struct w2c_host* host, uint32_t path_vector) {
    require_permission(host, QUE_HOST_READ, "list-dir!");
    char* path = checked_path(host, path_vector, "list-dir!");
    uint32_t result = w2c_main_make_vec(host->instance, 1);
#if defined(_WIN32)
    size_t pattern_length = strlen(path) + 3u;
    char* pattern = (char*)malloc(pattern_length);
    if (pattern == NULL) que_host_fail("list-dir!", "out of host memory");
    snprintf(pattern, pattern_length, "%s\\*", path);
    WIN32_FIND_DATAA found;
    HANDLE search = FindFirstFileA(pattern, &found);
    free(pattern);
    if (search == INVALID_HANDLE_VALUE) que_host_fail("list-dir!", "cannot read directory");
    char** names = NULL;
    size_t count = 0, capacity = 0;
    do {
        if (strcmp(found.cFileName, ".") == 0 || strcmp(found.cFileName, "..") == 0) continue;
        if (count == capacity) {
            capacity = capacity == 0 ? 16u : capacity * 2u;
            char** grown = (char**)realloc(names, capacity * sizeof(char*));
            if (grown == NULL) que_host_fail("list-dir!", "out of host memory");
            names = grown;
        }
        names[count] = copy_name(found.cFileName);
        if (names[count] == NULL) que_host_fail("list-dir!", "out of host memory");
        ++count;
    } while (FindNextFileA(search, &found));
    FindClose(search);
#else
    DIR* directory = opendir(path);
    if (directory == NULL) que_host_fail("list-dir!", strerror(errno));
    char** names = NULL;
    size_t count = 0, capacity = 0;
    struct dirent* entry;
    while ((entry = readdir(directory)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) continue;
        if (count == capacity) {
            capacity = capacity == 0 ? 16u : capacity * 2u;
            char** grown = (char**)realloc(names, capacity * sizeof(char*));
            if (grown == NULL) que_host_fail("list-dir!", "out of host memory");
            names = grown;
        }
        names[count] = copy_name(entry->d_name);
        if (names[count] == NULL) que_host_fail("list-dir!", "out of host memory");
        ++count;
    }
    closedir(directory);
#endif
    qsort(names, count, sizeof(char*), compare_names);
    for (size_t i = 0; i < count; ++i) {
        uint32_t name = write_que_string(host, names[i]);
        (void)w2c_main_vec_push(host->instance, result, name);
        (void)w2c_main_rc_release(host->instance, name);
        free(names[i]);
    }
    free(names);
    free(path);
    return result;
}

static uint32_t stream_chunks(struct w2c_host* host, FILE* stream,
                              uint32_t chunk_size, uint32_t callback,
                              const char* operation) {
    if (chunk_size == 0 || chunk_size > INT32_MAX) {
        que_host_fail(operation, "chunk size must be positive");
    }
    unsigned char* buffer = (unsigned char*)malloc(chunk_size);
    if (buffer == NULL) que_host_fail(operation, "out of host memory");
    for (;;) {
        size_t count = fread(buffer, 1, chunk_size, stream);
        if (count == 0) {
            if (ferror(stream)) que_host_fail(operation, "failed while reading stream");
            break;
        }
        uint32_t chunk = write_que_raw_bytes(host, buffer, count);
        uint32_t stop = w2c_main_apply1_i32(host->instance, callback, chunk);
        (void)w2c_main_rc_release(host->instance, chunk);
        if (stop != 0) {
            free(buffer);
            return 1;
        }
    }
    free(buffer);
    return 0;
}

uint32_t w2c_host_read_chunks(struct w2c_host* host, uint32_t path,
                              uint32_t size, uint32_t callback) {
    require_permission(host, QUE_HOST_READ, "read/chunks!");
    char* filename = checked_path(host, path, "read/chunks!");
    FILE* file = fopen(filename, "rb");
    if (file == NULL) que_host_fail("read/chunks!", strerror(errno));
    uint32_t result = stream_chunks(host, file, size, callback, "read/chunks!");
    fclose(file);
    free(filename);
    return result;
}

uint32_t w2c_host_read_stdin_chunks(struct w2c_host* host, uint32_t size,
                                    uint32_t callback) {
    require_permission(host, QUE_HOST_STDIN, "stdin/chunks!");
    return stream_chunks(host, stdin, size, callback, "stdin/chunks!");
}

uint32_t w2c_host_read_lines(struct w2c_host* host, uint32_t path,
                             uint32_t callback) {
    require_permission(host, QUE_HOST_READ, "read/lines!");
    char* filename = checked_path(host, path, "read/lines!");
    FILE* file = fopen(filename, "rb");
    if (file == NULL) que_host_fail("read/lines!", strerror(errno));
    size_t length = 0, capacity = 256;
    unsigned char* line = (unsigned char*)malloc(capacity);
    if (line == NULL) que_host_fail("read/lines!", "out of host memory");
    uint32_t result = 0;
    for (;;) {
        int c = fgetc(file);
        if (c == EOF || c == '\n') {
            if (length > 0 && line[length - 1] == '\r') --length;
            if (length > 0 || c == '\n') {
                uint32_t value = write_que_raw_bytes(host, line, length);
                uint32_t stop = w2c_main_apply1_i32(host->instance, callback, value);
                (void)w2c_main_rc_release(host->instance, value);
                if (stop != 0) { result = 1; break; }
            }
            length = 0;
            if (c == EOF) break;
            continue;
        }
        if (length == capacity) {
            capacity *= 2u;
            unsigned char* grown = (unsigned char*)realloc(line, capacity);
            if (grown == NULL) que_host_fail("read/lines!", "out of host memory");
            line = grown;
        }
        line[length++] = (unsigned char)c;
    }
    if (ferror(file)) que_host_fail("read/lines!", "failed while reading file");
    free(line);
    fclose(file);
    free(filename);
    return result;
}

enum que_type_kind {
    QUE_TYPE_UNKNOWN, QUE_TYPE_INT, QUE_TYPE_DEC, QUE_TYPE_BOOL,
    QUE_TYPE_CHAR, QUE_TYPE_UNIT, QUE_TYPE_VECTOR, QUE_TYPE_TUPLE
};

struct que_type {
    enum que_type_kind kind;
    struct que_type* child;
    struct que_type** parts;
    size_t part_count;
};

struct text_buffer {
    char* data;
    size_t length;
    size_t capacity;
};

static void buffer_reserve(struct text_buffer* buffer, size_t extra) {
    if (buffer->length + extra + 1u <= buffer->capacity) return;
    size_t capacity = buffer->capacity == 0 ? 64u : buffer->capacity;
    while (capacity < buffer->length + extra + 1u) capacity *= 2u;
    char* grown = (char*)realloc(buffer->data, capacity);
    if (grown == NULL) que_host_fail("serialize", "out of host memory");
    buffer->data = grown;
    buffer->capacity = capacity;
}

static void buffer_bytes(struct text_buffer* buffer, const char* value, size_t length) {
    buffer_reserve(buffer, length);
    memcpy(buffer->data + buffer->length, value, length);
    buffer->length += length;
    buffer->data[buffer->length] = '\0';
}

static void buffer_text(struct text_buffer* buffer, const char* value) {
    buffer_bytes(buffer, value, strlen(value));
}

static void buffer_char(struct text_buffer* buffer, char value) {
    buffer_bytes(buffer, &value, 1);
}

static void skip_spaces(const char** input) {
    while (isspace((unsigned char)**input)) ++*input;
}

static struct que_type* new_type(enum que_type_kind kind) {
    struct que_type* type = (struct que_type*)calloc(1, sizeof(*type));
    if (type == NULL) que_host_fail("serialization type", "out of host memory");
    type->kind = kind;
    return type;
}

static void free_type(struct que_type* type) {
    if (type == NULL) return;
    free_type(type->child);
    for (size_t i = 0; i < type->part_count; ++i) free_type(type->parts[i]);
    free(type->parts);
    free(type);
}

static int consume_word(const char** input, const char* word) {
    size_t length = strlen(word);
    if (strncmp(*input, word, length) != 0) return 0;
    *input += length;
    return 1;
}

static struct que_type* parse_type_node(const char** input) {
    skip_spaces(input);
    if (consume_word(input, "Int")) return new_type(QUE_TYPE_INT);
    if (consume_word(input, "Dec")) return new_type(QUE_TYPE_DEC);
    if (consume_word(input, "Bool")) return new_type(QUE_TYPE_BOOL);
    if (consume_word(input, "Char")) return new_type(QUE_TYPE_CHAR);
    if (consume_word(input, "()")) return new_type(QUE_TYPE_UNIT);
    if (**input == 'T' && isdigit((unsigned char)(*input)[1])) {
        ++*input;
        while (isdigit((unsigned char)**input)) ++*input;
        return new_type(QUE_TYPE_UNKNOWN);
    }
    if (**input == '[') {
        ++*input;
        struct que_type* type = new_type(QUE_TYPE_VECTOR);
        type->child = parse_type_node(input);
        skip_spaces(input);
        if (**input != ']') que_host_fail("serialization type", "expected ']'");
        ++*input;
        return type;
    }
    if (**input == '{') {
        ++*input;
        struct que_type* type = new_type(QUE_TYPE_TUPLE);
        for (;;) {
            skip_spaces(input);
            if (**input == '}') { ++*input; break; }
            struct que_type* part = parse_type_node(input);
            struct que_type** grown = (struct que_type**)realloc(
                type->parts, (type->part_count + 1u) * sizeof(*type->parts));
            if (grown == NULL) que_host_fail("serialization type", "out of host memory");
            type->parts = grown;
            type->parts[type->part_count++] = part;
            skip_spaces(input);
            if (**input == '*') ++*input;
            else if (**input == '\0') que_host_fail("serialization type", "expected '}'");
        }
        return type;
    }
    que_host_fail("serialization type", "unsupported type");
    return NULL;
}

static struct que_type* parse_type(struct w2c_host* host, uint32_t pointer) {
    char* text = read_que_string(host, pointer);
    const char* input = text;
    struct que_type* type = parse_type_node(&input);
    skip_spaces(&input);
    if (*input != '\0') que_host_fail("serialization type", "unexpected trailing text");
    free(text);
    return type;
}

static struct que_type* parse_type_text(const char* text) {
    const char* input = text;
    struct que_type* type = parse_type_node(&input);
    skip_spaces(&input);
    if (*input != '\0') que_host_fail("serialization type", "unexpected trailing text");
    return type;
}

static int type_is_managed(const struct que_type* type) {
    return type->kind == QUE_TYPE_VECTOR || type->kind == QUE_TYPE_TUPLE;
}

static void render_value(struct w2c_host* host, uint32_t value,
                         const struct que_type* type, struct text_buffer* out);

static void render_escaped_string(struct w2c_host* host, uint32_t vector,
                                  struct text_buffer* out) {
    uint32_t length = load_u32(host, vector);
    uint32_t data = load_u32(host, vector + 16u);
    buffer_char(out, '"');
    for (uint32_t i = 0; i < length; ++i) {
        uint32_t c = load_u32(host, data + i * 4u);
        switch (c) {
            case '\\': buffer_text(out, "\\\\"); break;
            case '"': buffer_text(out, "\\\""); break;
            case '\n': buffer_text(out, "\\n"); break;
            case '\r': buffer_text(out, "\\r"); break;
            case '\t': buffer_text(out, "\\t"); break;
            case 0: buffer_text(out, "\\0"); break;
            default: {
                char encoded[4];
                size_t n = 0;
                if (c <= 0x7f) encoded[n++] = (char)c;
                else if (c <= 0x7ff) {
                    encoded[n++] = (char)(0xc0 | (c >> 6));
                    encoded[n++] = (char)(0x80 | (c & 0x3f));
                } else if (c <= 0xffff) {
                    encoded[n++] = (char)(0xe0 | (c >> 12));
                    encoded[n++] = (char)(0x80 | ((c >> 6) & 0x3f));
                    encoded[n++] = (char)(0x80 | (c & 0x3f));
                } else {
                    encoded[n++] = (char)(0xf0 | (c >> 18));
                    encoded[n++] = (char)(0x80 | ((c >> 12) & 0x3f));
                    encoded[n++] = (char)(0x80 | ((c >> 6) & 0x3f));
                    encoded[n++] = (char)(0x80 | (c & 0x3f));
                }
                buffer_bytes(out, encoded, n);
            }
        }
    }
    buffer_char(out, '"');
}

static void render_value(struct w2c_host* host, uint32_t value,
                         const struct que_type* type, struct text_buffer* out) {
    char number[96];
    switch (type->kind) {
        case QUE_TYPE_INT:
            snprintf(number, sizeof(number), "%d", (int32_t)value);
            buffer_text(out, number);
            break;
        case QUE_TYPE_DEC: {
            long scale = 1000;
            const char* configured = getenv("QUE_DECIMAL_SCALE");
            if (configured != NULL && strtol(configured, NULL, 10) > 0) scale = strtol(configured, NULL, 10);
            int64_t signed_value = (int32_t)value;
            uint64_t absolute = signed_value < 0 ? (uint64_t)(-signed_value) : (uint64_t)signed_value;
            unsigned digits = 0;
            for (long n = scale; n > 1; n /= 10) ++digits;
            snprintf(number, sizeof(number), "%s%llu.%0*llu", signed_value < 0 ? "-" : "",
                     (unsigned long long)(absolute / (uint64_t)scale), (int)digits,
                     (unsigned long long)(absolute % (uint64_t)scale));
            while (strchr(number, '.') != NULL && number[strlen(number) - 1] == '0') number[strlen(number) - 1] = '\0';
            if (number[strlen(number) - 1] == '.') number[strlen(number) - 1] = '\0';
            buffer_text(out, number);
            break;
        }
        case QUE_TYPE_BOOL: buffer_text(out, value == 0 ? "false" : "true"); break;
        case QUE_TYPE_CHAR:
            snprintf(number, sizeof(number), "(char %u)", value);
            buffer_text(out, number);
            break;
        case QUE_TYPE_UNIT: buffer_text(out, "nil"); break;
        case QUE_TYPE_UNKNOWN: que_host_fail("serialize", "unresolved value type"); break;
        case QUE_TYPE_VECTOR: {
            if (type->child->kind == QUE_TYPE_CHAR) {
                render_escaped_string(host, value, out);
                break;
            }
            uint32_t length = load_u32(host, value);
            uint32_t data = load_u32(host, value + 16u);
            buffer_char(out, '[');
            for (uint32_t i = 0; i < length; ++i) {
                if (i != 0) buffer_char(out, ' ');
                render_value(host, load_u32(host, data + i * 4u), type->child, out);
            }
            buffer_char(out, ']');
            break;
        }
        case QUE_TYPE_TUPLE: {
            uint32_t length = load_u32(host, value);
            uint32_t data = load_u32(host, value + 16u);
            if (length != type->part_count) que_host_fail("serialize", "tuple shape does not match type");
            buffer_text(out, "{ ");
            for (uint32_t i = 0; i < length; ++i) {
                if (i != 0) buffer_char(out, ' ');
                render_value(host, load_u32(host, data + i * 4u), type->parts[i], out);
            }
            buffer_text(out, " }");
            break;
        }
    }
}

uint32_t w2c_host_serialize(struct w2c_host* host, uint32_t value, uint32_t type_pointer) {
    struct que_type* type = parse_type(host, type_pointer);
    struct text_buffer rendered = {0};
    render_value(host, value, type, &rendered);
    uint32_t result = write_que_bytes(host, (unsigned char*)rendered.data, rendered.length);
    free(rendered.data);
    free_type(type);
    return result;
}

void que_host_print_result(struct w2c_host* host, uint32_t value,
                           const char* type_text) {
    struct que_type* type = parse_type_text(type_text);
    struct text_buffer rendered = {0};
    render_value(host, value, type, &rendered);
    if (rendered.length != 0) fwrite(rendered.data, 1, rendered.length, stdout);
    fputc('\n', stdout);
    free(rendered.data);
    free_type(type);
}

struct literal_parser { const char* input; struct w2c_host* host; };

static int64_t parse_integer(struct literal_parser* parser) {
    skip_spaces(&parser->input);
    char* end = NULL;
    errno = 0;
    long long value = strtoll(parser->input, &end, 10);
    if (end == parser->input || errno == ERANGE) que_host_fail("deserialize", "expected integer literal");
    parser->input = end;
    return value;
}

static uint32_t parse_literal_value(struct literal_parser* parser,
                                    const struct que_type* type) {
    skip_spaces(&parser->input);
    switch (type->kind) {
        case QUE_TYPE_INT: {
            int64_t value = parse_integer(parser);
            if (value < INT32_MIN || value > INT32_MAX) que_host_fail("deserialize", "Int literal out of range");
            return (uint32_t)(int32_t)value;
        }
        case QUE_TYPE_DEC: {
            char* end = NULL;
            double value = strtod(parser->input, &end);
            if (end == parser->input) que_host_fail("deserialize", "expected Dec literal");
            parser->input = end;
            long scale = 1000;
            const char* configured = getenv("QUE_DECIMAL_SCALE");
            if (configured != NULL && strtol(configured, NULL, 10) > 0) scale = strtol(configured, NULL, 10);
            double scaled = value * (double)scale;
            if (scaled < INT32_MIN || scaled > INT32_MAX) que_host_fail("deserialize", "Dec literal out of range");
            return (uint32_t)(int32_t)(scaled >= 0 ? scaled + 0.5 : scaled - 0.5);
        }
        case QUE_TYPE_BOOL:
            if (consume_word(&parser->input, "true")) return 1;
            if (consume_word(&parser->input, "false")) return 0;
            que_host_fail("deserialize", "expected Bool literal");
            return 0;
        case QUE_TYPE_UNIT:
            if (!consume_word(&parser->input, "nil")) que_host_fail("deserialize", "expected nil");
            return 0;
        case QUE_TYPE_CHAR:
            if (!consume_word(&parser->input, "(char")) que_host_fail("deserialize", "expected Char literal");
            { int64_t value = parse_integer(parser); skip_spaces(&parser->input);
              if (*parser->input++ != ')') que_host_fail("deserialize", "expected ')' after Char");
              return (uint32_t)value; }
        case QUE_TYPE_UNKNOWN:
            que_host_fail("deserialize", "unresolved value type");
            return 0;
        case QUE_TYPE_VECTOR: {
            uint32_t out = w2c_main_make_vec(parser->host->instance, type_is_managed(type->child));
            if (type->child->kind == QUE_TYPE_CHAR) {
                if (*parser->input++ != '"') que_host_fail("deserialize", "expected string literal");
                while (*parser->input != '"') {
                    if (*parser->input == '\0') que_host_fail("deserialize", "unterminated string literal");
                    uint32_t c;
                    if (*parser->input == '\\') {
                        ++parser->input;
                        switch (*parser->input++) {
                            case 'n': c = '\n'; break; case 'r': c = '\r'; break;
                            case 't': c = '\t'; break; case '0': c = 0; break;
                            case '\\': c = '\\'; break; case '"': c = '"'; break;
                            default: que_host_fail("deserialize", "unknown string escape"); c = 0;
                        }
                    } else {
                        const unsigned char* bytes = (const unsigned char*)parser->input;
                        size_t remaining = strlen(parser->input), at = 0;
                        c = next_utf8(bytes, remaining, &at);
                        parser->input += at;
                    }
                    (void)w2c_main_vec_push(parser->host->instance, out, c);
                }
                ++parser->input;
                return out;
            }
            if (*parser->input++ != '[') que_host_fail("deserialize", "expected vector literal");
            for (;;) {
                skip_spaces(&parser->input);
                if (*parser->input == ']') { ++parser->input; break; }
                uint32_t value = parse_literal_value(parser, type->child);
                (void)w2c_main_vec_push(parser->host->instance, out, value);
                if (type_is_managed(type->child)) (void)w2c_main_rc_release(parser->host->instance, value);
            }
            return out;
        }
        case QUE_TYPE_TUPLE: {
            if (*parser->input++ != '{') que_host_fail("deserialize", "expected tuple literal");
            uint32_t out = w2c_main_make_vec(parser->host->instance, 1);
            for (size_t i = 0; i < type->part_count; ++i) {
                uint32_t value = parse_literal_value(parser, type->parts[i]);
                (void)w2c_main_vec_push(parser->host->instance, out, value);
                if (type_is_managed(type->parts[i])) (void)w2c_main_rc_release(parser->host->instance, value);
            }
            skip_spaces(&parser->input);
            if (*parser->input++ != '}') que_host_fail("deserialize", "tuple shape does not match type");
            return out;
        }
    }
    return 0;
}

uint32_t w2c_host_deserialize(struct w2c_host* host, uint32_t text_pointer,
                              uint32_t type_pointer) {
    char* source = read_que_string(host, text_pointer);
    struct que_type* type = parse_type(host, type_pointer);
    struct literal_parser parser = {source, host};
    uint32_t result = parse_literal_value(&parser, type);
    skip_spaces(&parser.input);
    if (*parser.input != '\0') que_host_fail("deserialize", "expected exactly one literal");
    free_type(type);
    free(source);
    return result;
}
