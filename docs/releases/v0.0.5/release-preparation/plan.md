# WhaleCode v0.0.5 发布准备执行计划

- Status: prepared-awaiting-release-scope
- Product Authority: `../../../../prd/2026-08-21-v0.0.5-release-identity.md#confirmed-product-decisions`
- Applicable Decisions: PD1, PD2

## Execution Contract

- Product Authority 中的 active 决策是本计划唯一用户权威；修改必须获得用户明确批准，Agent 不得自批。
- 工程证据可以修订本计划，但不得静默改写产品权威。
- 新的发布渠道、发布时间和外部副作用必须延期或获得用户确认。
- 每个物质阶段结束后只审计该阶段的 Product Decision Delta。
- 每个物质阶段开始前，必须基于实际实现和证据 rebase 剩余计划；gate 为 `pending` 或 `blocked-on-plan-approval` 时不得开始。
- 物质性计划变化必须记录 Plan Delta 并获得用户批准。

## Design

以 Rust workspace package version 作为 Whale runtime semver 来源，以本目录 `release.json` 作为候选发布登记。preflight 同时读取 upstream candidate，强制 Whale `v0.0.5` 与 Codex `rust-v0.149.0` 分域一致。发布入口保持离线、只读和无外部副作用。

## Pending Product Decisions

| ID | Decision Surface | Current / Proposed Behavior | Why Material | Evidence | Impact If Changed |
| --- | --- | --- | --- | --- | --- |
| P1 | v0.0.5 实际发布范围 | npm 是既有 Whale 独立渠道；本轮仅补齐离线候选，是否发布 npm/tag/GitHub Release 仍待明确授权 | 会产生外部包、权限和长期升级合同 | npm 历史发布记录存在；vendor 工作流仍指向 OpenAI 的其他渠道 | 授权后需要核验账号、六平台制品、dist-tag 和回滚责任 |

## Work Units

| ID | Objective | Change Axis | Change Location | Target Object | Concrete Action | Resulting Behavior | Benefit | Side Effects | Verification | Safe Stop / Rollback | Plan Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | 固定发布身份 | 文档/登记 | PRD、本目录 | PD1/PD2、release.json | 记录产品与 substrate 的独立版本 | 机器和人都能区分两套身份 | 防止错误登记 | 新增少量发布元数据 | JSON/链接检查 | 删除新增登记即可恢复 | verified |
| W2 | 统一 runtime semver | 构建元数据 | `codex-rs/Cargo.toml`、`Cargo.lock` | workspace package version | 将 Whale workspace 版本改为 `0.0.5` | CLI 编译版本为 `0.0.5` | 用户看到正确产品版本 | workspace 内部 crate 版本机械变化 | cargo metadata + `whale --version` | 恢复版本和 lock | verified |
| W3 | 阻断身份混用 | 发布门禁 | `scripts/release/`、root CI | release identity preflight | 校验 Cargo、登记、tag 和 candidate | 错误版本在发布前失败 | 把口头约束变为机器门禁 | 新增一个 Python 脚本和测试，无网络 | unittest + 正反例 | 可独立移除 | verified |
| W4 | 提供候选材料 | 文档 | 本目录、README、runbook | release notes/checklist | 写发布说明草稿与人工交接项 | 发布者能看到已验证与未授权边界 | 降低误发布风险 | 文档维护成本 | 链接和 preflight | 文档可回退 | verified |
| W5 | 验证候选 | 构建/安装 | workspace 隔离环境 | whale binary | 离线检查、构建、安装并执行版本 smoke | 证明源码版本进入真实二进制 | 关闭版本登记到运行时链路 | 本地编译耗时；无模型费用 | doctor + `whale --version` | 保留源码，重建即可 | verified |

## Phase 1：身份与离线门禁

#### Pre-Phase Plan Rebase Gate

- Rebase scope: 当前 Cargo、upstream candidate、发布脚本与文档
- Material plan delta: none
- Plan delta record: not-required
- User approval: not-required
- Gate status: ready

Entry: PD1/PD2 active。执行 W1–W4，不触发外部发布。

Product Decision Delta：仅实现 PD1/PD2，发布渠道 P1 保持未决。

Evidence：release、distribution、brand identity preflight 通过；npm 元包候选离线 staging/pack 门禁通过；release guard 正反例单元测试通过；`git diff --check` 通过。

## Phase 2：候选构建验证

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Phase 1 实现、Cargo lock、preflight 结果和 workspace 状态
- Material plan delta: none
- Plan delta record: not-required
- User approval: not-required
- Gate status: ready

Entry: Phase 1 preflight 与测试通过。执行 W5；不运行真实模型或付费 benchmark。

Evidence：`cargo build -p codex-cli --bin whale --locked` 通过；140 个 local workspace package 均为 `0.0.5`；workspace doctor 通过；隔离槽 `whale --version` 输出 `whale 0.0.5`；npm 元包 staging 后为 `@ceasarxuu/whalecode@0.0.5`，离线 pack inventory 与完整性元数据通过；cache-sensitive index gate 通过（fingerprint `9c817b9f59426efa097be43988d4731a2e7ba412bad63bdf69a466fcbcaaaced`）。

Product Decision Delta：`covered`（PD1/PD2）；没有新增产品语义。

## Phase 3：实际发布

#### Pre-Phase Plan Rebase Gate

- Rebase scope: Whale npm 账号与凭据、获批六平台构建 run、tag/GitHub Release 范围和最终 release notes
- Material plan delta: material
- Plan delta record: pending
- User approval: required-pending
- Gate status: blocked-on-plan-approval

本阶段不在当前授权范围。确认发布渠道并明确授权实际发布后另行 rebase。

## Plan Delta History

| ID | Before Phase | Previous Plan | Current Fact | Proposed Change | Impact | User Approval | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| — | — | — | — | 当前无物质性变更 | — | — | — |
