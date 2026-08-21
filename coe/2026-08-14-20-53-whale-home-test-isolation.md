# Problem P-001: Whale home 测试子进程失去隔离
- Status: fixed
- Created: 2026-08-14 20:53
- Updated: 2026-08-14 21:08
- Objective: 恢复 0.147 全量测试中子进程对临时 Whale home 的隔离，不放宽产品的 Whale/Codex 状态目录边界。
- Symptoms:
  - `cargo test --workspace` 中 app-server auth/config/plugin 等大量集成测试失败。
  - 串行 `get_account_with_api_key` 的 initialize 响应报告 `/home/zhangxu/.whale`，而夹具已创建并写入另一临时目录。
- Expected behavior:
  - 测试启动的 Whale 子进程只读取该用例的临时 home。
- Actual behavior:
  - 测试夹具向子进程设置 `CODEX_HOME`；Whale 产品只读取 `WHALE_HOME`，因此回落到真实用户默认目录。
- Impact:
  - Linux 全量无模型回归产生大面积同源假失败，并存在测试读取本机 Whale 配置的隔离风险。
- Reproduction:
  - `cargo test -p codex-app-server --test all suite::v2::account::get_account_with_api_key -- --exact --nocapture`
- Environment:
  - Ubuntu 24.04，branch `whalecode-codex`，commit `7c7944d70`，Codex substrate 0.147。
- Known facts:
  - 产品 `codex-utils-home-dir` 只读取 `WHALE_HOME` 并拒绝与 `CODEX_HOME` 指向同一路径。
  - app-server 测试进程 builder 只设置 `CODEX_HOME`。
  - 失败进程明确报告默认 `/home/zhangxu/.whale`。
- Ruled out:
  - DeepSeek provider 与 account RPC 序列化不是首因：失败发生在测试临时 home 未被进程采用之后。
- Fix criteria:
  - 原始串行失败通过；app-server 全包不再出现同源 home 泄漏；全 workspace 继续执行并对剩余失败单独归因。
- Current conclusion: H-001 已修复；测试子进程统一注入 `WHALE_HOME`，app-server 聚焦复现和同源失败均已恢复。
- Related hypotheses:
  - H-001
- Resolution basis:
  - 原始串行复现通过，initialize 返回夹具临时 home。
  - app-server 全套回归从首次运行的大面积 home 相关失败收敛到 818/858 通过；剩余失败均可独立归因到 DeepSeek 模型目录、OpenAI 远程插件市场或其他产品差异，未再观察到真实 `~/.whale` 泄漏。
  - Linux 测试子进程中旧 `.env("CODEX_HOME", ...)` / `set_var("CODEX_HOME", ...)` 注入已清理；保留项仅用于显式移除、隔离边界测试或已延期的 Windows 专项测试。
- Close reason:
  - 测试基础设施已与产品 `WHALE_HOME` 合同一致，P-001 修复标准满足。

## Hypothesis H-001: 测试仍注入旧 CODEX_HOME
- Status: confirmed
- Parent: P-001
- Claim: U3 将产品状态目录入口切换为 `WHALE_HOME` 后，子进程测试夹具仍只注入 `CODEX_HOME`，使被测进程回落到真实 `~/.whale` 并读取错误状态。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 失败覆盖多个依赖 home 的功能，但共享同一 app-server process builder。
- Falsifiable predictions:
  - If true: initialize 返回真实默认 home，builder 代码只设置 `CODEX_HOME`，resolver 只读取 `WHALE_HOME`。
  - If false: initialize 应返回夹具临时目录，或 resolver 仍应读取 `CODEX_HOME`。
- Diagnostic evidence plan:
  - Prediction or clause under test: 比较失败进程返回的 home、process builder 注入变量与产品 resolver 读取变量。
  - Signal: initialize `codexHome`、`.env(...)` 调用和 resolver 源码。
  - Capture method: 串行单测 `--nocapture` 与只读源码检查。
  - Event name or marker:
    - initialize.codexHome
  - Correlation keys:
    - test name `suite::v2::account::get_account_with_api_key`
  - Differentiates from:
    - account RPC 或 DeepSeek provider 行为错误。
  - Supports if:
    - 三个信号分别为真实 `~/.whale`、`CODEX_HOME`、`WHALE_HOME`。
  - Refutes if:
    - 子进程实际已获得临时 `WHALE_HOME` 或仍从 `CODEX_HOME` 解析。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 串行失败进程读取真实默认 home
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: focused app-server integration test with `--nocapture`
- Prediction or plan link:
  - H-001 If true
- Matched signal:
  - initialize.codexHome
- Correlation keys:
  - `suite::v2::account::get_account_with_api_key`
- Raw content:
  ```text
  "codexHome": "/home/zhangxu/.whale"
  account/read => {"account": null, "requiresOpenaiAuth": false}
  expected account: Some(ApiKey)
  ```
- Interpretation: 被测子进程没有采用夹具已写入 auth 的临时目录，足以解释 account 断言失败及其他 home 依赖失败。
- Time: 2026-08-14 20:53

## Evidence E-002: 注入变量与解析变量不一致
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `app-server/tests/common/test_app_server.rs:239` 与 `utils/home-dir/src/lib.rs:12`
- Prediction or plan link:
  - H-001 diagnostic evidence plan
- Matched signal:
  - builder=`CODEX_HOME`; resolver=`WHALE_HOME`
- Correlation keys:
  - none
- Raw content:
  ```text
  cmd.env("CODEX_HOME", codex_home);
  std::env::var("WHALE_HOME")
  ```
- Interpretation: 两端变量名不一致直接产生默认目录回退；resolver 还明确拒绝 Whale/Codex 指向同一目录，因此修复应迁移测试变量而非同时设置两者。
- Time: 2026-08-14 20:53

## Evidence E-003: 原始复现与 app-server 回归不再泄漏真实 home
- Related hypotheses:
  - H-001
- Direction: supports
- Type: fix-validation
- Source: focused app-server integration test and full `codex-app-server --test all`
- Prediction or plan link:
  - P-001 Fix criteria
- Matched signal:
  - initialize.codexHome
  - app-server pass/fail classification
- Correlation keys:
  - `suite::v2::account::get_account_with_api_key`
- Raw content:
  ```text
  initialize.codexHome = /tmp/.tmp9fvCkn
  focused result: 1 passed; 0 failed
  full result: 818 passed; 39 failed; 1 ignored
  ```
- Interpretation: 原始失败已恢复，完整套件的剩余失败不再出现真实 `~/.whale` 或找不到夹具 auth/config 的同源症状；后续按产品差异分别治理。
- Time: 2026-08-14 21:08
