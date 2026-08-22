# 敏感信息扫描门禁

仓库使用 Gitleaks `8.30.1` 扫描完整 Git 历史。GitHub Actions 工作流只允许通过
`workflow_dispatch` 手动启动，日常验证在本地执行：

```bash
GITLEAKS_BIN=/path/to/gitleaks bash scripts/security/check-secrets.sh
```

`GITLEAKS_BIN` 可省略，此时脚本从 `PATH` 查找 `gitleaks`。版本必须严格匹配，避免规则集变化让
基线静默漂移。扫描输出使用完全脱敏模式，不应把疑似凭据原文复制到 issue、日志或文档。

## 已确认误报

`.gitleaksignore` 只记录逐条 finding fingerprint，不使用目录级排除。新增命中必须检查原始上下文，
确认它是不可用的测试夹具、示例值或摘要后，才可把该次命中的 fingerprint 加入基线。不得为了让
门禁通过而排除整个 `docs/`、`benchmarks/` 或 `third_party/`。

若命中可能是真实凭据：

1. 立即撤销或轮换凭据，不要先在公开渠道讨论原值。
2. 检查完整 Git 历史和已发布制品，而不只检查当前文件。
3. 完成清理后重新运行本地门禁。

## 压缩包

Gitleaks 的 Git 历史模式默认不展开压缩包。`scripts/security/tracked-archives.sha256` 固定当前已审计
压缩包的文件清单和 SHA-256；新增、删除或修改压缩包都会阻断门禁。

更新压缩包前，先安全展开到临时目录并单独扫描内容，确认没有密钥、token、凭据文件或不必要的
个人信息，再更新清单哈希并运行完整门禁。不要仅根据压缩包文件名或外层扫描结果判定安全。
