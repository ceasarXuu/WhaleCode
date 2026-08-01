# R5-K3 S4.2 可恢复 B0 折叠结果

- Date: 2026-07-14
- Decision: HOLD
- Engineering: COMPLETE
- Deterministic contract: PASS
- Natural live activation: NOT OBSERVED
- Live benefit: UNVERIFIED
- Baseline: B0 / S4.0 `7040547`
- Candidate: S4.2 `080ed60`
- Design: `51-r5-k3-s4-2-recoverable-tier-fold-design.md`

## 1. 结论

S4.2已经按修正后的关系实现：B0仍是唯一默认详情选择器，S4.2不删除B0选出的详情，只把符合拓扑条件且被
B0省略的canonical详情变成可识别、可恢复的折叠内容。机制合同全部通过，但现有自然运行没有形成足够深的Map
依赖链，因此本轮只能确认实现正确和零触发路径无projection回归，不能宣称Agent侧成本或任务收益。

当前不进入K4，也不开始下一项压缩策略。S4.2保持`HOLD`，等待能自然产生深Map的客观样本或Map结构能力本身得到
独立改善后再做收益门禁。

## 2. B0与S4.2的实际关系

```text
B0 visible = D1/D2/D3选择结果
S4.2 hidden = 全部canonical详情 - B0 visible

hidden为空             -> 输出与S4.0逐字节一致
hidden非空且折叠更省字节 -> 保留B0 visible并增加folded引用
hidden非空但折叠不经济   -> 展示全部详情，不制造负压缩
Agent执行expand_nodes   -> 返回hidden引用并永久展示全部详情
```

这不是B0后的第二次裁剪。S4.2增加的是B0省略内容的可恢复性，而不是继续减少B0已经选择的内容。

## 3. 确定性合同

五节点、四边的Runtime状态机链路使用每个已完成节点4条真实canonical工具事件，活跃节点为`node-4`。

| 检查项 | 结果 |
|---|---:|
| canonical node可见 | 5/5 |
| canonical edge可见 | 4/4 |
| distance>=3且非root的eligible节点 | 1 |
| B0 D3可见详情保留 | 1/1 |
| hidden详情识别 | 3/3 |
| folded hidden详情 | 3/3 |
| 展开工具立即返回hidden引用 | 3/3 |
| 展开后节点全部详情可见 | 4/4 |
| mixed合法/非法expand批次partial commit | 0 |
| snapshot/restore 20轮后展开事件漂移 | 0 |
| expanded超预算时自动refold | 0 |

`cargo test -p codex-core action_map:: --lib`、`tools::handlers::taskspace_control`、`tools::sequence`、
`cargo test -p codex-tools taskspace_tool --lib`、observer与active-prefix harness测试均通过。

## 4. 自然active-prefix零触发验证

同一真实Agent前缀、同一workspace和continuation prompt各运行3次。该Map有4个节点、0条依赖边，因此没有任何
节点满足distance条件。

| Arm | 成功 | Request 总和/均值/P50 | Input 总和/均值/P50 | Cached 总和/均值/P50 | Cache加权/P50 | Wall ms 总和/均值/P50 | Projection P50 |
|---|---:|---:|---:|---:|---:|---:|---:|
| Standard | 3/3 | 33/11.00/11 | 325221/108407/95952 | 307456/102485/91904 | 94.54%/94.77% | 148004/49335/28276 | N/A |
| S4.0 | 3/3 | 32/10.67/9 | 378663/126221/107465 | 364672/121557/102400 | 96.31%/96.48% | 164489/54830/59592 | 1963 B |
| S4.2 | 3/3 | 35/11.67/10 | 438680/146227/131908 | 408832/136277/119040 | 93.20%/93.93% | 145362/48454/46256 | 1963 B |

S4.2三次均为`eligible=0 / folded=0 / activation=0`，且projection与S4.0同为`1963 B`。这通过了“无hidden时
不在B0之外增加projection结构”的门禁。S4.2相对S4.0的request和token波动发生在没有fold的路径，不能解释为
压缩收益；Req2+加权缓存仅相差`-0.17pp`，总缓存差异主要受首请求和不同动作路径影响。

证据：

- `target/r5-map-compression/S4.2-active-zero-regression/matrix-results.json`
- `target/r5-map-compression/S4.2-active-zero-regression/summary-v2.json`

## 5. Fresh Docker四臂矩阵

simple与complex分别执行Standard、B0、S4.0、S4.2各3次，共24次，业务和外部验证均为`24/24`。fresh epoch按
J6.7.7合同不构造active Map projection，所以下表只能验证正确性、工具schema和普通执行路径，不能验证S4收益。

| Sample | Arm | 成功 | Request 总和/均值/P50 | Input 总和/均值/P50 | Cache加权/P50 | Wall ms 总和/均值/P50 | Map节点/边 P50 |
|---|---|---:|---:|---:|---:|---:|---:|
| simple | Standard | 3/3 | 16/5.33/5 | 108754/36251/33751 | 95.45%/95.57% | 44595/14865/14665 | 0/0 |
| simple | B0 | 3/3 | 29/9.67/8 | 247852/82617/62698 | 95.70%/94.32% | 67058/22353/19984 | 3/0 |
| simple | S4.0 | 3/3 | 19/6.33/6 | 140962/46987/45302 | 94.53%/94.18% | 54791/18264/19336 | 3/0 |
| simple | S4.2 | 3/3 | 32/10.67/7 | 292872/97624/53075 | 93.97%/89.67% | 86289/28763/25343 | 3/0 |
| complex | Standard | 3/3 | 28/9.33/9 | 277320/92440/90465 | 93.00%/93.00% | 126679/42226/39597 | 0/0 |
| complex | B0 | 3/3 | 43/14.33/15 | 551054/183685/195942 | 96.21%/96.29% | 170585/56862/58053 | 5/0 |
| complex | S4.0 | 3/3 | 34/11.33/11 | 400379/133460/121050 | 94.53%/93.48% | 183741/61247/48484 | 4/0 |
| complex | S4.2 | 3/3 | 36/12.00/13 | 411039/137013/150678 | 95.70%/95.91% | 157105/52368/52866 | 4/0 |

证据：`target/r5-map-compression/S4.2-formal/observation/map-compression-observation.{json,md}`。

## 6. 未通过的收益门禁

既有`target/`内共扫描1051份rollout，没有发现同时具备至少3条依赖边和可折叠远端详情的自然轨迹。正式矩阵的
simple、complex和active-prefix同样全部为0条边。当前事实是：

1. S4.2机制在显式深图上正确工作；
2. 当前Agent运行没有自然形成S4.2所需的深图；
3. 因此fold频率、Agent自主expand、重读变化和实际token收益都没有live证据；
4. 不得通过改写rollout、在prompt中暗示TaskSpace或降低阈值来制造激活。

该缺口与S4.2折叠实现分离记录。后续若处理Map坍缩或依赖表达，应作为Map能力问题独立设计，不能为了让压缩测试
通过而让Runtime替Agent生成语义拓扑。

## 7. 状态决定

- 工程实现：完成；
- B0关系修正：完成；
- 确定性语义/恢复合同：通过；
- simple与complex正确性：通过；
- 零触发projection回归：通过；
- 自然fold/expand与收益：未验证；
- K4 Entry：不满足；
- 下一策略：暂停，等待用户指令。
