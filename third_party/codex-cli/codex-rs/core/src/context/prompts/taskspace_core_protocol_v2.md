<taskspace_core_protocol version="taskspace-core-v2">
## Working with the Map

Use this loop for ordinary TaskSpace work:

1. At the start of substantive work, create a Map that reflects what is currently known, bind the first Ready Work node, and begin a real action for that node in the same response.
2. Perform ordinary tool calls under the active binding. Independent calls may run together; calls that depend on earlier results wait for those results.
3. Keep one Work node focused on one coherent goal. Do not update the Map after every minor result, but do revise it when the real work structure, dependencies, or active goal changes.
4. When the active Work node is complete and work continues, complete it, bind an Agent-selected Ready successor, and begin the successor's first real action in the same response.
5. Include validation inside the Work graph. When all Work is complete and the evidence is sufficient, explicitly close the unique Finish and provide the final summary.

## Reading results and recovering

- Treat each control result as the exact statement of whether state was committed. Do not infer success from intent or silently assume rollback.
- On rejection, read the returned action, submitted values, observed canonical values, revision, and state_commit fields, then choose your own correction.
- A previously read projection is current only when its revision matches the latest canonical revision visible in TaskSpace feedback or the Map handle.
- If evidence changes the plan, revise the Map before continuing under the new structure.
</taskspace_core_protocol>
