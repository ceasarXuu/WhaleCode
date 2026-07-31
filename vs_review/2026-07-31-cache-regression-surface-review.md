# Subagent VS Review: 缓存回归门禁命中范围

- Created: 2026-07-31T19:19:19+08:00
- Updated: 2026-07-31T19:24:27+08:00
- Report schema: adversarial-v1
- Task: 审查缓存回归门禁是否漏掉有影响的变更，或因范围过宽频繁误报
- Report path: `vs_review/2026-07-31-cache-regression-surface-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: open

## Round 1: 命中范围与失效模式

### Review Input

#### Objective

验证门禁是否以合理工程成本覆盖所有可能改变 DeepSeek provider 稳定前缀或缓存 usage 观测的生产代码入口，同时
避免测试、纯重构、非请求路径和与缓存无关的 TaskSpace 代码频繁触发付费复验。

#### Review Target

缓存敏感面合同、指纹实现、pre-commit/release 集成及其测试。

#### Target Locations

- `benchmarks/cache-regression/cache-surface-contract.json`
- `scripts/cache-regression/cache_surface.py`
- `scripts/cache-regression/check_cache_regression_gate.py`
- `scripts/cache-regression/test_cache_regression_gate.py`
- `.githooks/pre-commit`
- `scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1`
- `third_party/codex-cli/codex-rs/core/src/` 中最终 provider payload、上下文、提示词和 Tool schema 构造入口

#### Change Introduction

当前实现用 18 个 glob 对缓存敏感源码做路径加内容 SHA-256 指纹。pre-commit 对 staged 指纹变化阻断；non-agent
release gate 还要求最近一次 live 基线通过。获批后的 live runner 固定执行 Standard 与 map-request 各一次。

#### Risk Focus

- 新的请求构造入口位于 glob 外，产生漏报。
- 宽目录 glob 包含测试、telemetry 或只读逻辑，产生高频误报。
- 删除、重命名、新增文件或未跟踪文件逃逸指纹。
- 规则本身变化或基线状态修改能够绕过验证。
- pre-commit 与 release gate 观察不同源码快照，产生不一致。

#### User-Perspective Review Focus

- 开发者能否从阻断信息准确知道哪个变更触发、为何需要付费复验、如何恢复工作。
- 无关改动是否会被迫申请 2 个 sample 预算。

#### Implementation Completeness Focus

- 实际 provider payload 构造链中的生产文件是否都被覆盖。
- 新增入口、规则漂移和误报边界是否有自动测试。
- tracked hook 与共享 non-agent gate 是否使用同一合同。

#### Target Benefit Focus

- 目标是低成本立即发现潜在缓存退化；验证证据是确定性门禁 fixture 与首次真实两臂运行。
- 重点检查覆盖率收益是否以过多误报、付费运行和开发阻塞为代价。

#### Assumptions To Attack

- 文件路径可以稳定代表最终 payload 影响面。
- 当前 18 个 glob 覆盖全部生产入口。
- 目录级匹配不会把大量非语义代码纳入。
- 指纹相同足以说明无需复验。
- 开发者不能仅修改合同或基线绕过门禁。

#### Adversarial Lenses

- implementation-completeness
- testing
- maintenance
- failure
- usability
- observability

#### Verification Status

- Python fixture：13 tests passed。
- PowerShell non-agent builder 与 ledger tests passed。
- 真实 2-sample 验证成功区分 Standard 96.62% 与 map-request 35.79%。
- 尚未完成从 provider payload 调用链反向枚举的覆盖审计。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files directly.
- Do not modify files.
- Try to produce concrete false-negative and false-positive examples.
- Cite evidence paths and line numbers when possible.

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| complex | 15 minutes | at most 10 minutes | 2 | accepted blocking findings require fresh re-review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-completeness-adversary | 最高风险是路径合同未覆盖真实生产入口或只覆盖脚手架 | 漏报、误报、生产接入完整性 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-completeness-adversary | `multi_agent_v1.spawn_agent` | `019fb7e7-2280-72e0-a680-291d94f9c6e2` | spawn tool call and completion notification | `fork_context=false` | Round 1 Review Input | main-agent history, reasoning, drafts and conclusions | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| IC-1 | implementation-completeness-adversary | 1 | `019fb7e7-2280-72e0-a680-291d94f9c6e2` | 54 s | completed | n/a | completed |

### Reviewer Outputs

#### IC-1

##### Summary

Reviewer found seven blocking coverage/integrity failures and one proven false-positive class.
The existing five fixture tests prove synthetic hash mechanics only; they do not prove the real production dependency surface.

##### Blocking Findings

1. **Critical: `--require-live-baseline` accepts `structural_bootstrap`.**
   - Broken assumption: release gate requires a real provider baseline.
   - Failure scenario: an unverified bootstrap hash passes release because both statuses are accepted.
   - Evidence: `scripts/cache-regression/check_cache_regression_gate.py:40-43`.
   - Proof needed: bootstrap rejection fixture.

2. **Critical: contract and enforcement files can self-authorize a change.**
   - Broken assumption: index validation evaluates the exact contract being committed.
   - Failure scenario: partial-stage a contract with empty rules while leaving the old contract in the worktree; hook reads the old
     contract and returns PASS. A fabricated `live_verified` hash can also be staged directly.
   - Evidence: `scripts/cache-regression/cache_surface.py:24`,
     `scripts/cache-regression/check_cache_regression_gate.py:35`, and main-thread executable reproduction.
   - Proof needed: staged/worktree divergence and promotion-integrity fixtures.

3. **Critical: actual DeepSeek wire serializer, Tool serializer and usage decoder are omitted.**
   - Broken assumption: `core/src/client*.rs` is the complete provider boundary.
   - Failure scenario: role conversion, message serialization, Tool JSON or `cached_tokens` parsing changes without a hash change.
   - Evidence: `codex-api/src/endpoint/chat_completions.rs:7`, `codex-api/src/endpoint/responses.rs:208`,
     `tools/src/tool_spec.rs:160`, `codex-api/src/sse/chat_completions.rs:258`.
   - Proof needed: real-contract coverage test and deterministic final-wire fixtures.

4. **Critical: primary context and Tool-selection constructors are omitted.**
   - Broken assumption: `context/**` and `turn_context.rs` cover context construction.
   - Failure scenario: reorder developer sections, move projection, change visible Tools or `tool_choice`; gate remains green.
   - Evidence: `core/src/session/mod.rs:2876`, `core/src/session/mod.rs:3337`, `core/src/session/turn.rs:1072`,
     `core/src/tools/router.rs:69`.
   - Proof needed: dependency coverage test and final-payload golden tests.

5. **High: model/provider routing and request-affecting model metadata are omitted.**
   - Broken assumption: cache identity depends only on prompt/context code.
   - Failure scenario: DeepSeek wire API, default model, parallel Tool capability or compaction behavior changes silently.
   - Evidence: `model-provider-info/src/lib.rs:360`, `core/src/config/mod.rs:1028`,
     `models-manager/models.json:4`.
   - Proof needed: provider/model identity in the deterministic request contract.

6. **High: one paid sample cannot validate the breadth represented by `live_verified`.**
   - Broken assumption: `single-file-fast-fix` exercises every matched conditional path.
   - Failure scenario: changes to Pro, compaction, MCP, Apps, permissions, model switching or long history pass an unrelated sample.
   - Evidence: `benchmarks/cache-regression/cache-surface-contract.json:60-76` and
     `scripts/cache-regression/run_cache_hit_regression.py:299`.
   - Proof needed: free final-payload scenario matrix and affected-subsystem validation routing.

7. **Medium: release source identity and untracked handling are inconsistent.**
   - Broken assumption: evidence describes one immutable source snapshot.
   - Failure scenario: release records HEAD while checking a dirty worktree; relevant untracked files are absent from enumeration.
   - Evidence: `scripts/cache-regression/cache_surface.py:44-45` and
     `scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1:21,65`.
   - Proof needed: clean-tree/exact-commit release fixture.

##### Non-blocking Risks

1. **Medium: broad globs produce proven paid-test false positives.**
   - Broken assumption: every byte change in a matched file can alter provider requests.
   - Failure scenario: editing only `*_tests.rs`, inline `cfg(test)` code, comments or formatting changes the hash and requests paid
     validation.
   - Evidence: 10 of the current 77 matched files are explicit test files; the digest hashes raw file bytes at
     `scripts/cache-regression/cache_surface.py:68-80`.
   - Proof needed: exclusions and semantic final-payload comparison.

##### User-Perspective Checks

- Usability: **fail** - unrelated test/comment changes can require paid validation.
- Ease of use: **risk** - blocked output cannot distinguish source-risk warning from proven wire change.
- Ease of understanding: **risk** - `live_verified` overstates what the fixed sample exercised.

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| Source-risk discovery | All request-affecting changes detected | Multiple missing modules | pre-commit | Synthetic fixture only | none | none | partial | IC-1.3-5 |
| Paid live validation | Validates affected behavior | Fixed two-arm runner | explicit command | one sample | provider trace | none | partial | IC-1.6 |
| Contract integrity | Cannot self-authorize | worktree contract trusted | hook | missing | none | none | partial | IC-1.1-2 |
| False-positive control | No paid run for semantic no-op | raw byte hash | hook | missing | none | none | partial | IC-1.N1 |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| Immediately detect cache risk | 77-file manual glob | Complete request dependency coverage | source hash | concrete missed paths | regressed | false negatives | weak-evidence | IC-1.3-5 |
| Avoid unnecessary API cost | no prior gate | paid run only for meaningful changes | raw byte hash | 10 test files plus comments included | regressed | false positives | regressed | IC-1.N1 |

##### Required Fixes

- Separate broad, free source-risk detection from semantic final-provider-payload change detection.
- Protect the contract, checker, hook, promotion path and result evidence as a control plane.
- Make release accept only a validated `live_verified` result.
- Build deterministic final-wire snapshots for Standard/TaskSpace and important conditional paths.
- Route paid scenarios from the changed semantic surface instead of always using one sample.

##### Missing Tests

- Real contract coverage anchors for session, tools, codex-api, provider routing and usage decoding.
- Bootstrap release rejection.
- Contract partial-staging/tampering and fabricated promotion rejection.
- Test/comment-only change does not request paid validation.
- Exact-commit release and relevant untracked-file rejection.

##### Missing Logs / Observability

- Gate result does not identify which deterministic provider-payload scenario changed.
- Release evidence does not currently prove that checked source equals recorded HEAD.

##### Evidence

- `benchmarks/cache-regression/cache-surface-contract.json:10-58` - incomplete path contract.
- `scripts/cache-regression/cache_surface.py:39-80` - tracked raw-byte hashing.
- `scripts/cache-regression/check_cache_regression_gate.py:35-43` - trusted contract/status logic.
- Main-thread reproduction: gate returned 0 while staged contract had `surface_rules=[]`.

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| IC-1 | Bootstrap accepted as live | release can pass without live evidence | blocking | accept | direct code condition | documented; no code change in review-only turn | fix first |
| IC-1 | Contract/control-plane self-authorization | committed contract differs from checked contract | blocking | accept | executable partial-stage reproduction | documented | redesign evidence authority |
| IC-1 | Wire/Tool/usage modules omitted | real payload changes do not alter hash | blocking | accept | direct call-chain evidence | documented | add dependency and payload contracts |
| IC-1 | Context/Tool constructors omitted | ordering and visibility changes escape | blocking | accept | direct call-chain evidence | documented | add free final-payload matrix |
| IC-1 | Provider/model metadata omitted | wire identity changes escape | blocking | accept | direct source evidence | documented | include provider identity scenarios |
| IC-1 | Fixed paid sample overclaims coverage | unexecuted paths can be promoted | blocking | accept | runner and contract are hard-coded | documented | scenario-based validation routing |
| IC-1 | Release source identity mismatch | dirty worktree checked as HEAD evidence | blocking | accept | source enumeration and builder differ | documented | exact-commit release gate |
| IC-1 | Test/comment false positives | raw bytes force paid run for semantic no-op | non-blocking | accept | 10/77 explicit test files plus raw digest | documented | exclusions plus semantic payload gate |

No finding was rejected. Code was intentionally not changed because the accepted corrections alter the gate architecture and paid-run
trigger semantics; project constraints require user agreement before that technical-route decision.

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no; required after fixes
- Blocking re-review passed: no
- Blocking re-review round links: n/a until implementation
- Blocking re-review launch records: n/a until implementation
- Rejected findings backed by evidence: n/a
- Deferred findings documented: no
- Implementation completeness gaps resolved or accepted by user: no
- Target benefit warnings recorded: yes
- Blocked reason: current source-hash gate has both material false negatives and paid-run false positives
- Allowed to proceed: no

## Final Conclusion

当前 18-glob/77-file 门禁范围不合理，不能作为可靠的缓存变更门禁继续扩展。应保留已建成的 provider usage runner
和账本能力，但把触发链重构为：宽范围源码风险哨兵 -> 免费确定性最终 payload 场景矩阵 -> 仅在 payload 或缓存
测量合同真实变化时申请对应的付费回归。完成修复并由新的 fresh reviewer 复审前，不应把 `live_verified` 解释为
整个缓存敏感面的验证结论。
