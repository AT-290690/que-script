#define _POSIX_C_SOURCE 200809L

#include <dirent.h>
#include <errno.h>
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#include "__HEADER__"
#include "wasm-rt.h"

#ifndef PATH_MAX
#define PATH_MAX 4096
#endif

#define VEC_LEN_OFFSET 0u
#define VEC_CAP_OFFSET 4u
#define VEC_RC_OFFSET 8u
#define VEC_ELEM_REF_OFFSET 12u
#define VEC_DATA_PTR_OFFSET 16u
#define VEC_MAGIC_OFFSET 20u
#define VEC_HEADER_SIZE 24u
#define VEC_MAGIC 1447380017u

struct w2c_host {
  w2c___MODULE__* instance;
};

extern wasm_rt_jmp_buf g_wasm_rt_jmp_buf;

static void host_fail(const char* message) {
  fprintf(stderr, "host io error: %s\n", message);
  fflush(stderr);
  wasm_rt_trap(WASM_RT_TRAP_UNREACHABLE);
}

static void* xmalloc(size_t size) {
  void* ptr = malloc(size == 0 ? 1 : size);
  if (ptr == NULL) {
    host_fail("malloc failed");
  }
  return ptr;
}

static void* xrealloc(void* ptr, size_t size) {
  void* next = realloc(ptr, size == 0 ? 1 : size);
  if (next == NULL) {
    free(ptr);
    host_fail("realloc failed");
  }
  return next;
}

static char* xstrdup(const char* text) {
  size_t len = strlen(text);
  char* out = (char*)xmalloc(len + 1);
  memcpy(out, text, len + 1);
  return out;
}

static uint32_t load_u32(w2c___MODULE__* instance, uint32_t addr) {
  uint8_t* p = instance->w2c_memory.data + addr;
  return ((uint32_t)p[0]) | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) |
         ((uint32_t)p[3] << 24);
}

static void store_u32(w2c___MODULE__* instance, uint32_t addr, uint32_t value) {
  uint8_t* p = instance->w2c_memory.data + addr;
  p[0] = (uint8_t)(value & 0xffu);
  p[1] = (uint8_t)((value >> 8) & 0xffu);
  p[2] = (uint8_t)((value >> 16) & 0xffu);
  p[3] = (uint8_t)((value >> 24) & 0xffu);
}

static void encode_utf8(uint32_t codepoint, unsigned char* out, size_t* written) {
  if (codepoint <= 0x7fu) {
    out[0] = (unsigned char)codepoint;
    *written = 1;
  } else if (codepoint <= 0x7ffu) {
    out[0] = (unsigned char)(0xc0u | (codepoint >> 6));
    out[1] = (unsigned char)(0x80u | (codepoint & 0x3fu));
    *written = 2;
  } else if (codepoint <= 0xffffu) {
    out[0] = (unsigned char)(0xe0u | (codepoint >> 12));
    out[1] = (unsigned char)(0x80u | ((codepoint >> 6) & 0x3fu));
    out[2] = (unsigned char)(0x80u | (codepoint & 0x3fu));
    *written = 3;
  } else if (codepoint <= 0x10ffffu) {
    out[0] = (unsigned char)(0xf0u | (codepoint >> 18));
    out[1] = (unsigned char)(0x80u | ((codepoint >> 12) & 0x3fu));
    out[2] = (unsigned char)(0x80u | ((codepoint >> 6) & 0x3fu));
    out[3] = (unsigned char)(0x80u | (codepoint & 0x3fu));
    *written = 4;
  } else {
    encode_utf8(0xfffdu, out, written);
  }
}

static uint32_t decode_utf8_one(const unsigned char* bytes,
                                size_t len,
                                size_t* advance) {
  if (len == 0) {
    *advance = 0;
    return 0xfffdu;
  }

  unsigned char b0 = bytes[0];
  if (b0 < 0x80u) {
    *advance = 1;
    return (uint32_t)b0;
  }

  if ((b0 & 0xe0u) == 0xc0u && len >= 2) {
    unsigned char b1 = bytes[1];
    if ((b1 & 0xc0u) == 0x80u) {
      uint32_t cp = ((uint32_t)(b0 & 0x1fu) << 6) | (uint32_t)(b1 & 0x3fu);
      if (cp >= 0x80u) {
        *advance = 2;
        return cp;
      }
    }
  }

  if ((b0 & 0xf0u) == 0xe0u && len >= 3) {
    unsigned char b1 = bytes[1];
    unsigned char b2 = bytes[2];
    if ((b1 & 0xc0u) == 0x80u && (b2 & 0xc0u) == 0x80u) {
      uint32_t cp = ((uint32_t)(b0 & 0x0fu) << 12) |
                    ((uint32_t)(b1 & 0x3fu) << 6) |
                    (uint32_t)(b2 & 0x3fu);
      if (cp >= 0x800u && !(cp >= 0xd800u && cp <= 0xdfffu)) {
        *advance = 3;
        return cp;
      }
    }
  }

  if ((b0 & 0xf8u) == 0xf0u && len >= 4) {
    unsigned char b1 = bytes[1];
    unsigned char b2 = bytes[2];
    unsigned char b3 = bytes[3];
    if ((b1 & 0xc0u) == 0x80u && (b2 & 0xc0u) == 0x80u &&
        (b3 & 0xc0u) == 0x80u) {
      uint32_t cp = ((uint32_t)(b0 & 0x07u) << 18) |
                    ((uint32_t)(b1 & 0x3fu) << 12) |
                    ((uint32_t)(b2 & 0x3fu) << 6) |
                    (uint32_t)(b3 & 0x3fu);
      if (cp >= 0x10000u && cp <= 0x10ffffu) {
        *advance = 4;
        return cp;
      }
    }
  }

  *advance = 1;
  return 0xfffdu;
}

static uint32_t alloc_guest_vec(w2c___MODULE__* instance, size_t len, uint32_t elem_ref) {
  if (len > INT32_MAX / 4) {
    host_fail("guest vector too large");
  }
  uint32_t header_ptr = w2c___MODULE___alloc(instance, VEC_HEADER_SIZE);
  uint32_t data_ptr = w2c___MODULE___alloc(instance, (uint32_t)(len * 4));
  store_u32(instance, header_ptr + VEC_LEN_OFFSET, (uint32_t)len);
  store_u32(instance, header_ptr + VEC_CAP_OFFSET, (uint32_t)len);
  store_u32(instance, header_ptr + VEC_RC_OFFSET, 1u);
  store_u32(instance, header_ptr + VEC_ELEM_REF_OFFSET, elem_ref);
  store_u32(instance, header_ptr + VEC_DATA_PTR_OFFSET, data_ptr);
  store_u32(instance, header_ptr + VEC_MAGIC_OFFSET, VEC_MAGIC);
  return header_ptr;
}

static uint32_t write_guest_codepoints(w2c___MODULE__* instance,
                                       const uint32_t* values,
                                       size_t len) {
  uint32_t header_ptr = alloc_guest_vec(instance, len, 0u);
  uint32_t data_ptr = load_u32(instance, header_ptr + VEC_DATA_PTR_OFFSET);
  for (size_t i = 0; i < len; ++i) {
    store_u32(instance, data_ptr + (uint32_t)(i * 4), values[i]);
  }
  return header_ptr;
}

static uint32_t write_guest_bytes(w2c___MODULE__* instance,
                                  const unsigned char* bytes,
                                  size_t len) {
  uint32_t header_ptr = alloc_guest_vec(instance, len, 0u);
  uint32_t data_ptr = load_u32(instance, header_ptr + VEC_DATA_PTR_OFFSET);
  for (size_t i = 0; i < len; ++i) {
    store_u32(instance, data_ptr + (uint32_t)(i * 4), (uint32_t)bytes[i]);
  }
  return header_ptr;
}

static void write_guest_bytes_into_existing(w2c___MODULE__* instance,
                                            uint32_t vec_ptr,
                                            const unsigned char* bytes,
                                            size_t len) {
  uint32_t vec_len = load_u32(instance, vec_ptr + VEC_LEN_OFFSET);
  uint32_t elem_ref = load_u32(instance, vec_ptr + VEC_ELEM_REF_OFFSET);
  uint32_t data_ptr = load_u32(instance, vec_ptr + VEC_DATA_PTR_OFFSET);
  uint32_t magic = load_u32(instance, vec_ptr + VEC_MAGIC_OFFSET);
  if (vec_len == 0u) {
    host_fail("read/buffer! buffer length must be positive");
  }
  if (elem_ref != 0u) {
    host_fail("read/buffer! requires a scalar [Int] byte buffer");
  }
  if (magic != VEC_MAGIC) {
    host_fail("read/buffer! received invalid vector");
  }
  if (len > (size_t)vec_len) {
    host_fail("read/buffer! attempted to overfill guest buffer");
  }
  for (size_t i = 0; i < len; ++i) {
    store_u32(instance, data_ptr + (uint32_t)(i * 4), (uint32_t)bytes[i]);
  }
}

static uint32_t write_guest_utf8_string(w2c___MODULE__* instance,
                                        const unsigned char* bytes,
                                        size_t len) {
  uint32_t* codes = (uint32_t*)xmalloc((len == 0 ? 1 : len) * sizeof(uint32_t));
  size_t count = 0;
  size_t pos = 0;
  while (pos < len) {
    size_t advance = 0;
    codes[count++] = decode_utf8_one(bytes + pos, len - pos, &advance);
    if (advance == 0) {
      advance = 1;
    }
    pos += advance;
  }
  uint32_t out = write_guest_codepoints(instance, codes, count);
  free(codes);
  return out;
}

static unsigned char* read_guest_utf8(w2c___MODULE__* instance,
                                      uint32_t vec_ptr,
                                      size_t* out_len) {
  uint32_t len = load_u32(instance, vec_ptr + VEC_LEN_OFFSET);
  uint32_t data_ptr = load_u32(instance, vec_ptr + VEC_DATA_PTR_OFFSET);
  size_t cap = (size_t)len * 4 + 1;
  unsigned char* out = (unsigned char*)xmalloc(cap);
  size_t pos = 0;
  for (uint32_t i = 0; i < len; ++i) {
    uint32_t cp = load_u32(instance, data_ptr + i * 4);
    unsigned char encoded[4];
    size_t wrote = 0;
    encode_utf8(cp, encoded, &wrote);
    memcpy(out + pos, encoded, wrote);
    pos += wrote;
  }
  out[pos] = '\0';
  *out_len = pos;
  return out;
}

static unsigned char* read_stream_all(FILE* file, size_t* out_len) {
  size_t cap = 8192;
  size_t len = 0;
  unsigned char* out = (unsigned char*)xmalloc(cap);
  for (;;) {
    if (len == cap) {
      cap *= 2;
      out = (unsigned char*)xrealloc(out, cap);
    }
    size_t n = fread(out + len, 1, cap - len, file);
    len += n;
    if (n == 0) {
      if (ferror(file)) {
        free(out);
        host_fail("stream read failed");
      }
      break;
    }
  }
  *out_len = len;
  return out;
}

static int cmp_cstr_ptr(const void* a, const void* b) {
  const char* left = *(const char* const*)a;
  const char* right = *(const char* const*)b;
  return strcmp(left, right);
}

static void mkdir_p_path(const char* raw_path) {
  char path[PATH_MAX];
  size_t len = strlen(raw_path);
  if (len >= sizeof(path)) {
    host_fail("path too long");
  }
  memcpy(path, raw_path, len + 1);
  for (size_t i = 1; i < len; ++i) {
    if (path[i] == '/') {
      path[i] = '\0';
      if (path[0] != '\0' && mkdir(path, 0777) != 0 && errno != EEXIST) {
        host_fail("mkdir failed");
      }
      path[i] = '/';
    }
  }
  if (path[0] != '\0' && mkdir(path, 0777) != 0 && errno != EEXIST) {
    host_fail("mkdir failed");
  }
}

static void mkdir_parent_dirs(const char* raw_path) {
  char path[PATH_MAX];
  size_t len = strlen(raw_path);
  if (len >= sizeof(path)) {
    host_fail("path too long");
  }
  memcpy(path, raw_path, len + 1);
  char* slash = strrchr(path, '/');
  if (slash == NULL) {
    return;
  }
  if (slash == path) {
    return;
  }
  *slash = '\0';
  mkdir_p_path(path);
}

static void delete_recursive(const char* path) {
  struct stat st;
  if (lstat(path, &st) != 0) {
    host_fail("delete stat failed");
  }
  if (S_ISDIR(st.st_mode)) {
    DIR* dir = opendir(path);
    if (dir == NULL) {
      host_fail("opendir failed");
    }
    struct dirent* entry;
    while ((entry = readdir(dir)) != NULL) {
      if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
        continue;
      }
      size_t path_len = strlen(path);
      size_t name_len = strlen(entry->d_name);
      char* child = (char*)xmalloc(path_len + 1 + name_len + 1);
      memcpy(child, path, path_len);
      child[path_len] = '/';
      memcpy(child + path_len + 1, entry->d_name, name_len + 1);
      delete_recursive(child);
      free(child);
    }
    closedir(dir);
    if (rmdir(path) != 0) {
      host_fail("rmdir failed");
    }
  } else if (unlink(path) != 0) {
    host_fail("unlink failed");
  }
}

static void set_guest_argv(struct w2c_host* host, int argc, char** argv) {
  w2c___MODULE___argv_clear(host->instance);
  uint32_t argv_vec = w2c___MODULE___make_vec(host->instance, 1u);
  for (int i = 1; i < argc; ++i) {
    uint32_t arg_ptr = write_guest_utf8_string(
        host->instance, (const unsigned char*)argv[i], strlen(argv[i]));
    w2c___MODULE___vec_push(host->instance, argv_vec, arg_ptr);
    w2c___MODULE___rc_release(host->instance, arg_ptr);
  }
  w2c___MODULE___set_argv(host->instance, argv_vec);
  w2c___MODULE___rc_release(host->instance, argv_vec);
}

u32 w2c_host_list_dir(struct w2c_host* host, u32 path_vec_ptr) {
  size_t path_len = 0;
  unsigned char* path = read_guest_utf8(host->instance, path_vec_ptr, &path_len);
  (void)path_len;
  DIR* dir = opendir((const char*)path);
  if (dir == NULL) {
    free(path);
    host_fail("list-dir open failed");
  }

  size_t count = 0;
  size_t cap = 16;
  char** names = (char**)xmalloc(cap * sizeof(char*));
  struct dirent* entry;
  while ((entry = readdir(dir)) != NULL) {
    if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
      continue;
    }
    if (count == cap) {
      cap *= 2;
      names = (char**)xrealloc(names, cap * sizeof(char*));
    }
    names[count++] = xstrdup(entry->d_name);
  }
  closedir(dir);
  qsort(names, count, sizeof(char*), cmp_cstr_ptr);

  size_t total = 0;
  for (size_t i = 0; i < count; ++i) {
    total += strlen(names[i]) + 1;
  }
  unsigned char* out = (unsigned char*)xmalloc(total == 0 ? 1 : total);
  size_t pos = 0;
  for (size_t i = 0; i < count; ++i) {
    size_t n = strlen(names[i]);
    memcpy(out + pos, names[i], n);
    pos += n;
    out[pos++] = '\n';
    free(names[i]);
  }
  free(names);
  free(path);

  uint32_t result = write_guest_utf8_string(host->instance, out, pos);
  free(out);
  return result;
}

u32 w2c_host_read_file(struct w2c_host* host, u32 path_vec_ptr) {
  size_t path_len = 0;
  unsigned char* path = read_guest_utf8(host->instance, path_vec_ptr, &path_len);
  (void)path_len;
  FILE* file = fopen((const char*)path, "rb");
  if (file == NULL) {
    free(path);
    host_fail("read! open failed");
  }
  size_t len = 0;
  unsigned char* bytes = read_stream_all(file, &len);
  fclose(file);
  free(path);
  uint32_t out = write_guest_utf8_string(host->instance, bytes, len);
  free(bytes);
  return out;
}

u32 w2c_host_read_stdin(struct w2c_host* host) {
  size_t len = 0;
  unsigned char* bytes = read_stream_all(stdin, &len);
  uint32_t out = write_guest_utf8_string(host->instance, bytes, len);
  free(bytes);
  return out;
}

u32 w2c_host_read_chunks(struct w2c_host* host,
                         u32 path_vec_ptr,
                         u32 chunk_size,
                         u32 callback) {
  if (chunk_size == 0) {
    host_fail("read/chunks! chunk size must be positive");
  }
  size_t path_len = 0;
  unsigned char* path = read_guest_utf8(host->instance, path_vec_ptr, &path_len);
  (void)path_len;
  FILE* file = fopen((const char*)path, "rb");
  free(path);
  if (file == NULL) {
    host_fail("read/chunks! open failed");
  }
  unsigned char* buffer = (unsigned char*)xmalloc(chunk_size);
  for (;;) {
    size_t n = fread(buffer, 1, chunk_size, file);
    if (n == 0) {
      if (ferror(file)) {
        free(buffer);
        fclose(file);
        host_fail("read/chunks! read failed");
      }
      break;
    }
    uint32_t chunk_ptr = write_guest_bytes(host->instance, buffer, n);
    uint32_t should_stop =
        w2c___MODULE___apply1_i32(host->instance, callback, chunk_ptr);
    w2c___MODULE___rc_release(host->instance, chunk_ptr);
    if (should_stop != 0) {
      free(buffer);
      fclose(file);
      return 1u;
    }
  }
  free(buffer);
  fclose(file);
  return 0u;
}

u32 w2c_host_read_buffer(struct w2c_host* host,
                         u32 buffer_vec_ptr,
                         u32 path_vec_ptr,
                         u32 callback) {
  size_t path_len = 0;
  unsigned char* path = read_guest_utf8(host->instance, path_vec_ptr, &path_len);
  (void)path_len;
  FILE* file = fopen((const char*)path, "rb");
  free(path);
  if (file == NULL) {
    host_fail("read/buffer! open failed");
  }
  uint32_t chunk_size = load_u32(host->instance, buffer_vec_ptr + VEC_LEN_OFFSET);
  if (chunk_size == 0u) {
    fclose(file);
    host_fail("read/buffer! buffer length must be positive");
  }
  unsigned char* buffer = (unsigned char*)xmalloc(chunk_size);
  for (;;) {
    size_t n = fread(buffer, 1, chunk_size, file);
    if (n == 0) {
      if (ferror(file)) {
        free(buffer);
        fclose(file);
        host_fail("read/buffer! read failed");
      }
      break;
    }
    write_guest_bytes_into_existing(host->instance, buffer_vec_ptr, buffer, n);
    uint32_t should_stop =
        w2c___MODULE___apply1_i32(host->instance, callback, (uint32_t)n);
    if (should_stop != 0) {
      free(buffer);
      fclose(file);
      return 1u;
    }
  }
  free(buffer);
  fclose(file);
  return 0u;
}

u32 w2c_host_read_stdin_chunks(struct w2c_host* host, u32 chunk_size, u32 callback) {
  if (chunk_size == 0) {
    host_fail("stdin/chunks! chunk size must be positive");
  }
  unsigned char* buffer = (unsigned char*)xmalloc(chunk_size);
  for (;;) {
    size_t n = fread(buffer, 1, chunk_size, stdin);
    if (n == 0) {
      if (ferror(stdin)) {
        free(buffer);
        host_fail("stdin/chunks! read failed");
      }
      break;
    }
    uint32_t chunk_ptr = write_guest_bytes(host->instance, buffer, n);
    uint32_t should_stop =
        w2c___MODULE___apply1_i32(host->instance, callback, chunk_ptr);
    w2c___MODULE___rc_release(host->instance, chunk_ptr);
    if (should_stop != 0) {
      free(buffer);
      return 1u;
    }
  }
  free(buffer);
  return 0u;
}

u32 w2c_host_read_lines(struct w2c_host* host, u32 path_vec_ptr, u32 callback) {
  size_t path_len = 0;
  unsigned char* path = read_guest_utf8(host->instance, path_vec_ptr, &path_len);
  (void)path_len;
  FILE* file = fopen((const char*)path, "rb");
  free(path);
  if (file == NULL) {
    host_fail("read/lines! open failed");
  }
  size_t cap = 256;
  unsigned char* line = (unsigned char*)xmalloc(cap);
  for (;;) {
    size_t len = 0;
    int ch = 0;
    while ((ch = fgetc(file)) != EOF) {
      if (ch == '\n') {
        break;
      }
      if (len == cap) {
        cap *= 2;
        line = (unsigned char*)xrealloc(line, cap);
      }
      line[len++] = (unsigned char)ch;
    }
    if (ch == EOF && len == 0) {
      break;
    }
    if (len > 0 && line[len - 1] == '\r') {
      len -= 1;
    }
    uint32_t line_ptr = write_guest_bytes(host->instance, line, len);
    uint32_t should_stop =
        w2c___MODULE___apply1_i32(host->instance, callback, line_ptr);
    w2c___MODULE___rc_release(host->instance, line_ptr);
    if (should_stop != 0) {
      free(line);
      fclose(file);
      return 1u;
    }
  }
  free(line);
  fclose(file);
  return 0u;
}

u32 w2c_host_write_file(struct w2c_host* host, u32 path_vec_ptr, u32 data_vec_ptr) {
  size_t path_len = 0;
  size_t data_len = 0;
  unsigned char* path = read_guest_utf8(host->instance, path_vec_ptr, &path_len);
  unsigned char* data = read_guest_utf8(host->instance, data_vec_ptr, &data_len);
  (void)path_len;
  mkdir_parent_dirs((const char*)path);
  FILE* file = fopen((const char*)path, "wb");
  if (file == NULL) {
    free(path);
    free(data);
    host_fail("write! open failed");
  }
  if (data_len > 0 && fwrite(data, 1, data_len, file) != data_len) {
    fclose(file);
    free(path);
    free(data);
    host_fail("write! failed");
  }
  fclose(file);
  free(path);
  free(data);
  return 0u;
}

u32 w2c_host_mkdir_p(struct w2c_host* host, u32 path_vec_ptr) {
  size_t path_len = 0;
  unsigned char* path = read_guest_utf8(host->instance, path_vec_ptr, &path_len);
  (void)path_len;
  mkdir_p_path((const char*)path);
  free(path);
  return 0u;
}

u32 w2c_host_delete(struct w2c_host* host, u32 path_vec_ptr) {
  size_t path_len = 0;
  unsigned char* path = read_guest_utf8(host->instance, path_vec_ptr, &path_len);
  (void)host;
  (void)path_len;
  delete_recursive((const char*)path);
  free(path);
  return 0u;
}

u32 w2c_host_move(struct w2c_host* host, u32 src_vec_ptr, u32 dst_vec_ptr) {
  size_t src_len = 0;
  size_t dst_len = 0;
  unsigned char* src = read_guest_utf8(host->instance, src_vec_ptr, &src_len);
  unsigned char* dst = read_guest_utf8(host->instance, dst_vec_ptr, &dst_len);
  (void)src_len;
  (void)dst_len;
  mkdir_parent_dirs((const char*)dst);
  if (rename((const char*)src, (const char*)dst) != 0) {
    free(src);
    free(dst);
    host_fail("move! failed");
  }
  free(src);
  free(dst);
  return 0u;
}

u32 w2c_host_print(struct w2c_host* host, u32 text_vec_ptr) {
  size_t len = 0;
  unsigned char* text = read_guest_utf8(host->instance, text_vec_ptr, &len);
  if (len > 0 && fwrite(text, 1, len, stdout) != len) {
    free(text);
    host_fail("print! failed");
  }
  fflush(stdout);
  free(text);
  return 0u;
}

u32 w2c_host_sleep(struct w2c_host* host, u32 millis) {
  (void)host;
  struct timespec req;
  req.tv_sec = millis / 1000u;
  req.tv_nsec = (long)(millis % 1000u) * 1000000l;
  if (nanosleep(&req, NULL) != 0) {
    host_fail("sleep! failed");
  }
  return 0u;
}

u32 w2c_host_time(struct w2c_host* host) {
  (void)host;
  time_t now = time(NULL);
  if (now < 0 || now > INT32_MAX) {
    host_fail("time! overflow");
  }
  return (u32)now;
}

u32 w2c_host_clear(struct w2c_host* host) {
  (void)host;
  static const char seq[] = "\x1b[2J\x1b[H";
  if (fwrite(seq, 1, sizeof(seq) - 1, stdout) != sizeof(seq) - 1) {
    host_fail("clear! failed");
  }
  fflush(stdout);
  return 0u;
}

static void print_int_tuple2(w2c___MODULE__* instance, uint32_t tuple_ptr) {
  uint32_t data_ptr = load_u32(instance, tuple_ptr + 16);
  int32_t first = (int32_t)load_u32(instance, data_ptr);
  int32_t second = (int32_t)load_u32(instance, data_ptr + 4);
  printf("{ %d %d }\n", first, second);
}

static void print_int_vector(w2c___MODULE__* instance, uint32_t vec_ptr) {
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

static void print_bool_int_vector_tuple(w2c___MODULE__* instance, uint32_t tuple_ptr) {
  uint32_t data_ptr = load_u32(instance, tuple_ptr + 16);
  uint32_t ok = load_u32(instance, data_ptr);
  uint32_t vec_ptr = load_u32(instance, data_ptr + 4);
  printf("{ %s ", ok ? "true" : "false");
  print_int_vector(instance, vec_ptr);
  printf(" }\n");
}

int main(int argc, char** argv) {
  wasm_rt_init();

  w2c___MODULE__ instance;
  struct w2c_host host;
  memset(&host, 0, sizeof(host));
  host.instance = &instance;
  __INSTANTIATE__

  wasm_rt_trap_t trap = (wasm_rt_trap_t)wasm_rt_try(g_wasm_rt_jmp_buf);
  if (trap != WASM_RT_TRAP_NONE) {
    fprintf(stderr, "wasm trap: %s\n", wasm_rt_strerror(trap));
    wasm2c___MODULE___free(&instance);
    wasm_rt_free();
    return 134;
  }

  set_guest_argv(&host, argc, argv);
  uint32_t result = w2c___MODULE___main(&instance);
  __RESULT_PRINT__

  wasm2c___MODULE___free(&instance);
  wasm_rt_free();
  return 0;
}
