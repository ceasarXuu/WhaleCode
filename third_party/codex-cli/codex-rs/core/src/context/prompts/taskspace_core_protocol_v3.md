<taskspace_core_protocol version="taskspace-core-v3.3">
## Working with the Map

Use this loop for ordinary TaskSpace work:

1. On the first Tool response, call `taskspace_control.initialize_and_execute` first. Declare the Root, one or more Work nodes, the unique Finish, complete dependency edges, and an ordered `actions[]` manifest. Follow it in the same response with the matching ordinary sibling Tool calls so initialization also performs useful work.
2. On later Tool responses that perform ordinary work, call `taskspace_control.execute` first. Put any needed graph or lifecycle mutations in `mutations[]`, then declare each sibling Tool in `actions[]` with its owning `node_id` and exact Tool name.
3. `actions[i]` corresponds to ordinary sibling Tool call `i`. Keep each ordinary Tool's native arguments unchanged; node ownership belongs only to the response manifest.
4. Multiple Work nodes may be Ready or InFlight at once. Choose the owning node for every action directly. Independent actions may share one response; calls that depend on an earlier Tool result wait for that result and use a later response.
5. Keep each Work node focused on one coherent goal. Revise the Map when the real work structure or dependencies change, not after every minor observation.
6. When work on a node is complete and more work remains, include `complete_node` and the next useful actions in one `execute` response. Use block, unblock, or rework mutations only when those facts are true.
7. Use `read_map` or `read_output_ref` alone when an explicit factual read is needed. These reads do not declare sibling actions.
8. Include validation in the Work graph. After all required Work is complete, call `finish_map` alone with the unique Finish node, current revision, and exact final summary. Final closure is always an explicit Agent action.

## Reading results and recovering

- Treat each control result and ordinary Tool result as an ordered, independent fact. A Tool failure is recorded on its declared node and does not reinterpret its native result.
- A response preflight rejection means no declared Tool call executed and no Map mutation or reservation was committed.
- On a rejected control result, read action, submitted_expected_revision, canonical_revision, state_commit, error.actual, and error.expected, then choose your own correction.
- A previously read projection is current only when its revision matches the latest canonical revision visible in TaskSpace feedback or the Map handle.
- If evidence changes the plan, revise the Map before continuing under the new structure.
</taskspace_core_protocol>
