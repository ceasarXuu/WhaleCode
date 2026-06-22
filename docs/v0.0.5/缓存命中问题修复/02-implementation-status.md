# TaskSpace DeepSeek 缓存命中修复实现状态

更新时间：2026-06-23

## 当前结论

`cache_optimized_action_contract` 已作为 opt-in 原型落地，但未达到 v0.0.5 缓存命中验收标准。

当前原型已经证明：

- TaskSpace 可以在 DeepSeek ChatCompletions hot path 上关闭 provider-native tools schema。
- provider cache trace 能识别 `tool_free_action_contract` 请求形态。
- DeepSeek 在无 tools schema 时仍会输出 DSML 残留，需要 transport 层做显式归一化。
- 仅移除 tools schema 后，TaskSpace 仍会重发增长中的动态历史，缓存命中率无法稳定达到 95%。

## 已落地内容

- `WHALE_TASKSPACE_PROVIDER_TRANSPORT=cache_optimized_action_contract`
  - 仅对 DeepSeek ChatCompletions TaskSpace 请求生效。
  - provider-native tools 被置空。
  - 稳定 action contract 进入 provider instructions。
  - 动态 active node state 保持为小型 developer item。
- `TaskSpaceActionV1`
  - 将模型 JSON action 映射回本地 `shell_command`、`apply_patch`、`taskspace_control` 等执行路径。
  - 保留 node kind 策略校验。
- DeepSeek DSML 残留恢复
  - 支持前导 JSON 后追加 DSML。
  - 支持有限的纯 DSML 只读命令恢复。
  - 支持常见 unified diff patch 转换为 `apply_patch` 可接受格式。
- 验证脚本
  - `verify-deepseek-cache-fix.ps1` 支持 `-TaskspaceProviderTransport cache_optimized_action_contract`。

## 最新验证

本地验证通过：

- `cargo test -p codex-core taskspace_action_contract --lib`
- `cargo check -p codex-core`
- `cargo build -p codex-cli --bin whale --locked`

DeepSeek live 验证未通过：

- Report: `target/deepseek-cache-fix-validation/action-contract-l5/deepseek-cache-fix-verification.md`
- Artifact: `target/deepseek-cache-fix-validation/benchmark-20260623-025852/single-file-fast-fix/20260623-025853-014`
- TaskSpace hit rate: `0.259476`
- TaskSpace business_success: `false`
- model_request_count: `11`

## 根因更新

原始根因“TaskSpace 反复发送带大 tools schema 的 DeepSeek ChatCompletions 请求”已被验证为必要根因，但不是充分根因。

新增确认的剩余根因：

- TaskSpace 当前仍把增长中的动态历史作为普通 ChatCompletions prompt 重放。
- DeepSeek 官方缓存按共享前缀计入 cached tokens；动态历史越长，稳定前缀占比越低。
- action-contract 串行动作协议比 native tools 更容易消耗 TaskSpace rollout 请求预算。
- DeepSeek 即使在无 provider tools schema 时，仍可能输出 DSML 风格工具调用残留。

## 下一步

v0.0.5 缓存修复不能继续以“在现有 turn loop 里补 fallback”为主线。下一步应切到结构化 transport：

- provider prompt 由稳定前缀、短 active state、短 action result 三段组成；
- 历史不再以完整 conversation replay 进入 DeepSeek ChatCompletions；
- TaskSpace runtime 明确区分 native-tools 并行动作预算和 action-contract 串行动作预算；
- 验收以 `provider-cache-trace-summary.json` 中的 tool-free request 2+ hit rate 和 bounded dynamic suffix 为准。
