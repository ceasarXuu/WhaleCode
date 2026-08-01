# CR-21.2 至 CR-23 收尾结果

- 日期：2026-08-01
- 状态：离线工程与 fresh closure review 完成；工程 P0/P1 已关闭
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
| `cccb47004` 至 `76568a061` | arm/artifact 身份、执行输入、共享计数、稳定清理和付费控制脚本授权闭合 |
| `ecd1e929b`、`26c5a87fd` | Realtime 按每次推理计数；无法前置计数的自动生成在硬上限下 fail closed |
| `45bd0c2f6` | Windows timeout 使用系统整树终止语义 |
| `fac57d0d8`、`d620349f9` | 真实 Key 与权威计数移入隔离 provider boundary，并清理配套容器和网络 |
| `e619890f1` | provider boundary 限定 method/path/model，并将真实 dispatch 与 Whale wire trace、正式结算逐条核对 |
| `84b94ceef` | Realtime 按 parser 归一化后的真实模式检查，非法 hard-limit 配置统一 fail closed |
| `d08be480a`、`123116c4a` | Windows Job Object 持有完整进程树；host provider secret 清理成为结算必需后置条件 |
| `72566a1b6` | 晋升中的退出码、repeat、请求数和 token 使用精确整数合同，拒绝布尔值冒充 |
| `a3344da1d` | Windows 子进程在创建时原子进入 Kill-on-close Job，durable recovery 不再覆盖旧 handle owner |
| `9ae528efb` | 预算标量使用精确类型，正式授权 JSON 拒绝重复 key |
| `5821b3354` | partial 结果可恢复，全局账本忠实区分请求精确值与已知下界 |
| `1042384ff` | recovery 在账本锁内绑定原 durable claim 的身份、矩阵与预算 |
| `e1fa83ef1`、`8083f31ab` | 结构化证据与 recovery scope 使用精确 JSON 类型比较 |
| `79f1c1d8c` | exact/inexact 请求证据改为互斥合同，Schema 与 PowerShell 一致 |
| `650657a1d` | proposal 与 recovery 共用完整 selection 合法性合同 |
| `b49765f47`、`94c3cf53e` | result 请求汇总由 attempts 复算，生产形状夹具同步 |
| `ad6df97d7` | completed attempt/cleanup/token 完整性与 unsettled 下限单调保留 |
| `bbbf1fc16` | direct settlement 绑定批准矩阵，请求下限先于证据复制持久化 |
| `837460b75`、`0d3af4b54` | 跨平台账本锁、完整 promotion 清理合同，以及全部 Realtime 模式 fail closed |
| `040c27ae6` | Windows 挂起创建后先入 Job；supervisor 请求计数与 performance/token evidence 分离结算 |
| `809e1d513`、`8bd820a9a`、`3410db334` | 结算事务、严格证据类型、实际 arm 身份、Windows 异常 owner 与网络稳定空状态收口 |
| `dc1faeecd`、`3b291b111` | 失败 handle 的可重试 owner 与复算证据精确类型合同 |
| `a23b29cb6`、`1ba6c1232`、`9204926c2` | 三 policy/cross-arm wire 身份、原子 recovery 与跨解释器 Windows owner journal |
| `13022a905` | 持久 Windows journal 路径与跨进程 launch handoff 锁 |

## 3. 离线验证

- Python cache control plane：`195 passed, 0 skipped`；provider boundary 固定上游、恢复与对账反例包含在内。
- 全局 Whale Agent ledger 的 JSON Schema 和 PowerShell checker 均通过；partial/unknown 请求数负例包含在内。
- 免费生产合同：7 条命令全部通过；覆盖 final-wire、Tool、provider、模型、默认路由、usage decoder 和比较合同。
- `cargo fmt --all -- --check`：通过；仅有 stable toolchain 对 nightly rustfmt 选项的既有警告。
- non-agent gate builder、E3 start gate、release decision 三组 PowerShell 测试：通过。
- Realtime 回归：`codex-core` 43 项、`codex-api` 46 项通过；包含 8 项 provider hard-limit 定向测试。显式生成
  逐次计数，自动生成、V1 归一化绕过和非法配置均在建连前 fail closed。
- provider boundary Docker 自检：Agent 无真实 secret mount、只有 internal network、不能直连 mock provider；
  非批准 route/model 与超额请求均在上游前拒绝，监督事件与 wire trace 不一致时正式证据拒绝；容器、网络和
  secret 均无残留。
- 正式 release gate：因 `live_regression_failed` 返回阻断，符合当前外部状态。
- 全部验证均为 mock/fixture/offline；本轮没有真实 Whale Agent 或 provider 请求。
- 最终空白 reviewer `019fbb4f-f64a-7ae0-ac4c-3c04c17140da`：P0=0、P1=0。唯一 P2 是多文件
  promotion 非事务；中间 dirty 状态会被正式 clean-HEAD 与 manifest 重验阻断，不构成发布绕过。

## 4. 尚未宣称的收益

本轮没有新的 DeepSeek API 运行，因此没有证明当前产品缓存命中率改善，也没有建立新的 accepted baseline。工程层面
证明的是门禁能可靠发现变化、约束成本、保留证据边界并拒绝残缺晋升。激活 release 权威需要用户另行批准最小真实
smoke 预算，并对实际结果作明确 acceptance。
