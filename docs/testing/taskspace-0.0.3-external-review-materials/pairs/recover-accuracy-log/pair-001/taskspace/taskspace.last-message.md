The logs have been recovered, sorted, split into per-run files, and accuracy computed. Here's the summary:

## Investigation Phase
- Read raw logs from `W:\app\task_deps/generator.log` and `W:\app\task_deps/judge.log` (30 events each)
- Grouped events by `question_id` (10 unique IDs, each with exactly 3 events per role)

## Implementation Phase
For each `question_id` and role, sorted events by timestamp and assigned:
- Earliest → **run_1** (the first execution)
- Middle → **run_2** (the second execution)
- Latest → **run_3** (the third execution)

Within each run, events were sorted by timestamp ascending. All 6 run files have exactly 10 lines in compact JSON format.

## Validation Phase
Verified: correct line count (10 per run file), compact JSON format (no spaces after `:` or `,`), sorted by timestamp, unique question_ids per file, and accuracy recomputed from scratch.

## Outputs (7 files in `W:\app\recovered_logs/`)

| File | Lines |
|---|---|
| `run_1_generator.jsonl` | 10 ✓ |
| `run_1_judge.jsonl` | 10 ✓ |
| `run_2_generator.jsonl` | 10 ✓ |
| `run_2_judge.jsonl` | 10 ✓ |
| `run_3_generator.jsonl` | 10 ✓ |
| `run_3_judge.jsonl` | 10 ✓ |
| `results.json` | 3 runs |

## Accuracy Results

| Run | Correct | Total (excl. errors) | Accuracy |
|---|---|---|---|
| **Run 1** | 7 | 9 | **0.78** |
| **Run 2** | 3 | 9 | **0.33** |
| **Run 3** | 8 | 8 | **1.00** |

Details per run:
- **Run 1**: q-0003 prediction was `Timeout` (error) → excluded. 7/9 non-error pairs matched.
- **Run 2**: q-0001 prediction was `API Error` (error) → excluded. 3/9 non-error pairs matched.
- **Run 3**: q-0007 prediction was `Quota Exceeds` (error) and q-0002 prediction was `Timeout` (error) → excluded. All 8 remaining pairs matched perfectly (1.00).