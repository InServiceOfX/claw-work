cmake_minimum_required(VERSION 3.24)
project(claw_lm_cuda LANGUAGES CXX CUDA)
find_package(CUDAToolkit 13 REQUIRED)
add_library(claw_lm_cuda STATIC Source/Numerics/Kernels.cu Source/Model/Decoder.cu)
target_include_directories(claw_lm_cuda PUBLIC Include PRIVATE Source)
set_target_properties(claw_lm_cuda PROPERTIES
  CXX_STANDARD 20 CUDA_STANDARD 20 POSITION_INDEPENDENT_CODE ON
  CUDA_ARCHITECTURES "86-real;120-real")
target_link_libraries(claw_lm_cuda PUBLIC CUDA::cudart CUDA::cublas)
