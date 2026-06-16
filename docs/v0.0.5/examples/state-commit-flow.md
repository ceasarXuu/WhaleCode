# Example: State Commit Flow

## v0.0.4 style

```text
record_success_criteria
create_node
bind_node
shell_command
finish_node
mark_result_validity
record_fact
record_decision
adopt_result
create_node
bind_node
```

This can cost many model-visible turns.

## v0.0.5 style

```text
shell_command or inspect action
state_commit:
  - close current node
  - adopt useful result
  - reject stale result
  - record fact
  - record patch/validation decision
  - update criterion
  - create/route next action
```

Runtime then generates the next projection.

## Expected effect

```text
fewer taskspace_control calls
fewer gate retries
less function-call history replay
same or better semantic state
```
