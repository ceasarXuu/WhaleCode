# Phase B4 VA-04A 问题重映射结果

- 日期：2026-08-09
- 状态：通过
- 真实 Whale Agent / Provider 请求：0
- 权威问题清单：[`../01-r8-known-issues.md`](../01-r8-known-issues.md)

## 1. 目标与边界

本轮只用当前生产源码、确定性测试和 Phase B4 离线证据重映射 R8-I01～I10。历史 control、sibling、developer
carrier 和旧 benchmark 结果只保留为历史，不再证明当前 TaskSpace Exec 的行为。离线证据只能确认代码路径是否存在，
不能代替目标 Provider 下的 Agent 行为和成本测量。

## 2. 重映射结果

| 分类 | 问题 | 当前结论 | Phase B5 观察点 |
|---|---|---|---|
| 已关闭 | I09 | 关系化 Store hydrate、图合法性和恢复测试继续成立 | 无 |
| 静态关闭候选 | I01 | 旧 control/final receipt 双 revision 路径已删除；Exec 只有一个 outer 结果，request revision 仅由 Runtime 使用 | 是否仍出现 stale 重试 |
| 静态关闭候选 | I02 | 旧高优先级 carrier 和 TaskSpace 专属 Event Store 已删除；当前链没有重复 developer 注入 | final wire 是否只携带一次事实 |
| 静态关闭候选 | I05 | preflight 在副作用前拒绝，未提交候选不进入 Map；失败只通过 outer Tool pairing 返回 | Agent 是否准确理解失败且不重复同错 |
| 静态关闭候选 | I06 | TaskSpace 顶层仅 Exec 与 Hosted；完整 plan 先预检，顶层 client 绕过和多 Patch 均确定性拒绝 | 生产入口是否出现旁路 |
| 当前静态缺口 | I10 | catalog 在单请求内共享同一快照且声明确定，但 provider wire、dispatch、cache/report 尚未共用一个 capability identity | 先补工程身份，再进入成本比较 |
| 工程完成待生产验收 | I07 | canonical request facts 已消除双计；OB-01/OB-02 已消费 response/outer/action/node/revision 身份 | 当前 trace 的 completeness、freshness 和逐 ID 对账 |
| 行为待验证 | I03 | 当前结构化 Exec 合同与旧 control+sibling 不同，历史失败不能外推 | 合法动作组合及协议拒绝 |
| 行为待验证 | I04 | DAG/readiness 硬规则通过，Agent 如何选择 frontier 尚无当前证据 | 错选未就绪或已完成节点 |
| 成本待验证 | I08 | 新协议尚无同 commit 四臂真实数据 | request、token、cache、time、cost 与业务结果 |

## 3. 确定性证据

- 固定离线验收：Core 1856/3、TaskSpace Exec 57、settlement/recovery 11、State 134、CLI 5、Viewer 4、
  App Server Protocol 183，workspace、zero-base 与 cache gate 全部通过。
- 当前生产符号审计未发现 `taskspace_control`、sibling manifest、final receipt 或 TaskSpace developer carrier。
- TaskSpace 顶层入口、整批预检、单 Patch、Hosted 分离和 lifecycle 顺序均有生产 Router 测试。
- OB-01 已把 request/response/outer/action/node/revision 接入现有日志；OB-02 以 I07 canonical request facts 统计
  请求与成本，并以 Exec 事件统计内部动作。

详细证据分别见 [`19-phase-b4-observability-audit.md`](19-phase-b4-observability-audit.md)、
[`21-phase-b4-performance-observer-result.md`](21-phase-b4-performance-observer-result.md) 和
[`22-phase-b4-offline-acceptance.md`](22-phase-b4-offline-acceptance.md)。

## 4. 结论

VA-04A 完成，Phase B4 达到退出条件。它没有关闭 I01/I02/I05/I06，也没有对 I03/I04/I08 的行为或成本作推断。
Phase B5 必须先处理 I10 的当前工程缺口，再按独立预算执行 VA-02、VA-03，最后由 VA-04B 更新唯一问题清单。
