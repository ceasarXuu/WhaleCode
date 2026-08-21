# 本地工作空间安全运行手册

本手册适用于Linux上的WhaleCode clone和Git worktree。目标是让每个workspace使用独立的runtime home、SQLite、sessions、logs、临时目录和开发binary，同时保留系统toolchain与安全缓存复用。

## 开工流程

在新clone/worktree、切换branch或门禁失败后，从仓库根目录执行：

```bash
PLAN_JSON="$(python3 scripts/workspace-safety/workspace_context.py bootstrap plan --json)"
printf '%s\n' "$PLAN_JSON"
PLAN_FINGERPRINT="$(printf '%s' "$PLAN_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["fingerprint"])')"
python3 scripts/workspace-safety/workspace_context.py bootstrap apply --expect "$PLAN_FINGERPRINT"
bash scripts/install-whale-local.sh --scope workspace
python3 scripts/workspace-safety/workspace_context.py doctor --require-binary
```

只有检查过计划内容后才能执行apply。fingerprint必须来自紧邻的同一次plan；branch、root或资源状态变化后重新生成。

日常开工检查：

```bash
python3 scripts/workspace-safety/workspace_context.py require-ready
```

需要使用隔离runtime环境运行命令时：

```bash
python3 scripts/workspace-safety/workspace_context.py exec -- whale --version
```

VS Code命令面板中的四个共享任务与上述接口一致：

- `Workspace: Bootstrap Plan`
- `Workspace: Bootstrap Apply`
- `Workspace: Doctor`
- `Rust: Check codex-cli`

Apply任务会提示输入Plan输出中的fingerprint，不会自动确认写入。

## 状态与恢复

| 状态 | 含义 | 恢复动作 |
| --- | --- | --- |
| `Unbootstrapped` | 当前workspace没有marker | plan并检查，然后用精确fingerprint apply |
| `Ready` | root、branch和资源绑定一致 | 可继续；需要binary的入口再运行doctor `--require-binary` |
| `Stale` | branch、Git common-dir或资源路径变化 | 重新plan/apply；不得复用旧fingerprint |
| `Conflict` | marker身份或schema与当前workspace冲突 | 停止写入，检查诊断；不得覆盖marker或fallback |
| `DoctorFailed` | 最近一次doctor未通过 | 按diagnostic code修复，再重新plan/apply/doctor |

## 安全边界

- 不迁移、不覆盖、不删除legacy `~/.whale`。
- 不复制凭据、history、sessions、plugins或skills到workspace资源。
- workspace安装只能使用`--scope workspace`；用户级安装必须显式使用`--scope user`。
- 不把PATH上的全局`whale`作为workspace入口fallback。
- 不共享`CARGO_TARGET_DIR`或Bazel output base；默认toolchain与只读registry缓存可以复用。
- 真实Whale Agent run仍按`AGENTS.md`登记账本并遵守预算授权；该要求是开发流程规范，不是产品运行时协议。

## 诊断与静态检查

以下命令不会执行模型请求；其中plan、require-ready和reference gate只读，doctor会追加审计事件：

```bash
python3 scripts/workspace-safety/workspace_context.py bootstrap plan --json
python3 scripts/workspace-safety/workspace_context.py require-ready --json
python3 scripts/workspace-safety/workspace_context.py doctor --require-binary --json
python3 scripts/workspace-safety/check_workspace_references.py
```

doctor会追加脱敏的机械审计事件，但不会记录环境变量值、凭据或命令内容。不得把含本机canonical path的原始JSON提交仓库。
