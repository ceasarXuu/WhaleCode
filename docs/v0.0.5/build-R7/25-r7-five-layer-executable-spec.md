# R7 TaskSpace 五层架构可执行规格

- Created: 2026-07-20
- Version: 1.1
- Status: Production active through FLA-3.5; FLA-4 is next
- Scope: FLA-0 至 FLA-3、FLA-3.5、FLA-4 至 FLA-8 的唯一实施与验收入口
- Rollback baseline: `48922ce9b`
- Compatibility: 不兼容旧合同，不保留双轨生产路径

## 1. 本文件解决什么问题

架构设计说明“为什么这样分层”，具体合同稿展示 Agent 会看到什么，但二者仍允许实施者在候选方案之间自行
选择。本规格消除这类隐藏决策：它冻结唯一主线、完整机器合同、生产入口、删除项、生命周期判定、测试、日志、
评估阈值和完成证据。

selected target、experiment 和 blocking repair 的 `implementation_status` 只能使用以下五种值：

| 状态 | 含义 |
|---|---|
| `selected_baseline` | 当前生产实现和回滚基线 |
| `selected_not_implemented` | 已确定产品合同，但生产代码尚未接通 |
| `active_verified` | 生产路径、合同测试、日志和阶段 smoke 全部通过 |
| `active_repair_verified` | 为修复已激活层的阻塞问题而提前接通选定合同，生产路径和定向回归已通过，但不代表其名义阶段已验收 |
| `experimental_disabled` | 独立实验，不得混入主线或被称为已完成 |

顶层 `contract_status` 描述整份 authority/manifest 的角色，生产 layer 使用独立 `runtime_status`；三类字段不得再
共用模糊的 `status`。机器约束由 `five-layer-contract-authority-v1.schema.json` 和
`taskspace-contract-manifest-v1.schema.json` 执行。

只有生产路径连通、定向测试通过、要求的日志可观测且阶段样本通过，阶段才能标为 `active_verified`。只有提示词、
schema、mock、脚手架或文档的提交一律不算阶段完成。

## 2. 权威与选定合同

机器可读权威清单是
[`five-layer-contract-authority-v1.json`](../../../benchmarks/taskspace/r7/five-layer-contract-authority-v1.json)。
发生冲突时按清单的 `authority_order` 判定；任何字节或状态冲突都阻止实施，不允许开发者自行择一。

| 层 | 当前生产基线 | 已选目标 | 当前状态 |
|---|---|---|---|
| L1 | TaskSpace Base v2.0.1；Map 段仅保留宏观模型，整份 Base 不携带 Tool wire 示例 | [`five-layer-l1-taskspace-base-section-v2.md`](../../../benchmarks/taskspace/r7/five-layer-l1-taskspace-base-section-v2.md) | `active_verified` |
| L2 | `taskspace-core-v2.9` | [`five-layer-l2-core-protocol-v2.md`](../../../benchmarks/taskspace/r7/five-layer-l2-core-protocol-v2.md) 作为现有 developer bundle 第一段 | `active_verified` |
| L3 | `taskspace-advanced` v1.0.0，会话锁定内容寻址快照 | [`five-layer-l3-taskspace-advanced-v1.SKILL.md`](../../../benchmarks/taskspace/r7/five-layer-l3-taskspace-advanced-v1.SKILL.md) | `active_verified` |
| L4 | 普通动作 Tool 的必填 `taskspace_action` carrier；纯 Map/read/terminal 使用 `taskspace_control` | FLA-4 在该单一基线上正式化描述与 input schema | `active_repair_verified` |
| L5 Result | `TaskSpaceControlResultV2`，布尔常量 `partial_commit=false` | [`five-layer-taskspace-result-v2.schema.json`](../../../benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json) | `active_repair_verified` |
| L5 Projection | 三策略共享 canonical Map 和 renderer | 维持 [`projection-policy-contract.json`](../../../benchmarks/taskspace/r7/projection-policy-contract.json)，补生命周期判定 | `selected_baseline` |

主线明确选择：普通动作由必填 `taskspace_action` 明确声明继续当前绑定或承载生命周期交接；纯 Map/read/terminal action
继续使用一个 `taskspace_control`；`strict=false`；不向 DeepSeek 声称 `output_schema`。读写拆 Tool、MCP
`outputSchema` 和 DeepSeek strict 是三个 `experimental_disabled` 单变量实验。移除 `required_next_call` 不是实验，
而是 FLA-3.5 修复 H-003 的组成部分。

### 2.1 FLA-2 阻塞修复例外

FLA-2 对抗审查证明：已激活的 L2 恢复协议依赖统一的 L5 factual result，而旧 L4 嵌套 discriminator 又会造成
Agent 可见能力歧义。为修复已上线层自身的合同断裂，生产路径提前接通选定 L4 input schema 和 L5 result schema。
这是阻塞修复，不改变名义 phase 顺序。FLA-3 现已独立验收，`activation_through` 为 `FLA-3`；L4/L5 仍只标记为
repair active，其完整阶段 smoke、三臂比较和接受决策仍需按后续 phase 执行。后续阶段以当前修复版本为基线验证，不得回退到旧
`transition_node` 或 R6 result。

### 2.2 FLA-3.5 连续动作修复优先级

FLA-3 重复运行证明当前 L4 仍允许 Agent 先发单独 lifecycle control，再因缺少 sibling 被拒绝。连续动作已经在
R5 J6 与 R7 D.2 证明有明确 request 和执行路径收益，不能降级成提示词建议或 FLA-6 可选实验。

[连续动作合同回归修复](33-r7-continuous-action-regression-repair-plan.md) 是 FLA-3.5 的权威入口。修复已直接接入
现有 Tool builder、router 和状态机路径；旧 sibling 字段与 preflight 已删除，不存在候选晋级、双 parser、
feature flag 或旧 session 兼容路径。

## 3. Agent 实际看到的内容

### 3.1 L1 与 L2 的 wire

L1 的英文文件是逐字权威文本，只替换
`whalecode_taskspace.md` 中 `## TaskSpace work map` 到下一个同级 `## Task execution` 之前的区间。L2 的英文文件
原样成为现有 developer bundle 的第一段，后面继续是 permissions、AGENTS、skills catalog 等现有 section。

DeepSeek 请求固定为：

```text
messages[0] role=system  content=<完整 TaskSpace Base，内含 L1>
messages[1] role=system  content=<L2> + <现有 developer bundle 其余 sections>
messages[...]            content=<自然历史和 Tool 结果>
messages[last]           content=<由 projection policy 决定的动态 section>
tools                    content=<L4 schema + 普通工具 schema>
```

这里没有新增第三条 system，也没有独立 developer 权限层。逻辑 developer 在 DeepSeek Chat adapter 中机械映射为
第二条 system。Standard 不装配 L1、L2、L3 正文或 TaskSpace Tool。

Standard 与 TaskSpace 完整 Base 都只能描述通用工具行为，不得内嵌 JSON 参数对象、patch 正文模板或其他
provider Tool wire 示例。具体调用语法由请求顶层的 Tool schema 唯一负责；FLA-2 合同测试同时扫描两份 Base，
任何命中都阻止验收。

### 3.2 L3 的载体

生产目标为
`third_party/codex-cli/codex-rs/skills/src/assets/samples/taskspace-advanced/SKILL.md`。会话建立时锁定
`name + skill_version + body_sha256 + immutable_snapshot_path`：

- 用户显式 mention：宿主将快照正文作为 `<skill>` user item 注入一次。
- Agent 自主使用：Agent 用普通文件读取 Tool 打开 catalog path，正文只作为该 Tool result 出现。
- compaction 只保留锁定身份，不自动重载正文。
- resume/fork 恢复锁定身份；快照缺失时，现有 Skill 装载/文件读取载体原样返回失败，观测日志记录
  `TASKSPACE_SKILL_SNAPSHOT_MISSING`，不得伪装成 `taskspace_control` 结果，也不得回退到最新版。
- 两种载体必须引用同一 SHA；同一会话不得因为 hot update 改变正文。

Skill 失败不创建新的 TaskSpace Tool result。显式 mention 缺少快照时，宿主返回固定事实
`TaskSpace skill snapshot unavailable: name={name} version={version} sha256={sha256} path={snapshot_path}`；Agent 自主
读取失败时继续使用原文件 Tool 的原始错误。两者都记录
`skill_load_status=snapshot_missing, reason_code=TASKSPACE_SKILL_SNAPSHOT_MISSING` 及 name/version/hash/path/carrier，
不追加“加载最新版”或下一步建议。

### 3.3 L4 当前 carrier 合同

独立 `taskspace_control` action 以
[`five-layer-taskspace-control-v2.schema.json`](../../../benchmarks/taskspace/r7/five-layer-taskspace-control-v2.schema.json)
为准。三个非终态 lifecycle action 已从该 Tool 删除；当前独立 action 是：

```text
mutate_graph, block_node, unblock_node, rework_node, finish_map,
expand_nodes, read_map, read_output_ref
```

`read_output_ref` 的 `head`、`tail`、`line_range`、`grep` 四种分支均已在 schema 中内联。`block_node` 和
`rework_node` 不接收 `reason`，因为当前 Rooted DAG 领域模型没有该字段；五层重构不得暗中扩展领域状态。
`initialize_map`、`bind_node`、`complete_then_continue` 只存在于普通动作 Tool 的轻量
`taskspace_action`。同节点普通动作使用 `continue_current`。Patch 正文继续是 `apply_patch.input` 顶层字段，原 Tool router、权限、sandbox、hook、
handler 和输出链保持唯一。Tool 身份继续包含
`provider_schema_profile + capability_set_hash + tools_hash`；Map revision 不触发 schema 变化。

### 3.4 L5 的完整结果合同

所有 control/read 成功与失败结果必须通过
[`five-layer-taskspace-result-v2.schema.json`](../../../benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json)。
共同约束：

- `schema_version` 恒为 `TaskSpaceControlResultV2`。
- `partial_commit` 恒为布尔值 `false`；原子性范围是一条 control 调用，不是整条 assistant response。
- 状态为 `committed`、`read_ok`、`argument_failed`、`protocol_failed`、`state_machine_failed` 或
  `resource_failed`。
- `actual` 是 Runtime 观测事实，`expected` 是调用合同要求或提交值；不得互换。
- 错误不得携带 Agent 下一步动作建议。
- carrier transition 成功后普通 Tool 失败时保留两份事实；Map 不自动回滚。
- 截断的 `line_range` 读取若仍有后续行，`continuation` 是一份可直接再次调用的完整
  `read_output_ref` 参数对象；其他 mode 或无剩余内容时为 `null`。

当前回归基线错误码与消息模板如下。模板只能填入 JSON 转义后的事实字段，不得追加“请重试”“建议读取”等指导：

| code | status | 固定 message 模板 | 必须携带事实 |
|---|---|---|---|
| `TASKSPACE_INVALID_ARGUMENT` | `argument_failed` | `arguments do not match the selected action schema` | action、字段路径、submitted value、schema requirement |
| `TASKSPACE_STALE_REVISION` | `state_machine_failed` | `expected_revision does not match the current canonical revision` | submitted revision、canonical revision |
| `TASKSPACE_GRAPH_INVARIANT` | `state_machine_failed` | `the submitted mutation violates a rooted DAG invariant` | invariant id、submitted node/edge ids、canonical revision |
| `TASKSPACE_LIFECYCLE_INVARIANT` | `state_machine_failed` | `the submitted transition is not valid from the observed lifecycle state` | action、node id、observed status、allowed source statuses |
| `TASKSPACE_OUTPUT_REF_NOT_FOUND` | `resource_failed` | `the requested retained output reference does not exist` | output_ref、requested range |
| `TASKSPACE_RANGE_INVALID` | `argument_failed` | `the requested retained output range is invalid` | output_ref、submitted range、available range |

carrier 使用一个 call id。action 校验或 transition 失败返回 `TaskSpaceCarrierResultV2` 且 `tool_dispatched=false`；提交成功后
设置 `tool_dispatched=true`，随后原样附加普通 Tool 输出。普通 Tool 失败不会覆盖 transition 事实，Runtime 也不
自动回滚。FLA-5 负责继续形式化全载体结果一致性，不在本阶段增加 prepare/reservation 或第二套结果代数。

Rust enum 常量使用上述大写值，JSON `error.code` 原样输出；日志也使用同一值。FLA-3.5 目标中不再存在
missing-sibling 运行时形态；非法 transition carrier 使用同源参数/状态错误分类，不另造带下一步建议的错误。

## 4. 生命周期的确定性判定

[`five-layer-lifecycle-oracles-v1.json`](../../../benchmarks/taskspace/r7/five-layer-lifecycle-oracles-v1.json)
保留 FLA-3.5 前的 12 个历史场景，供 FLA-7 重建等价性基线，覆盖：

- 历史初始化/sibling 路径；stale revision；历史 missing-sibling 拒绝。
- control 已提交后普通 Tool 失败；同一 response 多个 control 独立提交。
- `map-append` 精确 retry 去重与不同 request 同 revision 仍追加。
- `map-request` read rev4、mutate rev5 后不自动提醒或再读。
- `map-always` 只替换 current projection。
- resume、fork、compaction 在三种 policy 下的恢复。

oracle 已冻结两份完整 canonical Map、对应 SHA256 和 event-chain head，并给出 projection hash 公式。只由
FLA-7 在激活前实现 `freeze-r7-five-layer-fixtures.ps1`：先独立重算这些冻结 hash，再从现行共享 renderer 生成
projection/provider payload golden 并按每个场景的允许差异比较。没有脚本和 golden 产物时只能称为 schema 已
设计，不能称生命周期已验证；生成 golden 是机械固化现有 renderer 输出，不得成为改写 projection 语义的入口。

FLA-3.5 定向测试已覆盖“standalone 在 schema 中不可表达”“carrier 参数失败零提交”“transition commit + Tool
failure 两事实保留”和 code-mode carrier。canonical projection、resume、fork、compaction 与 recovery 仍由
FLA-7 的 lifecycle oracle 独占，不得把历史拒绝改写成未发生。

## 5. FLA 实施矩阵

所有阶段从上一 `active_verified` commit 开始。阶段失败使用 `git revert <phase-commit>` 回到该 commit；不保留
feature flag、兼容 parser 或双 schema。每个阶段必须先提交生产改动和测试，再运行 smoke；失败时用新提交修正，
不得改写已有证据。

同一 production target、gate 或决策只能有一个 owner phase。R7 Phase E/F/G 只能只读引用 FLA-6/7/8 evidence，
不得再次登记 production target 或 gate。

### FLA-0：冻结现行基线

- 生产入口：只读记录 `base_instructions_profile.rs`、`provider_wire_sections.rs`、`taskspace_tool.rs`、
  `taskspace_control_args*.rs`、`taskspace_control_output.rs`、`projection*.rs`。
- 新增：`five-layer-contract-authority-v1.json` 的 baseline hashes；基线 payload/tool/result snapshots。
- 删除：无。生产行为改动：无。
- 测试：`test-r7-base-instructions-contract.ps1`、`test-r7-projection-policy-contract.ps1`、
  `test-native-control-contract.ps1`。
- 运行：评估合同的 smoke 命令，Standard 与三种 policy 各 3 次两个开发样本。
- 日志：commit、image digest、model、provider、tools hash、section hash、request/token/cache/time、Map events。
- 完成证据：所有 hash 可从源码重算；基线 trace 完整；不声明性能收益。

### FLA-1：装配与 ownership

- 修改：`core/src/context/base_instructions_profile.rs`、`core/src/provider_wire_sections.rs` 及其 tests。
- 新增生产 artifact：`core/src/context/prompts/taskspace_contract_manifest_v1.json`；内容由 authority 清单生成，禁止手写漂移。
- 行为：给 L1-L5 product-owned sections 建立 identity 和固定顺序；user、AGENTS、外部 Tool result 不参与语义 lint。
- 删除：任何按自然语言关键词判断跨层冲突的 lint；当前没有则记录 `absent`。
- 定向测试：Standard 无 TaskSpace section；DeepSeek 两条 system 的字节顺序；capability set 只改变允许的 schema enum；
  同 payload 可按 section identity 重建。
- 命令：新增 `pwsh scripts/taskspace-benchmark/test-r7-five-layer-contracts.ps1 -Phase FLA-1`。
- 日志：section id/version/hash/bytes/role/order、provider schema profile、capability hash、tools hash。
- 完成证据：生产 payload snapshot 与 manifest 全字段一致，现行模型可见字节除 identity metadata 外不变。

### FLA-2：激活 L1 与 L2

- 修改：`protocol/.../whalecode_taskspace.md`、`base_instructions_profile.rs`、developer bundle 构造入口、
  `provider_wire_sections.rs`。
- 新增：`core/src/context/prompts/taskspace_core_protocol_v2.md`，字节必须等于 L2 权威文件。
- 删除：Base 中被 L2 接管的操作教程；`working-protocol-contract.json` 不再作为生产装配来源。
- 定向测试：L1 区间精确 hash；L2 是第二条 system 第一 section 且只出现一次；Standard 零出现；旧教程短语不重复。
- 日志：L1/L2 hash、wire index、role、bytes、重复次数。
- smoke：两个开发样本，Standard、受影响 policy 的冻结基线和候选，各 3 次配对运行。
- 完成证据：production wire 使用选定文本；所有正确性 gate 通过；只报告成本变化，不声称收益。

### FLA-3：激活高级 Skill

- 修改：bundled skill registry、安装 snapshot/session persistence、Skill catalog 生成和 compaction/resume/fork 恢复入口。
- 新增：`skills/src/assets/samples/taskspace-advanced/SKILL.md` 及 snapshot identity 类型。
- 删除：若有根据任务语义自动加载 Skill 的分支；不得按任务复杂度自动注入。
- 定向测试：显式 mention 与 Agent 文件读取载体不同但 body hash 相同；简单任务不加载；hot update、snapshot missing、
  catalog truncation、resume/fork/compaction 均符合第 3.2 节。
- 日志：load trigger/carrier/name/version/hash/path/bytes/status/reason。
- smoke：简单 `single-file-fast-fix` 与复杂 `subscription-billing-repair`。
- 完成证据：生产 catalog 可发现、两条加载路径可运行、失败进入 Agent 上下文且无 latest fallback。

### FLA-3.5：修复连续动作合同回归

- 权威入口：`33-r7-continuous-action-regression-repair-plan.md`。
- 实现：共享 decorator/parser 将小型状态交接附着到普通动作 Tool，Runtime 提交交接后复用原 router、权限、
  sandbox、hook、approval 和业务 handler。
- 删除：生产 `required_next_call`、missing-sibling preflight、三个非终态独立 control 分支和对应兼容 parser。
- 反馈：同一 call 返回 transition factual header，原 Tool 输出保持不变；transition 失败则不 dispatch 普通 Tool。
- 验证：Rust 定向回归、FLA-3.5 gate、CLI build 和 Docker paired smoke 已通过。
- 激活语义：FLA-3.5 为 `active_verified`；L4/L5-result 维持 `active_repair_verified`，由 FLA-4/5 正式完成各层。

FLA-3.5 已完成，FLA-4 可以开始。

### FLA-4：激活 L4 input schema

`FLA-4-Repair-Baseline` 只检查当前 repair baseline；名义 `FLA-4` 在 FLA-3.5 未完成时必须失败，不能用 baseline
通过冒充阶段完成。

- 修改：`tools/src/taskspace_tool.rs`、`taskspace_tool_simple_actions.rs`、
  `core/src/tools/handlers/taskspace_control_args.rs`、`taskspace_control_args_wire.rs`。
- 删除：`transition_node + transition` discriminator；旧 parser 分支和 schema 构造器同次删除。
- 以 FLA-3.5 晋级后的 carrier 为唯一基线；正式阶段优化 action-local 描述、discriminator 与静态 schema，
  不重新引入 sibling、旧 discriminator 或 R6 result。
- 定向测试：每个 action 一组 valid fixture、每个 required/extra/type 失败 fixture；最终 provider schema 等于权威 schema；
  `apply_patch` 不可见时只发生指定 enum 机械变化；旧 action 全部拒绝。
- 日志：schema profile/hash、action、parser branch、validation code、visible tool set。
- smoke：两个开发样本三臂各 3 次，观察首次有效 control、错误参数、request 和 cache。
- 完成证据：生产 Tool 和 parser 只接受新 action；所有 schema branches 100% covered。

### FLA-5：激活 L5 result algebra

`FLA-5-Repair-Baseline` 只检查当前 result repair baseline；名义 `FLA-5` 在 FLA-4 未完成时必须失败。

- 修改：`taskspace_control_output.rs`、`sequence_preflight.rs`、`sequence.rs`、control handler/read path。
- 当前修复基线已统一产生 `TaskSpaceControlResultV2`、错误码和布尔 `partial_commit=false`；FLA-3.5 拥有
  carrier typed outcome transport。正式阶段只补齐 transition fact + opaque Tool output conformance、fixture
  freezer 和三臂评估证据，不重复实现 transport。
- 删除：R6V1 formatter、整数 `partial_commit`、旧自由文本 envelope；不保留版本协商。
- 定向测试：结果 schema 每个 `oneOf` 分支 golden；LC-01 至 LC-05；当前双 call 与目标单 carrier 都保持两个
  独立事实；Agent 可见 transition fact 合规且 opaque Tool 子载体可逆等价。
- 日志：第 3.4 节 envelope 字段和 oracle 要求字段，禁止只写人类摘要。
- smoke：两个开发样本三臂各 3 次，错误调用仍计行为失败。
- 完成证据：生产所有 control/read 路径 100% 通过 V2 schema，R6V1 搜索结果为零。

### FLA-6：三个独立实验

- 顺序：E1 读写拆分、E2 MCP output schema、E3 DeepSeek strict。
- 每项从 FLA-5 冻结 commit 单独派生、单独评估；前项未接受不得叠加到后项。
- E1 必须同时实现 router/approval/executor 权限矩阵，否则只能评估可选择性，不能声称权限收益。
- E2 必须让 MCP `outputSchema` 和 `structuredContent` 同时符合 V2 schema。
- E3 先 probe adapter 转发、Beta endpoint、全部可见工具 schema 与 parallel calls，再允许行为测试。
- 完成证据：每项产生独立候选 commit、三臂评估和 accept/reject decision；reject 后生产代码回到 FLA-5，无残留分支。

### FLA-7：生命周期与 projection 恢复

- 修改：`action_map/projection.rs`、`projection_policy.rs`、canonical store/reducer、session history reducer、
  `provider_wire_sections.rs` 及 resume/fork/compaction 入口。
- 保持：三种 policy 共用状态、renderer、validator、Tool 和 result；只允许 emission 规则不同。
- 唯一接管 R7 Phase E/F 的实现项：三策略 scripted differential、retry/provider/tool error、resume/fork/
  compaction/subagent、context epoch、CLI/config/protocol/session、observer、Viewer、wire scanner 和旧 R6 路径删除。
- 定向测试：LC-06 至 LC-12 全部 golden；event hash 从同一快照重放一致；`map-request` 不自动提醒；
  `map-append` 只对相同 provider request retry 去重。
- 静态审计：Runtime 不读取命令正文、Patch 内容、测试名称或 reasoning 来生成状态/建议。
- 日志：policy、trigger、request identity/attempt、revision、projection hash、emission reason、dedupe key、恢复来源。
- smoke：两个开发样本，对三种 policy 分别跑 Standard/冻结基线/候选。
- 完成证据：12 个 oracle 全部通过，三策略 canonical event log 相同，差异只出现在 emission trace。

### FLA-8：正式评估与决策

执行
[`five-layer-evaluation-contract-v1.json`](../../../benchmarks/taskspace/r7/five-layer-evaluation-contract-v1.json)：

当前 v1 的 `combined_control_plus_next_rate` 只适用于历史 sibling 回归基线。FLA-8 开始时从已激活 carrier
生成、预注册并激活自己的 v2，将旧指标替换为 transition carrier 指标，同时保持
sealed held-out、30 repeats、bootstrap、顺序、停止和 correction 规则不变。

- shared change 使用 7 臂；单 policy 实验使用 3 臂。
- 七臂中 Standard + 晋级后 map-always/map-append/map-request 是 R7 Phase G 唯一四臂子矩阵；benchmark skill、
  report、默认值建议和产品取舍表全部由本阶段同一 raw artifacts 生成。
- 5 个冻结样本，每个样本固定 30 次配对重复；候选矩阵封存前不查看聚合门槛，也不提前停止。
- 两侧 95% paired bootstrap、10000 次重采样、按 sample 分层；同一决策族多候选用 Holm-Bonferroni。
- 正确性、语义完整性、合同违规为硬 gate；request、token、cache、wall time、组合调用为预注册非劣 gate。
- held-out 结果只在本阶段解封；30 次仍不确定则不晋级。

完成证据是 raw manifests、requests/tool/map JSONL、逐运行 verdict、汇总 CSV、bootstrap 结果和明确的
promote/reject/inconclusive 决策。非劣通过不等于证明收益。

## 6. 测试与日志落地清单

以下脚本在本设计提交时是“要求新增”，不是声称已经存在：

```text
scripts/taskspace-benchmark/test-r7-five-layer-contracts.ps1
scripts/taskspace-benchmark/freeze-r7-five-layer-fixtures.ps1
scripts/taskspace-benchmark/run-r7-five-layer-matrix.ps1
scripts/taskspace-benchmark/report-r7-five-layer-matrix.ps1
```

统一 artifact 根目录：

```text
artifacts/taskspace/r7/five-layer/<contract-id>/<subject-commit>/<run-id>/
```

每次 provider request 至少记录：`session_id`、`request_id`、`attempt`、arm/sample/pair、输入/缓存/输出/reasoning
tokens、wall/provider/tool/control time、section hashes/bytes、tools hash、policy、canonical revision、visible projection
revision、projection hash、Tool call/result 顺序、错误码和 scenario verdict。API key、完整环境变量和用户私有文件内容
不得进入日志。

## 7. 评估规则摘要

机器权威是 evaluation contract，下面仅给评审者快速核对：

| 指标 | 晋级条件 |
|---|---|
| scenario success | TaskSpace arm 失败数不高于基线，成功率不低于基线 |
| 语义完整性/硬合同违规 | 均为 0 |
| request mean / median / p95 | 增幅不超过 10% / 1 次 / 2 次 |
| uncached input / total input mean | 增幅不超过 10% / 15% |
| request 2+ cache hit | 下降不超过 2 个百分点 |
| wall time mean | 增幅不超过 20% |
| required transition carrier rate | 当前 TaskSpace arm 为 100%；历史 sibling 仅作机械归一化参考 |
| init/bind/continue 单独 transition | 0 |

缓存遥测缺失记为 unavailable，不得记零。价格没有冻结 artifact 时不计算金额，只比较 token。环境、鉴权和明确的
provider capacity 故障使配对无效并原序重跑；Agent 参数错误、协议错误、普通 Tool 错误和任务结果错误必须计入表现。

## 8. 回滚和完成声明

每阶段只允许一个生产合同。若阶段失败，revert 阶段 commit 并重新生成 authority hash；不得保留运行时选择旧/新
parser、旧/新 result 或旧/新 prompt 的兼容开关。已有 session 和数据无需迁移，可丢弃并重新开始。

阶段报告必须逐项列出：生产入口、删除项、测试命令与结果、日志样例、smoke 指标、未解决风险、实施 commit 和
回滚 commit。缺少任一项时状态仍为 `selected_not_implemented`。整个五层重构只有在 FLA-0 至 FLA-3、FLA-3.5、
FLA-4 至 FLA-8 均有完成证据、
且正式决策为 promote 后，才能称为实现完成或取得收益。
