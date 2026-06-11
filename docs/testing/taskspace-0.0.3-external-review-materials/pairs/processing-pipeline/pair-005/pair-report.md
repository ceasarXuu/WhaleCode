# TaskSpace Benchmark Pair Report

- scenario: terminal_bench__processing-pipeline
- level: L3
- requested_evidence_target: E3
- reported_evidence_level: E3-candidate
- oracle_isolation_policy: deferred_materialization_allowed
- valid_pair: True
- included_in_utility_aggregate: False
- left_logical_mode: standard
- right_logical_mode: taskspace
- included_in_e3_aggregate: False

## Evidence Gate Failures
- none

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
- e3_human_review_not_completed
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-005\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-005\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-005\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-005\external-e3-proof.json
- proof_official_runner_or_equivalent: True
- proof_agent_cannot_read_validator_source: True
- proof_validator_e3_eligible: True
- audit_review_source_path: 
- audit_review_failures: audit_review_missing

## Variable Control
- failures: none

## Prompt Guard
- invalid_prompt: False
- manual_review_required: False
- hard_hits: 
- context_hits: 

## Oracle Isolation Probe
- oracle_isolation_level: hard_deferred_materialization
- canary_leaked: False
- canary_materialized_during_probe: False
- path_mentioned: True
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-005\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: both_success_cost_within_budget
- taskspace_tool_call_ratio: 0.83
- taskspace_wall_time_ratio: 2.54
- taskspace_tool_call_ratio_warn: False
- taskspace_wall_time_ratio_warn: False
- note: excluded evidence is diagnostic only; it does not prove paired comparability or TaskSpace utility.

## left / standard
- business_success: True
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 0
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 297518
- tool_call_count: 52
- changed_paths: Dockerfile, generate_report.sh
- changed_file_inventory: Dockerfile[ M] sha256=b83bd1885a010bd411374c7824ed1792065dd6f23522cafa1ab5d7d1cc4a77eb size=451; generate_report.sh[ M] sha256=cc75a694a0bc4c3c38a409162f17f2673c5f770ce62af0f277e9306e1da5f658 size=495
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: 
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-005\left\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 0
- nodes: 0
- edges: 0
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False

## right / taskspace
- business_success: True
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 0
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 756249
- tool_call_count: 43
- changed_paths: Dockerfile, generate_report.sh, generate_report.sh.new
- changed_file_inventory: Dockerfile[ M] sha256=dd801eee59f28f7743d58ad1a451d7fe831ce4582233c97767fd658417c58b44 size=450; generate_report.sh[ M] sha256=cc75a694a0bc4c3c38a409162f17f2673c5f770ce62af0f277e9306e1da5f658 size=495; generate_report.sh.new[??] sha256=cc75a694a0bc4c3c38a409162f17f2673c5f770ce62af0f277e9306e1da5f658 size=495
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: 
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-005\right\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 1
- nodes: 16
- edges: 44
- edge_order_violations: 0
- spawn_agent_calls: 4
- subagent_results: 28
- open_leaf_nodes: 0
- ordinary_before_binding: False
