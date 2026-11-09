#!/usr/bin/env python3
"""Send a synthetic solid-red PNG to a local profile with a loaded projector."""
import argparse
import base64
import json
import struct
import time
import urllib.parse
import urllib.request
import zlib


def red_png():
    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xffffffff)
    header = struct.pack(">IIBBBBB", 128, 128, 8, 2, 0, 0, 0)
    pixels = (b"\0" + bytes([255, 0, 0]) * 128) * 128
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(pixels)) + chunk(b"IEND", b"")


def run(base):
    parsed = urllib.parse.urlparse(base)
    if parsed.scheme != "http" or parsed.hostname not in ("127.0.0.1", "localhost"):
        raise ValueError("Use a loopback server")
    with urllib.request.urlopen(base + "/v1/models", timeout=5) as response:
        model = json.load(response)["data"][0]["id"]
    encoded = base64.b64encode(red_png()).decode()
    payload = {"model": model, "messages": [{"role": "user", "content": [
        {"type": "text", "text": "What is the dominant color in this image? Answer one word."},
        {"type": "image_url", "image_url": {"url": "data:image/png;base64," + encoded}}]}],
        "max_tokens": 1024, "temperature": 0.6, "top_p": 0.95, "top_k": 20, "seed": 42}
    request = urllib.request.Request(base + "/v1/chat/completions", json.dumps(payload).encode(), {"Content-Type": "application/json"})
    start = time.monotonic()
    with urllib.request.urlopen(request, timeout=150) as response:
        data = json.load(response)
    choice = data["choices"][0]
    answer = (choice["message"].get("content") or "").strip().strip(".! ").lower()
    return {"model": model, "passed": choice["finish_reason"] == "stop" and answer == "red",
            "answer": answer, "seconds": time.monotonic() - start, "usage": data.get("usage"),
            "scope": "One synthetic-color smoke test, not a vision accuracy benchmark"}

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default="http://127.0.0.1:8080")
    args = parser.parse_args()
    result = run(args.url.rstrip("/"))
    print(json.dumps(result))
    raise SystemExit(0 if result["passed"] else 1)
