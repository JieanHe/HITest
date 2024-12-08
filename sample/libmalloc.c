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



#define MAX_ARR_LEN 100
#define IDX_CHECK(idx)                                                                                                    \
    do                                                                                                                    \
    {                                                                                                                     \
        if (idx >= MAX_ARR_LEN)                                                                                           \
        {                                                                                                                 \
            printf("[%s : line %d]  ERROR: out_idx %ld is bigger than max idx %ld\n", EXTRACT_BASE_FILE(__FILE__), __LINE__, idx, MAX_ARR_LEN); \
            return -12;                                                                                                   \
        }                                                                                                                 \
    } while (0)

#define MEM_CHECK(idx)                                                                              \
    do                                                                                              \
    {                                                                                               \
        if (ADDR_ARR[idx] == 0)                                                                     \
        {                                                                                           \
            printf("[%s : line %d]  ERROR: out_idx %ld get NULL address!\n", EXTRACT_BASE_FILE(__FILE__), __LINE__, idx); \
            return -12;                                                                             \
        }                                                                                           \
    } while (0)
static uint64_t ADDR_ARR[MAX_ARR_LEN] = {0};

EXPORT int my_malloc(int64_t len, int64_t out_idx, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7)
{
    IDX_CHECK(out_idx);
    void *addr = malloc(len);
    if (!addr)
        return -22;

    ADDR_ARR[out_idx] = (uint64_t)addr;
    return 0;
}

EXPORT int my_free(int64_t in_idx, int64_t offset, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7)
{
    IDX_CHECK(in_idx);
    if (ADDR_ARR[in_idx] == 0)
    {
        printf("[%s : line %d] invalid input idx %d\n", EXTRACT_BASE_FILE(__FILE__), __LINE__,  in_idx);
        return -12;
    }

    free((void *)ADDR_ARR[in_idx]);

    ADDR_ARR[in_idx] = 0;
    return 0;
}

EXPORT int my_read32(int64_t in_idx, int64_t offset, int64_t p2, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7)
{
    IDX_CHECK(in_idx);
    MEM_CHECK(in_idx);

    return *(int *)(ADDR_ARR[in_idx] + offset);
}

EXPORT int my_write32(int64_t in_idx, int64_t val, int64_t offset, int64_t p3, int64_t p4, int64_t p5, int64_t p6, int64_t p7)
{
    IDX_CHECK(in_idx);
    MEM_CHECK(in_idx);
    *(int *)(ADDR_ARR[in_idx] + offset) = val;
    return 0;
}
