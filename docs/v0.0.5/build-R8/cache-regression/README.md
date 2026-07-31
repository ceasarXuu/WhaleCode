# R8 缓存命中回归门禁子主题

- Created: 2026-07-31
- Status: Active; v1 仅作诊断，不能作为发布可信门禁
- Scope: DeepSeek 请求稳定前缀、缓存 usage 观测、变更门禁与付费复验
- Related R8 issues: R8-I02、R8-I07、R8-I08

## 1. 子主题目标

当提示词、上下文构造、TaskSpace projection、Tool schema、provider 路由或 wire payload 发生可能影响缓存命中的
变化时，工程门禁应自动发现变化并阻断静默发布，同时满足三个条件：

1. 不把注释、测试和不改变最终请求的重构误判成必须付费复验；
2. 不漏掉真正构造 DeepSeek 请求、Tool 列表和 usage 指标的生产入口；
3. 任何真实 Whale Agent 回归都先向用户申请明确预算，并写入全局运行账本。

本子主题建设的是测试与发布基础设施，不改变 TaskSpace 产品语义，也不替代 R8 各产品问题的根因调查。

## 2. 当前工程现场

当前 v1 已具备以下能力：

- 对一组配置路径计算源码内容 SHA-256，并接入 pre-commit 和 non-agent release gate；
- 使用统一 Docker benchmark 运行 Standard 与 map-request 各一次；
- 从 provider usage 读取 request 2+ 的 cached/uncached token；
- 自动登记 `benchmarks/whale-agent-run-ledger.json`，不自动重试；
- 已用 2 个获批 sample 证明当前 map-request 存在可观测缓存退化。

首次结果为 Standard `96.62%`、map-request `35.79%`，两臂业务均成功，usage 覆盖率均为 `100%`。这证明真实
runner 有发现能力，但不证明当前源码指纹门禁的覆盖范围正确。

对抗性审查确认 v1 同时存在严重漏报与误报：真实 wire/context/tool/provider 构造入口未完整覆盖，而测试、注释和
格式变化会因为原始文件字节变化触发付费复验。当前 `live_regression_failed` 状态继续阻断发布；不得把
`structural_bootstrap` 或一次固定样本解释成完整缓存敏感面的验证。

## 3. 文档导航

| 文档 | 职责 | 状态 |
|---|---|---|
| [00-cache-hit-regression-gate.md](00-cache-hit-regression-gate.md) | 记录 v1 已实现设计和操作方法 | reviewed，待替换 |
| [01-first-validation-result.md](01-first-validation-result.md) | 记录首次两臂真实验证数据与证据 | verified |
| [02-known-issues.md](02-known-issues.md) | 本子主题唯一问题清单 | reviewed |
| [03-repair-plan.md](03-repair-plan.md) | 三段式门禁修复计划与验收顺序 | planned |
| [对抗性审查](../../../../vs_review/2026-07-31-cache-regression-surface-review.md) | 独立审查漏报、误报和控制面完整性 | reviewed，blocking |

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
- 在控制面可信性修复完成前，不晋升任何 `live_verified` 基线；
- 免费 deterministic fixture 不计入 Whale Agent 预算；任何真实 provider 请求仍需用户单独批准；
- 一次真实回归只验证其实际覆盖的 commit、模型、arm 和场景，不得扩大解释范围；
- 门禁自身不能宣称抵抗有仓库写权限的恶意维护者；目标是阻止意外绕过、证据错配和未经审查的自授权；
- 修复完成后必须执行新的空白对抗性审查，blocking finding 未关闭前不得恢复发布权威性。

当前执行位置：`CR-01`、`CR-02` 已关闭；`CR-03` 已由提交 `38fc62830` 验证，关联问题 CR-I03 尚未关闭；
下一单元为 `CR-04`。
