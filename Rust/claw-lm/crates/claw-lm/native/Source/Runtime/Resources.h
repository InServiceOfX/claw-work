#pragma once
#include <cuda_runtime.h>
#include <cublas_v2.h>
#include <stdexcept>
#include <string>
#include <utility>

namespace Claw::Runtime
{
inline void check(cudaError_t status)
{
  if (status != cudaSuccess) throw std::runtime_error(cudaGetErrorString(status));
}
inline void check(cublasStatus_t status)
{
  if (status != CUBLAS_STATUS_SUCCESS)
    throw std::runtime_error("cuBLAS status " + std::to_string(static_cast<int>(status)));
}
class DeviceScope
{
  int previous_{};
public:
  explicit DeviceScope(int device) { check(cudaGetDevice(&previous_)); check(cudaSetDevice(device)); }
  ~DeviceScope() { cudaSetDevice(previous_); }
};
class Stream
{
  cudaStream_t stream_{};
public:
  Stream() { check(cudaStreamCreateWithFlags(&stream_, cudaStreamNonBlocking)); }
  ~Stream() { if (stream_) cudaStreamDestroy(stream_); }
  Stream(const Stream&) = delete;
  Stream& operator=(const Stream&) = delete;
  operator cudaStream_t() const { return stream_; }
};
class Blas
{
  cublasHandle_t handle_{};
public:
  Blas() { check(cublasCreate(&handle_)); }
  ~Blas() { if (handle_) cublasDestroy(handle_); }
  Blas(const Blas&) = delete;
  Blas& operator=(const Blas&) = delete;
  operator cublasHandle_t() const { return handle_; }
};
template<class T> class Buffer
{
  T* pointer_{};
  size_t count_{};
public:
  Buffer() = default;
  explicit Buffer(size_t count): count_{count}
  {
    if (count > SIZE_MAX / sizeof(T)) throw std::runtime_error("CUDA allocation size overflow");
    if (count) check(cudaMalloc(reinterpret_cast<void**>(&pointer_), count * sizeof(T)));
  }
  ~Buffer() { if (pointer_) cudaFree(pointer_); }
  Buffer(const Buffer&) = delete;
  Buffer& operator=(const Buffer&) = delete;
  Buffer(Buffer&& other) noexcept: pointer_{std::exchange(other.pointer_, nullptr)}, count_{other.count_} {}
  Buffer& operator=(Buffer&& other) noexcept
  {
    if (this != &other) { if (pointer_) cudaFree(pointer_); pointer_ = std::exchange(other.pointer_, nullptr); count_ = other.count_; }
    return *this;
  }
  T* data() const { return pointer_; }
  size_t size() const { return count_; }
};
} // namespace Claw::Runtime
