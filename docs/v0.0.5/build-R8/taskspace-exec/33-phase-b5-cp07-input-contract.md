# Phase B5 CP-07：模型可见输入合同

- Date: 2026-08-10
- Scope: `taskspace_exec` description、Map operation capabilities、canonical examples
- Status: verified offline / provider cache revalidation pending

## 1. 收敛内容

VA-02R 已把 outer 使用协议集中到 `taskspace_exec` declaration。CP-07 在该唯一位置补齐三个确定性缺口：

1. `calls[]` 的数组顺序只定义 Map 操作边界，不建立第二份 Work 依赖图；普通工作的依赖只来自 Map node
   `parents[]`。无结果依赖的 client calls 可沿用原生并行策略，有结果依赖的工作留到下一次请求；
2. 五个 Map operation 的 capability description 明确其硬边界：初始化/reopen 必须带真实工作、`read_map` 独占批次、
   完成 Work node 必须继续工作或 finish、`finish_map` 位于末尾；
3. 初始化、只读、完成最后 Work node 并显式结束三个示例由真实 `MapOperation` 类型序列化生成，不再手写 Map operation
   JSON。示例分别送回正式 Catalog decoder 和完整 preflight 验证。

普通 Tool 的名称、namespace、参数 schema 和描述仍来自同一有效 Catalog。CP-07 没有修改 base instructions、Standard
Tool、Runtime 决策、Map 状态机或 dispatch 顺序。

## 2. 产品边界

- Agent 决定本次 `calls[]` 中有哪些动作、数组位置、参数与 `node_id`；
- Map `parents[]` 决定 Work DAG，`calls[]` 不重复声明 B 到 C 的依赖；
- Runtime 只按已有 preflight 检查 Map 边界、节点可执行性、参数 schema 与单 Patch 底线；
- Tool outcome 不改变 Node 状态，节点完成和最终 Map 关闭都由 Agent 显式声明；
- 示例用于展示合同形状，不是固定工作计划，也不会让 Runtime 自动补动作。

## 3. 验证

- `cargo test -p codex-core taskspace_exec --lib`：67 PASS；
- 初始化示例通过 decoder/preflight 并生成一个 Work call；
- `read_map` 示例作为独占批次返回完整 Map view；
- completion + `finish_map` 示例完成最后 Work、Root 与唯一 Finish；
- declaration schema 与 description 均明确普通 work 依赖来自 Map parents；
- `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`：免费 final-wire PASS，候选指纹
  `b037930eb5920935305fdcfb9950f39b83c78121852b41b0b9bfa7e486db611a`；发布继续阻断。

## 4. 后续

CP-08 基于 CP-09 的固定 nested result 建立同源 outer output contract。CP-12/CP-13 再统一检查协议唯一性、缓存和
final-wire；真实 Provider 遵循与成本不由本单元离线推断。
