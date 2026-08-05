# TaskSpace Exec Phase A 调用链盘点

- Created: 2026-08-05
- Status: TX-01 completed
- Scope: 当前 Whale vendor、Codex main `5c44f110649f8811546745bb1635ba0b44a1639e`
- Runtime effect: None

## 1. 当前生产请求面

### Standard

```text
TurnContext
  -> core/src/tools/spec.rs::build_specs
  -> ToolRegistry 注册原生 handler
  -> model-visible ToolSpec 列表
  -> provider response 顶层 Tool calls
  -> ToolRouter::dispatch_any
  -> 原 handler / permission / sandbox / hook / result
```

### 当前 TaskSpace sibling 协议

```text
同一顶层 ToolSpec 列表
  -> taskspace_control + sibling ordinary Tool calls
  -> sequence_preflight::validate_tool_sequence
  -> control actions 与 sibling call_id/tool 按位置配对
  -> taskspace_sequence_context 写入 runtime-only node metadata
  -> sequence executor 调用原 ToolRouter
  -> pairing output + supplemental factual message
```

现有协议已经做到普通 Tool 参数不包含 `node_id`，但仍要求 `taskspace_control.actions` 复述 sibling Tool，且合法性发生在
多个顶层调用生成之后。这是 TaskSpace Exec 要替换的协议，不是可继续扩展的目标结构。

## 2. 当前可复用 seam

| 能力 | 当前唯一位置 | Phase A/后续用法 |
|---|---|---|
| 原生 Tool 合同 | `codex-tools::ToolSpec` | 能力快照和内部说明的唯一来源 |
| Function/Freeform/Namespace 派生 | `codex-tools::collect_code_mode_exec_prompt_tool_definitions` | TX-02 先冻结快照身份，TX-06 再抽成中性共享接口 |
| 原生 Tool 执行 | `core/src/tools/registry.rs`、`router.rs` | TX-10 机械还原后复用，不新建业务 Tool executor |
| nested payload 转换 | `core/src/tools/code_mode/mod.rs` | 只复用原生 payload/结果转换，不复用边执行边求值语义 |
| Map 参数解析 | `handlers/taskspace_control_args.rs` | TX-04/Phase B 复用，不复制 Map 业务规则 |
| hosted 原始事实 | `codex_protocol::models::ResponseItem` | TX-05 从 Web/Image output item 提取稳定身份 |
| runtime-only node metadata | `taskspace_sequence_context.rs` | 数据所有权可复用，旧 sibling 形状不可复用 |

## 3. 上游差异及落点决策

最新 Codex 已将本地 `spec.rs` 的职责拆入 `spec_plan.rs`、`hosted_spec.rs`，并以 `ToolExposure` 统一 direct、deferred 和
code-mode 可见性。当前 Whale vendor 尚未同步这组 seam。

因此：

1. TX-02～TX-05 只新增未接生产的 Whale 自有纯组件，基于当前 effective ToolSpec fixture 冻结合同；
2. TX-06 开始前先同步或等价移植上游的中性 Tool planning/exposure seam；
3. 不在当前 `spec.rs` 中建立第二个长期 TaskSpace catalog，也不把最新上游文件整段覆盖本地修改；
4. TaskSpace Exec 的 identity 必须由最终 model-visible/nested 可执行的同一快照计算，不能从配置列表或 description
   文本反向猜测。

## 4. 旧入口删除清单

以下对象在 TX-15 原子切换验证前保留，在 TX-16 一次删除，不做兼容：

- `sequence_preflight.rs` 中 TaskSpace control-first、actions/sibling 数量与位置配对；
- `sequence_manifest.rs` 的旧 TaskSpace manifest 事实；
- `taskspace_sequence_context.rs` 中依赖 sibling `call_id/call_index` 的旧载体形状；
- `provider_tool_declaration.rs` 中“隐藏 hosted Tool 后拒绝 output”的旧声明逻辑；
- `taskspace_hosted_binding_contract_tests.rs` 的单 `hosted_node_id` 整批绑定原型；该原型已被 A2 回撤结论否定，只保留为
  迁移反例，不得进入生产合同；
- 旧 pairing receipt、supplemental factual message 和相关 prompt/schema 合同。

普通 Tool handler、Router、permission、sandbox、hook、Map store、provider 原始 output item 不在删除范围。

## 5. TX-01 结论

- Function 超级 Tool、内部 ToolSpec 派生和原 Router dispatch 均有现成 seam；
- 任意 JavaScript 实时执行不能提供 TaskSpace 所需的“完整计划先于副作用”保证；
- Web Search 和 Image Generation 的 output item 均有 provider item identity 字段，但 Web Search ID 可缺失，必须在
  TX-05 fail closed，不能按内容或语义猜配，也不能写 Root/unbound 后继续；
- Phase A 可以在不接生产入口的条件下继续；Phase B 的共享 catalog 接线需调整为“先对齐上游 ToolExposure seam”。
