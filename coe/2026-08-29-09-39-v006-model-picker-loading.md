# Problem P-001: v0.0.6 Release 启动后模型列表持续 Loading
- Status: closed
- Created: 2026-08-29 09:39 +0800
- Updated: 2026-08-30 03:44 +0800
- Objective: 定位本机 Whale v0.0.6 Release 启动后模型列表无法完成加载的根因。
- Symptoms:
  - 用户报告本机 Release 版本 Whale 0.0.6 启动后 model 一直处于 loading 状态。
  - 用户在进程持续运行约 16.5 小时后再次查看，界面已显示 `deepseek-v4-flash`；实际完成加载的时间未知。
- Expected behavior:
  - 启动后模型目录在有限时间内加载完成，并显示当前 provider 的可选模型或明确错误。
- Actual behavior:
  - 启动初期 model 长时间显示 loading，未呈现进度或可操作错误；同一进程后来自行显示模型，但缺少状态切换时间戳。
- Impact:
  - 本机 Whale v0.0.6 交互式使用受阻。
- Reproduction:
  - 启动本机 Whale v0.0.6，观察 model 状态；更精确步骤待运行时取证。
- Environment:
  - 本机；Release 版本 Whale 0.0.6；其余待取证。
- Known facts:
  - E-001：用户在本机 Release v0.0.6 观察到稳定的持续 loading 症状。
  - E-002：npm wrapper、平台包和原生二进制一致为 v0.0.6，Doctor 报告安装一致。
  - E-003：本机配置可解析，但当前 DeepSeek 新式凭据缺失；该状态不会单独阻塞 model/list。
  - E-004：真实进程的启动 bootstrap 请求没有完成，但 thread/start 已独立完成。
  - E-005：无 cache、无凭据的隔离 model/list 测试在 0.32 秒内成功返回 bundled models。
  - E-007：原始进程没有重启或禁用 hooks，用户后来观察到 `deepseek-v4-flash` 已出现。
  - E-008：该进程的持久日志只覆盖启动后的约 9 秒，无法量出 UI 实际完成加载的时间。
  - E-009：开发版普通启动与禁用 hooks 启动的三个预取组件均在 32ms 内完成。
  - E-010：原始 Release 进程在启动后约 102ms 已完成 bootstrap 并成功调用最终首帧渲染。
  - E-011：两次开发版 PTY 启动都显示 `deepseek-v4-flash`，最终首帧分别在 59ms 和 58ms 调度完成。
- Ruled out:
  - PATH/npm/platform 二进制版本错配。
  - 单纯缺少 DeepSeek 凭据或模型 cache 导致 model/list 无限等待。
  - 原始进程发生不可恢复的永久死锁。
  - hooks/plugin、model/list 或 configRequirements/read 在当前同配置启动中形成可测量阻塞。
- Fix criteria:
  - 根因候选通过运行时信号与代码路径证据门禁；若后续获准修复，原始启动场景不再持续 loading。
- Current conclusion: 未复现模型加载故障。原始 Release 已在 102ms 内完成 bootstrap 和最终首帧渲染，开发版同配置对照在 58–59ms 完成并显示模型；用户最初看到的是启动草稿的短暂 `loading` 占位态，观察间隔无法证明其持续时间。无需以 API Key、hooks 或模型目录为方向修复。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - H-002：E-005、E-009、E-010、E-011
  - H-004：E-009、E-011
- Close reason:
  - not reproduced; runtime evidence shows successful bounded startup and final model rendering

## Hypothesis H-001: 本机 Release 安装产物与 v0.0.6 预期不一致
- Status: refuted
- Parent: P-001
- Claim: PATH 命中的 Whale、npm 包或原生二进制存在版本/来源错配，使运行时使用了不兼容的模型目录实现或资源。
- Layer: environment
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - npm wrapper、平台原生包与 PATH 二进制是独立层，Release 安装错配可产生仅本机出现的启动异常。
- Falsifiable predictions:
  - If true: 命令解析路径、包版本、二进制版本或安装 provenance 至少一项不一致。
  - If false: PATH、包与二进制均一致指向 v0.0.6 Release 安装。
- Diagnostic evidence plan:
  - Prediction or clause under test: 核对 PATH、包管理器元数据、文件链接和 `--version` 输出的一致性。
  - Signal: 安装路径、符号链接、包版本、二进制版本、文件哈希。
  - Capture method: 只读 shell 探针；运行 Whale 前先通过 workspace safety 门禁。
  - Event name or marker:
    - whale-install-identity
  - Correlation keys:
    - executable-path
  - Differentiates from:
    - H-002、H-003
  - Supports if:
    - 任一身份层不一致或命中非预期安装。
  - Refutes if:
    - 所有身份层一致为 v0.0.6 Release。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: PATH、npm 顶层包、Linux x64 平台包和原生二进制均一致为 v0.0.6，安装错配被反证。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: stop
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 模型目录异步请求没有完成或错误未退出 Loading 状态
- Status: refuted
- Parent: P-001
- Claim: 启动时模型目录刷新请求因网络、认证、协议或错误处理路径未完成，UI 的 loading 状态未被清除。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 持续 loading 直接对应模型目录异步生命周期，但必须以日志或复现信号区分请求失败与 UI 状态机错误。
- Falsifiable predictions:
  - If true: 日志/运行时显示目录请求未完成、重复重试、失败，或失败后缺失 loading 终止事件。
  - If false: 目录请求成功返回且 UI 收到并处理完成事件。
- Diagnostic evidence plan:
  - Prediction or clause under test: 捕获一次受控启动中的模型目录请求、配置要求请求与 UI 完成/失败信号。
  - Signal: `whale_startup_probe` 事件的 `component=model_list|config_requirements`、`duration_ms`、`outcome`。
  - Capture method: 在开发版 TUI 请求边界加入诊断耗时日志，构建后做最小启动复现，不发送自然语言请求。
  - Event name or marker:
    - model-catalog-load
  - Correlation keys:
    - startup timestamp
  - Differentiates from:
    - H-001、H-003
  - Supports if:
    - 请求链路异常与 loading 持续时间相关，且缺少成功完成信号。
  - Refutes if:
    - 目录请求与 UI 完成信号均正常。
  - Instrumentation status: completed and removed
  - Instrumentation lifecycle:
    - 诊断专用；根因确认后删除，或经单独判断转为永久启动可观测性。
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-004
  - E-005
  - E-007
  - E-008
  - E-009
  - E-010
  - E-011
- Conclusion: 隔离 model/list、开发版组件打点和原始 Release 首帧日志共同反证请求未完成；model_list 为 2ms，config_requirements 为 19–22ms，原始 Release bootstrap 为 34ms。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: stop
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: 当前 provider 或配置状态使模型目录无法解析
- Status: refuted
- Parent: P-001
- Claim: 本机持久化 provider/model 选择、认证状态或配置字段与 v0.0.6 不兼容，导致模型目录无法形成有效分组。
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - v0.0.6 引入多 provider 与模型选择持久化，环境特定配置是独立候选原因。
- Falsifiable predictions:
  - If true: 脱敏配置或日志显示无效 provider/model、缺失认证或解析错误，并与 loading 同时出现。
  - If false: 配置解析、provider 选择和凭据可用性均正常。
- Diagnostic evidence plan:
  - Prediction or clause under test: 读取脱敏配置结构、provider 选择和认证状态，不输出密钥。
  - Signal: 配置字段、provider 路由、doctor 机械诊断、相关错误日志。
  - Capture method: 脱敏文件检查和通过门禁后的 `whale doctor`。
  - Event name or marker:
    - provider-config-state
  - Correlation keys:
    - provider-id
  - Differentiates from:
    - H-001、H-002
  - Supports if:
    - 发现可导致模型目录失败的配置/认证事实。
  - Refutes if:
    - provider/config/认证诊断均正常。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
  - E-005
- Conclusion: 缺少 DeepSeek 新式凭据会阻止后续推理，但隔离 model/list 在相同无凭据、无 cache 条件下成功返回 bundled models，不能解释无限 loading。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: stop
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-004: startup hooks/plugin 加载阻塞 TUI bootstrap 组合等待
- Status: refuted
- Parent: P-001
- Claim: TUI 用 `tokio::join!` 同时等待 bootstrap 与 startup hooks review；本机 hooks/list 在插件解析阶段未返回，导致启动草稿持续显示 model loading，即使 thread/start 已完成。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - H-002
- Rationale:
  - 真实日志显示 hooks/list 进入配置和插件解析后无响应；代码要求 hooks 与 bootstrap 两侧都完成才离开 startup draft。
- Falsifiable predictions:
  - If true: 在已信任工作目录以 `whale --disable hooks` 启动时，model 状态会完成加载并进入主界面。
  - If false: 禁用 hooks 后仍在 post-trust bootstrap 阶段持续 loading，且日志显示 hooks/list 已快速完成或未执行。
- Diagnostic evidence plan:
  - Prediction or clause under test: hooks/list 的完成时间是否决定 post-trust startup loading 的结束，以及禁用 hooks 是否消除该延迟。
  - Signal: `whale_startup_probe` 事件的 `component=hooks_list`、`duration_ms`、`outcome`，并与 model/config 两个组件和 UI 状态比较。
  - Capture method: 在开发版 startup hooks 请求边界加入诊断耗时日志，分别运行普通启动与禁用 hooks 对照，不提交自然语言输入。
  - Event name or marker:
    - hooks-disabled-post-trust-startup
  - Correlation keys:
    - startup process uuid
  - Differentiates from:
    - H-002 中 model/list 自身阻塞；configRequirements/read 独立阻塞。
  - Supports if:
    - 禁用 hooks 后 model 在 10 秒内完成加载。
  - Refutes if:
    - 禁用 hooks 后仍持续 loading，且 hooks 路径已被跳过。
  - Instrumentation status: completed and removed
  - Instrumentation lifecycle:
    - 诊断专用；根因确认后删除，或经单独判断转为永久启动可观测性。
- Evidence gate: satisfied
- Related evidence:
  - E-004
  - E-005
  - E-006
  - E-007
  - E-008
  - E-009
  - E-010
  - E-011
- Conclusion: 普通启动 hooks_list 为 32ms，禁用 hooks 时为 30ms，最终首帧为 59ms 对 58ms；hooks/plugin 不构成可测量瓶颈。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: stop
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 用户报告 v0.0.6 Release 持续 Loading
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: neutral
- Type: user-feedback
- Source: 2026-08-29 用户消息
- Prediction or plan link:
  - P-001 症状定义
- Matched signal:
  - Release 版本 Whale 0.0.6 启动后 model 一直在 loading 状态
- Correlation keys:
  - local-machine
- Raw content:
  ```text
  本机的release 版本 whale 0.0.6 启动后model 一直在loading状态，检查问题
  ```
- Interpretation: 证明需要调查的用户可见症状存在，但不足以区分三个候选原因。
- Time: 2026-08-29 09:39 +0800

## Evidence E-002: Release 安装身份一致
- Related hypotheses:
  - H-001
- Direction: refutes
- Type: environment
- Source: `command -v whale`、npm metadata、平台二进制 `--version`、`whale doctor`
- Prediction or plan link:
  - H-001 安装身份一致性检查
- Matched signal:
  - wrapper、`@ceasarxuu/whalecode@0.0.6`、`0.0.6-linux-x64` 和原生 `whale 0.0.6` 一致；Doctor 显示 install consistent。
- Correlation keys:
  - `cdf2a26294894412226ab116861cdf4b76d4d8f899c12b7a6bd3f48c864bbcda`
- Raw content:
  ```text
  whale 0.0.6
  @ceasarxuu/whalecode@0.0.6
  @ceasarxuu/whalecode 0.0.6-linux-x64
  install consistent
  ```
- Interpretation: 反证 PATH、npm 包和原生平台包错配是本次 loading 根因。
- Time: 2026-08-29 09:42 +0800

## Evidence E-003: 配置正常但 DeepSeek 新式凭据缺失
- Related hypotheses:
  - H-003
- Direction: neutral
- Type: config
- Source: `whale doctor` 与脱敏 auth 结构检查
- Prediction or plan link:
  - H-003 provider/config/认证诊断
- Matched signal:
  - config.toml 解析成功，active model/provider 为 deepseek-v4-flash/deepseek；DEEPSEEK_API_KEY 与 auth.json 的 deepseek_api_key 均缺失。
- Correlation keys:
  - provider-id=deepseek
- Raw content:
  ```text
  config loaded
  model deepseek-v4-flash · deepseek
  active model provider auth env var is missing
  ```
- Interpretation: 凭据缺失是真实的后续推理阻断问题，但尚不能解释 picker 无限 loading。
- Time: 2026-08-29 09:43 +0800

## Evidence E-004: 真实启动的组合 bootstrap 未完成
- Related hypotheses:
  - H-002
  - H-004
- Direction: supports
- Type: diagnostic-log
- Source: `~/.whale/logs_2.sqlite` process_uuid `pid:1334303:704e4dc9-61d8-485a-a794-456f39063fad`
- Prediction or plan link:
  - H-002 model catalog 生命周期；H-004 startup hooks 组合等待
- Matched signal:
  - request_id=2 model/list、request_id=3 configRequirements/read 和 hooks/list 均只有开始信号；thread/start 已发出 thread/started；进程持续存活且 UI 保持 loading。
- Correlation keys:
  - process_uuid=pid:1334303:704e4dc9-61d8-485a-a794-456f39063fad
  - model-list-request-id=2
- Raw content:
  ```text
  app-server typed request ... rpc.method="model/list" rpc.request_id=2
  app-server typed request ... rpc.method="configRequirements/read" rpc.request_id=3
  app-server typed request ... rpc.method="hooks/list"
  app-server event: thread/started
  ```
- Interpretation: loading 不是二进制启动失败或 thread 创建失败，而是 startup prefetch 的组合等待没有闭合；单凭日志无法断言是哪一个 join 分支。
- Time: 2026-08-29 09:45 +0800

## Evidence E-005: 无 cache、无凭据的 model/list 隔离测试成功
- Related hypotheses:
  - H-002
  - H-003
  - H-004
- Direction: refutes
- Type: test
- Source: `just test -p codex-app-server --test all list_models_without_cache_or_credentials_returns_bundled_models -- --exact --nocapture`
- Prediction or plan link:
  - H-002/H-003 的无 cache、无凭据组合是否足以阻塞 model/list
- Matched signal:
  - 0.32 秒内返回三个 DeepSeek bundled models 和三个 MissingCredentials provider groups。
- Correlation keys:
  - nextest-run-id=7f6a57df-36aa-4500-b327-7dc679ba2bc5
- Raw content:
  ```text
  PASS ... list_models_without_cache_or_credentials_returns_bundled_models
  1 passed; 0 failed; finished in 0.32s
  ```
- Interpretation: 强力反证 cache/key 缺失本身导致 model/list 无限等待，并把调查重心转向真实启动中的并发/组合路径。
- Time: 2026-08-29 09:46 +0800

## Evidence E-006: hooks-disabled 对照被目录信任门禁截断
- Related hypotheses:
  - H-004
- Direction: neutral
- Type: experiment
- Source: `WAR-20260829-095058-V006-LOADING-HOOKS-OFF-R2`
- Prediction or plan link:
  - H-004 hooks-disabled-post-trust-startup
- Matched signal:
  - TUI 从 startup draft 进入目录信任屏幕；未替用户做信任决策，无法进入 post-trust bootstrap。
- Correlation keys:
  - process_uuid=pid:1345621:982156d2-1bed-412b-a14c-cb79bf8262c1
- Raw content:
  ```text
  model: loading
  Do you trust the contents of this directory?
  ```
- Interpretation: 该对照不支持也不反驳 H-004；下一步必须由用户在已信任环境完成同一命令。
- Time: 2026-08-29 09:52 +0800

## Evidence E-007: 原始进程最终自行显示 DeepSeek 模型
- Related hypotheses:
  - H-002
  - H-004
- Direction: refutes
- Type: user-feedback
- Source: 2026-08-30 用户消息与进程状态
- Prediction or plan link:
  - H-002 请求链路是否永久不完成；H-004 hooks 启用时是否永久阻塞组合等待
- Matched signal:
  - 未重启的原始 v0.0.6 进程仍在运行，用户再次查看时界面已显示 `deepseek-v4-flash`。
- Correlation keys:
  - process_uuid=pid:1334303:704e4dc9-61d8-485a-a794-456f39063fad
- Raw content:
  ```text
  我很长时间没观察了，刚才看了一眼deepseek-v4-flash 模型出现了
  process elapsed: about 16:36 at evidence capture
  ```
- Interpretation: 反证原始进程发生不可恢复的永久死锁，但由于用户没有持续观察，不能把约 16.5 小时当作实际加载耗时，也不能单独定位慢分支。
- Time: 2026-08-30 02:16 +0800

## Evidence E-008: 原始进程日志没有覆盖模型出现时刻
- Related hypotheses:
  - H-002
  - H-004
- Direction: neutral
- Type: diagnostic-log
- Source: `~/.whale/logs_2.sqlite`
- Prediction or plan link:
  - H-002/H-004 完成时间线取证
- Matched signal:
  - 该 process_uuid 共 81 条日志，时间范围仅为 2026-08-29 09:39:59 至 09:40:08；没有模型 UI 状态切换事件。
- Correlation keys:
  - process_uuid=pid:1334303:704e4dc9-61d8-485a-a794-456f39063fad
- Raw content:
  ```text
  81|2026-08-29 09:39:59|2026-08-29 09:40:08
  ```
- Interpretation: 不能从现有日志判断模型是在数秒、数分钟还是更久后出现；需要带秒表的新启动对照，而不是对当前进程继续等待。
- Time: 2026-08-30 02:16 +0800

## Evidence E-009: 开发版启动组件耗时均低于 33ms
- Related hypotheses:
  - H-002
  - H-004
- Direction: refutes
- Type: diagnostic-log
- Source: `~/.whale/logs_2.sqlite` 的 `whale_startup_probe` 事件
- Prediction or plan link:
  - H-002/H-004 预声明的组件耗时对照
- Matched signal:
  - 普通启动：model_list=2ms、config_requirements=22ms、hooks_list=32ms；禁用 hooks：2ms、19ms、30ms；全部 outcome=ok。
- Correlation keys:
  - baseline process_uuid=pid:1632709:72db7e7f-6da6-4566-a1b8-0944029ec378
  - hooks-disabled process_uuid=pid:1633126:89fac74d-382d-4ecb-a5fd-3642b3319e1a
- Raw content:
  ```text
  baseline: model_list=2ms config_requirements=22ms hooks_list=32ms
  hooks-disabled: model_list=2ms config_requirements=19ms hooks_list=30ms
  outcome=ok
  ```
- Interpretation: 开发环境同一 Whale home 和插件配置下，没有任何预取组件形成用户可感知延迟；禁用 hooks 没有实质改善。
- Time: 2026-08-30 03:44 +0800

## Evidence E-010: 原始 Release 在 102ms 内完成最终首帧渲染调用
- Related hypotheses:
  - H-002
  - H-004
- Direction: refutes
- Type: diagnostic-log
- Source: 原始 v0.0.6 process_uuid `pid:1334303:704e4dc9-61d8-485a-a794-456f39063fad`
- Prediction or plan link:
  - H-002/H-004 启动组合等待是否未完成
- Matched signal:
  - `render_startup_frame` 成功返回后记录 `tui startup initial frame scheduled duration_ms=102 bootstrap_ms=34`。
- Correlation keys:
  - log-id=786318
- Raw content:
  ```text
  tui startup initial frame scheduled duration_ms=102 bootstrap_ms=34 runtime_model_provider_ms=0 thread_and_widget_ms=12 initial_session_ms=0 event_stream_ms=0
  ```
- Interpretation: 原始 Release 的模型和配置 bootstrap 已在 34ms 完成，App 最终首帧渲染调用在总计约 102ms 内成功返回；现有日志不支持后台请求持续阻塞。
- Time: 2026-08-29 09:39:59 +0800

## Evidence E-011: 两次开发版 PTY 最终首帧均显示模型
- Related hypotheses:
  - H-002
  - H-004
- Direction: refutes
- Type: reproduction
- Source: `WAR-20260830-034123-DEV-STARTUP-PROBE-R1`
- Prediction or plan link:
  - H-002/H-004 UI 完成信号与禁用 hooks 对照
- Matched signal:
  - 普通启动和禁用 hooks 启动均从短暂 loading 草稿切换为 `deepseek-v4-flash high`；首帧日志分别为 59ms 和 58ms。
- Correlation keys:
  - baseline log-range=786411..786493
  - hooks-disabled log-range=786494..786571
- Raw content:
  ```text
  model: deepseek-v4-flash high
  baseline initial frame duration_ms=59
  hooks-disabled initial frame duration_ms=58
  ```
- Interpretation: 开发环境完成了视觉状态与结构化耗时的闭环验证，未复现持续 loading；最初可见的 loading 是启动草稿占位态。
- Time: 2026-08-30 03:44 +0800
