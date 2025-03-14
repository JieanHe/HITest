#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>

#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#define EXTRACT_BASE_FILE(file) (strrchr(file, '\\') ? strrchr(file, '\\') + 1 : file)
#else
#include <sys/mman.h>
#define EXPORT __attribute__((visibility("default")))
#define EXTRACT_BASE_FILE(file) (strrchr(file, '/') ? strrchr(file, '/') + 1 : file)
#endif

#define MAX_IDX 512
#define IDX_CHECK(idx)                                                                                                                 \
    do                                                                                                                                 \
    {                                                                                                                                  \
        if (idx >= MAX_IDX)                                                                                                            \
        {                                                                                                                              \
            printf("[%s : line %d]  ERROR: idx %ld is bigger than max idx %d\n", EXTRACT_BASE_FILE(__FILE__), __LINE__, idx, MAX_IDX); \
            return -12;                                                                                                                \
        }                                                                                                                              \
    } while (0)

#define MEM_CHECK(idx)                                                                                                    \
    do                                                                                                                    \
    {                                                                                                                     \
        if (param_page[idx] == 0)                                                                                           \
        {                                                                                                                 \
            printf("[%s : line %d]  ERROR: idx %ld get NULL address!\n", EXTRACT_BASE_FILE(__FILE__), __LINE__, idx); \
            return -12;                                                                                                   \
        }                                                                                                                 \
    } while (0)

#define LEN_CHECK(used_len, max_len)                                                         \
    do                                                                                       \
    {                                                                                        \
        if (used_len > max_len)                                                              \
        {                                                                                    \
            printf("[%s : line %d]  ERROR: used_len %d of %s is bigger than max len %d !\n", \
                   EXTRACT_BASE_FILE(__FILE__), __LINE__, used_len, __FUNCTION__, max_len);  \
            return -12;                                                                      \
        }                                                                                    \
    } while (0)

static inline uint64_t get_param(uint64_t *param_page, uint64_t idx)
{
    return param_page[idx];
}
static inline void set_param(uint64_t *param_page, uint64_t idx, uint64_t val)
{
    param_page[idx] = val;
}

EXPORT int my_malloc(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 2; // this function need 2 input
    LEN_CHECK(used_len, params_len);

    uint64_t len = params[0];
    uint64_t out_idx = params[1];
    IDX_CHECK(out_idx);
    void *addr = malloc(len);
    if (!addr) // this function need 1 input
        return -22;

    set_param(param_page, out_idx, (uint64_t)addr);
    return 0;
}

EXPORT int my_free(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 1; // this function need 1 input
    LEN_CHECK(used_len, params_len);

    uint64_t in_idx = params[0];
    IDX_CHECK(in_idx);
    MEM_CHECK(in_idx);
    free((void *)get_param(param_page, in_idx));
    set_param(param_page, in_idx, 0);
    return 0;
}

EXPORT int my_read32(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 2; // this function need 2 input
    LEN_CHECK(used_len, params_len);

    uint64_t in_idx = params[0];
    uint64_t offset = params[1];
    IDX_CHECK(in_idx);
    return *(int *)(get_param(param_page, in_idx) + offset);
}

EXPORT int my_write32(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 3; // this function need 3 input
    LEN_CHECK(used_len, params_len);

    uint64_t in_idx = params[0];
    uint64_t offset = params[1];
    uint64_t val = params[2];
    IDX_CHECK(in_idx);
    *(int *)(get_param(param_page, in_idx) + offset) = val;
    return 0;
}

EXPORT int my_memcpy(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 3; // this function need 3 input
    LEN_CHECK(used_len, params_len);
    uint64_t in_idx = params[0];
    uint64_t out_idx = params[1];
    uint64_t len = params[2];
    IDX_CHECK(in_idx);
    uint64_t len = params[2];
    IDX_CHECK(in_idx);
    MEM_CHECK(in_idx);
    void *dst = malloc(len);
    if (!dst)
        return -22;

    memcpy(dst, (void *)(get_param(param_page, in_idx) ), len);
    set_param(param_page, out_idx, (uint64_t)dst);
    return 0;
}

EXPORT int my_strlen(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 2; // this function need 1 input
    LEN_CHECK(used_len, params_len);
    uint64_t in_idx = params[0];
    uint64_t out_idx = params[1];

    IDX_CHECK(in_idx);
    IDX_CHECK(out_idx);
    return strlen((void *)(get_param(param_page, in_idx) ));
}

EXPORT int my_strcmp(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 2; // this function need 2 input
    LEN_CHECK(used_len, params_len);
    uint64_t in_idx = params[0];
    uint64_t in_idx2 = params[1];
    uint64_t len = params[2];

    IDX_CHECK(in_idx);
    IDX_CHECK(in_idx2);
    return strncmp((void *)(get_param(param_page, in_idx) ), (void *)(get_param(param_page, in_idx2) ), len);
}
#ifndef _WIN32
EXPORT int my_open_fd(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 1; // this function need 2 input
    LEN_CHECK(used_len, params_len);

    uint64_t out_fd_idx = params[0];
    IDX_CHECK(out_fd_idx);

    int fd = open("/dev/mem", O_RDWR|O_SYNC);
    if (fd < 0)
        return -22;

    set_param(param_page, out_fd_idx, (uint64_t)fd);
    return 0;
}

EXPORT int my_close_fd(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 1; // this function need 2 input
    LEN_CHECK(used_len, params_len);

    uint64_t in_fd_idx = params[0];
    IDX_CHECK(in_fd_idx);

    close((int)get_param(param_page, in_fd_idx));
    set_param(param_page, in_fd_idx, 0);
    return 0;
}

EXPORT int my_mmap(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 4; // this function need 4 input
    LEN_CHECK(used_len, params_len);
    uint64_t in_fd_idx = params[0];
    uint64_t out_addr_idx = params[1];
    uint64_t len = params[2];
    uint64_t prot = params[3];

    IDX_CHECK(in_fd_idx);
    IDX_CHECK(out_addr_idx);

    void *addr = mmap(NULL, len, prot, MAP_SHARED, (int)get_param(param_page, in_fd_idx), 0);
    if (!addr)
        return -22;

    set_param(param_page, out_addr_idx, (uint64_t)addr);
    return 0;
}

EXPORT int my_munmap(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 2; // this function need 2 input
    LEN_CHECK(used_len, params_len);
    uint64_t in_addr_idx = params[0];
    uint64_t len = params[1];

    IDX_CHECK(in_addr_idx);

    munmap((void *)get_param(param_page, in_addr_idx), len);
    set_param(param_page, in_addr_idx, 0);
    return 0;
}
#endif