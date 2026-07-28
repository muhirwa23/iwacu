#!/usr/bin/env bash
# Launch the Autonomous Symbiote CLI in interactive mode.
#
# Each model uses its own NVIDIA API key. Export them before running,
# OR paste them below (do NOT commit real keys).
#
#   export NVIDIA_API_KEY_GLM="nvapi-..."
#   export NVIDIA_API_KEY_INKLING="nvapi-..."
#   export NVIDIA_API_KEY_DEEPSEEK="nvapi-..."
#   export NVIDIA_API_KEY_KIMI="nvapi-..."
#
# Alternatively set a single shared key used as fallback for all models:
#   export NVIDIA_API_KEY="nvapi-..."
#
# Usage:
#   ./run.sh                 # interactive REPL against current directory
#   ./run.sh --repo /path    # interactive REPL against another repo
set -euo pipefail

cd "$(dirname "$0")"

exec cargo run --release -- --interactive "$@"
