# Phase B5 CP-12：单一协议与 final-wire 门禁

- Date: 2026-08-10
- Scope: TaskSpace 模型可见固定层、缓存敏感面、免费 final-wire gate
- Status: verified offline

## 1. 审计结论

当前详细操作协议只由 `taskspace_exec` declaration 的 description 承载；Standard/TaskSpace 共用的 base instructions 不含
TaskSpace wire 示例。TaskSpace 消息上下文只携带当前 Map projection/handle，不复制 outer Tool 的调用合同。

原缓存门禁存在两个覆盖缺口：

1. `protocol.rs`、`result.rs`、`deferred.rs`、`hosted.rs` 可以改变模型可见 Tool 合同或后续请求的有效能力，但不在敏感面；
2. 免费 final-wire 只冻结 Standard 请求；已接受基线 manifest 中的旧 TaskSpace 路径已被零基线删除，不能继续代表当前 Exec。

## 2. 修复

1. 将四个真实 TaskSpace declaration 构造源加入 `tool_declarations` 敏感面；
2. 保持 `handler.rs`、`dispatch.rs`、`preflight.rs`、`response_scope.rs` 等执行内部为非敏感，避免普通实现变更误报；
3. 新增通过正式 Session/Router/Provider request builder 生成的 `taskspace_production_tool_wire`；
4. 快照只冻结真实请求的 `model/tool_choice/tools`，避免 skills、路径、Map handle 等动态上下文污染 Tool 合同；
5. 同一测试确认 Map handle 确实进入请求，详细 outer 操作协议未在消息上下文重复；
6. 免费门禁同时运行 TaskSpace Exec 定向合同测试，覆盖 deferred、Namespace、output schema 和有效 surface。
7. 免费门禁子进程使用隔离的临时 HOME，但继续复用显式 CARGO/RUSTUP 缓存；避免开发机 `$HOME/.agents/skills`
   改变固定快照，不修改生产 skills 发现或 benchmark 环境。

新 TaskSpace 快照是门禁 fixture，不修改、伪造或晋升已接受的付费缓存基线。真实缓存表现仍必须使用获批 Provider run。

## 3. 验证

- `taskspace_production_tool_wire`：1 PASS，连续重放字节稳定；
- `python3 scripts/cache-regression/test_cache_surface_contract.py`：9 PASS；
- `python3 scripts/cache-regression/test_cache_regression_gate.py`：30 PASS；
- `python3 scripts/cache-regression/test_free_cache_contracts.py`：12 PASS；
- `python3 scripts/cache-regression/free_cache_contracts.py`：8 个命令全部 PASS；
- TaskSpace Exec：69 PASS；
- staged zero-base/cache gate：PASS；候选敏感面变化保持发布阻断，等待获批真实回归。

## 4. 边界

- 门禁负责发现 Provider 固定前缀和 Tool wire 变化，不判断产品收益；
- 不把 Tool 输出、Runtime trace、Map 数据或 Agent 决策写入 schema；
- 不因普通执行代码变化触发付费缓存复验；
- 不以更新 snapshot 代替真实 Provider 缓存测量。
