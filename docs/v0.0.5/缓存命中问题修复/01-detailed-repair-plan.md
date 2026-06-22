# TaskSpace DeepSeek 缓存命中问题详细修复方案与实施计划

- Created: 2026-06-22
- Updated: 2026-06-22
- Version: v0.0.5 cache blocker detailed plan
- Status: Draft / Ready for engineering review
- Owner / Responsible: WhaleCode v0.0.5 runtime
- Related Systems: TaskSpace runtime, action-map runtime, session turn loop, tool runtime, DeepSeek official ChatCompletions provider, benchmark harness
- Related Links:
  - `docs/v0.0.5/缓存命中问题修复/README.md`
  - `coe/2026-06-22-15-24-taskspace-deepseek-cache-hit-rate.md`
  - `scripts/taskspace-benchmark/verify-deepseek-cache-fix.ps1`
  - `docs/v0.0.5/build-R2/00-overview-and-gates.md`
- Risk Level: Critical
- Plan Type: Full
- Task Classification: architecture migration + performance optimization + bug fix
- Recommended AI Agent reasoning level: xhigh

## 1. Problem Definition

### Current Behavior

TaskSpace on DeepSeek official API currently sends repeated provider requests through a native-tools ChatCompletions path. Each request can include dynamic TaskSpace projection, budget guidance, recovery content, node state, and history items before the large tools schema. DeepSeek cache reuse is prefix-sensitive, so stable tool/schema content after dynamic messages is not reliably reused.

Latest live validation showed:

| Mode | Requests | Input | Cached Input | Uncached Input | Hit Rate |
|---|---:|---:|---:|---:|---:|
| Standard | 1 | 94,353 | 75,264 | 19,089 | 0.797685 |
| TaskSpace | 8 | 127,528 | 15,104 | 112,424 | 0.118437 |

TaskSpace input was only about `1.35x` Standard, but uncached input was about `5.9x` Standard. The blocker is repeated cache miss, not only larger context.

### Expected Behavior

TaskSpace should keep a stable provider prefix across request 2+ and move all task-specific variability into a compact suffix. The hot path should not resend large native tools schemas after dynamic messages.

### Gap

The current runtime depends on provider-native tool calls for execution. That makes the DeepSeek provider request shape expensive in TaskSpace because native tools schema is part of each model request and is serialized after dynamic messages in ChatCompletions.

## 2. Goals

| Goal | Expected Benefit | Baseline | Target | Measurement |
|---|---|---:|---:|---|
| Stable steady-state provider prefix | DeepSeek cache hit becomes reliable after cold start | TaskSpace hit rate 0.118437 | requests 2+ hit >= 0.95 | provider usage fields |
| Lower uncached cost | User experiments become economically viable | TaskSpace uncached 112,424 vs Standard 19,089 | TaskSpace uncached <= 1.2x Standard | benchmark token summary |
| Preserve correctness | Cost fix does not degrade TaskSpace usefulness | latest TaskSpace side failed | no regression vs current correctness gates | public/hidden validators and benchmark audit |
| Improve observability | Future failures are diagnosable without manual payload scraping | partial aggregate evidence | >= 99% request cache trace coverage | cache trace artifact |
| Keep fallback | New transport can be disabled quickly | NativeTools path exists | feature-flagged fallback retained until release gate | config and side-by-side run |

## 3. Non-Goals

- Do not lower cache thresholds to make current behavior pass.
- Do not hide fixed natural-language responses in the CLI or runtime.
- Do not remove TaskSpace node semantics, evidence gates, or permission checks.
- Do not rely on DeepSeek caching native tools schema unless a live provider probe proves it.
- Do not remove NativeTools until the new path passes correctness and cost gates.

## 4. Constraints And Assumptions

| Assumption | Verification Method | If Assumption Fails |
|---|---|---|
| DeepSeek official no-tool prefix cache remains high | Run `verify-deepseek-cache-fix.ps1` official probe | Treat as provider regression or account/model issue before runtime work |
| Current native-tools path remains useful as fallback | Run existing benchmark in NativeTools mode | Keep fallback debug-only; do not use for cost-sensitive release |
| Runtime can execute model-emitted action envelopes through existing handlers | Implement L1 action executor prototype | Add minimal executor bridge rather than bypass handlers |
| Existing action-map node policy can reject disallowed actions | Unit tests per node kind | Add explicit action-envelope policy layer |
| Tool-free action contract can preserve model compliance | Live L1 DeepSeek smoke | Add parser recovery and examples; if still poor, evaluate split planner/executor |

## 5. Target Architecture

### 5.1 Transport Modes

Add an explicit TaskSpace provider transport mode:

```rust
enum TaskspaceProviderTransportMode {
    NativeTools,
    CacheOptimizedActionContract,
}
```

`NativeTools` is the current behavior. `CacheOptimizedActionContract` removes provider-native tools from the hot path and asks the model to output a structured action envelope.

### 5.2 Provider Request Shape

The new hot-path provider request should look like:

```text
Stable prefix:
  system/developer policy
  TaskSpace action contract
  action schema and examples
  node-kind rules
  compact error recovery rules

Dynamic suffix:
  active task summary
  active node id/kind
  relevant evidence refs
  compact recent results

Provider tools:
  none
```

Hard invariant:

```text
DeepSeek TaskSpace CacheOptimizedActionContract requests must omit native tools schema.
```

### 5.3 Action Envelope

Model output should be one parseable action per provider turn:

```json
{
  "schema_version": "taskspace-action-v1",
  "action": "read_file",
  "node_id": "node-123",
  "args": {
    "path": "src/example.rs"
  },
  "rationale": "Need direct evidence for the active inspect node."
}
```

Initial action set:

| Action | Runtime Mapping | Allowed Node Kinds | Notes |
|---|---|---|---|
| `list_files` | shell/rg files wrapper | inspect_code_context | deterministic command wrapper preferred |
| `search` | `rg` wrapper | inspect_code_context, smoke_test, regression_test | bounded output only |
| `read_file` | file read wrapper | inspect_code_context, smoke_test, regression_test | bounded output and output refs |
| `apply_patch` | existing apply_patch handler/runtime | implement_solution | must use existing patch verification |
| `run_test` | shell test wrapper | smoke_test, regression_test | rejected in implement nodes |
| `taskspace_control` | existing taskspace_control handler/runtime | route/finish/final allowed by policy | action-map remains source of truth |
| `final_answer` | final synthesis path | final_synthesis | subject to existing final gate |
| `blocked` | structured blocker | any node | must include exact missing evidence |

### 5.4 Runtime Processing Flow

```text
build TaskSpace prompt
  -> choose transport mode
  -> if CacheOptimizedActionContract:
       omit provider tools
       append action-contract instructions
       send model request
       parse assistant output as TaskSpaceActionV1
       validate action against active node and permissions
       dispatch through existing runtime/tool handlers
       record action result in action-map ledger
       emit cache trace
       project compact state for next request
  -> if NativeTools:
       use current provider-native tool path
```

No model-emitted action may execute until runtime validates:

- active TaskSpace mode;
- active node id and kind;
- action allowed for node kind;
- path/workspace safety;
- sandbox and approval policy;
- output bounds;
- budget state;
- required dependencies and evidence level.

## 6. Code Touchpoints

| Area | Current Path | Planned Change |
|---|---|---|
| Turn prompt construction | `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | add transport-mode selection and tool-free prompt build path |
| Provider prompt struct | `third_party/codex-cli/codex-rs/core/src/client_common.rs` | preserve ability to send no tools and trace transport metadata |
| Sampling request flow | `core/src/session/turn.rs`, `core/src/client.rs` | route assistant output to action parser before no-action recovery |
| Action-map state | `core/src/action_map/runtime.rs` | add action-contract result events and cache gate metadata |
| TaskSpace control | `core/src/tools/handlers/taskspace_control.rs` | reuse existing normalized argument and runtime execution |
| Apply patch | `core/src/tools/handlers/apply_patch.rs` | reuse parser, verification, approval, and ledger event |
| ChatCompletions bridge | `codex-api/src/endpoint/responses.rs` | ensure CacheOptimizedActionContract omits tools and preserves usage mapping |
| Benchmark harness | `scripts/taskspace-benchmark/` | add cache trace extraction and release gate checks |

## 7. Phase Gate Overview

| Phase | Name | Primary Output | Exit Gate |
|---:|---|---|---|
| 0 | Baseline lock | current failure fixture and gate lock | failing artifact blocks release |
| 1 | Cache observability | `TaskSpaceProviderCacheTraceV1` | >= 99% trace coverage |
| 2 | Action contract design | `TaskSpaceActionV1` schema and parser | invalid actions rejected |
| 3 | Tool-free L1 prototype | feature-flagged transport | L1 solves with no native tools schema |
| 4 | Full node workflow | inspect/implement/test/final coverage | E2 diagnostic no correctness regression |
| 5 | Cost gate integration | release-decision cache gates | current failing fixture rejected |
| 6 | Formal validation | live DeepSeek E2/E3 readiness | cache and correctness gates pass |
| 7 | Cleanup and default switch | fallback policy and docs | DeepSeek TaskSpace default is cost-safe |

## 8. Phased Execution Plan

### Phase 0: Baseline Lock And Release Gate Assertion

#### Objective

Make the current cache miss state impossible to ignore in v0.0.5 execution.

#### Entry Criteria

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Latest live failure is available | inspect verification report | `post-edit-drain-force/deepseek-cache-fix-verification.md` | runtime |
| COE captures root cause | read COE | H-006 accepted | runtime |
| Existing gate entry links blocker | inspect docs | build-R2 gate points to cache project | runtime |

#### Implementation Tasks

- Keep `README.md` as project-level blocker statement.
- Add this detailed plan.
- Add failing fixture paths to future release-decision fixture list.

#### Deliverables

- `docs/v0.0.5/缓存命中问题修复/01-detailed-repair-plan.md`
- Release gate reference in `build-R2/00-overview-and-gates.md`

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Documentation parses and diff is clean | `git diff --check` | no whitespace errors |
| Benefit | Future release claims blocked | doc review | gate states cache blocker explicitly |

#### Exit Criteria

- v0.0.5 docs identify cache miss as a formal blocker.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Documentation not read by implementers | blocker bypassed | E3 run ignores cache gates | link from build-R2 entry | add release script hard gate in Phase 5 |

#### Gate To Next Phase

Proceed when docs are committed and pushed.

### Phase 1: Provider Cache Trace And Request Shape Observability

#### Objective

Create first-class artifacts that explain why each provider request hit or missed cache.

#### Design Approach

Add a runtime/benchmark artifact named `TaskSpaceProviderCacheTraceV1`. It should be emitted for Standard and TaskSpace requests so comparisons are direct.

#### Implementation Tasks

- Add per-request trace data:
  - logical mode;
  - transport mode;
  - model request index;
  - request phase;
  - node kind;
  - provider wire API;
  - tools count and tools-present bool;
  - stable prefix hash;
  - dynamic suffix hash;
  - messages hash;
  - input tokens;
  - cached input tokens;
  - uncached input tokens;
  - hit rate.
- Add an exact request-shape classifier:
  - `native_tools_schema_hot_path`;
  - `tool_free_action_contract`;
  - `unknown_or_unclassified`.
- Extend benchmark extraction to surface cache trace coverage and failure taxonomy.

#### Deliverables

- Cache trace artifact in benchmark output.
- Request-shape summary in verification report.
- Fixture test for existing failing artifact.

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Cache trace schema | benchmark/runtime artifact module | provider request completion | fixture test | cache trace JSONL | none | planned |
| Request classifier | benchmark extraction script | verification script | failing fixture test | failure taxonomy | none | planned |
| Standard baseline trace | request summary path | benchmark pair run | fixture test | Standard cache trace rows | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Trace coverage | run L1 benchmark | >= 99% provider requests have trace |
| Correctness | Usage mapping | compare raw usage and trace | cached/miss tokens match provider fields |
| Benefit | Diagnosis quality | run on known failing fixture | failure says `native_tools_schema_hot_path` or `cache_prefix_unstable` |

#### Exit Criteria

- A failing run no longer requires manual `rollout.jsonl` inspection to identify cache miss shape.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Trace changes request shape | invalid measurement | hit rate changes after adding trace | never include trace in provider-visible prompt | emit trace outside model input |
| Token usage unavailable after failure | incomplete diagnosis | `usage_unavailable` | fallback to rollout trace and mark confidence | block release if trace coverage below gate |

#### Gate To Next Phase

Proceed when trace coverage is >= 99% on at least one Standard+TaskSpace L1 run.

### Phase 2: TaskSpaceActionV1 Contract And Validator

#### Objective

Define the provider-visible action contract that replaces native tools schema in the DeepSeek hot path.

#### Design Approach

The contract must be stable, compact, and strict enough for runtime validation. The provider sees instructions and examples, not API tool schemas.

#### Implementation Tasks

- Define `TaskSpaceActionV1` schema.
- Add parser for strict JSON envelope.
- Add recovery classification:
  - valid action;
  - malformed action;
  - unsupported action;
  - node-policy violation;
  - ambiguous natural language.
- Add validator that checks:
  - active node id;
  - action allowed for node kind;
  - workspace path safety;
  - budget state;
  - required fields;
  - no hidden tool bypass.
- Add unit tests for each action and rejection path.

#### Deliverables

- Schema document or Rust type for `TaskSpaceActionV1`.
- Parser and validator tests.
- Developer-facing action contract prompt section.

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Action schema | new schema/type near TaskSpace runtime | prompt build path | schema/parser tests | emitted schema version | none | planned |
| Parser | session/action-contract module | assistant output handling | malformed/valid tests | parse result event | none | planned |
| Validator | action-map/session policy layer | before executor dispatch | node-kind policy tests | rejection event | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Valid envelopes parse | unit tests | all supported actions parse |
| Correctness | Invalid envelopes reject | unit tests | no tool executes |
| Security | Node policy | unit tests | implement cannot test, validation cannot patch |

#### Exit Criteria

- No action can execute from model output without parser and validator approval.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Model emits prose instead of JSON | no progress | malformed action rate high | prompt examples and recovery item | fall back to NativeTools for debug only |
| Contract too broad | permission bypass risk | unsupported args reach executor | strict allowlist and validator | reject unknown fields/actions |

#### Gate To Next Phase

Proceed when parser/validator unit tests cover every initial action and rejection class.

### Phase 3: Tool-Free L1 Transport Prototype

#### Objective

Wire `CacheOptimizedActionContract` through one real TaskSpace L1 scenario.

#### Design Approach

This phase should solve `single-file-fast-fix` end to end before expanding scope. It must use no native provider tools schema.

#### Implementation Tasks

- Add `TaskspaceProviderTransportMode` config and default it off.
- In `build_prompt_with_tool_visibility` or adjacent prompt construction:
  - when mode is `CacheOptimizedActionContract`, pass an empty tools list;
  - append stable action-contract instructions;
  - keep dynamic node state compact and late.
- In sampling output handling:
  - parse assistant content as `TaskSpaceActionV1`;
  - dispatch through executor bridge;
  - record result as normal tool/action-map evidence;
  - suppress native no-action recovery until parser classification is recorded.
- Implement L1 actions:
  - `list_files`;
  - `read_file`;
  - `search`;
  - `apply_patch`;
  - `run_test`;
  - `taskspace_control`.

#### Deliverables

- Feature-flagged L1-capable transport.
- L1 live verification artifact.
- Cache trace proving no native tools schema.

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Transport flag | session/config path | TaskSpace run config | config tests | trace `transport_mode` | none | planned |
| Tool-free prompt | session turn builder | model request | request-shape test | tools count 0 | none | planned |
| Executor bridge | session/tool runtime path | assistant output | integration tests | action result events | none | planned |
| L1 benchmark | benchmark script | DeepSeek live run | live artifact | cache trace and validators | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | L1 public/hidden validators | live DeepSeek run | exit code 0 |
| Benefit | Steady-state cache hit | verification script | requests 2+ hit >= 0.95 |
| Benefit | Uncached input ratio | token summary | TaskSpace uncached <= 1.2x Standard |
| Regression | NativeTools unaffected | run targeted old-path tests | no regression |

#### Exit Criteria

- `single-file-fast-fix` solves with the new transport.
- Provider request trace shows tools count `0` for hot-path requests.
- Cache target passes for request 2+.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Action executor duplicates tool logic | drift and bugs | different behavior from native handler | dispatch through existing handlers | keep executor bridge thin |
| Live model fails action format | L1 blocked | repeated malformed actions | add one recovery prompt and examples | keep NativeTools debug fallback |

#### Gate To Next Phase

Proceed only after L1 live DeepSeek run passes correctness and cache gates.

### Phase 4: Full Node Workflow Coverage

#### Objective

Expand the action-contract transport from one smoke scenario to normal TaskSpace node flow.

#### Implementation Tasks

- Cover node kinds:
  - inspect_code_context;
  - implement_solution;
  - smoke_test;
  - regression_test;
  - final_synthesis.
- Add policy tests:
  - inspect can read/search but not edit;
  - implement can patch but not run tests;
  - validation can test/read but not patch;
  - final can synthesize but not mutate state unexpectedly.
- Support output refs and bounded reads so large outputs do not re-enter prompt.
- Add budget-recovery action envelopes.
- Add final answer envelope and final gate integration.

#### Deliverables

- Node-kind action policy table in code/tests.
- E2 diagnostic run using action-contract transport.
- Failure taxonomy for malformed or policy-rejected model actions.

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Node policy matrix | action validator | every action dispatch | matrix unit tests | rejection/accept events | none | planned |
| Output refs | action-map/runtime output refs | read/search results | output-ref tests | ref creation/read events | none | planned |
| Final synthesis | final gate path | final answer envelope | final gate tests | final accepted event | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | E2 diagnostic | benchmark harness | no solved-count regression vs current baseline |
| Benefit | Cache | cache trace | requests 2+ hit >= 0.95 |
| Regression | Node policy | unit and integration tests | no cross-node action bypass |

#### Exit Criteria

- E2 diagnostic passes without engineering-unclean failures.
- Cache remains within target.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| More node types increase malformed action rate | lower solve rate | malformed rate > threshold | add compact examples per node kind | retain NativeTools for non-release debug |
| Output refs too aggressive | missing evidence | validator cannot inspect needed output | allow bounded ref slices | fail gate if evidence missing |

#### Gate To Next Phase

Proceed when E2 diagnostic passes correctness and cache gates.

### Phase 5: Release Gate And Benchmark Integration

#### Objective

Make cache safety a hard v0.0.5 release gate.

#### Implementation Tasks

- Extend release-decision scripts to consume cache trace.
- Add gates:
  - `steady_state_provider_cache_hit_rate_for_requests_2_plus >= 0.95`;
  - `taskspace_uncached_input_tokens <= 1.2x standard_uncached_input_tokens`;
  - `cache_trace_coverage >= 0.99`;
  - `native_tools_schema_hot_path_count == 0` for DeepSeek release-like TaskSpace runs.
- Add fixture tests:
  - current failing artifact must fail;
  - synthetic/pass artifact must pass;
  - missing cache trace must fail release-like runs.

#### Deliverables

- Release gate implementation.
- Fixture artifacts and tests.
- Updated build-R2 closeout instructions.

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Gate fields | release-decision script | benchmark closeout | fixture tests | release report | none | planned |
| Failure taxonomy | benchmark analyzer | release report | failing fixture test | taxonomy fields | none | planned |
| Docs update | v0.0.5 docs | release read path | doc review | links in report | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Current failing fixture rejected | release script test | fail with cache taxonomy |
| Correctness | Passing fixture accepted | release script test | pass only when all gates met |
| Benefit | Gate prevents cost regression | run on live artifact | low hit rate cannot pass |

#### Exit Criteria

- No release-like DeepSeek TaskSpace run can pass without cache evidence.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Gate too strict for tiny samples | false negatives | cold-start dominates aggregate | use request 2+ steady-state hard gate | aggregate hit as warning |
| Gate too loose | cost regression ships | high uncached despite hit rate | include uncached ratio gate | block release |

#### Gate To Next Phase

Proceed when release gate fixtures pass and current failing artifact is rejected.

### Phase 6: Formal Live Validation

#### Objective

Prove the new transport is suitable for v0.0.5 experiments.

#### Implementation Tasks

- Run L1 smoke with DeepSeek official.
- Run E2 diagnostic matrix.
- Run E3 readiness only after existing v0.0.5 gates also pass.
- Archive:
  - cache traces;
  - token summaries;
  - pair reports;
  - release decision reports;
  - COE update.

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Business success | benchmark validators | no regression vs current expected TaskSpace gates |
| Benefit | Cache | provider usage fields | requests 2+ hit >= 0.95 |
| Benefit | Cost | token summary | uncached input <= 1.2x Standard |
| Regression | Native fallback | optional side run | fallback still works for debug |

#### Exit Criteria

- Live DeepSeek official validation passes correctness and cache gates.
- COE records final acceptance evidence.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Provider balance blocks validation | cannot close project | 402 Payment Required | use low-cost probe first, record blocker | do not claim completion |
| Provider behavior changes | unstable result | no-tool probe no longer hits | isolate provider regression | pause release gate changes |

#### Gate To Next Phase

Proceed when live validation artifacts pass.

### Phase 7: Default Switch And Cleanup

#### Objective

Make the cost-safe transport the DeepSeek TaskSpace default and retire unsafe release usage of NativeTools.

#### Implementation Tasks

- Default DeepSeek TaskSpace to `CacheOptimizedActionContract`.
- Keep `NativeTools` behind explicit debug config.
- Document when NativeTools may be used:
  - provider debugging;
  - regression comparison;
  - non-cost-sensitive local tests.
- Remove obsolete prompt-order workaround code only after replacement is stable.
- Update v0.0.5 closeout docs.

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | Default mode | config tests | DeepSeek TaskSpace selects action contract |
| Regression | Explicit NativeTools | config tests | debug opt-in still works |
| Benefit | Final cache gate | formal benchmark | gates pass |

#### Exit Criteria

- DeepSeek TaskSpace release path is cost-safe by default.
- NativeTools is not used for release-like DeepSeek TaskSpace evidence.

## 9. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Cache trace | Every provider request has cache metadata | benchmark/runtime trace code | TaskSpace and Standard benchmark runs | fixture and live tests | `TaskSpaceProviderCacheTraceV1` | none | planned |
| Transport mode | DeepSeek TaskSpace can choose action contract | `session/turn.rs`, config path | TaskSpace prompt build | config/unit tests | `transport_mode` trace field | none | planned |
| Tool-free prompt | Provider request omits native tools | prompt builder/client request | DeepSeek model request | request-shape tests | tools count 0 | none | planned |
| Action parser | Assistant output becomes one runtime action | parser module | sampling output handler | parser tests | parse result event | none | planned |
| Action validator | Runtime rejects disallowed actions | validator/action-map policy | before executor dispatch | node policy tests | rejection event | none | planned |
| Executor bridge | Valid actions use existing handlers | session/tool runtime bridge | action dispatch | integration tests | tool/action-map result events | none | planned |
| Benchmark gate | Low cache hit fails release | release-decision script | benchmark closeout | fixture tests | release report taxonomy | none | planned |
| Live validation | Benefit proven on DeepSeek official | benchmark scripts | L1/E2/E3 readiness | live artifacts | cache trace and token summary | none | planned |

## 10. Dependencies

| Dependency | Type | Current Status | Blocking Risk | Handling Plan |
|---|---|---|---|---|
| DeepSeek official API balance | third-party | Unknown per future run | live validation may block | run no-tool low-cost probe before full benchmark |
| Existing TaskSpace runtime | system | Available | node policy and result lifecycle may reject new action path | keep executor bridge through existing runtime functions |
| Benchmark harness | system | Available | may not expose cache trace fields yet | Phase 1 adds trace extraction |
| Release-decision scripts | system | Available | cache gate absent | Phase 5 integrates hard gates |
| Existing NativeTools path | system | Available | fallback could hide cost issue | mark release-like NativeTools DeepSeek runs diagnostic-only |

## 11. Testing And Validation Strategy

| Validation Type | Test Type | Scope | Execution Method | Passing Standard |
|---|---|---|---|---|
| Correctness | Unit | action parser | cargo test targeted parser tests | valid/invalid envelopes handled |
| Correctness | Unit | node policy matrix | cargo test targeted validator tests | no disallowed action executes |
| Correctness | Integration | executor bridge | targeted TaskSpace runtime tests | action result recorded in ledger |
| Correctness | Regression | NativeTools path | existing provider budget/session tests | no regression |
| Benefit | Cache | DeepSeek live L1 | `verify-deepseek-cache-fix.ps1 -RunTaskspaceBenchmark` | requests 2+ hit >= 0.95 |
| Benefit | Cost | benchmark token summary | Standard vs TaskSpace comparison | uncached input <= 1.2x Standard |
| Release | Gate fixtures | release-decision tests | current failing fixture and passing fixture | fail/pass as expected |

## 12. Benefit Validation

| Benefit Hypothesis | Metric | Baseline | Target | Measurement Method | Data Source | Observation Window | Pass / Fail Threshold |
|---|---:|---:|---:|---|---|---|---|
| Stable prefix restores DeepSeek cache | requests 2+ hit rate | about 0.118 aggregate TaskSpace | >= 0.95 steady-state | provider usage fields | cache trace | per run | fail below 0.95 |
| User cost is controlled | TaskSpace uncached ratio vs Standard | about 5.9x | <= 1.2x | token summary comparison | benchmark artifact | per comparable sample | fail above 1.2x |
| Total TaskSpace overhead remains bounded | direct input+output ratio | unknown for new path | <= 2.0x | existing v0.0.5 gates | benchmark artifact | per gate sample | fail above 2.0x |
| Correctness is preserved | business success | current baseline varies by sample | no regression | validators/oracle | pair report | per sample set | fail on regression |

## 13. Release, Rollback, And Fallback Strategy

### Release Strategy

- Release method: feature-flagged transport mode.
- Canary scope: L1 DeepSeek official single-file scenarios.
- Expansion criteria:
  - L1 correctness pass;
  - requests 2+ cache hit >= 0.95;
  - uncached input <= 1.2x Standard.
- Pause criteria:
  - malformed action rate prevents progress;
  - cache trace coverage < 99%;
  - native tools schema appears in release-like action-contract request.
- Owner: WhaleCode v0.0.5 runtime.
- Release window: Unknown.

### Rollback Strategy

- Rollbackable changes:
  - transport mode default;
  - release gate config;
  - action-contract prompt section.
- Non-directly rollbackable changes:
  - none expected if NativeTools remains.
- Rollback triggers:
  - correctness regression;
  - policy bypass;
  - live cost regression.
- Rollback steps:
  - switch DeepSeek TaskSpace transport back to `NativeTools`;
  - mark results diagnostic-only;
  - keep cache blocker open.
- Rollback validation:
  - NativeTools targeted regression tests pass;
  - release gate still blocks cost-sensitive DeepSeek TaskSpace claims.

### Fallback / Degradation Strategy

- Degradable capability: cache-optimized transport.
- Trigger: action-contract path fails correctness.
- User-visible impact: TaskSpace DeepSeek experiments remain blocked for cost-sensitive usage.
- System behavior while degraded: NativeTools may run only as debug or diagnostic mode.
- Recovery steps: inspect action trace, fix parser/validator/executor, rerun L1.

## 14. Security And Permission Review

This is not a security feature, but it changes how model output reaches tools. It must preserve or strengthen tool permission boundaries.

Required checks:

- Model output cannot directly execute shell or patch without runtime validation.
- `apply_patch` still uses existing patch parser, verification, and approval path.
- File paths stay within workspace/sandbox rules.
- Node-kind policy blocks edits in inspect/test/final nodes.
- Test commands run only in validation nodes.
- Unknown actions and unknown args are rejected.
- Rejection events are recorded for audit.

## 15. Observability And Success Metrics

| Metric | Current Baseline | Target | Alert Threshold | Observation Window |
|---|---:|---:|---:|---|
| request 2+ cache hit rate | Unknown in current trace | >= 0.95 | < 0.95 | each live run |
| aggregate TaskSpace hit rate | 0.118437 latest | warning >= 0.85 for larger samples | < 0.85 | E2+ sample |
| TaskSpace uncached ratio vs Standard | about 5.9x | <= 1.2x | > 1.2x | comparable sample |
| cache trace coverage | partial/manual | >= 99% | < 99% | every benchmark |
| native tools schema in action-contract mode | present in old path | 0 | > 0 | every release-like run |
| malformed action rate | Unknown | <= 10% after warmup | > 20% | diagnostic run |

## 16. Open Questions

| Question | Blocking? | Handling |
|---|---|---|
| Should the hard cache gate use aggregate hit or request 2+ steady-state hit? | Yes for release criteria | Use request 2+ as hard gate and aggregate as warning until sample size is larger |
| Should the action envelope be strict JSON only or allow fenced JSON recovery? | No for Phase 1, yes for parser design | Start strict; allow one recovery parser only if live model compliance blocks L1 |
| Can DeepSeek provide a better tool-caching path than ChatCompletions native tools? | No for first fix | Discovery only; do not block action-contract path |
| Should Standard mode also adopt cache trace? | Yes | Include in Phase 1 so TaskSpace has a comparable baseline |
| What is the default transport for non-DeepSeek providers? | No | Keep NativeTools default outside DeepSeek until separately validated |

## 17. Change Log

| Date | Change | Reason |
|---|---|---|
| 2026-06-22 | Created detailed repair plan | Formalize v0.0.5 cache blocker execution path |

## 18. Plan Quality Checklist

- [x] Problem definition distinguishes current behavior, expected behavior, and gap.
- [x] Goals are measurable and include benefit validation.
- [x] Non-goals prevent threshold gaming and model-path bypasses.
- [x] Architecture migration is phased and reversible.
- [x] High-risk unknowns are moved to early observability and L1 prototype phases.
- [x] Each phase has entry criteria, tasks, validation, exit criteria, risks, and next gate.
- [x] Implementation completeness matrix distinguishes planned production paths from test-only scaffolding.
- [x] Release, rollback, fallback, and observability are explicit.
- [x] Security and permission boundaries are included because model output will drive local tools.
