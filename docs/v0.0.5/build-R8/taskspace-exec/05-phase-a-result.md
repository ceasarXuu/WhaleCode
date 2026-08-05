# TaskSpace Exec Phase A 结果

- Created: 2026-08-05
- Status: Completed with A2 no-go
- Runtime activation: None
- Paid Whale Agent runs: 0

## 1. 结果总览

| Unit | 结果 | 证据 | 结论 |
|---|---|---|---|
| TX-01 | 通过 | `9ecbbf257`、[`04-phase-a-discovery.md`](04-phase-a-discovery.md) | 当前生产、上游 seam 和旧协议删除范围已固定 |
| TX-02 | 通过 | `2cd2a26b2`、5 个 catalog tests | Function 外壳和完整 capability identity 可由同一 ToolSpec 快照派生；碰撞 fail closed |
| TX-03 | A1 通过 | `5c32b092f`、5 个 decoder tests | `taskspace.plan(<strict JSON>);` 在任何副作用前形成唯一 typed plan；动态 JS 不可进入 |
| TX-04 | 通过 | `f734b68a1`、5 个 preflight tests | 结构、Tool/input、node、Map 边界、递归和单 Patch 在 dispatch 前机械判断 |
| TX-05 | A2 未通过 | `446ac164b`、4 个 reconciliation tests、静态链路审计 | exact-set 算法可行，但 Agent-visible、持久化和 replay identity 链路不完整 |

新增代码全部位于未注册的 `core/src/tools/taskspace_exec/`，没有 handler、Router 注册、请求投影或 provider payload 改动。
Standard 和当前 TaskSpace 生产行为均未改变。

## 2. A1：source 与预检

Phase A 选择的候选 source 不是可执行 JavaScript，而是单一声明：

```text
taskspace.plan(<strict JSON>);
```

JSON 一次表达：

- `version` 和当前 `capability_id`；
- client/map call 的 `item_id`、原生 Tool 名、原生 input 和 Agent 声明的 `node_id`；
- provider hosted record 的 response、type、item 和 node identity。

变量、函数调用、条件、循环、`await`、Markdown fence、未知字段和尾随语句均被拒绝。Runtime 因而不需要解析 reasoning，
也不会像 Code Mode 那样边执行 source 边发现后续计划非法。

当前 preflight 负责可在纯输入上判断的硬合同。节点是否存在、revision 是否等于 canonical Map、DAG 和状态转换是否合法，
必须在 Phase B 通过现有 Map validator 接入；不允许在 preflight 中复制第二套状态机规则。

## 3. A2：Hosted 为什么未通过

Runtime 对已知 hosted output 可构造的最小稳定引用是：

```text
(response_id, provider_item_type, provider_item_id)
```

`status` 属于事实正文，不属于身份；Tool 成败也不推进节点状态。Phase A 的 exact-set 实现能稳定发现漏绑、错类型、重复、
伪造、缺 item ID 和缺 response scope，且没有顺序、URL 或内容猜配。

问题发生在身份链路，不在集合算法：

1. Web Search 的 `id` 是 `Option<String>`，当前 Standard rollout 和 inference trace 序列化会丢失该字段；
2. 当前 Action Map event 虽保存 provider item ID，却没有持久化 `response_id`，restart replay 不能恢复原 response scope；
3. 未识别 hosted 类型被归一化为 `ResponseItem::Other`，原 type/ID/status 丢失；
4. 已有两次真实探针中，Agent 均未能回显 provider item ID；因此不能要求 Agent 在同一响应的 `taskspace_exec` 中稳定
   写出 Runtime exact key；
5. Image Generation 已知结构要求 `result`，失败 output 若不含结果可能在 reconciliation 前解析失败。

所以“Runtime 看得到 output item ID”不等于“Agent 能在同一响应可靠声明该 ID”。继续生产接线会把漏绑变成常态；改用
同类顺序、query、URL 或语义相似度会违反已确认的 no-guess 合同。

## 4. 需要用户确认的产品取舍

### D1：Hosted 精确绑定时机

推荐改为**延迟精确绑定**：Provider 完成后由 Runtime 生成并持久化 opaque `provider_fact_ref`，下一次模型请求忠实暴露
原结果和引用；Agent 在下一次 `taskspace_exec` 中显式选择节点。Runtime 只核对引用和节点，不自动绑定。

收益：支持同响应多个 hosted 动作绑定不同节点，完整 restart replay，保持 no-guess 和 Agent 决策权。代价：Hosted 绑定
不能在原响应内完成，通常增加一次模型请求；provider 事实在绑定前明确保持 `unbound`。

备选是把同一 response 内全部 hosted 事实限制为一个 Agent 预声明节点。它减少一次请求，但无法表达同响应多节点，且会
人为限制 provider 原生并行能力，不推荐。

### D2：声明式 source 作为 Phase B 基线

建议保留 `taskspace.plan(<strict JSON>);`。它保持 `{source:string}` Function wire，同时用声明而非执行引擎满足整批预检。
已知代价是模型生成体验尚未经过真实样本验证；该验证属于 TX-15，需另行申请预算。若改成 Function 参数直接承载大型
结构化 plan，可减少一层 wrapper，但会显著放大顶层 JSON Schema，并偏离已验证的 Function Exec 使用形态。

## 5. 后续计划调整

1. TX-06 前先对齐 Codex 最新 `spec_plan/ToolExposure` seam，再把 Phase A catalog 变成唯一 effective snapshot；不在旧
   `spec.rs` 上建立长期平行 catalog。
2. `taskspace_control.actions[]` 属于旧 sibling 复述。新 exec 已承载 call 和 node binding，Phase B 必须设计 Map-only
   control 合同，并在 Phase C 与旧生产协议原子切换；不能让两份 actions 长期并存。
3. TX-04 的 production adapter 只调用 canonical Map validator；Phase A preflight 不扩展成第二状态机。
4. TX-09 在 D1 确认后改写为 provider fact collector、持久化引用和延迟 binding settlement；未经确认不实施。
5. TX-12/13 仍保持原子切换：registry、request projection、L2 contract、response executor、terminal feedback 和旧 observer
   同批迁移，避免双协议。

## 6. 离线验证

| 命令 | 结果 |
|---|---|
| `cargo test -p codex-core taskspace_exec --lib` | 22 passed |
| `cargo test -p codex-core taskspace_hosted_binding_contract --lib` | 5 passed |
| `cargo test -p codex-tools code_mode --lib` | 14 passed |
| 每次 staged `check_cache_regression_gate.py --source index` | PASS；final wire 未变化 |

本阶段没有真实 Whale Agent run，因此没有新增 run ledger 记录或 API 成本。A2 决策前不得进入 Phase B 生产组件接线。
