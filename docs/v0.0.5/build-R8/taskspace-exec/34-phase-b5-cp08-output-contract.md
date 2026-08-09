# Phase B5 CP-08：同源结果合同

- Date: 2026-08-10
- Scope: `taskspace_exec` typed result、Catalog output schema、模型可见返回合同
- Status: verified offline / provider cache revalidation pending

## 1. 已确认的 Provider 边界

`ResponsesApiTool.output_schema` 是 Codex 内部能力元数据，序列化 Provider Function Tool declaration 时会被跳过。
因此不能把它当作 DeepSeek 原生支持的函数返回 schema，也不能仅设置该字段后宣称 Agent 已获得结果合同。

最新 Codex Code Mode 的生产做法是：保留内部 `output_schema`，同时把它机械渲染为 TypeScript 返回类型，放入唯一 outer
Exec Tool description。TaskSpace Exec 沿用这一边界，不增加 developer prompt、feedback carrier 或 Provider 私有字段。

## 2. 实现

1. 新增唯一 typed `TaskSpaceExecResult`，直接承载 `kind/status`、outer call、Map revision、完整 Map read、client
   原生 nested result 和 Hosted 归属结果；handler 不再用手写 `json!` 另建反馈形状；
2. 从同一结果结构生成固定 outer schema，其中 Map read 使用 canonical Agent-visible Map view，client `result` 使用 CP-09
   已固定的中性 nested result；
3. Catalog 中现有 capability `output_schema` 只作为对应原生 Tool 的逻辑输出合同，明确位于 native result wrapper 内，
   不把逻辑输出误写成实际 transport envelope；
4. 同一 schema 写入内部 declaration、Runtime-only capability identity，并通过 Codex 的 JSON Schema -> TypeScript renderer
   进入唯一 `taskspace_exec` description；
5. Provider wire 继续只包含标准 Function Tool 的 name、description、parameters 等输入字段，不发送 `output_schema`。

## 3. 边界与收益

- Agent 能在调用前同时看到输入合同和一次 outer feedback 的完整返回形状；
- handler、Catalog、能力指纹不再各自维护结果字段；
- Tool 的成功、失败、MCP、Tool Search、Patch 与 output-reference 语义仍由 CP-09 的公共 nested result 忠实透传；
- Runtime 不解释结果、不改变节点状态、不根据结果补写 Agent 动作；
- Standard、Code Mode 和普通 Tool schema 均未修改。

## 4. 离线验证

- `cargo test -p codex-core taskspace_exec --lib`：67 PASS；
- typed feedback 序列化结果反向通过同一 outer schema；
- output schema JSON 往返稳定，capability output 变化同时改变结果合同和 capability identity；
- 输入参数仍不包含 `revision/capability_id/outer_call_id` 等 Runtime-owned 字段；
- Provider declaration 序列化结果不包含非标准 `output_schema` 字段；
- `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`：免费 final-wire PASS，候选指纹
  `5b48dc35524e237d1ec134504f6ca4d1202b42eab691926f84ec8bfb33a922cc`；发布继续阻断，等待真实回归。

## 5. 后续

CP-10 只补内层调用的机械观测归属，不让该身份参与授权、执行或状态语义。CP-11 收敛 Hosted 分类与逐项核对，CP-12/13
再统一执行协议唯一性、final-wire、缓存和离线总验收；在此之前不启动真实 Provider 运行。
