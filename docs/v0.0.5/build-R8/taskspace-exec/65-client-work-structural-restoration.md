# client work 结构前置条件恢复

- Date: 2026-08-16
- Scope: TaskSpace Exec 工作型序列
- Status: 生产代码、离线测试、免费缓存门禁和 13 轮真实 Agent 验证完成；accepted 缓存基线未晋升

## 1. 问题表现

C2 十轮真实运行中，4/10 的首次请求只创建 Map，没有提交任何 client Tool。Runtime 拒绝后，Agent 在下一请求重复提交同一
Map 并补上 work。业务最终可恢复，但每次都会浪费一次 Provider 请求、重复输入并污染执行节奏。

## 2. 回归窗口与根因

1. `e4e7fc874` 为兼容 Provider-first 路径，把四类工作型序列的 `tools[]` 改为可选，并以“Provider 或 client 任一存在”作为
   响应级 work 条件。
2. `682164844` 随后撤销 Provider 的 Agent 双写和待归属协议，改为 Runtime 在 Root 下机械归纳 Provider facts。
3. 第二步没有同步撤销第一步的响应级 OR gate。Provider 已退出 Exec 工作序列，`tools[]` 却仍可缺失，schema 因而继续向
   Agent 暴露 Map-only 工作序列，并在 C2 十轮中稳定出现 4 次。

这不是 Agent 对 Map 初始化说明的偶发误读，而是活动 Tool 合同主动允许了不再具备产品意义的结构。

## 3. 当前唯一合同

- `initialize_and_work`、`work`、`update_and_work`、`reopen_update_and_work` 必须声明非空 client `tools[]`。
- 缺失或空数组在 schema/decode 边界拒绝，不能进入 Map transaction 或 client dispatch。
- Provider-hosted Tool 继续按原生事实由 Runtime 在 Root 下机械归纳；不恢复双写、待归属或 Agent-visible Provider 字段。
- Provider fact 不替代 Exec 的 client work，也不参与工作型序列合法性判断。
- `update_map`、`update_and_finish`、`read_map`、`finish_map` 等纯 Map 合法序列保持原合同。

## 4. 实现范围

1. 四类工作 schema 将 `tools` 恢复为 required，并设置 `minItems: 1`。
2. typed decoder 不再把缺失 `tools` 归一为空数组。
3. preflight 和 response claim 删除 `has_provider_work` 对 Exec work 的替代判断。
4. Provider ResponseItem 采集、Root 聚合、原生命名和 escape 诊断保持不变。
5. Standard Tool schema、普通 Tool handler、Map 生命周期和节点选择规则零变化。

没有保留可选 `tools[]` 的兼容分支，也没有增加提示词补丁或 Runtime 语义判断。

## 5. 离线证据

| 验证 | 结果 |
|---|---:|
| `cargo test -p codex-core taskspace_exec --lib --locked` | 67 passed |
| `cargo test -p codex-core --test all cache_final_wire --locked` | 2 passed |
| `cargo test -p codex-state taskspace --lib --locked` | 16 passed |
| 四类工作 schema 要求 `tools` 且 `minItems=1` | passed |
| 缺失/空 `tools` 的 decode 负向测试 | passed |
| 同响应存在 Provider fact 仍不能替代 client work | passed |
| `python3 scripts/cache-regression/check_cache_regression_gate.py --source index` | PASS；surface `602b2906...adff`，发布保持阻断 |

`taskspace_production_tool_wire` 快照此前仍停留在已废弃的 Hosted 双写和旧初始化 `$ref` 结构。本次将该专用快照机械更新为
当前生产 wire，并由上述 final-wire 测试锁定；Standard final wire 同批保持通过。

后续三轮真实运行中，首请求均一次提交并执行 `initialize_and_work + client exec_command`，Map-only 空初始化为 0/3；业务、验证
和 Map 闭环均为 3/3。Request 2+ 加权缓存命中率为 93.09%。详见
[`66-client-work-restoration-repeat3-result.md`](66-client-work-restoration-repeat3-result.md)。该结果没有 Standard 臂，也没有按缓存
专用 runner 晋升 accepted baseline；目标子问题通过不等于整个 I03 关闭。

扩大十轮中首次合法初始化并执行 client work 为 10/10，顶层 client Tool 逃逸 0/10，业务、外部验证和 Map 闭环均 10/10；
结合前批累计为 13/13。Request 2+ 加权缓存命中率为 92.21%，无 Tool shape 切换或 Provider retry。结构恢复通过，但三个独立
Agent 行为异常仍归 I03/I04 verifying，详见
[`68-client-work-restoration-repeat10-result.md`](68-client-work-restoration-repeat10-result.md)。
