#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <errno.h>
#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#define EXTRACT_BASE_FILE(file) (strrchr(file, '\\') ? strrchr(file, '\\') + 1 : file)
#else
#include <sys/mman.h>
#include <fcntl.h> 
#include <unistd.h>
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

#define MEM_CHECK(idx)                                                                                                \
    do                                                                                                                \
    {                                                                                                                 \
        if (param_page[idx] == 0)                                                                                     \
        {                                                                                                             \
            printf("[%s : line %d]  ERROR: idx %ld get NULL address!\n", EXTRACT_BASE_FILE(__FILE__), __LINE__, idx); \
            return -12;                                                                                               \
        }                                                                                                             \
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
    uint64_t in_srcidx = params[0];
    uint64_t in_dstidx = params[1];
    uint64_t len = params[2];

    IDX_CHECK(in_srcidx);
    IDX_CHECK(in_dstidx);

    memcpy((void *)(get_param(param_page, in_dstidx)), (void *)(get_param(param_page, in_srcidx)), len);

    return 0;
}

EXPORT int my_strlen(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 2; // this function need 1 input
    LEN_CHECK(used_len, params_len);
    uint64_t in_idx = params[0];

    IDX_CHECK(in_idx);
    // printf("stris: %s\n", (void *)(get_param(param_page, in_idx)));
    return strlen((void *)(get_param(param_page, in_idx)));
}

EXPORT int my_strcmp(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 2; // this function need 2 input
    LEN_CHECK(used_len, params_len);
    uint64_t in_idx1 = params[0];
    uint64_t in_idx2 = params[1];
    uint64_t len = params[2];

    IDX_CHECK(in_idx1);
    IDX_CHECK(in_idx2);
    return strncmp((void *)(get_param(param_page, in_idx1)), (void *)(get_param(param_page, in_idx2)), len);
}

EXPORT int my_strfill(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 3; // this function need 3 input
    LEN_CHECK(used_len, params_len);
    uint64_t fill_idx = params[0];
    uint64_t src_addr = params[1];
    uint64_t len = params[2];

    IDX_CHECK(fill_idx);
    if (len != strlen((char *)src_addr))
        return 1;
    char *addr = (char *)(get_param(param_page, fill_idx));
    strcpy(addr, (char *)src_addr);

    return 0;
}
#ifndef _WIN32
EXPORT int my_open_fd(uint64_t *param_page, const uint64_t *params, int params_len)
{
    const static int used_len = 2; // this function need 2 input
    LEN_CHECK(used_len, params_len);

    char *strAddr = (char *)params[0];
    uint64_t out_fd_idx = params[1];
    IDX_CHECK(out_fd_idx);

    int fd = open(strAddr, O_RDWR | O_SYNC);
    if (fd < 0) {
        printf("open %s failed, error: %s\n", strAddr, strerror(errno));
        return fd;
    }
        
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

    IDX_CHECK(in_fd_idx);
    IDX_CHECK(out_addr_idx);

    void *addr = mmap(NULL, len, PROT_READ | PROT_WRITE, MAP_SHARED, (int)get_param(param_page, in_fd_idx), 0);
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