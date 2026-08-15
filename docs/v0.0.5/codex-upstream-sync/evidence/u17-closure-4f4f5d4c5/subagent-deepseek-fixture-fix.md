# 子 Agent DeepSeek 测试夹具修复证据

- 日期：2026-08-15
- 起始提交：`96da8ab53c1d1dd7a4d3322a22290d122412756d`
- 范围：U17 清单中的 `CL-GPT-SUBAGENT` 2 项与 `CI-GPT-SUBAGENT` 14 项
- 生产代码变更：无
- 真实模型/API 请求：0

## 修复内容

1. 模型继承、显式覆盖、role、fork、summary 与 reasoning 测试改用 `deepseek-v4-flash` / `deepseek-v4-pro`。
2. DeepSeek 的默认 reasoning 值按协议扩展值 `standard` 断言；显式设置使用生产目录支持的 `high` / `max`。
3. Flash/Pro 生产目录均不支持 service tier，因此相关测试改为验证显式 `priority` 被拒绝，以及继承或 role 中不支持的 tier 被清理；未伪造生产能力。
4. full-history V2 用例按 mock 的完整匹配条件选择 child request，排除同样包含委派文本与相同模型的 parent spawn 请求，并同时校验 child session snapshot 与最终 wire reasoning。
5. “默认模型支持 Ultra、role 模型不支持 Ultra”的专用能力差异仅在对应测试的私有 model catalog 中构造，不修改生产目录。

## 验证结果

| 矩阵 | 结果 | 结论 |
| --- | --- | --- |
| `just test -p codex-core --lib spawn_agent_service_tier_ ...` | 2/2 passed | 原 core lib 2 项关闭 |
| `just test -p codex-core --test all subagent_notifications ...` | 25/25 passed | 原 integration 14 项及 11 个邻近用例通过 |
| 完整 core integration | 1123 run；1100 passed；23 failed；8 skipped | U17 的 37 项失败精确减少 14 项，无新增分类 |
| 隔离完整 core lib | 2178 run；2157 passed；21 failed | U17 扣除已知代理污染后的 23 个有效失败减少 2 项 |

隔离 core lib 命令清除了大小写代理变量、使用 `TMPDIR=/dev/shm` 并设置 `--test-threads=1`。原因是宿主 `/tmp/.git`、`/tmp/.codex` 与代理变量会额外污染临时目录及 shell 环境测试；这些额外失败不属于本次代码变更。

剩余 21 个 core lib 与 23 个 core integration 失败仍属于既有延期边界：Guardian、OpenAI remote catalog/model manager、remote plugin 与 hosted image 等，本批未修改也未声明通过。
