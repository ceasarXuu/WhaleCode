# 本地发布验证

WhaleCode 根仓库的 GitHub Actions 默认不自动运行。所有 workflow 只允许
`workflow_dispatch` 手动触发，push、pull request 和定时任务不会消耗 Actions
运行时间。

日常发布身份与分发门禁在当前 worktree 本地执行：

```bash
bash scripts/release/run_local_preflight.sh 0.0.5
```

需要远端证据时，再按需手动运行轻量身份门禁：

```bash
gh workflow run release-identity.yml --ref main -f version=0.0.5
```

只有需要重新生成六平台原生制品时，才手动运行：

```bash
gh workflow run whale-native-artifacts.yml --ref main -f version=0.0.5
```

已有候选制品只需执行六平台安装 smoke 时，运行：

```bash
gh workflow run release-smoke.yml --ref main -f tag=v0.0.5 -f version=0.0.5
```

不要在仓库设置中禁用 Actions；完全禁用会同时阻止上述手动 workflow。静态门禁
`python3 scripts/release/check_manual_actions_only.py` 会拒绝任何新增的自动触发器。
