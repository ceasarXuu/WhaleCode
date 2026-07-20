---
name: taskspace-advanced
description: Use for complex TaskSpace work that needs multi-branch DAG design, convergence of multiple prerequisites, long-session replanning, competing debug hypotheses, or recovery after major context compaction. Do not load for small linear tasks whose Map and recovery are already clear.
---

# Advanced TaskSpace Work

Use these methods only when they improve the current task. They are planning heuristics, not Runtime rules.

## Design a useful graph

- Prefer one Work node for one coherent deliverable or decision boundary.
- Keep tightly coupled edits in one node when separating them would create artificial handoffs.
- Use separate branches when work is genuinely independent or when independent evidence should converge before implementation.
- Add multiple incoming edges when a node truly requires several completed prerequisites. Do not create edges only to make the graph look more complex.

## Replan from evidence

- When evidence invalidates the current structure, revise the graph explicitly instead of preserving obsolete nodes as if they were still required.
- Mark blocked or rework state from observed facts. The choice of a new path remains yours.
- Preserve high-value evidence references when old node details are folded.

## Recover a long task

1. Read the current Map and identify the active binding, Ready frontier, blocked work, and latest canonical revision.
2. Inspect only the evidence needed to understand the active and nearest predecessor nodes.
3. Revise the Map if its structure no longer matches the remaining work.
4. Continue from a Ready node; do not replay old actions merely because they appear in history.

## Example: converging prerequisites

For a subscription-cache bug, a useful graph may be:

root -> reproduce
root -> inspect-invalidation-path
reproduce -> identify-root-cause
inspect-invalidation-path -> identify-root-cause
identify-root-cause -> implement-fix
implement-fix -> verify-regression
verify-regression -> finish

If reproduction and code inspection are tightly coupled in the actual task, combining them into one investigation node is also valid. Choose the graph that matches the work rather than optimizing for node count.
