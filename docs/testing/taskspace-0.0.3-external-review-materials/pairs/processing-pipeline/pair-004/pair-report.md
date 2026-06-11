# TaskSpace Benchmark Pair Report

- scenario: terminal_bench__processing-pipeline
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
- docker_run_failure
- e3_human_review_not_completed
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-004\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-004\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-004\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-004\external-e3-proof.json
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
- path_mentioned: False
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-004\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: both_failed_or_inconclusive
- taskspace_tool_call_ratio: 0.36
- taskspace_wall_time_ratio: 0.81
- taskspace_tool_call_ratio_warn: False
- taskspace_wall_time_ratio_warn: False
- note: excluded evidence is diagnostic only; it does not prove paired comparability or TaskSpace utility.

## left / taskspace
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 1
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 324656
- tool_call_count: 25
- changed_paths: collect_data.sh, data/output/final_report.txt, data/output/processed_data.txt, data/output/raw_data.txt, generate_report.sh, process_data.sh
- changed_file_inventory: collect_data.sh[ M] sha256=97395cdc344b74c410bf5f6b7f115f5f075bcc8d2f94b24513912d1c9ad17e9a size=276; data/output/final_report.txt[??] sha256=c8b358f0cdf46f5fa767e81b6969d80a0ae4d6c740bc320a58496474cf348cf6 size=91; data/output/processed_data.txt[??] sha256=c252eaec6fb6f876c0b3f1d594c861d154e05b56fe2d1b20de9caba6ef21f18f size=10; data/output/raw_data.txt[??] sha256=0c15e883dee85bb2f3540a47ec58f617a2547117f9096417ba5422268029f501 size=10; generate_report.sh[ M] sha256=ba0bac108c2b7cd6d9a4f055ef78fc8cca16f930b7a765efac5e017afba9662d size=504; process_data.sh[ M] sha256=11a701bc3a232f592611a33c5a8feda4d5b420e14ca839a218f5643e553d8833 size=334
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: docker_run_failure
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-004\left\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 1
- nodes: 6
- edges: 9
- edge_order_violations: 0
- spawn_agent_calls: 2
- subagent_results: 5
- open_leaf_nodes: 1
- ordinary_before_binding: False

## right / standard
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 1
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 401804
- tool_call_count: 70
- changed_paths: collect_data.sh, data/output/final_report.txt, data/output/processed_data.txt, data/output/raw_data.txt, Dockerfile, generate_report.sh, process_data.sh
- changed_file_inventory: collect_data.sh[ M] sha256=c1dce0befd3dc71a37de5fdd539c019d9e01e1425406377ad5c59fe120d3f8c8 size=269; data/output/final_report.txt[??] sha256=0a95be02cac1945694eaee7a0a817efd50fb5e71822f6eece7088d6d8069cf8d size=91; data/output/processed_data.txt[??] sha256=c252eaec6fb6f876c0b3f1d594c861d154e05b56fe2d1b20de9caba6ef21f18f size=10; data/output/raw_data.txt[??] sha256=0c15e883dee85bb2f3540a47ec58f617a2547117f9096417ba5422268029f501 size=10; Dockerfile[ M] sha256=bdbab174e7ba497e202cd1eb8368ea127d1f271fa142ba4c39ceee8f1bf6b75e size=450; generate_report.sh[ M] sha256=7bbcf9eca507b21756aa1304c59c3403cc6ae7df2b42f503263282c67aaf232d size=489; process_data.sh[ M] sha256=8cfadbe0627c2dfecd9be7bee1913af95161033fb0deedbf410578b739062cb0 size=325
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: docker_run_failure
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-004\right\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 0
- nodes: 0
- edges: 0
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False
