# Codex CLI Upstream Substrate Migration Plan

Date: 2026-04-27

## Status

Accepted direction. This plan replaces the previous from-scratch V1 runtime
plan. The existing Rust demo runtime has been archived at
`archive/deprecated/2026-04-27-rust-demo/` and must not receive new feature
work.

## Goal

Rebase WhaleCode onto a whole-repo Codex CLI upstream substrate, then adapt it
through Whale-owned bridge and overlay layers for DeepSeek V4, multi-agent
coordination, Primitive Modules, Viewer, and Create/Debug workflows.

The objective is not to vaguely reference Codex. The objective is to keep Codex
implementation detail available in the repository, compile or test the relevant
parts, and preserve a future path for absorbing new Codex releases.

## Architecture Decision

```text
WhaleCode repo
  ├─ third_party/codex-cli/          # pinned Codex CLI upstream snapshot
  ├─ patches/codex-cli/              # Whale patch queue for unavoidable upstream edits
  ├─ crates/whalecode-codex-bridge/  # adapters from Codex runtime to Whale protocol
  ├─ crates/whalecode-*              # Whale protocol, DeepSeek, swarm, primitive overlay
  ├─ apps/viewer/                    # read-only Web Viewer later
  ├─ docs/migration/codex-sync/      # one log per upstream sync
  └─ archive/deprecated/             # inactive historical implementations
```

Codex should remain a pinned upstream substrate. Whale-specific behavior should
enter through bridge or overlay crates first. Direct vendor edits are allowed
only when the bridge cannot express the needed integration.

## Principles

| Principle | Requirement |
| --- | --- |
| Preserve upstream detail | Import Codex CLI whole repo rather than manually recreating selected behavior. |
| Keep upstream syncable | Keep `third_party/codex-cli/` pristine where possible and record every local patch. |
| Bridge before fork | Whale code depends on stable Whale interfaces; Codex types stay behind the bridge. |
| Product boundary stays Whale | DeepSeek, multi-agent, Primitive, Viewer, Create/Debug remain Whale-owned overlay behavior. |
| Replace demo, not build on it | The archived Rust demo is historical evidence only. It is not the base for new runtime work. |
| Logs and tests first | Every migrated subsystem needs event logging, sync notes, and regression tests before it is considered adopted. |

## Layer Boundaries

### Codex Upstream Layer

`third_party/codex-cli/` owns mature single-agent coding-agent substrate:

- permission and approval policy;
- filesystem and process sandboxing;
- shell command parsing and dangerous-command policy;
- unified exec and long-running command lifecycle;
- apply patch and file edit safety;
- session, thread, and rollout trace behavior;
- context compaction and history replacement;
- MCP, skills, plugins, TUI interaction details.

### Whale Bridge Layer

`crates/whalecode-codex-bridge/` translates between Codex internals and Whale
protocol:

- Codex permission decision -> Whale `PermissionEvent`;
- Codex exec invocation -> Whale `ToolEvent`;
- Codex patch application -> Whale `PatchArtifact`;
- Codex thread/session events -> Whale `SessionEvent`;
- Codex context fragments -> Whale `ContextPack`;
- Codex MCP/skills surfaces -> Whale `PrimitiveModule` host.

Whale runtime crates should not import Codex internals directly.

### Whale Overlay Layer

Whale-owned code adds behavior Codex does not provide as product primitives:

- DeepSeek V4 Flash/Pro routing and capability probes;
- `reasoning_content` streaming and preservation;
- multi-agent Supervisor, cohorts, WorkUnits, and Patch League;
- Evidence-chain Debug;
- Scaffolding-first Create;
- reference-driven decisions;
- independent Viewer concerns;
- skill evolution telemetry;
- Web Viewer event stream.

## Migration Phases

### Phase 0: Freeze And Archive Demo

Deliverables:

- Move the previous Rust demo workspace into
  `archive/deprecated/2026-04-27-rust-demo/`.
- Update README, ADRs, planning docs, and testing docs so they no longer
  describe the archived demo as active product runtime.
- Keep all deprecated files in git for recoverability.

Acceptance:

- No active root `Cargo.toml` claims a from-scratch Whale runtime.
- Documentation clearly says Codex substrate migration is the active path.

### Phase 1: Import Codex Upstream

Deliverables:

- Import Codex CLI whole repo into `third_party/codex-cli/`.
- Add `third_party/codex-cli/UPSTREAM.md` with upstream URL, commit, import
  method, import date, license, and local patch count.
- Add `docs/migration/codex-sync/2026-04-27-initial-import.md`.
- Add a top-level note that upstream vendor files may exceed Whale's normal
  500-line file limit because they are preserved third-party source.

Acceptance:

- Codex import is reproducible from the recorded upstream commit.
- License and attribution are documented.
- No Whale product changes are mixed into the initial import.

### Phase 2: Codex Inventory And Adoption Matrix

Deliverables:

- Create `docs/migration/codex-inventory.md`.
- Classify Codex crates/modules as:
  - `adopt_as_is`;
  - `adapt_in_bridge`;
  - `keep_disabled`;
  - `reject_for_whale`;
  - `needs_whale_redesign`.
- At minimum classify permission, sandbox, apply patch, exec, session, context,
  MCP, skills, TUI, login/auth, backend/cloud, and model provider modules.

Acceptance:

- Each adopted area names the Codex path and Whale bridge surface.
- Each rejected or disabled area records the product reason.

### Phase 3: Bridge Skeleton

Deliverables:

- Create `crates/whalecode-codex-bridge/`.
- Define stable Whale-side traits for permission, patch, exec, session, context,
  and MCP/skills adapters.
- Keep Codex concrete types private to the bridge crate.
- Add trace events for bridge calls before any complex behavior is migrated.

Acceptance:

- Whale overlay compiles against bridge traits, not Codex internals.
- Bridge smoke tests can instantiate at least one Codex-backed adapter or a
  fixture implementation.

### Phase 4: Replace Archived Demo Subsystems

Recommended order:

1. Apply patch and file edit safety.
2. Unified exec, shell command parsing, and sandbox.
3. Permission and approval policy.
4. Read/search/file discovery.
5. Session, thread, and replay.
6. Context compaction/history replacement.
7. MCP, skills, and plugin surfaces.
8. TUI/CLI runtime wiring.

Acceptance for each subsystem:

- Codex behavior is covered by Whale regression tests.
- Whale events are emitted for every relevant action.
- The bridge exposes a stable Whale interface.
- Any upstream patch is recorded in `patches/codex-cli/` and the sync log.

### Phase 5: DeepSeek Provider Integration

Deliverables:

- Add DeepSeek provider adapter compatible with the Codex-derived agent loop.
- Preserve DeepSeek `reasoning_content` across streaming and tool-call
  continuation.
- Add model capability probe for model name, context, output, thinking,
  tool-call compatibility, cache usage, pricing metadata, and 429 behavior.
- Keep OpenAI/Codex product login paths disabled unless explicitly supported.

Acceptance:

- Provider smoke test verifies auth failure, SSE parse, text delta, reasoning
  delta, tool-call delta, and usage aggregation.
- No natural-language fallback or local fixed answer bypasses the model path.

### Phase 6: Primitive And Multi-Agent Overlay

Deliverables:

- Reintroduce Whale protocol crates as overlay, not from-scratch substrate.
- Add `WorkflowPhase`, `AgentRole`, `WorkUnitId`, `PatchOwnership`, and
  `PrimitiveModule` hooks around the Codex substrate.
- Implement Create/Debug gates as deterministic overlay constraints.
- Add Viewer concern events and replay reducers.

Acceptance:

- Disabling Whale primitives still leaves a usable Codex-derived single-agent
  CLI.
- Enabling Whale primitives adds gates, artifacts, and events without forking
  the lower-level substrate.

### Phase 7: Upstream Sync Loop

Every Codex upgrade must follow this workflow:

1. Fetch upstream Codex commit or tag.
2. Generate diff from previous pin.
3. Classify changed areas by subsystem.
4. Update `third_party/codex-cli/`.
5. Reapply `patches/codex-cli/`.
6. Fix `whalecode-codex-bridge/` only where Codex internals changed.
7. Run substrate, bridge, provider, and primitive regression tests.
8. Write `docs/migration/codex-sync/YYYY-MM-DD-<codex-sha>.md`.

The sync log must record:

- previous and new Codex commit;
- adopted upstream features;
- disabled or rejected upstream features;
- patch conflicts;
- bridge changes;
- test results;
- residual risks.

## Patch Policy

Direct changes inside `third_party/codex-cli/` are last resort. If needed:

- create a patch file under `patches/codex-cli/`;
- explain why bridge-only integration was insufficient;
- mark whether the patch should be upstreamed to Codex;
- add a focused regression test;
- mention the patch in the next sync log.

## Testing Strategy

Testing shifts from the archived demo script to migration gates:

| Gate | Purpose |
| --- | --- |
| Documentation gate | New docs must consistently describe Codex substrate direction. |
| Import gate | Codex upstream snapshot, license, and commit metadata are reproducible. |
| Bridge unit tests | Whale interfaces remain stable while Codex internals evolve. |
| Substrate behavior tests | Permission, sandbox, patch, exec, session, context, MCP, and skills preserve Codex-grade behavior. |
| Provider tests | DeepSeek adapter preserves V4 streaming, reasoning, tool calls, and usage metadata. |
| Overlay tests | Primitive, Viewer, Create/Debug, and multi-agent gates replay correctly. |
| Sync tests | Replaying Whale patch queue after an upstream update is deterministic. |

## Acceptance Line

The migration is successful when:

- `whale` runs on a Codex-derived substrate rather than the archived demo.
- DeepSeek is the default provider path.
- Codex permission, sandbox, patch, exec, session, context, MCP, and skills
  behavior are available through Whale bridge interfaces.
- Whale Primitive and multi-agent layers can be enabled without modifying
  Codex internals for each feature.
- Upstream Codex can be advanced by updating the pin, replaying patches, and
  adjusting bridge code with documented risk.

