# Action Map 端到端用户场景测试

Date: 2026-05-08

## 目标

`run-action-map-e2e-scenario.ps1` 用于验证“用户提出真实工程问题 -> 主 agent 行动 -> subagent 调查 -> Action Map 绑定 node/lease -> 主 agent 修改代码 -> 主 agent 验证结果”这条完整路径。

这和 `run-action-map-regression.ps1` 不同：

- regression 脚本验证 runtime/handler 的底层状态机。
- e2e 脚本验证一次接近真实用户工作流的完整 agent 行动链路。

## 当前场景

场景名：`action-map-realistic-user-bugfix`

模拟代码库：

- `src/cache.py` 中 `cache_key(namespace, key)` 只归一化 `key`，没有归一化 `namespace`。
- `tests/test_cache.py` 初始只有 key 归一化测试。

模拟用户请求：

```text
这个沙盒项目有一个缓存 key 相关的回归失败。请先让子 agent 调查边界，再修复代码并验证。
```

脚本化 provider 驱动的 agent 行动：

1. 主 agent 开启 TaskSpace experiment 模式；初始 snapshot 必须保持 `tasks=[]`、`maps=[]`，并显示 `routing_required=true`、`bootstrap_required=true`。
2. 主 agent 接收用户任务后，先调用 `taskspace_control(action=start_task)`，由 agent 根据用户上下文创建语义 task、map 和第一个 node。
3. `start_task` 清除 routing gate 后，主 agent 才能继续调用普通工具或 `spawn_agent`。
4. 主 agent 调用 `spawn_agent`，runtime 只能把子 agent 绑定到已有 ready node，不能自动创建默认 map/node。
5. 子 agent 请求中必须包含 `TaskSpace node assignment` 和具体 `Task/Map/Node` 标识。
6. 子 agent 调用 `shell_command` 阅读 `src/cache.py` 和 `tests/test_cache.py`。
7. 子 agent 返回边界调查结果，completion watcher 写入 node result。
8. 主 agent 调用 `wait_agent` 等待子 agent 完成。
9. 主 agent 调用 `apply_patch` 修改代码并补回归测试。
10. 主 agent 调用 `shell_command` 执行 Python 验证。
11. 测试读取 rollout，验证 task/map/node/lease/result 事件完整出现。

## 验证入口

```powershell
.\scripts\run-action-map-e2e-scenario.ps1
```

等价 cargo 命令：

```powershell
cd third_party\codex-cli\codex-rs
$env:CARGO_TARGET_DIR='D:\whalecode-alpha\target-test'
$env:CARGO_BUILD_JOBS='2'
rustup run stable cargo test -p codex-core --test all realistic_user_bugfix_runs_agent_actions_with_action_map --locked --jobs 2
```

## 输出

脚本报告：

```text
target/test-reports/action-map-e2e-<timestamp>/report.md
```

场景证据：

```text
third_party/codex-cli/codex-rs/target/scenario-runs/action-map-realistic-user-bugfix/<run-id>/artifacts/
  report.md
  map-timeline.json
  provider-requests.json
  transcript.jsonl
  test-output.txt
```

关键通过条件：

- cargo exit code 为 0。
- 测试通过数为 1，失败数为 0。
- `apply_patch` 输出 `metadata.exit_code = 0`。
- 验证命令输出包含 `cache validation passed`。
- 开启 TaskSpace 后的初始 snapshot 中 `tasks=[]`、`maps=[]`，且 `routing_required=true`、`bootstrap_required=true`。
- rollout 中包含 `mode_changed`、`snapshot_updated`、`task_created`、`map_created`、`node_status_changed`、`lease_created`、`lease_attached`、`node_result_recorded`、`lease_released`。
- `task_created` 必须早于对应的 `map_created`，证明 map 是由 agent 的 `start_task` 路径创建，而不是由 runtime 默认模板提前创建。
- `lease_created` 和 `lease_released` 数量一致。

## 2026-05-27 TaskSpace 路径修正

当前 E2E 不再接受“开启 TaskSpace 后 runtime 自动创建默认 map/node”的路径。正确顺序是：

1. `/taskspace` 或 `SetMapRuntimeMode(Experiment)` 后，snapshot 必须保持 `tasks=[]`、`maps=[]`，并显示 `routing_required=true`、`bootstrap_required=true`。
2. 真实用户请求进入后，agent 必须先调用 `taskspace_control(action=start_task)`，由 agent 根据用户上下文创建语义 task、map 和第一个 node。
3. 只有 `start_task` 或 `route_task` 清除 routing gate 后，普通工具和 `spawn_agent` 才能继续。
4. `map_created` 必须出现在 `task_created` 之后，不能早于 agent 的 `start_task` 控制动作。
