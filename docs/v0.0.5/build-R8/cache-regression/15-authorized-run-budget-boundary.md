# 真实缓存回归的授权预算边界

- 日期：2026-08-01
- 状态：离线实现与测试通过；未运行真实 provider
- 适用范围：`run_cache_hit_regression.py` 启动的获批缓存 smoke

## 1. 产品边界

预算单把成本信息分成两类，不能混称为“上限”：

1. **执行前和执行中的硬边界**：精确源码、sample/arm/repeat 矩阵、禁止自动重试、每个 Whale 进程最多发出的
   provider 请求数、agent 容器运行时限，以及超时后的容器清理宽限。
2. **执行后的观测阈值**：常态 input/output token 和按实际 usage 估算的费用。超过阈值会停止后续 sample 并禁止
   晋升，但已经完成的 provider 请求不能被事后撤销，因此这些值不得宣称为硬成本边界。

预算单另给出保守最坏费用：

```text
最大 sample run × 每 run 请求硬上限 × provider 官方单请求最大 input/output × 冻结价格
```

input 全部按 cache miss 价格计算。该值明显高于常态观测预算，但它诚实表达当前 provider 基础设施下可证明的最坏
边界；不得用历史均值替换它。

## 2. 执行机制

- 通用 provider client 从 `WHALE_PROVIDER_REQUEST_HARD_LIMIT` 读取正整数；缺失表示普通产品运行不启用专项限制，
  非法值 fail closed。HTTP、压缩、memory 和显式 Realtime 生成均在真实 dispatch 前认领额度，隐藏 retry 被关闭。
- Realtime 一条连接可触发多次生成，因此显式 `response.create` 逐次计数；Server VAD 等无法在客户端推理前计数的
  Realtime 在专项硬上限启用时建连前 fail closed。检查使用 parser 归一化后的真实模式，避免 V1 把 transcription
  转成 conversation 后绕过；非法 hard-limit 配置同样拒绝全部 Realtime 建连。未配置专项硬限额的普通产品运行不受
  影响。
- 付费 Docker runner 不把真实 API Key 或权威计数状态交给 Agent。Agent 只连接 Docker internal network，并只持有
  假凭据；独立 provider boundary 同时连接内部网和出网网，固定转发到 DeepSeek、注入真实 Key 并统一计数。
- 代理只接受批准模型的 `POST /responses`，拒绝任意 method、endpoint、query 和 model。runtime 不判断请求由 Whale
  还是 Agent 的其他动作发出：凡通过批准边界的 dispatch 都允许且计数。Whale wire trace 对账只决定该运行能否
  成为缓存性能证据，不能删减真实请求数。
- 外层 runner 超时后，POSIX 终止进程组；Windows 先用 `CREATE_SUSPENDED` 创建进程，在恢复前分配到
  `KILL_ON_JOB_CLOSE` Job Object，从启动起持有整棵进程树。随后按唯一 `whalecode.run_id` 清理残留容器、
  provider-boundary 网络和 host secret。连续三次容器空集，且网络、secret 均验证为空后才允许收口。
- Agent 外 supervisor 计数是 `api_requests` 的权威来源。token usage 失败不把请求写成 0；边界证据缺失时写
  `api_requests=null`、`api_requests_minimum=<已知值>` 和 partial/unavailable。金额仍按已取得 token 遥测标为
  `estimated`、`estimated_partial` 或 `unavailable`。
- provider boundary 请求数一经解析，先写入 attempt 与原子 running-ledger checkpoint，再复制和哈希证据文件；
  复制失败只降低性能证据资格，不降低已发生请求下限。completed 结算还必须绑定批准矩阵、成功 attempt、完整清理
  与 `input=cached+uncached` token 恒等式。

## 3. 失败关闭

- 授权与预算摘要不一致、请求上限缺失、官方 provider 上限缺失、价格快照缺失或 proposal 被修改：启动前拒绝。
- 任一 attempt 失败、usage 不完整、观测阈值越界或超时清理失败：结果不得晋升。
- 进程崩溃导致结果已写但账本未结算：恢复命令按持久 result 幂等补结算；没有 result 时保留 `running/unsettled`，
  不猜测费用。

## 4. 已验证证据

- Python cache control plane：195 项通过且 0 skip，包括预算复算、篡改拒绝、授权重放、跨平台进程树终止、容器/网络/
  secret 回收、监督计数/对账、部分费用和恢复。
- Rust：43 项 core Realtime、46 项 API Realtime 通过，其中 8 项 provider hard-limit 定向测试覆盖超额前置拒绝、
  非法配置、跨 client/进程共享、隐藏 retry、自动生成和 V1 模式归一化。
- PowerShell：provider boundary 离线 Docker 自检证明 Agent 无 secret mount、只有 internal network、无法直连
  mock provider 且超额请求不到达上游；container runtime、benchmark runner、harness、E3、release 和 non-agent
  builder 自测通过。
- 本轮真实 Whale Agent/provider run：0；全局付费运行账本没有新增记录。
- 最新 Windows runner 在 `CreateProcessW` 阶段通过 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 原子进入 Kill-on-close Job，
  不再依赖创建后的 Python cleanup 或 owner journal 建立首要所有权；最新缓存控制面回归为 `195 passed`。
- 硬退出后的 recovery 必须在账本锁内重新匹配原 claim 的 commit、surface、proposal、authorization、matrix、
  run root 和预算；不完整请求证据以 `api_requests=null`、已知 minimum 和 evidence status 表达，禁止低报为 0。

## 5. 外部依据

- [DeepSeek 模型与价格](https://api-docs.deepseek.com/quick_start/pricing/)：冻结 Flash 的上下文、最大输出和
  cached/uncached/output 价格，用于预算最坏值。
- [DeepSeek Rate Limit](https://api-docs.deepseek.com/quick_start/rate_limit)：provider 只说明账号并发调度，
  不提供单次作业费用上限，因此项目必须在自身 provider dispatch 层限制请求数。
- [Docker internal network](https://docs.docker.com/reference/cli/docker/network/create/#network-internal-mode)：内部网络
  无外部默认路由，适合让 Agent 只能访问同时连接内部网与出网网的固定代理。
- [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)：Job Object 可把一组
  进程作为单元管理，子进程默认继承所在 Job；`KILL_ON_JOB_CLOSE` 为本门禁提供整树终止后置条件。
- [UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)：
  `PROC_THREAD_ATTRIBUTE_JOB_LIST` 可在创建进程时指定 Job，消除创建后再分配的逃逸窗口。
- [CreateProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessa)：
  `EXTENDED_STARTUPINFO_PRESENT` 使创建调用使用扩展属性列表。
- [OpenAI Realtime `response.create`](https://platform.openai.com/docs/api-reference/realtime-client-events#realtime-client-events-response-create)：
  每个事件触发一次模型推理；Server VAD 还可自动创建响应，因此连接数不能替代推理次数。
