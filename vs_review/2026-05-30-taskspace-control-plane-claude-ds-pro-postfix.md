以下是完整审查报告：

---

# Claude Opus Final Review: TaskSpace Control Plane

**整体结论：无阻塞性问题 (Blocking Findings: none)。** 所有 5 个 E2E 通过，143 个单元测试 0 失败。但存在 4 个中等风险需修复。

---

## Q1: spawn_agent 的 node contract 是否出现在 FinalSynthesis

**存在结构性缺口。**

- `contracts.rs:40-45` — `FINAL_ACTIONS` 不包含 `Spawn`，设计上正确
- `parallel.rs:331-341` — `should_attribute_taskspace_tool` 明确排除了 `spawn_agent`，因此 spawn_agent 从不走 `prepare_main_tool_call` → `contract_for(node.kind).allows()` 路径
- `runtime.rs:1185-1199` — `prepare_child_spawn` 只检查嵌套 spawn 防护，不检查主 agent 当前 node contract

**这意味着：** 如果 agent 在 FinalSynthesis node 上调用 `spawn_agent`，设计意图是阻断，但代码没有执行这个检查。设计文档明确写了 "assignment gate 仍必须读取当前主 node contract"（design.md:296）。

实际上 FinalSynthesis 是终端节点，agent 不太可能在此 spawn，但结构性缺口仍需修补。

---

## Q2: Build/Test 分类是否正确区分

**分类逻辑正确，但存在边缘情况。**

`classify_shell_text` 优先级: **Edit > Test > Build > Search > Read** (`parallel.rs:430-581`)

| 命令 | 分类 | 是否正确 |
|---|---|---|
| `cargo build` | Build | ✅ |
| `pytest -q` | Test | ✅ |
| `cargo build && cargo test` | Test (Test 优先级高于 Build) | ✅ 设计意图 |
| `pytest -q > pytest.log` | **Edit** (输出重定向触发 Edit) | ⚠️ 会导致 Test node 无法执行 |
| `git stash; pytest -q` | **Edit** (git stash 优先) | ⚠️ 同上有风险 |

**关键矛盾：** `TEST_ACTIONS` (`contracts.rs:31-39`) 不包含 `Edit`。所以 `pytest -q > pytest.log` 在 RegressionTest node 中会被阻断。目前 E2E 未触发此问题，因为 agent 通常使用 `2>&1` 而非文件重定向。测试文件 (`parallel.rs:712-718`) 对此行为有明确测试用例。

我的判断：这是 **Low-Medium 风险**，不需要立即修改分类逻辑，但应在 E2E 中增加监控，看实际运行中是否误阻断。

---

## Q3: multi-agent E2E 是否输出 observability_json/md/html

**Multi-agent E2E 有全部三种输出。** (`report.md:28-30`)

```
observability_json: .../observability/action-map-observability.json
observability_md:   .../observability/action-map-observability.md
observability_html: .../observability/action-map-observability.html
```

但 **Growth health E2E 报告不列出 observability 路径** — 脚本实际生成了这些文件 (`growth-health-e2e.ps1:243-250`)，但报告生成循环 (`growth-health-e2e.ps1:398-438`) 遗漏了 artifact 列表。这是报告格式不一致的 bug。

---

## Q4: growth E2E 的 gate 验证是否基于 kind/action_class

**主要依赖 title 正则，缺少 kind/action_class 级别的交叉验证。**

Growth E2E (`growth-health-e2e.ps1:268-272`):
```powershell
$hasParserNode = $titleText -match "parser|parse|order line|sku"   # title-based
$hasPricingNode = $titleText -match "pricing|discount|invoice|shipping"  # title-based
```

相比之下，Natural user E2E (`real-user-e2e.ps1:244-258`) 有:
```powershell
$hasInspectKind = $kindText -match "inspect_code_context"         # kind-based
$hasImplementationKind = $kindText -match "implement_solution"     # kind-based
$hasTestKind = $kindText -match "smoke_test|regression_test"       # kind-based
$implementationKindHasEdit = ...   # cross-node check
$testKindHasTest = ...             # cross-node check
$editResultsOutsideImplementation = ...
$testResultsOutsideDiagnosticOrTestNode = ...
$unknownActionResults = ...
```

Growth E2E 缺失所有这些 kind/action_class 交叉检查。设计文档明确要求 "E2E 从 title 正则逐步迁移到 node.kind 校验" (`design.md:101`)。

---

## Q5: whale.exe 路径是否应该统一

**是。** 5 个报告中：

| 报告 | whale_bin |
|---|---|
| 024310-236 (multi-agent) | `D:\BuildCache\...\debug\whale.exe` |
| 024612-200 (growth) | `D:\BuildCache\...\debug\whale.exe` |
| 024837-705 (natural #1) | `D:\BuildCache\...\debug\whale.exe` |
| 024948-734 (regression) | N/A (cargo test) |
| 025355-824 (natural #2) | `C:\Users\77585\.whale\bin\whale.exe` ✅ |

所有脚本的默认参数已经是 `$env:USERPROFILE\.whale\bin\whale.exe`。前 3 个用了显式 `-WhaleBin` 覆盖，因为 install 被文件锁阻塞（见 `vs_review/2026-05-29-...review.md:55`）。两个路径的 SHA256 相同 (`0C4BB61...`)，是同一个二进制。

**建议：** 构建后增加 install 步骤解锁文件，让默认路径始终可用。

---

## 需要修复的四项

| # | 修复 | 优先级 | 涉及文件 |
|---|---|---|---|
| 1 | spawn_agent 主 agent 路径增加 node contract 检查 | Medium | `multi_agents_v2/spawn.rs` 或 `parallel.rs` |
| 2 | Growth E2E 移植 kind/action_class 交叉检查 | Medium | `scripts/run-action-map-growth-health-e2e.ps1` |
| 3 | Growth E2E 报告增加 observability 路径 | Low | `scripts/run-action-map-growth-health-e2e.ps1` |
| 4 | 统一 whale.exe 到安装路径 | Low | 运维层面 |

---

**最终判断：** 核心 gate 逻辑 (`prepare_main_tool_call` → `contract_for` → `allows()`) 正确实现。主风险在 spawn_agent 绕过 contract 检查，以及 growth E2E 的验证深度不足。这些问题不阻塞合并，但应在下一轮 hardening 中修复。

---

## Main Agent Response

此 CLI 审查结论记录为“无阻塞性问题”，但正文中的部分风险判断已由当前代码或后续修正覆盖：

- `spawn_agent` 主路径已经在 `runtime.rs::prepare_spawn_assignment` 读取当前主 node contract，并在 `FinalSynthesis` 等不允许 `Spawn` 的节点上阻断；对应回归测试为 `final_synthesis_node_blocks_edit_test_and_spawn`。
- `Build/Test` 分类已拆分，`cargo build/check/clippy`、`npm run build/lint/typecheck`、`tsc --noEmit` 等为 `Build`，实现节点允许 `Build` 但仍禁止 `Test`。
- Growth E2E 已补充 `observability_json/md/html` artifact 路径、`inspect/implementation/test` kind 硬指标、`unknown_action_results` 硬失败条件。
- 安装态验证已完成，`target/real-user-e2e/action-map-natural-user-order-pipeline/20260530-025355-824/artifacts/report.md` 使用 `C:\Users\77585\.whale\bin\whale.exe`，SHA256 为 `0C4BB6139083C62398754D759C0C662985C2DC47807A0615455A9F6237A27F82`。
- 最新 growth health E2E 为 `target/real-user-e2e/action-map-growth-health-order-pipeline/20260530-030549-184/artifacts/report.md`，结果 PASS。

