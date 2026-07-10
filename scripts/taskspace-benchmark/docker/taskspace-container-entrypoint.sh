#!/usr/bin/env bash
set -euo pipefail

secret_path=/run/secrets/deepseek_api_key
if [[ -f "$secret_path" ]]; then
    DEEPSEEK_API_KEY="$(<"$secret_path")"
    export DEEPSEEK_API_KEY
fi

mkdir -p "${HOME:-/artifacts/home}"
exec "$@"
