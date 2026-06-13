# Subagent VS Review: TaskSpace E3 Runtime Guardrails

- Created: 2026-06-13T16:13:00+08:00
- Updated: 2026-06-13T16:44:00+08:00
- Report schema: adversarial-v1
- Task: Review the TaskSpace E3 runtime guardrail fixes before rerunning the long E3 suite.
- Report path: `vs_review/2026-06-13-taskspace-e3-runtime-guardrails-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: closed

## Round 1: Runtime Guardrail Readiness

### Review Input

#### Objective
Confirm whether the TaskSpace E3 guardrail changes are robust enough to proceed with another long 0.0.4 E3 run after a prior run was aborted by WSL/Docker failures caused by D drive exhaustion.

#### Review Target
Code implementation, operational guardrails, test strategy, and observability for TaskSpace E3 runtime preflight and validator probe handling.

#### Target Locations
- `scripts/taskspace-benchmark/lib/harness-health.ps1`
- `scripts/taskspace-benchmark/adapters/terminal-bench-adapter.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1`
- `scripts/taskspace-benchmark/test-harness.ps1`
- `coe/2026-06-13-05-10-e3-validator-probe-wsl.md`
- Commit range: `68ceda725..c19d901c2`

#### Change Introduction
The implementation adds stable classification for Docker/WSL validator probe failures, validates Docker backends before selecting them, and adds a preflight disk-space guard in harness health. The disk guard defaults to 20 GiB minimum free space and can be overridden with `TASKSPACE_MIN_FREE_BYTES` or `TASKSPACE_MIN_FREE_GIB`. A forced high-threshold runner smoke confirmed `disk_space_low` exits as `invalid_harness` with exit code 3 before pair execution.

#### Risk Focus
- Whether the space guard checks all disks that can fill during an E3 run, including run roots, repo/scenario roots, uv cache, Docker wrapper, and WSL/Docker VHDX location.
- Whether the 20 GiB default is too low or too high for 5-pair Terminal-Bench E3 runs.
- Whether path/root detection misses non-D-drive run roots or nonstandard Docker installs.
- Whether the preflight abort artifacts are actionable enough during an incident.
- Whether backend probing can hang, misclassify, or silently select an unusable fallback.
- Whether the tests prove the operational failure modes rather than only implementation details.

#### User-Perspective Review Focus
- A future operator should understand why E3 stopped, what disk failed, how much space is required, and what command/env override is relevant.
- A future operator should not waste hours running E3 when the environment is obviously unsafe.

#### Assumptions To Attack
- One free-space snapshot before the run is sufficient for multi-hour E3.
- 20 GiB is a safe default for this benchmark family.
- Checking one root per drive is enough.
- `D:\whale-docker` is the only local WSL/Docker disk path that matters on this machine.
- `Get-Command docker` source and manifest paths are enough to infer storage pressure.
- Tests using impossible thresholds represent real low-disk behavior well enough.

#### Adversarial Lenses
- release
- operations
- failure
- state
- testing
- observability
- maintenance

#### Verification Status
- `git diff --check`: pass
- PowerShell parse checks: pass
- `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1`: pass
- `scripts/taskspace-benchmark/test-harness.ps1`: pass
- Runner smoke with `TASKSPACE_MIN_FREE_BYTES=[int64]::MaxValue`: exited 3, wrote `disk_space_low` in `harness-health.json` and abort summary.
- Docker/WSL after cleanup: `docker version` and `wsl -d whale-docker -- docker version` returned `29.1.3`.

#### Reviewer Instructions
- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Cite evidence paths and line numbers when possible.
- Focus on falsifying readiness to rerun a long E3 suite.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | one bounded extension up to 10 minutes if alive | 2 | cannot pass if review is unavailable |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| release-ops-adversary | The highest-value risk is wasting another multi-hour E3 run or failing to recover/diagnose an operational environment problem. | runtime preflight, operational failure, recovery clarity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | multi_agent_v1.spawn_agent | 019ec011-cf4f-7770-ae42-c9c760846064 | spawn_agent returned agent_id and nickname Godel | fork_context=false | Round 1 Review Input | main-agent history, reasoning, drafts, conclusions, full diff unless independently inspected | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| release-ops-adversary-r1 | release-ops-adversary | 1 | 019ec011-cf4f-7770-ae42-c9c760846064 | 15 minutes | completed | reviewer returned findings | completed |

### Reviewer Outputs

#### release-ops-adversary-r1

##### Summary
Verdict: not robust enough to proceed with another long 0.0.4 E3 run without fixes. The changes improve classification and catch obvious host-drive low-space states, but they still do not prove the actual Docker/WSL storage path is safe, do not monitor disk during a 5-pair run, and do not make abort artifacts actionable enough for operators.

##### Blocking Findings
- Disk guard does not check actual Docker/WSL storage capacity.
  - Broken assumption: checking Windows roots plus `D:\whale-docker` is enough.
  - Failure scenario: host D has more than 20 GiB, but the WSL filesystem or Docker data root inside the distro is full or near full; Docker build/run fails mid-E3.
  - Trigger condition: `whale-docker` data-root or ext4 VHDX fills independently of Windows `DriveInfo`.
  - Impact: another multi-hour E3 can still die as Docker/WSL infra noise.
  - Proof needed: preflight records `wsl -d <distro> -- df -Pk <docker-root>` and `docker info` data root, with thresholds and JSON evidence.
- Single preflight snapshot is insufficient for a 5-pair E3 run.
  - Broken assumption: one free-space check before pair execution is enough.
  - Failure scenario: preflight passes at 21-30 GiB, then Docker layer cache, logs, workspaces, and validation artifacts consume remaining space during pair 2-5.
  - Trigger condition: long Terminal-Bench sample with repeated Docker builds/runs and Whale logs.
  - Impact: failure occurs after significant runtime instead of aborting early.
  - Proof needed: per-pair or pre/post-pair disk checks on all relevant roots, plus a suite-level trend artifact.
- External materialization can fail before the new disk guard runs.
  - Broken assumption: the new harness health preflight protects the whole E3 path.
  - Failure scenario: adapter materialization/download/copy fills or hits low space before `Get-TaskspaceHarnessHealth` runs, then reports generic materialization failure.
  - Trigger condition: E3 suite calls `run-taskspace-external-benchmark.ps1`, which runs the adapter before benchmark runner preflight.
  - Impact: operator does not get `disk_space_low`, free/required bytes, or override guidance.
  - Proof needed: external materialization preflight before adapter execution, including `RunRoot`, scenario output root, task source root, Docker/WSL storage, and uv cache target.
- `disk_space_low` is not treated as an immediate suite-global circuit breaker.
  - Broken assumption: sample-level abort is enough.
  - Failure scenario: first sample fails for disk space, suite still attempts a second sample before signature count reaches 2.
  - Trigger condition: `disk_space_low` invalid harness in E3 suite without `-ContinueAfterInvalidHarness`.
  - Impact: avoidable extra materialization and confusion during an already unsafe environment.
  - Proof needed: suite test showing first `disk_space_low` stops remaining samples.

##### Non-blocking Risks
- 20 GiB default is unproven. It may be too low for 10 Docker validations plus logs, or too high for small diagnostic runs. Current tests only use `1` byte and `[int64]::MaxValue`, not realistic thresholds.
- Native Docker fallback can be semantically wrong when `docker` is a WSL wrapper. It probes `docker version` successfully, labels backend `native`, then uses Windows mount paths.
- Invalid env var values for `TASKSPACE_MIN_FREE_BYTES/GIB` throw before structured abort.

##### User-Perspective Checks
- Usability: risk - aborts identify `invalid_harness`, `disk_space_low`, root, available GiB, required GiB, and `harness-health.json`, but not remediation.
- Ease of use: risk - operator does not get cleanup targets or override variable guidance in abort summary.
- Ease of understanding: risk - abort summary does not state whether Docker data root or WSL filesystem was checked.

##### Required Fixes
- Add Docker/WSL storage discovery and checks: Docker backend, distro, `docker info` data root, WSL `df`, VHDX host path where available.
- Run disk checks before external materialization and before each pair.
- Treat `disk_space_low` as suite-global immediate circuit breaker.
- Make abort summary include failed root/path, free/required GiB, `TASKSPACE_MIN_FREE_GIB` / `TASKSPACE_MIN_FREE_BYTES`, and likely cleanup paths.
- Detect WSL-wrapper `docker.cmd` and avoid labeling it as native unless mount semantics are validated.

##### Missing Tests
- Low-space simulation on a real temporary volume or mocked `DriveInfo` equivalent, not only impossible threshold.
- WSL Docker data-root full/near-full fixture.
- Forced `TASKSPACE_DOCKER_BACKEND=native` with a WSL-wrapper `docker.cmd`.
- Suite-level first-sample `disk_space_low` circuit-breaker test.
- External materialization low-disk preflight test.

##### Missing Logs / Observability
- No per-root disk trend over time.
- No Docker data-root / WSL filesystem free-space artifact.
- No suite-level disk snapshot before each sample.
- Abort summary lacks operator remediation commands/env overrides.

##### Evidence
- `scripts/taskspace-benchmark/lib/harness-health.ps1` - disk guard only adds run/scenario/manifest paths, `Get-Command docker`, and hard-coded `D:\whale-docker`.
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` - preflight currently runs before the pair loop, not before each pair.
- `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1` - adapter materialization runs before benchmark runner preflight.
- `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` - suite-global signatures do not include `disk_space_low`.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| release-ops-adversary | Missing Docker/WSL storage capacity check | Host root free space does not prove WSL/docker data root is safe. | blocking | accept | Reviewer evidence matches the prior incident class and current implementation only checks Windows roots. | fixed with WSL `df -Pk` checks for `/` and DockerRootDir plus JSON evidence | closure review required |
| release-ops-adversary | Single preflight snapshot insufficient | Disk can be consumed during pairs 2-5 after preflight passed. | blocking | accept | Pair loop currently starts after one harness health check. | fixed with per-pair `harness-health-pair-XXX.json` and pre-pair abort | closure review required |
| release-ops-adversary | External materialization can fail before disk guard | Adapter may consume/fail on disk before benchmark preflight runs. | blocking | accept | External benchmark wrapper runs adapter before runner. | fixed with external materialization preflight before adapter execution | closure review required |
| release-ops-adversary | disk_space_low not suite-global immediate breaker | Suite may waste one more sample after first low-space invalid harness. | blocking | accept | Suite global signatures omit `disk_space_low`. | fixed with suite sample preflight and global circuit breaker for `disk_space_low` / invalid threshold | closure review required |
| release-ops-adversary | 20 GiB default unproven | Threshold may be too low for long E3. | major | accept | Prior D-space failure was severe; current threshold has no empirical basis. | retained as configurable default with per-sample/pair monitoring; no empirical threshold change before E3 | observe during E3 |
| release-ops-adversary | Native Docker fallback can be wrong for WSL wrapper | Docker wrapper can be labeled native and use Windows mount paths. | major | accept | `D:\whale-docker\bin\docker.cmd` wrapper evidence exists in COE. | generated validator now detects `.cmd`/`.bat` wrappers containing WSL docker and refuses requested native backend | covered by `test-harness.ps1` text assertion |
| release-ops-adversary | Invalid threshold env vars throw unstructured | Bad env var can bypass structured abort. | major | accept | Direct numeric casts currently happen in helper. | invalid `TASKSPACE_MIN_FREE_BYTES/GIB` now produces `disk_space_threshold_invalid` invalid_harness health | covered by invalid-threshold smoke |

### Main-Agent Fix Evidence

| Finding | Fix Location | Evidence |
|---|---|---|
| Missing Docker/WSL storage capacity check | `scripts/taskspace-benchmark/lib/harness-health.ps1` | `New-TaskspaceDiskHealth` records host disk checks plus `docker_storage_checks`; WSL probe records distro, `/`, DockerRootDir, free/required bytes, and status. Normal probe returned pass with `whale-docker:/` and `whale-docker:/var/lib/docker`. |
| Single preflight snapshot insufficient | `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1` | Pair loop now writes `harness-health-pair-XXX.json`, emits `pair_disk_health_completed`, and aborts with exit 3 before the pair starts when disk health fails. |
| External materialization can fail before disk guard | `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1` | Wrapper now runs `New-TaskspaceDiskHealth` before adapter materialization and writes `external-materialization-preflight-health.json`, `abort-summary.md`, `run-status.json`, and `sample-status.json` on invalid harness. |
| `disk_space_low` not suite-global immediate breaker | `scripts/taskspace-benchmark/run-taskspace-e3-suite.ps1` | Suite now writes `suite-disk-health-XXX.json` before each sample and treats `disk_space_low` / `disk_space_threshold_invalid` as global circuit breaker signatures. |
| Actionable abort summary | `scripts/taskspace-benchmark/lib/harness-health.ps1` | `New-TaskspaceHarnessAbortSummaryLines` adds failed root/path, free/required GiB, threshold override env vars, and cleanup target hints for `disk_space_low`; invalid threshold summaries show only the relevant env vars. |

### Main-Agent Verification After Fixes

| Check | Result | Evidence Path / Output |
|---|---|---|
| `git diff --check` | pass | no whitespace errors; only CRLF conversion warnings from Git |
| PowerShell parse checks | pass | parsed changed PowerShell files with no parser errors |
| `scripts/taskspace-benchmark/test-e3-harness-guardrails.ps1` | pass | `D:\whalecode-alpha\target\e3-guardrails-selftest\20260613-163832-968` |
| `scripts/taskspace-benchmark/test-harness.ps1` | pass | `D:\whalecode-alpha\target\paired-bench-selftest\single-file-fast-fix\20260613-163833-578` |
| external materialization low-space smoke | pass | `D:\whalecode-alpha\target\external-disk-preflight-smoke-2\external-materialization-preflight-health.json`; exited 3 before adapter materialization |
| suite low-space circuit breaker smoke | pass | `D:\whalecode-alpha\target\suite-disk-preflight-smoke-2\suite-20260613-163722\suite-health.json`; first sample `disk_space_low`, second sample `suite_circuit_breaker` |
| invalid threshold smoke | pass | `D:\whalecode-alpha\target\external-invalid-threshold-smoke-2\abort-summary.md`; stable code `disk_space_threshold_invalid`, exited 3 |
| normal disk health probe | pass | returned `status=pass` with host `C:\`, `D:\`, and WSL `whale-docker:/var/lib/docker` checks |
| suite disk-guard skip reason smoke | pass | `D:\whalecode-alpha\target\suite-disk-preflight-smoke-3\suite-20260613-164323\skipped-samples.jsonl`; skipped reason is `suite_global_disk_guard` |

## Round 2: Blocking Closure Review

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| release-ops-closure-adversary | multi_agent_v1.spawn_agent | 019ec022-f0b7-74f2-9985-88390387af97 | spawn_agent returned agent_id and nickname Hegel | fork_context=false | Round 2 closure prompt with Round 1 findings, current diff, test artifacts | main-agent history, reasoning, drafts, conclusions except explicit prompt packet | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| release-ops-closure-adversary-r2 | release-ops-closure-adversary | 1 | 019ec022-f0b7-74f2-9985-88390387af97 | 15 minutes | completed | reviewer returned closure verdict | completed |

### Reviewer Outputs

#### release-ops-closure-adversary-r2

##### Verdict
PASS to rerun E3 from the current working tree.

##### Blocking Findings
None. The four accepted Round 1 blockers appear closed enough to avoid another obvious multi-hour waste from disk/Docker preflight failures.

##### Closure Evidence Checked
- Docker/WSL storage is checked via DockerRootDir plus WSL `df -Pk` on `/` and DockerRootDir in `scripts/taskspace-benchmark/lib/harness-health.ps1`, with JSON `docker_storage_checks` emitted.
- Per-pair checks run before each pair execution and abort with exit 3 before workspace creation in `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`.
- External materialization has a pre-materialization disk preflight before adapter execution in `scripts/taskspace-benchmark/run-taskspace-external-benchmark.ps1`.
- Suite-level `disk_space_low` is global: the suite sets `$suiteAbort` after first sample preflight failure and skips subsequent samples before materialization; child invalid signatures also treat `disk_space_low` and invalid threshold as global.
- Invalid threshold env vars become structured `disk_space_threshold_invalid`.
- WSL-wrapper `docker.cmd` is no longer labeled native; generated validator detects `.cmd` / `.bat` wrappers containing WSL Docker and rejects requested native backend.
- Abort summaries include root/path, free/required GiB, override env vars, and cleanup hints.

##### Nonblocking Risks
- Review report closure was still pending at the time of read-only closure review. This section resolves that documentation gap.
- Suite skip reason was mildly misleading for disk guard skips; main agent fixed it and verified `suite_global_disk_guard`.

##### Recommendation
Rerun E3 from this exact working tree, and record the new suite root plus `suite-health.json` immediately after launch so any remaining infra abort is traceable.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: yes
- Blocking re-review completed: yes
- Blocking re-review passed: yes
- Blocking re-review round links:
  - Round 2: `release-ops-closure-adversary-r2`
- Blocking re-review launch records:
  - `019ec022-f0b7-74f2-9985-88390387af97`
- Rejected findings backed by evidence: n/a
- Deferred findings documented: n/a
- Blocked reason: n/a
- Allowed to proceed: yes

## Final Conclusion

Round 1 found accepted blocking findings. Main-agent fixes and Round 2 closure review resolved the blockers. E3 rerun is allowed from the current working tree.
