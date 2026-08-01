# Subagent VS Review: R8 provider boundary transport alias

- Created: 2026-08-01T15:26:19+0800
- Updated: 2026-08-01T15:39:18+0800
- Report schema: adversarial-v1
- Task: 审查提交 `30d60a96f` 是否正确修复缓存 smoke 对保留 `deepseek` provider ID 的非法覆盖。
- Report path: `vs_review/2026-08-01-r8-provider-boundary-alias-review.md`
- Review mode: fresh internal subagents
- Source session policy: no inherited main-agent context
- Status: blocked

## Round 1: implementation and evidence-chain review

### Review Input

#### Objective

验证 benchmark 使用现有 custom provider transport alias 经过隔离 provider boundary 时，能够保持 DeepSeek
运行语义、缓存可比性和成本证据完整性，并且没有通过放宽生产合同或增加无关产品能力掩盖原始配置错误。

#### Review Target

提交 `30d60a96f` 的代码实现、配置合同、离线测试、日志证据与关闭文档。

#### Target Locations

- `benchmarks/taskspace/container-runtime-contract.json`
- `scripts/taskspace-benchmark/lib/container-contract.ps1`
- `scripts/taskspace-benchmark/run-taskspace-benchmark.ps1`
- `scripts/taskspace-benchmark/test-container-contract.ps1`
- `scripts/taskspace-benchmark/test-provider-boundary.ps1`
- `scripts/cache-regression/test_cache_run_contract.py`
- `third_party/codex-cli/codex-rs/core/tests/suite/cache_provider_boundary_route.rs`
- `third_party/codex-cli/codex-rs/core/tests/suite/cache_payload_contract.rs`
- `scripts/cache-regression/cache_run_analysis.py`
- `scripts/cache-regression/cache_run_ledger.py`
- `coe/2026-08-01-13-11-cache-smoke-reserved-provider-route.md`
- `docs/v0.0.5/build-R8/cache-regression/17-authorized-replacement-result.md`

#### Change Introduction

当 benchmark 启用 provider request hard limit 时，不再写入保留的
`model_providers.deepseek.base_url`，而是选择自定义 `deepseek-boundary` provider。该 provider 的字段以合同形式
声明为与内置 DeepSeek 等价，base URL 指向 Docker provider proxy；artifact 同时记录 logical provider 与 transport
provider。生产保留 ID 校验和 final-wire payload 合同未放宽。

#### Risk Focus

- PowerShell 生成的 CLI 配置与 Rust 测试构造的 TOML 是否真正等价。
- provider ID 变化是否影响默认模型、认证、重试、压缩、子 Agent、会话、遥测或缓存请求体。
- Standard 与 TaskSpace 两臂是否仍经过同一 provider 路径，且实验变量未污染。
- provider boundary、usage、ledger、final-wire 与 release gate 是否对 logical/transport 身份产生错配。
- 失败是否会在 provider 请求前明确暴露，而不是再次浪费真实运行预算。
- 测试是否只验证同一份合同的自洽值，而没有覆盖生产入口。

#### User-Perspective Review Focus

- 运行者是否能从 artifact 看懂真实模型身份和本地隔离路由。
- 配置错误是否给出可诊断反馈，且不会误报为 Agent、缓存或 API Key 问题。
- 后续真实缓存复验是否需要额外隐含操作。

#### Implementation Completeness Focus

- 修复是否进入实际 benchmark 参数构造和 Docker Agent 启动路径。
- provider alias 是否被生产配置解析器验证，而非只被测试替身接受。
- 路由身份是否持久化到 resolved manifest 与每臂 argv artifact。
- 最终请求、provider boundary 对账、缓存分析和账本是否仍可工作。
- 是否存在仅靠 mock、自定义测试构造或文档声明才能成立的结论。

#### Target Benefit Focus

- 声称的收益仅限于消除启动前配置拒绝并保持请求语义，不把实际缓存命中率视为已验证。
- 检查离线证据能否证明工程修复，及真实缓存效果是否仍明确保持发布阻断。

#### Assumptions To Attack

- `name = "DeepSeek"` 足以触发所有 DeepSeek 专用运行行为。
- 显式 `-m deepseek-v4-flash` 消除了自定义 provider 默认模型差异。
- custom provider 的未声明字段默认值与内置 DeepSeek 相同。
- transport provider ID 不参与 DeepSeek 服务端缓存键或本项目缓存证据判定。
- hard-limit 分支之外不应注入 transport alias。
- 当前测试能够发现未来内置 provider 字段增加后的 alias 漂移。

#### Adversarial Lenses

- implementation-completeness
- state
- failure
- data
- testing
- observability
- maintenance

#### Verification Status

- `provider_boundary_alias_matches_builtin_deepseek_runtime_fields`: 通过。
- DeepSeek `cache_payload_` 9 项：通过。
- cache regression Python 196 项：通过。
- Docker provider boundary、container contract、benchmark harness、E3 proof harness：通过。
- cache regression gate：开发检查通过，识别为待真实验证，发布保持阻断。
- 未在修复后执行真实 Whale Agent run；不得把缓存命中效果标记为已证明。

#### Reviewer Instructions

- Fresh internal subagent session.
- No inherited main-agent context.
- Read target files and relevant production call paths directly.
- Do not modify files.
- Try to falsify the implementation and evidence chain rather than confirm it.
- Cite evidence paths and line numbers when possible.
- Return summary, blocking findings, non-blocking risks, user-perspective checks,
  implementation-completeness checks, target-benefit checks, required fixes,
  missing tests, missing observability, and evidence.

### Internal Subagent Unavailable Fallback

- Required only when fresh internal subagents are unavailable.
- Internal subagent unavailable reason: n/a
- Local CLI discovery commands: n/a
- Discovered CLI candidates: n/a
- User-recommended alternative agent requested: n/a
- User-recommended agent command: n/a
- User-recommended agent verification: n/a
- User approval requested: n/a
- User-approved CLI command: n/a
- User decision: n/a
- Fallback outcome: n/a

### Reviewer Timeout Policy

| Complexity | Initial Wait | Extension | Max Attempts Per Role | Blocking Closure Behavior |
|---|---:|---:|---:|---|
| normal | 12 minutes | one bounded 8-minute extension | 2 | accepted blocking finding requires a fresh closure review |

### Reviewer Selection

| Reviewer | Reason Selected | Risk Area |
|---|---|---|
| implementation-adversary | 最高风险是配置组合、生产调用路径和失败对账是否正确 | correctness, failure handling, evidence integrity |

### Reviewer Launch Records

| Reviewer | Internal Mechanism | Session / Job ID | Trace Source | Context Forked | Input Packet | Context Explicitly Excluded | Read-only |
|---|---|---|---|---|---|---|---|
| implementation-adversary | `multi_agent_v1__spawn_agent` | `019fbc38-6be1-7262-b396-3525a100f499` (Hume) | spawn tool result and subagent completion notification | `fork_context=false` | Round 1 Review Input and report path | main-agent history, reasoning, drafts, conclusions, full diff | yes |

### Reviewer Timeout Records

| Reviewer Output Key | Reviewer Role | Attempt | Session / Job ID | Waited | Status | Reason | Action |
|---|---|---:|---|---:|---|---|---|
| R1-Hume | implementation-adversary | 1 | `019fbc38-6be1-7262-b396-3525a100f499` | about 8 minutes | completed | structured review returned normally | completed |

### Reviewer Outputs

#### R1-Hume

##### Summary

Verdict: **BLOCKED**. 提交很可能移除了具体的保留 ID 覆盖，但 reviewer 认为生产入口验证、alias final-wire
和 provider 身份证据链仍不完整。reviewer 未修改文件，也未运行测试。

##### Blocking Findings

- **B1: 没有生产代表性的 dispatch 前配置测试。**
  - Broken assumption: 手工构造 `(key, TomlValue)` 并转为 `ConfigToml`，等价于 PowerShell 字面 `-c` 参数通过真实 `exec` 配置入口。
  - Failure scenario: quoting、CLI parsing、merge precedence、完整配置校验或容器参数发生偏差，alias 在 dispatch 前再次被拒绝；缓存 runner 已把这次命令计为实际 sample。
  - Trigger condition: `Get-TaskspaceProviderBoundaryConfigOverrides`、`New-TaskspaceWhaleArgv`、`CliConfigOverrides::parse_overrides` 与 `Config::load_config_with_layer_stack` 发生偏差。
  - Impact: 再次发生零请求失败，浪费一次授权槽位、操作时间和证据周期；若偏差晚于首次 dispatch 才暴露，还可能产生费用。
  - Proof needed: 在 `actual_sample_runs` 增加前，以真实 Whale binary、精确生成的 `-c` 字符串、容器 HOME/env、CLI parser 和完整 `Config` 做无 provider dispatch preflight，并验证 provider ID、字段、模型和 base URL。
  - Evidence: `scripts/taskspace-benchmark/lib/container-contract.ps1:110`；`third_party/codex-cli/codex-rs/utils/cli/src/config_override.rs:42`；`third_party/codex-cli/codex-rs/core/tests/suite/cache_provider_boundary_route.rs:43`；`scripts/taskspace-benchmark/test-provider-boundary.ps1:90`；`scripts/cache-regression/run_cache_hit_regression.py:182`。

- **B2: final-wire 证据没有执行 transport alias。**
  - Broken assumption: 内置 DeepSeek final-wire 测试通过即可证明 alias final-wire 行为。
  - Failure scenario: provider-ID-dependent 分支改变请求构造、子 Agent 配置、session reconstruction 或未来 provider-specific 行为；现有 fixture 强制 `model_provider_id=deepseek`，仍会保持绿色。
  - Trigger condition: benchmark 实际使用 `model_provider_id=deepseek-boundary`。
  - Impact: release evidence 声称 provider identity 与 payload 受保护，但没有覆盖 benchmark 使用的身份。
  - Proof needed: 从精确 alias override 加载生产 session，并经 mock `/responses` 与内置 provider 比较 raw request；reviewer 建议额外覆盖 compaction 和 child-agent dispatch。
  - Evidence: `third_party/codex-cli/codex-rs/core/src/config/mod.rs:2060`；`third_party/codex-cli/codex-rs/core/tests/suite/cache_payload_contract.rs:207`；`benchmarks/cache-regression/final-wire-comparison-policy.json:14`；`third_party/codex-cli/codex-rs/core/tests/suite/cache_provider_boundary_route.rs:72`。

- **B3: evidence 与 ledger 没有证明 runtime 实际解析的 provider 身份。**
  - Broken assumption: 在 `whale-argv.json` 记录路由声明即可证明 runtime 使用了该路由。
  - Failure scenario: 配置优先级漂移或 artifact 不一致导致 runtime 解析其他 custom provider；boundary、cache analysis、arm validation 和 ledger 仍可能接受相同模型/URL/body。
  - Trigger condition: resolved provider 与声明的 transport alias 不一致。
  - Impact: 官方证据可以把 logical provider 写为 `deepseek`，但无法审计实际 transport provider。
  - Proof needed: reviewer 建议 runtime 发出 resolved ID、logical ID、provider-info digest 与 endpoint，并绑定到 boundary、observations、arm comparison、result integrity 和 ledger。
  - Evidence: `scripts/cache-regression/cache_run_analysis.py:95`；`scripts/cache-regression/cache_arm_identity.py:18`；`third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs:80`；`scripts/cache-regression/cache_run_ledger.py:201`。

##### Non-blocking Risks

- **N1: session 与 telemetry 记录 transport alias，而不是 logical provider。**
  - Broken assumption: alias ID 仅是传输局部信息。
  - Failure scenario: session、turn analytics、state filtering 与 rollout metadata 以 `deepseek-boundary` 分片。
  - Trigger condition: alias benchmark session 被用于恢复或跨运行分析。
  - Impact: benchmark 的 clean HOME 限制直接影响，但审计身份可能难以理解。
  - Proof needed: alias resume/list/state/telemetry 测试与显式 logical mapping。
- **N2: hard-limit 模式未限制为 DeepSeek 模型。**
  - Broken assumption: 启用 hard limit 的调用者一定传入 DeepSeek 模型。
  - Failure scenario: 非 DeepSeek 模型仍被路由到 DeepSeek alias/proxy。
  - Trigger condition: `ProviderRequestHardLimit > 0` 且 `Model` 不匹配合同。
  - Impact: 边界启动失败或运行无效，浪费授权槽位。
  - Proof needed: 在 provider route 建立前拒绝不匹配模型。
- **N3: provider model cache 不按 provider 分键。**
  - Broken assumption: alias 环境不会读取其他 provider 的 fresh `models_cache.json`。
  - Failure scenario: 复用 HOME 时模型元数据串用。
  - Trigger condition: 非隔离 HOME 中存在新鲜 cache。
  - Impact: 模型元数据可能与 alias 不匹配。
  - Proof needed: provider-keyed cache 或空 cache 断言。
- **N4: Standard-first 的服务端缓存串扰没有证明已隔离。**
  - Broken assumption: 固定臂顺序不会让第二臂继承第一臂缓存收益。
  - Failure scenario: 同一账户/模型先跑 Standard，再跑 TaskSpace，后者测量被预热。
  - Trigger condition: 服务端跨 session 复用共享前缀缓存。
  - Impact: 缓存收益比较可能偏向第二臂；这是收益测量风险，不是已证明的实现错误。
  - Proof needed: counterbalanced order、cold-cache control 或 provider cache namespace。

##### User-Perspective Checks

- Usability: partial - resolved manifest 与 argv artifact 有两个 ID，但 durable observation/ledger 未绑定。
- Ease of use: risk - alias 错误在已计数 benchmark 内才暴露，没有无成本前置步骤。
- Ease of understanding: risk - stderr 可区分配置错误，但缺少稳定的 route-preflight 状态码。
- Release safety: pass - accepted cache baseline 与 release 仍保持阻断。

##### Implementation Completeness Checks

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| alias override injection | 两臂共享 alias | runner common argv | Docker Agent launch | contract tests | argv artifact | none | landed | none |
| exact CLI/full Config load | 精确字面参数可加载 | CLI + `Config` | paid-run preflight | hand-built TOML only | manual probe only | partial | partial | B1 |
| alias normal final wire | alias 与内置 body 相同 | production session client | mock `/responses` | built-in ID only | none | test-only gap | partial | B2 |
| compaction | 保持 DeepSeek 策略 | provider-name strategy | session compact | provider field equality | none | none | partial | B2 |
| child agents | 继承 alias | role reload | spawn | source evidence | none | none | partial | B2 |
| route identity evidence | 声明与解析身份一致 | harness + artifacts | analysis/ledger | declaration tests | no resolved attestation | partial | partial | B3 |
| release blocking | 未有 live baseline 时阻断 | cache gate | release gate | passed | pending-validation status | none | landed | none |

##### Target Benefit Checks

| Claimed Benefit | Baseline | Target | Measurement Method | Comparison Evidence | Result | Regression / Side Effect | Status | Finding Link |
|---|---|---|---|---|---|---|---|---|
| 消除保留 provider 启动拒绝 | 旧运行 0 请求失败 | exact alias config loads | production-entry preflight | source + partial parser probe | unmeasured on exact entry | authorization slot risk | weak-evidence | B1 |
| 保持 DeepSeek 请求语义 | built-in final-wire | alias body equal | raw mock request comparison | built-in fixtures only | unmeasured | future ID branch risk | weak-evidence | B2 |
| 保持证据身份完整 | ledger logical provider | logical/transport reconciled | artifact integrity | declaration only | partial | attribution ambiguity | weak-evidence | B3 |
| 保持发布阻断 | no accepted live baseline | blocked | cache gate | gate output | achieved | none | proven | none |
| 实际缓存命中收益 | failed pre-fix run | accepted threshold | real smoke | none post-fix | unmeasured | unknown | deferred | N4 |

##### Required Fixes

- **RF-B1:** 在实际 sample 计数前增加 exact production-entry、no-dispatch provider route preflight。
- **RF-B2:** 增加 alias normal final-wire 与内置 DeepSeek 的 raw request 等价测试。
- **RF-B2-X:** reviewer 建议把 compaction 和 child-agent dispatch 一并升级为 blocking alias tests。
- **RF-B3:** 把 resolved logical/transport provider identity 绑定到正式证据与 ledger。
- **RF-B3-X:** reviewer 建议把 logical/transport identity 注入每个 runtime wire event。
- **RF-DOC:** 在 B1-B3 关闭前撤销 COE 的 `fixed` 表述。
- **RF-N2:** 非 DeepSeek 模型不得启用当前 provider boundary route。

##### Missing Tests

- **MT-B1:** exact PowerShell override 字符串通过 CLI parser 与完整 `Config`。
- **MT-B1-CONTAINER:** 真实 Whale binary 在 benchmark container 中以生成 argv 完成 no-dispatch preflight。
- **MT-B2:** alias normal final-wire 与内置 DeepSeek 请求等价。
- **MT-B2-COMPACTION:** alias compaction strategy/request。
- **MT-B2-CHILD:** alias child-agent、guardian/reviewer 与 resume。
- **MT-B3-LEDGER:** logical/transport identity 缺失或不一致时 ledger 拒绝。
- **MT-B3-ARM:** 两臂 route identity 缺失或不一致时拒绝。
- **MT-N2:** 非 DeepSeek model + hard limit 被拒绝。
- **MT-N4:** counterbalanced order 或显式服务端 cache isolation。

##### Missing Logs / Observability

- **ML-B1:** `provider_route_preflight_started/completed/failed`。
- **ML-B3:** resolved transport provider、logical provider、provider-info digest、endpoint、wire API 与 model。
- **ML-B3-WIRE:** 每个 wire request 的 provider identity。
- **ML-B2-CHILD:** child-agent resolved provider identity。
- **ML-N1:** session/telemetry logical-versus-transport mapping。
- **ML-BOUNDARY:** boundary rejection count 与 upstream completion/status summary。
- **ML-LEDGER:** ledger 分离 logical provider 与 transport provider。

##### Evidence

- `scripts/taskspace-benchmark/lib/container-contract.ps1:110` - PowerShell 生成字面 route override。
- `third_party/codex-cli/codex-rs/utils/cli/src/config_override.rs:42` - production CLI raw-string parser。
- `third_party/codex-cli/codex-rs/core/src/client.rs:1755` - request client 只持有 resolved provider object，不持有 provider ID。
- `third_party/codex-cli/codex-rs/core/src/compact.rs:89` - compaction strategy 按 provider capabilities/name 判定。
- `third_party/codex-cli/codex-rs/core/src/agent/role.rs:264` - child role reload 显式保留当前 provider ID。
- `scripts/taskspace-benchmark/lib/container-runtime.ps1:184` - 每个容器 HOME 位于该臂独立 artifact mount。
- `scripts/cache-regression/run_cache_hit_regression.py:182` - benchmark 命令启动前即增加 actual sample 计数。
- `scripts/cache-regression/cache_arm_identity.py:18` - arm identity 未校验 route identity。
- `scripts/cache-regression/cache_run_ledger.py:201` - ledger 仅记录 logical `deepseek`。

### Main Agent Response

| Reviewer | Finding | Broken Assumption / Failure Scenario | Severity | Decision | Evidence / Reason | Action Taken | Follow-up |
|---|---|---|---|---|---|---|---|
| implementation-adversary | B1 / RF-B1 / MT-B1 / MT-B1-CONTAINER / ML-B1 | 当前测试绕过精确 CLI 与完整 config 入口，可能再次零请求失败 | blocking | accept | Rust test 使用手工 `TomlValue`；cache runner 在命令前计数 | 本轮仅审查，未修改代码 | 设计一次性的 no-dispatch production-entry preflight，并在修复后 fresh re-review |
| implementation-adversary | B2 / RF-B2 / MT-B2 | built-in final-wire 未直接证明 alias normal request | blocking | accept | fixture 强制 built-in ID；虽然 `ModelClient` 只消费 provider object，仍缺少入口到 raw wire 的整体测试 | 本轮仅审查，未修改代码 | 增加一个正常请求的 alias/built-in raw body 等价测试 |
| implementation-adversary | RF-B2-X / MT-B2-COMPACTION | compaction 需要同级 blocking alias test | blocking | reject | `compact_strategy` 只消费 `ModelProviderInfo`，字段等价测试覆盖输入；不存在 provider-ID 分支 | none | 可作为非阻断未来回归，不作为本修复 blocker |
| implementation-adversary | MT-B2-CHILD / ML-B2-CHILD | child/guardian/reviewer alias dispatch 未逐项测试 | blocking | reject | role reload 明确保留当前 ID，provider definition 来自同一 config layer；请求 client 不消费 ID | none | 仅当正常 alias final-wire 或 role reload 测试暴露差异时升级 |
| implementation-adversary | B3 / RF-B3 / MT-B3-LEDGER / MT-B3-ARM / ML-B3 / ML-LEDGER | route declaration 未与 resolved transport identity 形成正式证据绑定 | blocking | accept | manifest/argv 只有声明；arm identity 和 ledger 未校验 route | 本轮仅审查，未修改代码 | 在 harness 层生成一次 resolved route attestation，并绑定 arm/result/ledger |
| implementation-adversary | RF-B3-X / ML-B3-WIRE | 每个 runtime wire event 都应携带 logical/transport identity | blocking | reject | logical provider 是 benchmark 语义，runtime 只知道实际 provider；逐请求注入会扩大 runtime 责任，且 boundary endpoint/body hash 已逐请求对账 | none | route attestation 留在 harness/evidence 层，不侵入 runtime wire schema |
| implementation-adversary | RF-DOC | 当前 COE `fixed` 超前 | blocking | accept | COE 同时把工程问题标为 fixed，又保留未满足的真实 smoke criterion | 待本报告提交时改回 validating | B1/B2/B3 关闭后再标 fixed |
| implementation-adversary | N1 / ML-N1 | alias 改变 session/telemetry 本地 ID | non-blocking | accept | `model_provider_id` 确实进入 session metadata；这是 transport alias 的预期结果但需映射 | 由 B3 harness/ledger mapping 覆盖 | 不向 runtime 注入第二套 semantic identity |
| implementation-adversary | N2 / RF-N2 / MT-N2 | 非 DeepSeek 模型可误用 DeepSeek boundary | non-blocking | accept | route 仅描述 DeepSeek，runner 当前未校验 Model | 本轮仅审查 | 在 route 建立前做机械模型合同校验 |
| implementation-adversary | N3 | model cache 可能跨 provider 串用 | non-blocking | reject | 每臂使用独立 artifact mount 下的临时 HOME，容器销毁；benchmark 显式传模型 | none | 若未来复用 HOME，需重新审查 |
| implementation-adversary | N4 / MT-N4 | 固定 Standard-first 可能造成服务端缓存串扰 | non-blocking | defer | 风险真实但早于本提交存在，且不影响本次配置启动修复；需要独立实验设计决策 | 记录在本报告 | 在下一次缓存收益实验设计前处理，不并入 alias 修复 |
| implementation-adversary | ML-BOUNDARY | boundary 缺少 rejection/upstream 状态汇总 | non-blocking | reject | 当前 boundary evidence 已记录 status、errors、request count，并与 wire hash 对账；本问题没有证据显示不足 | none | 出现无法分类的 boundary 故障后再升级 |

### Closure Status

- Blocking findings found: yes
- Accepted blocking findings fixed: no
- Blocking re-review completed: no
- Blocking re-review passed: no
- Blocking re-review round links:
  - pending after B1/B2/B3 repair
- Blocking re-review launch records:
  - pending after B1/B2/B3 repair
- Rejected findings backed by evidence: yes
- Deferred findings documented: yes
- Implementation completeness gaps resolved or accepted by user: no
- Target benefit warnings recorded: yes
- Blocked reason: exact production-entry preflight、alias normal final-wire 和 route evidence binding 尚未落地
- Allowed to proceed: no

## Final Conclusion

审查已完成但修复闭环未通过。`30d60a96f` 不应作为可执行真实缓存复验的最终状态；应先完成收敛后的 B1、B2、B3
与 N2，并由新的 fresh reviewer 做 blocking closure review。不得把 reviewer 提出的逐请求 logical identity 注入、全角色
alias 测试或 provider-keyed model cache 直接扩入本主题。
