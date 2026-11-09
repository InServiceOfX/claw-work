# Public artifact provenance

This project contains original Rust implementation code. It reads public model
and patch artifacts; those large files and their surrounding logs are not bundled.

- Model: [Qwen/Qwen3.5-0.8B](https://huggingface.co/Qwen/Qwen3.5-0.8B),
  source revision `2fc06364715b967f1860aea9cf38778875588b17`, Apache-2.0.
- BF16 GGUF: [ggml-org/Qwen3.5-0.8B-GGUF](https://huggingface.co/ggml-org/Qwen3.5-0.8B-GGUF),
  revision `807dfd9a6fadbf703207aac3bfb0b2b3040d077c`, Apache-2.0.
- Sparse edit: Takehi, Teoh, Zhang and Khattab, *Sparks of Weight-Space Programming*,
  [released Experiment 8](https://github.com/RikiyaT/lm_understand-logs/tree/4d09ba6d6a7e891dd8063974dfe81b3d78e3494f/exp8_math),
  revision `4d09ba6d6a7e891dd8063974dfe81b3d78e3494f`, `runs/p50c/edit/`.
  Data/weight artifacts are [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
  The upstream scripts have a separate MIT licence; this implementation does not
  execute or bundle those scripts. Model changes reproduce the released sparse
  BF16 values and are explicitly documented in the reconstruction receipt.
- Inference backend: [llama.cpp](https://github.com/ggml-org/llama.cpp), MIT,
  existing CUDA server image. The Docker image is a separate runtime dependency.

If redistributing the model, patch or edited model, preserve their original
licence notices and attribution. This repository does not grant new rights to
upstream benchmark content and does not redistribute the authors' transcripts.
