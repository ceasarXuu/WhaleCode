<taskspace_core_protocol version="taskspace-core-v3.2">
## Working with the Map

Use this loop for ordinary TaskSpace work:

1. Make the first real Tool initialize the Map and perform useful work in one call. Set that ordinary Tool's `taskspace_binding` to the `initialize_map` object with the Root, initial Work, Finish identity, other known Work nodes, and complete explicit edges. Do not emit a standalone `taskspace_control initialize_map`; that action exists only in the first ordinary Tool's binding.
2. Every later ordinary Tool has one mechanical `taskspace_binding` object. Use `{"action":"active"}` when the action serves the canonical current Work binding. Use `{"action":"after_boundary"}` only for the real Tool immediately following `bind_node` or `complete_then_continue` in the same response. Do not put revision, node identity, or lifecycle arguments in an `active` or `after_boundary` binding.
3. Independent `active` actions may run together. Calls that depend on an earlier Tool result wait for that result and use a later response.
4. Keep one Work node focused on one coherent goal. Do not update the Map after every minor result, but revise it when the real work structure, dependencies, or active goal changes.
5. When a Ready Work node needs its first action, emit `bind_node` and immediately follow it with that action using `after_boundary`.
6. When the active Work node is complete and work continues, emit `complete_then_continue` and immediately follow it with the successor's first action using `after_boundary`. The Runtime commits the Agent-selected lifecycle change before dispatching the action.
7. Use `taskspace_control` alone for graph mutations, block/unblock/rework, Map reads, expansion, and terminal closure.
8. Include validation inside the Work graph. After sufficient evidence, close the Map with one `finish_map` call. Name the current final Running Work as `terminal_node_id`; the same atomic transaction completes it and closes Finish and Root. If no Work remains active and the unique Finish is already Ready, such as after the final subagent result, name that Finish instead. The Runtime validates the submitted revision, selected node, binding, and canonical terminal frontier. Provide the final summary in the same call.

## Reading results and recovering

- Treat each control and ordinary Tool result as an ordered, independent fact. A committed boundary is not rolled back when the paired Tool fails.
- A sequence preflight rejection means no declared Tool call executed.
- On a rejected control result, read action, submitted_expected_revision, canonical_revision, state_commit, error.actual, and error.expected, then choose your own correction.
- A previously read projection is current only when its revision matches the latest canonical revision visible in TaskSpace feedback or the Map handle.
- If evidence changes the plan, revise the Map before continuing under the new structure.
</taskspace_core_protocol>
