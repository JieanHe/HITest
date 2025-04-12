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

#define GET_INPUT_IDX_NZ(type, name, param_idx)                                                \
    type name;                                                                                 \
    do                                                                                         \
    {                                                                                          \
        const int __idx = params[param_idx];                                                   \
        if (__idx >= MAX_IDX)                                                                  \
        {                                                                                      \
            fprintf(stderr, "[%s] input [%s=%d] out of range! the index limit is [0, 512) \n", \
                    __func__, #name, __idx);                                                   \
            return -12;                                                                        \
        }                                                                                      \
        name = (type)param_page[__idx];                                                        \
        if (name == 0)                                                                         \
        {                                                                                      \
            fprintf(stderr, "[%s] input [%s=%d] got nullptr!\n", __func__, #name, __idx);      \
            return -14;                                                                        \
        }                                                                                      \
    } while (0)

#define GET_INPUT_IDX(type, name, param_idx)                                                   \
    type name;                                                                                 \
    do                                                                                         \
    {                                                                                          \
        const int __idx = params[param_idx];                                                   \
        if (__idx >= MAX_IDX)                                                                  \
        {                                                                                      \
            fprintf(stderr, "[%s] input [%s=%d] out of range! the index limit is [0, 512) \n", \
                    __func__, #name, __idx);                                                   \
            return -12;                                                                        \
        }                                                                                      \
        name = (type)param_page[__idx];                                                        \
    } while (0)

#define GET_VALUE(type, name, param_idx) \
    type name = (type)params[param_idx];

#define SET_OUTPUT_IDX(param_idx, val)                                                     \
    do                                                                                     \
    {                                                                                      \
        if (param_idx >= MAX_IDX)                                                          \
        {                                                                                  \
            fprintf(stderr, "[%s] OUT_IDX %uout of range! the index limit is [0, 512) \n", \
                    __func__, param_idx);                                                  \
            return -12;                                                                    \
        }                                                                                  \
        param_page[param_idx] = (uint64_t)val;                                     \
    } while (0)

#endif // EXPORT_FUNCTION_H
