# R8 缓存命中回归门禁子主题

- Created: 2026-07-31
- Status: 离线工程完成；MVT-0 真实双臂 accepted baseline 已激活
- Scope: DeepSeek 请求稳定前缀、缓存 usage 观测、变更门禁与付费复验
- Related R8 issues: R8-I02、R8-I07、R8-I08

## 1. 子主题目标

当提示词、上下文构造、TaskSpace projection、Tool schema、provider 路由或 wire payload 发生可能影响缓存命中的
变化时，工程门禁应自动发现变化并阻断静默发布，同时满足三个条件：

1. 不把注释、测试和不改变最终请求的重构误判成必须付费复验；
2. 不漏掉真正构造 DeepSeek 请求、Tool 列表和 usage 指标的生产入口；
3. 任何真实 Whale Agent 回归都先向用户申请明确预算，并写入全局运行账本。

本子主题建设的是测试与发布基础设施，不改变 TaskSpace 产品语义，也不替代 R8 各产品问题的根因调查。

## 2. 工程现场

历史 v1 曾具备以下能力：

- 对一组配置路径计算源码内容 SHA-256，并接入 pre-commit 和 non-agent release gate；
- 使用统一 Docker benchmark 运行 Standard 与 map-request 各一次；
- 从 provider usage 读取 request 2+ 的 cached/uncached token；
- 自动登记 `benchmarks/whale-agent-run-ledger.json`，不自动重试；
- 已用 2 个获批 sample 证明当前 map-request 存在可观测缓存退化。

首次结果为 Standard `96.62%`、map-request `35.79%`，两臂业务均成功，usage 覆盖率均为 `100%`。这证明真实
runner 有发现能力，但不证明当前源码指纹门禁的覆盖范围正确。

对抗性审查确认 v1 同时存在严重漏报与误报。当前实现已用生产 final-wire 场景矩阵和完整证据链替代这套代理判断，
并关闭 [唯一问题清单](02-known-issues.md)中的 9 个工程问题。`live_regression_failed` 继续阻断发布；不得把离线
工程完成或一次固定样本解释成新的真实 accepted baseline。

## 3. 文档导航

| 文档 | 职责 | 状态 |
|---|---|---|
| [00-cache-hit-regression-gate.md](00-cache-hit-regression-gate.md) | 记录 v1 历史设计和操作方法 | historical |
| [01-first-validation-result.md](01-first-validation-result.md) | 记录首次两臂真实验证数据与证据 | verified |
| [02-known-issues.md](02-known-issues.md) | 本子主题唯一问题清单 | 9/9 closed |
| [03-repair-plan.md](03-repair-plan.md) | 三段式门禁修复计划与验收顺序 | completed |
| [04-final-wire-call-chain.md](04-final-wire-call-chain.md) | CR-06 生产 final-wire 调用链与本地捕获证据 | verified |
| [05-final-wire-evidence.md](05-final-wire-evidence.md) | CR-07 原始 body SHA 与结构化证据合同 | verified |
| [06-final-wire-comparison-policy.md](06-final-wire-comparison-policy.md) | CR-08 final-wire 保护面与差异分类合同 | verified |
| [07-production-tool-wire.md](07-production-tool-wire.md) | CR-09 普通 Tool 与 TaskSpace Tool 的生产 wire 合同 | verified |
| [08-provider-usage-decoder.md](08-provider-usage-decoder.md) | CR-10 两种 wire API 的 usage 解码合同 | verified |
| [09-usage-aggregation-contract.md](09-usage-aggregation-contract.md) | CR-11 Rust decoder 与 Python 聚合一致性合同 | verified |
| [10-standard-request-pair.md](10-standard-request-pair.md) | CR-12 Standard 连续两请求 final-wire 基准 | verified |
| [12-cr20-free-semantic-gate-result.md](12-cr20-free-semantic-gate-result.md) | CR-20 免费语义门禁实现与验收结果 | verified |
| [14-cr21-2-cr23-closeout.md](14-cr21-2-cr23-closeout.md) | CR-21.2 至 CR-23 实现、验证和剩余外部状态 | implementation verified |
| [15-authorized-run-budget-boundary.md](15-authorized-run-budget-boundary.md) | 真实回归的硬成本边界、观测阈值与超时回收 | implementation verified |
| [16-first-authorized-revalidation-result.md](16-first-authorized-revalidation-result.md) | 首次获批 revalidation 的预检失败、成本边界与后续动作 | diagnosed |
| [17-authorized-replacement-result.md](17-authorized-replacement-result.md) | 获批替代运行暴露的 provider 路由与 RunId 身份链阻塞 | diagnosed |
| [18-provider-route-preflight-repair.md](18-provider-route-preflight-repair.md) | transport alias 启动前预检、final-wire 等价与证据身份闭环 | closure passed；真实复验待预算 |
| [19-provider-terminal-usage-repair.md](19-provider-terminal-usage-repair.md) | provider terminal usage 唯一事实源与 binary-health 前置修复 | implementation verified；真实双臂待预算 |
| [20-single-arm-exit-contract-repair.md](20-single-arm-exit-contract-repair.md) | 单臂 cache smoke 与双臂 E2 退出语义冲突修复 | implementation verified；真实双臂待预算 |
| [21-mvt0-accepted-baseline-result.md](21-mvt0-accepted-baseline-result.md) | MVT-0 双臂真实结果、trace 与 accepted baseline | accepted |
| [22-approved-budget-contract-v3.md](22-approved-budget-contract-v3.md) | 用户批准预算与 Provider 理论容量分离、请求间 usage 预算监督 | implementation verified；真实复验待执行 |
| [对抗性审查](../../../../vs_review/2026-07-31-cache-regression-surface-review.md) | 独立审查漏报、误报和控制面完整性 | historical findings closed |
| [收尾对抗性审查](../../../../vs_review/2026-08-01-r8-cache-gate-closeout-review.md) | CR-21.2 至 CR-23 多轮独立闭环审查 | closure passed；P0/P1=0 |

`02-known-issues.md` 是缓存门禁工程缺陷的唯一清单。R8 产品问题状态仍以
[`../01-r8-known-issues.md`](../01-r8-known-issues.md) 为唯一事实源，两者不得重复登记或相互关闭。

## 4. 目标门禁模型

```text
可能相关的源码或配置变化
  -> 免费源码风险哨兵
  -> 免费生产 wire payload 场景矩阵
       -> 最终 payload 与缓存测量合同未变：通过，不申请付费运行
       -> 最终 payload 或测量合同变化：阻断并输出受影响场景
  -> 用户批准对应场景的真实 provider 预算
  -> 真实缓存 usage + 业务正确性验证
  -> 按精确 commit、模型、arm 和场景晋升证据
```

源码哨兵只决定是否运行免费测试，不直接决定是否需要真实 API。最终发送给 DeepSeek 的生产请求是缓存语义的
权威边界；测试必须复用生产构造和序列化路径，禁止维护第二套测试专用 serializer。

## 5. 执行约束

- 修复按 [03-repair-plan.md](03-repair-plan.md) 的小单元顺序执行，每个单元单独验证和提交；
- 只有完整 proposal、用户授权、执行证据、acceptance 和共享校验通过后才能晋升 `accepted` 基线；
- 免费 deterministic fixture 不计入 Whale Agent 预算；任何真实 provider 请求仍需用户单独批准；
- 一次真实回归只验证其实际覆盖的 commit、模型、arm 和场景，不得扩大解释范围；
- 门禁自身不能宣称抵抗有仓库写权限的恶意维护者；目标是阻止意外绕过、证据错配和未经审查的自授权；
- 修复完成后必须执行新的空白对抗性审查，blocking finding 未关闭前不得恢复发布权威性。

当前 CR-21.2 至 CR-23 的离线实现已完成：免费合同可严格区分未变、已变和不可比较；预算提案无 API/账本副作用；
授权一次性原子认领；失败或越预算结果不可晋升；pre-commit 允许明确可比较的候选产品提交，但 release 继续阻断，
直到独立 accepted 基线提交形成。付费执行启用硬上限时，Agent 无真实 Key 和直接 provider 出口，隔离代理只接受
批准模型的 Responses 请求并权威记录全部 dispatch；Whale wire 对账只控制性能证据资格。跨平台超时会终止进程树
并清理容器、网络与 host secret。当前仓库仍是历史 `live_regression_failed`，本轮没有真实 provider 运行。

2026-08-02 的 MVT-0 获批运行新增了 1 个 Standard 真实样本。其业务成功，但旧 runner 因混用 rollout 重复
快照而拒绝完整 usage；提交 `0076e720a` 已把 provider terminal 设为唯一计量事实，并将 binary-health 前置。
原始 artifact 已离线复算成功，新的 Standard + map-request 对照仍需单独预算。

第二次获批运行 `WAR-20260802-180016-CACHE-REGRESSION-2E8B3F50` 再次完成一个业务成功且 usage 完整的 Standard，
request 2+ 命中率为 `97.5422%`。底层 benchmark 却因未运行右臂而无法形成双臂 E2，返回退出码 1；缓存 runner
按停止条件未启动 map-request。提交 `c2246a6f1` 已让缓存专用单臂命令显式接受非 E2 结果，其他执行、业务、usage、
预算和清理门禁不变。本次授权同样已经消费，不能复用。

修复后运行 `WAR-20260802-181842-CACHE-REGRESSION-7A794B3A` 完成 Standard 与 map-request 双臂，业务、usage、
provider boundary 和清理全部通过。request 2+ 命中率分别为 `97.90%` 与 `67.85%`；用户明确接受该结果作为
MVT-0 当前基线。Promotion 只更新三种 TaskSpace final-wire 快照，Standard 保持不变；性能差距继续作为后续产品
问题，不因基线接受而关闭。

最终离线验收为 Python `195 passed, 0 skipped`，账本 Schema、PowerShell、容器/provider、non-agent、E3 和 release
自测全部通过；最终空白审查在 HEAD `bbbf1fc16` 未发现 P0/P1。历史 `live_regression_failed` 只表示尚未获得新的
用户授权真实 accepted baseline，不再表示缓存门禁工程仍有 open 问题。
