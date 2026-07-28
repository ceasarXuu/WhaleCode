# Problem P-001: A2-C observer 错报 request 边界和 canonical Map
- Status: fixed
- Created: 2026-07-29 03:31
- Updated: 2026-07-29 04:11
- Objective: 让 A2-C 报告从 frozen raw artifacts 忠实还原每个 provider request、控制结果和最终 canonical Map
- Symptoms:
  - TaskSpace request path 把整个 run 的 Tool calls 全部放进 `request_index=1`
  - 18 个 TaskSpace Map Store 导出全部报 `has no TaskSpace map binding`
  - 报告显示 Map/node/edge 为零，但 trace 中 `read_map` 和 `finish_map` 均成功
  - `committed_initialize_and_execute=0`，同时 `first_request_initialization_commits=1`
- Expected behavior:
  - 每次 provider response 的 Tool calls 归入对应 request
  - observer 从该 run 的持久化 Store 导出唯一 canonical Map
  - 初始化、失败、闭合和 Map 形状统计与原始结果 algebra 一致
- Actual behavior:
  - request parser 依赖并非逐 response 发出的 warning 事件，最后只在 `task_complete` 一次性 flush buffer
  - host observer 使用宿主默认 Whale Home，未读取容器挂载在 artifacts 下的 StateDB
  - control result parser 只认 `TaskSpaceControlResultV2`，漏掉现行 `TaskSpaceResponseCommitV1`
- Impact:
  - R-10/R-22 的动作路径、standalone、失败 request 和 Map 形状均无法可信验收
  - call-level 重复失败可能被误算为 response-level 失败
  - A2-C 不能基于现有 report 晋升或否决候选
- Reproduction:
  - 对 A2-C run root 重新执行 `report-r7-five-layer-matrix.ps1`
  - 检查任一 TaskSpace `request_path`、`taskspace-map-store.stdout.log` 和 `taskspace-control-usage.json`
- Environment:
  - Linux / PowerShell benchmark observer / source commit `abe2b872b6708e666293d0018ecd3654bf5a65cc`
  - run root `target/r7-five-layer-matrix/a2-c/abe2b872b/20260729-0315`
- Known facts:
  - 24 个 rollout 和 24 个业务结果均完整
  - 18/18 TaskSpace rollout 存在成功的 `finish_map` 且 `state_commit=true`
  - 将 `WHALE_HOME` 指向单 run 的 `artifacts/home/.whale` 后，CLI 立即导出 terminal Map
  - 成功初始化输出 schema 为 `TaskSpaceResponseCommitV1`
- Ruled out:
  - canonical Map 实际未闭合；18/18 `finish_map` 成功，定向 Store 导出 `terminal=true`
  - 容器未保留 StateDB；artifact 内存在 `state_5.sqlite`、WAL 和完整 Map
- Fix criteria:
  - request path 与 token-count/provider response 数一一对应，Tool calls 不跨 request
  - observer 显式读取每个 run 的 artifact Whale Home，18/18 Map Store 导出成功
  - 支持现行 commit/failure/result schema，并区分 response failure 与 skipped sibling
  - 使用现有 raw artifacts 重建报告，无需伪造或重跑 provider
  - observer 单元测试、harness 回归和 A2-C 数据一致性门通过
- Current conclusion: request boundary、run-scoped Store、现行 result schema 和初始化预检失败计数均已修复；冻结 raw artifacts 已完成全量重算
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - 18/18 TaskSpace Store 导出 `availability=measured`，Map 均 terminal
  - 24/24 request path 的边界数与 provider request 数一致
  - 初始化计数恢复为 `54 total / 18 committed / 36 failed`
  - 定向 observer、cost、performance 和 harness 回归通过
- Close reason:
  - observer 已能从 frozen raw artifacts 忠实恢复 A2-C 事实

## Hypothesis H-001: request parser 错把稀疏 warning 当作逐 response 边界
- Status: fixed
- Parent: P-001
- Claim: `Get-R7TaskspaceRequestPath` 只在 `provider_response_actionability` trace event 上 flush，而生产仅在特定 warning 条件下发该事件，普通 run 因此直到 `task_complete` 才 flush
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 当前 raw rollout 有逐请求 `token_count`，但没有逐请求 actionability event
- Falsifiable predictions:
  - If true: 生产 rollout 中 `token_count` 数等于 provider request 数，actionability event 数为零或更少，现 parser 将所有 call 放到首个 request
  - If false: 每个 response 都有递增 request_count tag，collapse 来自别的 buffer bug
- Diagnostic evidence plan:
  - Prediction or clause under test: 对比单 run 的 token_count、actionability event 和生成 request rows
  - Signal: raw rollout 事件序列与 parser 输出
  - Capture method: jq/PowerShell 统计并按时间列出 function_call、token_count、output
  - Event name or marker:
    - `token_count`
    - `provider_response_actionability`
  - Correlation keys:
    - rollout path
    - request index
    - call_id
  - Differentiates from:
    - provider 本身把全部 Tool calls 放在一个 response
  - Supports if:
    - Tool call batch 在每个 token_count 前出现，但 parser 仅生成一个非空 request
  - Refutes if:
    - raw trace 本身没有可用边界
  - Instrumentation status: existing-observability-sufficient
  - Instrumentation lifecycle:
    - 保留 token usage 和 call identity
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: 生产事件顺序与代码分支共同确认
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - 已改用逐 response `token_count` 边界，并按全局 `call_id` 回填 outcome

## Hypothesis H-002: Map Store exporter 读取了宿主默认 StateDB
- Status: fixed
- Parent: P-001
- Claim: benchmark 在容器退出后从宿主调用 `whale debug taskspace-map`，但没有把 `WHALE_HOME` 指向该 run 的 `artifacts/home/.whale`
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - Docker contract 把 HOME 设置为 `/artifacts/home`，Store 因而保存在每个 run 的 artifact 目录
- Falsifiable predictions:
  - If true: 当前命令找不到 binding；仅增加正确 `WHALE_HOME` 后同 thread 导出成功
  - If false: 指向 artifact Home 后仍找不到 binding
- Diagnostic evidence plan:
  - Prediction or clause under test: 同一 binary/thread 在两个 Home 下导出
  - Signal: CLI exit code、Map export envelope、terminal/revision
  - Capture method: 对一个 frozen run 执行只读 debug export
  - Event name or marker:
    - `taskspace.map_store_export_missing_binding`
    - `taskspace.map_store_exported`
  - Correlation keys:
    - thread_id
    - map_id
  - Differentiates from:
    - Map 未持久化或 thread ID 提取错误
  - Supports if:
    - artifact Home 下导出 terminal Map
  - Refutes if:
    - 两个 Home 都无 binding
  - Instrumentation status: existing-observability-sufficient
  - Instrumentation lifecycle:
    - observer 需永久记录实际 Store path/Home
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: 定向导出确认 observer 读错数据库
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - exporter 显式切换并恢复 run-scoped `WHALE_HOME/CODEX_SQLITE_HOME`

## Hypothesis H-003: control usage parser 仍只识别旧结果 schema
- Status: fixed
- Parent: P-001
- Claim: `cost-instrumentation.ps1` 只把 `TaskSpaceControlResultV2` 计为已提交初始化，而现行 sequence prepare 成功输出 `TaskSpaceResponseCommitV1`
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 统计结果同时出现初始化提交为零和首请求提交为一
- Falsifiable predictions:
  - If true: raw success output 是 V1，parser 条件只检查 V2
  - If false: raw output 仍是 V2，漏计来自 call_id 配对
- Diagnostic evidence plan:
  - Prediction or clause under test: 对照 raw output schema 与 parser 分支
  - Signal: `schema_version/action/state_commit` 和对应 call_id
  - Capture method: jq 提取成功初始化结果；读取 parser 条件
  - Event name or marker:
    - `TaskSpaceResponseCommitV1`
  - Correlation keys:
    - call_id
    - canonical revision
  - Differentiates from:
    - 初始化实际未提交
  - Supports if:
    - V1 成功结果未进入 V2-only 分支
  - Refutes if:
    - parser 已识别 V1
  - Instrumentation status: existing-observability-sufficient
  - Instrumentation lifecycle:
    - 现行 result schema 应有固定回归 fixture
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: raw result 与 parser 条件直接确认 schema 漂移
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - 已覆盖 response commit、commit failure、lifecycle result 和 response preflight failure

## Evidence E-001: raw rollout 有逐请求边界但 analyzer 没有使用
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: A2-C 任一 TaskSpace `artifacts/rollout.jsonl` 与 `r7-five-layer-trace-analysis.ps1:149-192`
- Prediction or plan link:
  - H-001 的事件频率和 buffer flush 预测
- Matched signal:
  - function calls 在各自 token_count 前成批出现，actionability event 不逐请求出现
- Correlation keys:
  - run `20260729-031503-741`
  - thread `019faa26-f0f3-7732-abf2-68615563a309`
- Raw content:
  ```text
function_call exec_command
TOKEN 12947
function_call taskspace_control
function_call exec_command
TOKEN 13129
...
Get-R7TaskspaceRequestPath only flushes on provider_response_actionability or task_complete.
  ```
- Interpretation: provider 已产生多个 response，collapse 发生在 observer
- Time: 2026-07-29 03:31

## Evidence E-002: 指向 run-scoped Whale Home 后 canonical Map 可直接导出
- Related hypotheses:
  - H-002
- Direction: supports
- Type: experiment
- Source: `WHALE_HOME=<artifact>/home/.whale whale debug taskspace-map --thread-id ...`
- Prediction or plan link:
  - H-002 的同 thread 双 Home 判别实验
- Matched signal:
  - export status ok，terminal true，Map revision 12
- Correlation keys:
  - thread `019faa26-f0f3-7732-abf2-68615563a309`
  - map `map-019faa26-f0f3-7732-abf2-68615563a309`
- Raw content:
  ```text
schema_version=TaskSpaceMapExportR7V2
status=ok
map_revision=12
store_revision=13
terminal=true
node_count=5
edge_count=4
completion_count=3
result_ref_count=7
  ```
- Interpretation: Map 和 binding 没有丢失；host observer 读取了错误的 Home/StateDB
- Time: 2026-07-29 03:31

## Evidence E-003: 初始化成功 schema 与 observer 条件不一致
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: A2-C raw rollout；`scripts/taskspace-benchmark/lib/cost-instrumentation.ps1:1428-1441`
- Prediction or plan link:
  - H-003 的 schema 对照
- Matched signal:
  - raw=`TaskSpaceResponseCommitV1`，parser only=`TaskSpaceControlResultV2`
- Correlation keys:
  - action `initialize_and_execute`
  - revision `0 -> 1`
- Raw content:
  ```text
{"schema_version":"TaskSpaceResponseCommitV1","status":"accepted","success":true,
 "state_commit":true,"action":"initialize_and_execute","revision_before":0,"revision_after":1}

if ($schemaVersion -eq "TaskSpaceControlResultV2" -and
    $rolloutInitializeCallIds.Contains($callId)) { ... }
  ```
- Interpretation: `committed_initialize_and_execute=0` 是 observer 漏计，不是运行时未提交
- Time: 2026-07-29 03:31

## Evidence E-004: 冻结 artifacts 重算通过完整性代数
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: supports
- Type: fix-validation
- Source: `target/r7-five-layer-matrix/a2-c/abe2b872b/20260729-0315`
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - request、Map 和初始化结果均可从原始 rollout/Store 重建
- Correlation keys:
  - source commit `abe2b872b6708e666293d0018ecd3654bf5a65cc`
  - observer fixes `342951999`、`7da03a15a`
- Raw content:
  ```text
provider rollouts: 24/24 complete
TaskSpace Map exports: 18/18 measured, terminal=true
initialize_and_execute: 54 total = 18 committed + 36 failed
request boundary mismatch: 0
  ```
- Interpretation: observer 不再把缺失证据默认为零，也不再混淆 call-level copied failure 与 request-level failure
- Time: 2026-07-29 04:11
