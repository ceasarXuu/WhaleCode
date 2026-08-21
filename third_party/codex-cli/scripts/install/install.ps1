$ErrorActionPreference = "Stop"

Write-Error @"
Whale does not publish a standalone installer yet.
Install the independently published npm package instead:
  npm install -g @ceasarxuu/whalecode@latest
"@
