# R5-J6.7.4 Projection 与 Compaction 收敛结果

- 日期：2026-07-12
- 状态：完成
- 下一阶段：J6.7.5 旧双轨代码物理删除

## 1. Projection 边界

provider-visible projection 现在只接收 typed Map 结构：task/map 状态、node ID/kind/status/goal、edge、
event ID、action class、tool success、raw ref 和 artifact refs。

- 删除 `NodeEvent.body/visible_excerpt/command` 的 projection 读取与正文裁剪。
- current node 只输出 ID；goal 只在 node inventory 出现一次。
- dependency 只输出 `from->to` edge，不再复述 dependency title/goal。
- mechanical blank 仍只暴露 `active_task_path_without_nodes` 硬状态。
- provider wire 中 `excerpt:`、`current_node_dependencies` 均为 0。

## 2. Canonical Compaction Checkpoint

TaskSpace compaction 不再清空并重建 Event Store，而是追加一个
`TaskSpaceCompactionCheckpointV1` canonical event：

1. covered sequence start/end/count；
2. covered raw events SHA-256；
3. covered events 中机械扫描得到的 `output-ref://sha256/...`；
4. 固定 omission reason `context_compaction`；
5. 当前 compaction replacement view。

raw events 保留原 ID、sequence、role、call pairing 和 payload。provider linearizer 只展开最新 checkpoint
view及其后缀，covered raw prefix 不再重复出现。restore 时 range/count/hash 不一致会显式失败。

TaskSpace 只持久化 checkpoint event，不再并行持久化 generic `CompactedItem`；Standard 保持原 compaction
路径。rollout resume 按 canonical event 顺序恢复，不使用字符串探测或旧数据兼容分支。

## 3. Output Ref 可移植性

`output-ref://sha256/<sha>` 的对象从 rollout sidecar 移到 session 级共享 CAS：

```text
<codex-home>/session-store/output-refs/sha256/<sha>.stdout
```

fork、resume、archive 日期变化只要共享同一 Codex home，就机械解析到同一对象。读取后重新计算 SHA-256，
corruption 返回 `InvalidData`，不回退旧 sidecar。若 ref 创建机制异常且 provider history 仍出现大 raw output，
composer 现在保留原 call/output pair，不再静默删反馈。

## 4. 工程验证

Rust：

- Event Store/checkpoint：10 passed。
- checkpoint ref scanner：1 passed。
- focused Runtime/projection：13 passed。
- SessionState：8 passed。
- rollout reconstruction：23 passed。
- compaction suite：37 passed。
- active provider composition：24 passed。
- output reference：7 passed。
- rollout persistence ref：1 passed。
- locked `whale` build：passed。

关键 deterministic 证据：checkpoint 前 raw payload 保留、provider view 只出现一次 replacement、stale
projection 被移除、hash corruption 被拒绝、resume 恢复 checkpoint、跨两个日期/rollout 路径可读取同一
output ref、损坏对象被拒绝。

## 5. Docker 横向结果

正式 run root：`target/r5-j6-7-4-live`。两个 run 均为单次诊断样本，复杂样本脚本的非零退出来自
E2 次数/aggregate 门禁，不是 Agent 或 validator 失败。

| Sample | Mode | Result | Agent | Requests | Runtime tools | Input | Cached | Wall | Map |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| large-output-ref-smoke | Standard | solved | complete | 10 | 11 | 78,571 | 75,264 | 16.52s | none |
| large-output-ref-smoke | R5 | solved | complete | 9 | 7 | 73,522 | 60,416 | 18.36s | 4 nodes, open=0 |
| multi-file-order-pipeline | Standard | solved | complete | 14 | 22 | 188,977 | 177,920 | 87.22s | none |
| multi-file-order-pipeline | R5 | solved | complete | 23 | 13 | 386,194 | 373,248 | 115.83s | 7 nodes, open=0 |

两组 R5 均为 payload/call/output record duplicate=0、orphan=0、protected miss=0、retention/salience=100%。
复杂样本的一次 output body duplicate 是 Agent 重复工具行为，不是同一 canonical record 或 projection
正文双载体。

## 6. Cache Gate

- complex R5 request 2+ cache hit：96.63%，strict prefix 21/22。
- large-output R5 的 request 2+ 聚合为86.95%，原因是 request 2发生 bootstrap tool schema切换；从
  request 3起的7个稳定-shape请求为60,416/61,641，即98.01%。

因此 active stable shape 未比约97%基线下降超过2个百分点。单次 complex 请求数和总input有明显方差，
本阶段不把它归因为 projection/checkpoint 改动。

## 7. 后续清理项

- `NodeEvent.body/visible_excerpt` 已无 projection production reader，但仍存在于 Map snapshot，交由
  J6.7.5 物理删除并改为 canonical event ref。
- 旧 projection marker/composer helper 已产生 dead-code warning，交由 J6.7.5 删除。
- `root_task_active_after_nodes_closed` 与 unreviewed result 旧状态字段一并在 J6.7.5 call graph 审计处理。

J6.7.4 的正文去重、checkpoint recovery、ref portability、protected feedback 和 active cache gate 均通过，
允许进入 J6.7.5。
