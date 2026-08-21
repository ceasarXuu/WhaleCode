# Codex upstream 分发配置隔离

`third_party/codex-cli` 是 Whale 使用的 Codex upstream substrate，但其中以下内容不是
Whale 的发布授权源：

- `.github/workflows/`、`.github/actions/` 与 `.github/scripts/` 中的 upstream 发布流水线；
- `sdk/` 和 `codex-rs/responses-api-proxy/npm/` 中的 OpenAI SDK/package manifest；
- Codex upstream 的 R2、WinGet、Homebrew、Desktop、代码签名和网站部署配置。

这些文件可以为了 upstream 可追溯性保留原样，但根目录 `.github/workflows/` 不得调用、
复制或启用它们。Whale 当前唯一登记的公开包渠道是 npm
`@ceasarxuu/whalecode`；尚未建立的 standalone、Homebrew、Desktop、R2、WinGet、SDK
和网站渠道必须保持禁用。

任何新增 Whale 分发入口都必须先使用 Whale 自有 owner、仓库、包名、制品名和凭据，
再通过 `python3 scripts/release/check_distribution_identity.py`。
