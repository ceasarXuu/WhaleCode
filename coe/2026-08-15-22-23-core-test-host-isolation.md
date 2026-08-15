# Problem P-001: core 完整测试受宿主临时目录和代理环境污染
- Status: resolved
- Created: 2026-08-15 22:23
- Updated: 2026-08-15 23:01
- Objective: 让当前工作区的 core 完整测试在不修改产品逻辑的前提下隔离宿主代理与共享临时目录标记。
- Symptoms:
  - 默认完整 core lib 运行额外出现代理继承、临时目录被识别为 Git 项目或 Codex 项目的失败。
- Expected behavior:
  - 测试结果只反映被测代码及显式 fixture，不依赖宿主 `/tmp/.git`、`/tmp/.codex` 或代理变量。
- Actual behavior:
  - 默认运行读取共享 `/tmp` 的祖先标记和大小写代理变量；隔离运行后额外失败消失。
- Impact:
  - 完整矩阵产生环境噪声，妨碍判断 Codex 上游融合的真实失败集合。
- Reproduction:
  - 默认运行 `just test -p codex-core --lib ...`，再与清除代理、`TMPDIR=/dev/shm`、串行运行对比。
- Environment:
  - Linux；branch `whalecode-codex`；起始提交 `36aa9da24d4c446b1a86eb4a619dec85026622e2`。
- Known facts:
  - 默认环境存在大小写 `HTTP_PROXY`/`HTTPS_PROXY`；`/tmp/.git` 与 `/tmp/.codex` 存在。
  - 隔离完整矩阵由 27 项失败降为 21 项。
- Ruled out:
  - 子 Agent DeepSeek 修复引入产品回归：其定向与邻近测试全部通过。
- Fix criteria:
  - 提供仓库内、开发者可重复执行的 core 测试隔离入口；定向代理测试及临时目录相关测试通过；不修改产品运行逻辑。
- Current conclusion: 已增加当前 vendor 的薄测试入口，并补充无污染物理临时根选择；6 个原环境失败全部通过。
- Related hypotheses:
  - H-001
- Resolution basis:
  - `scripts/codex-upstream/run_isolated_tests.py` 清理宿主 proxy，使用祖先无 `.git/.codex` 的私有临时根，并保留现有 qualification 环境合同。
  - 同步工具单测 48/48、原受污染 core 用例 6/6 通过。
- Close reason:
  - 修复标准已满足；没有修改产品逻辑或上游 vendor 测试入口。

## Hypothesis H-001: 完整测试入口未建立进程级宿主环境边界
- Status: confirmed
- Parent: P-001
- Claim: `just test` 直接继承宿主代理并使用共享系统临时根目录，使本应自包含的测试观察到外部 `.git/.codex` 标记和代理变量。
- Layer: environment
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 相同提交仅改变进程环境与临时根目录后，额外失败集合消失。
- Falsifiable predictions:
  - If true: 默认入口不清理代理、不提供隔离 TMPDIR；受影响测试均使用 `tempdir`/`TempDir` 或启动用户 shell。
  - If false: 测试入口已经隔离这些变量，或隔离后相同额外失败仍稳定出现。
- Diagnostic evidence plan:
  - Prediction or clause under test: 默认测试入口是否原样继承代理和系统 TMPDIR，受影响测试是否消费这些边界。
  - Signal: just recipe、环境快照、测试源码、默认与隔离矩阵差异。
  - Capture method: 只读检查入口与测试代码，对照已完成的两次矩阵结果。
  - Event name or marker:
    - none
  - Correlation keys:
    - core lib nextest run
  - Differentiates from:
    - 生产 Git discovery、config layering 或 user-shell 逻辑回归
  - Supports if:
    - 入口无隔离且隔离运行消除对应失败。
  - Refutes if:
    - 入口已有隔离或隔离运行不改变失败。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
  - E-005
  - E-006
  - E-007
- Conclusion: confirmed；默认入口直接继承环境，受影响测试消费相应边界，而既有候选资格 runner 已实现正确隔离合同。
- Repair design readiness: ready；新增当前 vendor 薄入口并复用既有 helper，不修改产品逻辑或 vendored `justfile`。
- Next step: none
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 默认宿主环境包含污染输入
- Related hypotheses:
  - H-001
- Direction: supports
- Type: environment
- Source: `env`、`ls -ld /tmp/.git /tmp/.codex`
- Prediction or plan link:
  - H-001 默认测试进程可见宿主代理和共享临时根标记
- Matched signal:
  - 大小写代理变量均指向 `http://127.0.0.1:7890`，`/tmp/.git` 与 `/tmp/.codex` 均存在。
- Correlation keys:
  - core lib nextest run
- Raw content:
  ```text
  HTTP_PROXY=http://127.0.0.1:7890
  http_proxy=http://127.0.0.1:7890
  /tmp/.git
  /tmp/.codex
  ```
- Interpretation: 默认测试可观察到未由 fixture 创建的外部输入。
- Time: 2026-08-15 22:23

## Evidence E-002: 隔离环境消除额外失败
- Related hypotheses:
  - H-001
- Direction: supports
- Type: experiment
- Source: 默认与隔离完整 core lib nextest 输出
- Prediction or plan link:
  - H-001 改变宿主边界应只消除环境相关失败
- Matched signal:
  - 默认 27 failed；清除代理、`TMPDIR=/dev/shm`、`--test-threads=1` 后 21 failed，剩余均为既有延期类别。
- Correlation keys:
  - core lib nextest run
- Raw content:
  ```text
  default: 2178 run; 2151 passed; 27 failed
  isolated: 2178 run; 2157 passed; 21 failed
  ```
- Interpretation: 环境边界是额外 6 项失败的必要条件，支持入口级隔离而非产品逻辑修复。
- Time: 2026-08-15 22:23

## Evidence E-003: 默认入口与失败测试直接消费宿主边界
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code
- Source: `third_party/codex-cli/justfile`、core config/git/realtime/session tests
- Prediction or plan link:
  - H-001 默认入口不清理代理或系统 TMPDIR，失败测试使用默认临时根或启动 shell
- Matched signal:
  - `just test` 仅设置 Rust stack 与 Nextest profile；相关测试使用 `tempdir()`/`TempDir::new()`，shell 测试读取 `HTTP_PROXY`。
- Correlation keys:
  - core lib nextest run
- Interpretation: 默认完整回归没有进程级宿主环境边界，失败签名与消费路径一致。
- Time: 2026-08-15 22:42

## Evidence E-004: 候选资格 runner 已有可复用隔离合同
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code
- Source: `scripts/codex-upstream/qualify_candidate.py`
- Prediction or plan link:
  - H-001 正确修复应位于测试入口而非产品逻辑
- Matched signal:
  - package tests 已清理大小写 proxy 与 ambient sandbox 变量，设置私有 `TMPDIR`、`GIT_CEILING_DIRECTORIES` 和固定 umask；对应单测覆盖代理清理。
- Correlation keys:
  - Codex upstream qualification
- Interpretation: 当前缺口是日常 vendor 回归未暴露同一合同，不需要另造测试框架或修改生产代码。
- Time: 2026-08-15 22:42

## Evidence E-005: Git ceiling 不能约束 Codex 自有祖先发现
- Related hypotheses:
  - H-001
- Direction: supports
- Type: experiment
- Source: 首版隔离入口的 6 项定向复验
- Prediction or plan link:
  - H-001 修复必须真正隔离共享临时根，而非只设置 Git CLI 环境变量
- Matched signal:
  - 清理 proxy 后 shell 用例通过；其余 5 项仍发现 `/tmp/.git` 或 `/tmp/.codex`，结果为 1 passed、5 failed。
- Correlation keys:
  - isolated core targeted run
- Interpretation: `GIT_CEILING_DIRECTORIES` 对 Codex 自有项目发现不充分；runner 必须选择祖先链无 `.git/.codex` 标记的物理临时根。
- Time: 2026-08-15 22:49

## Evidence E-006: 安全临时根入口关闭环境失败
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: 新入口单测与 core 定向 Nextest
- Prediction or plan link:
  - H-001 入口级物理隔离应消除 6 项宿主环境失败
- Matched signal:
  - Python 同步工具测试 48/48；代理、config、Git 与 realtime context 六项 core 测试 6/6 passed。
- Correlation keys:
  - isolated core targeted run
- Raw content:
  ```text
  Python: Ran 48 tests; OK
  Nextest: 6 tests run; 6 passed; 2172 skipped
  ```
- Interpretation: 失败由宿主边界造成，入口级隔离充分且不需要修改产品逻辑。
- Time: 2026-08-15 22:55

## Evidence E-007: 完整 core lib 失败集合回到既有延期边界
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: 新隔离入口完整 `codex-core --lib` Nextest
- Prediction or plan link:
  - H-001 完整回归应不再包含 6 项环境失败，且不产生新分类
- Matched signal:
  - 2178 run、2157 passed、21 failed；失败精确属于 Guardian、remote plugin、remote model refresh 与 hosted image 既有延期分类。
- Correlation keys:
  - isolated core full run
- Raw content:
  ```text
  Nextest: 2178 tests run; 2157 passed; 21 failed; 0 skipped
  ```
- Interpretation: runner 在并行完整矩阵中稳定去除宿主噪声，串行化不是必要条件。
- Time: 2026-08-15 23:01
