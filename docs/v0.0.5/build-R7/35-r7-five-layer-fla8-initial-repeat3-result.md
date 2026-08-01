# R7 五层架构 FLA-8 首轮四臂结果

- 日期：2026-07-23
- 状态：首轮观测完成，扩展评估待用户决策
- 范围：仅五层改造后 Standard、map-always、map-append、map-request
- 样本：`single-file-fast-fix`、`subscription-billing-repair`
- 重复：每个样本、每臂 3 次，共 24 次
- Subject commit：`f2baea6d13caef02f15e1a3c6938a3fa05a3d315`
- Observer commit：`9bc3bf7ca`
- Docker image：`sha256:55a8ac465c574efb57d8bd53f286812a77f41fd428de1c3b0b18b7c5165ee0ca`
- 机器摘要：[five-layer-fla8-initial-result.json](../../../benchmarks/taskspace/r7/five-layer-fla8-initial-result.json)

## 1. 结论边界

24 次运行全部完整且通过业务、公开和隐藏验证，没有删除异常值。首轮 repeat 3 只证明没有直接正确性回归，并暴露
执行路径和成本结构；它不选择默认 projection policy，也不声明收益或非劣。

`repeat 10` 和 held-out 样本没有启动，等待用户审阅本结果后决定。

## 2. 整体结果

| Arm | 成功 | Request 总 / 均 / 中位 | Input 总 / 均 / 中位 | Uncached input | Req2+ cache | Output 总 / 均 | Wall ms 总 / 均 / 中位 |
|---|---:|---:|---:|---:|---:|---:|---:|
| Standard | 6/6 | 66 / 11.000 / 11.0 | 1,058,248 / 176,374.667 / 176,651 | 31,560 | 96.87% | 25,545 / 4,257.500 | 230,790 / 38,465.000 / 35,801 |
| map-always | 6/6 | 60 / 10.000 / 11.0 | 1,588,363 / 264,727.167 / 290,456 | 970,123 | 34.73% | 24,893 / 4,148.833 | 233,500 / 38,916.667 / 40,868 |
| map-append | 6/6 | 65 / 10.833 / 9.0 | 2,085,648 / 347,608.000 / 263,461.5 | 92,816 | 95.58% | 27,348 / 4,558.000 | 246,595 / 41,099.167 / 34,072 |
| map-request | 6/6 | 78 / 13.000 / 10.5 | 2,236,203 / 372,700.500 / 280,216.5 | 104,491 | 95.06% | 33,997 / 5,666.167 | 310,739 / 51,789.833 / 41,438.5 |

相对 Standard：

| Arm | Request | Input | Uncached input | Output | Wall time |
|---|---:|---:|---:|---:|---:|
| map-always | -9.1% | +50.1% | 30.7x | -2.6% | +1.2% |
| map-append | -1.5% | +97.1% | 2.9x | +7.1% | +6.8% |
| map-request | +18.2% | +111.3% | 3.3x | +33.1% | +34.6% |

## 3. Request 路径

Standard 的 66 个 request 中，60 个携带工具动作，6 个是每次运行末尾的纯最终回答。三个 TaskSpace 臂都把
`finish_map + final_summary` 放在同一 request，没有额外终答 request。因此按实际工作 request 比较：

| Arm | 工作 request | 纯终答 request | 多工具 request |
|---|---:|---:|---:|
| Standard | 60 | 6 | 13 |
| map-always | 60 | 0 | 15 |
| map-append | 65 | 0 | 8 |
| map-request | 78 | 0 | 18 |

这说明 `map-always` 的总 request 优势完全来自终态合并：扣除 Standard 的 6 次纯终答后，两者均为 60 次工作
request。`map-append` 多 5 次工作 request，`map-request` 多 18 次。

### 3.1 map-request

复杂样本 3/3 都出现一次“同一 provider response 声明多个 `apply_patch`”并被 one-patch preflight 整体拒绝，
不是偶发单次。另有 2 个 TaskSpace 协议失败 request 和 5 个状态机拒绝 request。

最差的 complex repeat 3 使用 25 次 request。它在前 18 次中完成实际修改和测试，但始终把动作记在
`explore_repo` 节点；第 19 次提前 `finish_map` 被正确拒绝，随后读取 Map，并用 4 次仅执行 `echo` 的
`complete_then_continue` 事后闭合 `read_rules -> run_tests -> fix_issues -> verify`，第 25 次才完成最终闭合。
这是明确的工作过程与 Map 生命周期脱节，不是有效的工程步骤。

complex repeat 1/2 分别为 16/11 次 request，能在实际工作中完成节点交接，但同样各有一次 multi-patch 响应。
因此 map-request 同时存在稳定的 multi-patch 选择问题和一次严重的 Map 事后补账路径。

### 3.2 map-append

complex repeat 2 是 23-request 离群运行，其他两个 complex repeat 均为 11。该运行包含首次 Map 图错误、逐文件
串行读取、一次错误的跨节点交接、一次 `read_map`、两次 patch prepare 失败，以及 3 次只用于交接的 `echo`
动作。均值被该运行显著拉高，中位数 9 比均值 10.833 更能代表本轮多数路径。

### 3.3 map-always

工作 request 与 Standard 相同，并行读取最积极。它仍有 7 个状态机拒绝 request 和 1 次 `echo` 交接，但没有
multi-patch。复杂样本总 request 为 38，低于 Standard 的 45；首轮样本不足以证明这一差异稳定。

## 4. Input 成本根因

三种 TaskSpace 策略的首请求 input 均约 21.66K，Standard 为 10.87K，约为 1.99 倍。此时还没有长历史，说明
固定合同本身已引入约 10.8K token 的首请求差额。

| Section | Standard / request | 三个 TaskSpace 臂 / request | 固定差额 |
|---|---:|---:|---:|
| System messages | 6,507 | 6,813 | +306 |
| Tools schema | 5,418 | 15,186 | +9,768 |

Tools schema 占首请求差额约 90.5%，是当前 TaskSpace 总 input 偏高的第一直接原因。五层宏观说明和 L2 协议只占
较小固定差额；主要成本来自把完整 `taskspace_action` 合同复制进多个普通 Tool schema，再额外暴露
`taskspace_control`。

projection 自身形成第二层差异：

- `map-always` 每 request 当前 projection 约 577 estimated tokens，但动态替换使 req2+ cache 只有 34.73%；这是已知设计特征。
- `map-append` 每 request 累积 projection 平均约 4,252 estimated tokens，缓存仍为 95.58%，但总 input 最高增长很快。
- `map-request` 自动 projection 为 0，缓存为 95.06%；其额外 input 主要来自固定 Tool schema 和更多 request，而不是 Map 正文。

## 5. Map 与硬合同

18 个 TaskSpace 运行全部只有一张 Map，Root 和 Finish 最终均为 `closed`，开放叶为 0，没有孤立节点。平均规模：

| Arm | Nodes | Edges |
|---|---:|---:|
| map-always | 5.000 | 4.000 |
| map-append | 5.000 | 4.167 |
| map-request | 5.333 | 4.667 |

简单样本主要是 4 至 5 节点线性链。复杂样本大多仍是线性链；map-request complex repeat 1 唯一自然拆成三个
并行修复分支，结构合法。没有观察到孤立节点或 Map 坍缩，但出现了“工作已完成、节点尚未推进”的过程脱节。

硬合同和语义观测：

- `initialize_map`、`bind_node`、`complete_then_continue` 单独 control 调用：0。
- 首 request 尝试 `initialize_map`：16/18；首 request 直接成功提交：12/18。其余均被硬约束拒绝后恢复。
- exact provider payload scan：609/609 通过。
- retention coverage：100%；semantic replacement：0；未观察到反馈丢失或 projection 身份错误。

## 6. 首轮判断

1. 五层改造没有造成结果正确性或 Map 最终闭合回归。
2. `finish_map + final_summary` 合并稳定生效，消除了 Standard 每次运行的纯终答 request。
3. 当前最大的共同成本不是 projection，而是普通 Tool 中重复暴露的完整 `taskspace_action` schema。
4. map-always 的低缓存、map-append 的 projection 累积符合三策略已知设计特征。
5. map-request 的复杂样本 3/3 multi-patch 和 1/3 严重事后补 Map 值得先讨论；直接扩大 repeat 只能量化频率，不能消除已知机制问题。

本阶段按合同暂停。是否先修复固定 Tool schema 成本和 map-request 使用问题，或保持当前版本直接扩大到 repeat 10，
由用户下一步决定。
