# Product Intent (PRD)

This current product-intent record is maintained by `pdd intent apply`.
Each accepted change links to an immutable intent event.

<!-- pdd-intent-entry:voxcpm2-voice-clone-tts-for-macos-3b8a90ac:start -->
## VoxCPM2 voice-clone TTS for macOS

- Intent event: [`docs/intents/intent__voxcpm2-voice-clone-tts-for-macos-3b8a90ac.md`](intents/intent__voxcpm2-voice-clone-tts-for-macos-3b8a90ac.md)
- Change kind: `add`
- Supersedes: none
- Scope: `new_subproject_design`
- Technology: `Rust (macOS Apple Silicon; candle Metal native engine; Python mlx-audio reference engine only as fallback/oracle)`

> Given the model mlx-community/VoxCPM2-bf16 (https://huggingface.co/mlx-community/VoxCPM2-bf16) and the fact that we're on a MacBook Pro, make a macOS device-specific deployment of this text-to-speech open-weight model. We want to run it on macOS devices, specifically my devices: a MacBook Pro and a Mac Mini M4, both with the maximum RAM. Look at how earlier text-to-speech "deployments" were implemented in the InServiceOfX repository (CLITextToSpeech), but don't limit yourself to that, because at the time all code was written by hand. Code it up, deploy, and test on your own.
>
> As a user I want to provide a .mp3 or .wav file of one person's voice for the model to clone, and, given text of reasonable size (at least enough spoken word for a short-format video), get speech in that voice. The output can be produced in multiple parts AS LONG AS a tool automatically stitches them back together. As a user I guarantee the sample is just one person's voice with as little background noise as possible.
>
> As a user I want an application layer on top of the open-weight model, either a command line (CLI) or a stand-alone macOS .app (macOS-specific, no cross-platform concerns, no browser). I want the same features in the CLI and the Mac application, but start with the CLI.
>
> As a user I want the weights loaded automatically, and when I close the application (CLI or GUI) the weights are freed from RAM/memory.
>
> As a user I want to either type into a simple text editor (take cues from nocode) or, in the CLI, just provide the full path to the text file I want transformed into speech.
>
> As a user, given the text length, I want the tool to automatically divide the text into smaller parts for the TTS model if necessary (per the model card and documentation), convert each into speech as .wav, aggregate them into one total .wav, and discard the smaller parts, automatically cleaning up every time.
>
> As a user I'd like to see progress, in the CLI or GUI, and an estimate of how long it will take.
>
> As a user I prefer all of this to be written in Rust. If something is only possible in Python, Python is acceptable as a last resort, but prefer reverse engineering or porting to Rust (researching GitHub code, porting, or reverse engineering binaries).
>
> Refine these user stories, do PDD in claw-work, find the right subdirectory, and implement and test until a successful product is delivered.
<!-- pdd-intent-entry:voxcpm2-voice-clone-tts-for-macos-3b8a90ac:end -->
