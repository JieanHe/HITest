#ifndef EXPORT_FUNCTION_H
#define EXPORT_FUNCTION_H

#include <stdint.h>
#include <stdio.h>

#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif

#define MAX_IDX 512

#define EXPORT_FUNC(func_name, ...) \
    EXPORT int Call_##func_name(uint64_t *param_page, const uint64_t *params, int params_len)

#define CHECK_PARAM_LEN(expected)                                                                                 \
    const static int param_idx = 0;                                                                               \
    do                                                                                                            \
    {                                                                                                             \
        if (params_len != (expected))                                                                             \
        {                                                                                                         \
            fprintf(stderr, "[%s] params len mismatch! expected %d actual %d\n", __func__, expected, params_len); \
            return -12;                                                                                           \
        }                                                                                                         \
    } while (0)

#define IN_RELATIVE_IDX(type, name, param_idx)                                                          \
    type name;                                                                                          \
    do                                                                                                  \
    {                                                                                                   \
        const uint64_t __idx = params[param_idx];                                                       \
        if (__idx >= MAX_IDX)                                                                           \
        {                                                                                               \
            fprintf(stderr, "[%s] IN_IDX(%s) out of range! the index limit is [0, 512) bug got: %lu\n", \
                    __func__, #name, __idx);                                                            \
            return -12;                                                                                 \
        }                                                                                               \
        name = (type)param_page[__idx];                                                                 \
        if (name == 0)                                                                                  \
        {                                                                                               \
            fprintf(stderr, "[%s] IN_IDX(%s) got null ptr!\n", __func__, #name);                        \
            return -14;                                                                                 \
        }                                                                                               \
    } while (0)

#define IN_ABSOLUTE_IDX(type, name, param_idx) \
    type name = params[param_idx];

#define OUT_RELATIVE_IDX(param_idx, val)                                                            \
    do                                                                                              \
    {                                                                                               \
        uint64_t __out_idx_##param_idx = params[param_idx];                                         \
        if (__out_idx_##param_idx >= MAX_IDX)                                                       \
        {                                                                                           \
            fprintf(stderr, "[%s] OUT_IDX out of range! the index limit is [0, 512) bug got %lu\n", \
                    __func__, param_idx);                                                           \
            return -12;                                                                             \
        }                                                                                           \
        param_page[__out_idx_##param_idx] = (uint64_t)val;                                          \
    } while (0)

#define OUT_ABSOLUTE_IDX(param_idx, val) \
    param_page[param_idx] = (uint64_t)val

#define IN_VALUE(type, name, param_idx) \
    type name = (type)params[param_idx]

#endif // EXPORT_FUNCTION_H
