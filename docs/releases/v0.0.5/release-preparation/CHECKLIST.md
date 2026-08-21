# WhaleCode v0.0.5 发布准备清单

## 自动门禁

- [ ] `python3 scripts/release/check_release_identity.py`
- [ ] `python3 -m unittest discover -s scripts/release/tests -p 'test_*.py'`
- [ ] `cargo metadata --locked --no-deps`
- [ ] `cargo build -p codex-cli --bin whale --locked`
- [ ] workspace 安装后的 `whale --version` 输出 `whale 0.0.5`
- [ ] cache-sensitive index gate 通过（若版本改动被门禁判为敏感）

## 人工核对

- [ ] 产品 tag 是 `v0.0.5`，不是 `rust-v0.149.0`
- [ ] 发布标题和说明使用 WhaleCode `v0.0.5`
- [ ] Codex `0.149.0` 只出现在 upstream substrate/provenance 上下文
- [ ] release notes 不宣称 upstream 全量测试全绿
- [ ] 不包含 API key、凭据、用户目录或未脱敏日志
- [ ] 发布渠道、凭据、回滚方式和实际发布授权已经单独确认

## 当前明确禁止

- [ ] 未确认渠道前不得直接运行 vendor 内 `rust-release.yml`
- [ ] 未明确授权前不得创建/推送 tag 或发布 GitHub/npm/WinGet/R2 资产
- [ ] 不得用全局 `whale 0.1.0` 代替 workspace 候选二进制
