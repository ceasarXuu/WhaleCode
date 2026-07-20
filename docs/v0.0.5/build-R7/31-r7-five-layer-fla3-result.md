# R7 五层架构 FLA-3 结果

- 日期：2026-07-21
- 状态：`active_verified`
- 实现提交：`a42fcb123`
- Projection policy：`map-request`
- 机器结果：[`five-layer-fla3-result.json`](../../../benchmarks/taskspace/r7/five-layer-fla3-result.json)

## 1. 实施结果

1. 新增 `taskspace-advanced` v1.0.0 bundled Skill，生产正文与选定合同逐字一致，SHA256 为
   `a6f93fce96f7763ab3c36cb192091a908d744c1ad44ed26f6b4f5adf3422d5b1`。
2. 新 TaskSpace 会话把正文写入内容寻址的不可变 snapshot；session 保存名称、版本、hash 和路径，resume/fork
   恢复原 identity，缺失或损坏时返回事实错误，不回退到最新版。
3. 用户显式选择走宿主 `<skill>` 注入；Agent 自主选择走普通文件读取，Tool result 不被 TaskSpace 改写。
4. catalog、显式加载、普通读取、快照绑定和失败路径均有结构化日志；bundled Skills 不可用时不阻断 TaskSpace。
5. Standard 与 TaskSpace 的隔离条件改为实际 runtime mode。benchmark 中 projection policy 也是 TaskSpace
   treatment，不再作为 Standard 公共配置。

## 2. 合同验证

| 检查 | 结果 |
|---|---:|
| FLA-3 独立合同 | PASS |
| FLA-0 至 FLA-5 当前合同合集 | PASS |
| TaskSpace Skill 单测 | 11/11 PASS |
| 显式选择快照注入集成测试 | 1/1 PASS |
| benchmark harness / Docker runner 自测 | PASS |
| `cargo check -p codex-core` | PASS |
| Whale 开发构建 | PASS |

显式选择测试在 mock provider 的真实请求中确认 `<skill>` 携带的是会话 snapshot 全文。Docker smoke 中的
snapshot 是可读的 2,224-byte 普通文件，hash 与选定正文一致。两个自然任务均未主动读取高级 Skill，因此本阶段
只确认自主文件读取载体可用，不声称 Agent 已稳定学会在复杂任务中选择它。

## 3. Docker 冒烟

每个样本 1 个 pair；四个模式侧都通过公开与隐藏验证，`engineering_unclean=false`。这是阶段诊断 smoke，因
`repeats_lt_3` 不进入效用聚合。

| 样本 | 模式 | Request | 普通 Tool | Control/失败 | Input | Cached | Cache hit | Output | Wall ms | Map 节点/边 |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| simple | Standard | 5 | 7 | 0/0 | 60,308 | 58,880 | 97.63% | 1,593 | 14,631 | 不适用 |
| simple | TaskSpace | 16 | 13 | 7/3 | 263,860 | 248,448 | 94.16% | 4,208 | 42,184 | 5/4 |
| complex | Standard | 18 | 24 | 0/0 | 295,278 | 289,024 | 97.88% | 5,468 | 50,826 | 不适用 |
| complex | TaskSpace | 18 | 28 | 8/4 | 358,691 | 326,656 | 91.07% | 10,713 | 88,615 | 5/4 |

简单 TaskSpace 路径先产生错误 patch 和缩进错误，再自行修正；同时有 3 次 control preflight rejection，因此请求
明显放大。复杂样本两臂请求数相同，但 TaskSpace 的 input、uncached input、output 和耗时仍更高。高级 Skill 在
两组均未加载，不能把这些行动差异归因于 Skill 正文；同样也不能据一轮样本宣称没有影响。

## 4. 过程问题

首轮 smoke 因调用时遗漏 projection policy，TaskSpace 从未激活，结果已明确作废。根因是 runner 把必需配置留给
调用者拼接。修复后 policy 成为有枚举校验的一等参数，命令构造器保证 TaskSpace 必须携带它，并在 artifact 中记录
treatment delta；Standard 不再携带该配置。

随后审计发现生产 catalog 曾以“存在 policy/snapshot”代替“runtime 已激活”判断，可能让 Standard 暴露 L3。
当前绑定 API 必须显式接收 `taskspace_active`，Turn 以状态机 runtime mode 提供该事实。最终两组 Standard rollout
均记录 `policy=null`、`skill_snapshot=null`，TaskSpace 则固定为选定 hash。

## 5. 结论

FLA-3 的生产正文、不可变 identity、两类载体、失败语义、Standard 隔离、日志、合同测试和两个 Docker 样本均已
闭合，可以标记为 `active_verified`。阶段收益限于能力和边界正确性；自然选择率、请求成本和行为效用继续作为后续
复杂样本观测项，不提前归入 FLA-4，也不通过 Runtime 自动加载或语义判断来补偿 Agent 没有选择 Skill。
