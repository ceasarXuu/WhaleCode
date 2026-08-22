# Problem P-001: DeepSeek Vision 真实冒烟缺少预期 marker
- Status: resolved
- Created: 2026-08-23 00:09
- Updated: 2026-08-23 00:24
- Objective: 确定 Vision 冒烟失败发生在图片入站、provider 路由、模型视觉语义还是测试判定层。
- Symptoms:
  - `deepseek-v4-flash-vision-exp` 的 Responses 请求完成，但最终消息不含 `WHALE_DS_VISION_OK`。
- Expected behavior:
  - 图片进入原生 Responses `input_image`；验收素材与提示词描述一致时，模型返回固定 marker。
- Actual behavior:
  - 共享计数器记录 1 次 provider 请求；runner 因最终消息缺少 marker 退出 1。
- Impact:
  - Vision 的离线 wire 合同为绿，但真实视觉语义能力未获通过证据。
- Reproduction:
  - 见 `scripts/deepseek-responses/run_current_models_smoke.py` 的 Vision-only 路径和账本记录 `WAR-20260822-053058-DEEPSEEK-VISION-R1`。
- Environment:
  - Linux；分支 `whalecode-alpha`；实现提交 `521e7730c`；当前 HEAD `a4d54d5d5`；DeepSeek Responses API。
- Known facts:
  - 请求计数器为 1；命令退出发生在 marker 断言；当次 harness 没有在断言前持久化最终消息和 usage。
- Ruled out:
  - CLI `--image` 未进入历史：本地 mock 集成测试确认 `LocalImage` 被保存为 `input_image`。
  - 原始 `application/octet-stream` data URL 直接发送：历史边界的图片准备会解码并重新编码为规范图片 data URL。
- Fix criteria:
  - 用不付费证据定位最可能层；若仍需真实重现，必须先获得新预算并在请求前持久化脱敏 wire/最终消息/usage。
- Current conclusion: 失败由测试素材与提示词不一致导致。提示要求只在图片含 OpenAI knot logo 时输出 marker，实际 PNG 是打开的书本图标；模型不输出 marker 符合提示，runner 的严格断言随后将其判为失败。该记录不能作为 Vision 不支持图片的证据。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - E-002 直接检查实际输入素材，确认其为 100x100 的书本图标而非 OpenAI knot logo。
  - E-003 证明 `--image` 到 Responses `input_image` 的本地链路可用。
  - E-004 将错误判定确定到 marker 断言，且 provider 请求本身已完成。
- Close reason:
  - 根因已确认在测试 fixture/oracle，不在已观察到的图片传输路径。

## Hypothesis H-001: 图片在 Whale 输入转换或 wire 清理阶段丢失
- Status: refuted
- Parent: P-001
- Claim: CLI 收到图片路径，但最终 DeepSeek Responses 请求未包含有效 `input_image`。
- Layer: root-cause
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 真实证据未保存请求体；离线 API client 测试不覆盖 CLI 图片加载到最终 wire 的完整链路。
- Falsifiable predictions:
  - If true: CLI 图片加载/模型模态过滤路径会删除或替换该图片，或最终 mock wire 中没有 `input_image`。
  - If false: 相同 CLI 参数经本地 mock 可观察到有效 `input_image` data URL 和 Vision 模型 slug。
- Diagnostic evidence plan:
  - Prediction or clause under test: 相同 CLI 参数是否生成 `input_image`。
  - Signal: 本地 mock/final-wire 请求体与图片加载代码路径。
  - Capture method: 只读代码追踪，并优先复用本地 mock 测试；不得调用真实 provider。
  - Event name or marker:
    - input_image
  - Correlation keys:
    - model slug
  - Differentiates from:
    - H-002, H-003
  - Supports if:
    - 最终 wire 缺图或图片被占位文本替换。
  - Refutes if:
    - 最终 wire 包含正确 data URL、detail 与 Vision slug。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: 本地图片在历史边界经过图片准备，并以 `input_image` 形式保留；同路径 mock 集成测试通过。
- Repair design readiness: not applicable
- Next step: none
- Blocker:
  - none
- Close reason:
  - 预测“最终链路丢图”与 E-003 冲突。

## Hypothesis H-002: 请求带图但服务端未按 Vision 模型处理
- Status: blocked
- Parent: P-001
- Claim: 最终请求包含图片，但 provider 实际路由/模型响应未启用 Vision 处理。
- Layer: interaction
- Factor relation: any_of
- Depends on:
  - H-001 refuted
- Rationale:
  - DeepSeek 非 Vision 模型在 Responses 中可能把图片替换为占位文本；当次没有保存响应 model 或原始 SSE。
- Falsifiable predictions:
  - If true: 脱敏 wire 显示正确图片，但响应 model/文本表现为未看到图片或错误路由。
  - If false: 响应确认 Vision slug 且输出内容显示正确读取图片。
- Diagnostic evidence plan:
  - Prediction or clause under test: provider 请求与响应 model 是否一致。
  - Signal: provider wire trace、响应 headers/model 和最终消息。
  - Capture method: 检查现存 trace；缺失时仅提出下一次预算化诊断，不直接重跑。
  - Event name or marker:
    - response.created
  - Correlation keys:
    - model slug
    - ledger record id WAR-20260822-053058-DEEPSEEK-VISION-R1
  - Differentiates from:
    - H-001, H-003
  - Supports if:
    - 请求带图但响应路由或文本明确未处理图片。
  - Refutes if:
    - 响应确认 Vision slug且正确描述图片。
  - Instrumentation status: diagnostic-only
  - Instrumentation lifecycle:
    - 下次真实诊断前持久化脱敏证据；不记录凭据。
- Evidence gate: not satisfied
- Related evidence:
  - E-001
- Conclusion: 当次原始响应 model/SSE 未持久化，无法对服务端路由作独立确认；但无需用此假设解释已确认的 marker 失败。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: 如未来重新验证服务端路由，应在新预算下先持久化脱敏请求模型、响应模型与最终消息。
- Blocker:
  - 当次 runner 未持久化原始响应。
- Close reason:
  - 当次证据不可恢复；不影响 P-001 的测试判定根因结论。

## Hypothesis H-003: Vision 正常，固定 marker 测试产生语义误判
- Status: confirmed
- Parent: P-001
- Claim: 模型正常读取图片，但小图内容、提示词或严格 marker 规则导致最终消息未包含指定字符串。
- Layer: diagnostic
- Factor relation: any_of
- Depends on:
  - none
- Rationale:
  - 测试图片是 1429 字节的小型 OpenAI skill 图标；判定只接受精确 marker，未保存实际回复供人工判断。
- Falsifiable predictions:
  - If true: 图片本身并非清晰的“OpenAI knot logo”，或模型回复语义正确但格式不含 marker。
  - If false: 图片清晰、请求正确，而模型明确表示无法看到或错误识别图片。
- Diagnostic evidence plan:
  - Prediction or clause under test: 测试素材和 marker 规则是否足以稳定判定视觉能力。
  - Signal: 本地图片视觉检查、像素/尺寸信息、runner 断言位置和现存最终消息。
  - Capture method: 查看仓库图片并审计 runner；不调用 provider。
  - Event name or marker:
    - WHALE_DS_VISION_OK
  - Correlation keys:
    - image path
  - Differentiates from:
    - H-001, H-002
  - Supports if:
    - 图片含义不够唯一，或断言会拒绝语义正确的非精确输出。
  - Refutes if:
    - 图片清晰唯一且实际回复明确没有视觉输入。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-004
- Conclusion: 测试图片是书本图标；提示明确要求非 knot logo 时解释，runner 又只接受 marker，因此正确的否定回答必然被判失败。
- Repair design readiness: ready; repair requires user authorization
- Next step: 若用户要求修复，改用与问题一致的确定性素材/问题，并保留最终消息与 usage 证据。
- Blocker:
  - none
- Close reason:
  - E-002 与 E-004 共同满足根因证据门槛。

## Evidence E-001: 账本与失败证据只证明 marker 不匹配
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: neutral
- Type: observation
- Source: `benchmarks/whale-agent-run-ledger.json`、Vision failure-summary、provider request counter
- Prediction or plan link:
  - 三个假设的共同基线事实。
- Matched signal:
  - provider_request_count=1；stop_reason=vision_response_missing_expected_semantic_marker；usage unavailable。
- Correlation keys:
  - WAR-20260822-053058-DEEPSEEK-VISION-R1
- Raw content:
  ```text
  The provider response completed but did not contain the required image-recognition marker.
  provider-request-count.txt = 1
  usage_evidence_status = unavailable
  ```
- Interpretation: 现有证据不能区分图片丢失、provider 路由问题和测试误判。
- Time: 2026-08-23 00:09

## Evidence E-002: 实际 PNG 是书本图标而非 OpenAI knot logo
- Related hypotheses:
  - H-003
- Direction: supports
- Type: observation
- Source: `third_party/codex-cli/codex-rs/skills/src/assets/samples/openai-docs/assets/openai.png` 的原图检查与文件元数据
- Prediction or plan link:
  - H-003 对测试素材与提示词语义不一致的预测。
- Matched signal:
  - PNG 为 100x100、RGB；视觉内容是红色描边的打开书本，没有 OpenAI knot logo。
- Correlation keys:
  - image path
- Raw content:
  ```text
  PNG image data, 100 x 100, 8-bit/color RGB, non-interlaced
  Visual inspection: open-book icon
  ```
- Interpretation: 按提示的 `otherwise explain briefly` 分支，正确识别该图时不应返回 marker。
- Time: 2026-08-23 00:18

## Evidence E-003: 本地图片输入链路保留 input_image
- Related hypotheses:
  - H-001
- Direction: refutes
- Type: test
- Source: `core/src/image_preparation.rs`、`core/tests/suite/image_rollout.rs`
- Prediction or plan link:
  - H-001 的最终链路丢图预测。
- Matched signal:
  - `prepare_image` 解码 data URL 后以实际图片格式重新生成 data URL；`copy_paste_local_image_persists_rollout_request_shape` 断言本地图片保留为 `ContentItem::InputImage`。
- Correlation keys:
  - LocalImage
  - input_image
- Raw content:
  ```text
  RUST_MIN_STACK=33554432 cargo test ... copy_paste_local_image_persists_rollout_request_shape
  test suite::image_rollout::copy_paste_local_image_persists_rollout_request_shape ... ok
  1 passed; 0 failed
  ```
- Interpretation: 现有本地链路证据不支持“图片在 Whale 转换中丢失”。首次以默认线程栈运行出现测试进程 stack overflow；增大测试线程栈后同一测试通过，该首次失败不涉及产品断言。
- Time: 2026-08-23 00:23

## Evidence E-004: runner 的条件提示与严格 marker 断言构成确定性误判
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code
- Source: `scripts/deepseek-responses/run_current_models_smoke.py:106-123,185-197`
- Prediction or plan link:
  - H-003 的严格 marker 规则预测。
- Matched signal:
  - 提示仅在图片含 knot logo 时要求 marker，否则要求简短解释；runner 无论图片内容都要求最终消息包含 marker。
- Correlation keys:
  - WHALE_DS_VISION_OK
- Raw content:
  ```text
  Respond exactly WHALE_DS_VISION_OK if it contains the OpenAI knot logo; otherwise explain briefly.
  if marker not in message: raise RuntimeError(...)
  ```
- Interpretation: 对实际书本图标，模型遵循提示的正确输出与 runner 的通过条件互斥；因此“未通过”是 fixture/oracle 缺陷。
- Time: 2026-08-23 00:24
