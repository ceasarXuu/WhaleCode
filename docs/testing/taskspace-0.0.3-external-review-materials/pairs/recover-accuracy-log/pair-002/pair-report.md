# TaskSpace Benchmark Pair Report

- scenario: terminal_bench__recover-accuracy-log
- level: L3
- requested_evidence_target: E3
- reported_evidence_level: E2-candidate
- oracle_isolation_policy: deferred_materialization_allowed
- valid_pair: True
- included_in_utility_aggregate: False
- left_logical_mode: taskspace
- right_logical_mode: standard
- included_in_e3_aggregate: False

## Evidence Gate Failures
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
- e3_human_review_not_completed
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__recover-accuracy-log\20260608-235934-100\pair-002\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__recover-accuracy-log\20260608-235934-100\pair-002\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__recover-accuracy-log\20260608-235934-100\pair-002\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__recover-accuracy-log\20260608-235934-100\pair-002\external-e3-proof.json
- proof_official_runner_or_equivalent: True
- proof_agent_cannot_read_validator_source: True
- proof_validator_e3_eligible: True
- audit_review_source_path: 
- audit_review_failures: audit_review_missing

## Variable Control
- failures: none

## Prompt Guard
- invalid_prompt: False
- manual_review_required: True
- hard_hits: 
- context_hits: parallel

## Oracle Isolation Probe
- oracle_isolation_level: hard_deferred_materialization
- canary_leaked: False
- canary_materialized_during_probe: False
- path_mentioned: True
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__recover-accuracy-log\20260608-235934-100\pair-002\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: both_success_cost_within_budget
- taskspace_tool_call_ratio: 0.7
- taskspace_wall_time_ratio: 2.8
- taskspace_tool_call_ratio_warn: False
- taskspace_wall_time_ratio_warn: False
- note: excluded evidence is diagnostic only; it does not prove paired comparability or TaskSpace utility.

## left / taskspace
- business_success: True
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 0
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 657507
- tool_call_count: 16
- changed_paths: process_logs.py, raw_logs/generator.log, raw_logs/judge.log, recovered_logs/.done, recovered_logs/results.json, recovered_logs/run_1_generator.jsonl, recovered_logs/run_1_judge.jsonl, recovered_logs/run_2_generator.jsonl, recovered_logs/run_2_judge.jsonl, recovered_logs/run_3_generator.jsonl, recovered_logs/run_3_judge.jsonl
- changed_file_inventory: process_logs.py[??] sha256=2c90709a810c37ec7091fc8d5be888103c7001b1d0a5a21a7806026b8a20d0d6 size=4825; raw_logs/generator.log[??] sha256=22220e0728e4fe43e654d6e2679c14f03acc32dd1ce1f6e0828c0c492de5bc5b size=2745; raw_logs/judge.log[??] sha256=8c6e7ee5b374163047546550bba57026c8c5b32b2bfd6fc18e25ca35c13fbf6d size=2602; recovered_logs/.done[??] sha256=dc4dab0dc60d26a73992cabb8204684c0f8887235795f6841c3f053b2c3f0c2c size=41; recovered_logs/results.json[??] sha256=18397079484731f44d18c73058a561f5566dc2bd4ae9b63628a6fc29eed408bf size=306; recovered_logs/run_1_generator.jsonl[??] sha256=b14421fa68f174231bb08d0dc969ae6ac67242f89bfee3bbc851fcd6b8f09991 size=861; recovered_logs/run_1_judge.jsonl[??] sha256=24d5de9b51cba201cb3fcfdf83c94770a7ed98c8317a459a46cf2186f3853728 size=817; recovered_logs/run_2_generator.jsonl[??] sha256=32da93c63a755730307acc09307b77dfa0d64afa8310437fd5b087894296cae1 size=862; recovered_logs/run_2_judge.jsonl[??] sha256=123881dd5835c5e77c3f8cb9c49cdb606ecb1e76ce5cf3ecb4a2c60df85de89d size=817; recovered_logs/run_3_generator.jsonl[??] sha256=5ba71a314287d289c49eae9274399fc71e104712f76d51df26092e545149b2ee size=872; recovered_logs/run_3_judge.jsonl[??] sha256=7fde291b8f003b4469fe91f04b9fded0df23c02f93d1e5d03f4fa0a088c97d62 size=818
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: 
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__recover-accuracy-log\20260608-235934-100\pair-002\left\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 1
- nodes: 15
- edges: 15
- edge_order_violations: 0
- spawn_agent_calls: 3
- subagent_results: 14
- open_leaf_nodes: 1
- ordinary_before_binding: False

## right / standard
- business_success: True
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 0
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 234510
- tool_call_count: 23
- changed_paths: raw_logs/generator.log, raw_logs/judge.log, recovered_logs/results.json, recovered_logs/run_1_generator.jsonl, recovered_logs/run_1_judge.jsonl, recovered_logs/run_2_generator.jsonl, recovered_logs/run_2_judge.jsonl, recovered_logs/run_3_generator.jsonl, recovered_logs/run_3_judge.jsonl
- changed_file_inventory: raw_logs/generator.log[??] sha256=22220e0728e4fe43e654d6e2679c14f03acc32dd1ce1f6e0828c0c492de5bc5b size=2745; raw_logs/judge.log[??] sha256=8c6e7ee5b374163047546550bba57026c8c5b32b2bfd6fc18e25ca35c13fbf6d size=2602; recovered_logs/results.json[??] sha256=39b280206aa38e2925bdc1e0f01069d32ed3f0f2d65747871f78a7a5d8256c67 size=326; recovered_logs/run_1_generator.jsonl[??] sha256=8e63e74935fe81669c8fe71eda096b5fd19ee07b9dd2ae241f1aaee1f1f83fc3 size=871; recovered_logs/run_1_judge.jsonl[??] sha256=532dfaba71487d0495d841d87f788395b086fe0e26ec2b06d92ef4de8663cf73 size=827; recovered_logs/run_2_generator.jsonl[??] sha256=2a888effc41825ae901275f5fd119c346b3a7c7b13b324bec4c84f36e751ad11 size=872; recovered_logs/run_2_judge.jsonl[??] sha256=322be0ee36caed444eaae0292088da31756ee2a82a73965bff9890a4c060d48e size=827; recovered_logs/run_3_generator.jsonl[??] sha256=70ff5c178c59ae91dffcd1e76787bd73a9726ceb060e2fc8bc29fc7688bfabbd size=882; recovered_logs/run_3_judge.jsonl[??] sha256=42d553e86f0322001d525c344697e067ea94a0395fc7234906d7d475cc334e89 size=828
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: 
- docker_build_result_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__recover-accuracy-log\20260608-235934-100\pair-002\right\artifacts\external-validator-runtime\docker-build-result.json
- validator_environment_mismatch: False
- maps: 0
- nodes: 0
- edges: 0
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False
