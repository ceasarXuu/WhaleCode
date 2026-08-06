# Phase B2 Exec 合同实施结果

- Date: 2026-08-07
- Status: EX-01～EX-04 verified offline
- Scope: 合同、解析、请求级关联和零副作用预检
- Real Whale Agent run: 0

## 1. 结果

Phase B2 已从 Standard 原生 ToolSpec 和 Phase B1 canonical Map 建立新的 `taskspace_exec` 离线合同。它当前不注册生产
Tool、不调用 Router、不写 Map Store，也不改变 Standard payload；生产执行由 Phase B3 从 EX-05 开始接入。

| 单元 | 实施结果 | Commit |
|---|---|---|
| EX-01 | 定义 `initialize_map`、`update_map`、`read_map`、`reopen_map`、`finish_map` 五项操作，直接复用 canonical transaction | `0bd813e7a` |
| EX-02 | 从 Function/Freeform/Namespace/ToolSearch 原生 ToolSpec 确定性生成唯一结构化 Exec schema；Map 操作与 client call 是平级 variant | `e6887ab8f`、`671a213c8` |
| EX-03 | 请求级 envelope 固化 Map revision、catalog snapshot、outer call identity 和内部调用 identity；这些字段不要求 Agent 回显 | `a513acfd2` |
| EX-04 | 在任何 client/map 副作用前完成结构、Map、DAG、节点状态、原生参数、单 Patch 和 Hosted 逐项归属检查 | `2440a1446` |

## 2. 已证明边界

1. Agent 只声明本次调用实例：Map 操作、client Tool、原生参数、顺序和 node 归属。
2. Runtime 只验证硬合同并生成候选 Map、待执行 client calls 和已核对 Hosted bindings，不补全、重排或解释动作。
3. client Tool 的参数合同来自原生 ToolSpec，`node_id` 只属于 Exec 外层元数据，原 Tool 与 handler 无感。
4. Hosted 事实按 Provider 原始 output index 和身份机械核对；同一事实可绑定多个 Agent 声明节点，不默认 Root，也没有未绑定池。
5. Tool outcome 与节点生命周期正交；预检不会根据结果自动完成、阻塞或 reopen 节点。
6. 非法计划不会执行普通 Tool、不会提交 Map；已经由 Provider 完成的 Hosted 事实只保留原始事实，不伪装成可回滚动作。

## 3. 验收证据

| 检查 | 结果 |
|---|---|
| `cargo test -p codex-core taskspace_exec --lib --quiet` | PASS，33 tests |
| `cargo test -p codex-core action_map --lib --quiet` | PASS，15 tests |
| `cargo test -p codex-tools tool_spec_capability --lib --quiet` | PASS，5 tests |
| `cargo test -p codex-tools code_mode --lib --quiet` | PASS，15 tests |
| `cargo check -p codex-core -p codex-state -p codex-cli --tests` | PASS |
| `python3 scripts/taskspace-exec/check_zero_base.py` | PASS |
| `python3 scripts/cache-regression/check_cache_regression_gate.py --source index` | PASS，Standard final wire unchanged |
| 单代码文件行数 | PASS，生产与测试文件均不超过 500 行 |

## 4. 明确未完成

1. `taskspace_exec` 尚未注册到生产 Tool registry，也未替换 TaskSpace 顶层 Tool 暴露。
2. client calls 尚未还原为原生 ToolCall 并进入 Router；这是 EX-05。
3. Hosted facts 尚未接入真实 response envelope，也未写入 Node actions；这是 EX-06。
4. outer FunctionCallOutput、唯一反馈和生产入口尚未完成；这是 EX-07～EX-08。
5. `tool_search` 已直接复用其原生参数 schema 进入 Exec；Codex code-mode 仍按自身传输能力过滤它。`LocalShell` 没有可
   复用的参数 schema，B2 对它显式报错而不是静默丢失；EX-05 必须按原生 dispatch 类型矩阵解决，禁止手写第二份 Tool schema。
6. 本阶段没有真实 Provider/Agent 运行，因此不对模型生成稳定性、请求成本或缓存命中率作产品结论。

## 5. 下一步

Phase B3 从 EX-05 开始：只建设通过预检的 client call 到 Standard ToolRouter 的原生 dispatch 接缝。接入时必须保持普通
Tool schema、权限、sandbox、hook、handler 和结果语义不变；不得为了暂时运行而恢复旧 control/sibling 路径。
