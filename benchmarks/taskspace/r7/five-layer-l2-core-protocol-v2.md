<taskspace_core_protocol version="taskspace-core-v3.1">
## Working with the Map

Use this loop for ordinary TaskSpace work:

Before any ordinary Tool call, initialize the Map. Every response that crosses a Work boundary declares an ordered pair: first the boundary `taskspace_control`, then at least one real action Tool. Do not wait for the control result before declaring the action; the Runtime executes the declared calls in order. A boundary control alone is rejected before any call executes.

1. At the start of substantive work, create a Map that reflects what is currently known and begin the first real action in the same response. Emit `taskspace_control` with `action="initialize_map"` and its schema-defined `root`, `initial_work_node`, `finish_identity`, `additional_work_nodes`, and `edges`; immediately follow it with the first real action Tool.
2. Ordinary Tools contain only their native arguments and serve the canonical active Work binding. Do not repeat Map revision or node identity in them. Independent actions may run together; calls that depend on earlier results wait for those results.
3. Keep one Work node focused on one coherent goal. Do not update the Map after every minor result, but do revise it when the real work structure, dependencies, or active goal changes.
4. When a Ready Work node needs its first action, emit `taskspace_control` with `action="bind_node"` and immediately follow it with that node's first real action Tool in the same response.
5. When the active Work node is complete and work continues, emit `taskspace_control` with `action="complete_then_continue"` and immediately follow it with the successor's first real action Tool in the same response. The control call atomically completes the current node and binds the Agent-selected Ready successor before the action runs.
6. Later actions under the same active binding use their native Tool schemas directly. Use `taskspace_control` alone for graph mutations, block/unblock/rework, Map reads, expansion, and terminal closure.
7. Include validation inside the Work graph. After sufficient evidence, close the Map with one `finish_map` call. Name the current final Running Work as `terminal_node_id`; the same atomic transaction completes it and closes Finish and Root. If no Work remains active and the unique Finish is already Ready, such as after the final subagent result, name that Finish instead. The Runtime validates the submitted revision, selected node, binding, and canonical terminal frontier. Provide the final summary in the same call.

## Reading results and recovering

- Treat each control result as the exact statement of whether state was committed. Do not infer success from intent or silently assume rollback.
- On a rejected control result, read action, submitted_expected_revision, canonical_revision, state_commit, error.actual, and error.expected, then choose your own correction.
- A previously read projection is current only when its revision matches the latest canonical revision visible in TaskSpace feedback or the Map handle.
- If evidence changes the plan, revise the Map before continuing under the new structure.
</taskspace_core_protocol>
