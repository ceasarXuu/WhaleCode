# WhaleCode 实验制度

- 状态：可执行
- 创建日期：2026-06-18
- 负责人：WhaleCode 工程
- 范围：TaskSpace、E3、基准测试、冒烟测试、发布门禁、版本对比证据

本目录是实验定义的唯一入口。`docs/testing/` 下的历史文件继续作为证据记录保留，但不再作为实验等级定义、样本集命名和结论边界的权威来源。

## 权威文件

| 文件 | 用途 |
|---|---|
| [taskspace-evidence-levels-and-samples.md](./taskspace-evidence-levels-and-samples.md) | 定义 E1-E5、当前样本集、允许结论和报告规则。 |

## 不可违反的规则

1. 只有当报告等级明确为 `E3` 时，才能把一次运行称为 E3；`E3-candidate`、`E2`、`E2-candidate`、`E1` 都不能简称为 E3。
2. 每份结果摘要都必须写清样本集、样本名、重复次数、运行器命令族、运行根目录、分数有效性和审计状态。
3. 内部测试夹具矩阵只能支撑工程就绪结论，不能支撑外部基准测试正确率结论。
4. 只有证据等级和样本集相同，版本对比才是同口径对比；否则报告必须明确写出“不同口径，不可直接比较”。
5. 候选证据不是发布证据。`E3-candidate` 表示运行仍在等待必要 E3 门禁，通常是人工审计或证明闭环。

## 必填结果头

后续每份实验结果文档都必须以这个信息块开头：

```text
experiment_level: E1 | E2 | E3 | E3-candidate | E4 | E5
sample_set_id: <docs/experiments 中登记的样本集 id>
sample_names: <样本名列表>
repeats_per_sample: <每个样本重复次数>
runner_family: internal-matrix | terminal-bench | deepswe | historical-whale | release-calibration | product-benchmark
runner_entrypoint: <脚本或命令>
run_root: <绝对路径>
score_valid: true | false | not_applicable
human_audit_status: not_required | pending | completed | failed
allowed_claim: <一句中文允许结论>
```
