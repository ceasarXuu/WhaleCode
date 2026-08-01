# Problem P-001: R8 缓存门禁收尾的成本与证据可信性缺口
- Status: fixed
- Created: 2026-08-01 14:10
- Updated: 2026-08-01 14:10
- Objective: 关闭证据错配、授权后执行漂移、Realtime 请求绕过和迟到容器四条发布阻断路径。
- Symptoms:
  - 自报 arm 可把同一份 Standard 证据重复解释成 TaskSpace 证据。
  - 用户授权后，未提交的 runner 变更不改变 proposal，实际执行对象却已变化。
  - Realtime conversation 的 WebSocket 连接不消耗专项 provider 请求额度。
  - 容器第一次查询为空即被判定清理完成，迟到容器可能出现在确认之后。
- Expected behavior:
  - 每个矩阵行绑定唯一 run、唯一 artifact 和 artifact 内真实运行模式。
  - 授权精确绑定实际付费 runner、容器脚本和所选 sample 输入。
  - 专项硬上限覆盖所有真实网络 dispatch，且隐藏重试不能放大额度。
  - 进程树终止后，容器必须经过连续空集确认才能视为清理完成。
- Actual behavior:
  - 四条路径均可绕过当前相应校验或后置条件。
- Impact:
  - 缓存结果可能假阳性，或真实 API 成本、容器生命周期脱离用户批准边界。
- Reproduction:
  - 见 E-001 至 E-004。
- Environment:
  - Linux，分支 whalecode-alpha，起始 HEAD 0ac6e78b2。
- Known facts:
  - 119 项 Python 与 5 项 provider hard-limit Rust 测试通过，但缺少四条反例。
  - 当前真实基线仍为 live_regression_failed，发布保持 fail closed。
- Ruled out:
  - 不是 TaskSpace、普通 Tool 或 Agent 决策语义变化。
  - 不需要引入签名或外部信任根；威胁模型不包含恶意仓库维护者。
- Fix criteria:
  - 四条反例均有先失败后通过的离线测试；完整离线回归通过；fresh review 无 blocking。
- Current conclusion: H-001 至 H-056 的失败反例均已转绿；HEAD `bbbf1fc16` 通过 195 项 Python、全部离线集成门禁与最终空白复审，P0/P1 为 0。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
  - H-005
  - H-006
  - H-007
  - H-008
  - H-009
  - H-016
  - H-017
  - H-018
  - H-019
  - H-020
  - H-021
  - H-022
  - H-023
  - H-024
  - H-025
  - H-026
  - H-027
  - H-028
  - H-029
  - H-030
  - H-031
  - H-032
  - H-033
  - H-034
  - H-035
  - H-036
  - H-037
  - H-038
  - H-039
  - H-040
  - H-041
  - H-042
  - H-043
  - H-044
  - H-045
  - H-046
  - H-047
  - H-048
  - H-049
  - H-050
  - H-051
  - H-052
  - H-053
  - H-054
  - H-055
  - H-056
- Resolution basis:
  - satisfied by E-056 至 E-063 and final reviewer `019fbb4f-f64a-7ae0-ac4c-3c04c17140da`
- Close reason:
  - all blocking recovery, accounting, authorization and cleanup evidence gaps fixed; historical live_regression_failed remains an external baseline state

## Hypothesis H-001: observation 自报 arm 未被 artifact 身份约束
- Status: fixed
- Parent: P-001
- Claim: 校验器用 observation.arm 重算指标，却不验证 artifact.logical_mode 与 arm、run_id 的关系。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 同一 artifact 可被换标签后重复提交。
- Falsifiable predictions:
  - If true: 复制 Standard observation 并改成 map-request 可通过 validate_promotion。
  - If false: 校验器会因 logical_mode 或 artifact 路径身份不符拒绝。
- Diagnostic evidence plan:
  - Prediction or clause under test: 重复 artifact 能否冒充另一 arm。
  - Signal: validate_promotion 返回值。
  - Capture method: 离线复制 observation、改 arm/run_id，复用三个 artifact。
  - Event name or marker:
    - cache observation evidence mismatch
  - Correlation keys:
    - record_id
    - run_id
  - Differentiates from:
    - artifact 内容损坏
  - Supports if:
    - promotion 接受两臂。
  - Refutes if:
    - promotion 拒绝 arm 或 artifact 身份。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留为正式校验错误和回归测试。
- Evidence gate: satisfied
- Related evidence:
  - E-001
- Conclusion: fixed by artifact logical mode and durable run identity validation
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in cccb47004

## Hypothesis H-002: proposal 未绑定付费执行控制面
- Status: fixed
- Parent: P-001
- Claim: proposal 只绑定产品 surface，授权后 dirty runner 不参与重算。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - cache control-plane 分类只影响提交门，不自动进入 proposal identity。
- Falsifiable predictions:
  - If true: 修改 runner 后 validate_proposal_context 仍通过。
  - If false: 执行前会报告 execution manifest mismatch。
- Diagnostic evidence plan:
  - Prediction or clause under test: 授权后 runner 漂移是否进入 proposal equality。
  - Signal: proposal 重算结果。
  - Capture method: 临时仓库构造 proposal，修改 runner 后重新校验。
  - Event name or marker:
    - budget proposal does not match current evidence
  - Correlation keys:
    - proposal_id
  - Differentiates from:
    - 产品 surface 漂移
  - Supports if:
    - 旧实现仍通过。
  - Refutes if:
    - 修改 runner 或所选 sample 后失败。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - proposal 永久记录 execution manifest。
- Evidence gate: satisfied
- Related evidence:
  - E-002
- Conclusion: fixed by execution manifest and paid control-plane identity
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 2950b3802 and 76568a061

## Hypothesis H-003: Realtime conversation 使用独立 WebSocket client 绕过硬上限
- Status: fixed
- Parent: P-001
- Claim: Realtime conversation 直接 connect，未调用共享 claim，且保留 provider connect retry。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - none
- Rationale:
  - 该路径不经过 ModelClient 的普通 stream 方法。
- Falsifiable predictions:
  - If true: start_inner 的 direct 和 sideband connect 前均无 claim。
  - If false: 每个连接 dispatch 前会消费共享额度且 retry 为 0。
- Diagnostic evidence plan:
  - Prediction or clause under test: Realtime connect 是否经过共享预算。
  - Signal: 调用链和定向单测。
  - Capture method: 静态追踪 start_inner 到 RealtimeWebsocketClient，并测试 retry policy/claim。
  - Event name or marker:
    - provider_request_hard_limit_exceeded
  - Correlation keys:
    - provider request state path
  - Differentiates from:
    - WebRTC HTTP create call已计数
  - Supports if:
    - WebSocket connect 仍无 claim。
  - Refutes if:
    - direct 和 sideband 均先 claim，且隐藏 retry 禁用。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 复用现有 provider hard-limit 日志。
- Evidence gate: satisfied
- Related evidence:
  - E-003
- Conclusion: fixed at the actual response generation boundary
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in b58f2f9c4, ecd1e929b, 26c5a87fd and 84b94ceef

## Hypothesis H-004: 清理在直接子进程和首个空集后过早完成
- Status: fixed
- Parent: P-001
- Claim: subprocess timeout 未显式终止进程组，cleanup 首次查空即返回成功。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - descendant 可能在首个空集确认后才完成 docker run。
- Falsifiable predictions:
  - If true: mock 首次空集只调用一次 docker ps 就返回 verified_absent。
  - If false: 先终止进程树，并要求连续空集确认。
- Diagnostic evidence plan:
  - Prediction or clause under test: 首次空集是否立即成功。
  - Signal: docker 调用次数和 cleanup result。
  - Capture method: 离线 mock docker ps 的空、迟到容器、空序列。
  - Event name or marker:
    - stable_empty_polls
  - Correlation keys:
    - run_id
  - Differentiates from:
    - docker rm 失败
  - Supports if:
    - 第一次空集即成功。
  - Refutes if:
    - 迟到容器被发现并清理，三次连续空集后才成功。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - attempt 永久记录稳定空集确认次数。
- Evidence gate: satisfied
- Related evidence:
  - E-004
- Conclusion: fixed by process-tree ownership and stable resource absence checks
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 474597062, d620349f9, d08be480a and 123116c4a

## Hypothesis H-005: Agent 容器持有硬上限的凭据和可变计数状态
- Status: fixed
- Parent: P-001
- Claim: 真实 API Key 与共享计数文件都位于 Agent 可访问边界，shell 子进程可绕过 client claim 或重置计数。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-002
- Rationale:
  - 进程内硬上限无法约束同一容器中不经过 Whale client 的网络请求。
- Falsifiable predictions:
  - If true: Agent 能读取 secret mount，且可直接访问 provider 网络。
  - If false: Agent 只看到代理凭据、没有 provider 出网路径，真实 Key 和计数由独立监督边界持有。
- Diagnostic evidence plan:
  - Prediction or clause under test: Agent 是否能读取 Key、直连上游或让超额请求到达上游。
  - Signal: 容器 mount/network inspect、mock provider 收到的请求数和 Authorization。
  - Capture method: 离线双网络 Docker fixture 和固定上游代理测试。
  - Event name or marker:
    - provider_request_claimed
    - provider_request_rejected
  - Correlation keys:
    - run_id
    - side
  - Differentiates from:
    - Whale client 内部多进程共享计数
  - Supports if:
    - Agent 能看到 secret 或第二个超额请求到达 mock provider。
  - Refutes if:
    - Agent 只有 internal network，secret mount 缺失，超额请求由代理前置拒绝。
  - Instrumentation status: implemented
  - Instrumentation lifecycle:
    - 代理事件与边界清理结果作为永久运行 artifact。
- Evidence gate: satisfied
- Related evidence:
  - E-005
- Conclusion: fixed by a restricted and reconciled host-supervised Docker provider boundary
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in fac57d0d8, d620349f9 and e619890f1

## Hypothesis H-006: Realtime 的付费边界是每次生成而不是建连
- Status: fixed
- Parent: P-001
- Claim: 一条 Realtime 连接可发送多次 `response.create`，且 Server VAD 可自动触发生成，按建连计一次不能形成硬上限。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - H-003
- Rationale:
  - 连接生命周期与模型推理次数不是一一对应关系。
- Falsifiable predictions:
  - If true: hard limit=1 时连接后仍可触发多次生成，或 Server VAD 无客户端 claim 即生成。
  - If false: 每个显式 `response.create` 发送前 claim，无法前置计数的 conversational Realtime 在专项上限下 fail closed。
- Diagnostic evidence plan:
  - Prediction or clause under test: 显式和服务器自动生成路径是否都受前置硬边界约束。
  - Signal: 发送调用链、Server VAD `create_response` 配置和定向单测。
  - Capture method: 静态调用链与 `provider_request_hard_limit` Rust 测试。
  - Event name or marker:
    - provider_request_hard_limit_exceeded
  - Correlation keys:
    - realtime session
  - Differentiates from:
    - WebRTC 建连 HTTP 请求
  - Supports if:
    - claim 仍在 connect，或 conversational Server VAD 在 hard-limit 模式可启动。
  - Refutes if:
    - claim 位于 `send_create_now`，且自动生成会在建连前明确拒绝。
  - Instrumentation status: implemented
  - Instrumentation lifecycle:
    - 复用 provider hard-limit 错误日志。
- Evidence gate: satisfied
- Related evidence:
  - E-006
- Conclusion: fixed by per-inference claim and fail-closed automatic generation
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in ecd1e929b, 26c5a87fd and 84b94ceef

## Hypothesis H-007: Windows timeout 没有终止完整进程树
- Status: fixed
- Parent: P-001
- Claim: Windows 分支只 terminate/kill 父 `pwsh`，Docker 子进程可在清理确认后继续创建资源。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-004
- Rationale:
  - 父进程退出不代表后代退出。
- Falsifiable predictions:
  - If true: Windows timeout 只调用父进程 terminate。
  - If false: 使用操作系统 Job Object 整树所有权并拒绝无法证明的 fallback。
- Diagnostic evidence plan:
  - Prediction or clause under test: timeout 是否通过 Job Object 关闭整树，fallback 失败是否保持失败。
  - Signal: mock Job close、termination result 和 taskkill 非零反例。
  - Capture method: 平台分支离线单测。
  - Event name or marker:
    - process_tree_termination
  - Correlation keys:
    - run_id
  - Differentiates from:
    - Docker 标签后置清理
  - Supports if:
    - 只调用 `process.terminate()`。
  - Refutes if:
    - `KILL_ON_JOB_CLOSE` Job 被关闭并报告后代终止；taskkill 非零时即使父进程退出也不接受。
  - Instrumentation status: implemented
  - Instrumentation lifecycle:
    - attempt 保留整树终止结果。
- Evidence gate: satisfied
- Related evidence:
  - E-007
- Conclusion: fixed by Windows Job Object process-tree ownership
- Repair design readiness: implemented
- Next step: Windows 实机可作为非阻断环境复验
- Blocker:
  - none
- Close reason:
  - fixed in d08be480a; Windows real-host validation remains a non-blocking portability check

## Hypothesis H-008: provider boundary 可被当作任意凭据代理且未与正式证据对账
- Status: fixed
- Parent: P-001
- Claim: Agent 可直接调用代理的任意 method/path/model；代理真实 dispatch 未进入正式 observation，可能产生账外请求。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-005
- Rationale:
  - 隔离真实 Key 和统一计数只限制数量，不能证明请求属于获批模型或 Whale 本身。
- Falsifiable predictions:
  - If true: 任意 endpoint/model 可到达上游，或额外合法 `curl` 不影响正式结算。
  - If false: 代理只接受批准合同，且每个真实 dispatch 必须与 Whale wire payload 摘要逐条配对。
- Diagnostic evidence plan:
  - Prediction or clause under test: 非批准请求和账外合法请求能否进入上游或结算。
  - Signal: mock upstream 请求数、boundary event 与 wire trace 对账状态。
  - Capture method: 代理单测、对账器反例和正式 analyzer 负向测试。
  - Event name or marker:
    - provider_request_contract_rejected
    - provider_dispatch_trace_mismatch
  - Correlation keys:
    - boundary request count
    - provider payload SHA-256
  - Differentiates from:
    - 单纯请求数硬上限
  - Supports if:
    - 任意请求可转发或多出的 boundary event 仍可晋升。
  - Refutes if:
    - 非批准 method/path/model 在 claim 前拒绝，摘要序列不一致导致正式 artifact 拒绝。
  - Instrumentation status: implemented
  - Instrumentation lifecycle:
    - boundary event 和 reconciliation artifact 永久保留。
- Evidence gate: satisfied
- Related evidence:
  - E-008
  - E-009
- Conclusion: fixed by restricted proxy contract; all approved boundary dispatches are allowed and authoritatively counted, while wire reconciliation controls evidence eligibility rather than Agent intent
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in e619890f1 and 040c27ae6

## Hypothesis H-009: 强杀绕过 PowerShell finally 后会留下 host API Key 文件
- Status: fixed
- Parent: P-001
- Claim: provider secret 写在 host artifact 邻近目录，外层只清理 Docker 资源，强杀后可留下真实 Key。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-004
  - H-005
- Rationale:
  - PowerShell finally 不是操作系统强杀后的可靠清理边界。
- Falsifiable predictions:
  - If true: 构造残留 `.container-secrets/deepseek-*.secret` 后 cleanup_verified 仍为真。
  - If false: 外层 supervisor 精确覆盖清除本 run secret，且 secret 状态是成功结算必要条件。
- Diagnostic evidence plan:
  - Prediction or clause under test: host secret 是否参与外层清理证明。
  - Signal: 文件存在性、secret_cleanup_status、cleanup_verified。
  - Capture method: 临时 run 目录残留 secret 反例测试。
  - Event name or marker:
    - secret_cleanup_status
  - Correlation keys:
    - run_id
  - Differentiates from:
    - 容器 secret mount 是否可见
  - Supports if:
    - Docker 清理成功后 secret 仍存在或不影响结算。
  - Refutes if:
    - 文件被覆盖删除，目录为空，缺失/失败状态阻断结算。
  - Instrumentation status: implemented
  - Instrumentation lifecycle:
    - attempt 永久记录 secret cleanup 状态和路径。
- Evidence gate: satisfied
- Related evidence:
  - E-010
- Conclusion: fixed by outer paid-run secret erasure and verification
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 123116c4a

## Hypothesis H-010: V2 transcription Realtime 仍可在硬限额下产生未计数推理
- Status: fixed
- Parent: P-001
- Claim: 只拒绝 conversational Realtime 会错误放行 V2 transcription，但其服务端音频推理同样没有进入 HTTP 请求 claim。
- Falsifiable predictions:
  - If true: hard limit 存在时 V2 transcription helper 返回成功。
  - If false: 专项硬限额拒绝全部 Realtime，普通无硬限额运行不受影响。
- Diagnostic evidence plan:
  - Signal: `ensure_realtime_session_is_metered` 分支及定向 Rust 测试。
  - Supports if: transcription 返回 `Ok`。
  - Refutes if: V1/V2 的 conversational/transcription 均返回错误。
- Evidence gate: satisfied
- Related evidence:
  - E-011
- Conclusion: fixed by fail-closed handling for every Realtime mode under the paid-run hard limit
- Close reason:
  - fixed in 0d3af4b54

## Hypothesis H-011: 账本锁模块在 Windows 无法导入
- Status: fixed
- Parent: P-001
- Claim: 顶层无条件导入 POSIX-only `fcntl`，导致 Windows 运行器在启动前失败。
- Falsifiable predictions:
  - If true: Windows Python 导入即抛 `ModuleNotFoundError`。
  - If false: POSIX 使用 `flock`，Windows 使用 `msvcrt.locking`，共同保护同一 lockfile 字节。
- Diagnostic evidence plan:
  - Signal: 平台导入分支与 Windows backend mock。
  - Supports if: 仍存在无条件 `fcntl` import。
  - Refutes if: backend 按平台选择且 lock/unlock 操作成对。
- Evidence gate: satisfied
- Related evidence:
  - E-012
- Conclusion: fixed with explicit POSIX and Windows lock backends
- Close reason:
  - fixed in 837460b75

## Hypothesis H-012: Windows Job 分配前存在子进程逃逸窗口
- Status: fixed
- Parent: P-001
- Claim: `Popen` 先启动 PowerShell，再调用 `AssignProcessToJobObject`；分配前它可创建不属于 Job 的 Docker 后代。
- Falsifiable predictions:
  - If true: 创建顺序为 run -> assign，assign 失败仍可能声称整树终止。
  - If false: 顺序严格为 create_suspended -> assign -> resume，assign 失败直接终止挂起父进程。
- Diagnostic evidence plan:
  - Signal: Win32 调用顺序和 assign 失败反例。
  - Supports if: `Popen` 返回后才创建/分配 Job。
  - Refutes if: 恢复线程前已取得 Job 所有权，失败路径不执行 resume。
- Evidence gate: satisfied
- Related evidence:
  - E-013
- Conclusion: fixed with suspended process creation and pre-resume Job assignment
- Close reason:
  - fixed in 040c27ae6

## Hypothesis H-013: baseline 晋升忽略网络和 secret 清理失败
- Status: fixed
- Parent: P-001
- Claim: promotion 只检查容器状态和三次空轮询，未复用 runtime 的完整 cleanup contract。
- Falsifiable predictions:
  - If true: 缺少或失败的 network/secret 状态仍可晋升。
  - If false: promotion 直接调用 `cleanup_verified`，任一维度缺失或失败均拒绝。
- Diagnostic evidence plan:
  - Signal: promotion 负向 fixture。
  - Supports if: 删除 `network_cleanup_status` 后仍通过。
  - Refutes if: network/secret 缺失与失败均报告 failed attempt。
- Evidence gate: satisfied
- Related evidence:
  - E-014
- Conclusion: fixed by sharing the complete cleanup predicate
- Close reason:
  - fixed in 837460b75

## Hypothesis H-014: evidence 失败会把已发生 provider 请求结算为零
- Status: fixed
- Parent: P-001
- Claim: ledger 从 Agent usage observation 汇总请求数；wire 对账失败时 observation 缺失，物理请求被记为 0。
- Falsifiable predictions:
  - If true: boundary 有三次 claim、usage 失败时 `api_requests=0`。
  - If false: Agent 外 supervisor boundary 计数独立持久化；对账失败仍记三次，缺失时写 null 和已知最小值。
- Diagnostic evidence plan:
  - Signal: mismatch boundary fixture、持久 attempt 字段和 ledger settlement。
  - Supports if: 请求数依赖 observation 是否成功。
  - Refutes if: authority count 独立于 wire/token evidence，promotion 仍要求二者一致。
- Evidence gate: satisfied
- Related evidence:
  - E-015
- Conclusion: fixed by separating authoritative request accounting from performance and token evidence
- Close reason:
  - fixed in 040c27ae6

## Hypothesis H-015: Ctrl-C 清理失败仍被记录为正常取消
- Status: fixed
- Parent: P-001
- Claim: KeyboardInterrupt 路径记录 cleanup 结果后直接返回 cancelled/130，不检查资源是否真的清空。
- Falsifiable predictions:
  - If true: cleanup status=failed 时结果仍为 cancelled。
  - If false: 只有完整 cleanup proof 才 cancelled；否则 failed/3 并保留失败原因。
- Diagnostic evidence plan:
  - Signal: unverified cleanup 中断 fixture。
  - Supports if: cleanup 失败仍返回 130。
  - Refutes if: 返回 3、stop_reason=cancelled_cleanup_failed。
- Evidence gate: satisfied
- Related evidence:
  - E-016
- Conclusion: fixed by making cleanup proof part of cancellation settlement
- Close reason:
  - fixed in 040c27ae6

## Hypothesis H-016: cleanup 的成功标签未与残留对象和错误字段交叉校验
- Status: fixed
- Parent: P-001
- Claim: `cleanup_verified` 只检查状态枚举和轮询次数，矛盾的非空 container/network/secret 列表或 error 不影响晋升。
- Falsifiable predictions:
  - If true: `verified_absent` 携带残留 ID、secret path 或 error 时仍返回 true。
  - If false: 任一成功状态与残留证据矛盾时均拒绝。
- Diagnostic evidence plan:
  - Signal: 直接调用共享 cleanup predicate，并用 promotion fixture 复核。
  - Supports if: 矛盾 fixture 被接受。
  - Refutes if: 共享 predicate 和 promotion 均拒绝。
- Evidence gate: satisfied
- Related evidence:
  - E-017
- Conclusion: fixed by one shared cleanup proof predicate
- Close reason:
  - fixed in ec66423ca

## Hypothesis H-017: elapsed 硬上限没有从批准预算独立复算
- Status: fixed
- Parent: P-001
- Claim: observation 与 attempt 可同时报告超过 `per_sample_run_limits.elapsed_seconds` 的耗时，并用空 exceeded 列表通过晋升。
- Falsifiable predictions:
  - If true: 超时 fixture 只要自报 `budget_observation_exceeded=[]` 即可晋升。
  - If false: validator 直接比较批准上限并拒绝。
- Diagnostic evidence plan:
  - Signal: promotion 负向 fixture，不手工填 exceeded 列表。
  - Supports if: 超过批准 elapsed 仍通过。
  - Refutes if: 独立复算报告 elapsed 超限。
- Evidence gate: satisfied
- Related evidence:
  - E-018
- Conclusion: fixed by independent attempt limit validation
- Close reason:
  - fixed in ec66423ca

## Hypothesis H-018: provider 并发 claim 与事件持久化顺序可发生倒置
- Status: fixed
- Parent: P-001
- Claim: count 锁和 event 锁分离，使 `count=2` 可能先于 `count=1` 写入，破坏权威计数证据可用性。
- Falsifiable predictions:
  - If true: 强制延迟第一个 claim 后可观察事件顺序 2、1，verifier 拒绝。
  - If false: claim 和对应 claimed event 以同一临界区按计数顺序提交。
- Diagnostic evidence plan:
  - Signal: 并发代理单测和生成的 events.jsonl。
  - Supports if: 文件顺序与 count 顺序不一致。
  - Refutes if: 并发 fixture 始终得到 1、2，硬上限仍准确。
- Evidence gate: satisfied
- Related evidence:
  - E-019
- Conclusion: fixed by atomically committing count and claimed event
- Close reason:
  - fixed in ec66423ca

## Hypothesis H-019: Windows Job 异常路径未保持整树终止后置条件
- Status: fixed
- Parent: P-001
- Claim: Win32 wait、exit-code、resume、terminate 或 handle close 失败时，Job/线程 handle 可能泄漏，且后代终止未经确认。
- Falsifiable predictions:
  - If true: 注入各 Win32 API 失败后存在未关闭 handle 或未验证 terminate wait 的路径。
  - If false: 所有异常路径都关闭 thread/process/job，且终止失败明确 fail closed。
- Diagnostic evidence plan:
  - Signal: mock Win32 API failure matrix 与调用顺序。
  - Supports if: 任一失败路径跳过 close/wait 或提前丢弃 handle。
  - Refutes if: 每条路径均给出可验证的资源所有权结论。
- Evidence gate: satisfied
- Related evidence:
  - E-020
- Conclusion: fixed by preserving handles and failing closed on unconfirmed cleanup
- Close reason:
  - fixed in ec66423ca and 809e1d513

## Hypothesis H-020: 中断保护未覆盖清理、证据持久化和账本结算
- Status: fixed
- Parent: P-001
- Claim: benchmark wait 之后的 cleanup 或 evidence 阶段抛出 KeyboardInterrupt/OSError，会越过最终 result 与 ledger settlement。
- Falsifiable predictions:
  - If true: 在 cleanup 或 evidence 注入异常后 ledger 保持 running，且无失败 result。
  - If false: 外层监督 finally 总会再次清理并结算失败证据。
- Diagnostic evidence plan:
  - Signal: cleanup/evidence/settlement 阶段异常注入测试。
  - Supports if: running ledger 或资源清理未执行。
  - Refutes if: 每个注入点均产生 failed settlement 和 cleanup proof。
- Evidence gate: satisfied
- Related evidence:
  - E-021
- Conclusion: fixed by supervised cleanup and retryable final settlement
- Close reason:
  - fixed in ec66423ca and 809e1d513

## Hypothesis H-021: promotion 只检查费用状态而不复算金额
- Status: fixed
- Parent: P-001
- Claim: settled ledger 中任意 monetary amount/components/formula/pricing snapshot 只要 status=estimated 就能晋升。
- Falsifiable predictions:
  - If true: 篡改金额或价格组成后 promotion 仍通过。
  - If false: validator 从批准 proposal 与 token/request totals 重算并逐字段比较。
- Diagnostic evidence plan:
  - Signal: promotion monetary-cost mutation fixtures。
  - Supports if: 篡改后仍晋升。
  - Refutes if: 任一费用字段漂移均拒绝。
- Evidence gate: satisfied
- Related evidence:
  - E-022
- Conclusion: fixed by shared exact cost recomputation
- Close reason:
  - fixed in ec66423ca

## Hypothesis H-022: 空 secret 目录被错误标记为已删除秘密
- Status: fixed
- Parent: P-001
- Claim: `_cleanup_run_secrets` 把只删除空目录也记为 `removed_verified`，但证明中没有被删除的 secret path，导致正常清理被共享 predicate 拒绝。
- Falsifiable predictions:
  - If true: 只存在空 `.container-secrets` 目录时返回 `removed_verified` 和空 `secret_paths`。
  - If false: 无秘密材料时返回 `verified_absent`。
- Diagnostic evidence plan:
  - Signal: 空目录清理单测的状态和路径列表。
  - Supports if: 状态与证据自相矛盾。
  - Refutes if: 状态准确表达秘密材料始终不存在。
- Evidence gate: satisfied
- Related evidence:
  - E-023
- Conclusion: fixed by separating empty directory metadata from removed secret material
- Close reason:
  - fixed in 809e1d513

## Hypothesis H-023: Windows Job close 失败后没有显式整树终止兜底
- Status: fixed
- Parent: P-001
- Claim: `CloseHandle(job)` 失败时直接返回失败，没有调用 `TerminateJobObject`，因此无法证明后代进程已停止。
- Falsifiable predictions:
  - If true: 注入 job close 失败后 terminate 未调用且 descendants proof 为 false。
  - If false: close 失败会显式 terminate、等待并再次释放 handle。
- Diagnostic evidence plan:
  - Signal: Win32 mock 调用顺序与 termination result。
  - Supports if: close 异常后直接退出。
  - Refutes if: 显式整树终止兜底建立后置条件。
- Evidence gate: satisfied
- Related evidence:
  - E-024
- Conclusion: fixed with explicit TerminateJobObject fallback and wait proof
- Close reason:
  - fixed in 809e1d513

## Hypothesis H-024: 最终聚合阶段仍在结算保护区之外
- Status: fixed
- Parent: P-001
- Claim: attempts 循环结束后、`persist_final_settlement` 之前发生中断时，结果文件缺失且 ledger 保持 running。
- Falsifiable predictions:
  - If true: 在 evidence digest 聚合处注入 KeyboardInterrupt 会越过 settlement。
  - If false: 聚合失败会生成 failed result 并结算 ledger。
- Diagnostic evidence plan:
  - Signal: result 文件和 ledger 最终状态。
  - Supports if: main 抛出且 ledger 未结算。
  - Refutes if: main 返回失败码且两份证据均落盘。
- Evidence gate: satisfied
- Related evidence:
  - E-025
- Conclusion: fixed by fail-closed mechanical result aggregation
- Close reason:
  - fixed in 809e1d513

## Hypothesis H-025: 总耗时可与 attempt 总和和时间戳矛盾
- Status: fixed
- Parent: P-001
- Claim: promotion 只限制每个 attempt，没有校验 result elapsed、attempt 总和及 started/ended 时间差的一致性。
- Falsifiable predictions:
  - If true: 两次各 1 秒的 attempt 可配合总耗时 0 秒通过。
  - If false: validator 独立拒绝总耗时证据矛盾。
- Diagnostic evidence plan:
  - Signal: promotion 负向 fixture。
  - Supports if: 矛盾耗时仍晋升。
  - Refutes if: 报告 elapsed evidence mismatch。
- Evidence gate: satisfied
- Related evidence:
  - E-026
- Conclusion: fixed by checking total bound, attempt sum and timestamp delta
- Close reason:
  - fixed in 809e1d513

## Hypothesis H-026: Windows 挂起创建调用未被异常保护完整包围
- Status: fixed
- Parent: P-001
- Claim: `CreateProcessW` 成功写入 handles 后、Python 接收返回值前的异步中断会绕过清理块，泄漏挂起进程和 handles。
- Falsifiable predictions:
  - If true: mock 在写入 PROCESS_INFORMATION 后抛中断，terminate/close 均未调用。
  - If false: 以 PROCESS_INFORMATION handles 为所有权事实完成清理。
- Diagnostic evidence plan:
  - Signal: TerminateProcess、WaitForSingleObject 和 CloseHandle 调用。
  - Supports if: 中断直接传播且无清理。
  - Refutes if: 挂起进程被终止确认，所有 handles 被关闭。
- Evidence gate: satisfied
- Related evidence:
  - E-027
- Conclusion: fixed by treating populated native handles as the ownership fact
- Close reason:
  - fixed in 809e1d513

## Hypothesis H-027: 未确认终止时关闭了最后的 process handle
- Status: fixed
- Parent: P-001
- Claim: Job assign 与 TerminateProcess 同时失败后，清理仍关闭 process handle，使挂起进程失去后续终止能力。
- Falsifiable predictions:
  - If true: 失败 fixture 中 process handle 出现在 CloseHandle 调用列表。
  - If false: 只有终止得到确认后才关闭 process handle；失败时保留所有权并继续重试。
- Diagnostic evidence plan:
  - Signal: terminate/wait/close 调用顺序。
  - Supports if: 未确认终止仍释放 handle。
  - Refutes if: 重试后建立终止证明再释放。
- Evidence gate: satisfied
- Related evidence:
  - E-029
- Conclusion: fixed by direct termination retries, taskkill fallback and retained handle on unconfirmed termination
- Close reason:
  - fixed in 8bd820a9a

## Hypothesis H-028: result 聚合与 ledger settlement 不是一个监督事务
- Status: fixed
- Parent: P-001
- Claim: 聚合后的时间戳写入和 helper 间调用窗口仍可被中断，留下 running ledger。
- Falsifiable predictions:
  - If true: 在 `now()` 或 settlement 调用边界注入中断时没有 failed result。
  - If false: 单一监督入口捕获聚合和持久化全过程并落盘失败结论。
- Diagnostic evidence plan:
  - Signal: result 文件和 ledger 状态。
  - Supports if: result 缺失且 ledger=running。
  - Refutes if: result/ledger 均为 failed。
- Evidence gate: satisfied
- Related evidence:
  - E-030
- Conclusion: fixed by one finalize-and-persist supervision transaction
- Close reason:
  - fixed in 8bd820a9a

## Hypothesis H-029: JSON bool 可冒充 elapsed 数值
- Status: fixed
- Parent: P-001
- Claim: Python 的 bool/int 继承关系让 true/false 通过 attempt 与总耗时合同。
- Falsifiable predictions:
  - If true: 完整 promotion 接受布尔 elapsed。
  - If false: 只接受非 bool、有限、非负的 int/float。
- Diagnostic evidence plan:
  - Signal: 完整 promotion mutation fixture。
  - Supports if: bool 证据被接受。
  - Refutes if: validator 报 elapsed 类型错误。
- Evidence gate: satisfied
- Related evidence:
  - E-031
- Conclusion: fixed by finite non-bool numeric validation shared by attempts and totals
- Close reason:
  - fixed in 8bd820a9a

## Hypothesis H-030: Windows Job/process handle 所有权仍有异常泄漏
- Status: fixed
- Parent: P-001
- Claim: Job 创建后的准备阶段中断、returncode 设置顺序及 terminate fallback 失败都可能跳过可重试的 handle release。
- Falsifiable predictions:
  - If true: configure 中断不 close Job；process close 失败后 wait 不再重试；fallback 异常无最终 release。
  - If false: owner 从创建后立即进入保护区，状态只有在 handle 释放成功后提交。
- Diagnostic evidence plan:
  - Signal: 三组异常注入的 close 次数和对象状态。
  - Supports if: 任一 handle 丢失或不可重试。
  - Refutes if: 每条路径都保持可重试所有权并最终释放。
- Evidence gate: satisfied
- Related evidence:
  - E-032
- Conclusion: fixed by immediate protection, retryable process close and final handle-release retries
- Close reason:
  - fixed in 8bd820a9a

## Hypothesis H-031: ledger claim 后的 attempt 前置阶段不受最终结算保护
- Status: fixed
- Parent: P-001
- Claim: `store_entry` 后、attempt 内层 try 前发生中断会越过 final settlement。
- Evidence gate: satisfied
- Related evidence:
  - E-034
- Conclusion: durable claim is detected and settled from the outer finally boundary
- Close reason:
  - fixed in 3410db334

## Hypothesis H-032: Windows 异常清理可被第二次中断
- Status: fixed
- Parent: P-001
- Claim: assignment 失败后的 cleanup 抛 BaseException 时，后续 Job 与 handles 释放被跳过。
- Evidence gate: satisfied
- Related evidence:
  - E-035
- Conclusion: nested cleanup failures are recorded while Job and handle release continues
- Close reason:
  - fixed in 3410db334 and dc1faeecd

## Hypothesis H-033: 非标准 JSON 与宽松类型转换可伪造成功证据
- Status: fixed
- Parent: P-001
- Claim: false exit code、NaN business success 和 Infinity trace coverage 可通过完整 promotion。
- Evidence gate: satisfied
- Related evidence:
  - E-036
- Conclusion: strict JSON parsing, finite numeric checks and exact boolean/integer contracts reject all three mutations
- Close reason:
  - fixed in 3410db334 and 3b291b111

## Hypothesis H-034: arm 身份只由可编辑 metrics 标签证明
- Status: fixed
- Parent: P-001
- Claim: 未持久化实际 Whale argv 与 pair mode map，Standard artifact 可改标签冒充 TaskSpace。
- Evidence gate: satisfied
- Related evidence:
  - E-037
- Conclusion: durable Whale argv and logical mode map now prove the selected arm independently of metrics labels
- Close reason:
  - fixed in 3410db334

## Hypothesis H-035: 双重终止失败后的 process handle 不可达
- Status: fixed
- Parent: P-001
- Claim: helper 不 close handle 但也不把 owner 交给可重试监督对象，形成主动泄漏。
- Evidence gate: satisfied
- Related evidence:
  - E-038
- Conclusion: failed termination retains a retryable supervisor owner and blocks the next paid process until cleanup succeeds
- Close reason:
  - fixed in 3410db334 and dc1faeecd

## Hypothesis H-036: network removed 状态没有 post-remove 复查
- Status: fixed
- Parent: P-001
- Claim: docker network rm 返回 0 即被视为 verified，没有再次查询残留。
- Evidence gate: satisfied
- Related evidence:
  - E-039
- Conclusion: network cleanup now requires three consecutive empty post-remove observations and catches late networks
- Close reason:
  - fixed in 3410db334

## Hypothesis H-037: 不同 arm 可共享同一份 provider wire 证据
- Status: fixed
- Parent: P-001
- Claim: arm-specific argv 只证明启动配置，未阻止 Standard 与 TaskSpace observation 复用完全相同的 provider boundary。
- Evidence gate: satisfied
- Related evidence:
  - E-041
- Conclusion: full promotion rejects identical provider wire evidence across different arms
- Close reason:
  - fixed in a23b29cb6

## Hypothesis H-038: 三种 TaskSpace projection policy 被合并成 map-request
- Status: fixed
- Parent: P-001
- Claim: 非 Standard arm 的身份校验硬编码 map-request，导致其他 policy 被误拒绝且可被重标。
- Evidence gate: satisfied
- Related evidence:
  - E-042
- Conclusion: each TaskSpace arm now derives and validates its own exact projection policy argv
- Close reason:
  - fixed in a23b29cb6

## Hypothesis H-039: Windows owner 只在当前解释器内存中可达
- Status: fixed
- Parent: P-001
- Claim: 未确认终止的挂起进程只有内存 handle owner；当前矩阵停止并退出后，没有跨进程恢复入口。
- Evidence gate: satisfied
- Related evidence:
  - E-043
- Conclusion: suspended pre-Job ownership is journaled with PID and creation time and recovered before every future paid launch
- Close reason:
  - fixed in 9204926c2 and 13022a905

## Hypothesis H-040: recovery 在锁外读取并可覆盖活跃结算
- Status: fixed
- Parent: P-001
- Claim: recovery 先读旧 ledger、后无条件 store，可覆盖期间形成的最终状态，且输入使用宽松 JSON。
- Evidence gate: satisfied
- Related evidence:
  - E-044
- Conclusion: recovery now validates strict result evidence and performs compare-and-set under the ledger lock
- Close reason:
  - fixed in 1ba6c1232

## Hypothesis H-041: Python 布尔值可冒充正式整数证据
- Status: fixed
- Parent: P-001
- Claim: attempt exit code、ledger repeat/exit/token 只比较数值，`true/false` 可利用 Python 的整数兼容性通过晋升。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-033
- Rationale:
  - 严格 JSON 只保证语法标准，不会自动消除 Python `bool` 与 `int` 的继承关系。
- Evidence gate: satisfied
- Related evidence:
  - E-046
- Conclusion: fixed by exact type-and-value validation at every promotion integer boundary
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 72566a1b6

## Hypothesis H-042: Windows 创建成功到 journal 落盘之间存在无 owner 窗口
- Status: fixed
- Parent: P-001
- Claim: 两阶段 `CreateProcessW -> AssignProcessToJobObject` 仍允许进程级硬退出发生在创建成功后、Job 绑定和 journal 之前。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-039
- Rationale:
  - Python `try/finally` 无法处理进程被立即终止，正确性必须在 Win32 创建原语内建立。
- Evidence gate: satisfied
- Related evidence:
  - E-047
- Conclusion: fixed by assigning the process to the kill-on-close Job in CreateProcessW itself
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in a3344da1d

## Hypothesis H-043: durable recovery 会覆盖同 PID 的既有 handle owner
- Status: fixed
- Parent: P-001
- Claim: recovery 新开 process handle 后，cleanup 以 PID 为键弹出旧 owner，旧 process/thread handle 未关闭即失去引用。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-039
- Rationale:
  - PID 是进程身份，不等于单个 handle 所有权；恢复不能以新 handle 覆盖尚未释放的旧 owner。
- Evidence gate: satisfied
- Related evidence:
  - E-048
- Conclusion: fixed by releasing retained in-process ownership before durable PID recovery
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in a3344da1d

## Hypothesis H-044: ledger recovery 未绑定原 durable claim
- Status: fixed
- Parent: P-001
- Claim: 同 `record_id` 的结果可扩大原授权矩阵、预算或替换 proposal/authorization 后被 recovery 结算。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-031
- Evidence gate: satisfied
- Related evidence:
  - E-050
- Conclusion: recovery now validates durable claim identity and budget under the ledger lock
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 1042384ff

## Hypothesis H-045: partial durable result 不能完成恢复结算
- Status: fixed
- Parent: P-001
- Claim: runner 合法生成 `partial` 和退出码 3，但共享 envelope 不接受该状态。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-028
- Evidence gate: satisfied
- Related evidence:
  - E-051
- Conclusion: partial is a valid non-success durable result and settles with truthful minimum accounting
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 5821b3354 and 1042384ff

## Hypothesis H-046: proposal repeat 仍允许布尔值冒充整数
- Status: fixed
- Parent: P-001
- Claim: 末端 ledger 已做精确类型，但 proposal validator 仍把 `true` 当作 repeat=1。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-041
- Evidence gate: satisfied
- Related evidence:
  - E-052
- Conclusion: budget scalars and matrix identities now require exact integer types
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 9ae528efb

## Hypothesis H-047: truthful unknown request count 与全局账本合同冲突
- Status: fixed
- Parent: P-001
- Claim: 结算正确写出 `api_requests=null`，但 schema 和 PowerShell 校验器仍强制整数。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-025
- Evidence gate: satisfied
- Related evidence:
  - E-053
- Conclusion: global ledger contract now distinguishes exact count from known minimum
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 5821b3354

## Hypothesis H-048: 正式授权 JSON 接受重复 key
- Status: fixed
- Parent: P-001
- Claim: 标为 strict 的 JSON decoder 只拒绝 NaN/Infinity，重复授权字段会静默采用最后一个值。
- Layer: root-cause
- Factor relation: part_of
- Depends on:
  - H-033
- Evidence gate: satisfied
- Related evidence:
  - E-054
- Conclusion: durable cache evidence rejects duplicate object keys at every nesting level
- Repair design readiness: implemented
- Next step: none
- Blocker:
  - none
- Close reason:
  - fixed in 9ae528efb

## Hypothesis H-049: recovery 的嵌套比较允许 bool 冒充整数
- Status: fixed
- Parent: P-001
- Claim: Python 原生相等会把 `true` 与 `1` 视为相等，污染 scope、matrix 与 observation 定位。
- Evidence gate: satisfied
- Related evidence: E-056
- Conclusion: 全部结构化身份比较复用 `exact_json_equal`。
- Close reason: fixed in 8083f31ab

## Hypothesis H-050: 请求精确值与下限合同存在双重事实
- Status: fixed
- Parent: P-001
- Claim: Schema 与 PowerShell 同时要求 exact 和 minimum，且无法一致表达 partial/unavailable。
- Evidence gate: satisfied
- Related evidence: E-057
- Conclusion: exact 状态只保存 `api_requests`；inexact 状态只保存 `null + minimum`。
- Close reason: fixed in 79f1c1d8c

## Hypothesis H-051: 自洽污染的 selection 可绕过 recovery
- Status: fixed
- Parent: P-001
- Claim: strict equality 不能拒绝 claim 和 result 同时使用布尔计数字段。
- Evidence gate: satisfied
- Related evidence: E-058
- Conclusion: proposal 与 recovery 复用唯一 selection 合同和矩阵恒等式。
- Close reason: fixed in 650657a1d

## Hypothesis H-052: result 请求汇总字段可自我授权
- Status: fixed
- Parent: P-001
- Claim: attempts 正确时，非法 minimum/status 仍可进入 recovery 和 promotion。
- Evidence gate: satisfied
- Related evidence: E-059
- Conclusion: producer、ledger、recovery、promotion 复用同一机械请求汇总。
- Close reason: fixed in b49765f47

## Hypothesis H-053: completed recovery 未验证 attempt 与 token 完整性
- Status: fixed
- Parent: P-001
- Claim: 失败 attempt 或错误 input 恒等式可被结算为完整费用。
- Evidence gate: satisfied
- Related evidence: E-060
- Conclusion: completed result 统一校验 attempt、cleanup、scope、request 与 token 恒等式。
- Close reason: fixed in ad6df97d7

## Hypothesis H-054: unsettled 恢复会降低已知请求下限
- Status: fixed
- Parent: P-001
- Claim: `mark_unsettled` 忽略已有 minimum 并固定写回 0/unavailable。
- Evidence gate: satisfied
- Related evidence: E-060
- Conclusion: 恢复按 exact/minimum 最大已知值单调保留 partial 证据。
- Close reason: fixed in ad6df97d7

## Hypothesis H-055: direct settlement 未绑定批准矩阵
- Status: fixed
- Parent: P-001
- Claim: attempt 与 observation 可同步声明缺失或越权 scope 并获得完整结算。
- Evidence gate: satisfied
- Related evidence: E-061
- Conclusion: direct settlement 从 durable approved selection 构造唯一矩阵并精确匹配。
- Close reason: fixed in bbbf1fc16

## Hypothesis H-056: 请求下限检查点晚于证据复制
- Status: fixed
- Parent: P-001
- Claim: 请求数已解析后复制或哈希失败，会在 ledger 中低报为 0。
- Evidence gate: satisfied
- Related evidence: E-062
- Conclusion: request count 先写 attempt 和原子 ledger checkpoint，再复制证据并复核计数未变。
- Close reason: fixed in bbbf1fc16

## Evidence E-001: 重复 Standard artifact 被正式校验器接受为两臂
- Related hypotheses:
  - H-001
- Direction: supports
- Type: reproduction
- Source: fresh subagent 019fba13-f2d9-7400-9634-8cc688d320c3
- Prediction or plan link:
  - H-001 重复 artifact 预测
- Matched signal:
  - validate_promotion 接受 ['standard', 'map-request']
- Correlation keys:
  - HEAD 0ac6e78b2
- Raw content:
  ```text
  复制 standard observation，改成 arm=map-request 和新 run_id，保留相同三个 artifact；validate_promotion 通过。
  ```
- Interpretation: observation arm 与 artifact 真实模式没有闭合。
- Time: 2026-08-01 14:05

## Evidence E-002: proposal 重算不包含 control-plane snapshot
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: cache_budget.py:175; cache_run_contract.py:34; cache_surface.py:244
- Prediction or plan link:
  - H-002 runner 漂移预测
- Matched signal:
  - proposal 仅记录 product surface_sha256
- Correlation keys:
  - proposal_id
- Raw content:
  ```text
  build_budget_proposal 未记录 runner/container/scenario manifest；validate_proposal_context 仅按同一字段重建 proposal。
  ```
- Interpretation: 授权后未提交 runner 变更不改变已授权身份。
- Time: 2026-08-01 14:05

## Evidence E-003: Realtime WebSocket 独立连接路径没有 claim
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: core/src/realtime_conversation.rs:287; codex-api realtime_websocket/methods.rs:577
- Prediction or plan link:
  - H-003 WebSocket 绕过预测
- Matched signal:
  - RealtimeWebsocketClient.connect 直接 dispatch 并保留 retry
- Correlation keys:
  - provider hard-limit state
- Raw content:
  ```text
  start_inner 构造 RealtimeWebsocketClient 后直接 connect/connect_webrtc_sideband；路径中无 provider_request_hard_limit.claim。
  ```
- Interpretation: 当前专项硬上限未覆盖该网络出口。
- Time: 2026-08-01 14:05

## Evidence E-004: 首次空集即返回 verified_absent
- Related hypotheses:
  - H-004
- Direction: supports
- Type: reproduction
- Source: fresh subagent mock probe；run_cache_hit_regression.py:83
- Prediction or plan link:
  - H-004 首次空集预测
- Matched signal:
  - docker_calls=1
- Correlation keys:
  - run_id
- Raw content:
  ```text
  第一次 docker ps 返回空集时 cleanup_status=verified_absent，docker_calls=1。
  ```
- Interpretation: 清理成功不是稳定后置条件。
- Time: 2026-08-01 14:05

## Evidence E-005: 隔离代理阻止 Agent 取得真实凭据或越过请求上限
- Related hypotheses:
  - H-005
- Direction: refutes
- Type: deterministic-test
- Source: `scripts/taskspace-benchmark/test-provider-boundary.ps1`、`docker/test_provider_boundary_proxy.py`
- Prediction or plan link:
  - H-005 Agent 边界反例
- Matched signal:
  - Agent inspect 无 secret mount，仅一个 internal network；第二个请求返回 429 且 mock upstream 未收到。
- Correlation keys:
  - provider-boundary-agent-selftest
- Raw content:
  ```text
  provider boundary tests passed
  provider proxy and reconciliation tests: 4 passed
  ```
- Interpretation: 真实 Key 和计数已移出 Agent 可支配边界。
- Time: 2026-08-01 18:20

## Evidence E-006: Realtime 显式与自动生成路径均受硬边界约束
- Related hypotheses:
  - H-006
- Direction: refutes
- Type: deterministic-test
- Source: `core/src/realtime_conversation.rs`、`core/src/client_tests.rs`
- Prediction or plan link:
  - H-006 逐次生成与 Server VAD 反例
- Matched signal:
  - `send_create_now` 在发送前 claim；专项 hard limit 下所有 Realtime 模式在建连前拒绝；普通无硬限额运行通过。
- Correlation keys:
  - provider request hard limit
- Raw content:
  ```text
  cargo test -p codex-core provider_request_hard_limit --lib: 8 passed
  ```
- Interpretation: 无法在推理前计数的服务器自动生成不会在硬上限模式启动。
- Time: 2026-08-01 18:24

## Evidence E-007: Windows 分支使用系统整树终止语义
- Related hypotheses:
  - H-007
- Direction: refutes
- Type: deterministic-test
- Source: `cache_process_control.py`、`test_cache_run_execution.py`
- Prediction or plan link:
  - H-007 Windows 父进程绕过反例
- Matched signal:
  - Job Object assign/close 被调用并给出 descendants guarantee；taskkill 非零反例保持 failed。
- Correlation keys:
  - benchmark subprocess PID
- Raw content:
  ```text
  Python cache control plane: 128 passed
  ```
- Interpretation: 此证据证明 Job 整树语义，但后续 E-013 进一步发现并关闭分配前逃逸窗口。
- Time: 2026-08-01 18:20

## Evidence E-008: 空白审查复现任意代理与账外请求路径
- Related hypotheses:
  - H-008
- Direction: supports
- Type: adversarial-review
- Source: fresh reviewer 019fba3f-9c88-7cf3-a122-3d2766f028cc
- Prediction or plan link:
  - H-008 任意代理与账外请求预测
- Matched signal:
  - 代理接受任意 method/path/body 并注入真实 Authorization；正式 observation 只读取 Agent 侧三份 artifact。
- Correlation keys:
  - HEAD d620349f9
- Raw content:
  ```text
  Agent can call the proxy directly; supervisor events are not reconciled with provider-wire evidence or ledger settlement.
  ```
- Interpretation: 物理隔离和数量硬上限仍缺少获批请求合同与账实一致性。
- Time: 2026-08-01

## Evidence E-009: 非批准请求与额外 dispatch 均被离线反例拒绝
- Related hypotheses:
  - H-008
- Direction: refutes
- Type: deterministic-test
- Source: `docker/test_provider_boundary_proxy.py`、`test_cache_hit_regression.py`
- Prediction or plan link:
  - H-008 修复验证
- Matched signal:
  - 错误 method/path/model 在上游前拒绝且不消费额度；多出的 boundary request 使 analyzer 报 request count/trace mismatch。
- Correlation keys:
  - provider request body SHA-256 sequence
- Raw content:
  ```text
  provider boundary tests: 4 passed
  Python cache control plane: 128 passed
  ```
- Interpretation: wire 对账负责证明 performance artifact 一致性；物理请求计费权威随后由 E-015 独立收口。
- Time: 2026-08-01

## Evidence E-010: 外层清理移除 host secret 且结算要求成功状态
- Related hypotheses:
  - H-009
- Direction: refutes
- Type: deterministic-test
- Source: `cache_process_control.py`、`test_cache_run_execution.py`
- Prediction or plan link:
  - H-009 host secret 残留反例
- Matched signal:
  - fixture secret 被覆盖删除、secret 目录消失；secret_cleanup_status=failed 时 cleanup_verified=false。
- Correlation keys:
  - CACHE-SECRET
- Raw content:
  ```text
  test_cleanup_erases_host_provider_secret_before_verification: passed
  repository target scan: no .container-secrets files
  ```
- Interpretation: PowerShell finally 被强杀后，外层 paid-run supervisor 仍负责本 run 的凭据清理证明。
- Time: 2026-08-01

## Evidence E-011: 全部 Realtime 模式在专项硬限额下 fail closed
- Related hypotheses:
  - H-010
- Direction: refutes
- Type: deterministic-test
- Source: `core/src/client.rs`、`core/src/client_tests.rs`
- Matched signal:
  - V2 transcription 断言由允许改为拒绝；定向 Rust 测试通过。
- Raw content:
  ```text
  provider_request_hard_limit_rejects_unmetered_realtime_generation: passed
  ```
- Interpretation: 未被 claim 的音频推理不再进入付费专项运行。
- Time: 2026-08-01

## Evidence E-012: POSIX/Windows 账本锁使用平台后端
- Related hypotheses:
  - H-011
- Direction: refutes
- Type: deterministic-test
- Source: `cache_run_ledger.py`、`test_cache_run_ledger.py`
- Matched signal:
  - guarded imports；mock Windows backend 对同一字节执行 LK_LOCK/LK_UNLCK。
- Raw content:
  ```text
  test_windows_lock_backend_locks_the_same_byte: passed
  ```
- Interpretation: Windows 不再因 POSIX 模块缺失而无法启动账本流程。
- Time: 2026-08-01

## Evidence E-013: Windows 挂起创建后先入 Job 再恢复
- Related hypotheses:
  - H-012
- Direction: refutes
- Type: deterministic-test
- Source: `cache_windows_job.py`、`test_cache_process_control.py`、Microsoft Job Object/CreateProcess 文档
- Matched signal:
  - 事件顺序 create_suspended, assign, resume；assign 失败时 ResumeThread 未调用且 TerminateProcess 已调用。
- Raw content:
  ```text
  test_windows_process_is_assigned_before_its_thread_is_resumed: passed
  test_windows_assignment_failure_terminates_the_suspended_process: passed
  ```
- Interpretation: 父进程在获得 Job 所有权前没有执行机会，无法产生逃逸后代。
- Time: 2026-08-01

## Evidence E-014: promotion 使用完整清理证明
- Related hypotheses:
  - H-013
- Direction: refutes
- Type: deterministic-test
- Source: `accepted_cache_baseline.py`、`test_promote_cache_baseline.py`
- Matched signal:
  - network/secret 字段缺失或 secret failed 均拒绝晋升。
- Raw content:
  ```text
  test_rejects_incomplete_post_run_cleanup_proof: passed
  ```
- Interpretation: runtime 结算与 promotion 不再维护两套不同清理标准。
- Time: 2026-08-01

## Evidence E-015: supervisor 请求计数独立于 usage 对账持久化
- Related hypotheses:
  - H-014
- Direction: refutes
- Type: deterministic-test
- Source: `run_cache_hit_regression.py`、`cache_run_ledger.py`、`accepted_cache_baseline.py`
- Matched signal:
  - wire mismatch 时 boundary count=2 仍持久化；usage 为空时 ledger 保留 api_requests=3；promotion 要求 attempt、observation、ledger 三者一致。
- Raw content:
  ```text
  test_persists_boundary_accounting_before_full_reconciliation: passed
  test_authoritative_request_count_survives_usage_failure: passed
  test_rejects_tampered_provider_accounting_on_attempt: passed
  ```
- Interpretation: runtime 不判断 Agent 请求动机，只在批准代理边界机械计数；证据不完整不会抹掉费用事实。
- Time: 2026-08-01

## Evidence E-016: 中断结算要求完整清理证明
- Related hypotheses:
  - H-015
- Direction: refutes
- Type: deterministic-test
- Source: `run_cache_hit_regression.py`、`test_cache_run_execution.py`
- Matched signal:
  - cleanup failed 返回 status=failed、exit=3、stop_reason=cancelled_cleanup_failed。
- Raw content:
  ```text
  test_keyboard_interrupt_with_unverified_cleanup_is_a_failed_run: passed
  ```
- Interpretation: Ctrl-C 不再掩盖未清理资源。
- Time: 2026-08-01

## Evidence E-017: 矛盾 cleanup 证明被共享 predicate 和 promotion 接受
- Related hypotheses:
  - H-016
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_process_control.py`、`test_promote_cache_baseline.py`
- Matched signal:
  - 成功状态分别携带非空 container/network/secret 或 error 时，四组 predicate 与四组 promotion 断言均失败。
- Raw content:
  ```text
  cleanup_verified contradictory proof: 4 failures
  promotion contradictory proof: 4 failures
  ```
- Interpretation: 现有成功标签没有与原始清理事实形成自洽合同。
- Time: 2026-08-01

## Evidence E-018: elapsed 超限可通过空自报列表晋升
- Related hypotheses:
  - H-017
- Direction: supports
- Type: deterministic-test
- Source: `test_promote_cache_baseline.py`
- Matched signal:
  - attempt/observation elapsed=121，批准上限=120，`budget_observation_exceeded=[]` 时未抛错。
- Raw content:
  ```text
  AssertionError: ValueError not raised
  ```
- Interpretation: 晋升器信任了结果自报，没有从批准预算复算时间硬边界。
- Time: 2026-08-01

## Evidence E-019: 两个并行 provider claim 的事件顺序稳定倒置
- Related hypotheses:
  - H-018
- Direction: supports
- Type: deterministic-test
- Source: `test_provider_boundary_proxy.py`
- Matched signal:
  - 用 barrier 延迟 count=1 的 record 后，events.jsonl 顺序为 `[2, 1]`。
- Raw content:
  ```text
  AssertionError: Lists differ: [2, 1] != [1, 2]
  ```
- Interpretation: claim 与权威事件提交不是同一个原子操作。
- Time: 2026-08-01

## Evidence E-020: Windows 异常注入暴露 handle 与终止确认缺口
- Related hypotheses:
  - H-019
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_process_control.py`
- Matched signal:
  - CloseHandle 失败前 `_handle` 已清空；TerminateProcess=false 仍只抛原 assign 错误；wait OSError 未保证 Job close。
- Raw content:
  ```text
  expected TerminateProcess error, got assignment failed
  CloseHandle failure path discarded the owned handle
  ```
- Interpretation: 主启动顺序正确，但失败路径没有完整的可证明所有权结论。
- Time: 2026-08-01

## Evidence E-021: post-wait 中断越过最终清理和结算
- Related hypotheses:
  - H-020
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_run_execution.py`
- Matched signal:
  - cleanup 阶段注入 KeyboardInterrupt 后 unittest 进程以 130 退出，`main()` 未返回失败结果。
- Raw content:
  ```text
  KeyboardInterrupt at run_cache_hit_regression.py cleanup_labeled_containers
  ```
- Interpretation: 中断保护只包围 provider wait，不覆盖后续监督职责。
- Time: 2026-08-01

## Evidence E-022: 任意费用金额和公式仍可晋升
- Related hypotheses:
  - H-021
- Direction: supports
- Type: deterministic-test
- Source: `test_promote_cache_baseline.py`
- Matched signal:
  - status 保持 estimated，amount=999、伪造 components/formula 后未抛错。
- Raw content:
  ```text
  AssertionError: ValueError not raised
  ```
- Interpretation: token/request 已复算，但费用层只验证了状态标签。
- Time: 2026-08-01

## Evidence E-023: 空 secret 目录产生矛盾的 removed 证明
- Related hypotheses:
  - H-022
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_process_control.py`
- Matched signal:
  - 空目录被删除后返回 `removed_verified`，但 `secret_paths=[]`。
- Raw content:
  ```text
  expected verified_absent, got removed_verified
  ```
- Interpretation: 目录元数据被误表达成秘密材料删除事实。
- Time: 2026-08-01

## Evidence E-024: Job close 异常没有触发显式终止
- Related hypotheses:
  - H-023
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_process_control.py`
- Matched signal:
  - close 抛错后结果为 failed，`job.terminate` 未调用。
- Raw content:
  ```text
  expected terminated, got failed
  ```
- Interpretation: KILL_ON_JOB_CLOSE 失败时没有第二条可验证终止路径。
- Time: 2026-08-01

## Evidence E-025: 聚合中断越过最终 settlement
- Related hypotheses:
  - H-024
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_run_aggregation.py`
- Matched signal:
  - evidence digest 注入 KeyboardInterrupt 后 main 直接传播，unittest 以 130 退出。
- Raw content:
  ```text
  KeyboardInterrupt at final canonical_json_sha256
  ```
- Interpretation: settlement helper 本身可靠，但调用前仍存在未保护区。
- Time: 2026-08-01

## Evidence E-026: 两秒 attempt 可声明为零秒总运行
- Related hypotheses:
  - H-025
- Direction: supports
- Type: deterministic-test
- Source: `test_promote_cache_baseline.py`
- Matched signal:
  - 两个 attempt 各 1 秒、result 与 ledger 均改成 0 秒后仍通过。
- Raw content:
  ```text
  AssertionError: ValueError not raised
  ```
- Interpretation: 总耗时没有从组成证据独立复算。
- Time: 2026-08-01

## Evidence E-027: CreateProcessW 返回边界中断泄漏挂起进程
- Related hypotheses:
  - H-026
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_process_control.py`
- Matched signal:
  - mock 写入 process/thread handles 后抛 KeyboardInterrupt，TerminateProcess 未调用。
- Raw content:
  ```text
  Expected TerminateProcess(100, 1) to have been called once. Called 0 times.
  ```
- Interpretation: native 调用不在清理 try 内，所有权事实未被消费。
- Time: 2026-08-01

## Evidence E-028: 两轮收尾根因修复通过完整离线回归
- Related hypotheses:
  - H-016
  - H-017
  - H-018
  - H-019
  - H-020
  - H-021
  - H-022
  - H-023
  - H-024
  - H-025
  - H-026
- Direction: supports
- Type: regression-suite
- Source: HEAD `809e1d513`
- Matched signal:
  - cache control plane 151 项、provider proxy 5 项、Rust hard-limit 8 项及 PowerShell provider boundary 全部通过。
- Raw content:
  ```text
  Python: 151 passed
  proxy: 5 passed
  Rust provider_request_hard_limit: 8 passed
  provider boundary tests passed
  ```
- Interpretation: 失败反例转绿且既有授权、计数、清理、晋升和 runtime 边界合同未回归。
- Time: 2026-08-01

## Evidence E-029: 未确认终止会释放最后 process handle
- Related hypotheses:
  - H-027
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_process_control.py`
- Matched signal:
  - Assign 与 TerminateProcess 均失败时，旧实现仍 CloseHandle(process)。
- Raw content:
  ```text
  expected process handle 100 to be retained; closed_handles=[101, 100]
  ```
- Interpretation: 报告 cleanup 失败不能替代保留后续控制能力。
- Time: 2026-08-01

## Evidence E-030: finalization 边界中断留下 running ledger
- Related hypotheses:
  - H-028
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_run_aggregation.py`
- Matched signal:
  - `cache_run_result.now` 注入 KeyboardInterrupt 后 main 直接传播。
- Raw content:
  ```text
  KeyboardInterrupt at finalize_run_result ended_at
  ```
- Interpretation: 聚合内部捕获不足以覆盖聚合与 settlement 的组合事务。
- Time: 2026-08-01

## Evidence E-031: 完整 promotion 接受布尔 elapsed
- Related hypotheses:
  - H-029
- Direction: supports
- Type: deterministic-test
- Source: `test_promote_cache_baseline.py`
- Matched signal:
  - result=true、attempt/observation=false 的完整证据链未抛错。
- Raw content:
  ```text
  AssertionError: ValueError not raised
  ```
- Interpretation: Python bool/int 继承关系必须在 JSON 合同边界显式排除。
- Time: 2026-08-01

## Evidence E-032: Windows handle 所有权的三条异常路径可复现
- Related hypotheses:
  - H-030
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_process_control.py`、`test_cache_windows_ownership.py`
- Matched signal:
  - configure 中断不 close Job；process close 失败后 returncode 阻止重试；fallback wait 失败跳过第二次 job close。
- Raw content:
  ```text
  4 ownership negative tests failed before 8bd820a9a and passed after repair
  ```
- Interpretation: 所有权状态必须在资源释放成功后提交，不能先写完成标志。
- Time: 2026-08-01

## Evidence E-033: 第三轮修复通过完整离线回归
- Related hypotheses:
  - H-027
  - H-028
  - H-029
  - H-030
- Direction: supports
- Type: regression-suite
- Source: HEAD `8bd820a9a`
- Matched signal:
  - cache control plane 156 项全部通过，新增 7 条根因反例均转绿。
- Raw content:
  ```text
  Python: 156 passed
  Ruff: all checks passed
  ```
- Interpretation: 新所有权事务没有破坏既有预算、证据、晋升和清理合同。
- Time: 2026-08-01

## Evidence E-034: durable claim 后中断可留下未结算账本
- Related hypotheses:
  - H-031
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_claim_transaction.py`
- Matched signal:
  - claim 已写入账本后注入 KeyboardInterrupt；旧结构没有覆盖 claim 到 attempt try 的间隙。
- Raw content:
  ```text
  durable claim existed while final settlement had not run
  ```
- Interpretation: 是否结算必须由 durable claim 事实决定，不能依赖局部控制流是否走到 attempt。
- Time: 2026-08-01

## Evidence E-035: Windows 第二次中断跳过剩余 owner 释放
- Related hypotheses:
  - H-032
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_windows_ownership.py`
- Matched signal:
  - assign 失败后第一次 TerminateProcess 抛 KeyboardInterrupt，旧路径直接跳出清理并跳过 Job close。
- Raw content:
  ```text
  nested cleanup interrupt bypassed remaining release operations
  ```
- Interpretation: emergency ownership释放必须逐资源尝试并记录失败，不能由一个异常短路。
- Time: 2026-08-01

## Evidence E-036: 非标准 JSON 和 Python 宽类型可伪造成功
- Related hypotheses:
  - H-033
- Direction: supports
- Type: deterministic-test
- Source: `test_promote_cache_baseline.py`
- Matched signal:
  - `runner_exit_code=false`、`business_success=NaN`、`trace_coverage=Infinity` 三种完整 promotion 变体被旧逻辑接受。
- Raw content:
  ```text
  three hostile full-promotion mutations were accepted before strict decoding
  ```
- Interpretation: 正式证据必须采用标准 JSON、有限数值和精确类型合同。
- Time: 2026-08-01

## Evidence E-037: metrics 标签不足以证明实际 arm
- Related hypotheses:
  - H-034
- Direction: supports
- Type: deterministic-test
- Source: `test_promote_cache_baseline.py`
- Matched signal:
  - 修改 Standard metrics 标签后可形成自洽的 map-request observation，证据中没有真实启动参数和 mode map。
- Raw content:
  ```text
  forged map-request evidence reused a Standard launch
  ```
- Interpretation: arm 身份需要来自实际执行输入，而不是与结果同源的标签。
- Time: 2026-08-01

## Evidence E-038: 双重终止失败丢失后续控制入口
- Related hypotheses:
  - H-035
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_windows_ownership.py`
- Matched signal:
  - TerminateProcess 与 taskkill 均失败时 process handle 虽未关闭，但旧 helper 不返回 owner。
- Raw content:
  ```text
  PID 456 handle retained without a reachable retry owner
  ```
- Interpretation: 暂时无法终止时必须保留显式可重试所有权，并阻止后续付费执行。
- Time: 2026-08-01

## Evidence E-039: network rm 成功不是清理完成证明
- Related hypotheses:
  - H-036
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_process_control.py`
- Matched signal:
  - 旧逻辑在 `docker network rm` 返回 0 后立即给出 `removed_verified`，没有 post-remove 查询。
- Raw content:
  ```text
  network removal returned success without an absence observation
  ```
- Interpretation: 网络清理与容器清理一样需要稳定后置状态，而非命令返回码。
- Time: 2026-08-01

## Evidence E-040: 第四轮修复通过完整离线回归
- Related hypotheses:
  - H-031
  - H-032
  - H-033
  - H-034
  - H-035
  - H-036
- Direction: supports
- Type: regression-suite
- Source: HEAD `3b291b111`
- Matched signal:
  - cache control plane 165 项、provider hard-limit 8 项、provider boundary Docker、non-agent builder、E3 start gate、release decision 全部通过。
- Raw content:
  ```text
  Python: 165 passed
  Rust provider_request_hard_limit: 8 passed
  provider boundary tests passed
  v005 non-agent gates builder selftest passed
  E3 start gate self-test: PASS
  Release decision self-test: PASS
  ```
- Interpretation: 六条最新失败反例转绿，且预算、证据、晋升、清理和 release 阻断合同没有回归。
- Time: 2026-08-01

## Evidence E-041: mixed-arm provider wire 复用通过完整 promotion
- Related hypotheses:
  - H-037
- Direction: supports
- Type: adversarial-reproduction
- Source: fresh reviewer `019fbaca-d860-78a0-9e77-ec37d9e18a60`
- Matched signal:
  - Standard 与 map-request 保留各自 metrics/argv，但复制相同 cache/request/provider boundary 后仍通过完整 promotion。
- Raw content:
  ```text
  full promotion validated as completed with identical cross-arm provider evidence
  ```
- Interpretation: 启动身份和实际 provider wire 必须在矩阵层形成跨 arm 约束。
- Time: 2026-08-01

## Evidence E-042: 非 Standard arm 校验固定为 map-request
- Related hypotheses:
  - H-038
- Direction: supports
- Type: static-and-deterministic-test
- Source: `cache_arm_identity.py`、`test_cache_arm_identity.py`
- Matched signal:
  - map-always/map-append 的真实 argv 被旧校验拒绝，map-request argv 改标签后可被解释成其他 TaskSpace arm。
- Raw content:
  ```text
  all non-standard arms expected taskspace_projection_policy="map-request"
  ```
- Interpretation: policy 是 arm 身份的一部分，不能只校验 logical_mode=taskspace。
- Time: 2026-08-01

## Evidence E-043: 未确认 Windows 进程只有进程内 owner
- Related hypotheses:
  - H-039
- Direction: supports
- Type: static-and-deterministic-test
- Source: `cache_windows_job.py`、`test_cache_windows_ownership.py`
- Matched signal:
  - 终止失败后 PID/handle 只保存在模块字典，runner 停止矩阵并退出前没有保证再次清理。
- Raw content:
  ```text
  retained owner disappeared with the Python interpreter
  ```
- Interpretation: 挂起进程的恢复身份必须跨解释器持久存在，且避免 PID reuse 误杀。
- Time: 2026-08-01

## Evidence E-044: recovery 可丢失并发最终结算并接受 NaN
- Related hypotheses:
  - H-040
- Direction: supports
- Type: deterministic-test
- Source: `test_recover_cache_run_ledger.py`
- Matched signal:
  - recovery 锁外读取 running 后，runner 写入 settled，旧 recovery 仍以陈旧 entry 覆盖；`NaN` 可经 json.loads 进入结算。
- Raw content:
  ```text
  stale recovery replaced a newer terminal ledger entry
  ```
- Interpretation: 恢复必须在同一锁中读、比较、变更，并复用严格结果合同。
- Time: 2026-08-01

## Evidence E-045: 四项 accepted blocking 修复通过离线回归
- Related hypotheses:
  - H-037
  - H-038
  - H-039
  - H-040
- Direction: supports
- Type: regression-suite
- Source: HEAD `13022a905`
- Matched signal:
  - cache control plane 171 项、三 policy container argv、provider boundary、non-agent builder 和 release decision 全部通过。
- Raw content:
  ```text
  Python: 171 passed
  container benchmark runner tests passed
  provider boundary tests passed
  v005 non-agent gates builder selftest passed
  Release decision self-test: PASS
  ```
- Interpretation: 四条新失败反例转绿，且既有执行、隔离、成本和发布阻断合同没有回归。
- Time: 2026-08-01

## Evidence E-046: 布尔正式证据在旧晋升边界被接受
- Related hypotheses:
  - H-041
- Direction: supports
- Type: adversarial-reproduction
- Source: fresh reviewer `019fbae1-4d30-7d71-8459-9dadf8a13a54`、`test_promote_cache_baseline.py`
- Matched signal:
  - `attempt.exit_code=false`、ledger repeat=`true`、ledger runner exit=`false` 可满足旧数值等式。
- Raw content:
  ```text
  boolean integer evidence accepted before exact type checks
  ```
- Interpretation: 正式 JSON 整数必须同时校验运行时类型和期望值。
- Time: 2026-08-01

## Evidence E-047: 两阶段 Windows Job 绑定存在进程级硬退出空窗
- Related hypotheses:
  - H-042
- Direction: supports
- Type: adversarial-static-review
- Source: fresh reviewer `019fbae1-4d30-7d71-8459-9dadf8a13a54`、Microsoft Win32 process attribute contract
- Matched signal:
  - `CreateProcessW` 返回后才写 journal 并调用 `AssignProcessToJobObject`，硬退出不执行 Python cleanup。
- Raw content:
  ```text
  process created but neither Job-owned nor durably journaled
  ```
- Interpretation: 必须通过 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 在创建时完成 Job 归属。
- Time: 2026-08-01

## Evidence E-048: durable recovery 可丢失旧 handle owner
- Related hypotheses:
  - H-043
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_windows_ownership.py`
- Matched signal:
  - 同 PID 同时存在 retained owner 与 journal 时，旧流程打开新 handle 并弹出旧 tuple，未调用旧 kernel 的 CloseHandle。
- Raw content:
  ```text
  retained process and thread handles were discarded by PID-keyed recovery
  ```
- Interpretation: durable recovery 前必须先结清当前解释器内已知 owner。
- Time: 2026-08-01

## Evidence E-049: 第五轮修复通过完整免费回归
- Related hypotheses:
  - H-041
  - H-042
  - H-043
- Direction: supports
- Type: regression-suite
- Source: HEAD `a3344da1d`
- Matched signal:
  - cache control plane 177 项及五组 PowerShell/Docker 集成自测全部通过；正式 release 仅因历史基线阻断。
- Raw content:
  ```text
  Python: 177 passed
  container benchmark runner tests passed
  provider boundary tests passed
  v005 non-agent gates builder selftest passed
  E3 start gate self-test: PASS
  Release decision self-test: PASS
  formal release gate: exit 20 (live_regression_failed)
  ```
- Interpretation: 原子 Windows 所有权和精确证据类型没有破坏预算、隔离、执行或发布合同。
- Time: 2026-08-01

## Evidence E-050: recovery 可用同 record_id 扩大授权执行
- Related hypotheses:
  - H-044
- Direction: supports
- Type: adversarial-reproduction
- Source: fresh reviewer `019fbaf4-bdd0-7e00-be4d-cf189456c75f`
- Matched signal:
  - planned=1 的 running entry 接受 actual=4、requests=8 且 proposal/authorization 错配的 result。
- Raw content:
  ```text
  recovery settled planned=1 actual=4 requests=8
  ```
- Interpretation: recovery 必须复核 durable claim，而非只看 record_id。
- Time: 2026-08-01

## Evidence E-051: partial 结果被共享 envelope 拒绝
- Related hypotheses:
  - H-045
- Direction: supports
- Type: deterministic-test
- Source: `test_recover_cache_run_ledger.py`
- Matched signal:
  - 合法 `status=partial`、`runner_exit_code=3` 返回 envelope incomplete。
- Raw content:
  ```text
  ValueError: cache result envelope is incomplete
  ```
- Interpretation: 非成功不等于无效；partial 必须可恢复并保留请求下界。
- Time: 2026-08-01

## Evidence E-052: repeat=true 通过完整 promotion
- Related hypotheses:
  - H-046
- Direction: supports
- Type: adversarial-reproduction
- Source: fresh reviewer `019fbaf4-bdd0-7e00-be4d-cf189456c75f`
- Matched signal:
  - proposal/result/acceptance 使用 `repeat=true`、ledger 使用 repeat=1，完整 promotion 被接受。
- Raw content:
  ```text
  validate_promotion accepted boolean repeat
  ```
- Interpretation: 类型校验必须位于预算源合同，不能只在 ledger 末端。
- Time: 2026-08-01

## Evidence E-053: partial 结算无法通过全局 ledger checker
- Related hypotheses:
  - H-047
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_run_ledger.py`、`test-whale-agent-run-ledger.ps1`
- Matched signal:
  - `api_requests=null` 忠实表达未知总数，但旧 checker 只接受非负整数。
- Raw content:
  ```text
  execution.api_requests is not a nonnegative integer
  ```
- Interpretation: 全局合同必须同时保存精确值状态与已知下界。
- Time: 2026-08-01

## Evidence E-054: 重复授权字段静默采用后值
- Related hypotheses:
  - H-048
- Direction: supports
- Type: deterministic-test
- Source: `test_cache_run_contract.py`
- Matched signal:
  - 两个 `approved_maximums` 被普通 `json.loads` 合并为后一个值。
- Raw content:
  ```text
  duplicate approved_maximums accepted
  ```
- Interpretation: 正式授权读取必须拒绝重复 object key。
- Time: 2026-08-01

## Evidence E-055: 第六轮修复通过完整免费回归
- Related hypotheses:
  - H-044
  - H-045
  - H-046
  - H-047
  - H-048
- Direction: supports
- Type: regression-suite
- Source: HEAD `1042384ff`
- Matched signal:
  - cache control plane 182 项、账本 JSON Schema、五组 PowerShell/Docker 自测全部通过。
- Raw content:
  ```text
  Python: 182 passed
  ledger JSON Schema: pass
  global ledger PowerShell checker: pass
  container/provider/non-agent/E3/release self-tests: pass
  formal release gate: exit 20 (live_regression_failed)
  ```
- Interpretation: durable recovery 与账本语义收敛没有降低成本、隔离或发布硬边界。
- Time: 2026-08-01

## Evidence E-056: recovery 严格结构比较拒绝 bool/int 混淆
- Related hypotheses: H-049
- Direction: supports
- Type: adversarial-regression
- Source: `8083f31ab`
- Matched signal: scope、attempt matrix 与 observation 定位均使用递归精确 JSON 类型比较。
- Interpretation: `true` 不再能匹配整数 `1`。
- Time: 2026-08-01

## Evidence E-057: 请求证据互斥合同通过双校验器
- Related hypotheses: H-050
- Direction: supports
- Type: schema-and-gate-test
- Source: `79f1c1d8c`
- Matched signal: JSON Schema 与 PowerShell 均接受 exact 或 inexact 其中一种，拒绝 minimum 多余、缺失和 null 错配。
- Interpretation: 全局账本不存在两份相互矛盾的请求事实。
- Time: 2026-08-01

## Evidence E-058: 同步污染 selection 的四类布尔反例被拒绝
- Related hypotheses: H-051
- Direction: supports
- Type: adversarial-regression
- Source: `650657a1d`
- Matched signal: repeat、planned、retry、maximum 在 claim/result 同步设为 bool 时均失败。
- Interpretation: 合法性独立于两份证据是否彼此相等。
- Time: 2026-08-01

## Evidence E-059: result 请求汇总由 attempts 复算
- Related hypotheses: H-052
- Direction: supports
- Type: production-shaped-regression
- Source: `b49765f47`、`94c3cf53e`
- Matched signal: bool、负数、错误 minimum 与错误 status 在 recovery/promotion 均拒绝，partial 下限保留。
- Interpretation: result 汇总字段不能自我授权。
- Time: 2026-08-01

## Evidence E-060: completed 完整性与 unsettled 单调下限通过反例
- Related hypotheses: H-053、H-054
- Direction: supports
- Type: adversarial-regression
- Source: `ad6df97d7`
- Matched signal: failed attempt、token 恒等式破坏均拒绝完整结算；已有 minimum=2 经 unsettled 仍为 2/partial。
- Interpretation: recovery 不会把失败证据写成完整费用，也不会降低已知成本下限。
- Time: 2026-08-01

## Evidence E-061: direct settlement 精确绑定 durable approved matrix
- Related hypotheses: H-055
- Direction: supports
- Type: direct-settlement-regression
- Source: `bbbf1fc16`
- Matched signal: sample/arm/repeat 同时缺失或同步越权时，completed settlement 均拒绝。
- Interpretation: direct、recovery、promotion 三条结算路径使用同一批准 scope。
- Time: 2026-08-01

## Evidence E-062: 请求下限先于证据复制持久化
- Related hypotheses: H-056
- Direction: supports
- Type: failure-ordering-regression
- Source: `bbbf1fc16`
- Matched signal: persist callback 执行时 running ledger 已为累计 minimum/partial；复制或哈希失败不清零。
- Interpretation: 证据文件资格失败不会改写已发生请求事实。
- Time: 2026-08-01

## Evidence E-063: 最终空白 closure review 无阻断
- Related hypotheses: H-049、H-050、H-051、H-052、H-053、H-054、H-055、H-056
- Direction: supports
- Type: fresh-adversarial-review
- Source: reviewer `019fbb4f-f64a-7ae0-ac4c-3c04c17140da`，HEAD `bbbf1fc16`
- Matched signal: P0=0、P1=0；195 项 Python 0 skip，Schema、ledger 与正式阻断状态一致。
- Interpretation: R8 缓存门禁工程问题可关闭；真实 accepted baseline 仍需独立用户预算。
- Time: 2026-08-01
