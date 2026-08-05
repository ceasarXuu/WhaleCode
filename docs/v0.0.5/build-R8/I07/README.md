# R8-I07：性能观测可信性

- Status: Independent repair complete (`I07-W0`～`W8`、`W10`); TaskSpace Exec integration pending
- Authority: [`00-i07-observability-trust-repair-plan.md`](00-i07-observability-trust-repair-plan.md)
- Result: [`01-i07-independent-repair-result.md`](01-i07-independent-repair-result.md)
- Global issue: [`../01-r8-known-issues.md`](../01-r8-known-issues.md) 中的 `R8-I07`
- TaskSpace Exec integration: [`../taskspace-exec/02-engineering-plan.md`](../taskspace-exec/02-engineering-plan.md) 的 `TX-00`、`TX-11`

本目录只处理一个产品问题：性能报告是否忠实反映真实发生的请求、用量、失败和证据边界。它不负责评价 Agent 决策，
不改变 Tool、Map 或 Provider 的执行行为，也不建立第二套长期观测系统。

当前修复分成两部分：

1. 在 TaskSpace Exec 开工前独立修复已经坐实的 request/usage 双计、边界误判和证据新鲜度问题（已完成）；
2. 随 TaskSpace Exec 新协议接入，修正本地尝试、上游请求和 Provider 完成事实之间的身份与阶段关系。

完整 I07 在两部分均验收前保持 `queued`。第一部分完成不等于关闭 I07。
