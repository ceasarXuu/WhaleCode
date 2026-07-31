# 缓存命中回归门禁

- Created: 2026-07-31
- Status: Implementing
- Scope: Prompt、上下文构造、TaskSpace projection、provider payload、Tool declaration

## 1. 要解决的问题

DeepSeek 的上下文缓存依赖请求之间完全相同的前缀。提示词内容、消息角色或顺序、动态 projection 的插入位置、
Tool schema 都可能改变此前可复用的前缀。普通单元测试即使全部通过，也无法证明 provider 实际报告的缓存命中
没有退化；但每次提交都调用真实模型又会产生不必要的 API 成本。

本门禁把“发现风险”和“验证真实影响”分成两层：

1. **确定性敏感面门禁**：免费计算相关源码内容的稳定指纹。指纹与已验证基线不同就立即阻断提交。
2. **真实缓存回归**：用户批准预算后，运行最小的 Standard 与 map-request 对照，并以 provider usage 为准判定。

确定性门禁不声称预测缓存命中率。它只保证所有可能改变请求稳定前缀的已知入口都不能静默越过真实验证。

## 2. 缓存敏感面

权威配置是 `benchmarks/cache-regression/cache-surface-contract.json`，当前覆盖：

| 类别 | 风险 |
|---|---|
| Base instructions | 改变所有请求共享的固定 system 前缀 |
| Context construction | 改变消息角色、顺序、追加或替换方式 |
| TaskSpace projection/feedback | 改变动态内容的位置、载体或重复表达 |
| Provider payload | 改变最终发送结构或 usage 观测 |
| Tool declarations | 改变每次请求携带的 Tool schema |

指纹由匹配文件的相对路径和文件内容 SHA-256 构成，不依赖时间戳、构建目录或本机环境。

## 3. 门禁行为

开发者执行：

```bash
python3 scripts/cache-regression/check_cache_regression_gate.py --source index
```

相同检查已接入 `.githooks/pre-commit` 和 v0.0.5 non-agent gates。

- 指纹等于基线且基线状态可接受：通过。
- 指纹变化：退出码 `20`，列出 staged 变更和对应风险原因。
- 最近一次真实回归失败：即使指纹未再变化也持续阻断。
- 禁止通过 `git commit --no-verify` 绕过；需要向用户解释变化并申请真实运行预算。

首次接入允许 `structural_bootstrap` 状态，仅表示变更检测链路已经建立，不表示真实缓存性能已经验证。首次真实
回归通过后才晋升为 `live_verified`。

## 4. 最小真实回归

固定样本为 `single-file-fast-fix`：

| 项目 | 配置 |
|---|---|
| 模型 | `deepseek-v4-flash` |
| Arm | Standard、TaskSpace `map-request` |
| Repeat | 每臂 1 次 |
| 总 sample run | 2 |
| 自动重试 | 0 |
| 环境 | 现有统一 Docker benchmark |

获批后执行：

```powershell
pwsh scripts/cache-regression/run_cache_hit_regression.ps1 `
  -AuthorizationReference "<用户批准说明>"
```

runner 在启动前向 `benchmarks/whale-agent-run-ledger.json` 写入 `planned`，结束、失败或取消后结算请求数、
input/cached/uncached/output token、估算费用、耗时和证据路径。两臂任一失败都不会自动重试。
凭据优先继承进程环境；若缺失，只读取仓库 `.env.local` 中的 `DEEPSEEK_API_KEY`，不执行文件内容，也不把值写入
日志或结果。

## 5. 判定与晋升

判定只采用 `provider-cache-trace-summary.json` 中 request 2+ 的真实 provider usage：

- usage trace coverage 必须为 `100%`；
- 至少存在一个 request 2+，否则样本不具备发现能力；
- Standard request 2+ 命中率不得低于 `85%`；
- map-request request 2+ 命中率不得低于 `75%`；
- 已有 live 基线后，任一 arm 相对基线下降不得超过 5 个百分点；
- 业务验证仍须通过，缓存指标不能替代正确性。

通过后使用：

```bash
python3 scripts/cache-regression/promote_cache_baseline.py \
  benchmarks/cache-regression/results/<result>.json
```

晋升脚本验证结果状态、当前敏感面 SHA 和运行规模，不接受失败结果或超过 2 个 sample 的结果。

## 6. 已知边界

1. DeepSeek 缓存是 best-effort；一次真实回归适合作为小成本冒烟门禁，不等于统计性性能结论。
2. 指纹规则必须随新的 prompt/context/provider/tool 构造入口更新，否则会形成观测盲区。
3. 本门禁检测请求前缀退化，不诊断其产品根因；失败后仍需逐 request 对比 payload 和 trace。
4. 真实回归是付费操作，门禁不得自行运行，也不得把已有阶段授权解释为新增预算。

## 7. 外部依据

- [DeepSeek Context Caching](https://api-docs.deepseek.com/guides/kv_cache/)：缓存按相同前缀自动命中，并在 usage
  中报告 hit/miss token。
- [DeepSeek Pricing](https://api-docs.deepseek.com/quick_start/pricing/)：缓存命中与未命中采用不同输入价格。
- [Git Hooks](https://git-scm.com/docs/githooks/2.46.0.html)：`pre-commit` 非零退出可阻断提交，但本地 hook 可被
  `--no-verify` 绕过，因此还需共享 gate/CI 约束。
- [GitHub Required Status Checks](https://docs.github.com/en/repositories/configuring-branches-and-merges/managing-protected-branches/about-protected-branches)：
  受保护分支可以要求最新提交通过指定检查；远端接入后应将同一确定性门禁设为 required check。
