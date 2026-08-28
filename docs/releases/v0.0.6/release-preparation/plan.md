# WhaleCode v0.0.6 发布准备执行计划

- Status: local-candidate-verified
- Release scope: [多 Provider PRD](../../../../prd/2026-08-23-v0.0.6-multi-provider.md)、[DeepSeek Responses PRD](../../../../prd/2026-08-22-deepseek-responses-capability-completion.md)
- Release manifest: [release.json](release.json)

## 目标

把已完成的 v0.0.6 provider 适配固定为可审计候选：统一版本身份、准备发布说明、运行本地离线门禁，并明确六平台制品和外部发布的授权边界。

## 工作单元

| ID | 工作 | 结果 | 验证 | 状态 |
| --- | --- | --- | --- | --- |
| W1 | 固定 `v0.0.6` 发布身份 | Cargo、tag、发布登记和工作流默认值一致 | release identity tests | 已实现 |
| W2 | 汇总版本内容 | 发布说明聚焦多 Provider、DeepSeek Responses 与凭据隔离 | 人工通读、链接检查 | 已实现 |
| W3 | 运行本地候选门禁 | identity、distribution、brand、npm staging、workflow、单测、metadata 与本地二进制构建通过 | `run_local_preflight.sh 0.0.6`、`whale --version` | 已验证 |
| W4 | 复核缓存敏感面 | 当前 index 与已接受 provider final-wire 基线一致 | cache regression index gate | 已验证 |
| W5 | 固定候选提交 | `release.json` 记录最终 SHA | clean tree、commit 可达 | 待最终候选 |
| W6 | 构建六平台候选 | 六个平台未签名制品、manifest 与 checksum 完整 | 手动 workflow、六平台 smoke | 待外部运行授权 |
| W7 | 发布 npm/tag/GitHub Release | 公开 `v0.0.6` 并回读验证 | registry、tag、release smoke | 待明确发布授权 |

## 发布边界

- 本地离线验证不发送真实模型请求，不产生 Whale Agent API 费用。
- 不复用 vendor `rust-release.yml`，不启用自动触发器。
- 未取得明确授权前，不创建或推送 tag，不发布 npm 包，不创建 GitHub Release，不手动触发远端构建。
- DeepSeek hosted web search 展示合同和三模型 live matrix 声明按 [CHECKLIST](CHECKLIST.md) 处理；未关闭时不得扩大发布声明。

## 安全停止点

本地门禁通过后即可停止在“候选已准备”状态。外部构建或发布失败时保留 `status=preparing`，不覆盖 npm 已存在版本，不复用失败候选的版本号或校验值。
