#ifndef QUE_NATIVE_C_HOST_H
#define QUE_NATIVE_C_HOST_H

#include <stdint.h>

struct w2c_main;

enum que_host_permission {
    QUE_HOST_READ  = 1u << 0,
    QUE_HOST_STDIN = 1u << 1,
    QUE_HOST_WRITE = 1u << 2,
    QUE_HOST_PRINT = 1u << 3,
    QUE_HOST_CLOCK = 1u << 4,
    QUE_HOST_DELETE = 1u << 5,
    QUE_HOST_ALL = (1u << 6) - 1u
};

struct w2c_host {
    struct w2c_main* instance;
    uint32_t permissions;
};

/* Parse a comma/space-separated permission list such as "read,write,print". */
uint32_t que_host_parse_permissions(const char* value);
void que_host_init(struct w2c_host* host, struct w2c_main* instance,
                   uint32_t permissions);
/* Consume --allow options, populate Que's ARGV, and return 0 on success. */
int que_host_configure_argv(struct w2c_host* host, int argc, char** argv);
void que_host_print_result(struct w2c_host* host, uint32_t value,
                           const char* type_text);

#endif
