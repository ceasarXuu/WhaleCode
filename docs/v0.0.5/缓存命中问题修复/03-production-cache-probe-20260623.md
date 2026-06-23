# TaskSpace DeepSeek Production Cache Probe

- Date: 2026-06-23
- Scope: cache-hit validation only
- Run root: `target/deepseek-cache-production-probe/samples-3x2-20260623-152151`
- Matrix report: `target/deepseek-cache-production-probe/samples-3x2-20260623-152151/e2-matrix-report.md`
- Installed binary: `C:\Users\77585\.whale\bin\whale.exe`
- Installed binary SHA256: `D06E5C6B4E9D5FDAE1898FB7F16C2174C28CA3EED054B718F82F7D0EEDCF1077`
- Transport override: none. `WHALE_TASKSPACE_PROVIDER_TRANSPORT` was removed from the process environment before running.

## Method

Ran three real benchmark samples, two repeats each:

```powershell
Remove-Item Env:\WHALE_TASKSPACE_PROVIDER_TRANSPORT -ErrorAction SilentlyContinue
.\scripts\taskspace-benchmark\run-taskspace-e2-matrix.ps1 `
  -Scenarios 'single-file-fast-fix','multi-file-order-pipeline','subscription-billing-repair' `
  -RequiredLevels 'L1','L2' `
  -Repeats 2 `
  -RunRoot target\deepseek-cache-production-probe\samples-3x2-20260623-152151 `
  -WhaleBin "$env:USERPROFILE\.whale\bin\whale.exe" `
  -Model deepseek-v4-flash `
  -TimeoutSeconds 900 `
  -AllowNonE2Result
```

The first two attempts did not reach provider requests because the benchmark preflight marked the installed binary stale by LastWriteTime. The binary content hash was unchanged; after refreshing the installed file timestamp, preflight passed and the production probe below completed.

## Cache Results

| Sample | Repeat | TaskSpace side | Provider requests | Request 2+ count | Request 2+ hit rate | Cached input | Uncached input | Trace coverage | Missing cache usage | Native tools hot path | Tool-free action contract |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `single-file-fast-fix` | `pair-001` | right | 10 | 9 | `0.989095` | 1,065,856 | 11,751 | 1 | 0 | 0 | 10 |
| `single-file-fast-fix` | `pair-002` | left | 9 | 8 | `0.989222` | 947,328 | 10,322 | 1 | 0 | 0 | 9 |
| `multi-file-order-pipeline` | `pair-001` | right | 11 | 10 | `0.989145` | 1,184,640 | 13,000 | 1 | 0 | 0 | 11 |
| `multi-file-order-pipeline` | `pair-002` | left | 11 | 10 | `0.989226` | 1,185,408 | 12,911 | 1 | 0 | 0 | 11 |
| `subscription-billing-repair` | `pair-001` | right | 13 | 12 | `0.988999` | 1,422,208 | 15,820 | 1 | 0 | 0 | 13 |
| `subscription-billing-repair` | `pair-002` | left | 11 | 10 | `0.990220` | 1,185,280 | 11,706 | 1 | 0 | 0 | 11 |

Aggregate:

| Metric | Value |
|---|---:|
| Samples | 3 |
| TaskSpace rounds | 6 |
| Provider requests | 65 |
| Request 2+ cached input tokens | 6,990,720 |
| Request 2+ uncached input tokens | 75,510 |
| Request 2+ hit rate | `0.989314` |
| Minimum per-round request 2+ hit rate | `0.988999` |
| Maximum per-round request 2+ hit rate | `0.990220` |
| Native tools schema hot-path count | 0 for every round |
| Cache usage missing count | 0 for every round |

## Conclusion

The production-style TaskSpace cache probe passed the cache-hit acceptance gates on all six TaskSpace rounds:

- request 2+ hit rate stayed above `0.95`;
- trace coverage was `1`;
- provider cache usage was present;
- native tools schema did not appear in the hot path;
- tool-free action-contract requests were observed.

This document does not evaluate non-cache task outcomes.
