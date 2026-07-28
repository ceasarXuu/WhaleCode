<taskspace_core_protocol version="taskspace-core-v3.4">
## Working with the Map

Use this loop for ordinary TaskSpace work:

1. On the first Tool response, call `taskspace_control.initialize_and_execute` first. Declare the Root, one or more Work nodes, the unique Finish, complete dependency edges, and an ordered `actions[]` manifest. Follow it in the same response with the matching ordinary sibling Tool calls so initialization also performs useful work.
2. On later Tool responses that perform ordinary work, call `taskspace_control.execute` first. Put any needed graph or lifecycle mutations in `mutations[]`, then declare each sibling Tool in `actions[]` with its owning `node_id` and exact Tool name.
3. `actions[i]` corresponds to ordinary sibling Tool call `i`. Keep each ordinary Tool's native arguments unchanged; node ownership belongs only to the response manifest.
4. Multiple Work nodes may be Ready or InFlight at once. Choose the owning node for every action directly. Independent actions may share one response; calls that depend on an earlier Tool result wait for that result and use a later response.
5. Keep each Work node focused on one coherent goal. Revise the Map when the real work structure or dependencies change, not after every minor observation.
6. When work on a node is complete and more work remains, include `complete_node` and the next useful actions in one `execute` response. Use block or unblock mutations only when those facts are true. A completed Work node and its result or evidence facts are immutable.
7. Use `read_map` or `read_output_ref` alone when an explicit factual read is needed. These reads do not declare sibling actions.
8. Include validation in the Work graph. When the remaining final Work is complete, call `finish_map` alone with the current revision, unique Finish node, every final Work node completed by this transaction in `complete_work_node_ids[]`, and the exact summary. This atomically records those Work completions and closes Finish and Root.
9. If the user provides follow-up work after the Map is closed, continue the same Map with `reopen_map`. Declare one or more new Work nodes, the dependency edges that connect them into the existing Root-to-Finish DAG, and immediate sibling actions in the same response. Reopen preserves the prior terminal in history and never rewrites completed Work.

## Reading results and recovering

- Treat each control result and ordinary Tool result as an ordered, independent fact. A Tool failure is recorded on its declared node and does not reinterpret its native result.
- A response preflight rejection means no declared Tool call executed and no Map mutation or reservation was committed.
- On a rejected control result, read action, submitted_expected_revision, canonical_revision, state_commit, error.actual, and error.expected, then choose your own correction.
- A previously read projection is current only when its revision matches the latest canonical revision visible in TaskSpace feedback or the Map handle.
- A closed Map is reopened only in response to later user work. An active Map continues through `execute`; it is not reopened.
- If evidence changes the plan, revise the Map before continuing under the new structure.
</taskspace_core_protocol>
