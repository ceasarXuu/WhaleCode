# 当前 vendor 的宿主隔离回归入口

- 日期：2026-08-15
- 起始提交：`36aa9da24d4c446b1a86eb4a619dec85026622e2`
- 范围：开发测试环境，不修改 Whale/Codex 产品逻辑
- 真实模型/API 请求：0

## 问题与边界

本机同时存在大小写 proxy 变量、`/tmp/.git` 与 `/tmp/.codex`。上游 `just test` 保持官方原样，会继承这些宿主输入；core 中使用默认临时目录或启动用户 shell 的测试因此出现 6 项额外失败。

仅设置 `GIT_CEILING_DIRECTORIES` 不充分：Codex 自有项目发现仍可沿祖先读取 `/tmp/.git` 与 `/tmp/.codex`。修复因此位于开发测试入口：清理 proxy/ambient sandbox 变量，并把 Nextest 的临时目录放在祖先链无 `.git/.codex` 的物理根中。Linux 优先 `/dev/shm`；其他平台或特殊主机可通过 `WHALE_CODEX_TEST_TMPDIR` 指定已存在的安全目录。找不到安全目录时入口明确失败，不回退到受污染根。

## 使用方式

完整 crate 回归使用：

```bash
python3 scripts/codex-upstream/run_isolated_tests.py -p codex-core --lib
```

脚本接受原样的 `cargo nextest run` 参数，但要求开发者显式给出测试范围，避免误触发整个 workspace。定向测试仍可使用上游 `just test`。

## 验证

| 检查 | 结果 |
| --- | --- |
| Codex upstream 同步工具 Python 单测 | 48/48 passed |
| Python 语法与 Ruff | passed |
| 原受污染 core 定向集合 | 6/6 passed |
| 完整隔离 core lib | 2178 run；2157 passed；21 known-deferred failed |
| 产品代码或 vendor 代码变更 | 0 |

首次只清 proxy、设置私有 TMP 与 Git ceiling 的实现为 1 passed、5 failed，证实物理临时根隔离不可省略。修订后 config、Git project discovery、realtime context 与 user-shell proxy 六项全部通过；并行完整 core lib 回归稳定为 2157 passed、21 个既有延期失败，说明无需以串行化掩盖竞态。完整证据链见 `coe/2026-08-15-22-23-core-test-host-isolation.md`。

剩余 core lib 与 integration 失败仍按既有 Guardian、OpenAI hosted/remote catalog、remote plugin 和 hosted image 产品边界延期；本入口只去除宿主噪声，不把延期能力伪装成通过。
