# R8-I01 唯一最终 Revision 反馈修复计划

- Created: 2026-08-01
- Artifact status: drafted
- Issue status: investigating
- Plan mode: authoring
- Scope: 三种 TaskSpace projection policy 共用的 response 事务与反馈链路
- Standard impact: 只做隔离回归，不改变 Standard 行为
- Real Whale Agent runs in planning: 0
- Cache gate authorization: 用户已批准 I01 实施期间执行免费门禁检查，并按已解释的 final-wire 变化更新确定性合同与快照；不包含真实 provider 运行或 accepted baseline 晋升

## 1. 产品问题

Agent 在一次 TaskSpace response 中同时提交 `taskspace_control` 和一个或多个普通 Tool。当前实现先保存 Map
变更与普通动作预留，立即为 `taskspace_control` 构造一份带中间 revision 的成功结果；普通 Tool 执行结束后，
结果归档继续推进 canonical Map，Runtime 又通过独立 developer message 返回最终 revision。

因此，一次 response 会给 Agent 两个看起来都能用于下一次提交的成功 revision。Agent 使用较早值时会收到
`stale_revision`，随后需要额外读取、纠正和重试。

产品应有表现是：一次 response 全部结算后，原 `taskspace_control` Tool call 只返回一份最终结果，其中只有
`canonical_revision` 是下一次 Map 写操作的权威 revision。普通 Tool 继续返回原生结果，不承担 TaskSpace 状态
反馈。

## 2. 当前 HEAD 事实与证据边界

### 2.1 已确认实现事实

| Evidence | 当前事实 | 位置或验证 |
|---|---|---|
| E1-code | sequence 在普通 Tool 执行前用 `prepared.model_visible_result()` 构造 `taskspace_control` 成功输出，其中包含 `revision_after` | `core/src/tools/sequence.rs::execute_prepared_taskspace_siblings()` |
| E1-code | 每个普通 Tool 结果通过 `release_main_action_result()` 独立归档，并继续推进 canonical Map revision | `core/src/action_map/runtime/transactions.rs`、`core/src/session/taskspace_response.rs` |
| E1-code | 全部 sibling 处理后，sequence 读取最终 Map，再追加 `role=developer` 的 `TaskSpaceResponseFinalReceiptV1` | `core/src/tools/sequence.rs`、`core/src/action_map/response.rs` |
| E2-test | 当前确定性测试证明最终 receipt revision 晚于 reservation revision，并可被下一次 execute 使用 | `cargo test -p codex-core response_final_receipt_revision_is_accepted_by_the_next_execute -- --nocapture`：1 passed |
| E3-history | R7.1 三种 policy 都出现过隐藏 result attribution 后使用旧 revision 的 stale；该证据只作为线索 | `build-R7/47-r7.1-global-issue-register-legacy.md`、`coe/2026-07-29-04-11-r7-a2-c-revision-feedback-gap.md` |

### 2.2 尚未重新证明的内容

- 当前 HEAD 上三种 policy 的真实 Agent stale 频率尚未复验；历史 E3 不直接晋升为 R8 当前结论。
- 当前源码中的双反馈结构已经成立，但“它解释当前全部 stale”仍是待证伪假设。
- I01 修复可能移除 I02 已知的独立 system/developer receipt，但不能据此直接关闭 I02 或宣称缓存收益。

## 3. 影响范围

| 模式或路径 | 是否属于 I01 | 原因 |
|---|---|---|
| `map-always` | 是 | 使用共用 control prepare、普通结果归档和 response finalization |
| `map-append` | 是 | 同上；历史上独立 receipt 的缓存损害在此最明显，但不是 I01 独有 |
| `map-request` | 是 | 即使不自动注入 projection，control feedback 仍走同一 sequence |
| Standard | 否 | 没有 Map revision；只验证修复未改变普通 Tool sequence 与 final wire |
| 单独 `finish_map` | 条件相关 | 没有 ordinary sibling 时不存在 attribution 增量，但必须保持现有终态合同 |
| preflight/prepare 拒绝 | 非主要修复面 | 没有普通 Tool 执行，继续使用当前单一失败结果；作为回归覆盖 |

## 4. 修复不变量

1. Map Store 继续是 canonical Map 唯一事实源。
2. Agent-visible 成功反馈中只能有一个可用于后续提交的 revision 字段：`canonical_revision`。
3. 最终反馈必须属于原 `taskspace_control` 的 `call_id`，不得新建 developer/system/user 消息副本。
4. 普通 Tool 的 schema、输入、handler、成功/失败结果、`call_id` 和输出顺序保持原生。
5. Runtime 只报告最终机械事实，不替 Agent 选择节点、下一动作或修正 `expected_revision`。
6. 不把外部 Tool 执行包进长时间 SQLite 事务；现有 reservation 与 result attribution 提交边界保持不变。
7. 普通 Tool 失败不伪装成 TaskSpace 失败；只要归档完整，control settlement 可以成功，Tool 本身仍明确失败。
8. result attribution 不完整时不得返回伪造成功；必须报告已发生提交、最终已知 revision 和稳定失败原因。
9. 三种 projection policy 共用一个实现，不新增 policy 专属反馈分支。
10. 变更不得依靠提示词、Agent reasoning 解析、自动重试或 sample 特判掩盖反馈缺陷。

## 5. 目标数据流

```text
Provider response: taskspace_control(call-C) + ordinary calls(call-1..N)
  -> request preflight
  -> prepare Map mutation and reservations
  -> execute ordinary calls with native handlers
  -> attribute every executed/failed/skipped result to canonical Map
  -> read final canonical Map once
  -> build one finalized FunctionCallOutput for call-C
  -> return call-C final output + unchanged ordinary Tool outputs
```

关键点是“延迟构造模型可见结果”，不是延迟 Map prepare，也不是把所有写入压成一个数据库事务。sequence 当前本来
就在全部 sibling 处理完成后才把 output vector 交回会话，因此可以在保持 provider Tool call/result 配对顺序的同时，
最后生成并放回原 control 位置。

### 5.1 计划中的成功结果合同

以下为实施目标，不代表当前代码已经具备：

```json
{
  "schema_version": "TaskSpaceResponseResultV2",
  "status": "settled",
  "success": true,
  "state_commit": true,
  "map_id": "map-...",
  "action": "execute",
  "canonical_revision": 12,
  "reserved_actions": [
    {
      "call_index": 0,
      "call_id": "call-1",
      "node_id": "work-1",
      "tool": "exec_command",
      "reservation_id": "reservation-..."
    }
  ],
  "settlement": {
    "prepared_action_count": 1,
    "attributed_result_count": 1,
    "outstanding_reservation_count": 0
  }
}
```

- 不再暴露可竞争的 `revision_after` 或 `reservation_revision_after`。
- prepare revision 仍可保留在内部结构和 tracing 日志中，用于定位事务时序，但不作为 Agent continuation token。
- `status=settled` 只表示 TaskSpace 记账闭合，不表示普通 Tool 业务成功，也不表示整个 Map 已终态关闭。

### 5.2 计划中的不完整结算合同

prepare 已提交但最终归档或 Store 读取失败时，原 control result 应忠实表达部分事实：

```json
{
  "schema_version": "TaskSpaceResponseResultV2",
  "status": "settlement_incomplete",
  "success": false,
  "state_commit": true,
  "map_id": "map-...",
  "action": "execute",
  "canonical_revision": 12,
  "settlement": {
    "prepared_action_count": 2,
    "attributed_result_count": 1,
    "outstanding_reservation_count": 1
  },
  "error": {
    "class": "state_machine",
    "code": "taskspace_response_attribution_incomplete"
  }
}
```

若 canonical Store 无法读取，`canonical_revision` 为 `null`，错误类为 `resource`。Runtime 不猜测 revision，也不
自动重试 Agent 的语义动作。preflight/prepare 阶段零提交的拒绝继续使用现有 `state_commit=false` 结果，不并入
该合同改造。

## 6. 方案比较与决策

| 方案 | 结果 | 优点 | 代价或问题 | 决策 |
|---|---|---|---|---|
| A. 整批结算后回填原 control Tool result | 一个 `call_id`、一个最终 revision；普通 Tool 原样 | 符合 Tool 调用协议；不新增消息；三模式共用 | sequence 必须延迟构造 control 输出，并迁移旧 observer | 采用 |
| B. 给每个普通 Tool result 包装最新 revision | 每个结果都携带 Map 状态 | 最终 revision 就近可见 | 侵入普通 Tool、重复事实、并行完成顺序下产生多个竞争值 | 拒绝 |
| C. 保留 prepare result，再追加独立 developer/system receipt | 维持当前双反馈 | 改动少 | 已确认双重权威；DeepSeek 中途 system 消息破坏前缀缓存 | 拒绝 |
| D. 仅依赖 projection 或 map handle 暴露最新 revision | 不返回 final control result | 代码表面简单 | 三种 policy 暴露时机不同；`map-request` 默认不读 projection；事实可能缺失 | 拒绝 |
| E. 把 Map prepare、普通 Tool 执行和结果归档放进一个 SQLite 长事务 | 数据库只提交一次 | revision 表面单一 | 外部 Tool 时延和失败进入数据库锁范围，错误扩大事务责任边界 | 拒绝 |

## 7. 实施单元

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Verification | Safe Stop / Rollback | Plan Status |
|---|---|---|---|---|---|---|---|---|---|---|
| I01-W0 | 在当前 HEAD 建立可复算复现 | test/evidence | `core/src/tools/sequence_taskspace_tests.rs` | response output characterization | 增加确定性 fixture，逐项解析 control FunctionCallOutput、ordinary outputs、developer receipt 和 Store revision，证明当前 Agent-visible continuation revision 数量为 2 | 不依赖历史 trace 即可复现双反馈结构 | 后续每次改动都能直接证明问题消失，而不是凭字段命名判断 | 测试在旧实现上确认两个权威项，并记录各自 `call_id`、role、revision | 只新增测试；证据与源码不符则停止设计并回到调查 | planned |
| I01-W1 | 冻结唯一最终结果合同 | API/internal contract | `core/src/action_map/response.rs` | `ActionMapPreparedResponse`、`ActionMapResponseFinalReceipt` | 增加由 prepared facts 与最终 canonical settlement 共同生成 `TaskSpaceResponseResultV2` 的纯构造函数；成功结果只含 `canonical_revision` 这一 continuation revision | 同一结构完整表达动作接受、归档闭合和最终状态 | Agent 不再需要在两个成功事实中猜选 revision；测试可直接校验字段语义 | 纯单元测试覆盖 settled、ordinary failure 已归档、outstanding reservation、Store unavailable | 未接入生产前可单独回退构造函数与测试 | planned |
| I01-W2 | 将最终结果绑定回原 control call | feedback/wiring | `core/src/tools/sequence.rs` | `execute_prepared_taskspace_siblings()` | 删除 sibling 执行前的模型可见 prepare payload；全部归档后构造一个 `FunctionCallOutput`，使用原 `control_call_id` 并放回原 control 对应顺序 | Provider 下一轮只看到一个配对的 control result 和未改写的 ordinary results | 消除 stale revision 的直接反馈来源，同时保持多 Tool 连续执行能力 | sequence 集成测试断言 control output 恰好 1 个、developer receipt 为 0、每个 provider call_id 恰好一个 output | 一个提交只接线此路径；失败时整体回退 W2，不保留双路径或开关 | planned |
| I01-W3 | 保持执行失败与取消语义 | failure semantics | `core/src/tools/sequence.rs`、`core/src/tools/parallel.rs` | failed/skipped/cancelled sibling settlement | 让普通 Tool 失败、前序失败跳过、并行取消和 handler error 都先按现有路径释放 reservation，再生成最终 control result；不改 ordinary output body/success | control 结果只描述 Map 结算，ordinary result 继续描述工具成败 | 避免修复成功路径后在异常路径重新产生隐藏 revision 或悬挂 reservation | 定向测试覆盖 serial failure、skipped、parallel cancellation、timeout、result attribution failure；Map reservation 最终为 0 或明确 incomplete | 任一普通 Tool 输出字节变化或 reservation 静默遗留即停止，不进入 observer 迁移 | planned |
| I01-W4 | 建立最终结算日志 | observability | `core/src/tools/sequence.rs`、`core/src/session/taskspace_response.rs` | response finalization tracing | 用 `taskspace_response_finalized` 记录 control call、Map、prepare revision、final canonical revision、prepared/attributed/outstanding count、status 和稳定 reason code；不记录目标、命令或 Tool 正文 | 成功与不完整结算可由机械身份重建 | 后续定位 stale 不需要重放用户语义，也能区分 prepare 和 final 时序 | tracing capture tests 校验字段、成功/失败事件和敏感正文缺失 | 日志测试不过则停止；日志变更可独立回退，不改变反馈合同 | planned |
| I01-W5 | 迁移 receipt 专属观测 | observability | `core/src/provider_wire_trace.rs`、`core/src/provider_wire_sections.rs`、`scripts/taskspace-benchmark/lib/` | final receipt identity 与 request analyzer | 将 `TaskSpaceResponseFinalReceiptV1` developer-message 识别替换为按原 control `call_id` 识别 `TaskSpaceResponseResultV2` FunctionCallOutput；保持 request/attempt 身份与 section cost 分类 | 报告直接观察真实 control Tool result，不再要求不存在的独立 carrier | 避免代码修好后 benchmark 因旧 observer 产生假阻断或漏报 | Rust wire tests 与 PowerShell analyzer tests 同时覆盖唯一、缺失、重复、incomplete 和文本假阳性 | 先保持 release fail-closed；新 observer 未通过前不得删除旧测试证据 | planned |
| I01-W6 | 删除旧模型可见 carrier | cleanup | `core/src/action_map/response.rs`、`core/src/tools/parallel.rs`、相关测试与当前 benchmark parser | `TaskSpaceResponseFinalReceiptV1` model-visible path | 删除独立 receipt 的 `model_visible_result()`、生产 schema marker、追加 developer message的入口及仅服务该入口的解析；保留必要的内部 settlement 数据但不再称为 Agent receipt | 生产上下文不再存在第二份 revision 权威 | 降低反馈重复、上下文污染和维护两套结果 schema 的成本 | `rg` 证明生产 Rust/当前脚本无旧 schema；历史 docs 可保留；全量相关测试通过 | 若仍有生产 consumer 依赖旧 schema，停止删除并先迁移该 consumer，不增加兼容分支 | planned |
| I01-W7 | 验证三种 policy 共用一条反馈 | compatibility | `core/tests/suite/cache_payload_contract.rs`、final-wire snapshots | TaskSpace two-request fixtures | 对 `map-always`、`map-append`、`map-request` 分别运行同一两请求 fixture，断言 control result schema/call_id/revision 规则一致，差异只存在 projection；同时冻结 Standard 对照 | 三种产品模式不会形成反馈分叉，Standard 不受污染 | 降低后续模式间回归和重复修复成本 | 三 policy final-wire 测试通过；Standard snapshot 与普通 Tool schema/result 基线逐值相同 | 任一 policy 出现专属反馈分支或 Standard diff 即停止并回退 W2-W6 | planned |
| I01-W8 | 通过免费缓存门禁确认影响面 | cache gate | `scripts/cache-regression/`、受影响 snapshots | free final-wire matrix | 运行 index/worktree/HEAD 免费门禁，确认变化只来自旧 developer receipt 消失和 control output 定稿；输出首差异与受影响场景，不手工修改 accepted baseline | 缓存敏感变更在付费前可解释、可复算 | 防止用 I01 修复再次制造未知上下文结构变化 | `python3 scripts/cache-regression/check_cache_regression_gate.py --source index` 给出可比较 changed report；全部离线测试通过 | 报告出现 Tool schema、Standard、prompt 或 projection 非预期变化时停止，不申请真实预算 | planned |
| I01-W9 | 验证真实 Agent 不再因隐藏归档提交 stale | E3/product verification | Docker benchmark、全局 run ledger | 同一客观 sample 的三 policy matrix | 在 W0-W8 通过后单独申请预算，按同 commit/模型/镜像对三种 policy 各 repeat 3；关联每次运行账本和原始 trace | 当前版本中由双 revision 引起的 stale 为 0，业务结果不退化 | 证明工程修复真正减少 Agent 无效纠错，而非只让单测换字段 | `sample × 3 arms × repeat 3 = 9`；`hidden_attribution_stale=0`，业务验证通过，普通 Tool 无异常包装 | 属于 >3 sample 专项预算；未获批准不得执行；任一模式复现则不关闭 I01 | planned |
| I01-W10 | 结算缓存敏感发布证据 | release/cache verification | cache regression proposal、ledger、evidence | changed final-wire scenario set | 根据 W8 自动生成的实际变化范围另行申请最小真实缓存预算；执行后只按精确 commit、模型、arm 和场景晋升或拒绝 | 发布门能够区分 I01 正确性与缓存接受状态 | 不用未验证缓存收益换取语义修复，也不让历史 baseline 错配当前提交 | accepted 或 failed 结果均完整结算 token、费用、请求、耗时和证据路径；失败不得晋升 | 与 I01-W9 预算分开；provider 或证据异常立即停止且不自动重试 | planned |
| I01-W11 | 关闭问题并触发下游重评 | docs/governance | `build-R8/I01/`、`01-r8-known-issues.md` | I01 result and issue state | 记录已证根因、被否定假设、提交、测试、E3、缓存边界和全局约束检查；仅在全部 I01 条件通过后标记 closed，并重新评估 I02/I03/I08 | R8 保持一个权威状态和可追溯关闭依据 | 后续问题不会重复猜测 revision 事实或误用 I01 证据关闭缓存问题 | 结果文档逐项引用 code/test/runtime/review 证据；唯一账本与 Git HEAD 一致 | 任一 closing evidence 缺失则保持 verifying，不使用“基本完成” | planned |

## 8. 阶段与停止条件

### Phase A：当前版本证明与合同冻结

- Entry: I09 已关闭，当前 HEAD 可构建。
- Work units: I01-W0、I01-W1。
- Required evidence: 双反馈 characterization、目标 result contract 的正反例测试。
- Next condition: 当前缺陷与目标合同都能由确定性测试表达；否则停在 investigating。

### Phase B：单一 control feedback 接线

- Entry: Phase A 证据成立。
- Work units: I01-W2、I01-W3、I01-W4。
- Required evidence: 一个 control output、零独立 receipt、异常路径事实完整、日志可定位。
- Next condition: 普通 Tool 输出与 reservation 规则无回归。

### Phase C：观测迁移与旧路径删除

- Entry: Phase B 生产路径稳定。
- Work units: I01-W5、I01-W6。
- Required evidence: 新 observer 可精确识别 control final result；生产路径不存在旧 carrier consumer。
- Next condition: observer fail-closed 测试通过且未引入兼容双路径。

### Phase D：三模式、Standard 与缓存免费门禁

- Entry: Phase C 完成旧路径清理。
- Work units: I01-W7、I01-W8。
- Required evidence: 三 policy 共享反馈、Standard/普通 Tool 保真、缓存变化报告可比较。
- Next condition: 只剩需要用户授权的 E3 和缓存 accepted baseline。

### Phase E：预算验证与关闭

- Entry: 所有免费验证通过，预算提案包含模型、规模、请求/token/费用/耗时上限和停止条件。
- Work units: I01-W9、I01-W10、I01-W11。
- Required evidence: 当前 HEAD 三 policy stale 归因结果、真实缓存结果、全局约束回归。
- Close condition: I01 的唯一 final revision 成立；I02/I03/I08 只重评，不随带关闭。

## 9. 验收矩阵

| 验收面 | Passing standard |
|---|---|
| 唯一权威 | 每个成功 TaskSpace response 恰好一个与 control `call_id` 配对的最终结果；只有 `canonical_revision` 可用于下一提交 |
| 事实完整 | result attribution 完整/不完整、Store unavailable、ordinary failure、skip/cancel 均返回实际状态，不伪造零提交 |
| 普通 Tool 保真 | 同输入下 ordinary output 的类型、`call_id`、body、success 和顺序与改动前逐值一致 |
| 三模式一致 | always/append/request 的 control result schema 与事务语义一致；只有 projection 进入 context 的方式不同 |
| Standard 隔离 | Standard final wire、Tool schema、Tool result 和执行路径无 TaskSpace 相关变化 |
| Map 正确性 | 最终反馈 revision 等于 persistent Store canonical revision；无 outstanding reservation 时 settlement 才成功 |
| 上下文 | Provider wire 中不存在 `TaskSpaceResponseFinalReceiptV1` developer/system message |
| 行为收益 | 获批 repeat-3 中由隐藏 attribution 引起的 `stale_revision` 为 0，业务验证全部通过 |
| 成本纪律 | 所有真实 run 预登记、结算且不超预算；缓存结果只解释实际覆盖场景 |

## 10. 风险

| Risk | Trigger signal | Mitigation | Safe stop |
|---|---|---|---|
| 延迟 control output 破坏 provider call/result 配对 | final-wire 缺失、重复或错序 call_id | 结果在执行后构造，但放回原 control 对应位置；逐 call_id 对账 | 任何 provider fixture 不平衡即回退 W2 |
| ordinary Tool failure 被 control success 掩盖 | Tool 失败但报告只显示 control settled | control success 仅定义记账闭合；保留原 Tool `success=false` 和正文 | ordinary output 发生变化即停止 |
| 不完整 attribution 被误报成功 | outstanding count 非零但 `success=true` | final result builder 以 Store 中 reservation/result facts 机械判断 | 负向测试不过不得接入生产 |
| observer 继续依赖旧 receipt | benchmark 报告 receipt missing 或缓存身份不可比较 | 先迁移 consumer，再删除 producer；不保留兼容解析双路径 | release 保持阻断直到新 observer 通过 |
| I01 顺带被当作 I02 已关闭 | 独立 receipt 消失后直接宣称缓存恢复 | I02 保持 queued；真实缓存证据按独立预算和场景验收 | 只更新 I01 状态，不修改 I02 状态 |
| Runtime 责任扩大 | 修复中出现自动 revision 替换、动作选择或重试 | 只更改事实输出时机和 carrier；硬状态机不变 | 发现语义决策立即停止实现 |

## 11. 外部依据

1. [OpenAI Function calling](https://developers.openai.com/api/docs/guides/function-calling)：一个 response 可以包含
   多个 function call，每个 `function_call_output` 通过原 `call_id` 与调用对应。由此支持把最终 TaskSpace 事实
   返回到原 control Tool result，而不是另造 developer message。
2. [Model Context Protocol - Tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)：Tool result
   是 Tool 调用的结构化反馈面，执行错误也应在对应 result 中返回，使模型能够读取并纠正。由此支持 control 状态
   通过 control result 表达、普通 Tool 错误保持原结果。
3. [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/)：缓存按完整前缀单元匹配；中途插入新的
   system/developer 形态会改变后续前缀结构。由此要求删除独立 receipt，但不能删除最终 revision 事实。
4. [SQLite Atomic Commit](https://www.sqlite.org/atomiccommit.html)：数据库事务应呈现全部发生或全部不发生的原子
   边界。本问题不把外部 Tool 执行扩入 SQLite 长事务；继续让每次 canonical 提交保持既有原子性，并在 response
   末尾忠实报告最终持久状态。

这些资料只约束 Tool 结果归属、错误反馈、缓存前缀和持久化原子边界，不替代 WhaleCode 当前生产调用链证据。
