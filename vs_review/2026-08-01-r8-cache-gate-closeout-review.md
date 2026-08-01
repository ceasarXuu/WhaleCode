# Subagent VS Review: R8 缓存门禁收尾

- Created: 2026-08-01T00:00:00+08:00
- Updated: 2026-08-01T18:00:00+08:00
- Report schema: adversarial-v1
- Task: 审查 CR21.2 至 CR23 是否形成可信且不可自授权的缓存门禁闭环
- Report path: `vs_review/2026-08-01-r8-cache-gate-closeout-review.md`
- Review mode: fresh internal subagent
- Source session policy: 不继承主会话上下文，只接收中性导航包
- Status: closure passed at `bbbf1fc16`; P0/P1=0

## Round 1: 实现与发布链路审查

### Review Input

#### Objective

审查 R8 缓存命中回归门禁 CR21.2 至 CR23 的实现是否形成可信、简洁、不会自授权的闭环。

#### Review Target

代码实现、测试策略、证据持久化与发布门接线。

#### Target Locations

- `scripts/cache-regression/accepted_cache_baseline.py`
- `scripts/cache-regression/promote_cache_baseline.py`
- `scripts/cache-regression/run_cache_hit_regression.py`
- `scripts/cache-regression/check_cache_regression_gate.py`
- `scripts/cache-regression/cache_surface.py`
- `scripts/cache-regression/cache_budget.py`
- `scripts/cache-regression/cache_run_contract.py`
- `scripts/cache-regression/free_cache_contracts.py`
- `scripts/cache-regression/test_*.py`
- `benchmarks/cache-regression/cache-surface-contract.json`
- `.githooks/pre-commit`
- `scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1`

#### Change Introduction

实现把免费 final-wire 三态发现、人工选择预算、精确授权执行、持久证据、人工接受和 accepted baseline
晋升接为一条链。只有独立的 baseline 与 final-wire 晋升提交可通过；产品与 baseline 同提交应阻断。当前真实
基线仍为历史 `live_regression_failed`，本轮未运行付费 smoke。

#### Risk Focus

- status/hash、证据替换、index/worktree 混读或未跟踪文件能否自授权。
- 合法晋升是否永久锁死，release 在晋升提交后是否错误失败。
- result/proposal/authorization/ledger/artifact/acceptance/manifest 身份是否闭合。
- 失败、部分运行或未执行范围是否可能被晋升或扩大解释。
- 是否遗留 `live_verified` 双路径、固定阈值或固定两臂逻辑。
- 触发面是否漏报生产变化或过度阻断普通变更。
- 简化 fixture 是否造成测试假阳性。

#### User-Perspective Review Focus

- 门禁失败信息能否让维护者区分源码问题、证据问题和需要预算的下一步。
- 合法晋升路径是否可执行且不会要求维护者手工绕过门禁。

#### Implementation Completeness Focus

- 生产入口必须真实接线，不接受仅存在于测试或文档的实现。
- 检查付费 runner 是否保存可长期复算的最小证据集。
- 检查 pre-commit 和 non-agent release gate 是否调用同一权威门禁。

#### Target Benefit Focus

- 目标是重发现、强判别、轻处置，不判断产品变化是否正确或缓存率是否足够好。
- 成本和缓存命中收益尚未做新的真实复验，不得冒充已证明。

#### Assumptions To Attack

- Git source identity、持久证据完整性、人工授权唯一性、失败关闭和晋升提交独立性。

#### Adversarial Lenses

- state
- failure
- data
- maintenance
- testing
- observability
- implementation-completeness

#### Verification Status

- Python cache control-plane tests: 88 passed。
- 免费合同：7 条命令、11 个场景、0 changed、0 uncomparable。
- `cargo fmt --all -- --check`、运行账本门和 PowerShell wrapper parse 通过。
- 本轮没有真实 Whale Agent/provider 运行；新的 accepted baseline 尚未激活。

#### Reviewer Instructions

- Fresh internal subagent session，`fork_context=false`。
- 直接读取目标文件，禁止修改文件。
- 禁止运行 Whale Agent 或任何 provider 请求。
- findings 按严重度排列，并引用文件与行号。

### Internal Subagent Unavailable Fallback

- Internal subagent unavailable reason: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| high-risk | 20 minutes | 最多一次 10 minutes | 2 | 审查不可用时不能判定通过 |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| release evidence adversary | 发布门和证据链属于高风险状态/数据边界 | 自授权、身份错配、永久锁死、证据不可复算 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| release evidence adversary | `multi_agent_v1__spawn_agent` / gpt-5.6-sol xhigh | `019fb99f-838a-79a1-b14a-49a9bd6fcf97` | spawn tool call and completion notification | `fork_context=false` | Round 1 Review Input | 主会话历史、推理、草稿、结论 | yes |

### Reviewer Timeout Records

Round 1 在时限内完成，无 timeout。

### Reviewer Outputs

Reviewer 报告了 7 个 blocking finding：

1. `FixtureMode` 可产生没有明确 fixture 身份的全通过 artifact，正式 start/release 消费者可能接受。
2. accepted baseline 校验只核对相互一致的字段和 hash，未从 artifact 复算语义，也未绑定 gate trigger、attempt 和
   ledger totals；测试 helper 可现场构造简化证据并自证通过。
3. 同一用户授权可串行或并发重放，ledger 没有锁内原子 claim。
4. 产品 final-wire 改动无法先提交，proposal/promote 又要求 clean current HEAD，合法流程形成死锁；失败基线无
   显式 unchanged 复验路径。
5. 非零 attempt 留下 artifact 或逐 run 超预算时，结果仍可能被标为 completed 并晋升。
6. provider 触发面遗漏 `protocol/src/**`、`codex-api/src/**` 和相关 Cargo 配置。
7. `--source index` 的免费合同实际运行 worktree，未暂存 baseline/helper 可能影响 staged 判定。

Non-blocking 风险包括：当前真实基线仍失败、旧文档术语滞后、尚无远端 required workflow、晋升多文件写入不是事务。
Reviewer 还要求补充候选提交到晋升的闭环测试、授权重放、失败/越预算 attempt、触发面和 fixture/formal 隔离测试。

### Main Agent Response

| Finding | Response | Resolution |
|---|---|---|
| 1 | accept | `3ed8ddc0f` 增加 `mode=fixture`、`release_eligible=false`，formal consumers 必须拒绝 fixture，并要求 `cache_regression_surface`。 |
| 2 | accept（密码学签名要求 reject） | `2edb5458a` 抽取唯一 source-aware 校验器，晋升与 release 共同复算 proposal、gate、attempt、artifact、budget、ledger、acceptance 和 manifest。项目明确不承诺抵抗恶意仓库写权限者，因此不新增外部签名体系。 |
| 3 | accept | `724433022` 使用文件锁和一次性 `authorization_id` 原子 claim；重复或并发授权拒绝。 |
| 4 | accept | `97a6e0c9c` 允许免费合同明确判为 changed 的候选产品提交，但 release 保持阻断；增加 clean HEAD 报告和显式 `--request-revalidation`，支持空 scenario 的 baseline-only 晋升。 |
| 5 | accept | `724433022` 与 `2edb5458a` 同时在 runner 和独立晋升校验中拒绝失败 attempt、usage gap、逐 run 越预算和不完整矩阵。 |
| 6 | accept | `5ab106517` 扩展 provider/protocol/Cargo 生产触发面并增加 mutation fixture。 |
| 7 | accept | `5ab106517` 对 index 免费合同所依赖的相关未暂存/未跟踪输入 fail closed；不声称 materialize index，而是明确拒绝混读。 |

### Closure Status

- Blocking findings found: 7
- Accepted blocking findings fixed: 7
- Blocking re-review completed: pending
- Blocking re-review passed: pending
- Allowed to proceed: pending

## Round 2: Blocking Closure Re-review

### Review Input

审查当前 HEAD 的完整缓存门禁闭环，逐项复验 Round 1 七个 blocking，并主动寻找新的错误放行、错误成本、闭环不可执行
或关键漏报。威胁模型明确不包含恶意仓库写权限者伪造外部用户身份。禁止修改文件和运行任何 provider/Whale Agent。

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| release evidence adversary | accepted blocking 修复必须由新的无上下文会话复审 | 证据链、成本授权、source identity、发布接线 |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| release evidence adversary | `multi_agent_v1__spawn_agent` | `019fb9c6-28b4-7a32-ac4f-0ecdfccb8c88` | spawn tool call | `fork_context=false` | Round 2 中立导航包 | 主会话历史、Round 1 结论、实现推理 | yes |

### Verification Before Review

- Python：101 tests passed。
- 免费 final-wire：7 commands passed。
- Rust format：passed。
- non-agent builder、E3 start gate、release decision：passed。
- formal release gate：因当前 `live_regression_failed` 按预期阻断。
- Whale Agent/provider run：0。

### Reviewer Outputs

Round 2 报告 5 个 blocking：

1. formal marker 仍可用自洽文本冒充正式缓存门禁；
2. 请求/token/时间上限只是事后观测，外层超时可能遗留付费容器，失败费用可能被低报；
3. promotion/release 校验弱于真实 runner；
4. 控制面遗漏正式消费者和完整 fixture 生产链；
5. ledger claim 只有互斥，没有崩溃原子性和 settlement 恢复。

Non-blocking 包括晋升多文件写入不是事务、`fcntl` 的平台边界、文档滞后和既有格式问题。审查没有运行 Whale Agent
或 provider。

### Main Agent Response

| Finding | Response | Resolution |
|---|---|---|
| 1 | accept | `5173f8f89` 让 builder 保存结构化 gate report；start/release 复算 schema、source、live/clean 要求、commit、SHA、baseline 和免费矩阵，文本 selftest 反例拒绝。 |
| 2 | accept，拆分硬边界与观测边界 | `7d233aaa6`、`e5d4c3afd`、`f8f1a2180` 在通用 provider 层增加进程共享请求硬上限；`4e7c293ad` 按官方单请求上限计算最坏费用、传入 runner、按标签回收超时容器，并把不完整费用标为 partial/unavailable。token 常态值明确降为观测阈值，不伪称硬上限。 |
| 3 | accept | `5173f8f89` 让执行、晋升和 release 复用完整 validator，补全时间、claim receipt、授权唯一性、ledger outcome/usage 等负例。 |
| 4 | accept | `5173f8f89` 将 E3/release 正式消费者、test crate 注册和 helper 闭包纳入控制面发现。 |
| 5 | accept | `5173f8f89` 使用独立 lockfile、临时文件、文件/目录 `fsync` 和原子 replace，并增加从持久 result 幂等恢复 settlement 的命令与中断测试。 |

### Closure Status

- Blocking findings found: 5
- Accepted blocking findings fixed: 5
- Blocking re-review completed: pending（必须使用新的无上下文 reviewer）
- Blocking re-review passed: pending
- Allowed to proceed: pending

## Round 3: 物理付费出口与证据时序审查

新的空白 reviewer 主动扩展到逻辑请求与物理请求的差异，报告 8 项 blocking：隐藏 retry、WebSocket 与子进程可能
放大物理请求；本地 JSON 授权和 formal report 不具外部签名；付费 runner/container 控制面未绑定授权；超时清理
缺少稳定后置确认；崩溃结算恢复和证据时序不完整；accepted baseline 可能只证明自洽而非语义。审查未运行 provider。

Main Agent 处理：

- 接受物理出口、控制面、清理、恢复、时序和语义复算问题，分别由 `d9224e8cd` 至 `76568a061`、
  `474597062` 和既有共享 validator/原子账本修复；
- 拒绝在当前威胁模型中引入密码学授权和外部签名。拥有仓库写权限的恶意维护者可同时改授权、ledger、证据和
  release 规则，Git/远端保护才是该信任根；本门禁只防意外绕过、身份错配和未获授权执行；
- formal report 继续由正式消费者从结构化 artifact 复算，不接受单纯自报字段。

## Round 4: arm、执行输入、Realtime 与迟到容器审查

Reviewer `019fba13-f2d9-7400-9634-8cc688d320c3` 报告 4 项 blocking：

1. observation 自报 arm，可复用 Standard artifact 冒充 map-request；
2. 用户授权未绑定实际 runner、container control plane 和 scenario；
3. Realtime conversation 独立 WebSocket dispatch 绕过共享上限；
4. cleanup 首次空集即成功，进程后代可能创建迟到容器。

对应修复：`cccb47004` 绑定 arm/logical mode/run path；`2950b3802`、`76568a061` 绑定执行输入和付费控制脚本；
`b58f2f9c4`、`ecd1e929b` 将 Realtime 约束移到真实生成边界；`474597062` 要求终止进程组和连续三次容器空集。

## Round 5: Agent 可支配边界与跨平台清理审查

Reviewer `019fba2a-d21b-7c82-80c0-aa9a6fc1cf80` 在 HEAD `76568a061` 报告 3 项 blocking：

1. Agent 容器可读取真实 Key、重置 `/artifacts` 计数，或用 `curl` 绕过 Whale client；
2. Realtime 一条连接内可触发多次 `response.create`，连接级 claim 不等于推理级 claim；
3. Windows timeout 只停止父 `pwsh`，不停止完整进程树。

对应修复：

- `fac57d0d8` 用 Docker internal network 和固定上游 provider boundary 把真实 Key、权威计数及 provider 出口移出
  Agent 边界；`d620349f9` 将配套网络纳入超时清理；
- `ecd1e929b` 在每个显式 `response.create` 发送前 claim；主线程随后主动发现 Server VAD 自动生成，
  `26c5a87fd` 对无法前置计数的 conversational Realtime 在专项硬上限下 fail closed；
- `45bd0c2f6` 在 Windows 使用 `taskkill /PID ... /T /F` 并等待退出。

离线验证包括 125 项 Python、2 项固定上游代理单测、6 项 Rust hard-limit 单测和真实 Docker 网络隔离自检；未运行
Whale Agent/provider。Round 5 原 reviewer 的三项 finding 均已实施修复，最终结论等待 Round 6 对最新 HEAD 复验。

## Round 6: 最新 HEAD 独立闭环复审

- Reviewer：`019fba3f-9c88-7cf3-a122-3d2766f028cc`，`fork_context=false`，只读。
- Target：`d620349f9`。
- Focus：凭据/计数不可绕过、Realtime 每次生成、Windows 整树终止、资源清理及普通 Tool/TaskSpace 回归。
- Status：completed。该轮继续发现代理合同/对账、Realtime 模式归一化、Windows Job Object 和 host secret 清理缺口，
  对应由 `e619890f1`、`84b94ceef`、`d08be480a`、`123116c4a` 修复。

## Round 7: 付费边界与跨平台复审

- Reviewer：`019fba5b-bfdc-7741-bd72-562f8f33a0a2`，`fork_context=false`，只读。
- Target：`123116c4a`。
- Findings：
  1. V2 transcription Realtime 仍未计数；
  2. Windows 无法导入顶层 `fcntl`；
  3. 进程在 Job assign 前已运行，存在后代逃逸窗口；
  4. promotion 忽略 network/secret cleanup；
  5. wire evidence 失败会让已发生请求从账本消失，且旧设计错误地试图区分 Agent 请求动机；
  6. KeyboardInterrupt 未检查 cleanup proof。
- Resolution：`0d3af4b54`、`837460b75`、`040c27ae6`。runtime 只对批准 provider 合同和请求数做机械硬约束；
  supervisor boundary count 是费用请求数权威，wire/token 只控制 performance evidence 与金额估算；Windows 改为
  `create_suspended -> assign -> resume`；promotion 和 cancellation 复用完整 cleanup contract。
- Offline verification：Python `136 passed`；Realtime 定向 Rust 测试通过；provider boundary Docker、non-agent
  builder、release decision、E3 start gate 自测通过；formal release 按预期只因 `live_regression_failed` 阻断。

## Round 8: 最新 HEAD 最终复审

- Reviewer：`019fba7a-2fc5-74e1-94f6-74487b167c03`，`fork_context=false`，只读，gpt-5.6-sol xhigh。
- Target：`040c27ae6`。
- Focus：Realtime 全出口、Windows 启动竞态、supervisor 权威计数、promotion/cleanup、runtime 职责边界。
- Status：completed。该轮与后续定向复核继续暴露 claim/settlement、严格类型和 Windows owner 异常路径，分别由
  `809e1d513`、`8bd820a9a` 收口；因此没有将 Round 8 误记为 closure pass。

## Round 9-10: 结算、类型与 owner 定向复核

- Mode：fresh internal subagent，`fork_context=false`，只读。
- Targets：`809e1d513`、`8bd820a9a`。
- Findings：final settlement 调用边界、布尔 elapsed/exit code、嵌套 Windows cleanup、非标准 JSON、arm 启动证据、
  未确认终止 owner 和 network post-remove proof 仍有缺口。
- Resolution：`8bd820a9a`、`3410db334`、`dc1faeecd`、`3b291b111`。
- Traceability note：这两轮 reviewer 的完整 runtime ID 没有在启动时写入报告，是审查记录缺口；finding、失败反例和
  修复提交已完整进入 COE。后续轮次恢复启动时登记 ID。

## Round 11: accepted blocking 复审

- Reviewer：`019fbaca-d860-78a0-9e77-ec37d9e18a60`（Sagan），gpt-5.6-sol xhigh，`fork_context=false`，只读。
- Initial target：`3410db334`；final target：`3b291b111`。
- Findings：
  1. P0：Standard/map-request 可共享相同 provider wire evidence 并通过完整 promotion；
  2. P1：map-always/map-append 被身份校验错误地合并成 map-request；
  3. P1：未确认终止的 Windows owner 只存在当前解释器内存；
  4. P1：recovery 锁外读、锁内覆盖，可丢失并发最终结算并接受非标准 JSON；
  5. P2：多文件 promotion 不是事务；门禁会阻断中间状态，当前不构成 release 绕过。
- Main Agent response：前四项全部 `accept` 并作为 blocking 修复；第五项 `defer` 为非阻断的独立原子 promotion
  增强，不将其伪装成已解决。
- Resolution：`a23b29cb6`、`1ba6c1232`、`9204926c2`；离线 Python `171 passed`，相关 PowerShell 合同通过。

## Round 12: accepted blocking closure 复审

- Reviewer：`019fbae1-4d30-7d71-8459-9dadf8a13a54`（Bacon），gpt-5.6-sol xhigh，`fork_context=false`，只读。
- Target：`9af758760`。
- Focus：cross-arm/policy 身份、atomic recovery、跨解释器 Windows owner、资源和费用后置条件、TaskSpace 语义边界。
- Findings：
  1. P1：attempt、ledger execution、ledger evidence 中的布尔值可利用 Python `bool` 继承 `int` 冒充退出码、repeat 和 token；
  2. P1：`CreateProcessW` 成功到 owner journal 落盘之间若发生进程级硬退出，挂起子进程尚未进入 Job，也没有 durable owner；
  3. P2：durable recovery 为同一 PID 打开新 handle 后，会直接覆盖并丢弃进程内已保留的旧 handle。
- Resolution：`72566a1b6` 对所有晋升整数证据执行精确类型和值校验；`a3344da1d` 使用
  `PROC_THREAD_ATTRIBUTE_JOB_LIST` 在 `CreateProcessW` 创建时原子归属 Kill-on-close Job，并在 durable recovery
  打开 PID 前先完成已有 handle owner 的释放。三项均有离线失败反例。

## Round 13: 最终空白 closure 复审

- Reviewer：`019fbaf4-bdd0-7e00-be4d-cf189456c75f`（Anscombe），gpt-5.6-sol xhigh，`fork_context=false`，只读。
- Target：`a3344da1d`。
- Focus：Windows hard-exit/handle/journal、promotion 类型与伪造、旧 assign 残留、缓存门禁及产品隔离全局约束。
- Findings：无 P0，发现 4 项 P1：recovery 未绑定原 claim 身份与预算、`partial` 结果无法恢复、proposal
  `repeat=true` 仍可通过、失败结算的 `api_requests=null` 与全局账本合同冲突；另有 P2 重复 JSON key 未拒绝。
- Resolution：`9ae528efb` 统一精确预算标量并拒绝重复 key；`5821b3354` 让 `partial` 成为正式结果状态，账本以
  `null + minimum + evidence_status` 忠实表达不完整请求计数；`1042384ff` 在账本锁内把恢复结果绑定到 durable
  claim 的 commit、surface、proposal、authorization、matrix、run root、evidence boundary 和预算。

## Round 14: 修复后最终 closure 复审

- Reviewer：`019fbb05-7c2e-7b32-bb1f-22986d88720a`（Laplace），gpt-5.6-sol xhigh，`fork_context=false`，只读。
- Target：`1042384ff`。
- Focus：Round 13 四项 P1、重复 key、Windows 原子所有权及修复回归。
- Findings：嵌套 bool/int、全局账本字段兼容和 `null` 聚合仍有 3 项 P1。
- Resolution：`e1fa83ef1` 引入递归精确 JSON 比较并迁移账本必需字段。

## Round 15: recovery 与 Schema 合同复审

- Reviewer：`019fbb0f-ceb9-7d70-b4ce-c7b6033e6ef6`（Plato），只读。
- Findings：recovery 仍有原生比较；JSON Schema 与 PowerShell 对 exact/minimum 关系不一致。
- Resolution：`8083f31ab`、`79f1c1d8c`。请求证据改为 exact 与 inexact 两种互斥形态。

## Round 16: 同步污染与强制测试复审

- Reviewer：`019fbb1d-1229-73d3-9029-41ce95c0dcd5`（Jason），gpt-5.6-sol xhigh，只读。
- Findings：claim/result 同步使用 bool 时相等性检查不足；result producer 残留宽整数；Schema 测试可跳过。
- Resolution：`650657a1d`。proposal/recovery 共用 selection 合同，producer 使用精确整数，Schema 结构合同测试无可选依赖。

## Round 17: result 汇总与 completed 完整性复审

- Reviewer：`019fbb29-737d-7e52-8d89-0c19ba97450c`（Einstein），gpt-5.6-sol xhigh，只读。
- Findings：result 自报 minimum/status 未被 recovery/promotion 复算；免费 timeout 接受 bool。
- Resolution：`b49765f47`、`94c3cf53e`。四条路径共用请求汇总，production-shaped fixtures 同步。

- Reviewer：`019fbb33-05e2-7c71-8948-e4b5adfa2d0a`（Cicero），gpt-5.6-sol xhigh，只读。
- Findings：completed recovery 未验证失败 attempt/token 恒等式；`mark_unsettled` 会降低已有下限。
- Resolution：`ad6df97d7`。共享 completed integrity、纯 cleanup 合同、运行中 request checkpoint 与单调恢复。

## Round 18: direct settlement 与 checkpoint 时序复审

- Reviewer：`019fbb42-a7b6-7040-8824-ae507c73b4ea`（Mencius），gpt-5.6-sol xhigh，只读。
- Findings：direct settlement 未绑定批准矩阵；请求 checkpoint 晚于证据复制。
- Resolution：`bbbf1fc16`。direct path 使用唯一 selection matrix；计数先写 attempt/ledger，再复制并哈希证据。

## Round 19: 最终限域 closure review

- Reviewer：`019fbb4f-f64a-7ae0-ac4c-3c04c17140da`（Turing），gpt-5.6-sol xhigh，`fork_context=false`，只读。
- Target：`bbbf1fc16`。
- Result：**无阻断发现，P0=0、P1=0**。195 项 Python 0 skip；Schema、8 条 ledger、正式 release 阻断均符合合同。
- Residual P2：promotion 多文件写入不是事务。中断会留下 dirty worktree，但 clean-HEAD、release relevant changes 与
  accepted manifest 重验会 fail closed；没有发布绕过或证据丢失，因此不阻断 CR-23 closure。

## Final Conclusion

CR-21.2 至 CR-23 的离线工程问题已关闭。最终 HEAD `bbbf1fc16` 在 fresh review 中无 P0/P1；真实 Whale
Agent/provider 运行总数为 0。仓库继续因历史 `live_regression_failed` 阻断 release，直到用户另行批准最小真实
smoke 并明确接受结果；该外部状态不重新打开本次工程问题。
