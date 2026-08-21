# Problem P-001: rebase 后 cache regression 测试隔离失效
- Status: fixed
- Created: 2026-08-21 00:53
- Updated: 2026-08-21 00:55
- Objective: 恢复 cache regression 测试在不调用真实 workspace/provider、且不受仓库历史账本影响时的确定性。
- Symptoms:
  - 完整 cache regression 231 项中 1 failure、12 errors。
- Expected behavior:
  - 临时目录测试应通过 mock 边界运行；跨语言 usage fixture 应存在；账本单测仅验证自建夹具。
- Actual behavior:
  - workspace preflight 访问非 Git 临时目录；usage fixture 缺失；全局账本历史记录影响断言。
- Impact:
  - rebase 后 cache regression 门禁无法作为有效回归证据。
- Reproduction:
  - `python3 -m unittest discover -s scripts/cache-regression -p 'test_*.py'`
- Environment:
  - Linux，分支 whalecode-codex，rebase origin/main 后。
- Known facts:
  - Cargo metadata --locked 已通过；sync 工具 48 项通过。
- Ruled out:
  - Cargo workspace/lock 不一致。
- Fix criteria:
  - 三类定向测试通过，完整 cache regression 重跑通过或只剩有独立证据的非本次问题。
- Current conclusion: 三个独立回归均已修复，16 项原始失败集合全部通过。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-001、H-002、H-003；E-004、E-005、E-006
- Close reason:
  - 原始失败集合通过 fix-validation。

## Hypothesis H-001: 临时 repo 测试未隔离 workspace binary preflight
- Status: confirmed
- Parent: P-001
- Claim: runner 测试把 `--repo-root` 指向非 Git 临时目录，却未 mock 新增的 `resolve_workspace_binary`，因此在原有测试 double 之前失败。
- Layer: interaction
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 10 个错误共享同一 `git rev-parse --show-toplevel` 栈。
- Falsifiable predictions:
  - If true: 失败均发生在 `run_cache_hit_regression.main` 的 workspace binary 解析，且测试使用临时 repo。
  - If false: mock 已覆盖该函数或失败发生在 provider/ledger 原断言路径。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查失败栈与测试 patch 集合。
  - Signal: 调用栈及 `patch(...)` 列表。
  - Capture method: 完整回归输出与代码搜索。
  - Event name or marker:
    - resolve_workspace_binary
  - Correlation keys:
    - test name
  - Differentiates from:
    - 产品 workspace gate 本身失效
  - Supports if:
    - 临时 repo 测试未 mock preflight，真实 workspace require-ready 仍通过。
  - Refutes if:
    - 真实 workspace 也失败或测试已 mock preflight。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-004
- Conclusion: 完整回归栈与真实 workspace ready 结果共同确认测试隔离缺口。
- Repair design readiness: ready
- Next step: 在共享测试 helper 或各 runner 测试中 mock workspace binary 解析。
- Blocker:
  - none
- Close reason:
  - fixed

## Hypothesis H-002: provider usage fixture 在 vendor 截断时丢失
- Status: confirmed
- Parent: P-001
- Claim: Python/Rust 跨语言 usage contract 测试仍引用固定路径，但 0.147 vendor tree中该 JSON 已不存在。
- Layer: regression-window
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 两个测试均报同一路径 FileNotFoundError。
- Falsifiable predictions:
  - If true: 引用常量存在而目标文件缺失，原分支历史中可找到该 fixture。
  - If false: 文件存在但读取权限或路径解析错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: 对照 HEAD 文件树与原分支历史 fixture。
  - Signal: `git cat-file`/文件存在性和 fixture 内容。
  - Capture method: Git 对象读取与定向测试。
  - Event name or marker:
    - provider_usage_contract.json
  - Correlation keys:
    - fixed fixture path
  - Differentiates from:
    - Python 导入或工作目录问题
  - Supports if:
    - 文件缺失且引用路径正确。
  - Refutes if:
    - 文件存在或测试引用已迁移。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-002
  - E-005
- Conclusion: FileNotFoundError 直接确认 fixture 缺失。
- Repair design readiness: ready
- Next step: 恢复 provider-neutral fixture 到当前 contract 路径并运行 Rust/Python定向测试。
- Blocker:
  - none
- Close reason:
  - fixed

## Hypothesis H-003: 账本单测读取了仓库真实历史记录
- Status: confirmed
- Parent: P-001
- Claim: `test_global_checker_accepts_truthful_partial_request_count` 调用全局 checker 时未把输入隔离到测试账本，main 新增的 R8 记录先触发 monetary cost 状态错误。
- Layer: environment
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 失败记录 ID `WAR-20260820-214337-R8-MAP-REQUEST-R3` 不属于测试名所述夹具。
- Falsifiable predictions:
  - If true: checker 默认读取仓库账本且测试未传入隔离路径。
  - If false: 该记录由测试创建或 checker 已接受显式夹具路径。
- Diagnostic evidence plan:
  - Prediction or clause under test: 检查测试 subprocess 参数和 checker 输入解析。
  - Signal: 命令行参数、默认账本路径、失败记录来源。
  - Capture method: 代码读取与单测定向复现。
  - Event name or marker:
    - WAR-20260820-214337-R8-MAP-REQUEST-R3
  - Correlation keys:
    - ledger run id
  - Differentiates from:
    - partial request count contract 本身错误
  - Supports if:
    - 测试 assertion 未被执行，先被外部历史记录阻断。
  - Refutes if:
    - 失败记录由测试夹具生成。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-006
- Conclusion: 失败 ID 证明测试环境泄漏到仓库历史账本。
- Repair design readiness: ready
- Next step: 让 checker 接受测试账本路径或在测试 repo 中运行完整隔离副本。
- Blocker:
  - none
- Close reason:
  - fixed

## Evidence E-001: workspace preflight 栈与真实 ready 对照
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: cache regression 完整输出；`workspace_context.py require-ready --json`
- Prediction or plan link:
  - H-001 临时 repo 与真实 workspace 行为分离预测。
- Matched signal:
  - 10 个测试在临时目录 `git rev-parse` 失败；真实 workspace 返回 `workspace_ready`。
- Correlation keys:
  - resolve_workspace_binary
- Raw content:
  ```text
  ContextError: git rev-parse --show-toplevel failed: fatal: not a git repository
  {"ready": true, "reason_code": "workspace_ready"}
  ```
- Interpretation: 产品门禁可用，缺口位于 runner 测试 double。
- Time: 2026-08-21 00:53

## Evidence E-002: usage fixture 固定路径缺失
- Related hypotheses:
  - H-002
- Direction: supports
- Type: test
- Source: `test_cache_hit_regression.py` 完整回归栈
- Prediction or plan link:
  - H-002 文件存在性预测。
- Matched signal:
  - 两项测试对同一 vendor JSON 报 FileNotFoundError。
- Correlation keys:
  - provider_usage_contract.json
- Raw content:
  ```text
  FileNotFoundError: third_party/codex-cli/codex-rs/codex-api/tests/fixtures/provider_usage_contract.json
  ```
- Interpretation: 跨语言 contract fixture 未随 0.147 cutover 保留。
- Time: 2026-08-21 00:53

## Evidence E-003: 账本断言被外部 R8 记录抢先阻断
- Related hypotheses:
  - H-003
- Direction: supports
- Type: test
- Source: `test_cache_run_ledger.CacheRunLedgerTest.test_global_checker_accepts_truthful_partial_request_count`
- Prediction or plan link:
  - H-003 外部记录来源预测。
- Matched signal:
  - checker 报仓库历史 R8 run id，而非测试创建记录。
- Correlation keys:
  - WAR-20260820-214337-R8-MAP-REQUEST-R3
- Raw content:
  ```text
  WAR-20260820-214337-R8-MAP-REQUEST-R3 monetary cost status is invalid
  ```
- Interpretation: 测试未隔离 checker 的账本输入。
- Time: 2026-08-21 00:53

## Evidence E-004: runner 临时 repo 原始失败集合通过
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: 定向 unittest
- Prediction or plan link:
  - H-001 修复后临时 repo 不再进入真实 workspace gate。
- Matched signal:
  - binary health、claim、aggregation、execution 测试全部通过。
- Correlation keys:
  - 16-test fix-validation
- Raw content:
  ```text
  Ran 16 tests in 0.682s
  OK
  ```
- Interpretation: 共享 resolver mock 恢复测试隔离且未放宽产品门禁。
- Time: 2026-08-21 00:55

## Evidence E-005: usage contract 原始失败用例通过
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: 定向 unittest
- Prediction or plan link:
  - H-002 fixture 恢复后两项跨语言 contract 测试通过。
- Matched signal:
  - 两项 fixture 测试包含在 16 项通过集合中。
- Correlation keys:
  - provider_usage_contract.json
- Raw content:
  ```text
  Ran 16 tests in 0.682s
  OK
  ```
- Interpretation: fixture 路径和内容满足当前 Python contract。
- Time: 2026-08-21 00:55

## Evidence E-006: 账本定向 contract 不再受历史记录影响
- Related hypotheses:
  - H-003
- Direction: supports
- Type: fix-validation
- Source: 定向 unittest
- Prediction or plan link:
  - H-003 只保留测试拥有记录后 partial/exact 两侧断言均执行。
- Matched signal:
  - global checker 定向测试通过。
- Correlation keys:
  - partial-ledger.json
- Raw content:
  ```text
  Ran 16 tests in 0.682s
  OK
  ```
- Interpretation: 测试不再把仓库历史账本有效性当作自身前置条件。
- Time: 2026-08-21 00:55
