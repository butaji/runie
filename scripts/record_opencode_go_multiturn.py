#!/usr/bin/env python3
"""Record multi-turn OpenCode Go conversations as runie replay fixtures.

Usage:
    export OPENCODE_GO_API_KEY=sk-...
    python3 scripts/record_opencode_go_multiturn.py

Outputs:
    - target/tmp/opencode-go-raw/multiturn/<fixture>.sse   (raw capture per turn)
    - crates/runie-testing/src/fixtures/{openai,anthropic}/opencode_go_*_multiturn_*.sse
"""

import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Any, Literal

import requests

BASE_URL = "https://opencode.ai/zen/go/v1"
RAW_DIR = Path("target/tmp/opencode-go-raw/multiturn")
FIXTURE_OPENAI_DIR = Path("crates/runie-testing/src/fixtures/openai")
FIXTURE_ANTHROPIC_DIR = Path("crates/runie-testing/src/fixtures/anthropic")

ANTHROPIC_MODELS = {
    "minimax-m3",
    "minimax-m2.7",
    "minimax-m2.5",
    "qwen3.7-max",
    "qwen3.7-plus",
    "qwen3.6-plus",
    "qwen3.5-plus",
}


def get_api_key() -> str:
    key = os.environ.get("OPENCODE_GO_API_KEY")
    if not key:
        print("Set OPENCODE_GO_API_KEY", file=sys.stderr)
        sys.exit(1)
    return key


def tool_definitions_openai() -> list[dict[str, Any]]:
    return [
        {
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get current weather for a city",
                "parameters": {
                    "type": "object",
                    "properties": {"city": {"type": "string"}},
                    "required": ["city"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {"path": {"type": "string", "description": "File path to read"}},
                    "required": ["path"],
                },
            },
        },
    ]


def tool_definitions_anthropic() -> list[dict[str, Any]]:
    return [
        {
            "name": "get_weather",
            "description": "Get current weather for a city",
            "input_schema": {
                "type": "object",
                "properties": {"city": {"type": "string"}},
                "required": ["city"],
            },
        },
        {
            "name": "read_file",
            "description": "Read the contents of a file",
            "input_schema": {
                "type": "object",
                "properties": {"path": {"type": "string", "description": "File path to read"}},
                "required": ["path"],
            },
        },
    ]


def make_openai_messages(turns: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Build OpenAI message list from prior turns."""
    messages: list[dict[str, Any]] = []
    for turn in turns:
        messages.append({"role": "user", "content": turn["user"]}) if not messages else None
        # If this turn already has recorded assistant/tool exchange, replay it.
        if "assistant_response" in turn:
            assistant = {"role": "assistant", "content": turn["assistant_response"].get("content", "")}
            if "tool_calls" in turn["assistant_response"]:
                assistant["tool_calls"] = turn["assistant_response"]["tool_calls"]
            messages.append(assistant)
            for tr in turn.get("tool_results", []):
                messages.append({
                    "role": "tool",
                    "tool_call_id": tr["tool_call_id"],
                    "content": tr["content"],
                })
    return messages


def make_anthropic_messages(turns: list[dict[str, Any]]) -> list[dict[str, Any]]:
    messages: list[dict[str, Any]] = []
    for turn in turns:
        messages.append({"role": "user", "content": turn["user"]})
        if "assistant_response" in turn:
            content: list[dict[str, Any]] = []
            if "content" in turn["assistant_response"] and turn["assistant_response"]["content"]:
                content.append({"type": "text", "text": turn["assistant_response"]["content"]})
            for tc in turn["assistant_response"].get("tool_calls", []):
                content.append({
                    "type": "tool_use",
                    "id": tc["id"],
                    "name": tc["name"],
                    "input": tc["input"],
                })
            messages.append({"role": "assistant", "content": content})
            for tr in turn.get("tool_results", []):
                messages.append({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tr["tool_call_id"],
                        "content": tr["content"],
                    }],
                })
    return messages


def parse_openai_tool_calls(response_text: str) -> list[dict[str, Any]]:
    """Extract tool_calls from an OpenAI streaming response."""
    tool_calls: dict[int, dict[str, Any]] = {}
    for line in response_text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("data:"):
            continue
        payload = stripped[len("data:"):].strip()
        if payload in ("", "[DONE]"):
            continue
        try:
            obj = json.loads(payload)
        except json.JSONDecodeError:
            continue
        for choice in obj.get("choices", []):
            for tc in choice.get("delta", {}).get("tool_calls", []) or []:
                idx = tc.get("index", 0)
                entry = tool_calls.setdefault(idx, {"id": "", "type": "function", "function": {"name": "", "arguments": ""}})
                if tc.get("id"):
                    entry["id"] = tc["id"]
                if tc.get("function", {}).get("name"):
                    entry["function"]["name"] = tc["function"]["name"]
                if tc.get("function", {}).get("arguments"):
                    entry["function"]["arguments"] += tc["function"]["arguments"]
    return list(tool_calls.values())


def parse_anthropic_tool_calls(response_text: str) -> list[dict[str, Any]]:
    """Extract tool_use blocks from an Anthropic streaming response."""
    tool_calls: list[dict[str, Any]] = []
    current: dict[str, Any] | None = None
    input_parts: list[str] = []
    for line in response_text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("data:"):
            continue
        payload = stripped[len("data:"):].strip()
        if not payload:
            continue
        try:
            obj = json.loads(payload)
        except json.JSONDecodeError:
            continue
        t = obj.get("type")
        if t == "content_block_start":
            block = obj.get("content_block", {})
            if block.get("type") == "tool_use":
                current = {"id": block.get("id", ""), "name": block.get("name", ""), "input": {}}
                input_parts = []
        elif t == "content_block_delta" and current is not None:
            partial = obj.get("delta", {}).get("partial_json", "")
            if partial:
                input_parts.append(partial)
        elif t == "content_block_stop" and current is not None:
            try:
                current["input"] = json.loads("".join(input_parts)) if input_parts else {}
            except json.JSONDecodeError:
                current["input"] = {}
            tool_calls.append(current)
            current = None
    return tool_calls


def extract_openai_content(response_text: str) -> str:
    parts: list[str] = []
    for line in response_text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("data:"):
            continue
        payload = stripped[len("data:"):].strip()
        if payload in ("", "[DONE]"):
            continue
        try:
            obj = json.loads(payload)
        except json.JSONDecodeError:
            continue
        for choice in obj.get("choices", []):
            content = choice.get("delta", {}).get("content")
            if content:
                parts.append(content)
    return "".join(parts)


def extract_anthropic_content(response_text: str) -> str:
    parts: list[str] = []
    for line in response_text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("data:"):
            continue
        payload = stripped[len("data:"):].strip()
        if not payload:
            continue
        try:
            obj = json.loads(payload)
        except json.JSONDecodeError:
            continue
        if obj.get("type") == "content_block_delta":
            text = obj.get("delta", {}).get("text", "")
            if text:
                parts.append(text)
    return "".join(parts)


def call_openai(api_key: str, model: str, messages: list[dict[str, Any]], tools: list[dict[str, Any]] | None, max_tokens: int) -> str:
    payload: dict[str, Any] = {
        "model": model,
        "messages": messages,
        "stream": True,
        "stream_options": {"include_usage": True},
        "max_tokens": max_tokens,
    }
    if tools:
        payload["tools"] = tools
    r = requests.post(
        f"{BASE_URL}/chat/completions",
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        json=payload,
        stream=True,
        timeout=120,
    )
    r.raise_for_status()
    chunks: list[str] = []
    for line in r.iter_lines(decode_unicode=True):
        if line is not None:
            chunks.append(line + "\n")
    return "".join(chunks)


def call_anthropic(api_key: str, model: str, messages: list[dict[str, Any]], tools: list[dict[str, Any]] | None, max_tokens: int) -> str:
    payload: dict[str, Any] = {
        "model": model,
        "messages": messages,
        "max_tokens": max_tokens,
        "stream": True,
    }
    if tools:
        payload["tools"] = tools
    r = requests.post(
        f"{BASE_URL}/messages",
        headers={"x-api-key": api_key, "anthropic-version": "2023-06-01", "Content-Type": "application/json"},
        json=payload,
        stream=True,
        timeout=120,
    )
    r.raise_for_status()
    chunks: list[str] = []
    for line in r.iter_lines(decode_unicode=True):
        if line is not None:
            chunks.append(line + "\n")
    return "".join(chunks)


def fake_tool_result(name: str, arguments: dict[str, Any]) -> str:
    if name == "get_weather":
        city = arguments.get("city", "Unknown")
        temps = {"Paris": 22, "Berlin": 18, "London": 15, "Moscow": -5, "Tokyo": 26}
        temp = temps.get(city, 20)
        return json.dumps({"temperature": temp, "unit": "celsius", "condition": "sunny"})
    if name == "read_file":
        path = arguments.get("path", "README.md")
        return json.dumps({"path": path, "content": "# Example Project\n\nThis is a sample README.\n"})
    return json.dumps({"result": "ok"})


def sanitize_openai(text: str, model: str) -> str:
    out_lines = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("data:"):
            payload = stripped[len("data:"):].strip()
            if payload and payload != "[DONE]":
                try:
                    obj = json.loads(payload)
                    if isinstance(obj, dict) and "id" in obj:
                        obj["id"] = "chatcmpl-opencode-go-fixture"
                    if isinstance(obj, dict) and "created" in obj:
                        obj["created"] = 0
                    if isinstance(obj, dict) and "system_fingerprint" in obj:
                        obj["system_fingerprint"] = "fp_opencode_go"
                    if isinstance(obj, dict) and "model" in obj:
                        obj["model"] = model
                    line = "data: " + json.dumps(obj, separators=(",", ":"), ensure_ascii=False)
                except json.JSONDecodeError:
                    pass
        out_lines.append(line)
    return "\n".join(out_lines) + "\n"


def sanitize_anthropic(text: str, model: str) -> str:
    text = re.sub(r'("message"\s*:\s*\{[^}]*"id"\s*:\s*)"[^"]+"', r'\1"msg_opencode_go_fixture"', text)
    text = re.sub(r'("content_block"\s*:\s*\{[^}]*"id"\s*:\s*)"[^"]+"', r'\1"content_opencode_go_fixture"', text)
    text = re.sub(r'"model":\s*"[^"]+"', f'"model": "{model}"', text)
    text = re.sub(r'"cost":"[0-9.eE+-]+"', '"cost":"0.00000000"', text)
    return text


Scenario = dict[str, Any]


SCENARIOS: list[Scenario] = [
    {
        "name": "math_chain",
        "description": "Simple multi-turn math follow-up",
        "max_tokens": 40,
        "turns": [
            {"user": "What is 2 + 2?"},
            {"user": "Multiply that by 3."},
        ],
    },
    {
        "name": "weather_chain",
        "description": "Tool call with follow-up tool call",
        "tools": True,
        "max_tokens": 80,
        "turns": [
            {"user": "What is the weather in Paris?"},
            {"user": "What about Berlin?"},
        ],
    },
    {
        "name": "read_summarize_followup",
        "description": "Read file, summarize, then answer a follow-up",
        "tools": True,
        "max_tokens": 120,
        "turns": [
            {"user": "Read README.md and summarize it."},
            {"user": "What is the main topic?"},
        ],
    },
    {
        "name": "reasoning_followup",
        "description": "Reasoning model answers, then follows up on the result",
        "max_tokens": 120,
        "turns": [
            {"user": "What is 9 times 7? Show your reasoning briefly."},
            {"user": "Now divide that by 3."},
        ],
    },
    {
        "name": "multi_tool_then_compare",
        "description": "Parallel tool calls then comparison question",
        "tools": True,
        "max_tokens": 120,
        "turns": [
            {"user": "What is the weather in Paris and Berlin?"},
            {"user": "Which city is warmer?"},
        ],
    },
    {
        "name": "clarification",
        "description": "Vague request, model asks clarification, then answers",
        "max_tokens": 80,
        "turns": [
            {"user": "What should I wear tonight?"},
            {"user": "I'm going to a casual dinner in Paris."},
        ],
    },
]


def record_scenario(api_key: str, model: str, scenario: Scenario) -> list[dict[str, Any]]:
    protocol = "anthropic" if model in ANTHROPIC_MODELS else "openai"
    turns: list[dict[str, Any]] = []
    recorded_files: list[dict[str, str]] = []

    for turn_idx, turn in enumerate(scenario["turns"]):
        turns.append({"user": turn["user"]})
        if protocol == "anthropic":
            messages = make_anthropic_messages(turns)
            tools = tool_definitions_anthropic() if scenario.get("tools") else None
            raw = call_anthropic(api_key, model, messages, tools, scenario["max_tokens"])
        else:
            messages = make_openai_messages(turns)
            tools = tool_definitions_openai() if scenario.get("tools") else None
            raw = call_openai(api_key, model, messages, tools, scenario["max_tokens"])

        # Parse assistant response and inject fake tool results if needed.
        if protocol == "anthropic":
            content = extract_anthropic_content(raw)
            tool_calls = parse_anthropic_tool_calls(raw)
        else:
            content = extract_openai_content(raw)
            tool_calls = parse_openai_tool_calls(raw)

        assistant_response: dict[str, Any] = {"content": content}
        if tool_calls:
            assistant_response["tool_calls"] = tool_calls
            tool_results = []
            for tc in tool_calls:
                name = tc.get("name") or tc.get("function", {}).get("name", "")
                arguments = tc.get("input") or tc.get("function", {}).get("arguments", {})
                if isinstance(arguments, str):
                    try:
                        arguments = json.loads(arguments)
                    except json.JSONDecodeError:
                        arguments = {}
                result = fake_tool_result(name, arguments)
                tool_results.append({"tool_call_id": tc.get("id", ""), "content": result})
            turns[-1]["tool_results"] = tool_results
        turns[-1]["assistant_response"] = assistant_response

        # Save per-turn fixture.
        safe_model = model.replace(".", "_").replace("-", "_")
        fixture_name = f"opencode_go_{safe_model}_multiturn_{scenario['name']}_turn{turn_idx + 1}"
        raw_path = RAW_DIR / f"{fixture_name}.sse"
        raw_path.write_text(raw, encoding="utf-8")

        if protocol == "anthropic":
            sanitized = sanitize_anthropic(raw, model)
            fixture_dir = FIXTURE_ANTHROPIC_DIR
        else:
            sanitized = sanitize_openai(raw, model)
            fixture_dir = FIXTURE_OPENAI_DIR

        fixture_path = fixture_dir / f"{fixture_name}.sse"
        fixture_path.write_text(sanitized, encoding="utf-8")

        recorded_files.append({
            "turn": turn_idx + 1,
            "fixture": str(fixture_path),
            "raw": str(raw_path),
        })

        time.sleep(0.5)

    return recorded_files


def main() -> None:
    api_key = get_api_key()
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    FIXTURE_OPENAI_DIR.mkdir(parents=True, exist_ok=True)
    FIXTURE_ANTHROPIC_DIR.mkdir(parents=True, exist_ok=True)

    models = {
        # Full coverage for representative models.
        "deepseek-v4-pro": SCENARIOS,
        "deepseek-v4-flash": SCENARIOS,
        "minimax-m3": SCENARIOS,
        "qwen3.7-max": SCENARIOS,
        # Core scenarios for additional representative models.
        "glm-5.2": [s for s in SCENARIOS if s["name"] in ("math_chain", "weather_chain", "reasoning_followup")],
        "kimi-k2.6": [s for s in SCENARIOS if s["name"] in ("math_chain", "weather_chain", "reasoning_followup")],
    }

    manifest: list[dict[str, Any]] = []

    for model, scenarios in models.items():
        for scenario in scenarios:
            print(f"Recording {model} / {scenario['name']} ...", end=" ", flush=True)
            try:
                files = record_scenario(api_key, model, scenario)
            except Exception as e:
                print(f"FAILED: {e}")
                continue
            manifest.append({
                "model": model,
                "scenario": scenario["name"],
                "description": scenario["description"],
                "turns": files,
            })
            print(f"ok ({len(files)} turns)")

    manifest_path = RAW_DIR / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(f"\nRecorded {len(manifest)} multi-turn scenarios. Manifest: {manifest_path}")


if __name__ == "__main__":
    main()
