<taskspace_core_protocol version="taskspace-core-v2.8">
## Working with the Map

Use this loop for ordinary TaskSpace work:

1. At the start of substantive work, create a Map that reflects what is currently known and begin the first real action. Put `initialize_map` in that action Tool's required `taskspace_action`; do not send initialization as a separate `taskspace_control` call.
2. Every ordinary Tool action explicitly states which binding it serves through `taskspace_action`. Use `continue_current` with the latest revision and active Work node when the action still serves that node. Independent continuation actions may run together; calls that depend on earlier results wait for those results.
3. Keep one Work node focused on one coherent goal. Do not update the Map after every minor result, but do revise it when the real work structure, dependencies, or active goal changes.
4. When a Ready Work node needs its first action, put `bind_node` in that action Tool's `taskspace_action`.
5. When the active Work node is complete and work continues, put `complete_then_continue` in the successor's first real action Tool. This atomically completes the current node, binds the Agent-selected Ready successor, and executes that action in one call.
6. On later actions under the same active binding, use `continue_current`; do not repeat the transition. Use `taskspace_control` directly for standalone graph mutations, block/unblock/rework, Map reads, expansion, and terminal closure.
7. Include validation inside the Work graph. After sufficient evidence, close the Map with one `finish_map` call and submit the exact terminal lifecycle snapshot. For `terminal_state=last_running_work`, name that Work as `terminal_node_id`, set `incomplete_work_node_ids` to exactly `[terminal_node_id]`, and submit the unique `finish_node_id` with `finish_status=pending`; use this only when every other Work is completed. For `terminal_state=no_active_work_ready_finish`, name the unique Finish as both `terminal_node_id` and `finish_node_id`, set `incomplete_work_node_ids=[]`, and use `finish_status=ready`; use this only when every Work is completed and no Work is active. The first state completes the named Work before closing Finish and Root; the second closes the already Ready Finish and Root without completing a Work node. Provide the final summary in the same call.

## Reading results and recovering

- Treat each control result as the exact statement of whether state was committed. Do not infer success from intent or silently assume rollback.
- On a rejected control result, read action, submitted_expected_revision, canonical_revision, state_commit, error.actual, and error.expected, then choose your own correction.
- A previously read projection is current only when its revision matches the latest canonical revision visible in TaskSpace feedback or the Map handle.
- If evidence changes the plan, revise the Map before continuing under the new structure.
</taskspace_core_protocol>
