All tasks are complete. Here is the final summary:

---

## Log Recovery & Accuracy Results

### Investigation Phase

The raw log files (`generator.log` and `judge.log`) each contained 30 events across 10 question IDs (3 per question). Events were out of order and some contained API error markers. Parallel investigation confirmed the sorting and run assignment logic.

### Per-Run Log Files (Step 1)

Each event was grouped by `question_id`, sorted by timestamp, and assigned to runs (earliest = run 1, middle = run 2, latest = run 3). Six JSONL files were created with exactly 10 lines each, in compact JSON format, sorted by timestamp ascending, with error records included:

| File | Path |
|------|------|
| Run 1 Generator | `W:\app\recovered_logs\run_1_generator.jsonl` |
| Run 1 Judge | `W:\app\recovered_logs\run_1_judge.jsonl` |
| Run 2 Generator | `W:\app\recovered_logs\run_2_generator.jsonl` |
| Run 2 Judge | `W:\app\recovered_logs\run_2_judge.jsonl` |
| Run 3 Generator | `W:\app\recovered_logs\run_3_generator.jsonl` |
| Run 3 Judge | `W:\app\recovered_logs\run_3_judge.jsonl` |

**Errors detected:**
- **Run 1**: q-0003 (generator `Timeout`)
- **Run 2**: q-0001 (generator `API Error`)
- **Run 3**: q-0002 (generator `Timeout`), q-0007 (generator `Quota Exceeds`)
- No judge events contained errors.

### Accuracy Computation (Step 2)

After excluding error pairs, the accuracy was computed:

```json
{
  "runs": [
    { "run_index": 1, "correct": 7, "total": 9, "accuracy": 0.78 },
    { "run_index": 2, "correct": 3, "total": 9, "accuracy": 0.33 },
    { "run_index": 3, "correct": 8, "total": 8, "accuracy": 1.0 }
  ]
}
```

- **Run 1**: 7/9 correct (q-0003 excluded for generator error)
- **Run 2**: 3/9 correct (q-0001 excluded for generator error)
- **Run 3**: 8/8 correct (q-0002 and q-0007 excluded for generator errors)

All 7 output files are in `W:\app\recovered_logs\`.