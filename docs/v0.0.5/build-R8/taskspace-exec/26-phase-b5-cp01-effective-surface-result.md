# Phase B5 CP-01 Effective Tool Surface 结果

- Date: 2026-08-10
- Status: Verified offline
- Scope: 当前 Whale `ToolRegistryPlan -> ToolRouter -> TaskSpaceExecCatalog` 生产链
- Provider run: 0

## 1. 结论

当前不存在一个可直接等同于 TaskSpace 内层有效能力的现成 `Vec<ToolSpec>`：

- `ToolRouter.specs` 是可注册集合，但会包含 Code Mode 专用入口、被 Code Mode 改写的描述和 deferred dynamic 完整 schema；
- `ToolRouter.model_visible_specs` 是当前 Standard 顶层 Provider 集合，但 Code Mode Only 下只剩 `exec/wait`，不能作为 TaskSpace 内层能力事实；
- `TaskSpaceExecCatalog` 当前从 `specs` 构建，因此默认 DeepSeek 基本能力可用，但 deferred 与 Code Mode 组合语义失真。

CP-04 应从现有 Registry plan 中抽取一个中性的 effective capability view，同时驱动 TaskSpace declaration、decoder、identity 和
dispatch lookup；它不是第二个 Registry，也不能从两份列表猜测或做字符串清洗。

## 2. 目标 DeepSeek 默认矩阵

测试使用 bundled `deepseek-v4-flash`、默认 Features、Tool Search、一个 deferred MCP 和一个 deferred dynamic Tool。

| Capability | Registry / Runtime | Standard Provider-visible | TaskSpace top-level | TaskSpace Exec inner | Finding |
|---|---|---|---|---|---|
| `exec_command` / `write_stdin` | yes | yes | no | yes | `UnifiedExec` feature 覆盖模型静态 `shell_command`，这是实际目标 shell surface |
| Provider `web_search` | yes | yes | yes | hosted binding only | DeepSeek hosted web search 已进入生产配置，不能按旧 client web search 假设设计 |
| `tool_search` | yes | yes | no | yes | 搜索动作本身可通过 Exec 调用 |
| deferred dynamic | handler + namespaced spec | `defer_loading=true` | no | full schema copied | Outer Function 丢失 Standard 的延迟暴露收益，CP-05 必须收敛 |
| deferred MCP | handler only | 由 Tool Search output 渐进暴露 | no | absent | 当前静态 Catalog 无法解码搜索后能力；CP-05 必须沿现有 Tool Search 事实继续闭环 |
| `LocalShell` | no | no | no | no | 当前目标配置不适用，不实施 CP-G12 生产支持 |
| Code Mode `exec/wait` | no | no | no | no | 默认 DeepSeek TaskSpace 不受 Code Mode 影响 |

## 3. Code Mode Only 组合矩阵

| Surface | Current fact |
|---|---|
| Registry specs | 同时包含 `exec/wait` 与被 JS Exec declaration 增补过描述的普通 Tool |
| Standard model-visible | 只有 `exec/wait` |
| TaskSpace Catalog source | 仍读取全部 Registry specs，排除 `exec/wait` 后复制被增补的普通 Tool 描述 |
| TaskSpace top-level Hosted | 从 Standard model-visible 过滤，Provider `web_search` 同时消失 |

因此 CP-G10 已坐实：TaskSpace 不能消费 surface-specific augmentation 后的 Tool description。CP-04/05 需要复用 Registry plan 的
原生定义边界，不允许手写删除 `exec tool declaration` 文本。

## 4. Deferred 停点

CP-01 只证明当前集合，不决定 deferred 产品策略。CP-05 必须先回答：Standard Tool Search output 中已加载的原生 ToolSpec 能否在
下一次 TaskSpace request 机械进入同一 request-local Catalog。若可行，复用该生命周期；若不可行，必须带调用链证据回到用户决策，
不能默认全量展开、隐藏能力或建立第二 Registry。

## 5. 验证

| Verification | Result |
|---|---|
| `cargo test -p codex-core taskspace_cp01 --lib` | 2 passed |
| `cargo test -p codex-core shell_zsh_fork_prefers_shell_command_over_unified_exec --lib` | 1 passed |
| deferred dynamic Tool Registry test | 1 passed |
| deferred MCP source Tool Registry test | 1 passed |
| Code Mode Only Standard exposure test | 1 passed |
| TaskSpace Catalog suite | 10 passed |

既有编译 warning 未由本单元引入。本单元没有修改生产行为、Provider payload 或缓存敏感面，也没有运行 Whale Agent。
