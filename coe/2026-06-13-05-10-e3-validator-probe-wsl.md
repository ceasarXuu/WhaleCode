# Problem P-001
0.0.4 E3 formal run did not complete. The guardrail stopped two samples as `invalid_harness` because the external validator probe failed with `getpwnam(root) failed 5` while invoking the WSL/Docker validator path. The fix criterion is that the immediate infra failure is understood, the harness classifies it as a stable infrastructure failure, and a minimal validator probe/regression path demonstrates that the same failure no longer occurs after environment recovery.

# Hypothesis H-001
The E3 abort was caused by local D drive exhaustion. `whale-docker` stores its WSL VHDX on D, and the TaskSpace run roots also accumulated under D. When D reached 0 free bytes, WSL/Docker startup failed with root user lookup and I/O errors. Prediction: after freeing D drive space, `wsl -d whale-docker -- id` and Docker version probes should recover without changing benchmark task definitions.

# Hypothesis H-002
The harness still needs a code fix because it previously treated a Docker wrapper/backend that had not passed a live probe as usable, and classified WSL root lookup/I/O errors as a generic `validator_probe_failed`. Prediction: generated validators should require a successful, time-bounded `docker version` probe before selecting a backend and should classify `getpwnam(root)`, `getpwuid(0)`, WSL `E_UNEXPECTED`, and WSL I/O errors as `docker_backend_unavailable`.

# Evidence E-001
Run artifact `C:\w\e3v004-20260613-031321\runs\suite-20260613-031322\samples\multi-source-data-merger\runs\terminal_bench__multi-source-data-merger\20260613-045206-997\pair-001\left\artifacts\validator-probe.stderr.log` contained `external-validator.ps1 : <3>WSL (1148 - Relay) ERROR: CreateProcessParseCommon:1014: getpwnam(root) failed 5`.

# Evidence E-002
No `TASKSPACE_DOCKER_BACKEND` or `TASKSPACE_DOCKER_WSL_DISTRO` environment override was set in the shell used for diagnosis. `wsl.exe -l -v` reported `whale-docker` as the default WSL2 distro.

# Evidence E-003
Before space cleanup, `wsl.exe -d whale-docker -- id`, `wsl.exe -d whale-docker -- getent passwd root`, and `wsl.exe -d whale-docker -- docker version --format '{{.Server.Version}}'` all failed outside TaskSpace with `getpwnam(root) failed 5`, `getpwuid(0) failed 5`, and `I/O error @util.cpp`.

# Evidence E-004
`Get-Command docker` resolved to `D:\whale-docker\bin\docker.cmd`, whose content was `wsl -d whale-docker -- docker %*`. The Windows `docker` command was therefore a wrapper around the same WSL distro, not an independent native fallback.

# Evidence E-005
`Get-PSDrive -PSProvider FileSystem` reported drive `D:` free bytes as `0` during the failed validation/debug cycle. `test-harness.ps1` also failed with `curl: (23) Failure writing output to destination` while writing a uv cache under `D:\whalecode-alpha\target`.

# Evidence E-006
After moving generated run artifacts from `D:\whalecode-alpha\target`, `D:\whalecode-alpha\target-test`, and `D:\whalecode-alpha\target-map-show` into `E:\whalecode-alpha-target-backup\20260613-d-drive-full`, drive `D:` recovered to about 92.86 GiB free.

# Evidence E-007
After space cleanup, `wsl.exe -d whale-docker -- id` returned `uid=0(root) gid=0(root) groups=0(root)`, and both `docker version --format '{{.Server.Version}}'` and `wsl.exe -d whale-docker -- docker version --format '{{.Server.Version}}'` returned `29.1.3`.

# Evidence E-008
PowerShell parse validation passed for `terminal-bench-adapter.ps1`, `harness-health.ps1`, `test-e3-harness-guardrails.ps1`, and `test-harness.ps1`.

# Evidence E-009
`powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1` passed. The self-test now includes a WSL `getpwnam(root) failed 5` stderr fixture classified as `docker_backend_unavailable`.

# Evidence E-010
`powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-harness.ps1` passed. The harness regression verifies generated Terminal-Bench validators contain the time-bounded Docker backend probe and WSL root lookup failure classification.

# Evidence E-011
A generated Terminal-Bench `external-validator.ps1 -ProbeOnly -ProbeDocker` run passed after space cleanup. Output included `docker_backend=wsl`, Docker server version `29.1.3`, `docker_version_exit=0`, `validator_probe_completed=true`, and `validator-probe-result.json` with `status: pass`, `stage: probe_docker`, and `failure_signature: null`.
