# R5-J6.7.2 Canonical Store 原子切换结果

- 日期：2026-07-12
- 状态：完成
- 下一阶段：J6.7.3 Map/control 语义去重

## 1. 最终所有权边界

TaskSpace active 时，所有 provider-visible 动态 `ResponseItem` 只有一个 owner：

1. `TaskSpaceEventStore` 保存原始 payload、role、sequence、call pair、provider item id 和 tool success。
2. `ConversationHistory` 不再平行保存这些正文。
3. provider 输入由 Store 按 sequence 机械还原原生 `ResponseItem`，不摘要、不改写、不推断。
4. immutable base instructions、tool schema 和进程环境仍由原有请求构造层提供，不进入动态事件 Store。
5. TaskSpace 退出时一次性线性化回 Standard history；subagent fork 只获得线性化上下文，不复制父 runtime。

这比原计划“global 动态 item 留在旧 history”更简单：若把 developer/system 动态 item 单独留在 base，
必须额外维护跨 Store 的全局顺序锚点，会重新形成双轨。当前方案按可见 item 和不可变请求基座分界，
不存在第二套 task body。

## 2. 生命周期缺陷关闭

| 缺陷 | 修复 |
|---|---|
| B01 退出后仍暴露 TaskSpace tools | provider 只读取 typed runtime mode，不再读 projection marker/budget 旁路 |
| B02 resume 恢复旧 Experiment mode | 每次 mode change 后追加最新 snapshot |
| B03 rollback 恢复已撤销 Map | snapshot 进入 turn segment，与 history 一起按 surviving turn 选择；无 snapshot 时清空 runtime |
| B04 fork 保留旧 owner/lease | root fork 重绑 task/map/main lease owner，释放不可继承的 child lease，并追加修正 snapshot |
| B05 maintenance barrier 丢失 | snapshot restore 重建 barrier |
| B08 subagent 出现第二份 runtime | subagent fork 只线性化 canonical context，runtime 保持 Standard |

错误 sequence、损坏 payload、unsupported item 不 silent fallback；直接进入显式失败路径。

## 3. 工程验证

Rust：

- protocol map runtime：3 passed。
- event store/codec：6 passed。
- focused runtime：13 passed。
- SessionState：7 passed。
- rollout reconstruction：22 passed。
- locked `whale` build：passed。

PowerShell：

- metrics extractor harness：passed。
- cost instrumentation：passed。
- performance observation：passed。

benchmark extractor 已支持 `task_context_event_recorded.rawPayload`。首次 Docker run 因 extractor 仍只识别
顶层 `response_item`，把真实 final answer 误报为 `agent_incomplete`；修复后重新运行，正式结果不再包含该
观测假失败。

## 4. Docker 横向结果

正式 run root：`target/r5-j6-7-2-live2`。

| Sample | Mode | Result | Agent | Requests | Input | Cached | Wall | Map |
|---|---|---|---|---:|---:|---:|---:|---|
| count-call-stack | Standard | solved | complete | 8 | 61,705 | 58,752 | 13.44s | none |
| count-call-stack | R5 | solved | complete | 11 | 92,012 | 88,320 | 25.44s | 4 nodes, open=0 |
| multi-file-order-pipeline | Standard | solved | complete | 13 | 137,146 | 129,536 | 52.54s | none |
| multi-file-order-pipeline | R5 | solved | complete | 13 | 161,078 | 155,264 | 75.38s | 3 nodes, open=0 |

聚合机械观察：R5/Standard requests `1.14x`、tools `1.00x`、input `1.27x`、uncached input
`0.90x`、wall `1.53x`；R5 request 2+ cache hit 高 `1.71` 个百分点。单次模型路径有方差，本阶段
不把这些差异归因为 canonical cutover。

## 5. Single-owner 门禁

两组 R5 rollout：

- canonical exact payload duplicates：0。
- duplicate call records：0。
- duplicate output records：0。
- orphan call/output：0/0。
- protected miss：0。
- semantic retention/salience：100%/100%。
- `legacy_taskspace_history_present`：false。

J6.7.2 退出条件满足，允许进入 J6.7.3。
