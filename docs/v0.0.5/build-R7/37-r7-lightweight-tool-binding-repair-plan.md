# R7 普通 Tool 轻量绑定与连续动作修复

> 状态：轻量绑定闭环修复已实现；工程、Docker 与独立复审待最终确认
>
> 日期：2026-07-24
>
> 范围：FLA L2/L4、普通 Tool provider schema、Tool sequence、ActionMap runtime 与观测
>
> 对应 COE：`coe/2026-07-23-18-24-r7-fla-tool-schema-amplification.md`

## 1. 修复边界

当前生产基线把完整 `taskspace_action` 四分支联合复制到每个普通 Tool。它把初始化 Root、Work、Finish、
edges 等完整结构重复暴露约 12 次，使 TaskSpace 首请求 Tool section 从 Standard 的 `21,669` bytes
放大到 `60,743` bytes。

普通 Tool 与 TaskSpace 的机械关联仍是必要合同。本轮只删除重复的状态内容，不删除关联本身：

- Agent 继续明确声明普通动作沿用当前 binding，或承接紧邻的边界 control；
- Runtime 继续强制 Map、binding、lease、reservation 和顺序硬约束；
- Runtime 不选择节点、不推断下一动作、不修改普通 Tool 参数或结果；
- 生命周期参数只在唯一 `taskspace_control` 中精确暴露一次。

## 2. Agent 可见合同

### 2.1 普通 Tool 的唯一扩展

TaskSpace 下每个普通 Tool 增加一个必填字符串字段：

```json
{
  "taskspace_binding": "active | after_boundary"
}
```

字段只表达调用序列中的机械关系：

| 值 | 含义 |
|---|---|
| `active` | 此动作服务当前 canonical active Work binding |
| `after_boundary` | 此动作紧邻前一个边界 `taskspace_control`，服务该 control 新建立的 binding |

字段不携带 revision、node id、目标、图结构或生命周期参数。Router 在调用普通 Tool handler 前机械移除
该字段，普通 Tool 的原生参数和结果保持不变。

Standard 不暴露该字段，也不把同名业务字段识别成 TaskSpace binding。外部 Tool 合法拥有
`taskspace_binding` 业务参数时，Router 原样转发，是否接受由该 Tool 自己的原生 schema 和 handler 决定。

### 2.2 边界 control

以下三个 action 以完整精确 schema 只在 `taskspace_control` 暴露一次：

- `initialize_map`
- `bind_node`
- `complete_then_continue`

其他 graph、read、block、rework 和 `finish_map` action 继续由同一个 control Tool 承载。

## 3. 连续动作硬合同

Tool sequence preflight 在任何调用执行前检查完整 provider response：

1. 边界 control 后必须立即出现一个普通 Tool，且其 `taskspace_binding` 为 `after_boundary`；
2. `after_boundary` 普通 Tool 前必须立即是边界 control；
3. 其余普通 Tool 必须使用 `active`；
4. 任一不匹配使整份 response 零执行拒绝，不允许先提交 control 再发现缺失动作；
5. `finish_map` 是终态 control，不要求后续动作；
6. 现有单 response 最多一个 patch 约束保持不变。
7. control JSON、action 类型和保留字段等机械错误也在整响应 preflight 中拒绝；
8. 只有收到 `response.completed` 的完整响应才能进入 preflight 和执行；mailbox 抢占不能取得含
   pending Tool calls 的响应所有权。

合法：

```text
initialize_map + exec(after_boundary)
complete_then_continue + apply_patch(after_boundary) + exec(active)
complete_then_continue + read(after_boundary) + complete_then_continue + exec(after_boundary)
```

非法：

```text
initialize_map
initialize_map + exec(active)
exec(after_boundary)
complete_then_continue + taskspace_control
```

## 4. 执行与反馈

`taskspace_control` 和 `apply_patch` 继续是 sequence barrier：

1. 边界 control 先执行并提交 Agent 明确声明的状态变化；
2. control 成功后，紧邻普通动作在新 canonical binding 下执行；
3. control 失败时，后续声明调用全部返回 `skipped_due_to_prior_failure`；
4. 普通动作失败时，已提交的 control 不自动回滚；
5. control 结果和普通 Tool 原始结果作为两个有序事实进入上下文，不合并、不再解释；
6. 同一普通并行段只包含彼此不依赖返回值的动作；有结果依赖的动作等待下一次 provider response。

ToolSearch 的 provider 配对输出必须保留协议要求的 `status=completed`。它不再被当成业务成功状态：
Runtime 独立保留真实 `succeeded`，错误时额外返回包含原始错误文本和 call id 的
`ToolSearchFailureV1` 事实，后续依赖 segment 按失败处理。

## 4.1 Tool 形态统一策略

TaskSpace provider visibility 和 ToolSearch 延迟加载结果共用同一个 binding 投影：

| Tool 形态 | TaskSpace 行为 |
|---|---|
| Function / Namespace member / ToolSearch | 保持原业务 schema，仅增加轻量 binding |
| ToolSearch 返回的 Function / Namespace | 使用同一投影后再返回 Agent |
| apply_patch / code mode Freeform | 投影为等价 Function，保留原 raw input/source |
| taskspace_control | 保持中央 lifecycle schema，不增加 binding |
| LocalShell / WebSearch / ImageGeneration / 未知 Freeform | 因不能进入客户端整响应 preflight，在 TaskSpace 确定性隐藏并记录 |

即使 provider 伪造已隐藏的 Custom 或 LocalShell payload，Runtime 仍会在整响应 preflight
以 `taskspace_tool_shape_unsupported` 零执行拒绝。此策略只按可否进入机械预检分类，不根据任务语义
替 Agent 选择 Tool。

schema 已包含保留字段时，投影返回 `TaskSpaceToolProjectionError`，prompt 构建确定性失败并记录
Tool 名与字段；不再使用 panic，也不静默覆盖业务字段。

## 5. 删除项

- 删除普通 Tool 上的完整 `taskspace_action` 联合；
- 删除 `continue_current` 的 revision/node 重复提交；
- 删除普通 Tool 内执行 lifecycle transition 的 carrier executor 和复合结果 envelope；
- 删除独立 `taskspace_transition: { action, arguments }` 通用对象方案；
- 不保留旧字段兼容 parser、fallback 或 feature flag。

## 6. 验收门

### 6.1 结构

- 每个普通 Tool 只增加一个两值字符串字段；
- Root、edge、Finish、revision、node id 不出现在该字段；
- 三个边界 action 的完整参数只在 `taskspace_control` 出现一次；
- Standard 普通 Tool schema 不变；
- TaskSpace 普通 Tool 仍经过 canonical binding、lease 和 reservation gate。

### 6.2 连续动作

- 三类边界与 `after_boundary` 动作可在同一 response 连续声明；
- 任一单独边界、孤立 `after_boundary` 或错误配对均零执行拒绝；
- 合法 response 中 control 先于普通 Tool；
- control 失败阻止后续动作，control 成功而 Tool 失败保留两项事实；
- 支持同一 response 中多个合法边界动作对。

### 6.3 成本与行为

- 普通 Tool 上重复的 TaskSpace 扩展字节相对完整生命周期联合至少降低 80%；
- TaskSpace Tool section 总字节应显著低于旧 `60,743` bytes，并单独记录中央
  `taskspace_control` 与普通 Tool 扩展的占比，不以不可达阈值替代实测；
- 普通 Tool 重复扩展不得包含生命周期联合；
- Standard、map-always、map-append、map-request 使用同一二进制和 Docker image；
- 三个 TaskSpace arm 不出现生命周期参数 shape 猜测；
- 记录 request、input、cached、output、时间、Tool bytes、配对率、拒绝码和 Map 闭合状态。

## 7. 非目标

- 不修改三种 projection policy；
- 不自动创建 Root、Work 或 Finish；
- 不让 Runtime 根据 Tool 类型选择节点或状态转换；
- 不实现 nested all-tools dispatcher；
- 不改变 DAG、terminal、revision、lease 或 reservation 领域规则。

## 8. 实施结果

最终实现没有采用“普通 Tool 零侵入”，也没有恢复完整 lifecycle 联合：

- TaskSpace 普通 Tool 仅增加必填 `taskspace_binding=active|after_boundary`；
- `taskspace_control` 是 lifecycle 参数的唯一 schema owner；
- Router 在普通 handler 前机械移除 binding，不改变业务参数；
- response 级 preflight 对 boundary/`after_boundary` 做双向紧邻校验，失败时整份 response 零执行；
- preflight 反馈包含实际 Tool 序列和机械期望序列，不补动作、不选节点；
- control 与 ordinary Tool 结果保持两个独立有序事实；
- Standard schema 不经过装饰；
- Standard Router 不提取或删除同名业务字段；
- ToolSearch 返回的延迟 Tool 与初始 prompt Tool 使用同一 binding 投影；
- 不能参加客户端 preflight 的 provider-native Tool 在 TaskSpace 明确隐藏；
- mailbox 只在 pending Tool calls 为空时允许抢占，未完成响应的 Tool 前缀永不执行；
- ToolSearch 失败的 provider 配对状态与真实执行成功状态分离；
- Ready、BuildFailed 与 RejectedNative 共用完整 provider response declaration 序列；
- 任一 build failure 或隐藏 native event 都使同一 response 的 client Tool 全部零执行；
- client ToolSearch 即使缺少 call_id 也作为无配对 build failure 保留，不得降级为非 Tool；
- 所有 call pairing output 先于 ToolSearch 或 response-level factual message；
- hidden WebSearch/ImageGeneration 的 added/done event 都在非 Tool 处理和本地落盘前拒绝并去重；
- 完整 carrier executor、复合结果 envelope 和旧 parser 已删除，不保留兼容路径。

初始化 `edges` 的 schema 还明确写入 Root/Finish 硬图规则。该修复只陈述已存在的状态机不变量，不让
Runtime 生成或修复图。

## 9. 验证结果

### 9.1 工程验证

| 验证 | 结果 |
|---|---:|
| `cargo test -p codex-tools taskspace --lib` | 12/12 |
| `cargo test -p codex-core taskspace --lib --no-fail-fast` | 101/101 |
| `cargo test -p codex-core --test all taskspace_terminal_contract --no-fail-fast` | 2/2 |
| mailbox 未完成响应前缀集成测试 | 1/1 |
| build-malformed suffix 整响应零执行 SSE | 1/1 |
| client ToolSearch missing call_id（有/无 provider item id） | 2/2 variants |
| hidden native added/done 零落盘 | 2/2 |
| TaskSpace deferred search -> invoke | 1/1 |
| Standard business field 实际 dispatch | 1/1 |
| response sequence / pairing 顺序 | 18/18 |
| 五层合同 `-Phase All` | 通过 |
| trace/cost/performance observer 自测 | 通过 |
| observer Skill 校验 | 通过 |
| `cargo check -p codex-tools -p codex-core` | 通过 |
| `cargo build -p codex-cli --bin whale --locked` | 通过 |

### 9.2 固定 schema 成本

同一 DeepSeek Docker harness 的最终 wire Tool section：

| 合同 | Tool bytes | estimated tokens | 相对 Standard 的额外 bytes |
|---|---:|---:|---:|
| Standard | 21,669 | 5,418 | 0 |
| 历史完整 carrier | 60,743 | 15,186 | 39,074 |
| 最终轻量 binding | 29,200 | 7,300 | 7,531 |

最终实现把相对 Standard 的额外 Tool schema 从 `39,074` bytes 降到 `7,531` bytes，下降约
`80.7%`。完整 lifecycle 结构只在中央 control 出现一次；普通 Tool 的 binding schema 设有独立字节上限回归。

最终 map-request 自然样本：

- 业务验证与 hidden oracle 均通过；
- 12 个 provider request，input `201,901`、cached input `174,592`、request-2+ cache
  `89.41%`；
- Map 为 5 nodes / 4 edges，Root、Finish 均闭合；
- 3 个合法 boundary pair 全部按 control -> ordinary Tool 顺序执行；
- 首次 `initialize_map` 仍有 1 次单独调用并被零执行 preflight 拒绝；
- Agent 还曾过早 `finish_map`，收到状态机事实后主动 `read_map` 并完成剩余 Work。

该 repeat-1 只证明最终合同可运行、可恢复和成本收敛，不用于选择 projection policy。

## 10. 明确未决边界

单个 Tool 的 JSON Schema 不能结构化要求同一 provider response 中存在另一个 sibling Tool。实测也证明，
再增加固定 `next_call="ordinary_tool"` 只会形成重复声明：Agent 能填该字段，仍可能单独结束 control
response，因此该试验已删除。

当前轻量方案对连续动作提供三层配合：

1. L2 描述工作协议；
2. control 与 ordinary Tool 两侧都暴露机械配对关系；
3. Runtime preflight 严格保证非法序列零执行并忠实反馈。

这保证了状态不会被错误提交，但不能保证 Agent 首次一定生成 sibling。若后续要求“provider schema 本身绝对
不能表达单独 boundary”，必须单独决策单调用复合 Tool 的产品形态及其 patch、MCP、schema 成本，不得继续
堆提示词、固定字段或 Runtime 语义纠正来假装获得结构保证。

## 11. Docker 运行经验

- `run-r7-five-layer-matrix.ps1` 会从 `.env.local` 导入 `DEEPSEEK_API_KEY`；直接调用基础 runner
  时需在子进程环境显式加载，否则 credential preflight 会在模型调用前退出；
- 中立 benchmark 根目录不能包含 `taskspace`、`standard`、`map` 等 treatment 词，使用
  `/tmp/whale-paired-bench-runs/<run-id>/a0..a3`；
- 无效 credential preflight run 与有效 Agent run 使用不同目录，禁止混入聚合。
