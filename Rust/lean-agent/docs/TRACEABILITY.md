# Authored story, contract and evidence map

This is a reviewed-intent implementation with manually maintained prompt/story units.
It is not a claim of completed PDD generation/sync or semantic story validation.
See PROGRESS.md for the interrupted automated architecture operation.

| Story | Contract | Implementation | Evidence |
|---|---|---|---|
| offline mathlib container | container_shell.prompt R-101–R-110 | Dockerfile, Dockerfile.source, docker/ | container_smoke.sh, source_smoke.sh, lsp_live.rs, container_metrics example |
| mathematical review | input_rust.prompt R-201–R-206; report_rust.prompt R-401–R-405 | input.rs, main.rs, proof.rs | input_privacy.rs, identity/withdrawal examples |
| proof trust | proof_rust.prompt R-301–R-312 | proof.rs, process.rs, immutable export-only judge | proof_live.rs, additional_live.rs, LSP feedback test |
| specification-derived improvement | workflow_markdown.prompt R-501–R-508 | portable skill, WORKFLOWS.md, authored examples | withdrawal/summation model checks, spec_derived Rust tests/benchmark |
| local harness setup | setup_python.prompt R-601–R-610 | scripts/, two self-contained skills, SETUP.md | test_setup.py, platform_pins.rs, native loader inspections and local Docker smoke |
| separate root public boundary | ../../prompts/public_boundary_rust.prompt R-001–R-006 | privacy.rs, artifact.rs, claw-public-check, root hook/CI | index/worktree disagreement, synthetic secrets/artifacts, actual context/image canaries |

Contract lint is deterministic structural evidence only. Functional negative tests,
actual container behavior, model proof verdicts and performance measurements are
separate evidence. Source mappings do not turn any of these into a source refinement
proof. Split obligation groups into separate manifests for property-by-property verdicts.
