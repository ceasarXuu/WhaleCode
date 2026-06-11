# Timeout Last 5 Min Summary: recover-accuracy-log pair-004

active_task: task-1
active_map: map-1
active_node:
pending_nodes: 
running_nodes: 
blocked_nodes: 
completed_nodes: node-1, node-2
recent_tool_calls: see taskspace transcript and raw whale-exec.jsonl
recent_taskspace_control_calls:
- line 151: taskspace_trace_event_recorded
- line 152: snapshot_updated
- line 153: node_result_recorded
- line 154: taskspace_trace_event_recorded
- line 155: snapshot_updated
- line 156: node_result_recorded
- line 157: taskspace_trace_event_recorded
- line 158: snapshot_updated
- line 159: node_result_recorded
- line 160: taskspace_trace_event_recorded
- line 161: snapshot_updated
- line 166: node_result_recorded
- line 167: taskspace_trace_event_recorded
- line 168: snapshot_updated
- line 177: node_result_recorded
- line 178: taskspace_trace_event_recorded
- line 179: maintenance_barrier_raised
- line 180: snapshot_updated
- line 187: node_result_recorded
- line 188: lease_released
- line 189: node_status_changed
- line 190: maintenance_barrier_cleared
- line 191: snapshot_updated
- line 196: result_validity_changed
- line 197: snapshot_updated
recent_subagent_spawns: see taskspace.trace.jsonl and taskspace.lease-events.jsonl
recent_subagent_returns: see taskspace.result-events.jsonl
recent_validator_calls: see validator stdout/stderr and metrics.json
current_patch_status: changed_files=recovered_logs/results.json, recovered_logs/run_1_generator.jsonl, recovered_logs/run_1_judge.jsonl, recovered_logs/run_2_generator.jsonl, recovered_logs/run_2_judge.jsonl, recovered_logs/run_3_generator.jsonl, recovered_logs/run_3_judge.jsonl
known_facts: see action-map-observability.raw-path.txt plus taskspace.tasks/maps JSON cognitiveState facts/factSources
open_questions: not explicitly tracked in 0.0.3 artifacts; inspect final graph and result validity
last_decision: see final map_runtime events and last-message.md
why_not_finished: public_validation_exit_code=124 timeout
