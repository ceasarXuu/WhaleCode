# Problem P-001: Codex 0.151 发布收口回归

- Status: resolved
- Created: 2026-09-01 06:00
- Updated: 2026-09-02 07:20
- Objective: 在不改变 DeepSeek 默认行为和 TaskSpace 状态权威的前提下，为 Codex 0.151 当前 vendor 建立可审计的隔离回归与 W6 发布资格证据。
- Symptoms:
  - 完整 `codex-core` 隔离回归曾记录 7 个失败与 1 个超时，但未持久化最终原始日志和逐项延期映射。
  - W6 metadata validator 在当前 HEAD 报 replay overlay tree 与 inventory 过期。
  - 已批准的真实 cache 双臂运行在有效请求 pair 前失败，usage unavailable，accepted baseline 未晋升。
- Expected behavior:
  - 每个残余非绿项均可复现并绑定精确基线、生产影响与用户批准的延期权威；W6 静态工件可由 HEAD 复验；真实 cache 资格只在新预算授权后执行。
- Actual behavior:
  - 残余测试只有四个问题簇摘要；W6 closeout 工件仍在 `stash@{0}`；当前 metadata validator exit 1。
- Impact:
  - 无法诚实宣布 0.151 追赶完成或合入 main。
- Reproduction:
  - `python3 scripts/codex-upstream/run_isolated_tests.py -p codex-core`
  - `python3 scripts/codex-upstream/validate_sync_metadata.py`
- Environment:
  - Linux；分支 `whalecode-codex`；vendor `rust-v0.151.0`；final-wire subject `b631eb7e67`。
- Known facts:
  - 对抗性审查 B1/B2 已由主 Agent 接受为 blocker。
  - 既有 cache record `WAR-20260901-073444-CACHE-REGRESSION-3BF5A4B3` 为 partial/failed，usage unavailable。
- Ruled out:
  - none
- Fix criteria:
  - 持久化最终隔离结果并逐项证明残余非绿项属于已批准延期，或修复不能证明者。
  - W6 metadata/generator/cache gate 可复验；真实 cache qualification 成功并结算后才晋升 baseline。
  - focused closure review 关闭 B1/B2。
- Current conclusion: B2 已由精确 7+1 JUnit、pristine 对照和逐项授权映射关闭；B1 的静态 metadata/final-wire 与真实 cache 双臂资格均通过，`e39d5bd4…` 已晋升 accepted baseline。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - E-003 至 E-006 关闭逐项延期和静态资格缺口；E-007 证明持久双臂真实资格、账本结算与 live baseline 晋升完成。
- Close reason:
  - Fix criteria satisfied；不修改已批准延期的 TaskSpace 产品路径。

## Hypothesis H-001: 残余 7+1 均可从隔离运行恢复并逐项绑定既有延期

- Status: refuted
- Parent: P-001
- Claim: 当前 vendor 的残余失败均稳定落在 Cyber/TaskSpace/W9 会话继承、turn state 或 extension request 编排问题簇，没有额外 0.151 overlay 回归。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 既有 closeout 摘要列出四类失败，但缺少原始 test-level 证据。
- Falsifiable predictions:
  - If true: 新隔离回归的每个 failure/timeout 都能与既有问题簇、0.151 pristine/baseline 或明确用户延期逐项对应。
  - If false: 出现不属于这些簇的新失败，或某项在 pristine/baseline 中通过且由 Whale overlay 引入、又没有延期权威。
- Diagnostic evidence plan:
  - Prediction or clause under test: 每个残余非绿项都有精确 test name、signature、baseline comparison 和 authority mapping。
  - Signal: isolated nextest final status、JUnit、相关 test/code path 与 0.151 pristine qualification log。
  - Capture method: 运行当前 vendor 的完整 `codex-core` 隔离回归并交叉核对持久化基线日志和 Git 差分。
  - Event name or marker:
    - nextest test case name
  - Correlation keys:
    - HEAD commit
    - upstream commit
  - Differentiates from:
    - H-002
  - Supports if:
    - 所有非绿项均形成无遗漏的证据表，且没有未授权的当前目标回归。
  - Refutes if:
    - 任一非绿项无法映射或证明为新 overlay regression。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
- Conclusion: refuted；7+1 中除 pristine 已失败的 recover 外，其余项在 pristine 0.151 通过，因此不能描述为“没有额外 overlay regression”。
- Repair design readiness: closed；由 H-003 取代
- Next step: 由 H-003 建立准确延期边界。
- Blocker:
  - none
- Close reason:
  - 被 H-003 的更精确机制取代

## Hypothesis H-002: 至少一个残余非绿项是未批准的当前目标回归

- Status: refuted
- Parent: P-001
- Claim: 现有四簇摘要把至少一个 Whale 0.151 overlay 回归错误归入 TaskSpace/W9 延期。
- Layer: regression-window
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 相关测试覆盖 Cyber、pending input、websocket 和 skills extension 生产 lifecycle，概念相近不能代替基线证据。
- Falsifiable predictions:
  - If true: 至少一项当前失败在官方 0.151/pristine 或 0.149 Whale 基线通过，且变更路径属于本轮 overlay、无既有延期权威。
  - If false: 所有当前非绿项在基线已存在或被精确用户授权延期。
- Diagnostic evidence plan:
  - Prediction or clause under test: 寻找失败签名在基线与当前实现之间的差异。
  - Signal: pristine 0.151 log、0.149 log、当前 JUnit、相关文件 Git history。
  - Capture method: 按 test name 查询三组证据并核对变更归属。
  - Event name or marker:
    - nextest test case name
  - Correlation keys:
    - baseline commit
    - subject commit
  - Differentiates from:
    - H-001
  - Supports if:
    - 找到至少一个未授权的新回归。
  - Refutes if:
    - 所有非绿项均有基线同签名或精确延期权威。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
- Conclusion: refuted；这些项确有当前集成回归成分，但用户已明确批准留到 TaskSpace 专项分支，故“未批准”不成立。
- Repair design readiness: closed；由 H-003 取代
- Next step: 由 H-003 逐项记录延期权威，不在本轮修复产品逻辑。
- Blocker:
  - none
- Close reason:
  - 被 H-003 的更精确机制取代

## Hypothesis H-003: 精确的 7+1 是已批准延期的当前集成回归集合

- Status: confirmed
- Parent: P-001
- Claim: 7 个失败与 1 个超时是当前 Whale overlay 相对 pristine 0.151 的真实生命周期回归集合，但精确落入用户已批准留到 TaskSpace 专项分支处理的 Cyber/previous-turn/extension/turn-state 四簇。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-001
  - H-002
- Rationale:
  - 需要同时保留“它们是真回归”和“它们已获延期授权”两个事实，不能用任一事实覆盖另一个。
- Falsifiable predictions:
  - If true: 8 项在隔离定向运行中精确复现为 7 failed + 1 timeout；每项均属于四簇之一；pristine 对照与用户延期边界可定位。
  - If false: 运行出现额外测试、某项不属于四簇，或缺少既有延期权威。
- Diagnostic evidence plan:
  - Prediction or clause under test: 精确集合、签名、pristine 对照和延期权威四列均闭合。
  - Signal: 8-test JUnit、pristine 0.151 log、review 冻结合同与用户明确延期决定。
  - Capture method: targeted isolated nextest + test-name baseline lookup + durable manifest。
  - Event name or marker:
    - nextest run `83e27242-db88-4ac4-812c-36a46a9bfdd7`
  - Correlation keys:
    - HEAD `b3a05d73ac`
    - upstream `78c290807ce710180111df227df3b7a4fe845452`
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - 结果严格为目标 8 项且形成逐项映射。
  - Refutes if:
    - 任一项无法映射或出现未授权额外回归。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
  - E-005
  - E-006
- Conclusion: confirmed
- Repair design readiness: ready；只补证据与 W6 静态工件，不修改延期产品路径
- Next step: 进入 B1/B2 聚焦收口复审。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 对抗性审查确认 B2 证据缺口

- Related hypotheses:
  - H-001
  - H-002
- Direction: neutral
- Type: external-review
- Source: `vs_review/2026-09-01-codex-0151-catchup-review.md`
- Prediction or plan link:
  - H-001/H-002 的逐项映射前置条件
- Matched signal:
  - 最终完整日志与 per-test baseline/authority mapping 缺失
- Correlation keys:
  - review commit `b3a05d73ac`
- Raw content:
  ```text
  B2 — 剩余 7 failed + 1 timeout 未逐项绑定既有延期权威
  ```
- Interpretation: 证明当前证据不足，但不证明残余项本身应修复或延期。
- Time: 2026-09-02 09:00

## Evidence E-002: 当前 metadata 与 cache 状态阻断 W6

- Related hypotheses:
  - H-001
- Direction: neutral
- Type: reproduction
- Source: metadata validator、cache result 与 ledger
- Prediction or plan link:
  - P-001 的 W6 当前状态
- Matched signal:
  - validator exit 1；cache result partial；usage unavailable
- Correlation keys:
  - `WAR-20260901-073444-CACHE-REGRESSION-3BF5A4B3`
- Raw content:
  ```text
  replay overlay tree is stale relative to the Git index
  overlay inventory is stale relative to the Git index
  provider_boundary_accounting_status: unavailable
  ```
- Interpretation: B1 可复现；不授权真实运行，也不判定 H-001/H-002。
- Time: 2026-09-02 09:00

## Evidence E-003: 8 项定向隔离运行精确复现 7 failed + 1 timeout

- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: reproduction
- Source: `docs/releases/v0.0.7/codex-upstream-sync/evidence/current-vendor/core-approved-deferrals.junit.xml.gz`
- Prediction or plan link:
  - H-003 的精确集合预测
- Matched signal:
  - 8 tests run；0 passed；7 failed；1 timed out
- Correlation keys:
  - nextest run `83e27242-db88-4ac4-812c-36a46a9bfdd7`
  - HEAD `b3a05d73ac`
- Raw content:
  ```text
  Summary [60.025s] 8 tests run: 0 passed, 7 failed, 1 timed out
  Cyber inheritance/compaction/recover/websocket: 5 failed
  pending-input previous-turn context: 1 failed
  executor skill request count: 1 failed (2 != 3)
  websocket turn state reset: 1 timeout
  ```
- Interpretation: 当前 7+1 是稳定、有限且可逐项命名的集合；没有混入 full-run 的 cache snapshot 或 zsh-fork 资源超时。
- Time: 2026-09-02 09:16

## Evidence E-004: pristine 0.151 对照区分上游既有失败与 Whale 集成回归

- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: experiment
- Source: `docs/releases/v0.0.7/codex-upstream-sync/evidence/rust-v0.151.0/attempt-1-isolated-qualification/04-core-tests.log`
- Prediction or plan link:
  - H-003 的 pristine comparison 条款
- Matched signal:
  - recover 在 pristine 0.151 同样失败；child、remote compaction v1/v2、pending-input、websocket reuse、skills extension、turn-state 在 pristine 通过
- Correlation keys:
  - upstream `78c290807ce710180111df227df3b7a4fe845452`
- Raw content:
  ```text
  pristine 0.151: 3808 passed, 7 failed, 9 skipped
  recover_turn_restores_cyber_access_program_without_making_it_sticky: FAIL
  other seven target tests: PASS
  ```
- Interpretation: 不能把整个 7+1 归因于 upstream；准确结论是 1 个 upstream 同签名加 7 个 Whale 集成非绿项，整个集合已由用户批准延期到 TaskSpace 专项。
- Time: 2026-09-02 09:18

## Evidence E-005: W6 final-wire 与同步 metadata 静态修复通过

- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: final-wire targeted run、current overlay generator、metadata validator、script tests
- Prediction or plan link:
  - P-001 的 B1 静态关闭条件
- Matched signal:
  - cache final-wire 2/2；current overlay 883 paths；generator check PASS；metadata validator PASS；56/56 script tests PASS
- Correlation keys:
  - commit `b631eb7e67`
  - nextest run `9b54065a-6b43-4ebd-a6ad-b51bbbc397b3`
- Raw content:
  ```text
  2 tests run: 2 passed
  current overlay inventory: 883 paths
  sync metadata validation passed
  Ran 56 tests: OK
  ```
- Interpretation: B1 的零成本静态部分已修复；不代表真实 cache baseline 已通过或晋升。
- Time: 2026-09-02 09:35

## Evidence E-006: 受控全量区分 TaskSpace 7+1 与宿主 zsh-fork 限制

- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `docs/releases/v0.0.7/codex-upstream-sync/evidence/current-vendor/core-full-j4.junit.xml.gz`
- Prediction or plan link:
  - H-003 的“没有混入其他测试”条款与 W6 完整矩阵边界
- Matched signal:
  - 7 failed 精确等于已批准集合；turn-state timeout 稳定复现；另 13 项全部是 zsh-fork/exec-wrapper，单项复跑同样 timeout
- Correlation keys:
  - nextest full run `7612801b-9552-45fa-929d-e4d04697efbc`
  - zsh single run `1c9982d8-2c00-49fd-ac9b-f45d1876e8c7`
- Raw content:
  ```text
  3969 tests run: 3948 passed (1 flaky), 7 failed, 14 timed out, 9 skipped
  single zsh-fork probe: 1 test run, 1 timed out
  ```
- Interpretation: 7+1 的延期集合可审计；额外 zsh-fork timeout 是当前宿主验证限制，单列且不冒充 TaskSpace 延期或通过。
- Time: 2026-09-02 09:35

## Evidence E-007: 持久双臂真实资格通过并晋升当前 baseline

- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `WAR-20260902-071009-CACHE-REGRESSION-BBE4EBE4`、全局账本、cache acceptance 与 live gate
- Prediction or plan link:
  - P-001 的 B1 真实资格关闭条件
- Matched signal:
  - Standard + map-request 均 business success；usage/trace 完整；`e39d5bd4…` 晋升后 live gate PASS
- Correlation keys:
  - subject `7d492442f1`
  - proposal `CBP-F62CE3162005EC13`
  - result `WAR-20260902-071009-CACHE-REGRESSION-BBE4EBE4`
- Raw content:
  ```text
  2 sample runs completed; 13 provider requests
  input 166833 (cached 156928, uncached 9905); output 3014
  estimated cost 0.01907156 CNY
  cache regression gate: PASS e39d5bd4...
  ```
- Interpretation: B1 的真实运行、费用结算、持久证据与 baseline 晋升闭合；R2 另有成功审计记录，两组累计费用 0.05895688 CNY，低于 1 CNY 总包。
- Time: 2026-09-02 07:20
