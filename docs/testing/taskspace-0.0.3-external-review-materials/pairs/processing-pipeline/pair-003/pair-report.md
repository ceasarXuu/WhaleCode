# TaskSpace Benchmark Pair Report

- scenario: terminal_bench__processing-pipeline
- level: L3
- requested_evidence_target: E3
- reported_evidence_level: E1
- oracle_isolation_policy: deferred_materialization_allowed
- valid_pair: True
- included_in_utility_aggregate: False
- left_logical_mode: standard
- right_logical_mode: taskspace
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
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-003\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-003\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-003\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-003\external-e3-proof.json
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
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-003\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: both_failed_or_inconclusive
- taskspace_tool_call_ratio: 0.68
- taskspace_wall_time_ratio: 2.64
- taskspace_tool_call_ratio_warn: False
- taskspace_wall_time_ratio_warn: False
- note: excluded evidence is diagnostic only; it does not prove paired comparability or TaskSpace utility.

## left / standard
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 1
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 335250
- tool_call_count: 56
- changed_paths: collect_data.sh, generate_report.sh, output/final_report.txt, output/processed_data.txt, output/raw_data.txt, process_data.sh, run_pipeline.sh, test_pipeline.py
- changed_file_inventory: collect_data.sh[ M] sha256=422b6f7240e6e23d2a4c2d2c5e96dc8cb0c224b32f8f4bfecb2cd0b1edf7775e size=266; generate_report.sh[ M] sha256=f88561b7a1a64170f4ebdec3565b7af542f2bd374b6d6e205ca3144b6589e7e8 size=479; output/final_report.txt[??] sha256=064f65d782657f2574eb442fc14102d9cc1c8cfca1e65b4f873b28d066392d7a size=91; output/processed_data.txt[??] sha256=c252eaec6fb6f876c0b3f1d594c861d154e05b56fe2d1b20de9caba6ef21f18f size=10; output/raw_data.txt[??] sha256=0c15e883dee85bb2f3540a47ec58f617a2547117f9096417ba5422268029f501 size=10; process_data.sh[ M] sha256=d6e7aaa00a0222e2a1849e3126a1b9fdb622b1574e43f21293ff964a25105be2 size=319; run_pipeline.sh[ M] sha256=f2bc2964c7ac229e53ef4b4141c298bb31d277f8e4d004fa78b1a373f914097b size=561; test_pipeline.py[??] sha256=f8a8e3cae74a7d52805ccda55e357e6e8e053eaab1c3a956b419a4fa85b9462f size=2793
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: docker_run_failure
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-003\left\artifacts\external-validator-runtime\docker-build-result.json
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
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 1
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 884845
- tool_call_count: 38
- changed_paths: collect_data.sh, generate_report.sh, output/final_report.txt, output/processed_data.txt, output/raw_data.txt, process_data.sh, run_pipeline.ps1
- changed_file_inventory: collect_data.sh[ M] sha256=ad3e1b0a52358fecc2aa1118190ed818595f00457a2f5cabffd62fa93499e169 size=264; generate_report.sh[ M] sha256=edbb6348ceeaa03d7d6e6a36cbca8601665583ef8c758e3b26f04b0c1ab8f6c8 size=463; output/final_report.txt[??] sha256=c7070988496710cc46569fffb8da42ad1976f389b0f41a084e13ab7839cd96b0 size=91; output/processed_data.txt[??] sha256=c252eaec6fb6f876c0b3f1d594c861d154e05b56fe2d1b20de9caba6ef21f18f size=10; output/raw_data.txt[??] sha256=0c15e883dee85bb2f3540a47ec58f617a2547117f9096417ba5422268029f501 size=10; process_data.sh[ M] sha256=f27f762d44219380dc9b8e9c0c27f66886e9fda933d8012614afd41ef6a9ebbe size=313; run_pipeline.ps1[??] sha256=964399bcd196e6e752f72afaf1a8496d33b7d5bdea1234482f2d359f693891bc size=1260
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: docker_run_failure
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__processing-pipeline\20260608-202555-620\pair-003\right\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 2
- nodes: 20
- edges: 29
- edge_order_violations: 0
- spawn_agent_calls: 6
- subagent_results: 58
- open_leaf_nodes: 1
- ordinary_before_binding: False
