#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>

#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#define EXTRACT_BASE_FILE(file) (strrchr(file, '\\') ? strrchr(file, '\\') + 1 : file)
#else
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

static inline uint64_t get_param(uint64_t *param_page, long idx)
{
    return param_page[idx];
}
static inline void set_param(uint64_t *param_page, long idx, uint64_t val)
{
    param_page[idx] = val;
}

EXPORT int my_malloc(uint64_t *param_page, const long *params, int params_len)
{
    const static int used_len = 2; // this function need 2 input
    LEN_CHECK(used_len, params_len);

    long len = params[0];
    long out_idx = params[1];
    IDX_CHECK(out_idx);
    void *addr = malloc(len);
    if (!addr) // this function need 1 input
        return -22;

    set_param(param_page, out_idx, (uint64_t)addr);
    return 0;
}

EXPORT int my_free(uint64_t *param_page, const long *params, int params_len)
{
    const static int used_len = 1; // this function need 1 input
    LEN_CHECK(used_len, params_len);

    long in_idx = params[0];
    IDX_CHECK(in_idx);
    MEM_CHECK(in_idx);
    free((void *)get_param(param_page, in_idx));
    set_param(param_page, in_idx, 0);
    return 0;
}

EXPORT int my_read32(uint64_t *param_page, const long *params, int params_len)
{
    const static int used_len = 2; // this function need 2 input
    LEN_CHECK(used_len, params_len);

    long in_idx = params[0];
    long offset = params[1];
    IDX_CHECK(in_idx);
    return *(int *)(get_param(param_page, in_idx) + offset);
}

EXPORT int my_write32(uint64_t *param_page, const long *params, int params_len)
{
    const static int used_len = 3; // this function need 3 input
    LEN_CHECK(used_len, params_len);

    long in_idx = params[0];
    long offset = params[1];
    long val = params[2];
    IDX_CHECK(in_idx);
    *(int *)(get_param(param_page, in_idx) + offset) = val;
    return 0;
}