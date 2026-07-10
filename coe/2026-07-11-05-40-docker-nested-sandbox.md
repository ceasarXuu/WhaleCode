# Problem P-001: Docker benchmark 内嵌套 sandbox 导致工具链失效
- Status: open
- Created: 2026-07-11 05:40
- Updated: 2026-07-11 05:40
- Objective: 让统一 Docker benchmark 中的 Whale 工具可正常访问容器工作区，同时保持 Docker 硬隔离边界。
- Symptoms:
  - `count-call-stack` Standard 首侧连续 51 次出现 bwrap namespace 权限错误，随后产生 253 次盲目 `apply_patch`。
- Expected behavior:
  - Agent 在 Docker 工作区中可执行读取、编辑和测试；隔离由容器 contract 提供。
- Actual behavior:
  - runner 在容器内传入 `--full-auto`，Whale 再启动 bwrap；容器不允许创建所需 namespace，所有 shell 工具失败。
- Impact:
  - R5 J4/I3 Docker 对照无效，并持续放大 provider 请求、时间和 token 成本。
- Reproduction:
  - 执行 `run-taskspace-benchmark.ps1 -Scenario count-call-stack -Repeats 3`，默认 `SandboxMode=full-auto`。
- Environment:
  - Linux Docker，branch `whalecode-alpha`，Whale commit `85c9d5a`，run `20260711-053429-428`。
- Known facts:
  - `whale-argv.json` 明确包含 `--full-auto`。
  - 原始 tool output 忠实包含 bwrap namespace 权限错误，反馈没有丢失或改写。
  - shell 读取全部失败后，Agent 才开始盲目 patch；该退化不是 TaskSpace 状态机触发。
- Ruled out:
  - TaskSpace projection/Map 状态机不是触发源，因为失败发生在 Standard 侧且首个 shell 调用即出现。
- Fix criteria:
  - Docker agent argv 固定使用 `--dangerously-bypass-approvals-and-sandbox`，并在启动前拒绝嵌套 sandbox 参数。
  - 容器内 smoke 能成功读取工作区；paired sample 不再出现 bwrap namespace 错误。
- Current conclusion: Docker 已经提供硬隔离，但 benchmark 又在容器内启用 bwrap，两个隔离层能力不兼容；这是已由 argv 与原始工具输出共同确认的执行环境配置缺陷。
- Related hypotheses:
  - H-001
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: `--full-auto` 在 Docker 内触发不可用的嵌套 bwrap
- Status: confirmed
- Parent: P-001
- Claim: Docker agent argv 中的 `--full-auto` 使 Whale 在容器内创建 bwrap namespace，并导致所有 shell 工具在执行任务前失败。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 同一容器既有 Docker 资源/挂载隔离，又向 Whale 传入本机 sandbox 模式；错误文本直接指向 namespace 创建失败。
- Falsifiable predictions:
  - If true: argv 包含 `--full-auto`，首个普通 shell 输出即为 bwrap namespace 错误；改为 bypass 后相同 Docker contract 中 shell 可执行。
  - If false: argv 不包含 sandbox 参数，或 bypass 后仍在首个 shell 上出现同一 bwrap 错误。
- Diagnostic evidence plan:
  - Prediction or clause under test: argv 与首个 shell 失败必须同时符合上述预测。
  - Signal: `whale-argv.json` 与 rollout `function_call_output`。
  - Capture method: 保留失败 run artifact，并执行结构化查询。
  - Event name or marker:
    - `container.agent_started`
  - Correlation keys:
    - run `20260711-053429-428`, pair `pair-001`, side `left`
  - Differentiates from:
    - provider tool-call 扭曲、TaskSpace projection 丢失、任务本身测试失败
  - Supports if:
    - argv 包含 `--full-auto` 且首批 shell output 均为 bwrap namespace 权限错误。
  - Refutes if:
    - shell 在相同 argv 下可读取工作区，或错误发生在工具执行前的其他层。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留 argv、container lifecycle 和原始 rollout 观测；增加启动前机械契约校验。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
- Conclusion: argv 与原始工具反馈完整支持因果机制，并排除了 TaskSpace 专属链路。
- Repair design readiness: ready
- Next step: 固定 Docker agent 使用 bypass，增加拒绝嵌套 sandbox 的契约测试，并执行同样本验证。
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 容器内 Whale argv 启用了 full-auto
- Related hypotheses:
  - H-001
- Direction: supports
- Type: config
- Source: `target/r5-j4-fixed-topology/count-call-stack/20260711-053429-428/pair-001/left/artifacts/whale-argv.json`
- Prediction or plan link:
  - H-001 的 argv 预测
- Matched signal:
  - `--full-auto`
- Correlation keys:
  - run `20260711-053429-428`, pair `pair-001`, side `left`
- Raw content:
  ```text
  ["exec","--json",...,"-C","/workspace","--full-auto","--output-last-message","/artifacts/last-message.md","-"]
  ```
- Interpretation: runner 明确要求 Whale 在 Docker 容器内再次启用本机 sandbox。
- Time: 2026-07-11 05:38

## Evidence E-002: 首批 shell 工具忠实返回 bwrap 权限错误
- Related hypotheses:
  - H-001
- Direction: supports
- Type: diagnostic-log
- Source: 失败 run 的 session rollout 与 `whale-exec.stderr.log`
- Prediction or plan link:
  - H-001 的首个 shell 失败预测
- Matched signal:
  - 51 条同类 bwrap namespace 错误
- Correlation keys:
  - thread `019f4df4-1b83-7211-baba-1b1f9afcd914`
- Raw content:
  ```text
  Execution outcome: exited
  Shell exit code: 1
  Output: bwrap: No permissions to create a new namespace, likely because the kernel does not allow non-privileged user namespaces
  ```
- Interpretation: shell 命令尚未读取任务文件就被嵌套 sandbox 拒绝，工具结果本身没有丢失。
- Time: 2026-07-11 05:38

## Evidence E-003: 失败发生在 Standard 且先于盲目 patch
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: 同一 run 的三个 session rollout 结构化计数
- Prediction or plan link:
  - H-001 对 TaskSpace 专属原因的区分条款
- Matched signal:
  - Standard left：`exec_command=45`、`apply_patch=253`；最先出现的是 shell/bwrap 失败，之后才出现随机目标 patch。
- Correlation keys:
  - run `20260711-053429-428`, logical mode `standard`
- Raw content:
  ```text
  253 apply_patch
   45 exec_command
    2 spawn_agent
  ```
- Interpretation: 这是 Docker 执行基底故障导致的 Agent 退化，不是 TaskSpace Map 或 projection 对 Agent 的约束。
- Time: 2026-07-11 05:39
