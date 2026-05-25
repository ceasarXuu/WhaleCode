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

The TUI `/map-show` browser viewer has its own installed-binary E2E:

```powershell
.\scripts\run-tui-map-show-viewer-e2e.ps1
```

That path launches the installed `whale.exe`, sends `/map-mode experiment`, `/map-restart`, `/map-show`, makes a real user turn, then fetches the live `snapshot.json` URL exposed by the localhost viewer.

The script creates a temporary git repository under:

```text
target/real-user-e2e/action-map-real-user-cache-bugfix/<timestamp>/
```

It then runs:

```powershell
%USERPROFILE%\.whale\bin\whale.exe exec --json --map-mode experiment --map-restart -m deepseek-v4-flash -C <repo> --dangerously-bypass-approvals-and-sandbox --output-last-message <file> -
```

The prompt is passed through stdin, matching a real CLI user request without embedding a fake model response. `--map-restart` uses the app-server Action Map restart request before the turn starts, so the agent no longer needs to treat `/map-restart` as natural-language work or accidentally run it as a shell command.

The E2E also exports a human-readable observability bundle next to the raw artifacts:

```text
action-map-observability.html
action-map-observability.md
action-map-observability.json
```

The HTML view is intentionally static and local-only. It reconstructs the map, node states, lease lifecycle, subagent binding, result recording, collaboration tool calls, and timeline from the real rollout/JSONL output.

Runtime observability now has two direct read surfaces in addition to rollout export:

```text
/map-show
thread/actionMap/read
```

`/map-show` opens a localhost browser viewer from the TUI. The viewer polls `thread/actionMap/read` every 2 seconds, so it shows the current Action Map without forcing large map state into the terminal. `thread/actionMap/read` remains the structured snapshot API for viewer, automation, and external observability integrations.

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
- The run contains no evidence that `/map-restart` was attempted as a shell command.
- The run contains no `failed to record rollout items` runtime errors.
- The Action Map observability HTML is generated.
- The command/API read path is covered by `/map-show` slash dispatch tests, live viewer endpoint tests, core snapshot formatting tests, and app-server protocol/schema checks.
- The TUI viewer path is covered by launching the installed `whale.exe`, opening `/map-show`, and reading the auto-refresh viewer endpoint.

## Latest Verified Run

Latest successful run:

```text
target/real-user-e2e/action-map-real-user-cache-bugfix/20260510-034456-537/artifacts/report.md
```

Observed evidence:

```text
overall: PASS
thread_started: 1
turn_completed: 1
command_execution: 28
git_diff_bytes: 768
spawn_agent: 2
map_created: 1
lease_created: 1
lease_attached: 1
map_completion_or_release: 1
map_restart_shell_misuse: 0
rollout_record_errors: 0
validation_exit_code: 0
```

## Real Bug Found

The first real run showed that `--map-mode experiment` only switched the Action Map mode, while the installed configuration could still expose the old multi-agent tool handler. That meant the agent could spawn a subagent without real node/lease binding.

Fix: `whale exec --map-mode experiment` now forces the existing `multi_agent_v2` feature on for that exec session by enabling `features.multi_agent_v2.enabled`, so Action Map node binding uses the existing v2 multi-agent infrastructure instead of the legacy spawn handler.

The next real run found a second runtime bug: `multi_agent_v2` claimed a node before validating spawn arguments. If validation failed, the node lease could remain stuck and block all later subagent work.

Fix: `multi_agent_v2` now performs pre-spawn validation before claiming an Action Map node, and releases the lease if later input/source construction fails.

The next product-path issue was command routing. `/map-restart` exists as a TUI slash command, but the real `whale exec` path had no equivalent machine entrypoint. A model could interpret the text as something to execute in PowerShell, which produces noise and hides the intended map lifecycle action.

Fix: the app-server now exposes `thread/actionMap/restart`, and `whale exec --map-restart` sends that request before the first turn. The real-user E2E script now requires the installed binary to expose both `--map-mode` and `--map-restart`, and fails if the run shows `/map-restart` shell misuse.

The same real run exposed a shutdown-time observability issue: after the app-server closed a session, late rollout appends could still try to write through a removed live thread recorder and log `failed to record rollout items: thread ... not found`.

Fix: session shutdown now marks the session as shutting down before tearing down live persistence. Late `ThreadNotFound` rollout appends during that window are treated as benign shutdown races instead of runtime errors, while non-shutdown persistence failures remain errors. The real-user E2E script now fails on rollout persistence errors in stderr.

The 2026-05-10 run exposed a regression in the first fix: passing `features.multi_agent_v2=true` on the CLI created the wrong TOML shape for a configurable feature, so the old handler could still be exposed. The E2E failed with no `lease_created` / `lease_attached` evidence.

Fix: the exec override now uses `features.multi_agent_v2.enabled=true`. The next installed-binary E2E run passed with real `spawn_agent`, `lease_created`, `lease_attached`, `lease_released`, static observability export, and no rollout persistence errors.

The 2026-05-25 dynamic-node regression found that runtime-created nodes such as `node-1` were not fully usable by the real `spawn_agent` path. The dispatcher could claim the dynamic node, but the node id was reused as the subagent path segment, and `AgentPath` rejects hyphens with `agent_name must use only lowercase letters, digits, and underscores`.

Fix: the Action Map dispatcher now exposes and schedules dynamic nodes, while `spawn_agent` sanitizes node-derived task names for `AgentPath` without changing the canonical map node id. The installed-binary E2E run `019e5b88-9ac9-7401-aa24-969a55b93c21` passed with `node-1` created, lease `lease-1` attached to `/root/node_1`, `node_result_recorded`, and `lease_released`.

Follow-up review on 2026-05-25 tightened the same path: non-final completion watcher exits now notify the parent before recording the blocker result, main binding skips nodes already running under a subagent lease, canceled main tool calls use the same TaskSpace prepare semantics as normal tool calls, and failed main tool attribution stores a generic failure preview instead of the raw internal error string. The installed-binary E2E run `019e5d54-e63f-75c2-a661-99c32ba09c70` passed with dynamic node creation, `/root/node_1` binding, `dynamic-subagent-ok`, `node_result_recorded`, `lease_released`, and successful `close_agent` cleanup.
