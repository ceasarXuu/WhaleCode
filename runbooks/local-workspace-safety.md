# 本地工作空间安全运行手册

本手册适用于Linux上的WhaleCode clone和Git worktree。目标是让每个workspace使用独立的runtime home、SQLite、sessions、logs、临时目录和开发binary，同时保留系统toolchain与安全缓存复用。正式release只使用全局`whale`和`~/.whale`；开发版只通过全局`whale-dev`进入当前worktree的隔离slot。

## 开工流程

在新clone/worktree、切换branch或门禁失败后，从仓库根目录执行：

```bash
PLAN_JSON="$(python3 scripts/workspace-safety/workspace_context.py bootstrap plan --json)"
printf '%s\n' "$PLAN_JSON"
PLAN_FINGERPRINT="$(printf '%s' "$PLAN_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["fingerprint"])')"
python3 scripts/workspace-safety/workspace_context.py bootstrap apply --expect "$PLAN_FINGERPRINT"
bash -lc 'cd third_party/codex-cli/codex-rs && cargo build -p codex-cli --bin whale'
bash scripts/install-whale-local.sh --scope workspace
python3 scripts/workspace-safety/workspace_context.py doctor --require-binary
whale-dev --version
```

只有检查过计划内容后才能执行apply。fingerprint必须来自紧邻的同一次plan；branch、root或资源状态变化后重新生成。
安装脚本只复制已有构建产物，不会隐式编译源码；源码变更后的本机验收必须先重建`whale`，再执行workspace安装，避免用旧二进制得到假阴性或假通过。

日常开工检查：

```bash
python3 scripts/workspace-safety/workspace_context.py require-ready
```

安装完成后，`~/.local/bin/whale-dev`是无状态dispatcher。它根据当前目录所在worktree选择对应slot；在worktree及其子目录直接运行：

```bash
whale-dev --version
whale-dev --yolo
```

`workspace_context.py exec -- whale ...`保留为自动化内部入口；日常人工开发统一使用`whale-dev`。`whale-dev`会重新校验marker、branch、资源权限、binary hash和attestation，并精确启动slot内binary。仓库外、未bootstrap、branch stale或binary无效时非零退出，不搜索PATH上的`whale`。

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
- 全局`whale`只用于release；全局`whale-dev`只分发到当前worktree的开发slot。
- workspace安装只能使用`--scope workspace`；用户级安装必须显式使用`--scope user`。
- 不把PATH上的全局`whale`作为workspace入口fallback。
- 不把最近安装的开发binary作为全局活动版本；`whale-dev`必须按cwd解析worktree。
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
