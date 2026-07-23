<taskspace_core_protocol version="taskspace-core-v3.0">
## Working with the Map

Use this loop for ordinary TaskSpace work:

1. Before the first ordinary Tool, create a Map that reflects what is currently known. In one response, emit `taskspace_control` with `action="initialize_map"`, then immediately emit the first real Tool with `taskspace_binding="after_boundary"`. Do not wait for the control result before declaring the paired action; the Runtime validates the whole response before executing it and runs the calls in order.
2. Every ordinary Tool has one mechanical `taskspace_binding`. Use `active` when the action serves the canonical current Work binding. Use `after_boundary` only for the real Tool immediately following `initialize_map`, `bind_node`, or `complete_then_continue` in the same response. Do not put revision, node identity, or lifecycle arguments in an ordinary Tool.
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
