#include <stdint.h>
#include <stddef.h>

#if defined(_WIN32)
#define GANDR_EXPORT __declspec(dllexport)
#else
#define GANDR_EXPORT
#endif

GANDR_EXPORT int64_t gandr_test_add(int64_t left, int64_t right)
{
    return left + right;
}

GANDR_EXPORT uint64_t gandr_test_strlen(const char *text)
{
    uint64_t length = 0;
    while (text[length] != '\0') {
        length += 1;
    }
    return length;
}

GANDR_EXPORT int32_t gandr_add_i32(int32_t left, int32_t right)
{
    return left + right;
}

GANDR_EXPORT uint64_t gandr_identity_u64(uint64_t value)
{
    return value;
}

GANDR_EXPORT uint32_t gandr_identity_u32(uint32_t value)
{
    return value;
}

GANDR_EXPORT float gandr_identity_f32(float value)
{
    return value;
}

GANDR_EXPORT double gandr_identity_f64(double value)
{
    return value;
}

GANDR_EXPORT void *gandr_identity_ptr(void *value)
{
    return value;
}

GANDR_EXPORT const char *gandr_greeting(void)
{
    return "hello from testlib";
}

GANDR_EXPORT const char *gandr_null_string(void)
{
    return NULL;
}

GANDR_EXPORT const char *gandr_invalid_string(void)
{
    static const char invalid[] = "\xff";
    return invalid;
}

GANDR_EXPORT void gandr_void(void)
{
}

GANDR_EXPORT void exit(int32_t code)
{
    (void)code;
}
