#!/usr/bin/env bash
set -euo pipefail

fixture=${1:?usage: capture-scenario.sh FIXTURE OUTPUT_DIR COMMAND [ENV_ASSIGNMENTS]}
output_dir=${2:?usage: capture-scenario.sh FIXTURE OUTPUT_DIR COMMAND [ENV_ASSIGNMENTS]}
command_line=${3:?usage: capture-scenario.sh FIXTURE OUTPUT_DIR COMMAND [ENV_ASSIGNMENTS]}
env_assignments=${4:-}

[[ -f "$fixture" ]] || { echo "fixture not found: $fixture" >&2; exit 1; }
command -v ruby >/dev/null || { echo "ruby is required to read YAML capture scenarios" >&2; exit 1; }

# Psych is part of Ruby's standard library on the supported capture hosts.
# Emit shell-safe, single-line fields; the command itself remains a positional
# argument and is never interpreted as YAML or shell source.
read -r prompt quit_key resize_schedule < <(ruby -ryaml -e '
  value = YAML.load_file(ARGV.fetch(0))
  capture = value.fetch("capture", {})
  prompt = capture.fetch("prompt", value.fetch("initial_prompt", "Hey"))
  quit = capture.fetch("quit_key", "C-q")
  resize = capture.fetch("resize", [])
  abort "capture.prompt must be a single line" unless prompt.is_a?(String) && !prompt.include?("\n")
  abort "capture.quit_key must be a single line" unless quit.is_a?(String) && !quit.include?("\n")
  abort "capture.resize must be an array" unless resize.is_a?(Array)
  encoded = resize.map do |entry|
    abort "capture.resize entries must be mappings" unless entry.is_a?(Hash)
    at = Integer(entry.fetch("at_ms"))
    cols = Integer(entry.fetch("cols"))
    rows = Integer(entry.fetch("rows"))
    abort "capture.resize values must be positive" unless at >= 0 && cols > 0 && rows > 0
    "#{at},#{cols},#{rows}"
  end.join(";")
  times = resize.map { |entry| Integer(entry.fetch("at_ms")) }
  abort "capture.resize entries must be ordered" unless times == times.sort && times.uniq.length == times.length
  puts [prompt, quit, encoded].map { |field| field.gsub(/[^[:print:]]/, "") }.join(" ")
' "$fixture")

scripts/capture-matrix.sh "$output_dir" "$command_line" "$quit_key" "$prompt" "$env_assignments" "$resize_schedule"
