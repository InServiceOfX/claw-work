#include "claw_lm.h"
#include "Runtime/Resources.h"
#include "Numerics/Kernels.cuh"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <cstdlib>
#include <fstream>
#include <map>
#include <memory>
#include <vector>

namespace Claw::Model
{
using namespace Runtime;
using Numerics::Bf16;
class Decoder
{
  ClmConfig c_;
  int device_;
  size_t expected_bytes_, uploaded_bytes_{};
  int prefix_{};
  std::string trace_directory_;
  bool traced_{};
  Stream stream_;
  Blas blas_;
  std::map<std::string, Buffer<Bf16>> weights_;
  Buffer<float> cache_k_, cache_v_, x_, norm_, q_, k_, v_, attention_, projection_, gate_, up_, logits_;
  Buffer<Bf16> matrix_input_;
  Buffer<float> frequencies_;
  Buffer<uint32_t> ids_;

  void capture(const std::string& name, const float* data, size_t count)
  {
    if (trace_directory_.empty() || traced_) return;
    std::vector<float> host(count);
    check(cudaMemcpyAsync(host.data(), data, count * sizeof(float), cudaMemcpyDeviceToHost, stream_));
    check(cudaStreamSynchronize(stream_));
    std::ofstream file{trace_directory_ + "/" + name + ".f32", std::ios::binary};
    file.write(reinterpret_cast<const char*>(host.data()), count * sizeof(float));
    if (!file) throw std::runtime_error("Cannot write explicitly requested intermediate capture");
  }
  const Bf16* weight(const std::string& name) const
  {
    auto it = weights_.find(name);
    if (it == weights_.end()) throw std::runtime_error("Missing uploaded weight " + name);
    return it->second.data();
  }
  void linear(const float* input, const std::string& name, float* out, int rows, int columns, int inner)
  {
    Numerics::matrix_input(input, matrix_input_.data(), rows * inner, stream_);
    const float alpha = 1, beta = 0;
    check(cublasGemmEx(blas_, CUBLAS_OP_T, CUBLAS_OP_N, columns, rows, inner,
      &alpha, weight(name), CUDA_R_16BF, inner, matrix_input_.data(), CUDA_R_16BF, inner,
      &beta, out, CUDA_R_32F, columns, CUBLAS_COMPUTE_32F, CUBLAS_GEMM_DEFAULT_TENSOR_OP));
  }
public:
  static void validate_device(int device, const ClmConfig& c, size_t bytes)
  {
    cudaDeviceProp p{};
    check(cudaGetDeviceProperties(&p, device));
    std::string name{p.name};
    bool supported = (p.major == 8 && p.minor == 6 && name.find("GeForce RTX 30") != std::string::npos) ||
      (p.major == 12 && p.minor == 0 && name.find("GeForce RTX 50") != std::string::npos);
    if (!supported) throw std::runtime_error("Unsupported GPU: " + name + "; choose an RTX 30 (sm_86) or RTX 50 (sm_120) device explicitly");
    if (c.head_dim != 128 || c.context <= 0 || c.chunk <= 0 || c.chunk > 128 ||
        c.hidden <= 0 || c.intermediate <= 0 || c.layers <= 0 || c.query_heads <= 0 ||
        c.kv_heads <= 0 || c.query_heads % c.kv_heads || c.original_context <= 0 ||
        c.hidden > 16384 || c.intermediate > 65536 || c.layers > 64 || c.query_heads > 128 ||
        c.kv_heads > 128 || c.vocabulary <= 0 || c.vocabulary > 1048576 || c.context > 1048576)
      throw std::runtime_error("Invalid native model dimensions (head_dim must be 128)");
    size_t free{}, total{};
    check(cudaMemGetInfo(&free, &total));
    size_t cache = static_cast<size_t>(c.layers) * c.context * c.kv_heads * 128 * sizeof(float) * 2;
    size_t scratch = static_cast<size_t>(c.chunk) * (c.hidden * 3 + c.query_heads * 128 * 2 + c.kv_heads * 128 * 2 + c.intermediate * 2 + c.vocabulary) * sizeof(float)
      + static_cast<size_t>(c.chunk) * std::max({c.hidden, c.intermediate, c.query_heads * 128}) * sizeof(Bf16) + 4096;
    if (bytes > free || cache > free - bytes || scratch + (64ULL << 20) > free - bytes - cache)
      throw std::runtime_error("Insufficient GPU memory for BF16 weights, context KV and scratch; lower --context or select a device with more free VRAM");
  }
  Decoder(const ClmConfig& c, int device, size_t bytes): c_{c}, device_{device}, expected_bytes_{bytes}
  {
    if (const char* trace = std::getenv("CLAW_LM_TRACE_DIR")) trace_directory_ = trace;
    check(cublasSetStream(blas_, stream_));
    // Preserve FP32 reduction for BF16 outputs, including cuBLAS split-K choices.
    check(cublasSetMathMode(blas_, CUBLAS_MATH_DISALLOW_REDUCED_PRECISION_REDUCTION));
    size_t rows = c.chunk;
    size_t cache = static_cast<size_t>(c.layers) * c.context * c.kv_heads * 128;
    cache_k_ = Buffer<float>{cache}; cache_v_ = Buffer<float>{cache};
    x_ = Buffer<float>{rows * c.hidden}; norm_ = Buffer<float>{rows * c.hidden};
    q_ = Buffer<float>{rows * c.query_heads * 128}; k_ = Buffer<float>{rows * c.kv_heads * 128};
    v_ = Buffer<float>{rows * c.kv_heads * 128}; attention_ = Buffer<float>{rows * c.query_heads * 128};
    projection_ = Buffer<float>{rows * c.hidden}; gate_ = Buffer<float>{rows * c.intermediate}; up_ = Buffer<float>{rows * c.intermediate};
    logits_ = Buffer<float>{rows * c.vocabulary};
    matrix_input_ = Buffer<Bf16>{rows * std::max({c.hidden, c.intermediate, c.query_heads * 128})};
    ids_ = Buffer<uint32_t>{rows}; frequencies_ = Buffer<float>{64};
    std::vector<float> frequencies(64);
    auto correction = [&](double rotations) { return 128 * std::log(c.original_context / (rotations * 2 * 3.14159265358979323846)) / (2 * std::log(c.theta)); };
    double low = std::max(std::floor(correction(c.beta_fast)), 0.);
    double high = std::min(std::ceil(correction(c.beta_slow)), 127.);
    double width = low == high ? .001 : high - low;
    for (int j = 0; j < 64; ++j)
    { double ramp = std::clamp((j - low) / width, 0., 1.); frequencies[j] = std::pow(c.theta, -2. * j / 128) * (1 - ramp + ramp / c.factor); }
    check(cudaMemcpyAsync(frequencies_.data(), frequencies.data(), frequencies.size() * sizeof(float), cudaMemcpyHostToDevice, stream_));
    check(cudaStreamSynchronize(stream_));
  }
  int device() const { return device_; }
  void upload(const char* name, const void* data, size_t length)
  {
    if (!length || length % 2 || uploaded_bytes_ + length > expected_bytes_ || weights_.contains(name))
      throw std::runtime_error("Invalid or duplicate CUDA weight upload");
    Buffer<Bf16> buffer{length / 2};
    check(cudaMemcpyAsync(buffer.data(), data, length, cudaMemcpyHostToDevice, stream_));
    check(cudaStreamSynchronize(stream_));
    weights_.emplace(name, std::move(buffer)); uploaded_bytes_ += length;
  }
  void crop(size_t length)
  {
    if (length > static_cast<size_t>(prefix_)) throw std::runtime_error("Invalid CUDA cache crop");
    prefix_ = static_cast<int>(length);
  }
  void forward(const uint32_t* ids, int rows, bool causal, bool commit, bool last_only, float* host_logits)
  {
    if (uploaded_bytes_ != expected_bytes_ || rows <= 0 || rows > c_.chunk || prefix_ + rows > c_.context)
      throw std::runtime_error("Incomplete weights or forward exceeds configured chunk/context");
    for (int i = 0; i < rows; ++i) if (ids[i] >= static_cast<uint32_t>(c_.vocabulary)) throw std::runtime_error("Token outside vocabulary");
    check(cudaMemcpyAsync(ids_.data(), ids, rows * sizeof(uint32_t), cudaMemcpyHostToDevice, stream_));
    Numerics::embedding(ids_.data(), weight("encoder.embed_tokens.weight"), x_.data(), rows, c_.hidden, stream_);
    capture("embedding", x_.data(), static_cast<size_t>(rows) * c_.hidden);
    auto mscale = [&](double s) { return c_.factor <= 1 ? 1. : 1 + .1 * s * std::log(c_.factor); };
    float magnitude = mscale(c_.mscale) / mscale(c_.mscale_all_dim);
    for (int layer = 0; layer < c_.layers; ++layer)
    {
      std::string p = "encoder.layers." + std::to_string(layer) + ".";
      Numerics::rms(x_.data(), weight(p + "input_layernorm.weight"), norm_.data(), rows, c_.hidden, c_.epsilon, stream_);
      capture("layer-" + std::to_string(layer) + "-input-norm", norm_.data(), static_cast<size_t>(rows) * c_.hidden);
      linear(norm_.data(), p + "self_attn.q_proj.weight", q_.data(), rows, c_.query_heads * 128, c_.hidden);
      linear(norm_.data(), p + "self_attn.k_proj.weight", k_.data(), rows, c_.kv_heads * 128, c_.hidden);
      linear(norm_.data(), p + "self_attn.v_proj.weight", v_.data(), rows, c_.kv_heads * 128, c_.hidden);
      Numerics::rotary(q_.data(), frequencies_.data(), rows, c_.query_heads, prefix_, c_.original_context, c_.query_scale_beta, magnitude, stream_);
      Numerics::rotary(k_.data(), frequencies_.data(), rows, c_.kv_heads, prefix_, c_.original_context, 0, magnitude, stream_);
      capture("layer-" + std::to_string(layer) + "-query", q_.data(), static_cast<size_t>(rows) * c_.query_heads * 128);
      capture("layer-" + std::to_string(layer) + "-key", k_.data(), static_cast<size_t>(rows) * c_.kv_heads * 128);
      capture("layer-" + std::to_string(layer) + "-value", v_.data(), static_cast<size_t>(rows) * c_.kv_heads * 128);
      size_t layer_stride = static_cast<size_t>(c_.context) * c_.kv_heads * 128;
      float* pk = cache_k_.data() + layer * layer_stride;
      float* pv = cache_v_.data() + layer * layer_stride;
      Numerics::attention(q_.data(), k_.data(), v_.data(), pk, pv, attention_.data(), rows, c_.query_heads, c_.kv_heads, prefix_, causal, stream_);
      capture("layer-" + std::to_string(layer) + "-attention", attention_.data(), static_cast<size_t>(rows) * c_.query_heads * 128);
      if (commit)
      {
        size_t offset = static_cast<size_t>(prefix_) * c_.kv_heads * 128;
        size_t length = static_cast<size_t>(rows) * c_.kv_heads * 128 * sizeof(float);
        check(cudaMemcpyAsync(pk + offset, k_.data(), length, cudaMemcpyDeviceToDevice, stream_));
        check(cudaMemcpyAsync(pv + offset, v_.data(), length, cudaMemcpyDeviceToDevice, stream_));
      }
      linear(attention_.data(), p + "self_attn.o_proj.weight", projection_.data(), rows, c_.hidden, c_.query_heads * 128);
      Numerics::add(x_.data(), projection_.data(), rows * c_.hidden, stream_);
      Numerics::rms(x_.data(), weight(p + "post_attention_layernorm.weight"), norm_.data(), rows, c_.hidden, c_.epsilon, stream_);
      capture("layer-" + std::to_string(layer) + "-post-norm", norm_.data(), static_cast<size_t>(rows) * c_.hidden);
      linear(norm_.data(), p + "mlp.gate_proj.weight", gate_.data(), rows, c_.intermediate, c_.hidden);
      linear(norm_.data(), p + "mlp.up_proj.weight", up_.data(), rows, c_.intermediate, c_.hidden);
      Numerics::gated_silu(gate_.data(), up_.data(), rows * c_.intermediate, stream_);
      capture("layer-" + std::to_string(layer) + "-gate-product", gate_.data(), static_cast<size_t>(rows) * c_.intermediate);
      linear(gate_.data(), p + "mlp.down_proj.weight", projection_.data(), rows, c_.hidden, c_.intermediate);
      Numerics::add(x_.data(), projection_.data(), rows * c_.hidden, stream_);
      capture("layer-" + std::to_string(layer) + "-output", x_.data(), static_cast<size_t>(rows) * c_.hidden);
    }
    int output_rows = last_only ? 1 : rows;
    const float* output = x_.data() + (last_only ? (rows - 1) * c_.hidden : 0);
    Numerics::rms(output, weight("encoder.norm.weight"), norm_.data(), output_rows, c_.hidden, c_.epsilon, stream_);
    capture("final-norm", norm_.data(), static_cast<size_t>(output_rows) * c_.hidden);
    linear(norm_.data(), "diffusion_head.weight", logits_.data(), output_rows, c_.vocabulary, c_.hidden);
    check(cudaGetLastError());
    check(cudaMemcpyAsync(host_logits, logits_.data(), static_cast<size_t>(output_rows) * c_.vocabulary * sizeof(float), cudaMemcpyDeviceToHost, stream_));
    check(cudaStreamSynchronize(stream_));
    traced_ = true;
    if (commit) prefix_ += rows;
  }
};
} // namespace Claw::Model

namespace
{
void set_error(char* error, size_t capacity, const std::exception& e)
{ if (capacity) { std::strncpy(error, e.what(), capacity - 1); error[capacity - 1] = 0; } }
}
extern "C" void* clm_create(const ClmConfig* c, int device, size_t bytes, char* error, size_t capacity)
{
  try
  {
    Claw::Runtime::DeviceScope scope{device};
    Claw::Model::Decoder::validate_device(device, *c, bytes);
    return new Claw::Model::Decoder{*c, device, bytes};
  }
  catch (const std::exception& e) { set_error(error, capacity, e); return nullptr; }
}
extern "C" int clm_upload(void* opaque, const char* name, const void* data, size_t length, char* error, size_t capacity)
{
  try { auto* d = static_cast<Claw::Model::Decoder*>(opaque); Claw::Runtime::DeviceScope scope{d->device()}; d->upload(name, data, length); return 0; }
  catch (const std::exception& e) { set_error(error, capacity, e); return 1; }
}
extern "C" int clm_forward(void* opaque, const uint32_t* ids, size_t rows, int causal, int commit, int last, float* logits, char* error, size_t capacity)
{
  try { auto* d = static_cast<Claw::Model::Decoder*>(opaque); Claw::Runtime::DeviceScope scope{d->device()}; d->forward(ids, static_cast<int>(rows), causal, commit, last, logits); return 0; }
  catch (const std::exception& e) { set_error(error, capacity, e); return 1; }
}
extern "C" int clm_crop(void* opaque, size_t length, char* error, size_t capacity)
{
  try { static_cast<Claw::Model::Decoder*>(opaque)->crop(length); return 0; }
  catch (const std::exception& e) { set_error(error, capacity, e); return 1; }
}
extern "C" void clm_destroy(void* opaque)
{
  if (!opaque) return;
  auto* d = static_cast<Claw::Model::Decoder*>(opaque);
  int previous{};
  if (cudaGetDevice(&previous) == cudaSuccess && cudaSetDevice(d->device()) == cudaSuccess)
  { delete d; cudaSetDevice(previous); }
}

extern "C" int clm_device_count(int* count, char* error, size_t capacity)
{
  try { Claw::Runtime::check(cudaGetDeviceCount(count)); return 0; }
  catch (const std::exception& e) { set_error(error, capacity, e); return 1; }
}
extern "C" int clm_device_info(int index, char* name, size_t name_size, int* major, int* minor, char* error, size_t capacity)
{
  try { cudaDeviceProp p{}; Claw::Runtime::check(cudaGetDeviceProperties(&p, index));
    if (name_size) { std::strncpy(name, p.name, name_size - 1); name[name_size - 1] = 0; }
    *major = p.major; *minor = p.minor; return 0; }
  catch (const std::exception& e) { set_error(error, capacity, e); return 1; }
}
