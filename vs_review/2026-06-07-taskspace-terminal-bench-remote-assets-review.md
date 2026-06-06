# TaskSpace Terminal-Bench Remote Asset Review

## Review Target

- target type: code implementation and benchmark evidence gate
- objective: repair P0/E3 Terminal-Bench remote asset preflight without weakening E3 proof boundaries
- files:
  - `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
  - `scripts/taskspace-benchmark/adapters/terminal-bench-remote-assets.ps1`
  - `scripts/taskspace-benchmark/adapters/terminal-bench-uv-cache.ps1`
  - `scripts/taskspace-benchmark/test-terminal-bench-adapter-harness.ps1`

## Round 1 Launch Record

- reviewer role: benchmark harness adversarial reviewer
- mechanism: internal subagent `multi_agent_v1.spawn_agent`
- reviewer id: `019e9f1d-b121-7273-9a06-ab8d98c30441`
- fork_context: false
- context isolation: reviewer received only a neutral navigation packet with target files, observed P0 failure mode, and review questions
- permissions: read-only, explicitly instructed not to modify files
- trace source: subagent notification in current Codex thread

## Round 1 Reviewer Output Summary

The reviewer accepted the high-level direction that Terminal-Bench uv installer URLs should not be treated as unknown task remote assets when they are already covered by the validator uv cache. It reported three blocking findings:

- B1: uv coverage was URL-global and could incorrectly allow Docker build-stage assets.
- B2: all URLs were treated as required file remote assets, causing registry/source endpoints to block E3 even when they were not downloaded artifacts.
- B3: uv coverage metadata did not point at concrete cache files and used copied hash values rather than proof from actual files.

Non-blocking risks included wide hash association, limited curl rewrite support, and comment URL parsing edge cases.

## Main-Agent Response

- B1: accept. Implemented context classification. uv cache coverage is now only applied when the URL appears in validator runtime paths such as `run-tests.sh`, `verify.sh`, `test.sh`, or `tests/*`. Dockerfile uv URLs remain required unknown network dependencies unless independently proven.
- B2: accept. Added remote asset kinds: `materialized_file_asset`, `validator_dependency_cache`, `registry_or_source_endpoint`, `metadata_comment`, and `unknown_runtime_network_dependency`. Only materialized file assets and unknown runtime network dependencies are required for E3.
- B3: accept. `New-TerminalBenchUvCache` now records concrete installer/archive paths and byte sizes. Remote asset metadata for uv coverage now points to the concrete cache file and recomputes actual SHA-256.
- Additional accepted issue found during validation: single-line files were under-scanned because `Get-Content` returned a scalar string and `$lines[$index]` indexed characters. Fixed by array-wrapping `Get-Content`.

## Validation Evidence

- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-e3-proof-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-terminal-bench-uv-cache-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-external-wrapper-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-harness.ps1`: PASS
- PowerShell file length check: all `scripts/taskspace-benchmark/**/*.ps1` files remain under 500 lines.

## Closure Status

- status: closure review round 1 found an accepted blocker; fix implemented, awaiting final closure review
- accepted blocking findings: B1, B2, B3
- closure review required: yes

## Closure Review Round 1 Launch Record

- reviewer role: closure reviewer
- mechanism: internal subagent `multi_agent_v1.spawn_agent`
- reviewer id: `019e9f27-e185-7cb0-b2f4-5fe97a8d50bc`
- fork_context: false
- context isolation: reviewer received only the report path, target files, and closure questions
- permissions: read-only
- trace source: subagent notification in current Codex thread

## Closure Review Round 1 Output Summary

The closure reviewer found B1 mostly closed and B3 implementation mostly closed, but reported a remaining blocking B2 case:

- C1: Dockerfile `ARG/ENV` URLs were too broadly classified as `registry_or_source_endpoint`. A file URL such as `ARG ASSET_URL=https://example.invalid/asset.tar.gz` followed by `RUN curl -o /app/asset.tar.gz "$ASSET_URL"` could be treated as non-required and allow E3 to fail open.

It also requested stronger uv proof assertions.

## Main-Agent Closure Response

- C1: accept. Dockerfile `ARG/ENV` URLs are now classified by file-like leaf. Registry/source endpoints remain non-required only when the URL does not look like a materialized file; file-like URLs now fail closed as `unknown_runtime_network_dependency`.
- Test added: `ARG ASSET_URL=https://example.invalid/asset.tar.gz` plus variable curl download must downgrade remote asset proof.
- Test strengthened: uv cache coverage now asserts concrete cache path, actual SHA, expected SHA, equality, `equivalence_proven=true`, and actual file size.

## Closure Validation Evidence

- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-terminal-bench-adapter-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-e3-proof-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-terminal-bench-uv-cache-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-external-wrapper-harness.ps1`: PASS
- `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\taskspace-benchmark\test-harness.ps1`: PASS
- `git diff --check`: PASS
- PowerShell file length check: all `scripts/taskspace-benchmark/**/*.ps1` files remain under 500 lines.
- Real P0 preflight on pinned Terminal-Bench `1a6ffa9674b571da0ed040c470cb40c4d85f9b9b`:
  - `processing-pipeline`: E3 eligible; remote asset downgrade false.
  - `multi-source-data-merger`: E3 eligible; remote asset downgrade false.
  - `recover-accuracy-log`: E3 eligible; remote asset downgrade false.
  - `query-optimize`: E3 ineligible; remote asset downgrade true because a real unproven remote file asset remains fail-closed.

## Final Closure Review

- reviewer role: final closure reviewer
- mechanism: internal subagent `multi_agent_v1.spawn_agent`
- reviewer id: `019e9f2c-1888-75f2-b855-46cbb1478299`
- fork_context: false
- context isolation: reviewer received only the report path, target files, focused closure questions, and validation summary
- permissions: read-only
- verdict: passed closure with no blocking findings
- non-blocking residual risk: extensionless Dockerfile `ARG/ENV` asset URLs used indirectly by later curl commands are still heuristic; future hardening can add Dockerfile variable dataflow or make all Dockerfile ARG/ENV URLs conservative by default.

## Final Closure Status

- status: passed
- unresolved blocking findings: none
