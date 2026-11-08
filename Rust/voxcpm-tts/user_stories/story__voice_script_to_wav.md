# A script spoken in my reference voice

As a Mac user, I provide a clean WAV or MP3 containing one person's voice and a
text-file path, and receive one WAV speaking my full script in that voice.
Long scripts split and rejoin automatically. Progress and estimated completion
remain visible; closing the tool releases the weights. The original voice file
remains unchanged, and intermediate parts are cleaned automatically.

Source: the original product request preserved in docs/intents/.
The requested CLI precedes a standalone macOS app sharing its core features.
Rust is preferred; Python is an acceptable last resort for inference.

Acceptance examples: a short video script spanning several chunks; a source
path with spaces; a silent or missing reference rejected clearly; cancellation
leaving no final output or temporary parts; explicit model unload and reload.

<!-- pdd-story-prompts: job_rust.prompt, native_engine_rust.prompt, worker_python.prompt -->
