# R5-J7.4 Patch 可观测性与回归门禁结果

- Date: 2026-07-13
- Status: Complete
- Scope: patch lifecycle tracing、rollout extractor、通用性能报告、回归构建

## 1. 结果

J7.4退出门禁通过。通用性能观察工具新增`patch`分账，覆盖：

```text
single/multi patch carrier
request patch total/max and multi-patch preflight reject
multi-file patch
prepare/commit/partial commit failure
post-patch action/skipped
explicit read target/repeat/feedback coverage
```

extractor从canonical rollout按连续provider call batch还原request，不读取reasoning，不从shell正文推断读写。输出只含
计数、request index和rejected布尔值，不包含patch、路径、文件正文或secret。

## 2. 执行层日志

`codex-apply-patch`新增四个阶段事件：

| Event | Fields |
|---|---|
| `apply_patch.prepare_completed` | `hunk_count` |
| `apply_patch.prepare_failed` | `stage`, `hunk_count` |
| `apply_patch.commit_completed` | added/modified/deleted count |
| `apply_patch.commit_failed` | committed/pending/restored/rollback-failed count |

request preflight继续使用`tool.request_patch_count_validated`和`tool.request_multi_patch_rejected`。所有事件依赖现有
tool span关联call id，不另存payload hash或正文。

## 3. 历史真实回放

对J6.7后真实缺陷artifact只读回放：

`target/r5-final-loop-fix-repeat3/subscription-billing-repair/20260713-002149-397/pair-002/left/artifacts`

| Metric | Observed |
|---|---:|
| request patch declarations | 6 |
| max patch per response | 5 |
| multi-patch request attempts | 1 |
| preflight rejects | 0（历史版本尚无J7.3） |

该结果与原始“同response 5 patch”证据一致，证明observer不依赖合成夹具。

## 4. 验证

| Gate | Result |
|---|---|
| patch observer fixture | passed；15类指标与payload不泄漏断言 |
| performance observation self-test | passed；JSON aggregate与Markdown section |
| performance skill validation | passed |
| apply-patch full suite | 64 lib + 22 CLI/scenario passed |
| core sequence/preflight | 9 passed |
| locked Whale build | passed |
| file size | new code <=500；performance observer exactly 500 lines |

## 5. 当前边界

1. 显式读取观察只覆盖`read_file`和`read_output_ref`，不猜测`exec_command`中的`cat/sed`语义。
2. `patch_partial_commit_count`只统计commit失败且rollback为`best_effort_partial`，不会把成功回滚误报为残留提交。
3. J7.4证明工程正确性和可观测性，不代表J7整体收益已经完成Docker对比。

## 6. 暂停点

按用户要求停在J7.4。J7.5仍待执行：Docker Standard/R4/R5样本、成本/cache/map对比和结构收益验收。
