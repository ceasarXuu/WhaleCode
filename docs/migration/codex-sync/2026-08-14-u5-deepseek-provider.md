# U5：恢复 DeepSeek provider 与默认鉴权

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 真实模型请求：0

## 1. 实施边界

本单元只恢复 DeepSeek provider 身份、环境变量鉴权和 Whale 默认选择：

- 注册内置 provider ID `deepseek`；
- 使用官方 base URL `https://api.deepseek.com`；
- 通过 `DEEPSEEK_API_KEY` 提供 Bearer token，且不进入 OpenAI 登录流程；
- 无显式配置时默认选择 `deepseek` 与 `deepseek-v4-flash`；
- 将 `deepseek` 纳入保留 provider ID，防止配置覆盖内置安全边界。

本单元没有修改模型目录、Pro 可见性、Responses payload/SSE、用量、压缩、缓存策略或 TaskSpace。

## 2. 最小实现

生产变化仅位于三个现有 seam：

- `model-provider-info`：增加 DeepSeek 常量、构造器、身份判断和 built-in 注册；
- `config`：把 `deepseek` 加入不可覆盖的内置 provider ID；
- `core/config`：恢复 DeepSeek provider 与 Flash 模型默认值。

鉴权继续复用 0.147 已有的 `env_key -> Bearer` 通用链路，没有新增 provider factory、鉴权状态或请求分支。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | passed；仅 stable rustfmt 的已知 unstable-option warning |
| `cargo test -p codex-model-provider-info` | 23 passed |
| `cargo test -p codex-config` | 235 passed |
| `cargo test -p codex-core config::tests` | 307 passed |
| sync replay / metadata 门禁 | passed；当前 overlay 16 路径 |
| cache regression index gate | passed；指纹 `440d559abb53833117c353e4dfaff731b0beda51a2f7f62a31542064af8794ba` |

缓存门禁仍明确记录最近一次 live 回归失败；本次免费验证没有晋升 accepted live baseline。

## 4. 结论

U5 已恢复 DeepSeek 的最小接入与 Whale 默认值，未引入旧 Chat Completions 兼容层。下一单元 U7 可以基于显式 DeepSeek provider fixture 验证 0.147 原生 Responses 请求与 SSE；模型目录和 Pro 可见性继续留到 U6。
