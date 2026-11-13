"""Word-level timestamps for a narration WAV, using the locally cached Whisper
model through mlx-audio. Offline: it never downloads weights.

Usage: .venv/bin/python words.py NARRATION.wav OUTPUT.json
Writes [{"text": "This", "start_ms": 0, "end_ms": 300}, ...].
"""
import json
import os
import sys

os.environ.setdefault("HF_HUB_OFFLINE", "1")

from mlx_audio.stt.utils import load_model  # noqa: E402

MODEL = "mlx-community/whisper-large-v3-turbo-asr-fp16"


def words_from(result):
    out = []
    for segment in getattr(result, "segments", None) or []:
        items = segment.get("words") if isinstance(segment, dict) else getattr(segment, "words", None)
        for w in items or []:
            get = (lambda k: w[k]) if isinstance(w, dict) else (lambda k: getattr(w, k))
            text = str(get("word")).strip()
            if text:
                out.append({"text": text,
                            "start_ms": round(float(get("start")) * 1000),
                            "end_ms": round(float(get("end")) * 1000)})
    return out


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    audio, dest = sys.argv[1:]
    model = load_model(MODEL)
    result = model.generate(audio, word_timestamps=True, language="en")
    words = words_from(result)
    if not words:
        sys.exit("no word timestamps returned; result type: %r" % type(result))
    with open(dest, "w") as f:
        json.dump(words, f, indent=1)
    print(f"{len(words)} words -> {dest}")


if __name__ == "__main__":
    main()
