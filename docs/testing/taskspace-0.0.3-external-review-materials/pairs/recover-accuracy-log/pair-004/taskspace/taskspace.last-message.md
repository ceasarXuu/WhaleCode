All output files have been created and validated. Here's a summary of the work:

## Summary

### Step 1: Per-run log recovery

For each role (generator/judge), I grouped the 30 events by `question_id`, sorted each group by timestamp, and assigned:
- **Earliest** → run 1
- **Middle** → run 2  
- **Latest** → run 3

Then sorted each run's 10 events by timestamp and wrote them as compact JSONL files.

**Output files (6 files):**
| File | Lines | Status |
|------|-------|--------|
| `recovered_logs/run_1_generator.jsonl` | 10 | ✓ sorted, compact JSON |
| `recovered_logs/run_1_judge.jsonl` | 10 | ✓ sorted, compact JSON |
| `recovered_logs/run_2_generator.jsonl` | 10 | ✓ sorted, compact JSON |
| `recovered_logs/run_2_judge.jsonl` | 10 | ✓ sorted, compact JSON |
| `recovered_logs/run_3_generator.jsonl` | 10 | ✓ sorted, compact JSON |
| `recovered_logs/run_3_judge.jsonl` | 10 | ✓ sorted, compact JSON |

### Step 2: Accuracy computation

For each run, compared prediction vs target as integers, excluding pairs where either side contained an error marker (`Quota Exceeds`, `API Error`, `Timeout`). Accuracy rounded half-up to 2 decimals using `Decimal.ROUND_HALF_UP`.

**`recovered_logs/results.json`:**
| Run | Correct | Total | Accuracy |
|-----|---------|-------|----------|
| 1 | 7 | 9 | 0.78 |
| 2 | 3 | 9 | 0.33 |
| 3 | 8 | 8 | 1.00 |

Errors excluded per run:
- **Run 1:** q-0003 generator had "Timeout" (1 error)
- **Run 2:** q-0001 generator had "API Error" (1 error)
- **Run 3:** q-0007 generator had "Quota Exceeds", q-0002 generator had "Timeout" (2 errors)