#pragma once
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
struct ClmConfig {
  int hidden, intermediate, layers, query_heads, kv_heads, head_dim, vocabulary;
  int context, chunk, original_context;
  float epsilon, query_scale_beta;
  double theta, factor, beta_fast, beta_slow, mscale, mscale_all_dim;
};
int clm_device_count(int* count, char*, size_t);
int clm_device_info(int index, char* name, size_t name_size, int* major, int* minor, char*, size_t);
void* clm_create(const struct ClmConfig*, int device, size_t weight_bytes, char* error, size_t capacity);
int clm_upload(void*, const char* name, const void* bytes, size_t length, char*, size_t);
int clm_forward(void*, const uint32_t* tokens, size_t count, int causal, int commit,
                int last_only, float* logits, char*, size_t);
int clm_crop(void*, size_t length, char*, size_t);
void clm_destroy(void*);
#ifdef __cplusplus
}
#endif
