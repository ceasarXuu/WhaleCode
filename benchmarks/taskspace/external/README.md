# TaskSpace External Benchmark Intake

This directory tracks external benchmark sources used for E3 TaskSpace evidence.

Rules:

- Store source pointers, pinned revisions, sample ids, checksums, adapter versions, and license notes.
- Do not vendor full benchmark datasets into this repository.
- Do not store gold patches, solutions, hidden tests, secrets, or private user data here.
- External tasks must be materialized into temporary run directories before execution.
- Benchmark prompts must remain natural user instructions and must not mention TaskSpace internals.

Current priority:

1. DeepSWE adapter spike.
2. Terminal-Bench adapter spike.
3. SWE-bench Multilingual feasibility intake.

E3 results are not valid until an external sample has:

- pinned source metadata
- original instruction checksum
- validator checksum
- validator fidelity metadata proving an official or equivalent runner
- proof that validator source, hidden tests, and solutions are not readable by the agent
- file-level changed-path inventory with SHA256 values
- paired standard/taskspace artifacts
- external validator output
- artifact audit review
- runtime proof artifacts generated from the run, not only manifest assertions

Current Terminal-Bench adapter status:

- The adapter now validates through a real Docker `/app` post-hoc runtime: the agent workspace is mounted at `/app`, and `run-tests.sh` plus `tests/` are mounted read-only at `/tbench-validator`.
- The runner also creates a Windows `/app` execution alias for Terminal-Bench samples so `/app/foo` maps to the side repo root during Whale execution.
- Each external pair now emits `external-runtime-proof.json`, `external-isolation-proof.json`, and `external-e3-proof.json`.
- The isolation proof records validator source hash and whether the source is outside the agent repo. It does not promote `agent_cannot_read_validator_source` unless the adapter also has a stronger explicit isolation claim.
- `scripts/taskspace-benchmark/run-taskspace-e3-external.ps1` is the strict E3 entrypoint. It enforces `Repeats >= 5` and aggregate generation.
- This is not yet marked as official Terminal-Bench harness-equivalent E3 evidence. E3 promotion remains disabled until official runner equivalence, repeated runs, and audit review are proven from runtime artifacts.
