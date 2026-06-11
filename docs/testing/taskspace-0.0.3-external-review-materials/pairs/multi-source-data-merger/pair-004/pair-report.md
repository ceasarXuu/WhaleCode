# TaskSpace Benchmark Pair Report

- scenario: terminal_bench__multi-source-data-merger
- level: L3
- requested_evidence_target: E3
- reported_evidence_level: E1
- oracle_isolation_policy: deferred_materialization_allowed
- valid_pair: True
- included_in_utility_aggregate: False
- left_logical_mode: taskspace
- right_logical_mode: standard
- included_in_e3_aggregate: False

## Evidence Gate Failures
- business_success_false
- manual_review_required

## E3 Gate
- sample_origin_type: external_benchmark
- human_review_required: True
- human_review_completed: False
- human_review_decision: 
- human_review_disagreement: False
- e3_minimum_repeats: 5
- claim_scope: Terminal-Bench coding/file/debug/data-processing subset
- declared_validator_runtime: terminal_bench_equivalent_docker_app
- declared_official_runner_or_equivalent: True
- declared_agent_cannot_read_validator_source: True
- declared_validator_e3_eligible: True
- declared_validator_downgrade_reason: 
- public_validation_timeout
- e3_external_validator_fidelity_unproven
- e3_external_validator_not_e3_eligible
- e3_human_review_not_completed
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-004\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-004\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-004\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-004\external-e3-proof.json
- proof_official_runner_or_equivalent: False
- proof_agent_cannot_read_validator_source: True
- proof_validator_e3_eligible: False
- audit_review_source_path: 
- audit_review_failures: audit_review_missing

## Variable Control
- failures: none

## Prompt Guard
- invalid_prompt: False
- manual_review_required: True
- hard_hits: 
- context_hits: Map

## Oracle Isolation Probe
- oracle_isolation_level: hard_deferred_materialization
- canary_leaked: False
- canary_materialized_during_probe: False
- path_mentioned: True
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-004\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: both_failed_or_inconclusive
- taskspace_tool_call_ratio: 2
- taskspace_wall_time_ratio: 4.26
- taskspace_tool_call_ratio_warn: False
- taskspace_wall_time_ratio_warn: False
- note: excluded evidence is diagnostic only; it does not prove paired comparability or TaskSpace utility.

## left / taskspace
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 124
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 307049
- tool_call_count: 22
- changed_paths: __pycache__/test_merge_output.cpython-312-pytest-9.0.3.pyc.18684, conflicts.json, merge_users.py, merged_users.parquet, run_merge.bat, test_merge_output.py
- changed_file_inventory: __pycache__/test_merge_output.cpython-312-pytest-9.0.3.pyc.18684[??] sha256=4ea91736142b03b68aad0ae30f0e808a13eb5f5cedadc77e3ba819e04a288b87 size=14280; conflicts.json[??] sha256=89ae1fdb9418bfe94d307ad11bd508f6c23c7dddc92896380586363f3f2f994b size=732; merge_users.py[??] sha256=4546324e2ff04aa3dbda34961e6a0b7220ec95e88286bc505352086efba05dfc size=7426; merged_users.parquet[??] sha256=3ceb43b02a6febf2178630e209cf98fff71cc33535cfe0ef53151b01df57ddc7 size=3419; run_merge.bat[??] sha256=c75cd5001a33585f91fd9091afbb4cd72ea76f8900e9ad43555145c7ffa897da size=174; test_merge_output.py[??] sha256=8a05e7cd1ced340a3b6d6d404f1920f1f5d99e340f97287d6561d87d87f1f60f size=2618
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: public_validation_timeout
- docker_build_result_path: 
- validator_environment_mismatch: False
- maps: 1
- nodes: 7
- edges: 6
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False

## right / standard
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 124
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 72074
- tool_call_count: 11
- changed_paths: conflicts.json, merged_users.parquet
- changed_file_inventory: conflicts.json[??] sha256=94808c758b54f9c530e2f1d8e26da43b45cffa9e489b4b55ab84d618c3408d65 size=669; merged_users.parquet[??] sha256=cd0bfb86d53d9d968482b898534415e86040d93506be7a96537cdbc30b918142 size=1694
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: public_validation_timeout
- docker_build_result_path: 
- validator_environment_mismatch: False
- maps: 0
- nodes: 0
- edges: 0
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False
