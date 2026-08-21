#!/bin/sh
set -eu

cat >&2 <<'EOF'
Whale does not publish a standalone installer yet.
Install the independently published npm package instead:
  npm install -g @ceasarxuu/whalecode@latest
EOF
exit 1
