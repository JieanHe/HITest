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
    GET_VALUE(size_t, len, 0);
    GET_VALUE(int, mem_idx, 1);
    void *ptr = malloc(len);
    if (!ptr)
    {
        fprintf(stderr, "[%s] malloc failed! errno: %d\n", __func__, errno);
        return -1;
    }
    SET_OUTPUT_IDX(mem_idx, (uint64_t)ptr);
    return 0;
}

EXPORT_FUNC(free, mem_idx)
{
    CHECK_PARAM_LEN(1);
    GET_INPUT_IDX_NZ(void *, ptr, 0);

    free(ptr);
    return 0;
}

EXPORT_FUNC(memcpy, dst_idx, src_idx, len)
{
    CHECK_PARAM_LEN(3);
    GET_INPUT_IDX_NZ(void *, dst_idx, 0);
    GET_INPUT_IDX_NZ(void *, src_idx, 1);
    GET_VALUE(size_t, len, 2);

    memcpy(dst_idx, src_idx, len);
    return 0;
}

EXPORT_FUNC(memset, dst_idx, val, len)
{
    CHECK_PARAM_LEN(3);
    GET_INPUT_IDX_NZ(void *, dst_idx, 0);
    GET_VALUE(int, val, 1);
    GET_VALUE(size_t, len, 2);

    memset(dst_idx, val, len);
    return 0;
}

EXPORT_FUNC(memcmp, dst_idx, src_off, src_idx, dst_off, len)
{
    CHECK_PARAM_LEN(5);
    GET_INPUT_IDX_NZ(void *, dst_idx, 0);
    GET_VALUE(size_t, dst_off, 1);
    GET_INPUT_IDX_NZ(void *, src_idx, 2);
    GET_VALUE(size_t, src_off, 3 );

    GET_VALUE(size_t, len, 4);

    return memcmp(dst_idx + dst_off, src_idx + src_off, len);
}
//==========================================================
// data access
//==========================================================

EXPORT_FUNC(read32, addr_idx, off)
{
    CHECK_PARAM_LEN(2);
    GET_INPUT_IDX_NZ(uint64_t, addr_idx, 0);
    GET_VALUE(size_t, off, 1);

    return *(uint32_t *)(addr_idx + off);
}

EXPORT_FUNC(write32, addr_idx, off, val)
{
    CHECK_PARAM_LEN(3);

    GET_INPUT_IDX_NZ(uint64_t, addr_idx, 0);
    GET_VALUE(size_t, off, 1);
    GET_VALUE(uint32_t, val, 2);

    *(uint32_t *)(addr_idx + off) = val;
    return 0;
}

EXPORT_FUNC(strncpy, dst_idx, str, len)
{
    CHECK_PARAM_LEN(3);
    GET_INPUT_IDX_NZ(char *, ptr, 0);
    GET_VALUE(const char *, str, 1);
    GET_VALUE(size_t, len, 2);

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
    GET_INPUT_IDX_NZ(const char *, str_idx, 0);
    return strlen(str_idx);
}


EXPORT_FUNC(atoi, str_idx)
{
    CHECK_PARAM_LEN(1);
    GET_INPUT_IDX_NZ(const char *, str_idx, 0);
    return atoi(str_idx);
}

EXPORT_FUNC(strcmp, str1, str2)
{
    CHECK_PARAM_LEN(2);
    GET_INPUT_IDX_NZ(const char *, str1, 0);
    GET_INPUT_IDX_NZ(const char *, str2, 1);

    return strcmp(str1, str2);
}

// caller should make sure length of dse_addr must not less than len!
EXPORT_FUNC(strfill, dst_addr, content, len)
{
    CHECK_PARAM_LEN(3);
    GET_INPUT_IDX_NZ(char *, dst_addr, 0);
    GET_VALUE(char *, content, 1);
    GET_VALUE(size_t, len, 2);

    int content_len = strlen(content);
    if (len <= content_len)
    {
        strncpy(dst_addr, content, len);
    } else {
        for (size_t i = 0; i < len; i += content_len) {
            strncpy(dst_addr + i, content, content_len);
        }
    }
    dst_addr[len] = '\0';
    return 0;
}

EXPORT_FUNC(read64, addr_idx, off)
{
    CHECK_PARAM_LEN(2);
    GET_INPUT_IDX_NZ(uint64_t, addr_idx, 0);
    GET_VALUE(size_t, off, 1);

    addr_idx += off;
    return *(uint64_t*)addr_idx;
}

EXPORT_FUNC(write64, addr_idx, off, val)
{
    CHECK_PARAM_LEN(3);

    GET_INPUT_IDX_NZ(uint64_t, addr_idx, 0);
    GET_VALUE(size_t, off, 1);
    GET_VALUE(uint64_t, val, 2);
    addr_idx += off;
    *(uint64_t *)addr_idx = val;
    return 0;
}

#if defined(__linux__)
#include <sys/mman.h>
#include <fcntl.h>
#include <unistd.h>
#define BIT(x) (1ULL << (x))
EXPORT_FUNC(open, pathname,  fd_idx)
{
    CHECK_PARAM_LEN(2);
    GET_VALUE(const char *, pathname, 0);
    GET_VALUE(int, fd_idx, 1);

    int fd = open(pathname, O_RDWR);
    if (fd == -1)
    {
        fprintf(stderr, "[%s] open failed! errno: %d\n", __func__, errno);
        return -1;
    }
    SET_OUTPUT_IDX(fd_idx, (uint64_t)fd);
    return 0;
}

EXPORT_FUNC(close, fd_idx)
{
    CHECK_PARAM_LEN(1);
    GET_INPUT_IDX_NZ(int, fd, 0);

    if (close(fd) == -1)
    {
        fprintf(stderr, "[%s] close failed! errno: %d\n", __func__, errno);
        return -1;
    }
    return 0;
}

EXPORT_FUNC(mmap, addr, len, prot, flags, fd_idx, offset, addr_idx)
{
    CHECK_PARAM_LEN(7);
    GET_VALUE(void *, addr, 0);
    GET_VALUE(size_t, len, 1);
    GET_VALUE(int, iprot, 2);
    GET_VALUE(int, iflags, 3);
    GET_INPUT_IDX_NZ(int, fd, 4);
    GET_VALUE(off_t, offset, 5);
    GET_VALUE(int, addr_idx, 6);

    int flags = 0;
    int prot = 0;
    if (iprot & BIT(0)) {
        prot |= PROT_READ;
    }
    if (iprot & BIT(1)) {
        prot |= PROT_WRITE;
    }

    if (iflags & BIT(0)) {
        flags |= MAP_SHARED;
    } else {
        flags |= MAP_PRIVATE;
    }

    if (iflags & BIT(1)) {
        flags |= MAP_FIXED;
    }

    if (iflags & BIT(2)) {
        flags |= MAP_ANONYMOUS;
    }

    void *mapped_addr = mmap(addr, len, prot, flags, fd, offset);
    if (mapped_addr == NULL)
    {
        fprintf(stderr, "[%s] mmap failed! err: %s(%d)\n", __func__, strerror(errno), errno);
        return -1;
    }
    SET_OUTPUT_IDX(addr_idx, (uint64_t)mapped_addr);
    return 0;
}

EXPORT_FUNC(munmap, addr_idx, len)
{
    CHECK_PARAM_LEN(2);
    GET_INPUT_IDX_NZ(void *, addr, 0);
    GET_VALUE(size_t, length, 1);
    if (munmap(addr, length) == -1)
    {
        fprintf(stderr, "[%s] munmap failed! errno: %d\n", __func__, errno);
    }
    return 0;
}
#endif
