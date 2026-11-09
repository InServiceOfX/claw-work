#!/usr/bin/env python3
"""Bounded local chat/code/tool checks; raw captures must stay outside Git."""
import argparse
import json
import pathlib
import re
import statistics
import subprocess
import tempfile
import time
import urllib.parse
import urllib.request

PRIMES = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71]
PROMPTS = {
    "greeting": "Say hello in one short sentence. Do not call tools.",
    "primes": "List the first 20 prime numbers, in ascending order, separated by commas. Output only the numbers.",
    "rust": "Write a Rust function that merges two sorted slices of i32 into a sorted Vec<i32>. Preserve duplicates. Include tests for empty inputs, negative numbers, and duplicates. Show the code without using any tools.",
}

def valid_primes(text):
    return bool(re.fullmatch(r"\s*\d+(?:\s*,\s*\d+){19}\s*", text)) and [int(x) for x in text.split(",")] == PRIMES


def rust_check(text):
    fragments = re.findall(r"^```(?:rust|rs)?[ \t]*\n(.*?)^```[ \t]*(?:\n|$)", text, re.S | re.M)
    source = "\n".join(fragments) if fragments else text.strip()
    # This task only requires pure slice/Vec code, never host access or macros
    # that read files at compile time. Decline any such generated program.
    if re.search(r"\b(?:unsafe|extern|include|include_str|include_bytes|env|option_env)\b|(?:std::)?(?:fs|net|process|thread)::|#\s*\[link", source):
        return {"logic": False, "included_tests": False, "diagnosis": "outside pure merge-task scope"}
    function = re.search(r"fn\s+(\w*merge\w*)\s*\(", source)
    has_tests = "#[test]" in source
    if not function:
        return {"logic": False, "included_tests": has_tests, "diagnosis": "merge function missing"}
    name = function.group(1)
    oracle = """
#[cfg(test)] mod independent_acceptance {
    use super::FUNCTION as merge;
    #[test] fn empty_both() { assert_eq!(merge(&[], &[]), Vec::<i32>::new()); }
    #[test] fn empty_left() { assert_eq!(merge(&[], &[-2, 0, 3]), vec![-2, 0, 3]); }
    #[test] fn empty_right() { assert_eq!(merge(&[-2, 0, 3], &[]), vec![-2, 0, 3]); }
    #[test] fn negatives() { assert_eq!(merge(&[-7, -2, 4], &[-9, -2, 0]), vec![-9, -7, -2, -2, 0, 4]); }
    #[test] fn duplicates() { assert_eq!(merge(&[1, 1, 3], &[1, 2, 3, 3]), vec![1, 1, 1, 2, 3, 3, 3]); }
    #[test] fn bounds() { assert_eq!(merge(&[i32::MIN, 0], &[0, i32::MAX]), vec![i32::MIN, 0, 0, i32::MAX]); }
}
""".replace("FUNCTION", name)
    with tempfile.TemporaryDirectory(prefix="llama-merge-check-") as directory:
        path = pathlib.Path(directory)
        (path / "merge.rs").write_text(source + oracle)
        try:
            build = subprocess.run(["rustc", "--edition=2024", "--test", "-Awarnings", str(path / "merge.rs"), "-o", str(path / "checks")], capture_output=True, text=True, timeout=30)
            if build.returncode:
                return {"logic": False, "included_tests": has_tests, "diagnosis": "compile failed", "compiler": build.stderr}
            independent = subprocess.run([str(path / "checks"), "independent_acceptance::"], capture_output=True, text=True, timeout=5)
            tested = subprocess.run([str(path / "checks")], capture_output=True, text=True, timeout=5)
            diagnosis = "pass" if tested.returncode == 0 else "authored tests failed" if independent.returncode == 0 else "independent tests failed"
            return {"logic": independent.returncode == 0, "included_tests": has_tests, "authored_tests_pass": has_tests and tested.returncode == 0, "diagnosis": diagnosis, "test_output": tested.stdout}
        except subprocess.TimeoutExpired:
            return {"logic": False, "included_tests": has_tests, "diagnosis": "compile/test timeout"}


def run(base, destination, max_tokens=2048):
    parsed = urllib.parse.urlparse(base)
    if parsed.scheme != "http" or parsed.hostname not in ("127.0.0.1", "localhost"):
        raise ValueError("Benchmark only accepts an explicitly selected loopback server")
    destination = pathlib.Path(destination).resolve()
    for parent in [destination, *destination.parents]:
        if (parent / ".git").exists():
            raise ValueError("Choose a private result directory outside a Git working tree")
    destination.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(base + "/v1/models", timeout=10) as response:
        model = json.load(response)["data"][0]["id"]
    measurements = []
    def post(label, messages, tools=None):
        body = {"model": model, "messages": messages, "stream": False, "max_tokens": max_tokens,
                "temperature": 0.6, "top_p": 0.95, "top_k": 20, "min_p": 0.0, "repeat_penalty": 1.05, "seed": 42, "cache_prompt": False}
        if tools:
            body["tools"] = tools
        request = urllib.request.Request(base + "/v1/chat/completions", json.dumps(body).encode(), {"Content-Type": "application/json"})
        start = time.monotonic()
        with urllib.request.urlopen(request, timeout=150) as response:
            data = json.load(response)
        seconds = time.monotonic() - start
        (destination / (label + "-response.json")).write_text(json.dumps(data, indent=2) + "\n")
        choice = data["choices"][0]
        measurements.append({"case": label, "seconds": seconds, "finish_reason": choice.get("finish_reason"), "usage": data.get("usage"), "timings": data.get("timings", {})})
        return choice
    grades = {}
    details = {}
    for label, prompt in PROMPTS.items():
        try:
            choice = post(label, [{"role": "user", "content": prompt}])
            text = choice["message"].get("content") or ""
            complete = choice.get("finish_reason") == "stop"
            if label == "greeting":
                sentences = len(re.findall(r"[.!?]+(?:\s|$)", text.strip()))
                grades[label] = complete and bool(text.strip()) and len(text.split()) <= 20 and sentences <= 1
            elif label == "primes":
                grades[label] = complete and valid_primes(text)
            else:
                checked = rust_check(text) if complete else {"logic": False, "included_tests": False, "diagnosis": "output limit; code not graded"}
                (destination / "rust-review.json").write_text(json.dumps(checked, indent=2) + "\n")
                grades["rust_logic"] = complete and checked["logic"]
                grades["rust_tests_included"] = complete and checked["logic"] and checked["included_tests"] and checked.get("authored_tests_pass", False)
                details["rust"] = checked["diagnosis"]
        except Exception as error:
            details[label] = type(error).__name__ + ": " + str(error)
            for key in (["rust_logic", "rust_tests_included"] if label == "rust" else [label]):
                grades[key] = False
    tools = [{"type": "function", "function": {"name": "add", "description": "Add two integers.", "parameters": {"type": "object", "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}}, "required": ["a", "b"]}}}]
    history = [{"role": "system", "content": "Use add exactly once when requested. A tool result is the completed call. After it arrives, answer with the result and stop; never repeat a successful call."}, {"role": "user", "content": "Call add with a=12 and b=30 exactly once. After the tool result arrives, output only the sum."}]
    grades.update(tool_call=False, tool_followup=False)
    try:
        choice = post("tool-call", history, tools)
        message = choice["message"]
        calls = message.get("tool_calls", [])
        grades["tool_call"] = len(calls) == 1 and calls[0]["function"]["name"] == "add" and json.loads(calls[0]["function"]["arguments"]) == {"a": 12, "b": 30}
        if grades["tool_call"]:
            arguments = json.loads(calls[0]["function"]["arguments"])
            value = str(arguments["a"] + arguments["b"])
            history.extend([message, {"role": "tool", "tool_call_id": calls[0]["id"], "content": value}])
            followup = post("tool-followup", history, tools)
            grades["tool_followup"] = followup.get("finish_reason") == "stop" and not followup["message"].get("tool_calls") and (followup["message"].get("content") or "").strip() == "42"
    except Exception as error:
        details["tool"] = type(error).__name__ + ": " + str(error)
    rates = [m["timings"].get("predicted_per_second") for m in measurements if m["timings"].get("predicted_per_second")]
    summary = {"model": model, "score": sum(grades.values()), "possible": 6, "grades": grades, "details": details,
               "median_generation_tokens_per_second": statistics.median(rates) if rates else None,
               "total_response_seconds": sum(m["seconds"] for m in measurements), "measurements": measurements,
               "settings": {"temperature": 0.6, "top_p": 0.95, "top_k": 20, "min_p": 0.0, "repeat_penalty": 1.05, "seed": 42, "max_tokens": max_tokens},
               "scope": "One seeded bounded local smoke suite, not a general model-quality benchmark. Server reasoning budget and context must be recorded separately."}
    (destination / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({k: v for k, v in summary.items() if k != "measurements"}), flush=True)
    return summary

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", default="http://127.0.0.1:8080")
    parser.add_argument("--out", required=True)
    parser.add_argument("--max-tokens", type=int, default=2048)
    args = parser.parse_args()
    run(args.url.rstrip("/"), args.out, args.max_tokens)
