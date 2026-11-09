#!/usr/bin/env python3
"""Measure one already-running local model; launch GPU backends sequentially."""
import argparse
import json
import pathlib
import statistics
import subprocess
import threading
import time
import urllib.parse
import urllib.request

LONG_PROMPT = "Write a long tutorial explaining how to merge two sorted integer arrays. Include duplicates, empty inputs, negative numbers, complexity, and several step-by-step examples. Write at least 600 words. Do not call tools."
PREFIX_PROMPT = ("This is synthetic reference data for a local latency measurement. " * 500) + "\nReply with exactly READY. Do not call tools."
RUST_PROMPT = "Write a Rust function that merges two sorted slices of i32 into a sorted Vec<i32>. Preserve duplicates. Include tests for empty inputs, negative numbers, and duplicates. Show the code without using any tools."
PRIME_PROMPT = "List the first 20 prime numbers, in ascending order, separated by commas. Output only the numbers."
TOOLS = [{"type": "function", "function": {"name": "add", "description": "Add two integers.", "parameters": {"type": "object", "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}}, "required": ["a", "b"]}}}]


def stream_request(url, body, raw_path):
    request = urllib.request.Request(url + "/v1/chat/completions", json.dumps(body).encode(), {"Content-Type": "application/json"})
    start = time.monotonic()
    first_text = first_answer = None
    text = []; thoughts = []; usage = None; reason = None; metadata = None
    done = False
    with urllib.request.urlopen(request, timeout=240) as response, raw_path.open("wb") as capture:
        for line in response:
            capture.write(line)
            if not line.startswith(b"data: "):
                continue
            raw = line[6:].strip()
            if raw == b"[DONE]":
                done = True
                break
            event = json.loads(raw)
            if "error" in event:
                raise RuntimeError(event["error"].get("message", str(event["error"])))
            now = time.monotonic() - start
            for choice in event.get("choices", []):
                delta = choice.get("delta", {})
                content = delta.get("content") or ""
                thinking = delta.get("reasoning_content") or delta.get("reasoning") or ""
                if (content.strip() or thinking.strip()) and first_text is None:
                    first_text = now
                if content.strip() and first_answer is None:
                    first_answer = now
                text.append(content); thoughts.append(thinking)
                reason = choice.get("finish_reason") or reason
            usage = event.get("usage") or usage
            metadata = event.get("claw") or metadata
    seconds = time.monotonic() - start
    if not done or not usage:
        raise RuntimeError("Incomplete SSE stream or missing usage")
    tokens = usage["completion_tokens"]
    result = {"seconds": seconds, "completion_tokens": tokens, "prompt_tokens": usage["prompt_tokens"],
              "tokens_per_second_end_to_end": tokens / seconds, "first_text_seconds": first_text,
              "first_answer_seconds": first_answer, "answer_characters": len("".join(text)),
              "thinking_characters": len("".join(thoughts)), "finish_reason": reason,
              "backend_metadata": metadata}
    raw_path.with_suffix(".answer.txt").write_text("".join(text))
    return result


def gpu_usage(index):
    return int(subprocess.check_output(["nvidia-smi", "-i", str(index), "--query-gpu=memory.used", "--format=csv,noheader,nounits"], text=True, timeout=5).strip())


def run(args):
    parsed = urllib.parse.urlparse(args.url)
    if parsed.scheme != "http" or parsed.hostname not in ("127.0.0.1", "localhost"):
        raise ValueError("Use an explicit loopback endpoint")
    output = pathlib.Path(args.out).resolve()
    if any((parent / ".git").exists() for parent in [output, *output.parents]):
        raise ValueError("Raw captures must be outside Git")
    output.mkdir(parents=True, exist_ok=True); output.chmod(0o700)
    with urllib.request.urlopen(args.url + "/v1/models", timeout=10) as response:
        cards = json.load(response)["data"]
    card = next(c for c in cards if c["id"] == args.model)
    context = card.get("context_window") or card.get("meta", {}).get("n_ctx")
    if context != 16384:
        raise ValueError("Run this common-context comparison with a 16384-token server")
    def body(prompt, limit, tools=False):
        data = {"model": args.model, "messages": [{"role": "user", "content": prompt}], "stream": True,
                "stream_options": {"include_usage": True}, "max_tokens": limit}
        if args.backend == "native":
            data.update(temperature=0, block_length=32, threshold=0.9)
        else:
            data.update(temperature=0.6, top_p=0.95, top_k=20, min_p=0.0, repeat_penalty=1.05,
                        seed=42, reasoning_effort="low", cache_prompt=False)
        if tools:
            data["tools"] = TOOLS
        return data
    samples = []; stopped = threading.Event()
    def sample():
        while not stopped.is_set():
            try:samples.append(gpu_usage(args.gpu_index))
            except (OSError, ValueError, subprocess.SubprocessError):pass
            stopped.wait(0.2)
    sampler = threading.Thread(target=sample, daemon=True)
    idle = gpu_usage(args.gpu_index); sampler.start()
    results = []
    try:
        stream_request(args.url, body("Say hello in one short sentence. Do not call tools.", 1024), output / "warmup.sse")
        for repeat in range(args.repetitions):
            result = stream_request(args.url, body(LONG_PROMPT, 512), output / f"long-{repeat}.sse")
            result.update(case="long_generation", repetition=repeat)
            results.append(result); print(json.dumps({"label":args.label, **result}), flush=True)
        for name, prompt, limit, tools in [("primes", PRIME_PROMPT, 512, False), ("prefix_latency", PREFIX_PROMPT, 128, False), ("rust_with_tools_declared", RUST_PROMPT, 2048, True)]:
            try:
                result = stream_request(args.url, body(prompt, limit, tools), output / (name + ".sse"))
                result.update(case=name); results.append(result)
                print(json.dumps({"label":args.label, **result}), flush=True)
            except Exception as error:
                results.append({"case":name,"error":type(error).__name__+": "+str(error)})
                print(json.dumps({"label":args.label, **results[-1]}), flush=True)
    finally:
        stopped.set(); sampler.join(timeout=6)
    sustained = [r for r in results if r["case"] == "long_generation"]
    summary = {"label":args.label,"model":args.model,"backend":args.backend,"context":16384,
               "repetitions":args.repetitions,"median_tokens_per_second_end_to_end":statistics.median(r["tokens_per_second_end_to_end"] for r in sustained),
               "median_first_text_seconds":statistics.median(r["first_text_seconds"] for r in sustained if r["first_text_seconds"] is not None),
               "idle_gpu_mib":idle,"sampled_peak_gpu_mib":max(samples) if samples else idle,
               "results":results,"scope":"Short fresh requests; includes prefill/HTTP overhead and all generated reasoning tokens. Tokenizers, precision and implementations differ; this is not an isolated decoding-algorithm benchmark."}
    (output / "summary.json").write_text(json.dumps(summary,indent=2)+"\n")
    print(json.dumps({k:v for k,v in summary.items() if k!='results'}),flush=True)

if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url",required=True)
    parser.add_argument("--model",required=True)
    parser.add_argument("--label",required=True)
    parser.add_argument("--backend",choices=["native","llama"],required=True)
    parser.add_argument("--out",required=True)
    parser.add_argument("--gpu-index",type=int,default=0)
    parser.add_argument("--repetitions",type=int,default=3)
    args=parser.parse_args();args.url=args.url.rstrip("/");run(args)
