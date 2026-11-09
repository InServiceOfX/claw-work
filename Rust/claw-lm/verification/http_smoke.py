"""Real model HTTP/SSE and tool-round-trip acceptance; uses only loopback."""
import json
import pathlib
import sys
import time
import urllib.request
import urllib.error

base = sys.argv[1].rstrip("/")
assert base.startswith("http://127.0.0.1:")
model = "nemotron-labs-diffusion-3b"
results = []
with urllib.request.urlopen(base + "/v1/models", timeout=5) as response:
    cards = json.load(response)["data"]
card_by_id = {card["id"]: card for card in cards}
assert model in card_by_id
context = card_by_id[model]["context_window"]



def request(body):
    start = time.monotonic()
    req = urllib.request.Request(base + "/v1/chat/completions", json.dumps(body).encode(), {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as response:
        data = response.read().decode()
    return data, time.monotonic() - start


for mode in ["autoregressive", "diffusion", "self-speculation"]:
    suffix = "block-diffusion" if mode == "diffusion" else mode
    selected_model = model + "-" + suffix
    assert card_by_id[selected_model]["context_window"] == context
    body = {"model": selected_model, "messages": [{"role": "user", "content": "What is 2 + 2? Answer briefly."}], "max_tokens": 32, "temperature": 0, "block_length": 8}
    text, seconds = request(body)
    response = json.loads(text)
    assert response["model"] == selected_model
    content = response["choices"][0]["message"]["content"]
    assert "4" in content, response
    assert response["claw"]["generation_mode"] == mode
    body["stream"] = True
    body["stream_options"] = {"include_usage": True}
    events, stream_seconds = request(body)
    emitted = []
    usage = None
    assert "data: [DONE]" in events
    for line in events.splitlines():
        if not line.startswith("data: ") or line == "data: [DONE]":
            continue
        item = json.loads(line[6:])
        assert "error" not in item, item
        assert item["model"] == selected_model
        for choice in item.get("choices", []):
            emitted.append(choice["delta"].get("content", ""))
        usage = item.get("usage", usage)
    assert "".join(emitted) == content, (mode, emitted, content)
    assert usage["completion_tokens"] > 0
    results.append({"case": "chat_and_sse", "mode": mode, "model": selected_model, "passed": True, "text": content, "seconds": seconds, "stream_seconds": stream_seconds})

for prompt in ["List the first five prime numbers, separated by commas. No explanation.", "Return exactly this text: Rust + CUDA."]:
    outputs = {}
    for mode in ["autoregressive", "self-speculation"]:
        text, seconds = request({"model": model, "messages": [{"role": "user", "content": prompt}], "max_tokens": 64, "temperature": 0, "generation_mode": mode, "block_length": 8})
        data = json.loads(text)
        outputs[mode] = data["choices"][0]["message"]["content"]
    assert outputs["autoregressive"] == outputs["self-speculation"], outputs
    results.append({"case": "ar_self_speculation_agreement", "passed": True, "prompt": prompt, "text": outputs["autoregressive"]})

# A true oversized request must be recognizable by an ordinary OpenAI client.
try:
    request({"model": model, "messages": [{"role": "user", "content": " probe" * (context * 2)}], "max_tokens": 16})
except urllib.error.HTTPError as error:
    payload = json.load(error)
    assert error.code == 400 and payload["error"]["code"] == "context_length_exceeded", payload
    assert f"maximum context length is {context} tokens" in payload["error"]["message"]
    results.append({"case": "recognizable_context_overflow", "passed": True, "context": context})
else:
    raise AssertionError("Oversized input was accepted")

tools = [{"type": "function", "function": {"name": "add", "description": "Add two integers and return their sum.", "parameters": {"type": "object", "properties": {"a": {"type": "integer"}, "b": {"type": "integer"}}, "required": ["a", "b"]}}}]
messages = [{"role": "system", "content": "Use the add tool exactly once for the requested calculation. A tool message is the completed result of your previous call. After a tool message, do not call tools again: answer the user with that result and stop. Never repeat a successful call."}, {"role": "user", "content": "Call add with a=12 and b=30 exactly once. After the result arrives, answer with the sum."}]
text, seconds = request({"model": model, "messages": messages, "tools": tools, "max_tokens": 256, "temperature": 0})
reply = json.loads(text)
message = reply["choices"][0]["message"]
calls = message.get("tool_calls", [])
assert len(calls) == 1 and calls[0]["function"]["name"] == "add", reply
assert json.loads(calls[0]["function"]["arguments"]) == {"a": 12, "b": 30}
messages.append(message)
messages.append({"role": "tool", "tool_call_id": calls[0]["id"], "content": "42"})
text, seconds = request({"model": model, "messages": messages, "tools": tools, "max_tokens": 128, "temperature": 0})
reply = json.loads(text)
assert "42" in reply["choices"][0]["message"]["content"], reply
results.append({"case": "actual_model_tool_round_trip", "passed": True, "tool": "add", "text": reply["choices"][0]["message"]["content"]})
report = {"passed": True, "cases": results, "scope": "Real checkpoint on local native CUDA server; actual harness tests are separate"}
pathlib.Path(sys.argv[2]).write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
