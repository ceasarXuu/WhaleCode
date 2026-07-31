# Subagent VS Review: R8 缓存门禁收尾

- Created: 2026-08-01T00:00:00+08:00
- Updated: 2026-08-01T18:00:00+08:00
- Report schema: adversarial-v1
- Task: 审查 CR21.2 至 CR23 是否形成可信且不可自授权的缓存门禁闭环
- Report path: `vs_review/2026-08-01-r8-cache-gate-closeout-review.md`
- Review mode: fresh internal subagent
- Source session policy: 不继承主会话上下文，只接收中性导航包
- Status: Round 2 blocking fixes implemented; fresh closure review pending

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

## Final Conclusion

待审查闭环后填写。
