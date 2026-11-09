#include "Numerics/Kernels.cuh"
#include <cmath>
#if defined(__CUDA_ARCH__) && __CUDA_ARCH__ != 860 && __CUDA_ARCH__ != 1200
#error "claw-lm supports only native sm_86 and sm_120"
#endif
namespace Claw::Numerics
{
__device__ float warp_sum(float value)
{
  for (int delta = 16; delta; delta /= 2) value += __shfl_down_sync(0xffffffff, value, delta);
  return __shfl_sync(0xffffffff, value, 0);
}
__global__ void embedding_kernel(const uint32_t* ids, const Bf16* w, float* out, int total, int hidden)
{
  int i = blockIdx.x * blockDim.x + threadIdx.x;
  if (i < total) out[i] = __bfloat162float(w[static_cast<size_t>(ids[i / hidden]) * hidden + i % hidden]);
}
void embedding(const uint32_t* ids, const Bf16* w, float* out, int rows, int hidden, cudaStream_t s)
{ embedding_kernel<<<(rows * hidden + 255) / 256, 256, 0, s>>>(ids, w, out, rows * hidden, hidden); }
__global__ void rms_kernel(const float* x, const Bf16* w, float* out, int hidden, float epsilon)
{
  int row = blockIdx.x;
  float total = 0;
  for (int j = threadIdx.x; j < hidden; j += blockDim.x) { float a = x[row * hidden + j]; total += a * a; }
  total = warp_sum(total);
  __shared__ float partial[4];
  if (threadIdx.x % 32 == 0) partial[threadIdx.x / 32] = total;
  __syncthreads();
  float sum = partial[0] + partial[1] + partial[2] + partial[3];
  float scale = rsqrtf(sum / hidden + epsilon);
  for (int j = threadIdx.x; j < hidden; j += blockDim.x)
    out[row * hidden + j] = x[row * hidden + j] * scale * __bfloat162float(w[j]);
}
void rms(const float* x, const Bf16* w, float* out, int rows, int hidden, float e, cudaStream_t s)
{ rms_kernel<<<rows, 128, 0, s>>>(x, w, out, hidden, e); }
__global__ void rotary_kernel(float* x, const float* f, int pairs, int heads, int start, int original, float beta, float magnitude)
{
  int i = blockIdx.x * blockDim.x + threadIdx.x;
  if (i >= pairs) return;
  int pair = i % 64, head_row = i / 64, position = start + head_row / heads;
  float angle = position * f[pair];
  float cosine = cosf(angle) * magnitude, sine = sinf(angle) * magnitude;
  int index = head_row * 128 + pair;
  float a = x[index], b = x[index + 64];
  float scale = 1.f + beta * log1pf(position / original);
  x[index] = (a * cosine - b * sine) * scale;
  x[index + 64] = (b * cosine + a * sine) * scale;
}
void rotary(float* x, const float* f, int rows, int heads, int start, int original, float beta, float magnitude, cudaStream_t s)
{ int n = rows * heads * 64; rotary_kernel<<<(n + 255) / 256, 256, 0, s>>>(x, f, n, heads, start, original, beta, magnitude); }
// One warp per query/head; online softmax keeps state in FP32 without S/P matrices.
__global__ void attention_kernel(const float* q, const float* k, const float* v,
  const float* pk, const float* pv, float* out, int rows, int qh, int kh, int prefix, bool causal)
{
  int item = (blockIdx.x * blockDim.x + threadIdx.x) / 32, lane = threadIdx.x % 32;
  if (item >= rows * qh) return;
  int row = item / qh, head = item % qh, kv_head = head / (qh / kh);
  float query[4], accumulator[4] = {0, 0, 0, 0};
  for (int d = 0; d < 4; ++d) query[d] = q[item * 128 + lane + d * 32];
  float maximum = -INFINITY, normalizer = 0;
  int end = causal ? prefix + row + 1 : prefix + rows;
  for (int t = 0; t < end; ++t)
  {
    const float* key = (t < prefix ? pk : k) + static_cast<size_t>(t < prefix ? t : t - prefix) * kh * 128 + kv_head * 128;
    const float* value = (t < prefix ? pv : v) + static_cast<size_t>(t < prefix ? t : t - prefix) * kh * 128 + kv_head * 128;
    float score = 0;
    for (int d = 0; d < 4; ++d) score += query[d] * key[lane + d * 32];
    score = warp_sum(score) * 0.08838834764831845f;
    float next_maximum = fmaxf(maximum, score), old_scale = expf(maximum - next_maximum), weight = expf(score - next_maximum);
    normalizer = normalizer * old_scale + weight;
    for (int d = 0; d < 4; ++d) accumulator[d] = accumulator[d] * old_scale + weight * value[lane + d * 32];
    maximum = next_maximum;
  }
  for (int d = 0; d < 4; ++d) out[item * 128 + lane + d * 32] = accumulator[d] / normalizer;
}
void attention(const float* q, const float* k, const float* v, const float* pk, const float* pv, float* out, int rows, int qh, int kh, int prefix, bool causal, cudaStream_t s)
{ attention_kernel<<<(rows * qh + 3) / 4, 128, 0, s>>>(q, k, v, pk, pv, out, rows, qh, kh, prefix, causal); }
__global__ void add_kernel(float* x, const float* y, int n)
{ int i = blockIdx.x * blockDim.x + threadIdx.x; if (i < n) x[i] += y[i]; }
void add(float* x, const float* y, int n, cudaStream_t s)
{ add_kernel<<<(n + 255) / 256, 256, 0, s>>>(x, y, n); }
__global__ void silu_kernel(float* x, const float* y, int n)
{ int i = blockIdx.x * blockDim.x + threadIdx.x; if (i < n) x[i] = x[i] / (1.f + expf(-x[i])) * y[i]; }
void gated_silu(float* x, const float* y, int n, cudaStream_t s)
{ silu_kernel<<<(n + 255) / 256, 256, 0, s>>>(x, y, n); }
__global__ void input_kernel(const float* x, Bf16* y, int n)
{ int i = blockIdx.x * blockDim.x + threadIdx.x; if (i < n) y[i] = __float2bfloat16(x[i]); }
void matrix_input(const float* x, Bf16* y, int n, cudaStream_t s)
{ input_kernel<<<(n + 255) / 256, 256, 0, s>>>(x, y, n); }
} // namespace Claw::Numerics
