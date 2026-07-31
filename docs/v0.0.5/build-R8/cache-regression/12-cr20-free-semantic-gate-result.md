# CR20 免费语义门禁结果

- Date: 2026-07-31
- Status: completed
- Commits: `71c8f0cf0`、`3b7d7b4fa`
- API usage: 无真实 Whale Agent run，无 DeepSeek API 请求

## 1. 产品结果

缓存门禁不再把源码文件 SHA 变化直接解释为缓存 payload 变化。当前控制流为：

1. 风险规则识别可能影响上下文、Tool、provider、模型、能力注入或压缩结构的生产源码；
2. `index` 模式确认相关 staged 内容与 worktree 完全一致；
3. 自动运行使用生产 Session 和 Responses serializer 的免费 final-wire 合同；
4. 合同全部通过时允许语义等价改动提交，不申请真实 API 预算；
5. 合同失败时阻断并输出失败命令及日志尾部；
6. 产品源码与 final-wire 快照禁止同提交更新，快照单独更新也在 CR22 晋升机制完成前保持阻断。

旧 `surface_sha256` 继续写入诊断结果，供历史结果复算，但不再作为配置了 `free_validation` 后的语义通过条件。
release 仍独立要求 `live_verified`，当前 `live_regression_failed` 状态不会被免费测试覆盖或改写。

## 2. 免费验证矩阵

| ID | 覆盖对象 | 最终结果 |
|---|---|---|
| `final_wire_matrix` | Standard、三种 TaskSpace、权限、Skill、Apps、Plugin、MCP、压缩 | pass |
| `provider_contract` | DeepSeek provider 与 Responses 路由 | pass |
| `model_catalog_contract` | Flash 默认与 Pro 隐藏边界 | pass |
| `default_route_contract` | 默认 DeepSeek/Flash/Responses 配置 | pass |
| `usage_decoder_contract` | Responses/Chat usage 解码合同 | pass |
| `final_wire_comparison_contract` | Responses `instructions/input/tools/tool_choice/model` 精确比较 | pass |

生产矩阵最终复验总耗时约 `29.4s`，其中 provider 文件触发增量重编译；同一矩阵在无需重编译时约 `4.5s`。
non-agent release gate 超时由 `30s` 调整为 `900s`，避免冷构建被错误判为语义失败。

## 3. 反例与稳定性

- 临时 Git fixture 覆盖普通文件不触发、敏感等价变更通过、免费 runner 失败阻断；
- staged/worktree 不一致时不运行并拒绝证明错误源码；
- 产品与快照同提交、仅快照提交均拒绝；
- runner 超时、命令不存在和非零退出均形成结构化失败；
- 已知生产入口合同检查可发现指向不存在目录或遗漏 Responses 公共请求结构的 glob；
- 6 个 runner 测试、21 个 gate 测试、3 个 surface 合同测试及 34 个既有缓存控制面测试通过。

真实 index 探针在 `model-provider-info/src/lib.rs` 加入临时无语义注释后，自动运行全部 6 个免费合同并通过；探针
随后用 `apply_patch` 移除并恢复到 HEAD，没有进入提交。

## 4. 后续边界

CR20 只回答“最终请求语义是否变化”，不自行判断有意变化是否值得接受，也不执行真实回归。CR21 先把失败形成
结构化 impact；只有人工确认的有意变化才进入 coverage 判断和最小预算提案。CR22 负责用真实证据独立晋升对应
快照和 scoped baseline。
