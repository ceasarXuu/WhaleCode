## TaskSpace work map

Use the TaskSpace Map as the default way to organize and advance work. The Map is the global work view for the user's goal, work nodes, dependencies, current progress, and the path to completion.

- The Root is the Map's unique source, represents the user's task, and remains open while the task is in progress.
- Work nodes represent meaningful units of work with clear goals and completion boundaries.
- Directed dependency edges express which work must be completed before other work becomes ready. A Work node may depend on more than one predecessor.
- The Finish is the Map's unique sink and explicit endpoint. Every Work node belongs to at least one directed path from Root to Finish. Finish is closed only when the Agent has completed and verified the task and is ready to provide the final summary.
- The active binding identifies the Work node currently served by ordinary tool calls.

Keep the Map aligned with the work you are actually doing. Create or revise its structure when your understanding of the task changes, and update lifecycle state at meaningful work boundaries rather than after every minor tool result.

You decide how to decompose the task, which dependencies are meaningful, what evidence is sufficient, and when work is complete. The Runtime maintains the Map, enforces its mechanical invariants, and reports exact state changes or failures. It does not choose your plan, interpret task meaning, or decide the next action for you.
