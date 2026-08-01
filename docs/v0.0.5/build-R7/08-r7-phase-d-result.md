# R7 Phase D map-request 接入结果

- Created: 2026-07-19
- Updated: 2026-07-19
- Status: Engineering Complete / Product Finding Open / Phase E Not Started
- Production Commit: `148f1d816`
- Observer Commit: `8202c3a1a`
- Machine Result: `benchmarks/taskspace/r7/phase-d-result.json`
- COE: `coe/2026-07-19-04-58-r7-map-request-complex-interruption.md`

## 1. 阶段结论

Phase D 的工程合同已经接通：三种 policy 共用一个 `taskspace_control.read_map` schema 和同一 renderer；
`map-request` 的普通 provider request 不再自动放入完整 projection，Agent 显式读取时才通过 tool result
得到当时的完整 Map。初始/恢复上下文只放机械 Map handle，Runtime 没有自动读取、提醒、纠错或读取频率
判断。

机械退出门禁全部通过。两个 Docker TaskSpace 臂共 30 个 provider request，自动 projection 均为 0；
简单样本由 Agent 主动读取 1 次，返回 revision 3、canonical lag 0，随后正常闭合。合法但未读取 Map 的
普通工具没有新增 policy rejection：复杂样本共尝试 34 次普通工具，其中初始化前 2 次被原
`no_task_path` hard gate 拒绝，初始化后 32 次均由原 binding 合法承载。

复杂样本同时暴露出明确的产品风险：Agent 完成代码和 8/8 validator 后，没有维护 Map，而是直接输出
普通 final。既有 R6 终局门禁按冻结合同产生 `taskspace_terminal_protocol_violation`，且明确不发起
纠正请求，因此 CLI 以 exit 1 结束。该 hard gate 不是 Phase D 引入；当前证据只能证明 map-request
全程没有当前 projection 与 Agent 遗忘并存，尚不能把二者确认为因果。Phase E 不自动启动，避免后续
工作掩盖这个单次负向样本。

## 2. 工程改造

| 区域 | 结果 |
|---|---|
| Shared tool | `taskspace_control` 新增零参数 `read_map`，三策略 schema 相同 |
| Policy | `map-request`: provider request -> `None`；explicit read -> `ReturnAsToolResult` |
| Renderer | `read_map` 直接调用共享 current-projection renderer，不复制 Map 构造逻辑 |
| Context | 初始/恢复只提供 `TaskSpaceMapHandleR7V1`；普通 request 不自动附加 projection |
| Scanner | map-request 要求 automatic projection count 为 0，仍检查大输出和越界 marker |
| Hard gates | 空 Map、binding/lease、Root/Finish 和 terminal gate 均未修改 |
| Observer | 新增 read request/completion/failure、重复 revision、canonical lag 和 stale error 指标 |
| Logs | `map.read_completed` 保留 policy、revision、前次读取、canonical/projection hash |

观察日志把“相邻两次读取之间的 revision 前进量”命名为
`revision_advance_since_previous_read`，把返回值相对 canonical 的滞后命名为
`canonical_revision_lag`，避免把两个不同概念混成一个 lag。

## 3. 退出门禁

| 门禁 | 结果 | 证据 |
|---|---|---|
| automatic full projection = 0 | PASS | simple 12/12、complex 18/18 payload 均为 0 |
| `read_map` 对齐 canonical | PASS | simple 1/1 completed，revision 3，canonical lag 0 |
| shared tool schema | PASS | `codex-tools` 141 passed / 1 ignored |
| bypass 仍被拒绝 | PASS | terminal contract 2/2；复杂样本 plain final 被原 hard gate 拒绝 |
| 未读取时合法 ordinary action 不被拒绝 | PASS | complex 初始化后 32 次 ordinary tool，无 TaskSpace gate rejection |
| 无自动 read/reminder/correction | PASS | 只有 Agent call_id 对应的 1 个 `map.read_completed`；无自动事件 |
| simple correctness/closure | PASS | Standard/R7 均 solved；R7 5 nodes / 4 edges / open 0 |
| complex correctness/closure | MIXED | 文件修复与 8/8 validator 通过，但 R7 未闭合 Map，CLI interrupted |

最后一项不是 emission 实现门禁失败，而是 map-request 产品行为和既有 no-retry terminal gate 的组合风险；
因此必须作为负向结果保留，不能写成全绿效用结论。

## 4. 三臂快速对照

Frozen R6 来自 Phase A 冻结基线；Current Standard 和 R7 map-request 来自本阶段同提交、同 Docker
hard boundary 的成对运行。每臂仅 1 次，只用于工程诊断。

### 4.1 Simple：single-file-fast-fix

| 指标 | Current Standard | Frozen R6 | R7 map-request |
|---|---:|---:|---:|
| 结果 | solved | solved | solved |
| provider request | 7 | 19 | 12 |
| ordinary / control | 9 / 0 | 9 / 9 | 10 / 9 |
| wall time | 16.13s | 46.55s | 28.46s |
| input token | 49,826 | 231,221 | 131,219 |
| cached input | 47,744 | 207,744 | 119,424 |
| uncached input | 2,082 | 23,477 | 11,795 |
| output token | 1,731 | 4,499 | 3,138 |
| request 2+ cache hit | 95.53% | 89.74% | 96.42% |
| message prefix preserved | 100.00% | 88.89% | 100.00% |
| explicit read | 0 | N/A | 1 completed / lag 0 |
| Map nodes / edges / open | 0 / 0 / 0 | 5 / 4 / 0 | 5 / 4 / 0 |

相对当前 Standard，R7 request `1.71x`、wall `1.76x`、input `2.63x`、uncached input `5.67x`。
相对 Frozen R6，R7 request 减少 36.8%，input 减少 43.2%，uncached input 减少 49.8%，并恢复
100% message prefix preservation。简单样本上，按需读取在保留 Map 工作流的同时明确降低了 R6 成本。

### 4.2 Complex：subscription-billing-repair

| 指标 | Current Standard | Frozen R6 | R7 map-request |
|---|---:|---:|---:|
| 结果 | solved | solved | validator pass / terminal fail |
| provider request | 14 | 16 | 18 |
| ordinary / control | 20 / 0 | 15 / 7 | 34 / 1 |
| wall time | 49.55s | 58.93s | 71.14s |
| input token | 149,935 | 209,772 | 287,027* |
| cached input | 142,848 | 184,704 | 279,040* |
| uncached input | 7,087 | 25,068 | 7,987* |
| output token | 6,034 | 5,767 | 9,507* |
| request 2+ cache hit | 95.12% | 87.87% | 97.19% |
| message prefix preserved | 100.00% | 86.67% | 100.00% |
| explicit read | 0 | N/A | 0 |
| Map nodes / edges / open | 0 / 0 / 0 | 4 / 3 / 0 | 4 / 4 / 1 |

`*` CLI 因终局协议错误退出，通用 metrics 将 usage 标为 unavailable；表中 token 来自覆盖 18/18 请求
的 provider request terminal trace，不伪造为正式成功运行成本。

R7 的未缓存 input 只比 Standard 高 12.7%，比 Frozen R6 低 68.1%，request 2+ cache hit 达
97.19%。因此缓存目标明确达成；总 input 仍高，是 18 个请求、34 次普通工具尝试和固定 27,377 bytes/request
工具 schema 共同造成，不是 projection 成本。由于 R7 没有合法终局，不能用该成本与 solved 臂做效用
优劣结论。

## 5. 复杂样本失败链

```text
request 1: Agent先尝试普通工具，bootstrap hard gate拒绝
request 2: initialize_map 成功，revision=2，绑定 explore_project
request 2 continuation + request 3..17: 初始化后 32 次 ordinary tool；无 read_map、无 transition
request 18: provider 正常完成，输出普通最终总结
local terminal gate: map仍 work_active -> taskspace_terminal_protocol_violation
CLI: exec_exit_code=1, completion=interrupted，不自动重试
```

直接原因置信度高：Agent 没有提交 Map lifecycle/`finish_end`，既有 hard gate 拒绝 plain final。更深层
原因只有中等以下证据：18 个 request 都没有当前 projection，但初始化 call/output 与固定 tool schema
并未丢失；Phase C map-append 的同一复杂样本 3/3 solved，支持显著性可能影响行为，但不足以确认因果。
完整证据门见对应 COE。

## 6. 测试与构建

```text
just fmt                                                     PASS
just fix -p codex-tools                                      PASS（仅既有 warning）
just fix -p codex-core                                       PASS（仅既有 warning）
cargo test -p codex-tools --lib                              141 passed / 1 ignored
cargo test -p codex-core phase_d_ --lib                      PASS
cargo test -p codex-core map_request --lib                   4 passed
cargo test -p codex-core read_map --lib                      1 passed
provider canonical projection shape test                     1 passed
taskspace_terminal_contract                                  2 passed
cost instrumentation selftest                                PASS
performance observation selftest                             PASS
R7 projection policy contract                               PASS
cargo build -p codex-cli --bin whale --locked               PASS
Docker simple Standard/R7                                   solved / solved
Docker complex Standard/R7                                  solved / terminal fail
```

构建二进制 SHA256：`3f4611ce78fd91fbd56976c2e0f7e9e8c0fb302a9278da7d2f0a593753f628a7`。

## 7. 运行证据与经验

```text
target/r7-phase-d/request/simple/single-file-fast-fix/20260719-045253-348
target/r7-phase-d/request/complex/subscription-billing-repair/20260719-045442-647
```

两次前置无效运行没有进入 provider，不纳入结果：第一次未将 `.env.local` 导出到当前进程；第二次把
`map-request` 放进 `RunRoot`，触发 benchmark 的 treatment-neutral cwd 保护。后续 shell 运行应使用
`set -a; source .env.local; set +a`，且 `RunRoot` 不得包含 `map`、`taskspace`、`node` 等 treatment
标签。密钥内容未写入日志或文档。

## 8. 后续门禁

Phase D 的代码与机械合同完成，但 Phase E 保持未启动。进入 Phase E 前应先明确：

1. 将复杂样本视为 map-request 已知产品权衡，留到 Phase G 重复矩阵判断稳定性；或
2. 单独重新审视 R6 plain-final no-retry 终局合同；或
3. 对 map-request 做受控重复实验，证明当前 Map 可见性与生命周期遗忘是否存在因果。

在没有更多证据前，不增加自动 read、动态提醒、下一步建议或 Runtime 语义纠错。
