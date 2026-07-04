#!/usr/bin/env python3
"""Record OpenCode Go model interactions as runie replay fixtures.

Usage:
    export OPENCODE_GO_API_KEY=sk-...
    python3 scripts/record_opencode_go.py

Outputs:
    - target/tmp/opencode-go-raw/<model>_<scenario>.sse   (raw capture)
    - crates/runie-testing/src/fixtures/openai/opencode_go_*.sse
    - crates/runie-testing/src/fixtures/anthropic/opencode_go_*.sse
"""

import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Any

import requests

BASE_URL = "https://opencode.ai/zen/go/v1"
RAW_DIR = Path("target/tmp/opencode-go-raw")
FIXTURE_OPENAI_DIR = Path("crates/runie-testing/src/fixtures/openai")
FIXTURE_ANTHROPIC_DIR = Path("crates/runie-testing/src/fixtures/anthropic")

# Models known to use the Anthropic-compatible /v1/messages endpoint.
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


def list_models(api_key: str) -> list[str]:
    r = requests.get(
        f"{BASE_URL}/models",
        headers={"Authorization": f"Bearer {api_key}"},
        timeout=30,
    )
    r.raise_for_status()
    return sorted(m["id"] for m in r.json()["data"])


def openai_payload(model: str, scenario: str) -> dict[str, Any]:
    base = {
        "model": model,
        "stream": True,
        "stream_options": {"include_usage": True},
    }
    if scenario == "simple":
        return {
            **base,
            "messages": [{"role": "user", "content": "Reply with only the word 'ok'."}],
            "max_tokens": 20,
        }
    if scenario == "tool":
        return {
            **base,
            "messages": [{"role": "user", "content": "What is the weather in Paris?"}],
            "tools": [
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
                }
            ],
            "max_tokens": 80,
        }
    if scenario == "multi_tool":
        return {
            **base,
            "messages": [
                {"role": "user", "content": "What is the weather in Paris and Berlin?"}
            ],
            "tools": [
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
                }
            ],
            "max_tokens": 120,
        }
    if scenario == "reasoning":
        return {
            **base,
            "messages": [
                {"role": "user", "content": "What is 9 times 7? Show your reasoning briefly."}
            ],
            "max_tokens": 120,
        }
    raise ValueError(f"unknown scenario: {scenario}")


def anthropic_payload(model: str, scenario: str) -> dict[str, Any]:
    base = {
        "model": model,
        "max_tokens": 80,
        "stream": True,
        "messages": [],
    }
    if scenario == "simple":
        return {
            **base,
            "max_tokens": 20,
            "messages": [{"role": "user", "content": "Reply with only the word 'ok'."}],
        }
    if scenario == "tool":
        return {
            **base,
            "messages": [{"role": "user", "content": "What is the weather in Paris?"}],
            "tools": [
                {
                    "name": "get_weather",
                    "description": "Get current weather for a city",
                    "input_schema": {
                        "type": "object",
                        "properties": {"city": {"type": "string"}},
                        "required": ["city"],
                    },
                }
            ],
        }
    if scenario == "multi_tool":
        return {
            **base,
            "max_tokens": 120,
            "messages": [
                {"role": "user", "content": "What is the weather in Paris and Berlin?"}
            ],
            "tools": [
                {
                    "name": "get_weather",
                    "description": "Get current weather for a city",
                    "input_schema": {
                        "type": "object",
                        "properties": {"city": {"type": "string"}},
                        "required": ["city"],
                    },
                }
            ],
        }
    if scenario == "reasoning":
        return {
            **base,
            "max_tokens": 120,
            "messages": [
                {"role": "user", "content": "What is 9 times 7? Show your reasoning briefly."}
            ],
            "thinking": {"type": "enabled", "budget_tokens": 1024},
        }
    raise ValueError(f"unknown scenario: {scenario}")


def capture_openai(api_key: str, model: str, scenario: str) -> str:
    payload = openai_payload(model, scenario)
    r = requests.post(
        f"{BASE_URL}/chat/completions",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
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


def capture_anthropic(api_key: str, model: str, scenario: str) -> str:
    payload = anthropic_payload(model, scenario)
    r = requests.post(
        f"{BASE_URL}/messages",
        headers={
            "x-api-key": api_key,
            "anthropic-version": "2023-06-01",
            "Content-Type": "application/json",
        },
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


def capture(api_key: str, model: str, scenario: str) -> tuple[str, str]:
    """Returns (protocol, raw_sse)."""
    if model in ANTHROPIC_MODELS:
        return "anthropic", capture_anthropic(api_key, model, scenario)
    return "openai", capture_openai(api_key, model, scenario)


def sanitize_openai(text: str, model: str) -> str:
    # Normalize the top-level completion id in each JSON data line by parsing
    # and re-serializing. This handles id fields that appear anywhere in the
    # object (e.g. after service_tier) and avoids accidentally touching nested
    # tool_call ids.
    out_lines: list[str] = []
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
    # OpenCode Go returns message ids as hex strings rather than Anthropic's
    # msg_ prefix, and tool-use content blocks use call_function_* ids.
    text = re.sub(
        r'("message"\s*:\s*\{[^}]*"id"\s*:\s*)"[^"]+"',
        r'\1"msg_opencode_go_fixture"',
        text,
    )
    text = re.sub(
        r'("content_block"\s*:\s*\{[^}]*"id"\s*:\s*)"[^"]+"',
        r'\1"content_opencode_go_fixture"',
        text,
    )
    # Model name.
    text = re.sub(r'"model":\s*"[^"]+"', f'"model": "{model}"', text)
    # Normalize cost pings so fixtures are deterministic.
    text = re.sub(r'"cost":"[0-9.eE+-]+"', '"cost":"0.00000000"', text)
    return text


def fixture_name(model: str, scenario: str) -> str:
    safe_model = model.replace(".", "_").replace("-", "_")
    return f"opencode_go_{safe_model}_{scenario}"


def main() -> None:
    api_key = get_api_key()
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    FIXTURE_OPENAI_DIR.mkdir(parents=True, exist_ok=True)
    FIXTURE_ANTHROPIC_DIR.mkdir(parents=True, exist_ok=True)

    models = list_models(api_key)
    print(f"Discovered {len(models)} models")

    # Scenario matrix.
    simple_scenarios = ["simple", "tool"]
    extended_models = {
        # Representative OpenAI-compatible models.
        "deepseek-v4-pro",
        "deepseek-v4-flash",
        "glm-5.2",
        "kimi-k2.6",
        "mimo-v2.5",
        # Representative Anthropic-compatible models.
        "minimax-m3",
        "minimax-m2.7",
        "qwen3.7-max",
        "qwen3.7-plus",
    }

    manifest: list[dict[str, str]] = []

    for model in models:
        scenarios = list(simple_scenarios)
        if model in extended_models:
            scenarios.extend(["multi_tool", "reasoning"])

        for scenario in scenarios:
            name = fixture_name(model, scenario)
            print(f"Recording {name} ...", end=" ", flush=True)
            try:
                protocol, raw = capture(api_key, model, scenario)
            except Exception as e:
                print(f"FAILED: {e}")
                continue

            raw_path = RAW_DIR / f"{name}.sse"
            raw_path.write_text(raw, encoding="utf-8")

            if protocol == "anthropic":
                sanitized = sanitize_anthropic(raw, model)
                fixture_dir = FIXTURE_ANTHROPIC_DIR
            else:
                sanitized = sanitize_openai(raw, model)
                fixture_dir = FIXTURE_OPENAI_DIR

            fixture_path = fixture_dir / f"{name}.sse"
            fixture_path.write_text(sanitized, encoding="utf-8")

            manifest.append({
                "model": model,
                "scenario": scenario,
                "protocol": protocol,
                "fixture": str(fixture_path),
                "raw": str(raw_path),
            })
            print(f"ok ({protocol}, {len(raw)} bytes)")
            time.sleep(0.5)

    manifest_path = RAW_DIR / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(f"\nRecorded {len(manifest)} fixtures. Manifest: {manifest_path}")


if __name__ == "__main__":
    main()
