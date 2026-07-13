# R5-K1 压缩合同与 S1 选择结果

- 日期：2026-07-13
- 状态：COMPLETE
- 基线：`R5-K-B0` / `37bddb2bad9f8f92d52b082eb55c0c1a4171654a`
- 下一阶段：K2.0 -> K2.F -> 仅实施 S1

## 1. 结论

K1 冻结了不可变 B0、四臂实验合同和 atomic strategy ledger。首个且唯一获准进入本轮实施的策略为：

`S1 = completed_inactive_leaf_batch_archive_projection`

S1 只在 provider projection 中，将至少 3 个满足全部机械条件的历史节点替换为一个可逆 archive index：节点必须
已完成、非 current、非 active frontier、无 lease，并且没有任何入边或出边。canonical Map、event store、
checkpoint/delta 和 replay 不改变。

选择该策略不是因为 Runtime 判断这些节点“不重要”，而是因为它们的拓扑和状态满足无歧义硬条件。S1 不生成
自然语言摘要，不合并 Agent 语义，不改变 Agent 的状态推进。

## 2. B0 冻结

| 字段 | 值 |
|---|---|
| baseline source | `37bddb2bad9f8f92d52b082eb55c0c1a4171654a` |
| latest Codex source in binary | `c774467436460cfab371e9eae5df4d80a662a02f` |
| binary SHA-256 | `90dd355bf118edf8ee72590fac29aa38f1feee681984c2509f5f267d27dcb2c5` |
| local Docker image ID | `sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca` |
| base image | `ubuntu@sha256:4fbb8e6a8395de5a7550b33509421a2bafbc0aab6c06ba2cef9ebffbc7092d90` |
| frozen artifact | `target/r5-k1-baseline/R5-K-B0/whale` |

冻结后的 binary 和 attestation 已重新计算 hash，与原 build artifact 一致。tracked 合同见
`benchmarks/taskspace/map-compression/experiment-contract.json`。

## 3. 四臂与样本

| Arm | 定义 |
|---|---|
| STD1 | candidate build 的 Standard |
| B0 | immutable B0 的 TaskSpace |
| P0 | B0 的同一身份别名，首轮不重复消耗 provider 请求 |
| C1 | 仅增加 S1 的 candidate TaskSpace |

简单样本固定为 `single-file-fast-fix`，复杂样本固定为 `subscription-billing-repair`，均运行 3 次。复杂样本使用
同一会话两轮 live continuation：第一轮要求分别完成 README、实现和测试三项只读调查并结束，第二轮通过
`exec resume --last` 实施原修复任务。Standard、B0 与 C 使用相同 prelude、主 prompt、fixture、model、Docker
contract、validator 和 oracle；TaskSpace 在 resume 时自然生成 projection，S1 不新增自动触发逻辑。

复杂样本的预登记 primary benefit 是：发生激活的 projection bytes 相对 P0/B0 中位数至少下降 10%。运行后不得
更换指标。简单样本必须 activation=0，且 requests/input/wall 中位数比不高于 1.10，Req2+ cache 下降不超过
2 个百分点。

### 3.1 触发前提校正

K3 首轮实测否定了原合同中“单轮复杂样本稳定建立 5 节点”的假设：Agent 会建立 3–6 个节点，且节点完成时机
随执行路径变化。使用 10K/15K token 阈值强制 compaction 时，projection 可能在首节点运行中出现，既无法触发
S1，也会改变 B0/C 的语义连续性和请求路径。该数据不得作为 S1 收益证据。

因此在正式三次矩阵前修正实验触发协议，不修正 S1：复杂样本增加固定只读 prelude
`subscription-billing-repair-prelude.txt`，其 SHA-256 为
`ce7cb5fc4f9ff2bdbbff703d31a8a0bbd7c9e641a265c2f9c7127cc8a43d3d36`；三臂在同一 Docker workspace 和
`WHALE_HOME` 中 resume。provider 配置恢复默认 compaction 阈值。该修正只提供确定的 projection epoch，不改变
canonical Map、S1 eligibility 或生产触发语义。

## 4. 公共不变量

1. archive payload 必须由 canonical node/edge/event/result 机械编码，hash 稳定；
2. expand 后 canonical payload hash、node set 和 topology 100% 一致；
3. archive ref 缺失、损坏或 hash mismatch 显式失败，不返回 partial view；
4. projection 只是派生视图，不把 archive 写成第二事实源；
5. Runtime 不生成结论、摘要、相关性排序或状态变更；
6. S1 不新增 tool action/schema，读取沿用既有 `read_output_ref` 动作；
7. 本轮完成并汇报 S1 后暂停，不进入 S2。

## 5. 分阶段边界

- K2.0：只建设 archive codec/ref、hash/round-trip fixture、日志和实验 runner；provider projection 必须与 B0
  字节等价，strategy activation=0。
- K2.F：只把损坏 rollout 的 panic 改为 structured session fatal，zero partial restore；不加入压缩行为。
- K3-S1：才允许 projection 输出 S1 archive index，并执行四臂 live 与 deterministic scale/replay。

## 6. 风险关闭情况

K1 已关闭 S1 行为边界、owner、样本、重复次数、primary metric、fallback 和 forbidden co-change。仍需由 K2.0
实际证明 codec 可逆、runner 身份可信；仍需由 K2.F 证明损坏链路不会 panic 或 partial restore。这些是进入 S1
production slice 的硬门禁，不由 live sample 的随机成功替代。
