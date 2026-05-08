# Action Map real-user E2E

Date: 2026-05-08

## Purpose

This is the non-mock validation path for Action Map multi-agent work.

The test must run the installed `whale.exe`, send a real user prompt through `whale exec`, call the configured real model, let the real agent invoke tools and subagents, and inspect the resulting real JSONL output plus rollout records.

Offline unit tests and mock Responses servers remain useful for deterministic regression, but they do not count as real end-to-end validation.

## Entry Point

```powershell
.\scripts\run-action-map-real-user-e2e.ps1
```

The script creates a temporary git repository under:

```text
target/real-user-e2e/action-map-real-user-cache-bugfix/<timestamp>/
```

It then runs:

```powershell
%USERPROFILE%\.whale\bin\whale.exe exec --json --map-mode experiment -m deepseek-v4-flash -C <repo> --dangerously-bypass-approvals-and-sandbox --output-last-message <file> -
```

The prompt is passed through stdin, matching a real CLI user request without embedding a fake model response.

## Required Evidence

The report is marked PASS only when all of these are true:

- `whale exec` exits with code 0.
- The post-run validation command exits with code 0.
- The JSONL stream contains a real thread start, turn completion, command executions, `spawn_agent`, and agent messages.
- The sandbox git diff is non-empty.
- The validation output prints `cache validation passed`.
- The copied rollout contains Action Map runtime evidence:
  - `mode_changed`
  - `map_created`
  - `lease_created`
  - `lease_attached`
  - `node_result_recorded` or `lease_released`

## Latest Verified Run

Latest successful run:

```text
target/real-user-e2e/action-map-real-user-cache-bugfix/20260508-161749-774/artifacts/report.md
```

Observed evidence:

```text
thread_started: 1
turn_completed: 1
command_execution: 32
spawn_agent: 2
map_created: 1
lease_created: 1
lease_attached: 1
map_completion_or_release: 1
validation_exit_code: 0
```

## Real Bug Found

The first real run showed that `--map-mode experiment` only switched the Action Map mode, while the installed configuration could still expose the old multi-agent tool handler. That meant the agent could spawn a subagent without real node/lease binding.

Fix: `whale exec --map-mode experiment` now forces the existing `multi_agent_v2` feature on for that exec session, so Action Map node binding uses the existing v2 multi-agent infrastructure.

The next real run found a second runtime bug: `multi_agent_v2` claimed a node before validating spawn arguments. If validation failed, the node lease could remain stuck and block all later subagent work.

Fix: `multi_agent_v2` now performs pre-spawn validation before claiming an Action Map node, and releases the lease if later input/source construction fails.
