# R5-J7.2 Singular Carrier 与 Request Manifest 结果

- Date: 2026-07-13
- Status: Complete
- Scope: TaskSpace bootstrap schema、typed parser、共享request manifest

## 1. 结果

J7.2退出门禁通过。工具契约已从宽泛的bootstrap `actions[]`替换为互斥`continuation`：

| Kind | Patch cardinality | Tail actions |
|---|---:|---|
| `actions` | 0 | 至少一个非patch工具 |
| `patch_then_actions` | 1 | 零个或多个非patch工具 |

旧`actions[]`不保留兼容；普通action union在结构上排除`apply_patch`。patch slot由同轮model-visible
`apply_patch` ToolSpec派生，function/custom两种payload仍由原ToolRouter校验和执行。

active Map没有恢复nested action carrier。它继续使用原生顶层ordinary tools，状态操作仍是共享sequence中的barrier。

## 2. Request Manifest

新增`ToolSequenceManifest`，在Standard与TaskSpace共享的`execute_response_tool_sequence`入口构造：

```text
top-level canonical tools
  + taskspace_control declared continuation tools
  -> identity-only entries
  -> request_patch_count
```

manifest只记录`call_id`、父call、canonical tool name和patch identity，不读取patch正文，不解析reasoning，不推断
shell行为。J7.2只构造和记录清单，超限拒绝在J7.3启用。

## 3. 验证

| Gate | Result |
|---|---|
| TaskSpace schema tests | 3 passed |
| typed parser/handler tests | 19 passed |
| manifest tests | 2 passed |
| sequence tests | 6 passed |
| TaskSpace scenario integration | 8 passed |
| file size gate | all touched/new code files <= 500 lines |

## 4. 工程收益

1. bootstrap内第二个patch已经无法通过机器可读schema表达，不依赖提示词劝导。
2. `init map + patch + test`仍可在一次provider response中声明。
3. Standard顶层、TaskSpace顶层与bootstrap nested patch使用同一个request计数入口。
4. manifest没有新增语义判断，runtime边界仍限于工具形状和资源硬规则。

## 5. 下一门禁

J7.3在任何Map transition、ToolRouter调用或文件副作用前验证`request_patch_count <= 1`。超限时必须为整组
provider calls生成闭合的机械失败/跳过结果，且state/filesystem快照保持不变。
