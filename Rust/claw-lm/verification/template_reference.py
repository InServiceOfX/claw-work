"""Compare Rust template rendering with standalone Jinja, without Transformers."""
import copy
import json
import os
import pathlib
import subprocess
import sys
import tempfile
from jinja2.sandbox import ImmutableSandboxedEnvironment

root = pathlib.Path(os.environ["MODEL_DIR"])
env = ImmutableSandboxedEnvironment(trim_blocks=True, lstrip_blocks=True)
env.filters["tojson"] = lambda x: json.dumps(x, ensure_ascii=False, sort_keys=False)
template = env.from_string((root / "chat_template.jinja").read_text())
tools = [{"type": "function", "function": {"name": "example", "description": "Synthetic <example> 世界.", "parameters": {"type": "object", "properties": {"enabled": {"type": "boolean"}, "count": {"type": "integer"}, "payload": {"type": "object", "additionalProperties": True}}, "required": ["enabled", "count", "payload"]}}}]
cases = [
    ("unicode", [{"role": "user", "content": "Hello, 世界!"}], []),
    ("reasoning_history", [{"role": "system", "content": "Be brief."}, {"role": "user", "content": "First?"}, {"role": "assistant", "content": "<think>hidden old thought</think>First answer."}, {"role": "user", "content": "Next?"}], []),
    ("typed_tool_history", [{"role": "user", "content": "Use example."}, {"role": "assistant", "content": "", "tool_calls": [{"id": "synthetic_call", "type": "function", "function": {"name": "example", "arguments": json.dumps({"enabled": True, "count": 2, "payload": {"text": "世界 <&>", "ok": False}})}}]}, {"role": "tool", "tool_call_id": "synthetic_call", "content": "done"}, {"role": "user", "content": "Explain the result."}], tools),
]
results = []
with tempfile.TemporaryDirectory() as directory:
    for name, messages, tools in cases:
        path = pathlib.Path(directory)
        (path / "messages.json").write_text(json.dumps(messages))
        (path / "tools.json").write_text(json.dumps(tools))
        run = subprocess.run([sys.argv[1], "--model-dir", str(root), "--messages", str(path / "messages.json"), "--tools", str(path / "tools.json"), "--render-only"], check=True, text=True, capture_output=True)
        actual = json.loads(run.stdout)
        normalized = copy.deepcopy(messages)
        for message in normalized:
            for call in message.get("tool_calls", []):
                arguments = call["function"]["arguments"]
                if isinstance(arguments, str):
                    call["function"]["arguments"] = json.loads(arguments)
        expected = template.render(messages=normalized, tools=tools, add_generation_prompt=True, enable_thinking=False, truncate_history_thinking=True)
        if actual["prompt"] != expected:
            mismatch = next((i for i, (a,b) in enumerate(zip(actual["prompt"], expected)) if a != b), min(len(actual["prompt"]), len(expected)))
            raise AssertionError((name, mismatch, repr(actual["prompt"][mismatch:mismatch+100]), repr(expected[mismatch:mismatch+100])))
        results.append({"case": name, "passed": True, "prompt_characters": len(expected), "token_count": len(actual["token_ids"])})
print(json.dumps({"cases": results, "scope": "Exact Rust/Jinja text equality; token IDs emitted by native Rust tokenizer, no Python tokenizer oracle"}, indent=2))
