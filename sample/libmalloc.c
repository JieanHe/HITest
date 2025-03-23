#include <stdlib.h>
#include <string.h>
#include <errno.h>

#include "export_function.h"
//==========================================================
// memory operation
//==========================================================

EXPORT_FUNC(malloc, len, mem_idx)
{
    CHECK_PARAM_LEN(2);
    IN_VALUE(size_t, len, 0);
    IN_ABSOLUTE_IDX(uint64_t, mem_idx, 1);
    void *ptr = malloc(len);
    if (!ptr)
    {
        fprintf(stderr, "[%s] malloc failed! errno: %d\n", __func__, errno);
        return -1;
    }
    OUT_ABSOLUTE_IDX(mem_idx, (uint64_t)ptr);
    return 0;
}

EXPORT_FUNC(free, mem_idx)
{
    CHECK_PARAM_LEN(1);
    IN_RELATIVE_IDX(void *, ptr, 0);

    free(ptr);
    return 0;
}

EXPORT_FUNC(memcpy, dst_idx, src_idx, len)
{
    CHECK_PARAM_LEN(3);
    IN_RELATIVE_IDX(void *, dst_idx, 0);
    IN_RELATIVE_IDX(void *, src_idx, 1);
    IN_VALUE(size_t, len, 2);

    memcpy(dst_idx, src_idx, len);
    return 0;
}

EXPORT_FUNC(memset, dst_idx, val, len)
{
    CHECK_PARAM_LEN(3);
    IN_RELATIVE_IDX(void *, dst_idx, 0);
    IN_VALUE(int, val, 1);
    IN_VALUE(size_t, len, 2);

    memset(dst_idx, val, len);
    return 0;
}

EXPORT_FUNC(memcmp, dst_idx, src_idx, len)
{
    CHECK_PARAM_LEN(3);
    IN_RELATIVE_IDX(void *, dst_idx, 0);
    IN_RELATIVE_IDX(void *, src_idx, 1);
    IN_VALUE(size_t, len, 2);

    return memcmp(dst_idx, src_idx, len);
}
//==========================================================
// data access
//==========================================================

EXPORT_FUNC(read32, addr_idx)
{
    CHECK_PARAM_LEN(1);
    IN_RELATIVE_IDX(uint32_t *, addr_idx, 0);

    return *addr_idx;
}

EXPORT_FUNC(write32, addr_idx, val)
{
    CHECK_PARAM_LEN(2);

    IN_RELATIVE_IDX(uint32_t *, addr_idx, 0);
    IN_VALUE(uint32_t, val, 1);

    *addr_idx = val;
    return 0;
}

EXPORT_FUNC(strncpy, dst_idx, str, len)
{
    CHECK_PARAM_LEN(3);
    IN_RELATIVE_IDX(char *, ptr, 0);
    IN_VALUE(const char *, str, 1);
    IN_VALUE(size_t, len, 2);

    if (strlen(str) > len)
    {
        fprintf(stderr, "[%s] strncpy failed! the src string is too long!\n", __func__);
        return -1;
    }

    strncpy(ptr, str, len);
    return 0;
}

EXPORT_FUNC(mem_strlen, str_idx)
{
    CHECK_PARAM_LEN(1);
    IN_RELATIVE_IDX(const char *, str_idx, 0);

    return strlen(str_idx);
}


EXPORT_FUNC(atoi, str_idx)
{
    CHECK_PARAM_LEN(1);
    IN_RELATIVE_IDX(const char *, str_idx, 0);
    return atoi(str_idx);
}

EXPORT_FUNC(strcmp, str1, str2)
{
    CHECK_PARAM_LEN(2);
    IN_RELATIVE_IDX(const char *, str1, 0);
    IN_RELATIVE_IDX(const char *, str2, 1);

    return strcmp(str1, str2);
}

EXPORT_FUNC(strfill, dst_addr, content, len)
{
    CHECK_PARAM_LEN(3);
    IN_RELATIVE_IDX(char *, dst_addr, 0);
    IN_VALUE(char *, content, 1);
    IN_VALUE(size_t, len, 2);

    if (len < strlen(content))
    {
        fprintf(stderr, "[%s] strfill failed! the content string is too long!\n", __func__);
        return -1;
    }

    strncpy(dst_addr, content, len);
    return 0;
}
