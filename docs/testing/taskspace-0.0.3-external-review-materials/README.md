# TaskSpace 0.0.3 External Review Evidence Package

Generated: 2026-06-11T20:47:10

Source run root: `D:\whalecode-alpha\target\bench004-20260608-202551`

This package contains real 0.0.3 E3 evidence derived from `bench004-20260608-202551`.

## Entry Points

- `e3-pair-index.csv` / `e3-pair-index.jsonl`
- `pairs/<sample>/pair-XXX/`
- `pairs/<sample>/pair-XXX/taskspace/taskspace.trace.jsonl`
- `pairs/<sample>/pair-XXX/taskspace/taskspace.graph.*.json`
- `code-evidence/`
- `focus-packages/`
- `aggregate/e3-aggregate-raw-index.json`

## Limits

- Full sandbox directories and dependency trees are not copied.
- Large raw `rollout.jsonl` files are referenced through `*.raw-artifact-paths.json`.
- Clean utility inclusion remains false for all executed pairs because manual artifact audit was not completed.
