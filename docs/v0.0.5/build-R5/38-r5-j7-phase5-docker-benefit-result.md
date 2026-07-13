# R5-J7.5 Docker 样本与收益验收结果

- Date: 2026-07-13
- Status: **COMPLETE，14/14 acceptance gates verified**
- Final binary commit: `f48f3c80dcbd3d5241925bbb5792f2934abd61d0`
- Final binary SHA-256: `e03912bd7f254dd96595c0ccd66634732aaf631e1352d298ae17ca0f1dc656a9`
- Execution substrate: Docker hard boundary
- Model: `deepseek-v4-flash`

## 1. 结论

J7.5 在 J7.6-J7.8 修复后完成第二次 Docker 复验并关闭。两个样本的 Standard/R5 均完成且通过外部验证；
两组 R5 的 protocol/state failure、terminal duplicate、identity missing、committed repeat finish、Map open、
request-wide multi-patch 和 terminal bad commit 均为 0，patch max/request 均为 1。14 项 correctness/benefit
门禁全部通过。

下文第2-8节保留首次验收及 J7.6 中间复验的历史证据；最终结果以第9节为准。不能把 order 的单次成本下降
或 billing 的单次成本上升外推为稳定性能收益。

## 2. 执行证据

| Sample | Evidence | Eligibility |
|---|---|---|
| `multi-file-order-pipeline` | `target/r5-j7-5-contract-order/multi-file-order-pipeline/20260713-041900-801` | valid single-run diagnostic；两侧 complete/solved |
| `subscription-billing-repair` | `target/r5-j7-5-contract-billing/subscription-billing-repair/20260713-041900-801` | valid single-run diagnostic；外部验证通过；原 lifecycle 假阴性由 CoE 修复并重算为 complete |
| description 修复前基线 | `target/r5-j7-5-order/.../20260713-040903-270`、`target/r5-j7-5-billing/.../20260713-040903-283` | 缺口发现证据，不进入最终收益结论 |

R4 没有同 revision、同 Docker contract、同 observer 口径的这两个样本 artifact。build-R4 只保留了
`multi-file-order-pipeline` 的更早 R3 sweep 上下文（wrong、约 346s、无 patch），billing 无历史同样本。
按观测规则标记为 unavailable，不补造横向数值，也不把旧失败当作可比性能基线。

## 3. 结果、动作与成本

| Sample | Mode | Result | Requests | Tools | Patch max/request | Multi request reject | Multi-file patch | State failures | Map nodes/open | Wall | Input | Uncached | Req2+ cache |
|---|---|---|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|
| order | Standard | complete/solved | 9 | 14 | 1 | 0 | 1 | 0 | N/A | 50.52s | 89,689 | 12,121 | 90.94% |
| order | R5 | complete/solved | 18 | 14 | 1 | 0 | 1 | 4 | 9/4 | 85.53s | 220,066 | 10,402 | 95.21% |
| billing | Standard | complete/solved | 14 | 21 | 1 | 0 | 0 | 0 | N/A | 54.24s | 162,539 | 12,395 | 94.98% |
| billing | R5 | complete/solved | 7 | 19 | 4 | 1 | 1 | 0 | 3/3 | 44.58s | 71,872 | 9,408 | 86.21% |

说明：billing R5 的旧 `metrics.json` 在 rollout 已有 `task_complete` 时错误标记 incomplete。CoE
`coe/2026-07-13-04-24-r5-benchmark-final-evidence-mismatch.md` 证明不是采集竞态，而是 extractor 漏读正式
lifecycle event。`bb69813` 修复后对原 artifact 重算为
`agent_final_observed=true/source=task_complete_event/actionability=final_candidate`。原 artifact 不静默改写。

单次成本方差方向相反：order R5 requests 为 Standard 的 `2.00x`、input `2.45x`、wall `1.69x`；billing
R5 分别为 `0.50x`、`0.44x`、`0.82x`。样本数为 1，且两条 Agent 路径不同，不能归因于工具描述改动。
两组 R5 uncached input 分别为 Standard 的 `0.86x` 和 `0.76x`，同样只作机械观察。

## 4. Gate 逐项验收

### 4.1 Correctness：5/7

| Gate | Result | Evidence |
|---|---|---|
| Standard/R5 最终正确 | PASS | 两组 public/hidden validator 均为 0；billing lifecycle 原证据重算 complete |
| validation failure workspace hash 不变 | PASS | J7.1 transaction/fault fixtures；本轮 partial commit=0 |
| request-wide multi-patch executed=0 | PASS | billing R5 preflight reject=1，executed=0；其他侧无非法执行 |
| R5 multi-patch carrier accepted/generated=0 | PASS | carrier multi-patch=0；非法声明来自 active top-level siblings |
| protocol/state failure=0 | **FAIL** | order R5 state failure=4 |
| permission/sandbox/hook/cancel/raw feedback 无回退 | PASS | J7.3 shared router/security regression；live raw reject 可恢复 |
| Map node/edge/result health 不下降 | **FAIL** | order 9 nodes/open 4；billing 3 nodes/open 3 |

### 4.2 Benefit：6/7

| Gate | Result | Evidence |
|---|---|---|
| Standard/R5 patch max/request=1 | **FAIL** | billing R5 首次为 4；门禁拒绝后才修正 |
| 同一 carrier patch slot max=1 | PASS | schema 与 observer 均无 multi-patch carrier |
| 前 patch 失败导致后续 patch skipped=0 | PASS | 两组两侧均为 0 |
| 相关多文件修改由一个 prepared patch 表达 | PASS | order 两侧各 1；billing R5 恢复后 1 个四文件 patch |
| `finish + patch + test` 能力保持 | PASS | J7.3 sequence fixture；live post-patch action adoption=0，不冒充采用收益 |
| request/token/cache/wall 完整分账 | PASS | 两份 performance observation 覆盖率 100% |
| 不以少读、少测或 Map 坍缩制造收益 | PASS | 不声明成本因果；读取/pytest 未设 gate；Map 缺口显式记失败 |

## 5. 已验证工程收益

| Benefit | Baseline | Observed | Status |
|---|---|---|---|
| multi-patch 批次零副作用 | J6.7 曾出现 4 success + 1 failure 的部分提交 | 本轮非法 4-patch executed=0，partial=0 | verified |
| 多文件 patch validation 原子性 | 旧实现可在后序 validation failure 后保留前序写入 | 全量 prepare 先于 commit；failure hash fixture 不变 | verified |
| 忠实、可恢复的硬错误 | 旧路径存在 partial + skipped 恢复负担 | Agent 读取明确 reason 后下一请求合并为一个 patch | verified |
| patch cadence 成本下降 | 无稳定基线 | order 回退、billing 改善，方向不一致 | not verified |
| Agent 首次遵守 singular request | 旧复杂样本多 patch | order 遵守，billing 未遵守 | not verified |

## 6. 未关闭项与影响

| Item | Reason | Impact | Next action |
|---|---|---|---|
| 首次 response 仍可能声明多个 patch | 单个 tool JSON/Lark schema 无法约束兄弟 tool-call cardinality；描述增强仍非硬约束 | 多 1 次拒绝与恢复 request，但无副作用 | 保留 request preflight；跨复杂样本继续观察，不增加 reasoning/prompt 解析 |
| order 状态机失败与 map 膨胀 | Agent 重复 finish 已完成节点，并用 draft next node 重建已存在阶段 | J7.5 correctness gate 不通过，request 增长 | 作为 J8/后续 TaskSpace tool 可用性问题单独诊断，不在 patch runtime 增加语义干预 |
| R4 同口径基线缺失 | 历史 artifact 不满足当前 Docker/observer 合同 | 不能量化 R4→R5 收益 | 明确 unavailable；不为补表重建旧产品路径 |
| live `finish+patch+test` 未采用 | unit 能力通过，但两个样本 post action=0 | 只能声明能力保留，不能声明 cadence 收益 | 后续复杂样本继续观测 |

## 7. 首次决定（已被第9节取代）

J7.0-J7.4 保持 complete；J7.5 已执行但 gate paused。J7 不因模型偶发遵守或外部验证通过而关闭。
后续应先把本轮暴露的 TaskSpace control 可用性/Map 生命周期问题纳入下一阶段诊断，再决定是否复验 J7.5；
不得通过 runtime 自动 finish、自动合并 patch 或语义化 projection 追求表面通过。

## 8. J7.6 复验补充（2026-07-13）

J7.6 恢复 control committed identity 并收敛输入 schema 后，order R5 从18 requests/4 state failures/9 nodes
4 open 收敛到9/1/3/0；billing R5 为15 requests/0 state failures/5 nodes/0 open。两组 success identity
missing=0、repeat committed finish=0。Map health gate 恢复，但 order 新出现一次 terminal self-loop reject；
billing Standard 仍产生一次 multi-patch request。因此 J7.5 更新为12/14，保持 paused。完整证据见
`40-r5-j7-6-control-contract-fidelity-result.md`。

## 9. J7.7-J7.8 最终复验（2026-07-13）

### 9.1 证据与资格

| Sample | Evidence | Eligibility |
|---|---|---|
| `multi-file-order-pipeline` | `target/r5-j7-8-order/multi-file-order-pipeline/20260713-192050-364` | valid single-run diagnostic；两侧 complete/solved |
| `subscription-billing-repair` | `target/r5-j7-8-billing/subscription-billing-repair/20260713-192050-364` | valid single-run diagnostic；两侧 complete/solved |

runner 因 `Repeats=1` 固定报告 `repeats_lt_3,aggregate_not_enabled`，因此不进入统计 utility aggregate；本轮是计划
要求的单次结构验收，pair audit、public/hidden validator、rollout、provider cache trace 和 Map artifact 均完整。
R4 仍无同 revision、同 Docker contract、同 observer 口径 artifact，继续标记 unavailable。

### 9.2 结果、动作与成本

| Sample | Mode | Requests | Runtime tools | Controls | State failures | Patch max/request | Map nodes/open | Wall | Input | Cached | Uncached | Output | Req2+ cache |
|---|---|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|
| order | Standard | 10 | 15 | 0 | 0 | 1 | N/A | 52.81s | 98,944 | 90,240 | 8,704 | 5,726 | 90.70% |
| order | R5 | 7 | 12 | 2 | 0 | 1 | 4/0 | 44.87s | 67,906 | 60,416 | 7,490 | 5,304 | 88.33% |
| billing | Standard | 12 | 19 | 0 | 0 | 1 | N/A | 45.18s | 133,631 | 128,128 | 5,503 | 4,869 | 95.73% |
| billing | R5 | 17 | 19 | 6 | 0 | 1 | 5/0 | 64.22s | 233,317 | 222,848 | 10,469 | 6,880 | 95.46% |

order R5 相对 Standard：requests `0.70x`、wall `0.85x`、input `0.69x`、uncached input `0.86x`。billing
R5 相对 Standard：requests `1.42x`、wall `1.42x`、input `1.75x`、uncached input `1.90x`。两组均无 zero-hit
或 same-shape-zero；R5 只保留 bootstrap `named -> auto` 的一次 tool-choice/cache-shape 转换。成本方向相反，
不声明稳定成本收益。

### 9.3 最终门禁：14/14

| 类别 | Gate | Result | Evidence |
|---|---|---|---|
| Correctness | Standard/R5 最终正确 | PASS | 两组 public/hidden validator=0，Agent complete，external passed |
| Correctness | validation failure workspace hash 不变 | PASS | J7.1 fault fixtures；本轮 partial commit=0 |
| Correctness | request-wide multi-patch executed=0 | PASS | 四侧 multi request=0 |
| Correctness | R5 multi-patch carrier accepted/generated=0 | PASS | 两组 carrier attempt=0 |
| Correctness | protocol/state failure=0 | PASS | 两组 R5 均为0 |
| Correctness | permission/sandbox/hook/cancel/raw feedback 无回退 | PASS | shared router/security regression 与 Docker execution clean |
| Correctness | Map node/edge/result health 不下降 | PASS | 4/5 nodes 均各有 result，open=0，task completed；无相对 J7.6 edge 回退 |
| Benefit | Standard/R5 patch max/request=1 | PASS | 四侧均为1 |
| Benefit | 同一 carrier patch slot max=1 | PASS | multi carrier=0 |
| Benefit | 前 patch 失败导致后续 patch skipped=0 | PASS | 四侧均为0 |
| Benefit | 相关多文件修改由一个 prepared patch 表达 | PASS | order 两侧、billing 两侧均有 multi-file prepared patch |
| Benefit | `finish + patch + test` 能力保持 | PASS | J7.3 sequence fixture；live adoption 不作为门禁替代物 |
| Benefit | request/token/cache/wall 完整分账 | PASS | performance observation coverage=100% |
| Benefit | 不以少读、少测或 Map 坍缩制造收益 | PASS | 两组外部验证通过；Map闭合且节点/结果一一对应 |

### 9.4 J7.7/J7.8 专项反馈门禁

| Signal | order R5 | billing R5 | Result |
|---|---:|---:|---|
| terminal chain duplicate | 0 | 0 | PASS |
| identity missing / committed repeat finish | 0 / 0 | 0 / 0 | PASS |
| `map_state` present / missing | 2 / 0 | 6 / 0 | PASS |
| open-node visibility | 1 | 5 | PASS |
| terminal failure nonzero commit | 0 | 0 | PASS |
| terminal extra request | 0 | 0 | PASS |

billing 仍有4次 `finish_nodes` 未携带 sibling ordinary action，形成额外 request 成本；它没有造成状态失败、
语义丢失或 Map 未闭合，按既定边界只记录为 Agent/tool cadence 效率观察，不新增 Runtime 语义约束。

两组 R5 的 Map 均为多节点但无 dependency edge，observer 保留 `multi_node_map_without_edges` 警告。J7.6
同样本基线也是0 edge，因此不构成 J7.5 的 Map health 回退；它仍是后续 G3 应跨复杂样本观察的 Agent Map
建模质量信号，Runtime 不自动补边。

### 9.5 最终决定

J7.0-J7.8 全部完成，J7.5 以14/14关闭。后续路线解锁为 `R5-K -> R5-G3 final regression -> R5-H closeout`。
