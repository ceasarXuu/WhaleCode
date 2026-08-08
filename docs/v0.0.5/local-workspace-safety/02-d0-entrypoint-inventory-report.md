# D0 工作区入口与构建根盘点报告

- Artifact status：verified
- 执行日期：2026-08-08
- 决策边界：新 workspace/Coding Agent 进入仓库后，到获得可信的入口、构建根和共享资源风险清单
- 实施范围：只读静态 inventory、JSON Schema、临时 Git clone/worktree fixtures
- 未执行：Cargo/Bazel build、Docker、Whale、模型请求、真实 benchmark、legacy 数据迁移

## 1. 结论

D0 已建立可重复的只读盘点工具：

```bash
python3 scripts/workspace-safety/workspace_inventory.py --repo-root .
```

工具将 JSON 输出到 stdout，不写 repo、Git、XDG state 或 legacy `~/.whale`。需要保存原始证据时由调用者显式重定向到系统临时目录；原始 JSON 含本机 canonical path，因此不提交仓库。

当前 workspace 的脱敏结果：

| 项目 | 数量/结论 |
| --- | --- |
| Build roots | 4：1 archived Cargo、2 vendored Cargo、1 vendored Bazel |
| 敏感可执行入口 | 23 |
| Cross-platform Python | 5 |
| Linux/POSIX shell | 2 |
| PowerShell | 16，归入 W14 deferred |
| Legacy whale binary 默认 | 7处引用；可执行宿主风险集中于 cache regression Python/PowerShell runner |
| 隐私检查 | 通过；remote credentials、query、匹配原文和环境变量值未输出 |

证据质量：Git/build manifest 与静态路径事实为高可信；“可能发起模型请求”是保守静态分类，只用于阻断自动执行和安排后续检查，不代表已运行验证。

## 2. 构建根

| Scope | Kind | Manifest | D0 处理 |
| --- | --- | --- | --- |
| archived | Cargo | `archive/deprecated/2026-04-27-rust-demo/Cargo.toml` | 只报告，不作为活动验证根 |
| vendored | Bazel | `third_party/codex-cli/MODULE.bazel` | 仅在 Bazel 可用时检查；不从仓库根无条件运行 |
| vendored | Cargo | `third_party/codex-cli/codex-rs/Cargo.toml` | 活动 Rust workspace，后续命令必须传 manifest 或切换 cwd |
| vendored | Cargo | `third_party/codex-cli/tools/argument-comment-lint/Cargo.toml` | 独立工具 workspace，不替代主 Rust workspace |

## 3. Linux 与跨平台入口归类

| 入口 | 角色 | Workspace 后续动作 |
| --- | --- | --- |
| `scripts/install-whale-local.sh` | Linux宿主安装入口 | W7：安装前require-ready，workspace/user scope分离 |
| `scripts/cache-regression/run_cache_hit_regression.py` | Linux可用、真实模型/cache宿主入口；当前默认legacy binary | W8：移除global默认，零请求前校验workspace slot与attestation |
| `scripts/taskspace-benchmark/run-active-prefix-matrix.py` | Docker矩阵宿主入口，可创建目录并运行最多3个默认arm | W9a：任何目录/Docker/请求前校验workspace与既有预算合同 |
| `scripts/taskspace-benchmark/r7_a2_b0_provider_wire_probe.py` | 直接HTTP provider探针，repeat×scenario可产生多请求 | W9b：任何输出/请求前校验workspace与既有预算合同 |
| `scripts/taskspace-benchmark/docker/provider_boundary_proxy.py` | 容器内部provider边界 | 不在容器内bootstrap；由宿主launcher负责门禁 |
| `scripts/taskspace-benchmark/docker/taskspace-container-entrypoint.sh` | 容器内部secret加载与exec入口 | 不在容器内bootstrap；由宿主launcher负责门禁 |
| `scripts/codex-upstream/generate_replay_ledger.py` | 本地元数据生成器，无模型与binary解析 | 无需workspace runtime门禁；保留静态引用观察 |

PowerShell 入口共16个，包含安装、cache、TaskSpace benchmark与探针。按既有范围统一留给W14，不在Linux首版中修改或声称验证。

## 4. Findings 与优先级

| ID | Priority | Domain | Finding | Evidence | 后续单元 |
| --- | --- | --- | --- | --- | --- |
| D0-F1 | P0 | evidence trust | cache runner默认`~/.whale/bin/whale`，可让一个workspace运行另一workspace binary | inventory的`legacy-whale-binary`与源码默认参数 | W8 |
| D0-F2 | P0 | environment/model gate | 两个Python宿主模型入口可在未确认workspace身份时直接写证据或发请求 | 静态调用链显示目录创建、Docker/HTTP请求位于main路径 | W9a、W9b |
| D0-F3 | P1 | build feedback | 仓库根不是Cargo/Bazel执行根，使用泛化根命令只产生失败而不增加证据 | manifest inventory | W10/验证矩阵已校正 |
| D0-F4 | P4 | platform scope | 16个PowerShell入口与Linux首版混合会扩大验证面 | platform分类 | W14 deferred |

## 5. Next Best Intervention

| Field | Decision |
| --- | --- |
| Finding | 还没有可供所有后续门禁复用的只读workspace context与plan fingerprint |
| Priority class | P0：没有可信身份，后续installer/runner证据仍可能归属错误 |
| Why first | W2-W9均依赖同一个canonical root、common-dir、branch与资源路径事实 |
| Expected critical-path benefit | 不以速度为目标；把后续错误环境失败提前到任何写入或请求之前 |
| Scope | `workspace_context.py`的纯resolver/plan及fixtures |
| Effort | 中；标准库实现，无新依赖和持久状态 |
| Correctness risk | 中；路径规范化或Git worktree判断错误会产生误阻断 |
| Evidence preserved or moved | 保留Git与manifest原始事实；不删除任何既有测试或门禁 |
| Rollback | 单提交revert；W1保持零写入 |
| Validation | clone、linked worktree、两套common-dir、detached HEAD、同名目录、remote脱敏和plan零写入 |
| Follow-up gate | W1全部fixture通过且当前两workspace输出可解释后，才进入W2 |

## 6. 验证证据

```bash
python3 -m py_compile \
  scripts/workspace-safety/workspace_inventory.py \
  scripts/workspace-safety/tests/test_workspace_inventory.py

python3 -m unittest discover \
  -s scripts/workspace-safety/tests \
  -p 'test_*.py' -v

python3 scripts/workspace-safety/workspace_inventory.py --repo-root . \
  >"$(mktemp -d)/workspace-inventory.json"
```

结果：4项fixture通过，覆盖无Cargo/嵌套Cargo、可选Bazel、linked worktree、两套Git dir语义、remote credential/query脱敏、Schema顶层合同与collect阶段零文件变化。当前workspace盘点完成，隐私字符串检查通过。

## 7. Guardrail 与停止决定

- Guardrail：测试/库不进入`entrypoints`，但其共享资源引用继续计入证据；inventory自身不扫描自身规则文本。
- Guardrail：输出不包含匹配原文、环境变量值或凭据化remote；本机原始JSON不提交。
- Stop decision：`retain and guard`。D0目标已达到，不继续扩张到W1实现。
- Residual risk：静态启发式可能随新脚本语法漂移；W12后续将把规则变成持续回归门禁。
