# TaskSpace Exec 闭集合法序列设计

- Status: Draft / product review required before implementation
- Created: 2026-08-12
- Product Authority: [`00-product-contract.md`](00-product-contract.md#confirmed-product-decisions)
- Applicable Decisions: PD1, PD2
- Scope: 只重构 Agent-visible TaskSpace Exec 序列表达；不改变 Map、普通 Tool、Router、Hosted 原始事实或 Standard

## 1. 目标

当前 `taskspace_exec` 让 Agent 在通用 `calls[]` 中任意排列 Map 与 client call，再由 Runtime 事后判断组合是否可执行。目标模型改为：

1. Agent 每次只能选择一个命名的合法序列类型；
2. 每种类型只暴露完成该场景所需的字段；
3. Runtime 将合法序列机械归一化为 canonical Map transaction 和原生 client Tool 调用；
4. 动态 DAG、节点状态、Tool 参数、Hosted 对账和单 Patch 仍由 Runtime 硬校验；
5. 新场景逐个加入，每次都能独立验证收益和回归。

这不是让 Runtime 选择计划。Agent仍然决定序列类型、节点、完成事实、Tool、参数、归属和结束时机。

## 2. 当前证据

### 2.1 已观察形状

近期 R8 rollout 的可解析 TaskSpace Exec 调用主要为：

| 形状 | 观察次数 | 判定 |
|---|---:|---|
| `initialize_map + exec_command` | 10 | 稳定合法场景 |
| 单独 `exec_command` | 15 | 稳定合法场景 |
| `completed + finish_map` | 6 | 稳定合法场景 |
| `completed + apply_patch` | 5 | 稳定合法交接 |
| `completed + exec_command` | 4 | 稳定合法交接 |
| `read_map` | 1 | 合法独占场景 |
| `completed + in_flight + apply_patch` | 3 | 非法；误把派生子节点显式推进 |
| `apply_patch + exec_command` | 3 | 只有无结果依赖时可并行；样本中常被误作串行 |
| `completed + apply_patch + exec_command` | 2 | 跨两层依赖时非法 |

统计用于发现场景，不代表稳定性概率；无效 JSON 和旧实验 wire 未计入。

### 2.2 waiting 复发揭示的表达缺口

正确交接实际上已可写为 `update_map(parent=completed) + client(child)`，但它不是 schema 中的一等动作。Agent必须从通用字段自行推导：

1. 当前工作语义上已经完成；
2. 需要显式完成父节点；
3. 子节点 readiness 由 parents 派生；
4. 不要再写子节点 `in_flight`；
5. 只能执行本轮已经解锁的 work，不能把其 Tool outcome 当成同批下一层解锁条件。

这组规则被文字和示例描述，却没有被输入结构直接表达。

### 2.3 当前时序模型不统一

- 外层 `calls[]` 中 Map operation 按位置更新候选 Map；
- client calls 在完整预检后作为没有额外顺序承诺的集合交给原生并行 Runtime；
- 单个 `update_map.node_patches[]` 全部应用后才统一推导 readiness。

因此“数组顺序”在三个位置具有不同含义。闭集模型应直接表达 Map 边界和 work 集合，不再要求 Agent理解这些内部差异。

### 2.4 `in_flight` 不是当前 client work 的自然结果

当前生产链会把 client/Hosted 动作及其 outcome 记入节点 `actions[]`，但不会改变节点状态。Agent 只有
额外提交 `update_map(state=in_flight)` 才能让 `ready` 节点进入 `in_flight`。这使“在某节点执行 work”与“该节点
已开始执行”成为两份可以不一致的 Agent 声明。

闭集序列可以消除这一重复：Agent 选择在某 `node_id` 上执行 work，可以被视为对“启动该节点”的显式
声明，Runtime 只机械生成 `ready -> in_flight` transaction，对已是 `in_flight` 或 `blocked` 的节点不暗中改变状态。
这不是根据 Tool
结果推理状态，但会改变
现有“work 与状态完全分开声明”的产品行为，因此只能在用户确认后落地。

## 3. 设计原则

1. **闭集**：没有 generic/raw/custom/other/calls 逃生分支。
2. **判别明确**：每个序列有唯一 `type`，分支字段互斥，未知字段拒绝。
3. **场景优先**：序列名表达 Agent 当前要完成的工作阶段，不暴露 Runtime内部执行步骤。
4. **Tool 无侵入**：client Tool 的 name/input 继续从原生 ToolSpec 派生，`node_id` 仍在 TaskSpace 外层。
5. **Map 无语义推断**：Agent显式声明完成、重开、结束或结构调整；Runtime只机械归一化。
6. **work 不建第二 DAG**：同一 `client_work[]` 不表示依赖顺序；结果依赖动作必须等待后续请求。
7. **结构与动态分层**：schema/decoder限制序列形状，preflight结合当前 Map判断节点是否真实可执行。
8. **直接替换**：不兼容旧 `calls[]`，不双写、不保留旧 decoder 或 fallback。

## 4. Agent-visible 外层

推荐使用一个根对象和一个判别联合：

```json
{
  "sequence": {
    "type": "handoff_and_work",
    "completed": [
      {
        "node_id": "inspect",
        "content": "已定位税费舍入错误"
      }
    ],
    "client_work": [
      {
        "client": {
          "name": "apply_patch",
          "node_id": "fix",
          "input": "*** Begin Patch..."
        }
      }
    ]
  }
}
```

### 4.1 Schema 形态

- `sequence` 使用 nested `anyOf`，每个分支要求唯一 exact `type`；各分支只列出该场景合法字段，
  `additionalProperties=false`。
- 使用 `anyOf` 而不是 `oneOf`：exact `type` 已让分支互斥，避免 `oneOf` 的全分支 XOR 校验成本。
- `client_work[]` 的 client variant继续由 request-local Catalog 从原 ToolSpec 机械生成，并作为一份共享 definition 被
  各 work-bearing branch 引用；不得在 S1/S2/S3/S4/S6/S7 中重复展开整份 Tool catalog。
- `hosted_work[]` 是已由 Provider 执行的事实及节点归属，只出现在明确允许 Hosted 的 sequence branch 内；不作为
  游离于合法序列之外的顶层逃生字段。
- 共享 definition 只是静态 schema 去重，不是运行时能力查找。当前 Rust `JsonSchema`、本地 validator 和 ToolSpec
  parser 已支持 local definitions/references；DeepSeek 官方 strict schema 也明确支持 `anyOf` 和 `$ref` + 可复用
  definition。但官方文档写作 `$def`，当前共享 Rust 类型序列化为标准 JSON Schema `$defs`；LS-01 必须
  用 Provider final-wire fixture 冻结唯一形式，不修改 Standard，不保留双写。
- DeepSeek Function Call 仍可能生成无效 JSON或未遵守 schema，Runtime typed decoder 与 preflight继续是权威边界。

JSON Schema 官方将 `anyOf` 定义为至少匹配一个分支，将 `oneOf` 定义为恰好匹配一个分支，并提醒 `oneOf` 需要验证全部分支；本设计通过
唯一判别值让 `anyOf` 分支天然互斥。[JSON Schema composition](https://json-schema.org/understanding-json-schema/reference/combining)

### 4.2 固定成本边界

闭集的成本应近似“一份原生 Tool catalog + 若干个小型序列骨架”，而不是“序列数 × 整份 Tool catalog”。LS-01
必须对 final wire 进行按模块字节统计；若 Tool catalog 被重复展开，直接视为设计失败，不进入 Provider 验证。

## 5. 首批序列候选

以下是设计候选，不在用户复核前升级为最终产品合同。

| ID | `type` | 必填内容 | 用途 | 当前证据 | 建议 |
|---|---|---|---|---|---|
| S1 | `initialize_and_work` | `initialize`，至少一项 client/Hosted work | 创建根/Work/Finish并立即工作 | 真实 trace 高频 | 首批 |
| S2 | `work` | 至少一项 client/Hosted work | 推进一个或多个当前可执行节点 | 真实 trace 高频 | 首批 |
| S3 | `handoff_and_work` | 非空 `completed`，以及至少一项后续 client/Hosted work | 基于已有结果完成前置节点并继续 | waiting 修复核心场景 | 首批 |
| S4 | `complete_and_finish` | 非空 `completed`，`finish.content` | 完成最后节点并显式关闭 Map | 真实 trace 稳定 | 首批 |
| S5 | `read_map` | 无其他字段 | 独占读取完整 Map | 真实 trace + 既有合同 | 首批 |
| S6 | `reopen_and_work` | reopen 后的结构调整及 work | 用户反馈后重开已关闭 Map | 已确认产品生命周期，缺少本轮真实 trace | 首批但需静态场景设计 |
| S7 | `revise_and_work` | 非终态 Map 结构/内容调整及后续 work | 新发现需要增加节点、调整依赖或目标 | 现有产品能力；长任务必需 | 切换前必须纳入，单独冻结最小字段 |

### 5.1 `initialize_and_work`

```json
{
  "sequence": {
    "type": "initialize_and_work",
    "initialize": {
      "root": {"node_id": "root", "goal": "修复失败测试", "content": "", "parents": []},
      "work_nodes": [
        {"node_id": "inspect", "goal": "定位原因", "content": "", "parents": ["root"]}
      ],
      "finish": {"node_id": "finish", "goal": "交付修复", "content": "", "parents": ["inspect"]}
    },
    "client_work": [
      {"client": {"name": "exec_command", "node_id": "inspect", "input": {"cmd": "pwd"}}}
    ]
  }
}
```

Runtime机械归一化为一个 `initialize_map` transaction，再验证 work 节点归属。Agent不声明新节点初始 state。

### 5.2 `work`

```json
{
  "sequence": {
    "type": "work",
    "client_work": [
      {"client": {"name": "exec_command", "node_id": "inspect", "input": {"cmd": "rg --files"}}}
    ]
  }
}
```

`client_work[]` 可以包含多个没有结果依赖的 client calls。数组位置只提供稳定身份和反馈顺序，不声明
B→C 执行依赖。若 Q2 确认 work 即启动节点，则 Runtime 在 dispatch 前只对这些 Agent 已选节点机械生成
`in_flight` 转移。

### 5.3 `handoff_and_work`

```json
{
  "sequence": {
    "type": "handoff_and_work",
    "completed": [
      {"node_id": "inspect", "content": "已确认 round(..., 1) 应改为 2"}
    ],
    "client_work": [
      {"client": {"name": "apply_patch", "node_id": "fix", "input": "*** Begin Patch..."}}
    ]
  }
}
```

约束：

- `completed[]` 只能表达完成事实，不包含 `ready`、`waiting` 或 `in_flight`；
- 可同时完成 join 所需的多个父节点；Runtime在全部 completion 进入一个候选 transaction 后统一推导 readiness；
- `client_work[]` 和 `hosted_work[]` 中每个目标必须在 completion 后真实可执行，但不要求全部都是
  completed 节点的直接子节点；独立 Ready 节点仍可同行；
- 当前 work 的 Tool outcome 不在同一请求继续解锁下一层节点。

### 5.4 `complete_and_finish`

```json
{
  "sequence": {
    "type": "complete_and_finish",
    "completed": [
      {"node_id": "verify", "content": "pytest 3 passed"}
    ],
    "finish": {"content": "修复完成并通过验证"}
  }
}
```

Runtime先提交 Agent声明的 Work completion，再用 canonical `finish_map` 检查唯一 Finish 是否 Ready。该类型没有 client work，
不会产生单独的非终态 completion 请求。

### 5.5 `read_map`

```json
{"sequence":{"type":"read_map"}}
```

该类型禁止 client work、Hosted work 和其他 Map 变化，返回完整当前 Map。

### 5.6 `reopen_and_work`

已闭合 Map 中原 Work 节点已全部 completed，单独 `reopen_map` 只会重开 Root/Finish，不会凭空产生新的可执行
Work。因此该序列的最小产品能力必须是一个整体：

1. Agent 显式选择 reopen；
2. Agent 新增至少一个 Work 节点，并声明其 `parents[]`；
3. Agent 将 Finish 的 `parents[]` 改为包含新的收敛节点；
4. Agent 在同一序列中提交新的 client/Hosted work。

Runtime 可将它机械归一化为 `reopen_map -> update_map(add nodes + finish parents) -> work`，但不能自己生成
返工节点、目标或依赖。该最小形状是 S6 首批落地的推荐，仍需 Q1 确认。

### 5.7 `revise_and_work`

闭集切换不能让当前 Map 失去长任务中的演化能力。该序列建议只允许 Agent 声明：

- `add_work_nodes[]`：新节点的 `node_id/goal/content/parents[]`；
- `node_updates[]`：已有节点的 `goal/content/parents[]` 变化，不包含 state；
- 至少一项后续 client/Hosted work。

它不接受通用 `node_patches[]`，不处理 completed、in-flight、blocked 或 Finish 结束，因此不会与 S3/S4 重叠。
Runtime 把 Agent 声明机械编译成 canonical `update_map`，然后对完整候选 DAG 和后续 work 执行预检。若该最小
能力未冻结，则不能切换移除旧 `update_map`，否则会形成明确功能回归。

## 6. Runtime 内部模型

Agent-visible联合类型解析为 typed enum，再机械归一化：

```text
AgentSequence
  -> NormalizedExecPlan
       pre_map_transactions[]
       client_work[]
       terminal_map_transaction?
       hosted_work[]
```

`NormalizedExecPlan` 是内部执行值，不进入 Tool schema、上下文或持久化。现有 `ExecCall` 若保留，只能作为这一归一化结果，不能继续作为
Agent-visible 任意数组 decoder。

预检固定分为：

1. decode 唯一 sequence variant；
2. 将 Agent明确声明的 Map 部分机械转换并应用到候选 Map；
3. 对 resulting candidate 校验全部 client work 和 Hosted work；
4. 应用该类型允许的 terminal Map operation；
5. 对完整候选 Map 做 invariant 校验；
6. 通过后沿现有持久化、原 Router dispatch、Action settlement 和 outer result 路径执行。

Runtime不得把 client result解释为节点完成，也不得为 Agent选择 sequence、node或 completion。

## 7. Feedback

- 结构错误返回未知/缺失 sequence type 或该类型不允许的字段；
- 动态错误返回具体 sequence type、字段位置、节点状态和违反的 DAG 硬规则；
- 整个序列预检失败仍保持 Map/client 零副作用；
- 不提供下一步建议，不把非法序列改写为另一个合法类型；
- outer result继续按原生 client result、Hosted事实和 Map read返回，不增加语义摘要。

## 8. Hosted 边界

Hosted事实继续由 Provider 原生执行，Agent只声明绑定，Runtime逐项核对。`hosted_work[]` 必须位于明确允许
Hosted 的 sequence branch 内，与该序列的 Map 边界一起预检。这只改变 Agent-visible 组织方式；真实 Provider ID、
逐项多节点归属、漏绑/错绑拒绝和不重执行原则原样复用。

仍需在实施前冻结组合矩阵：哪些 work-bearing sequence 可由纯 Hosted事实满足，以及 `complete_and_finish`
是否允许携带本响应已完成的 Hosted事实。`read_map` 明确禁止 Hosted。该选择会改变合法序列集合，
属于待确认产品决策。

## 9. 不采用的方案

| 方案 | 不采用原因 |
|---|---|
| 保留 `calls[]`，只加强 description | 已证明规则可被理解但不能稳定进入动作生成 |
| `type` + 一组全部可选公共字段 | 仍允许大量结构组合，只把事后拒绝换了名字 |
| generic/custom/raw sequence | 直接破坏闭集目标 |
| Runtime自动选择最近合法类型 | 替 Agent重写动作和计划，越过责任边界 |
| 为每个普通 Tool复制一套 TaskSpace Tool | 侵入原生 Tool合同并导致长期双轨维护 |
| 一次预建全部生命周期组合 | 与 PD2 冲突，无法独立归因复杂度和收益 |

## 10. Provider 与行业依据

1. DeepSeek明确说明 Function Call arguments 由模型生成，可能不是合法 JSON或包含 schema 外参数，调用方必须校验；因此闭集 schema
   减少生成空间，但不能替代 Runtime validation。[DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls)、
   [Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion)
2. JSON Schema支持 `anyOf`/`oneOf` 组合；本设计使用 exact discriminator + `anyOf`，避免分支重叠和不必要的 XOR验证。
   [JSON Schema Boolean combination](https://json-schema.org/understanding-json-schema/reference/combining)
3. OpenAI Structured Outputs说明严格 schema 可以约束 Function arguments，但也明确结构正确不等于值或任务决策正确；动态 DAG仍必须由
   Runtime验证。[OpenAI Structured Outputs](https://openai.com/index/introducing-structured-outputs-in-the-api/)
4. Anthropic同样以 Tool description + `input_schema` 定义 client Tool，并由应用执行 Tool；这支持把结构合同放在 Tool schema、把执行和
   动态状态校验留在 Runtime。[Claude Tool Use](https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview)

## 11. 待用户复核的产品决策

| ID | 决策点 | 推荐 | 为什么需要确认 |
|---|---|---|---|
| Q1 | 首次切换是否采用 S1～S7 | 是；S7 必须在切换前单独冻结，不得丢失 Map 演化能力 | 决定首轮可用场景和是否存在功能回归 |
| Q2 | 选择 work 是否同时是显式的“启动节点”声明 | 是；`ready -> in_flight`，`in_flight/blocked` 保持原状态，Tool outcome仍不推进节点 | 决定是否删除正常启动时独立 `state=in_flight` 的重复声明，又不静默 unblock |
| Q3 | Hosted 与各 sequence 的组合矩阵 | Hosted 作为 sequence 内 work；S1/S2/S3/S4/S6/S7允许，S5禁止 | 决定哪些 Provider响应被接受，不能留在闭集外 |
| Q4 | `revise_and_work` 的最小能力 | 只允许 add nodes、更新 goal/content/parents 和后续 work；不允许 state 或 generic patch | 通用 update会重新打开任意组合逃生口，完全移除又会丢失 Map 演化能力 |
| Q5 | blocked 生命周期如何进入闭集 | 保留 blocked 产品能力，但不塞进 S7 通用状态补丁；在切换前单独冻结 block/resume 场景 | 当前 Map 明确包含 blocked 且允许 blocked work，无声删掉操作能力会形成回归 |

## 12. 验收基线

1. Agent-visible schema 中不存在 `calls[]` 或通用 Map/client排列。
2. 每个 sequence type 都有正例、字段级反例、动态 DAG反例和同源 canonical normalization 测试。
3. `handoff_and_work` 能覆盖单父、多父 join 和独立 Ready work；不接受 Agent 重复声明 child `in_flight`，
   且不允许同批跨两层解锁。Q2 若确认，`in_flight` 由 Agent 的 work 选择机械归一化。
4. 普通 Tool原生 input、Router、权限、sandbox、hook和结果逐字语义不变。
5. Standard final wire逐字不变；TaskSpace Tool declaration变化必须触发缓存门禁。
6. 旧 `RawPlan.calls` Agent decoder、旧 canonical examples、旧任意序列测试和无消费者 helper直接删除，不保留兼容分支。
7. 离线通过后另行申请真实预算；首轮每个新增策略只做最小 sample，不把一次成功作为稳定结论。
