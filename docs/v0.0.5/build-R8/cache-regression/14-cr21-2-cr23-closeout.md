# CR-21.2 至 CR-23 收尾结果

- 日期：2026-08-01
- 状态：离线工程完成；Round 2 blocking 已修复，fresh closure review 待完成
- 真实 Whale Agent/provider 运行：0
- 当前 release：按预期阻断，基线仍为 `live_regression_failed`

## 1. 已完成链路

```text
缓存敏感源码变化
  -> 免费 final-wire 完整发现
  -> 明确 changed 候选可先形成产品 HEAD（不具 release 权威）
  -> 在该 HEAD 上生成阻断报告
  -> 人工选择 model/sample/arm/repeat 和上限
  -> 零副作用预算提案
  -> 用户授权与提案精确绑定
  -> runner 在文件锁内一次性认领 authorization_id
  -> 按精确矩阵执行并记录 attempt、artifact、usage、预算和 ledger
  -> 用户 acceptance 只接受实际 scope 与 changed scenario
  -> 共享 source-aware 校验器复算全部证据
  -> 独立 baseline/snapshot 晋升提交
  -> release 只接受完整可复算的 accepted baseline
```

代码未变但最近一次真实回归失败时，可显式使用 `--request-revalidation`。该路径要求干净 HEAD、失败基线、完整免费
合同通过，并允许 accepted scenario 集合为空；它不会自动触发真实运行或自动接受结果。

## 2. 关键提交

| 提交 | 内容 |
|---|---|
| `5edb46ce6` | 有界、无副作用预算提案 |
| `26fa4625f` | 免费合同不因首个失败停止，保留完整发现 |
| `27d5aa5f9` | 执行与精确用户授权绑定 |
| `cbed6ce00` 至 `c7a5cddc6` | 发现、接受、晋升状态和证据持久化 |
| `724433022` | 授权一次性原子认领，失败/越预算硬失败 |
| `5ab106517` | 扩展 provider/protocol/Cargo 触发面，阻断 index 混读 |
| `3ed8ddc0f` | fixture 证据与 formal release 证据隔离 |
| `2edb5458a` | 晋升与 release 共用 source-aware 证据校验 |
| `97a6e0c9c` | 候选提交到独立晋升的可执行生命周期与显式复验 |
| `5173f8f89` | 正式结构化证据、共享完整校验、控制面闭包、崩溃原子账本与恢复 |
| `7d233aaa6`、`e5d4c3afd`、`f8f1a2180` | provider 进程共享请求硬上限，覆盖全部已知请求出口 |
| `4e7c293ad` | 官方最坏成本、观测阈值、runner 传参、超时容器回收和部分费用结算 |

## 3. 离线验证

- Python cache control plane：`110 passed`。
- 免费生产合同：7 条命令全部通过；覆盖 final-wire、Tool、provider、模型、默认路由、usage decoder 和比较合同。
- `cargo fmt --all -- --check`：通过；仅有 stable toolchain 对 nightly rustfmt 选项的既有警告。
- non-agent gate builder、E3 start gate、release decision 三组 PowerShell 测试：通过。
- provider request hard limit：3 项 Rust 单测通过；子 Agent 使用同一进程计数器，超额 dispatch 在请求前拒绝。
- 正式 release gate：因 `live_regression_failed` 返回阻断，符合当前外部状态。

## 4. 尚未宣称的收益

本轮没有新的 DeepSeek API 运行，因此没有证明当前产品缓存命中率改善，也没有建立新的 accepted baseline。工程层面
证明的是门禁能可靠发现变化、约束成本、保留证据边界并拒绝残缺晋升。激活 release 权威需要用户另行批准最小真实
smoke 预算，并对实际结果作明确 acceptance。
