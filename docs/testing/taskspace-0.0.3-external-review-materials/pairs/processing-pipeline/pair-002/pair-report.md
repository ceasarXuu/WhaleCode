# TaskSpace Benchmark Pair Report

- scenario: terminal_bench__processing-pipeline
- level: L3
- requested_evidence_target: E3
- reported_evidence_level: E3-candidate
- oracle_isolation_policy: deferred_materialization_allowed
- valid_pair: True
- included_in_utility_aggregate: False
- left_logical_mode: taskspace
- right_logical_mode: standard
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
- docker_run_failure
- e3_human_review_not_completed
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-002\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-002\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-002\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-002\external-e3-proof.json
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
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-002\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: taskspace_worse
- taskspace_tool_call_ratio: 0.88
- taskspace_wall_time_ratio: 1.87
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
- wall_time_ms: 495008
- tool_call_count: 38
- changed_paths: collect_data.sh, generate_report.sh, output/final_report.txt, output/processed_data.txt, output/raw_data.txt, process_data.sh
- changed_file_inventory: collect_data.sh[ M] sha256=ad3e1b0a52358fecc2aa1118190ed818595f00457a2f5cabffd62fa93499e169 size=264; generate_report.sh[ M] sha256=82feb0635eaadf52057c2e958b0cd23c9466c8a80bc59fffccd027c27087f28e size=475; output/final_report.txt[??] sha256=5861b3a5e0d2105271d25258e48c853209abfef6ec6b514155f0de43f8c6a938 size=91; output/processed_data.txt[??] sha256=c252eaec6fb6f876c0b3f1d594c861d154e05b56fe2d1b20de9caba6ef21f18f size=10; output/raw_data.txt[??] sha256=0c15e883dee85bb2f3540a47ec58f617a2547117f9096417ba5422268029f501 size=10; process_data.sh[ M] sha256=e18e0cff9197ad3af1c3fdded0a63778a883e4db808e2c719317c40169aa295b size=317
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: docker_run_failure
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-002\left\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 1
- nodes: 9
- edges: 11
- edge_order_violations: 0
- spawn_agent_calls: 2
- subagent_results: 9
- open_leaf_nodes: 0
- ordinary_before_binding: False

## right / standard
- business_success: True
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 0
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 264966
- tool_call_count: 43
- changed_paths: Dockerfile, generate_report.sh, run_pipeline.sh
- changed_file_inventory: Dockerfile[ M] sha256=77f0cc539e09abfa9b9ee354784d4f0b6d80867b61ed5fec125ab949ff1d1800 size=570; generate_report.sh[ M] sha256=7016526278c61eb2f43ba62416fc0be00f341ae4b73e44aed0453e793ca35d15 size=494; run_pipeline.sh[ M] sha256=a375758c4a17c1e2f44611a8fe5e59d9f6e0100dac2b08c9c52d81d7bee021de size=836
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: 
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-002\right\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 0
- nodes: 0
- edges: 0
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False
