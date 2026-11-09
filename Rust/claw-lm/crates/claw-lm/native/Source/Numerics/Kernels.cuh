#pragma once
#include <cstdint>
#include <cuda_bf16.h>
#include <cuda_runtime.h>
namespace Claw::Numerics
{
using Bf16 = __nv_bfloat16;
void embedding(const uint32_t*, const Bf16*, float*, int, int, cudaStream_t);
void rms(const float*, const Bf16*, float*, int, int, float, cudaStream_t);
void rotary(float*, const float*, int, int, int, int, float, float, cudaStream_t);
void attention(const float*, const float*, const float*, const float*, const float*,
               float*, int, int, int, int, bool, cudaStream_t);
void add(float*, const float*, int, cudaStream_t);
void gated_silu(float*, const float*, int, cudaStream_t);
void matrix_input(const float*, Bf16*, int, cudaStream_t);
}
