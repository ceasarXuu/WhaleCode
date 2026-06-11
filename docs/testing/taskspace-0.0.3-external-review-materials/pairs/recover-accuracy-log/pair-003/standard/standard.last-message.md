All 7 output files have been generated successfully in `W:\app\recovered_logs/`:

**Step 1 — Per-run log files (6 JSONL files, each with exactly 10 lines sorted by timestamp ascending):**

| File | Description |
|---|---|
| `run_1_generator.jsonl` | Earliest generator events per question_id |
| `run_1_judge.jsonl` | Corresponding earliest judge events |
| `run_2_generator.jsonl` | Middle generator events per question_id |
| `run_2_judge.jsonl` | Corresponding middle judge events |
| `run_3_generator.jsonl` | Latest generator events per question_id |
| `run_3_judge.jsonl` | Corresponding latest judge events |

Error records (Timeouts, API Errors, Quota Exceeds) are preserved in their respective run files.

**Step 2 — Accuracy results (`results.json`):**

```json
{
  "runs": [
    { "run_index": 1, "correct": 7, "total": 9, "accuracy": 0.78 },
    { "run_index": 2, "correct": 3, "total": 9, "accuracy": 0.33 },
    { "run_index": 3, "correct": 8, "total": 8, "accuracy": 1.0 }
  ]
}
```

**Key decisions during processing:**
- **Run assignment**: For each `question_id`, the 3 records were sorted by timestamp; earliest → run 1, middle → run 2, latest → run 3.
- **Error exclusion**: Records with `prediction` or `target` containing (case-insensitive) "Quota Exceeds", "API Error", or "Timeout" were excluded from accuracy computation.
- **Rounding**: Used `Decimal` with `ROUND_HALF_UP` to correctly implement round-half-up to 2 decimal places.
- **Run 1**: q-0003 excluded (generator Timeout) → 7/9 correct.
- **Run 2**: q-0001 excluded (generator API Error) → 3/9 correct.
- **Run 3**: q-0002 excluded (generator Timeout), q-0007 excluded (generator Quota Exceeds) → 8/8 correct.