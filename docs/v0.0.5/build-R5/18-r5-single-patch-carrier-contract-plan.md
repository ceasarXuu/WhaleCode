# R5-J7 单 Request Patch Slot 与 Patch 预检计划

- Created: 2026-07-12
- Updated: 2026-07-13
- Version: 1.2
- Status: COMPLETE；J7.0-J7.8 与 J7.5 14/14 final gate verified
- Owner / Responsible: WhaleCode TaskSpace / apply_patch substrate
- Related Systems: provider response tool sequence、`taskspace_control` tool schema、nested ToolSpec、ToolRouter、
  `codex-apply-patch`、benchmark observer
- Related Links: `17-r5-schema-first-taskspace-control-plan.md`、`01-r5-phased-simplification-plan.md`、
  `22-r5-j6-7-canonical-task-context-plan.md`
- Risk Level: High
- Plan Type: Full

## 1. 背景与问题定义

执行依赖：J6.7必须先完成TaskSpace任务上下文单一事实源切换，并冻结新的canonical event/carrier边界。
J7.0需要基于J6.7最终production path重新审计schema、sequence和feedback入口；本文现有J6证据保留为
历史缺陷基线，不允许直接在旧双轨carrier上开始J7实现。

J6 复杂样本中，Agent 在一个 `finish_then_actions` carrier 内声明了三个连续 `apply_patch`：

```text
finish analyze node
  -> apply_patch(parser.py)   success
  -> apply_patch(pricing.py)  context verification failed
  -> apply_patch(test_invoice.py) skipped after prior failure
```

最终任务通过后续请求恢复并完成，但这条路径暴露了两个不同层级的问题：

1. `taskspace_control.actions[]` 允许重复出现任意可见工具，tool schema 没有表达“一个 carrier 最多一个
   `apply_patch`”。Agent 因而自然地把多个文件任务映射为多个 patch action。
2. 当前 `codex-apply-patch` 按 hunk 顺序直接写文件。仓库测试
   `test_apply_patch_cli_failure_after_partial_success_leaves_changes` 明确固化了“前序 hunk 已写入、后序 hunk
   失败后保留前序改动”的行为。因此，仅把多个 patch 文本机械合并为一个调用，并不能自动获得事务安全。

该问题不是 Agent 语义能力不足，也不是 projection 需要增加提示。它是工具操作形态和底层 patch 执行语义未对齐：

```text
Agent 看到：actions 是可重复工具数组
runtime 做到：顺序执行，首错停止，尾部 skipped
文件系统得到：可能已经发生部分写入
```

J6.7修复后的3-repeat进一步证明原计划的scope不足。R5在active Map下通过顶层native tools生成了多个兄弟
`apply_patch`，并未经过`taskspace_control.actions[]`：

- complex repeat-2在一个provider response中生成5个顶层patch，其中两个修改同一`plans.py`；前4个成功、
  `test_plans.py` patch失败，形成部分提交并触发后续恢复；
- complex repeat-3在一个provider response中生成4个顶层patch，分别修改4个文件，本轮全部成功但仍无整组原子性；
- 证据：`target/r5-final-loop-fix-repeat3/subscription-billing-repair/20260713-002149-397`。

因此J7约束单位必须从“单carrier”提升为“单provider response”。仅修改`taskspace_control` schema无法约束
顶层兄弟工具调用数量；request-wide计数必须在共享tool-sequence dispatcher中、任何工具执行前完成。

## 2. 本轮证据分类

| Item | Classification | Current Decision | Reason |
|---|---|---|---|
| 分层目录发现与按文件读取 | Observation | 保留观察，不改 schema、不加 gate | 未发现同一路径/同一范围在反馈可见后无意义重读，也未发现读取结果丢失或扭曲 |
| 同一 carrier 三个 `apply_patch` | Confirmed defect | 纳入 J7 工具契约升级 | schema 允许重复写动作，失败会形成已执行/失败/skipped 的不完整批次 |
| 同一response多个顶层`apply_patch` | Confirmed defect | request-wide共享工具序列硬约束 | 单工具JSON Schema不能约束兄弟调用数量，最新repeat-2已产生部分提交 |
| 单个 multi-file patch 的失败原子性 | Confirmed substrate defect | 作为 J7 前置门禁 | 当前实现和测试都允许 validation failure 后保留前序文件改动 |
| 额外一次 pytest | Observation | 跨复杂样本继续观察 | 单样本不足以证明重复验证失当，不增加工具限制 |

读取观察只增加统计口径：记录读取目标、范围、内容 hash、反馈进入下一请求的证据，以及反馈可见后的完全重复读取。
不得把“读取次数较多”直接归类为失败，也不得由 runtime 判断某次读取是否有语义价值。

## 3. 目标与非目标

### 3.1 目标

1. 一个provider response在Standard和TaskSpace中合计最多只能声明一个`apply_patch`，计数覆盖顶层调用、
   `taskspace_control` singular patch slot及nested tool alias。
2. 多文件相关修改由 Agent 在一个 `apply_patch` 输入中完整声明；runtime 不合并、不拆分、不重排 patch。
3. 单个 patch 的所有语法、路径和上下文校验在首个文件写入前完成；此类 validation failure 必须零文件副作用。
4. patch 成功后仍允许在同一 carrier 中执行 Agent 已声明的 read/test 等普通动作。
5. 非 patch 的多工具能力保持不变；多个读取、多个测试以及互不依赖的普通工具仍可并行或串行声明。
6. carrier schema、typed parser、request tool-sequence preflight和tool description表达同一份机械契约；
   schema无法表达的response-wide计数由执行前dispatcher硬校验补齐。
7. patch 成功、validation failure、commit failure、tail skipped 等事实忠实进入反馈与日志，不做语义改写。

### 3.2 非目标

1. 不限制 Agent 读取次数，不判断读取是否“过度严谨”。
2. 不限制 pytest 次数，不由 runtime 决定测试是否必要。
3. 不解析 reasoning 或自然语言计划来识别“相关修改”。
4. 不自动合并多个 patch，不静默丢弃第二个 patch，不只执行第一个 patch。
5. 不把整个 `actions[]` 粗暴限制为 `maxItems=1`。
6. 不把 shell 命令按正文推断为“潜在写操作”；J7 只约束工具身份明确的 `apply_patch`。
7. 不通过减少 Map 节点、自动 finish 或压缩语义来制造 request 收益。
8. 不保留旧 multi-patch carrier 兼容形态。
9. 不只约束TaskSpace而让Standard保留同类部分写风险；request-wide singularity与patch prepare/commit均走共享路径。

## 4. 设计原则

1. **Tool-first**：carrier内合法形态首先由tool schema表达；跨兄弟调用计数由共享tool-sequence preflight表达，
   不依赖提示词劝导。
2. **Agent-owned intent**：patch 内容、文件集合、顺序和后续动作均由 Agent 声明。
3. **Runtime mechanical baseline**：runtime 只验证工具形状、可见性、状态机和资源硬规则。
4. **Preflight before side effect**：整包结构校验和 patch validation 必须先于状态提交或文件写入。
5. **No post-hoc punishment**：dispatcher必须先读取完整response tool sequence再执行；不得先执行第一个patch，
   再拒绝第二个patch或注入recovery prompt。
6. **Faithful feedback**：不得把 failed/skipped 改写为 success，不摘要掉原始 patch 错误和受影响路径。
7. **No false atomicity claim**：语义校验零副作用与底层 I/O 全事务是两个不同承诺，必须分别验证和命名。

## 5. 外部依据与本地事实

### 5.1 外部依据

1. [DeepSeek Function Calling](https://api-docs.deepseek.com/guides/function_calling/) 说明 strict mode 会按
   function JSON Schema 约束输出，但使用 beta endpoint，且只支持 JSON Schema 子集。J7 必须先做目标 endpoint
   probe，不能假设 `contains/maxContains` 可用。
2. [DeepSeek Chat Completion API](https://api-docs.deepseek.com/api/create-chat-completion/) 明确 function 参数由
   JSON Schema 描述，provider 可以在一次响应中生成一个或多个工具调用。单 patch 约束必须作用于
   `taskspace_control` 自身参数，而不能假设 provider 会替 runtime 管理兄弟调用。
3. [JSON Schema Draft 2020-12 Validation](https://json-schema.org/draft/2020-12/json-schema-validation) 定义了
   `contains`、`minContains` 和 `maxContains` 的数组成员计数语义；这是保留有序数组时的标准表达，但是否被目标
   provider 接受必须实测。
4. [OpenAI Function Calling strict reference](https://platform.openai.com/docs/api-reference/fine-tuning/event-object?lang=python)
   同样强调 strict 只支持 JSON Schema 子集，且执行函数前仍应由应用侧验证参数。Whale 的本地 typed parser 和
   preflight 不能因 provider schema 存在而省略。

### 5.2 已确认的本地事实

| Fact | Evidence | Design Impact |
|---|---|---|
| J6 nested action 已嵌入原工具精确参数 schema | `tools/src/taskspace_tool.rs`，commit `81d2702` | J7 必须复用该单一来源，不手写第二份 patch 参数 schema |
| typed args 只禁止递归 control/update_plan | `handlers/taskspace_control_args.rs::validate_nested_actions` | 当前没有重复 patch 计数或 singular patch 类型 |
| runtime 先构造全部 nested calls，再提交 state，然后串行执行 | `tools/sequence.rs::execute_taskspace_barrier` | carrier 形状必须在 state commit 前完成预检 |
| nested 首错后尾部明确 skipped | `tools/sequence.rs` | 三 patch 批次天然可能形成 partial + skipped |
| `apply_patch` 逐 hunk 读写 | `apply-patch/src/lib.rs::apply_hunks_to_files` | 合并文本仍可能在后序 validation failure 时部分写入 |
| partial success 被测试固化 | `apply-patch/tests/suite/tool.rs` | J7.1 必须主动变更契约和测试，不能只改 TaskSpace schema |
| J6 complex run 中首 patch 成功、次 patch 失败 | `target/j6-complex-b/.../20260712-042255-022` | 作为 J7 固定回归基线 |
| J6.7后active Map允许顶层ordinary tools | `session/turn.rs` tool visibility、J6.7 live trace | carrier schema不能覆盖顶层兄弟patch |
| 最新repeat-2顶层5 patch形成4 success + 1 failure | `target/r5-final-loop-fix-repeat3/.../pair-002/left` | request-wide preflight必须先于全部工具执行 |
| 最新repeat-3顶层4 patch全部成功 | `target/r5-final-loop-fix-repeat3/.../pair-003/right` | 成功样本也不能证明多写批次安全 |
| stable/beta strict均接受`contains/maxContains`但仍生成2个patch | `target/r5-j7-schema-probe/singular-patch-capability.json` | provider关键字不可作为约束；选择显式singular continuation |
| function/custom patch最终共享同一`apply_patch`身份 | `tools/src/apply_patch_tool.rs`、Chat endpoint mapper | manifest按canonical tool name计数，payload carrier只影响提取方式 |
| `ExecutorFileSystem`无rename/transaction API | `exec-server/src/file_system.rs::ExecutorFileSystem` | validation atomicity必须实现；I/O transaction不得虚假承诺 |

## 6. 目标工具契约

### 6.1 Request-wide singular patch invariant

共享provider response dispatcher在任何ToolRouter调用、Map transition或文件副作用前，先把本次response中全部
工具声明机械展开为`ToolSequenceManifest`。patch计数范围为：

```text
direct top-level apply_patch
+ taskspace_control.continuation.patch
+ nested ordinary action resolved to apply_patch
= request_patch_count
```

工具身份按ToolRouter解析后的canonical tool name计数，不依靠自然语言、reasoning或shell正文推断。硬规则：

```text
request_patch_count <= 1
```

若计数大于1，整个response tool sequence在执行前失败：

- 不提交Map或node状态；
- 不执行patch、read、test或其他兄弟工具；
- 不修改文件；
- 为每个provider tool call保留闭合的机械failure/skipped结果，reason code固定为
  `request_multiple_apply_patch_calls_not_allowed`；
- 不自动合并patch、不只执行第一个、不生成策略性recovery内容。

该规则位于Standard和TaskSpace共享的response tool-sequence入口。TaskSpace carrier schema提供更早、更显著的
机器可读约束，但不是request-wide正确性的唯一防线。

### 6.2 TaskSpace carrier推荐形态

J6.6之后active Map已经通过provider response中的原生顶层ordinary tools表达后续动作，不再存在
`finish_then_actions`。因此J7.2只替换bootstrap的`initialize_then_actions.actions[]`，将其收敛为互斥
`continuation`；active control schema继续只表达Map状态操作，patch由共享request manifest覆盖：

```json
{
  "action": "initialize_then_actions",
  "initial_nodes": ["Agent 声明的初始节点"],
  "current_node_id": "implement",
  "continuation": {
    "kind": "patch_then_actions",
    "patch": {
      "tool_name": "apply_patch",
      "input": "*** Begin Patch\n...multi-file patch...\n*** End Patch"
    },
    "actions": [
      {"tool_name": "exec_command", "arguments": {"cmd": "pytest -q"}}
    ]
  }
}
```

合法 continuation 只有两类：

| Kind | Shape | Constraint |
|---|---|---|
| `actions` | `actions: [non_patch_action, ...]` | 至少一个普通动作；列表中结构上不存在 `apply_patch` |
| `patch_then_actions` | `patch: exact_apply_patch_payload` + `actions: [non_patch_action, ...]` | patch 恰好一个；尾部普通动作可为空 |

该形态保留 `init map + patch + test` 的单carrier能力，同时让bootstrap内重复patch在schema中不可表达。
active Map中的`finish + patch + test`继续通过同一provider response的顶层control barrier、patch和test表达。
`patch`字段直接从当前request的model-visible `apply_patch` ToolSpec派生；若本轮未暴露`apply_patch`，则不生成
`patch_then_actions`分支。

### 6.3 Provider 选择门禁

J7.0 必须对两个方案做真实 wire probe：

| Option | Shape | Decision Rule |
|---|---|---|
| A | 保留 `actions[]`，用 `contains + maxContains=1` | 仅当目标 endpoint 接受、模型稳定生成且本地 schema 类型可忠实 round-trip 时采用 |
| B | 显式 `continuation.kind=actions/patch_then_actions` | 推荐默认；不依赖数组成员计数关键字，约束更显著 |

若两种结构都不能被目标 provider 稳定接受，carrier schema保持当前可用最小形态并明确报告能力缺口；
request-wide共享preflight仍是硬门禁。不得退回description-only或执行后拒绝。

### 6.4 Typed parser 与 request preflight

无论 provider 是否 strict，本地必须在任何状态提交前验证：

1. continuation 只匹配一个互斥分支；
2. `actions` 分支不含 `apply_patch`；
3. `patch_then_actions` 只有一个 singular patch，尾部 actions 不含 `apply_patch`；
4. patch tool 在当前 request 可见，payload 类型和参数符合原 ToolSpec；
5. nested action 仍不得调用 `taskspace_control` 或 `update_plan`。

随后共享dispatcher对完整response执行request-wide计数。顺序必须固定为：

```text
parse all response tool calls
  -> resolve canonical tool identities
  -> expand declared carrier/nested tool identities without executing
  -> validate request_patch_count <= 1
  -> validate carrier schema/visibility/payload
  -> only then submit state transitions and dispatch tools
```

非法旧carrier形态返回`multiple_apply_patch_actions_not_allowed`；跨顶层/嵌套调用超限返回
`request_multiple_apply_patch_calls_not_allowed`。两者均明确“未提交state、未执行工具、未修改文件”。错误不得
附带语义建议、自动修复内容或recovery strategy。

## 7. Patch 执行契约

### 7.1 Prepare / Commit 分离

`codex-apply-patch` 先构造不可变的 `PreparedPatch`，再进入 commit：

```text
parse all hunks
  -> resolve all source/destination paths
  -> read all required originals
  -> compute every resulting file body
  -> validate every context/path conflict
  -> produce PreparedPatch + original snapshots
  -> only then begin filesystem mutation
```

硬门禁：parse、missing source、context mismatch、invalid move、path conflict 等 validation failure 均不得修改任何文件。
这直接覆盖 J6 complex sample 的失败类型。

### 7.2 I/O 事务边界

J7.0 必须先盘点 `ExecutorFileSystem` 是否具备跨文件 staging、atomic rename、backup/restore 所需原语，并冻结承诺：

| Level | Required Semantics | J7 Decision |
|---|---|---|
| Validation atomicity | 所有可预检失败在首个写入前返回 | 必须实现，J7.1 硬门禁 |
| Single-file replacement atomicity | 更新文件通过同目录临时文件 + rename 或等价原语提交 | 能力存在时必须实现 |
| Cross-file I/O transaction | 任意中途 I/O failure 后所有文件自动回滚 | 仅在 rollback 可验证时承诺；不得用 best-effort 假装原子 |

若底层无法保证跨文件 I/O 事务，commit failure 必须返回结构化的 `committed_paths`、`pending_paths` 和
`rollback_status`。这不是允许 validation partial success，而是诚实暴露不可消除的底层故障边界。

## 8. 分阶段实施计划

### J7.0：证据冻结与能力探针

**Entry:** J6.7 已完成并冻结canonical event/carrier边界；complex trace 和当前 patch partial-success test 可重放。

**Work:**

1. 固化三patch carrier、顶层5 patch partial success、顶层4 patch success、首错停止和恢复请求的trace fixture。
2. 对 Option A/B 执行本地 schema round-trip 与目标 DeepSeek endpoint wire probe。
3. 盘点function/custom两种`apply_patch` ToolSpec、当前Chat API映射及共享response tool-sequence入口。
4. 盘点 `ExecutorFileSystem` staging/rename/restore 能力，冻结 validation atomicity 与 I/O transaction 的准确承诺。
5. 记录读取/pytest 为 observation-only，不创建对应 gate。

**Exit:** schema 形态、底层原子性边界、测试夹具和回退点全部有证据；任何 Unknown 不得带入生产实现。

### J7.1：`apply_patch` 全量预检

**Entry:** J7.0 100% 通过。

**Work:**

1. 把 patch 处理拆为 prepare 与 commit 两个明确阶段。
2. prepare 对全部 hunk 完成上下文、路径和目标内容计算，不产生文件副作用。
3. 按 J7.0 冻结的 I/O 能力实现单文件原子替换及可验证的 rollback；不能保证的边界结构化暴露。
4. 删除“validation failure after partial success leaves changes”的旧契约测试，替换为零副作用断言。
5. Standard 与 TaskSpace 共用同一实现，不建立 TaskSpace 专用 patch 分支。

**Independent verification:** `codex-apply-patch` 单元/CLI 测试，包含 add+later-missing、update+later-context-mismatch、
move/delete、write failure 和 rollback failure injection。

**Exit:** 所有 validation failure 零文件副作用；I/O failure 行为与 J7.0 承诺完全一致。

### J7.2：冻结 singular patch carrier schema与request manifest

**Entry:** J7.1 通过，目标 provider probe 通过。

**Work:**

1. 直接替换 J6 宽泛 continuation，不保留旧 multi-patch schema 或 normalizer。
2. ordinary action union 排除 `apply_patch`，singular patch 字段复用原 ToolSpec。
3. typed enum 与 JSON Schema 从同一结构派生，禁止 schema/parser 双轨。
4. tool description 只描述合法形态、执行顺序和失败停止，不增加策略性提示。
5. 定义共享`ToolSequenceManifest`和canonical patch identity，覆盖Standard顶层调用与TaskSpace carrier/nested声明。

**Independent verification:** schema snapshot、provider request body、positive/negative typed fixtures。

**Exit:** 一个carrier中第二个`apply_patch`在机器可读契约中不可表达；完整response可在执行前稳定计算
`request_patch_count`；patch + test仍可表达。

### J7.3：Runtime 预检与忠实反馈

**Entry:** J7.2 通过。

**Work:**

1. 完整provider response的tool identities、patch count、visibility和payload校验全部早于任何state transition或工具执行。
2. singular patch 继续经 ToolRouter/ToolCallRuntime 执行，不绕过权限、沙箱、hook、取消和日志。
3. patch failure 后尾部 ordinary actions 继续使用现有 explicit skipped 语义。
4. 删除旧 multi-patch batch 测试和任何 description-only 兼容路径。
5. 错误反馈保留原 patch stderr/output ref、失败类别和零副作用声明。
6. Standard与TaskSpace共用同一request-wide validator；不得在TaskSpace runtime复制第二份计数逻辑。

**Independent verification:** state snapshot + filesystem snapshot 负例、权限/hook/sandbox 集成测试、raw feedback 等价测试。

**Exit:** 非法carrier或request-wide multi-patch对state/filesystem均零副作用；合法单patch request的native tool行为无回退。

### J7.4：Observer 与回归门禁

**Entry:** J7.1-J7.3 通过。

新增机械指标：

```text
single_patch_carrier_count
multi_patch_carrier_attempt_count
request_patch_count
request_multi_patch_attempt_count
request_multi_patch_preflight_reject_count
multi_file_patch_count
patch_prepare_failure_count
patch_commit_failure_count
patch_partial_commit_count
post_patch_action_count
post_patch_skipped_count
```

读取观察指标只进入报告：`unique_read_target_count`、`exact_repeat_read_after_visible_feedback_count`、
`read_feedback_visibility_coverage`。不得据此触发 runtime 拒绝。

**Exit:** observer 能从真实 rollout 区分 schema failure、patch prepare failure、commit failure 和普通工具失败；日志不记录
patch 正文、文件正文或 secret。

### J7.5：Docker 样本与收益验收

**Entry:** locked binary attestation、测试和 observer 门禁全部通过。

**Samples:**

1. `multi-file-order-pipeline`：直接覆盖 J6 三 patch 回归。
2. 一个包含多文件修改、失败注入和验证动作的复杂 sample；若现有 sample 不足则新增固定 fixture。

每个 sample 各执行一次 Standard、R4 历史基线和 R5-J7 Docker run。读取与 pytest 只观察，不以单次差异触发新限制。

**Correctness gate:**

- Standard/R5 最终结果正确；
- validation failure 后 workspace hash 不变；
- Standard/R5 request-wide multi-patch executed = 0；
- R5 multi-patch carrier accepted/generated = 0；
- protocol/state failure = 0；
- 权限、沙箱、hook、取消和原始反馈无回退；
- Map node/edge/result health 不下降。

**Benefit gate:**

- Standard/R5单个provider response的patch action最大值为1；
- 同一TaskSpace carrier的patch slot最大值为1；
- 因前一个 patch 失败而 skipped 的后续 patch 数量为0；
- 相关多文件修改通过一个 prepared patch 表达；
- `finish + patch + test` 仍可在一个 carrier 完成；
- request、token、cache、wall time 完整分账，但不预设一定下降；
- 收益不得来自减少读取、减少测试或 Map 坍缩。

**Exit:** correctness 与工具结构收益同时通过才关闭 J7。成本未改善时只声明工具契约和副作用边界收益。

**首次实施结果（2026-07-13，历史）：** J7.5 已执行，11/14 acceptance gates verified，但严格退出门禁未通过。
非法 multi-patch response 零执行、multi-file prepare/commit 和忠实反馈收益成立；billing R5 首次仍声明4个
patch，order R5 出现4次state failure并留下4个open node。J7保持paused。完整证据、成本与R4 unavailable
说明见 `38-r5-j7-phase5-docker-benefit-result.md`。

### J7.6：TaskSpace Control 契约忠实性修复

**历史 Priority:** 在首次 J7.5 复验后执行；该门禁已由 J7.6-J7.8 和最终复验关闭。

J7.5 order trace 证明状态机提交正确，但 success ack 删除了 init/finish 的节点身份，且 active schema 的
existing/create next 与 terminal 形状存在不必要歧义。J7.6 只修改 tool contract、typed parser、忠实回执与
可观测性：不自动 finish/bind/create，不增加 projection 提示，不放宽硬状态规则。详细 phase、字段合同、日志、
回退和 Docker 门禁见 `39-r5-j7-6-control-contract-fidelity-plan.md`。

**Exit:** J7.6 工程门禁通过后重跑 order/billing；只有 control failure=0、success identity coverage=100%、
Map open=0 且外部验证通过，才重新判定 J7.5。

**Result（2026-07-13）：** success identity coverage 100%、repeat committed finish 0、两组 Map open 0，H-025
验证关闭；order 新暴露 terminal self-loop affordance 并产生1次state reject。J7.5 重算为12/14，继续 paused。
详见 `40-r5-j7-6-control-contract-fidelity-result.md`。

### J7.7：Terminal finish chain

**Result:** J7.7 已关闭 H-026 工程缺口；H-027/J7.8 与 J7.5 最终复验也已完成。

`finish_then_end` 改为唯一的 Agent 显式 `finish_node_ids[]` 有序链：最后一个 ID 是 terminal，前面每个节点只
机械绑定数组下一个 ID。删除 `preceding_finishes`、`terminal_node_id` 和全部旧解析路径；全链在 Action Map clone
上通过后一次提交，失败零部分状态。详细合同、日志、测试和 Docker 门禁见
`41-r5-j7-7-terminal-finish-chain-plan.md`。

**Exit:** 两组 R5 terminal duplicate/protocol/state failure=0、identity missing/repeat finish=0、Map open=0，
并满足 J7.5 correctness gate。

### J7.8：Control Map 状态反馈

**Result（2026-07-13）：** mutation 回执统一包含 `state_commit` 与 canonical `map_state`，terminal 原子失败明确
零提交；不添加动作建议或 Runtime 自动状态推进。focused/regression/build/attestation 均通过。最终 order/billing
Docker 复验中两组 R5 state/protocol failure、map-state missing、terminal bad commit、identity missing、repeat finish、
Map open 均为0，J7.5 14/14 gate 关闭。详见 `42-r5-j7-8-control-map-state-feedback-plan.md` 和
`38-r5-j7-phase5-docker-benefit-result.md` 第9节。

## 9. Phase Gate Matrix

| Phase | Independent Verification | Forbidden Future Dependency | Exit Evidence | Completion Required | Proceed Decision |
|---|---|---|---|---|---|
| J7.0 | trace fixture、schema wire probe、FS capability audit | 不依赖生产 handler | schema/atomicity 决策无 Unknown | 100% | proceed J7.1 |
| J7.1 | apply-patch unit/CLI/fault injection | 不依赖 TaskSpace schema | validation failure 零副作用 | 100% | proceed J7.2 |
| J7.2 | schema snapshot、typed fixtures、request manifest、provider body | 不依赖 runtime observer | carrier multi-patch不可表达；request count可计算 | 100% | proceed J7.3 |
| J7.3 | Standard/TaskSpace response sequence、state/filesystem snapshots、router/security integration | 不依赖 live sample | request-wide预检顺序与反馈完整 | 100% | proceed J7.4 |
| J7.4 | telemetry schema/extractor fixtures | 不依赖 J7.5 补证 | 失败类别和读取观察可分账 | 100% | proceed J7.5 |
| J7.5 | Docker Standard/R4/R5 samples | 无后续 phase 补证 | correctness + structural benefit | 100% | complete；14/14 |

## 10. Implementation Completeness Matrix

| Plan Item | Expected Behavior | Production Code Path | Test Evidence | Runtime / Log Evidence | Status |
|---|---|---|---|---|---|
| evidence and provider probe | 冻结真实失败和可用 schema | benchmark artifacts / ToolSpec serialization | wire fixtures | provider body/hash | complete |
| patch prepare/commit | validation failure 零副作用 | `apply-patch/src/transaction*.rs` | 64 lib + 22 CLI/scenario + fault injection | structured commit error | complete |
| singular patch schema | bootstrap carrier最多一个patch；active走native siblings | `tools/src/taskspace_tool.rs` | 3 schema tests | model-visible ToolSpec | complete |
| typed carrier parser | schema/parser单一契约，旧`actions[]`拒绝 | `taskspace_control_args.rs` | 19 parser/handler tests | protocol reason code | complete |
| request tool manifest | 顶层/carrier/nested统一计算patch count | `tools/sequence_manifest.rs` + shared sequence入口 | 2 manifest + 6 sequence + 8 scenario tests | `tool.request_patch_count_validated` | complete |
| pre-state request validation | 非法request不执行任何工具、不提交Agent声明state、不改filesystem | `sequence_preflight.rs` + shared sequence | 9 unit + 2 zero-side-effect integration tests | validated/rejected events | complete |
| native patch dispatch | 合法单patch继续走权限/沙箱/hook/反馈原链路 | ToolRouter/ToolCallRuntime | 9 scenario + 16 core apply_patch tests | canonical call ids | complete |
| observer | patch lifecycle和显式读取观察可分账，不暴露payload | `patch-observability.ps1` + performance observer | extractor/performance/skill tests | lifecycle counts + request rows | complete |
| benefit proof | 结构收益且无负向收益 | Docker runner | paired sample | report artifacts | complete：14/14 gates；J7 closed |

## 11. Change-chain Logging Matrix

| Change Link | Key State | Success Signal | Failure Signal | Failure Reason | Correlation | Level |
|---|---|---|---|---|---|---|
| carrier schema parse | parsed/validated | model-visible schema + typed parser | existing typed invalid-arguments output | `reason_code` | `outer_call_id` | tool feedback |
| request patch preflight | counted/validated | `tool.request_patch_count_validated` | `tool.request_multi_patch_rejected` | `reason_code` | enclosing turn/response sequence | info/warn |
| patch prepare | prepared | `apply_patch.prepare_completed` | `apply_patch.prepare_failed` | `stage/hunk_count` | enclosing tool span `call_id` | info/warn |
| patch commit | committed | `apply_patch.commit_completed` | `apply_patch.commit_failed` | affected/rollback path counts | enclosing tool span `call_id` | info/warn |
| nested patch execution | completed | existing tool success | existing tool failure | native reason | `outer_call_id/derived_call_id` | info/warn |
| post-patch actions | completed | batch completed | tail skipped | `prior_failed_call_id` | `outer_call_id/action_index` | info/warn |
| read observation | observed | performance rollout extractor | coverage unavailable | explicit read identity only | `call_id/request_index` | report-only |

日志只记录数量、阶段、状态和关联id；observer输出只保留聚合计数和request index。不得记录patch正文、路径正文、
文件正文或secret。

## 12. 风险、替代方案与回退

| Risk | Probability | Impact | Mitigation | Fallback |
|---|---:|---:|---|---|
| provider 不支持目标 schema | Medium | High | J7.0 两种形态真实 probe | 暂停，不退回 prompt-only |
| singular patch schema 增大 tools payload | Medium | Medium | 复用原 ToolSpec、记录 tools hash/token/LCP | 回退 J7.2，重新简化结构 |
| patch prepare 改变既有覆盖/移动行为 | Medium | High | 现有 apply-patch suite 全量回归 + golden diff | 回退 J7.1 |
| I/O rollback 无法可靠实现 | Medium | High | 分离 validation atomicity 与 I/O transaction 承诺 | 只声明已验证层级并忠实反馈 partial commit |
| patch 后测试动作因 patch 失败被 skipped | Expected | Low | 保留现有明确 skipped 语义 | Agent 下一请求基于失败反馈决定 |
| Agent 改用 shell 绕过 patch 形态 | Low | Medium | 只观察工具选择，不做正文语义识别 | 作为独立产品问题评估 |
| 读取观察被误用为行为 gate | Medium | High | 指标标记 observation-only，禁止 runtime consumer | 删除对应 consumer |

明确拒绝的方案：

| Alternative | Decision | Reason |
|---|---|---|
| 只在 description 中写“请合并 patch” | Rejected | schema 仍允许多个 patch，约束不一致 |
| runtime 自动合并 patch 文本 | Rejected | 改写 Agent 声明，且不能解决底层 partial apply |
| 执行第一个、静默丢弃其余 patch | Rejected | 语义丢失，反馈失真 |
| 执行后再拒绝 multi-patch | Rejected | 已产生状态/文件副作用，属于后置惩罚 |
| 整个 actions 最大长度设为1 | Rejected | 破坏多个 read/test 和 patch 后验证能力 |
| 用提示词要求少读文件 | Rejected | 当前读取没有实际错误，runtime 不应判断语义价值 |

回退以 phase commit 为单位。J7.2/J7.3 必须同组回退，禁止新 schema 与旧 parser/runtime 混用；J7.1 为 shared
`apply_patch` 独立 commit，可在不保留 TaskSpace 兼容分支的前提下单独回退。

## 13. Open Questions

| Question | Resolution Phase | Blocking Rule |
|---|---|---|
| 目标 DeepSeek endpoint 是否支持 `contains/maxContains` | Resolved J7.0 | 接受schema但stable/strict均生成2 patch，不采用Option A |
| function/custom apply_patch 是否能共享 singular payload builder | Resolved J7.0 | 复用model-visible ToolSpec；manifest按canonical identity归一 |
| ExecutorFileSystem 能否可靠 staging + atomic rename | Resolved J7.0 | 不能；trait无rename/transaction，禁止声明I/O atomicity |
| rollback failure 如何结构化报告 | Resolved J7.0 | 报告committed/pending/restored/rollback_failed paths及status |
| patch 前普通动作是否需要进入同一 continuation | Resolved J7.0 | 默认不需要；有结果依赖时下一provider request |

## 14. Decision Log

| Decision | Status | Reason |
|---|---|---|
| 读取行为只观察 | Accepted | 没有实际错误或反馈丢失证据 |
| pytest 重复只观察 | Accepted | 单样本不足以形成工具约束 |
| 单个 provider response 最多一个 apply_patch | Accepted | 同时覆盖顶层、carrier 和 nested alias，避免写动作批次的 partial/skipped 形态 |
| 多文件 patch 由 Agent 完整声明 | Accepted | runtime 不替 Agent 合并或重写 |
| 先修 patch validation atomicity，再启用 singular carrier | Accepted | 否则“合并成一个 patch”仍可能部分写入 |
| 普通多工具能力保持 | Accepted | 限制只作用于明确写工具身份 |
| 不兼容旧 multi-patch 形态 | Accepted | 实验性产品无历史数据迁移要求 |
| 不采用`contains/maxContains` | Accepted | provider接受但stable/strict实测均未执行maxContains约束 |
| I/O failure不声明跨文件事务 | Accepted | substrate无rename/transaction；只做best-effort rollback和忠实报告 |

## 15. Plan Quality Checklist

- [x] 读取观察与 patch 缺陷已分开分类。
- [x] 根因落在工具 schema 和 patch 执行语义，不归因于 Agent reasoning。
- [x] 明确了 tool、parser、runtime 和 substrate 的责任边界。
- [x] 没有自动合并、静默丢弃、后置惩罚或兼容分支。
- [x] validation atomicity 与 I/O transaction 分开承诺。
- [x] 每个 phase 可独立验证，有退出门禁和回退路径。
- [x] Standard/TaskSpace shared patch substrate、权限、沙箱、hook 和反馈均纳入回归。
- [x] request、token、cache、wall time 和 Map health 纳入收益验证但不预设结论。
- [x] 文档阶段未修改生产代码或测试契约。
