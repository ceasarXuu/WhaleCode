# TaskSpace E3 Benchmark Harness Review

Date: 2026-06-03
Reviewer: subagent-vs-review via internal subagents

## Scope

Reviewed the TaskSpace E3 benchmark harness changes for:

- Terminal-Bench fixture materialization and validator execution
- External validator dependency caching
- Agent execution timeout and validator timeout separation
- E3 proof, audit freshness, and exec-timeout gate behavior
- Automated harness coverage and real Terminal-Bench smoke evidence

## Review Results

Initial adversarial review found no blocking issue for the narrow hello-world 5-repeat E3 run, but raised non-blocking auditability and cache-boundary concerns around the uv/apt shim.

Second review found two blocking issues:

- The apt wrapper used `command -v curl`, which could be polluted by the shim itself.
- Partial uv cache materialization could still produce a generated scenario with misleading runtime proof behavior.

Fixes applied:

- The apt wrapper now only short-circuits curl installation when `/usr/bin/curl` is executable.
- Non-curl apt fallback restores `apt-get update` and preserves the `install` subcommand.
- Terminal-Bench adapter now fails fast when uv cache materialization is incomplete.
- E3 runtime proof now records and validates `/tbench-uv-cache` mount source, read-only status, and cached artifact hashes.
- Added targeted regression coverage in `test-terminal-bench-uv-cache-harness.ps1` and `test-e3-proof-harness.ps1`.

Final adversarial review verdict:

> Blocking: none. Non-blocking: none. Verdict: pass.

## Evidence

Automated tests passed:

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-oracle-runner-harness.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-proof-harness.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-terminal-bench-uv-cache-harness.ps1`
- `git diff --check`

Real E3 evidence:

- `D:\whalecode-alpha\target\e3-clean-terminal-bench-hello5-20260603-164732\runs\terminal_bench__hello-world\20260603-164737-287\aggregate-report.md`
- Result: `valid_e3_pairs=5`, `excluded_pairs=0`, `include_no_clear_delta=4`, `include_taskspace_better=1`

Final current-code smoke:

- `D:\whalecode-alpha\target\e3-clean-terminal-bench-smoke-final-20260603-173900\runs\terminal_bench__hello-world\20260603-173904-812`
- Result: both standard and TaskSpace passed the public validator; TaskSpace produced map/node/edge evidence; uv cache proof was proven for both sides.
