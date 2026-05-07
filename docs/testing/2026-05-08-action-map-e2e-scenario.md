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

mock 模型脚本驱动的 agent 行动：

1. 主 agent 开启 Action Map experiment 模式后接收用户任务。
2. 主 agent 调用 `spawn_agent`。
3. runtime 自动创建 map，认领 `define_scope` node，并创建 lease。
4. 子 agent 请求中必须包含 `Action Map node assignment` 和 `Node: define_scope`。
5. 子 agent 调用 `shell_command` 阅读 `src/cache.py` 和 `tests/test_cache.py`。
6. 子 agent 返回边界调查结果，completion watcher 写入 node result。
7. 主 agent 调用 `wait_agent` 等待子 agent 完成。
8. 主 agent 调用 `apply_patch` 修改代码并补回归测试。
9. 主 agent 调用 `shell_command` 执行 Python 验证。
10. 测试读取 rollout，验证 map runtime 事件完整出现。

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
- rollout 中包含 `mode_changed`、`map_created`、`node_status_changed`、`lease_created`、`lease_attached`、`node_result_recorded`、`lease_released`。
- `lease_created` 和 `lease_released` 数量一致。
