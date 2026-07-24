# R7 评测证据快照

本目录保存里程碑机器门禁所需的最小原始 reporter 证据，避免 `target/` 清理后只剩人工汇总结论。

- `fla8-repeat3-f2baea6/`：原始 `run-manifest.json`、`summary.csv`、`aggregate.csv` 和
  `trace-analysis.json`。
- `fla9-repeat3-3e827065/`：原始 `run-manifest.json` 和 24 个
  `performance-observation.json`。该轮有一个 incomplete observation，因此 reporter 没有生成正式 aggregate。

快照只做一项机械脱敏：绝对仓库前缀替换为 `$REPO_ROOT`。对应快照哈希记录在结果 JSON 中。目录不包含
provider wire、源码正文、环境变量、凭据或密钥。

`test-r7-five-layer-contracts.ps1 -Phase FLA-8` 和 `-Phase FLA-9` 必须从这些快照重新计算关键结论，不能只验证
结果 JSON 中的固定字符串。
