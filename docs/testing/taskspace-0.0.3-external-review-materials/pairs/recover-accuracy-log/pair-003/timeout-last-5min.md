# Timeout Last 5 Min Summary: recover-accuracy-log pair-003

active_task: task-1
active_map: map-1
active_node:
pending_nodes: 
running_nodes: 
blocked_nodes: node-10, node-14, node-15, node-3, node-4, node-5, node-7
completed_nodes: node-1, node-11, node-12, node-13, node-16, node-2, node-6, node-8, node-9
recent_tool_calls: see taskspace transcript and raw whale-exec.jsonl
recent_taskspace_control_calls:
- line 727: snapshot_updated
- line 733: node_result_recorded
- line 734: taskspace_trace_event_recorded
- line 735: snapshot_updated
- line 740: tool_action_blocked
- line 741: snapshot_updated
- line 746: node_result_recorded
- line 747: taskspace_trace_event_recorded
- line 748: snapshot_updated
- line 757: node_result_recorded
- line 758: lease_released
- line 759: node_status_changed
- line 760: snapshot_updated
- line 765: result_validity_changed
- line 766: snapshot_updated
- line 771: node_status_changed
- line 772: node_status_changed
- line 773: lease_created
- line 774: snapshot_updated
- line 779: node_result_recorded
- line 780: lease_released
- line 781: node_status_changed
- line 782: snapshot_updated
- line 787: result_validity_changed
- line 788: snapshot_updated
recent_subagent_spawns: see taskspace.trace.jsonl and taskspace.lease-events.jsonl
recent_subagent_returns: see taskspace.result-events.jsonl
recent_validator_calls: see validator stdout/stderr and metrics.json
current_patch_status: changed_files=recover_accuracy.py, recovered_logs/results.json, recovered_logs/run_1_generator.jsonl, recovered_logs/run_1_judge.jsonl, recovered_logs/run_2_generator.jsonl, recovered_logs/run_2_judge.jsonl, recovered_logs/run_3_generator.jsonl, recovered_logs/run_3_judge.jsonl
known_facts: see action-map-observability.raw-path.txt plus taskspace.tasks/maps JSON cognitiveState facts/factSources
open_questions: not explicitly tracked in 0.0.3 artifacts; inspect final graph and result validity
last_decision: see final map_runtime events and last-message.md
why_not_finished: public_validation_exit_code=124 timeout
