# TaskSpace 投影模式命令与全局记忆

- Date: 2026-08-20
- Commit: `5ce2cfaf8`
- Default: `map-request`
- Status: 离线实现完成；真实 runner 已确认 `map-request` 参数进入 TaskSpace 单臂

## 产品合同

1. TaskSpace 默认使用 `map-request`。
2. `/map-request`、`/map-append`、`/map-always` 只在当前会话已经开启 TaskSpace 后出现在 slash command 列表中。
3. 命令同时切换当前会话，并把选择写入用户级根 `config.toml`；后续新建、恢复和 fork 的 TaskSpace 会话默认使用最后一次选择。
4. 用户级根配置优先于 profile 内的旧值，避免切换只对某个 profile 生效而不构成全局记忆。
5. 三种模式只改变 Map projection 进入 Provider context 的方式；Runtime、Map、Tool、状态机与反馈保持共用。
6. `/taskspace` 机械开启 TaskSpace；不存在配置值时由 Core 使用 `map-request`，不要求 Agent 决定默认模式。

## 工程结果

- 新增独立的 `SetTaskSpaceProjectionPolicy` operation；没有把 projection policy 塞回原有 `SetMapRuntimeMode`，避免“切模式”和
  “选投影”互相耦合。
- TUI 启动、resume 和 fork 时通过本地 Map 读取判断 TaskSpace 是否活跃，不产生 Provider 请求。
- App Server API 复用同一 Core operation；旧的纯 runtime-mode 调用保持原语义。
- 缺少历史 policy 的真实 TaskSpace Map 按 `map-request` 恢复；普通 Standard history 不因全局默认值被误判为必须存在 Map。

## 验证

- `cargo check -p codex-tui -p codex-core -p codex-app-server -p codex-app-server-protocol -p codex-exec`：PASS。
- Slash command 可见性与 dispatch 聚焦测试：PASS。
- 用户级根配置持久化与 profile 覆盖测试：PASS。
- `cargo fmt --all`、`git diff --check`：PASS。
- 缓存敏感面免费 final-wire gate：PASS；候选指纹
  `bff130c627f82a7035da65311111916b7f0237106a89aedcca54c65266d71b5f`。
- 真实执行 `PlanOnly` 展开 6 个物理候选，仅选中 3 个 TaskSpace、0 个 Standard；随后真实运行确实只启动 TaskSpace，
  第 1 轮因独立 I03 失败按停止条件结束，未启动剩余两轮。

真实运行不能证明三个 slash command 的交互体验，但已证明默认 `map-request` 和 runner 配置进入生产 TaskSpace 路径；命令可见性、
当前会话切换和全局持久化由确定性测试覆盖。
