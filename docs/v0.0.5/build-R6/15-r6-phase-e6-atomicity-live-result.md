# R6 Phase E6 原子性与 Live 门禁结果

- Created: 2026-07-16
- Updated: 2026-07-16
- Status: Completed
- Scope: Phase E6 and Phase E closeout
- R6 runtime commit: `0ce775278`
- R6 observer commit: `84a35dbaa`
- Frozen R5 commit: `d12818f055494e`
- Related design: `10-r6-terminal-replay-convergence-design.md`

## 1. 结论

Phase E6 与 Phase E 的全部退出门禁通过，可以进入 Phase F。

R6 的 6 个 Docker live run 全部通过公开测试和隐藏 oracle，全部由 Agent 显式调用一次
`finish_end`，Root/Finish 同 revision 闭合。每个 raw terminal envelope 的 snapshot hash 都与 canonical
Rust replay proof 一致；没有 plain-final-open-map、terminal recovery request、partial commit 或自动终结。

本阶段验证的是终结、持久化和恢复可靠性，不宣称成本收益。R6 相对同轮 Standard 的请求放大在 simple/complex
分别为 `1.40x/1.16x`，input 为 `1.52x/1.35x`；成本优化属于 Phase F/G，不能倒灌到 Runtime 终结语义。

## 2. 实施结果

### 2.1 Terminal 原子事务

```text
Agent finish_end
  -> clone candidate ActionMap
  -> validate and close Root + Finish at one revision
  -> construct terminal envelope with exact Agent summary
  -> persist envelope and flush durability barrier
  -> install durable in-memory candidate
  -> return terminal carrier
```

持久化失败时不安装 candidate、不生成 carrier。`terminal_committed` envelope 同时携带 snapshot、hash、revision、
graph revision 和 terminal trace；它是唯一 durable terminal checkpoint。旧式 split finish graph event 被明确识别为
`incomplete_transaction`，不会静默恢复为半闭合状态。

### 2.2 Replay、resume 与 fork

- production resume、fork 和离线 observer 共用 canonical Rust replay；
- terminal envelope 作为完整 checkpoint 被校验，hash/sequence/patch/corruption 任一错误都明确失败；
- terminal 后的 Root/Finish/revision/summary ref 从同一 snapshot 恢复；
- Hook 未修改，也不参与 terminal 选择、提交或 replay。

### 2.3 本阶段发现并修复的反馈/观测缺口

| 缺口 | 根因 | 修复 | 结果 |
|---|---|---|---|
| Agent 在 `finish.goal` 与 `root.goal` 间反复错误修正 | internally tagged enum 的 serde 错误丢失嵌套路径 | 先解析 action，再对具体 payload 做 path-aware deserialize | `finish`、`root`、`additional_work_nodes[0]` 可唯一定位；复杂样本不再 bootstrap 循环 |
| R6 state failure 被报告为 0 | observer 只允许旧 Result V1/V2 | failure allowlist 纳入 `TaskSpaceControlResultR6V1` | 6-run 重放恢复 1 次 protocol、2 次 state failure |
| observer 从相对路径启动 replay 失败 | observer cwd 与 runner cwd 不同 | Whale binary 在 preflight 后规范化为绝对路径 | agent、validator、observer 使用同一 binary |
| R6 runtime event 时间解析失败 | observer 只按 DateTime 解析 Unix ms | 增加 Unix ms/sec/ISO 的机械解析 | graph health 可读取 live R6 event |

相关 COE：

- `coe/2026-07-16-07-47-r6-taskspace-argument-path-loss.md`
- `coe/2026-07-16-08-15-r6-control-failure-observer-version-gap.md`

## 3. 确定性验证

| 验证 | 结果 | 覆盖重点 |
|---|---:|---|
| `cargo test -p codex-protocol --lib` | 197 passed | terminal durable envelope 协议 |
| `cargo test -p codex-core taskspace_replay_tests --lib` | 18 passed | checkpoint/delta/terminal/corruption |
| `cargo test -p codex-core rollout_reconstruction --lib` | 33 passed | resume/fork/reconstruction |
| `cargo test -p codex-core terminal_transaction --lib` | 4 passed | persist-before-install 与 failure injection |
| `cargo test -p codex-core taskspace_terminal_contract --test all -- --nocapture` | 2 passed | carrier sole-final、plain final no retry |
| `cargo test -p codex-core taskspace_control --lib` | 21 passed | typed result、路径反馈、terminal identity |
| `cargo test -p codex-core tools::sequence::tests --lib` | 11 passed | batch/barrier/carrier 透传 |
| `cargo test -p codex-tools taskspace_tool --lib` | 3 passed | provider schema 合同 |
| `test-cost-instrumentation.ps1` | PASS | R6 control failure 分类 |
| `test-performance-observation.ps1` | PASS | 报告聚合口径 |
| `test-harness.ps1` | PASS | Docker benchmark harness 回归 |
| `just bazel-lock-check` | PASS | Rust/Bazel 依赖锁一致 |
| `cargo build -p codex-cli --bin whale --locked` | PASS | CLI 开发构建 |

Phase B 的 20-cycle reducer/replay fixture 与本阶段 terminal resume/fork/crash/corruption fixture 共同覆盖
20-cycle 状态 hash 和 terminal 边界。本阶段未执行 full workspace Rust test；按项目规则该操作仍需用户单独授权。

## 4. Live 方法

两个样本均在 Docker hard boundary、`deepseek-v4-flash`、同一 public validator 和 hidden oracle 下运行。每个
TaskSpace arm 重复 3 次并轮换左右位置：

- simple：`single-file-fast-fix`；
- complex：`subscription-billing-repair`。

R5 使用 detached worktree 和其原生 harness，避免 R6 observer 对 R5 legacy proof 增加产品兼容。R5 与 R6 各自
和同次 paired Standard 比较；跨版本绝对值是非同时历史对照，ratio 只使用各自 paired Standard。

两个 preflight abort 不计入结果：一次为 commit 后 binary attestation 未刷新，一次为当前 shell 未导入
`.env.local`。两次都在 provider 请求前 fail-fast，没有模型成本。

## 5. Correctness 与 Map

| Sample | Arm | Solved | Public/Hidden | Map closed | Nodes | Edges | 显式终结 |
|---|---|---:|---:|---:|---:|---:|---|
| simple | Standard | 3/3 | 3/3 | N/A | 0 | 0 | Standard final |
| simple | R5 | 3/3 | 3/3 | 1/3 | 9 | 0 | legacy control；2 次 Map 仍 active |
| simple | R6 | 3/3 | 3/3 | 3/3 | 11 | 8 | `finish_end` 3/3 |
| complex | Standard | 3/3 | 3/3 | N/A | 0 | 0 | Standard final |
| complex | R5 | 3/3 | 3/3 | 2/3 | 10 | 0 | legacy control；1 次 Map 仍 active |
| complex | R6 | 3/3 | 3/3 | 3/3 | 12 | 9 | `finish_end` 3/3 |

R5 的业务结果能通过，但零边 Map 与未闭合 Map 说明“任务完成”和 Map 状态仍可能分离。R6 的明确收益是把
Root、Work、Finish、边、终结和 replay 变成同一个状态机事实，不是让 Runtime 判断任务语义。

## 6. 成本明细

表内格式为 `总和 / 均值 / 中位数`；时间单位为秒，cache 为 request 2+ 加权命中率。

### 6.1 Simple

| Arm | Requests | Tools | Wall | Input | Uncached | Output | Cache |
|---|---:|---:|---:|---:|---:|---:|---:|
| Standard | 20 / 6.67 / 7 | 28 / 9.33 / 10 | 40.50 / 13.50 / 13.04 | 140,021 / 46,674 / 49,015 | 6,261 / 2,087 / 2,167 | 4,062 / 1,354 / 1,307 | 95.17% |
| R5 | 23 / 7.67 / 8 | 27 / 9.00 / 9 | 54.35 / 18.12 / 18.49 | 177,854 / 59,285 / 63,108 | 9,662 / 3,221 / 3,150 | 5,995 / 1,998 / 2,041 | 94.17% |
| R6 | 28 / 9.33 / 9 | 26 / 8.67 / 8 | 68.49 / 22.83 / 23.63 | 213,502 / 71,167 / 68,001 | 20,862 / 6,954 / 6,902 | 7,931 / 2,644 / 2,635 | 89.74% |

### 6.2 Complex

| Arm | Requests | Tools | Wall | Input | Uncached | Output | Cache |
|---|---:|---:|---:|---:|---:|---:|---:|
| Standard | 37 / 12.33 / 13 | 63 / 21.00 / 20 | 139.23 / 46.41 / 43.29 | 396,993 / 132,331 / 132,510 | 19,905 / 6,635 / 6,411 | 15,921 / 5,307 / 5,412 | 94.83% |
| R5 | 36 / 12.00 / 12 | 72 / 24.00 / 22 | 152.21 / 50.74 / 50.88 | 461,425 / 153,808 / 154,767 | 19,953 / 6,651 / 6,742 | 20,291 / 6,764 / 6,430 | 95.60% |
| R6 | 43 / 14.33 / 11 | 64 / 21.33 / 18 | 155.50 / 51.83 / 53.54 | 536,250 / 178,750 / 132,423 | 46,394 / 15,465 / 12,999 | 18,791 / 6,264 / 6,694 | 91.18% |

### 6.3 各自 paired Standard ratio

| Sample | Arm | Requests | Tools | Wall | Input | Uncached | Output | Cache delta |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| simple | R5 | 1.15x | 0.96x | 1.20x | 1.27x | 1.54x | 1.25x | -1.02pp |
| simple | R6 | 1.40x | 0.93x | 1.69x | 1.52x | 3.33x | 1.95x | -5.43pp |
| complex | R5 | 0.90x | 1.20x | 1.21x | 1.10x | 1.15x | 1.37x | -0.13pp |
| complex | R6 | 1.16x | 1.02x | 1.12x | 1.35x | 2.33x | 1.18x | -3.65pp |

R6 complex 的 request 中位数为 11，低于总和所暗示的稳定水平；pair-002 是 21-request outlier。trace 显示其
主要来自 Agent 的 patch 轨迹：两次 context 不匹配、一次同 response 三 patch 被既有单 patch hard rule 拒绝，
随后拆分修复；另有一次未 bind 即 complete 的 state reject。反馈均完整，Agent 下一次立即纠正，不是 terminal
recovery 或上下文丢失。该成本问题留给 Phase F/G 的上下文唯一性和请求结构分析。

## 7. 六次 R6 终结证明

| Sample/Repeat | Finish | Control failure | Final revision | Raw terminal hash = replay hash |
|---|---:|---|---:|---:|
| simple/1 | 1 | protocol 1 | 6 | PASS |
| simple/2 | 1 | 0 | 6 | PASS |
| simple/3 | 1 | 0 | 4 | PASS |
| complex/1 | 1 | state 1 | 6 | PASS |
| complex/2 | 1 | state 1 | 6 | PASS |
| complex/3 | 1 | 0 | 6 | PASS |

三次拒绝均是底线规则：空 `mutate_graph`，或未 bind 即 complete。state failure 均明确
`state_commit=false/partial_commit=0`；没有 Runtime 语义纠正、额外 provider recovery request 或失败后状态扭曲。

## 8. 退出门禁

| Gate | 判定 |
|---|---|
| 没有 Agent terminal call 时 Root/Finish 不闭合 | PASS |
| terminal 预检或持久化失败不产生部分提交/carrier | PASS |
| 成功 terminal 同 revision 闭合 Root/Finish，summary 原样保留 | PASS |
| 20-cycle replay 与 terminal resume/fork hash 一致 | PASS |
| corruption 与 split terminal 明确 fatal | PASS |
| Finish READY 只暴露 named control，不选择具体 variant | PASS |
| plain final 不产生成功 completion 或 recovery request | PASS |
| observer、raw terminal envelope 与 replay hash 一致 | PASS，6/6 |
| simple/complex 三臂 public/hidden pass rate | PASS，18/18 arm-runs |
| R6 `finish_end` adoption | PASS，6/6 |

## 9. 运行经验

1. Rust/source commit 后必须刷新 `whale.build-attestation.json`；Cargo no-op build 的 mtime 不能代替 commit/hash 证明。
2. benchmark runner 只读取进程环境，不自动加载 `.env.local`；启动命令需先静默 source，且不得打印 secret。
3. observer 子进程可能改变 cwd，所有 binary path 必须在 preflight 后绝对化。
4. frozen R5 必须使用其原生 harness/replay 合同；不为实验数据向 R6 production 增加兼容分支。
5. performance report 生成后必须读取 events 文件，并对 typed failure 与 raw trace 抽样对账。

## 10. 下一阶段

Phase F 收敛 projection、tool schema、反馈和上下文唯一 owner。优先检查 R6 的 uncached input、named/auto
tool-choice shape 变化和基础上下文/Map 重复；不通过 Runtime 语义约束压低请求数，也不在 Phase F 提前叠加压缩策略。
