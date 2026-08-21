# U19 Codex CLI 0.149 发布收口报告

## 结论

当前 `whalecode-codex` vendor 已追赶至官方稳定版 `rust-v0.149.0`。固定对象为 tag object `a4e15bf371341b067c8278d3b70b1a8c7b3d793e`、peeled commit `758ef40f50c1a458425c7cfbf1eb12cbc07af0b0`、tree `0f7a27df60e01dccf918f3203235266a0d6e3258`。CLI 报告 `whale 0.149.0`。

U19a–U19d 均已验证。D1 保持 DeepSeek-only 公共模型目录、`deepseek-v4-flash` 默认和 `deepseek-v4-pro` 可见；D2 保持 TaskSpace 单一 relational state authority。没有为了让上游专属测试变绿而启用 OpenAI remote catalog、ChatGPT 登录产品面、远程插件分享或 Bedrock 模型目录。

## 合入内容

- 应用官方 0.147→0.149 vendor 差分，保留最小 Whale identity、home、auth、DeepSeek Responses/cache 与 TaskSpace overlay。
- 刷新 app-server schema、overlay inventory 和 replay ledger。当前官方原始导入到 0.149 的 delta inventory 为 5,155 路径；Whale 相对官方 0.149 的 overlay/replay 为 292 路径。
- 解决上游新 migration 编号与 Whale 历史 47/48 的冲突：TaskSpace migration 移至 51/52；仅当两条旧 checksum 都精确匹配时执行历史桥接，未知或混合历史保持 fail-closed。
- 将 final-wire 测试迁移到 0.149 `TurnInputRequest`，并用 production registry、typed `thread/fork`、真实 SQLite、shutdown/reopen/request 验证 TaskSpace 绑定恢复。

## 验证矩阵

| 范围 | 结果 |
| --- | --- |
| 通用编译 | `codex-core`、`codex-tui`、`codex-app-server-protocol`、`codex-model-provider-info` cargo check 通过 |
| DeepSeek | provider/model/core 8/8；compact 18/18；usage 18/18 |
| 免费缓存合同 | Python 232/232；release-head index gate 通过 |
| TaskSpace | state 203/203；tools 106/106；core 75/75；TUI 4/4；fork→reload→request production composition 1/1 |
| 同步元数据 | metadata validator、delta/inventory/replay generator check、同步工具测试 49/49 通过 |
| CLI/schema | `whale 0.149.0`；app-server schema generation 通过 |

release-head 免费门禁证据见 `benchmarks/cache-regression/gate-reports/2026-08-21-u19-codex-0149-release-head.json`。

## 真实缓存验收

用户批准的整个 rebase/Codex 合入总预算为 3 CNY。本次仅执行一次最小资格运行，未重试：

| Record | 模型 | 规模 | Provider 请求 | Token（input/cached/uncached/output） | 费用 | 耗时 | 结果 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `WAR-20260821-080834-CACHE-REGRESSION-8DC2D672` | `deepseek-v4-flash` | Standard + map-request，各 1 repeat，共 2 sample run | 14 | 168,253 / 143,104 / 25,149 / 3,654 | 0.03531908 CNY | 54.291 s | 两臂完成且业务成功，baseline accepted |

账本、proposal、authorization、result 与 acceptance 均已提交；稳定 surface 为 `bc5d56702d1ac61498d85d822b0a68ddc0ae58ab533bda1818db2e10fed3bf14`。

## 全量回归边界

全量套件不被误报为全绿。官方 pristine 0.149 候选自身为 core 3573/3580、app-server 1234/1235、TUI 3720/3747；集成树隔离运行分别为 core 2368/2388（20 fail，1 skip）、app-server 911/957（46 fail，1 skip）、TUI 3673/3741（68 fail，2 skip）。

集成树失败主要聚类于 Guardian/auto-review 的 OpenAI 模型假设、OpenAI remote plugin/sharing/recommended model list、Bedrock、ChatGPT account/rate-limit/status、Whale identity 与 DeepSeek 默认导致的快照差异，以及宿主时序。发现的陈旧 `code-mode-host` 二进制问题已用 0.149 构建定向复验通过；command cwd permission 的定向重跑也通过。官方候选自身的 `external_agent_config_secondary_source_imports_session_and_plugin_end_to_end` 失败仍保留为上游已知签名。

这些非绿项不属于当前已确认的 Whale Linux + DeepSeek + TaskSpace 发布矩阵，故不阻塞 U19，但也不视为修复或通过。延期范围为 OpenAI/ChatGPT 登录及远程插件产品面、Bedrock 专属目录、Windows 验证和非产品语义的 TUI 快照对齐；后续若要启用这些产品面，必须建立独立工作单元重新验证。

官方候选隔离日志位于 `docs/v0.0.5/codex-upstream-sync/evidence/rust-v0.149.0/attempt-1-isolated-qualification/`。

## 提交边界

- `899136014`：固定并资格验证官方 0.149 候选。
- `698b5c048`、`685027171`：加固 final-wire 资格测试并退役已删除的 admission suite。
- `0044ffee5`：应用官方 vendor 差分与 Whale overlay。
- `8637fed5d`、`7282b520d`：刷新 provenance 并记录缓存门禁发现。
- `130a562ce`、`44c8f933a`：登记、执行、结算并接受 0.149 真实缓存基线。

本报告与唯一执行计划共同关闭 U19；后续上游追赶从官方 0.149 固定对象继续增量执行。
