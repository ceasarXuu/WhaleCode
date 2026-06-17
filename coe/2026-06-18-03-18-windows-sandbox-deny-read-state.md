# Problem P-001: Windows sandbox deny-read state parse failure pollutes TaskSpace benchmark
- Status: fixed
- Created: 2026-06-18 03:18
- Updated: 2026-06-18 03:54
- Objective: Prevent corrupted Windows sandbox deny-read ACL state from failing ordinary sandboxed commands and invalidating v0.0.5 TaskSpace benchmark evidence.
- Symptoms:
  - `large-output-ref-smoke` TaskSpace side solved the business task but expanded to 7 nodes after a read command failed with a Windows sandbox ACL state parse error.
- Expected behavior:
  - A corrupt or unreadable persistent deny-read state file should not fail unrelated sandboxed command execution when the runtime can safely rebuild desired deny-read ACL state for the current principal.
- Actual behavior:
  - A command fails with `windows sandbox: parse deny-read ACL state C:\Users\77585\.whale\.sandbox\deny_read_acl_state.json`.
- Impact:
  - v0.0.5 live smoke cost evidence becomes unreliable because sandbox infrastructure failures trigger extra TaskSpace recovery nodes and model turns.
- Reproduction:
  - Run `large-output-ref-smoke` TaskSpace side on Windows after the sandbox state file becomes unparsable.
- Environment:
  - Windows, PowerShell, branch `whalecode-alpha`, latest known pushed commit before this case `ffde65cdb`.
- Known facts:
  - E-001
  - E-002
  - E-004
  - E-005
  - E-006
- Ruled out:
  - none
- Fix criteria:
  - A focused test proves corrupt state no longer causes `load_state` / sync startup to fail.
  - Existing deny-read state tests, if present, still pass.
  - A focused live smoke or self-test shows the original parse-error signature is gone.
- Current conclusion: H-001 is confirmed and repaired; focused live smoke no longer contains the original sandbox parse-error signature.
- Related hypotheses:
  - H-001
  - H-002
- Resolution basis:
  - H-001
  - E-001
  - E-002
  - E-004
  - E-005
  - E-006
- Close reason:
  - fixed

## Hypothesis H-001: Corrupt persistent state is treated as fatal
- Status: confirmed
- Parent: P-001
- Claim: `windows-sandbox-rs/src/deny_read_state.rs::load_state` fails command setup when `deny_read_acl_state.json` exists but cannot be parsed, instead of quarantining/rebuilding the state file.
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The live smoke error text names parse failure of the deny-read ACL state file, and code inspection shows `serde_json::from_slice` errors are returned directly.
- Falsifiable predictions:
  - If true: code has a `serde_json::from_slice` path whose error context is `parse deny-read ACL state`, and no recovery branch for malformed JSON.
  - If false: parse errors are already recovered or quarantined somewhere before command execution fails.
- Diagnostic evidence plan:
  - Prediction or clause under test: inspect code path and live smoke error for the same error context.
  - Signal: code-location plus benchmark artifact error text.
  - Capture method: read `deny_read_state.rs` and the failing `whale-exec.jsonl`.
  - Event name or marker:
    - parse deny-read ACL state
  - Correlation keys:
    - RunDir `target\v005-powershell-cdand-largeout-r1\large-output-ref-smoke\20260618-030314-126`
  - Differentiates from:
    - H-002
  - Supports if:
    - The code returns parse errors directly and the live artifact contains the same parse-error context.
  - Refutes if:
    - The code already recovers parse errors or the artifact error originates from a different path.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-004
  - E-005
  - E-006
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: none for this bug; continue v0.0.5 cost work separately.
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: The failure is caused by missing benchmark permissions rather than persistent state corruption
- Status: unverified
- Parent: P-001
- Claim: The read command fails because benchmark permissions omit access to `pyproject.toml`, and the state parse text is only incidental.
- Layer: alternative
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - The failed command was `Get-Content pyproject.toml`, so a permissions issue is a plausible alternative.
- Falsifiable predictions:
  - If true: other reads in the same batch should fail with access denied or deny-read policy messages.
  - If false: other file reads in the same batch succeed while only one command reports the ACL state parse error.
- Diagnostic evidence plan:
  - Prediction or clause under test: compare sibling command results in the same `whale-exec.jsonl` batch.
  - Signal: command outputs around item 6.
  - Capture method: inspect the artifact lines adjacent to the failure.
  - Event name or marker:
    - item_6
  - Correlation keys:
    - RunDir `target\v005-powershell-cdand-largeout-r1\large-output-ref-smoke\20260618-030314-126`
  - Differentiates from:
    - H-001
  - Supports if:
    - Multiple sibling reads fail with policy/access denied.
  - Refutes if:
    - Sibling reads of README/tests/source succeed while only pyproject read reports state parse error.
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: pending
- Related evidence:
  - E-003
- Conclusion: unverified
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: inspect sibling command results.
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: Live smoke command failed on deny-read ACL state parse
- Related hypotheses:
  - H-001
- Direction: supports
- Type: observation
- Source: `target\v005-powershell-cdand-largeout-r1\large-output-ref-smoke\20260618-030314-126\pair-001\right\artifacts\whale-exec.jsonl`
- Prediction or plan link:
  - H-001 predicts live artifact contains the `parse deny-read ACL state` error context.
- Matched signal:
  - `windows sandbox: parse deny-read ACL state C:\Users\77585\.whale\.sandbox\deny_read_acl_state.json`
- Correlation keys:
  - RunDir `target\v005-powershell-cdand-largeout-r1\large-output-ref-smoke\20260618-030314-126`
- Raw content:
  ```text
  execution error: Io(Custom { kind: Other, error: "windows sandbox: parse deny-read ACL state C:\\Users\\77585\\.whale\\.sandbox\\deny_read_acl_state.json" })
  ```
- Interpretation: The runtime failure was in sandbox ACL state parsing, not in the Python project task itself.
- Time: 2026-06-18 03:18

## Evidence E-002: `load_state` returns JSON parse errors directly
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party\codex-cli\codex-rs\windows-sandbox-rs\src\deny_read_state.rs`
- Prediction or plan link:
  - H-001 predicts a fatal `serde_json::from_slice` path with `parse deny-read ACL state` context.
- Matched signal:
  - `serde_json::from_slice(&bytes).with_context(|| format!("parse deny-read ACL state {}", path.display()))`
- Correlation keys:
  - none
- Raw content:
  ```text
  fn load_state(path: &Path) -> Result<PersistentDenyReadAclState> {
      match std::fs::read(path) {
          Ok(bytes) => serde_json::from_slice(&bytes)
              .with_context(|| format!("parse deny-read ACL state {}", path.display())),
          Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
              Ok(PersistentDenyReadAclState::default())
          }
          Err(err) => Err(err).with_context(|| format!("read deny-read ACL state {}", path.display())),
      }
  }
  ```
- Interpretation: Existing code has no malformed-state recovery branch.
- Time: 2026-06-18 03:18

## Evidence E-003: Sibling reads succeeded in same batch
- Related hypotheses:
  - H-002
- Direction: refutes
- Type: observation
- Source: `target\v005-powershell-cdand-largeout-r1\large-output-ref-smoke\20260618-030314-126\pair-001\right\artifacts\whale-exec.jsonl`
- Prediction or plan link:
  - H-002 predicts multiple sibling reads should fail if benchmark file permissions are the cause.
- Matched signal:
  - README, tests, and source reads completed successfully around the pyproject failure.
- Correlation keys:
  - RunDir `target\v005-powershell-cdand-largeout-r1\large-output-ref-smoke\20260618-030314-126`
- Raw content:
  ```text
  item_3 Get-Content README.md exit_code=0
  item_4 Get-Content src/large_output_demo.py exit_code=0
  item_5 Get-Content tests/test_large_output_demo.py exit_code=0
  item_6 Get-Content pyproject.toml failed with windows sandbox parse deny-read ACL state
  ```
- Interpretation: The failure is not explained by a simple benchmark permission omission for project files.
- Time: 2026-06-18 03:18

## Evidence E-004: Corrupt state focused tests pass
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: local cargo test output
- Prediction or plan link:
  - P-001 fix criteria require a focused test proving corrupt state no longer causes `load_state` / sync startup to fail.
- Matched signal:
  - `load_state_recovers_corrupt_json_with_backup` passed.
  - `store_state_replaces_corrupt_state_after_recovery` passed through the `deny_read_state` filter.
- Correlation keys:
  - none
- Raw content:
  ```text
  cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-windows-sandbox deny_read_state -- --nocapture
  test result: ok. 2 passed; 0 failed

  cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-windows-sandbox load_state_recovers_corrupt_json_with_backup -- --nocapture
  test result: ok. 1 passed; 0 failed
  ```
- Interpretation: The focused malformed-state path now preserves a backup and returns a usable default state instead of surfacing the parse error.
- Time: 2026-06-18 03:42

## Evidence E-005: Windows sandbox crate regression passes
- Related hypotheses:
  - H-001
- Direction: supports
- Type: regression-test
- Source: local cargo test output
- Prediction or plan link:
  - P-001 fix criteria require existing deny-read state tests and related Windows sandbox tests to still pass.
- Matched signal:
  - Full `codex-windows-sandbox` crate test suite passed.
- Correlation keys:
  - none
- Raw content:
  ```text
  cargo test --manifest-path third_party\codex-cli\codex-rs\Cargo.toml -p codex-windows-sandbox -- --nocapture
  test result: ok. 83 passed; 0 failed; 2 ignored
  Doc-tests codex_windows_sandbox
  test result: ok. 4 passed; 0 failed
  ```
- Interpretation: The repair does not regress the crate-level sandbox test suite.
- Time: 2026-06-18 03:42

## Evidence E-006: Focused live smoke no longer contains deny-read parse error
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: `target\v005-denyread-state-recovery-largeout-r1\large-output-ref-smoke\20260618-031943-172`
- Prediction or plan link:
  - P-001 fix criteria require a focused live smoke or self-test showing the original parse-error signature is gone.
- Matched signal:
  - Pair report has `infra_signatures: none`.
  - `Select-String` over TaskSpace `whale-exec.jsonl` found no `parse deny-read ACL state`, `corrupt deny-read`, `deny_read_acl_state`, or `execution error` matches.
- Correlation keys:
  - RunDir `target\v005-denyread-state-recovery-largeout-r1\large-output-ref-smoke\20260618-031943-172`
  - Installed Whale hash `19E7A64505EC3C924588A9C08F151ADE0B4AA0E2D6753001D230FD85D7233373`
- Raw content:
  ```text
  outcome_standard=solved
  outcome_taskspace=solved
  engineering_unclean=False
  infra_signatures=none
  taskspace nodes=3

  Select-String whale-exec.jsonl "parse deny-read ACL state|corrupt deny-read|deny_read_acl_state|execution error"
  no matches
  ```
- Interpretation: The original sandbox persistent-state parse failure no longer pollutes the focused live smoke. The suite cost gate still fails separately, so this closes only the sandbox reliability bug.
- Time: 2026-06-18 03:54
