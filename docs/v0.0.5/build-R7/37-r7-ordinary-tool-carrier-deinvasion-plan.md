# R7 普通 Tool 去侵入与连续动作修复

> 状态：普通 Tool 去侵入已验证；bootstrap 连续动作存在待决机制缺口
>
> 日期：2026-07-23
>
> 范围：FLA L2/L4、普通 Tool provider schema、Tool sequence、ActionMap runtime 与观测
> 对应 COE：`coe/2026-07-23-18-24-r7-fla-tool-schema-amplification.md`

## 1. 问题与根因

原实现把完整 `taskspace_action` 判别联合复制到每个普通 Tool，并要求每次普通调用都提交
`continue_current`。这产生了两类结构性问题：

1. TaskSpace 固定 Tool 成本从 Standard 的约 `5,418` estimated tokens 放大到约 `15,186`；
2. TaskSpace 生命周期合同侵入了 `exec_command`、`apply_patch` 等普通 Tool，Agent 需要反复提交
   Runtime 已经掌握的 revision 和 active node。

普通 Tool 的节点归属并不依赖这些重复字段。ActionMap runtime 已在动作执行前校验 Map、binding
和 lease，按 canonical active binding 为 call id 建立 reservation，并在动作结束后把结果写回该
reservation。

## 2. 判废的中间方案

第一版修复尝试把普通 Tool 上的完整联合收敛为可选的紧凑
`taskspace_transition: { action, arguments }`。首请求 Tool section 从约 `60,743` bytes 降到
`28,711` bytes，estimated tokens 从约 `15,186` 降到约 `7,178`，但 Docker 自然样本暴露出
以下问题：

- `arguments` 只能以通用对象暴露，模型看不到每个 action 的精确字段；
- 三个 TaskSpace policy 均反复猜错 `root`、`finish_identity`、`node_id` 等初始化参数；
- 普通 Tool 仍被 TaskSpace 修改，职责侵入没有真正消除。

该方案虽然降低了固定成本，但破坏了 Tool 参数合同的可见性，且没有解决架构边界问题，因此判废，
不保留兼容解析。

## 3. 最终设计

### 3.1 普通 Tool 保持原样

Standard 与 TaskSpace 使用字节一致的普通 Tool schema。普通 Tool 不包含
`taskspace_action`、`taskspace_transition` 或其他 TaskSpace 字段。

普通动作始终服务于 Runtime 维护的 canonical active binding。Runtime 只执行既有机械硬校验和
记账，不选择节点、不推断任务意图，也不改写普通 Tool 的参数或结果。

### 3.2 生命周期动作集中在一个 Tool

`initialize_map`、`bind_node`、`complete_then_continue` 重新归入唯一的
`taskspace_control` schema，并为每个 action 暴露精确字段、类型、必填项和
`additionalProperties: false` 合同。

Agent 在一个 provider response 中按以下顺序发出两个独立调用：

```json
[
  {
    "name": "taskspace_control",
    "arguments": {
      "action": "complete_then_continue",
      "expected_revision": 7,
      "current_node_id": "implement",
      "next_node_id": "verify"
    }
  },
  {
    "name": "exec_command",
    "arguments": {
      "cmd": "cargo test -p codex-core"
    }
  }
]
```

这不是增加一次 provider request。`taskspace_control` 与下一实际动作仍由 Agent 在同一次响应中
连续声明，Runtime 按顺序执行。

### 3.3 序列硬合同

以下边界 action 必须紧邻一个非 `taskspace_control` 的实际动作：

- `initialize_map`
- `bind_node`
- `complete_then_continue`

Tool sequence preflight 在任何调用执行前检查整个响应：

- 合法：`taskspace_control(boundary) + ordinary action`；
- 拒绝：边界 action 单独出现；
- 拒绝：边界 action 后仍是另一个 control；
- 拒绝结果使用
  `taskspace_boundary_action_requires_follow_up`，且 `executed_tool_call_count=0`。

`finish_map` 是最终闭合动作，不要求后续动作。图变更、读取和展开等非边界 control 也不受该规则
影响。

### 3.4 执行与反馈语义

`taskspace_control` 本身已经是 sequence barrier，因此不新增 hook、carrier 或平行执行层：

1. control 失败：其后的实际动作按照现有序列失败传播规则处理；
2. control 成功：canonical binding 先更新，随后普通 Tool 在新 binding 下执行；
3. 普通 Tool 失败：control 已提交和 Tool 失败作为两个有序、独立、忠实的结果进入上下文；
4. Runtime 不回滚已提交状态，不合并或再解释普通 Tool 结果。

L2 和三个边界 action 的 Tool description 明确说明：边界 control 的输出不是声明下一动作的前置条件，
Agent 必须在看到结果前就把二者作为同一 response 的有序调用提交。这只约束协议形状，不替 Agent 选择动作。

## 4. 分层改动

| 层 | 改动 |
|---|---|
| L2 协议 | 说明边界 control 与下一实际动作必须在同一响应连续声明 |
| L4 Tool | 精确边界 action 只在 `taskspace_control` 暴露；普通 Tool 零扩展 |
| Router | 删除普通 Tool 参数提取、剥离和重写 |
| Sequence | preflight 拒绝单独边界；沿用 control barrier 和已有顺序执行 |
| Runtime | 沿用 active binding、lease、reservation 硬基线 |
| Projection | bootstrap 只说明 `taskspace_control_then_ordinary_tool` |
| Observer | 记录普通 schema 扩展数、Tool bytes、边界配对及结构化失败码 |

## 5. 明确删除

- 删除 `continue_current`；
- 删除普通 Tool 上所有 TaskSpace 字段；
- 删除普通 Tool decorator、transition parser、carrier executor 和结果 envelope；
- 删除 `TASKSPACE_ACTION_REQUIRED`；
- 不兼容旧 carrier，不增加双字段解析或 fallback；
- 不由 Runtime 选择节点、补全 Map、推断动作意图或修复 Agent 参数。

## 6. 验收门

### 6.1 合同与边界

- Standard 与 TaskSpace 的每个普通 Tool schema 字节一致；
- TaskSpace 只额外暴露一个 `taskspace_control`；
- 三个边界 action 的精确参数只出现一次；
- 普通 Tool 仍必须经过 active Map、binding 和 lease gate；
- 生产 Rust 代码不存在旧 carrier 或 transition 解析路径。

### 6.2 连续动作与反馈

- 初始化、绑定、handoff 与下一实际动作在同一 provider response；
- 单独边界在任何调用执行前整体拒绝；
- control 成功、普通 Tool 失败时，两项事实分别保留；
- 普通 Tool 原始参数和结果不被 Runtime 语义改写；
- 一次响应最多一个 patch 的既有合同保持不变。

### 6.3 自然样本

- Standard、map-always、map-append、map-request 四臂业务结果通过；
- TaskSpace 首次边界调用不再猜测 schema；
- 不新增单独生命周期 request；
- map-request 不因去除普通 Tool carrier 而绕过强制 Map 工作流；
- 最终 Map 显式闭合。

## 7. 参考依据

1. [DeepSeek Tool Calls](https://api-docs.deepseek.com/guides/tool_calls/)：Tool 参数使用 JSON
   Schema；对象字段应明确暴露，避免依赖模型猜测通用对象的内部合同。
2. [OpenAI Function Calling](https://developers.openai.com/api/docs/guides/function-calling)：
   Function 描述和参数 schema 会随 Tool 提供给模型，因此按普通 Tool 数量复制合同是实际输入
   成本。
3. [Claude Define Tools](https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools)：
   Tool 定义应聚焦自身用途与输入；TaskSpace 生命周期不应成为每个普通 Tool 的参数职责。

## 8. 非目标

- 本轮不重构三种 Map projection policy；
- 不改变 DAG、Root、Finish、revision、lease 或 reservation 状态机；
- 不处理 map-request 的多 patch 行为；
- 不新增 lifecycle Tool、hook 或平行状态通道。

## 9. 实施验证

### 9.1 Schema 与状态硬基线

- Standard 与 TaskSpace 普通 Tool schema 字节一致的 Rust 回归通过；
- 旧 decorator、transition parser、carrier executor 和结果 envelope 已删除；
- 三个边界 action 以精确 schema 只在 `taskspace_control` 出现一次；
- 空 Map、无 binding 和无 lease 的普通动作仍由既有 ActionMap gate 拒绝；
- sequence、control parser、terminal integration、89 项 TaskSpace 回归和五层权威合同均通过。

首请求 Tool section：

| 版本 | Tool count | Bytes | Estimated tokens | 普通 Tool TaskSpace 字段 |
|---|---:|---:|---:|---:|
| Standard | 12 | 21,669 | 5,418 | 0 |
| 旧必填 carrier | 13 | 60,743 | 15,186 | 每个普通 Tool 1 个完整联合 |
| 判废的紧凑嵌套 transition | 13 | 28,711 | 7,178 | 每个普通 Tool 1 个通用对象 |
| 当前 control + sibling | 13 | 25,394 | 6,349 | 0 |

相较旧 carrier，当前每个 request 的 Tool section 减少 `35,349` bytes，约 `8,837`
estimated tokens；相较 Standard 的固定增量只剩唯一 `taskspace_control` 的约 `931` tokens。

### 9.2 Docker 自然样本

`single-file-fast-fix` 使用同一新二进制在 Docker 中运行 Standard 和三个 TaskSpace policy：

| Arm | Solved | Requests | Input | Cached | Request 2+ hit | Map |
|---|---:|---:|---:|---:|---:|---|
| Standard | 1/1 | 6 | 73,043 | 71,424 | 97.53% | N/A |
| map-always | 1/1 | 7 | 101,631 | 43,904 | 49.28% | 5 nodes / 4 edges / 0 open |
| map-append | 1/1 | 8 | 131,536 | 111,360 | 93.60% | 5 nodes / 4 edges / 0 open |
| map-request | 1/1 | 8 | 114,710 | 87,552 | 85.71% | 5 nodes / 4 edges / 0 open |

三个 TaskSpace arm 的后续 `complete_then_continue + patch/test` 均在同一 provider response
正确执行，最终 `finish_map` 显式闭合。没有字段 shape 猜测，也没有普通 Tool 参数改写。

## 10. 待决机制缺口

三种 policy 都在首请求先提交了普通命令，被空 Map gate 忠实拒绝；下一轮又各出现一次单独
`initialize_map`，被 sequence preflight 拒绝，第三轮才正确组合初始化和实际动作。加强 L1 宏观前提、
L2 有序调用合同和三个边界 action description 后重复运行，仍稳定出现该路径。

这说明跨两个独立 Tool call 的 sibling 关系不能由单个 JSON Schema 结构化保证。继续消除这两个 bootstrap
request，需要在以下产品方向中选择，而不能继续堆叠同义提示词：

1. 动态限制首请求可见 Tool 或 `tool_choice`：能提高初始化确定性，但改变 request cache shape，且可能让同
   response 的普通 sibling 不可见；
2. Runtime 机械预初始化：不修改普通 Tool，但需要重新定义空 Map、Root 和首个 Work binding 的所有权；
3. 在 `taskspace_control` 内嵌下一动作：可获得单 call 结构保证，但引入 nested dispatch、权限与反馈
   envelope，并可能重新产生大 schema；
4. 接受首轮硬门反馈：架构最简洁，但固定增加约一至两个 provider request。

该选择涉及缓存、Agent 状态所有权和连续动作产品合同，超出“删除普通 Tool 侵入”的局部修复范围，不应由
实现层静默决定。
