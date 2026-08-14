# U17 收口失败逐项清单

- 被测生产提交：`4f4f5d4c55bb527fb842fa4076117ae79badf79d`
- 分支：`whalecode-codex`
- 工作空间：仅 `<WORKSPACE>`（日志提交前仅将本机绝对路径、主机名和临时 installation id 做确定性脱敏；测试输出、断言和计数保持原样）
- 日期：2026-08-15
- 环境：Linux `7.0.0-28-generic` x86_64；Rust/Cargo `1.96.1`；just `1.55.1`
- 真实模型/API 请求：0

## 1. 命令与结果

命令均在 `third_party/codex-cli/` 执行；`just test` 使用仓库 `local` nextest profile 和 `RUST_MIN_STACK=8388608`。

| 矩阵 | 精确命令 | 结果 | 脱敏原始日志 |
| --- | --- | --- | --- |
| app-server | `just test -p codex-app-server --status-level fail --final-status-level fail` | 1122 run；1089 passed（1 flaky）；33 failed；1 skipped | `app-server.log` |
| core lib | `just test -p codex-core --lib --status-level fail --final-status-level fail` | 2178 run；2154 passed；24 failed | `core-lib.log` |
| core integration | `just test -p codex-core --test all --status-level fail --final-status-level fail` | 1123 run；1086 passed（1 flaky）；37 failed；8 skipped | `core-integration.log` |
| 代理隔离复跑 | 清除大小写 `HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY` 后运行 `just test -p codex-core --lib user_shell_commands_do_not_inherit_managed_network_proxy --status-level fail --final-status-level fail` | 1 passed | `core-lib-proxy-isolated-rerun.log` |

分类依据：`plan.md` 的 D2/Phase C/U3/U6/U17 边界要求保持 DeepSeek-only 公共模型目录，且不在本轮启用 Apps、remote plugins/sharing、remote Code Mode、audio/image/realtime、OpenAI hosted 或 Bedrock 产品面。下列分类表示“明确不在本轮发布合同且未声明通过”，不表示相应上游能力通过。

## 2. app-server：33 项

### AS-BEDROCK：Bedrock/default account/static catalog（3）

映射：U17 明确延期的 Bedrock 产品面；原始断言要求 Bedrock 默认账户或 GPT static catalog，与 Whale DeepSeek 默认合同冲突。

- `suite::v2::account::logout_managed_bedrock_restores_default_account`
- `suite::v2::thread_start::thread_start_provider_model_fallback_applies_to_configured_model`
- `suite::v2::thread_start::thread_start_provider_model_fallback_uses_bedrock_static_catalog`

### AS-OPENAI-CATALOG：OpenAI/ChatGPT 公共模型目录（4）

映射：U6 的公共列表仅展示 DeepSeek；这些用例要求 OpenAI hidden、remote catalog、pagination 或 source-of-truth 语义。

- `suite::v2::model_list::list_models_includes_hidden_models`
- `suite::v2::model_list::list_models_pagination_works`
- `suite::v2::model_list::list_models_returns_all_models_with_large_limit`
- `suite::v2::model_list::list_models_uses_chatgpt_remote_catalog_as_source_of_truth`

### AS-REMOTE-PLUGIN：OpenAI hosted remote plugin catalog（14）

映射：U3 将 remote plugin 默认关闭，U17 明确不交付该 hosted 产品面。三条 30 秒失败是在等待被关闭的 startup/force refresh，不是 watcher 或 TaskSpace 失败。

- `suite::v2::plugin_list::app_server_startup_refreshes_cached_remote_catalog_without_blocking_plugin_list`
- `suite::v2::plugin_list::app_server_startup_skips_disabled_remote_plugin_catalog_scopes`
- `suite::v2::plugin_list::plugin_installed_includes_created_by_me_when_remote_plugins_enabled`
- `suite::v2::plugin_list::plugin_installed_prefers_api_curated_conflicts_after_switching_to_api_auth`
- `suite::v2::plugin_list::plugin_installed_prefers_remote_curated_conflicts_when_remote_plugin_enabled`
- `suite::v2::plugin_list::plugin_list_fetches_shared_with_me_kind`
- `suite::v2::plugin_list::plugin_list_fetches_user_plugins_in_created_by_me_remote_marketplace`
- `suite::v2::plugin_list::plugin_list_force_refetch_bypasses_fresh_global_remote_catalog_cache`
- `suite::v2::plugin_list::plugin_list_honors_global_remote_catalog_cache_ttl`
- `suite::v2::plugin_list::plugin_list_includes_remote_marketplaces_when_remote_plugin_enabled`
- `suite::v2::plugin_list::plugin_list_marks_remote_plugin_disabled_by_admin`
- `suite::v2::plugin_list::plugin_list_preserves_plan_ineligible_remote_plugin_metadata`
- `suite::v2::plugin_list::plugin_list_sync_upgrades_and_removes_remote_installed_plugin_bundles`
- `suite::v2::plugin_list::plugin_list_vertical_kind_noops_when_remote_plugin_enabled`

### AS-PLUGIN-SHARE：OpenAI hosted plugin sharing（10）

映射：U3 将 plugin sharing 默认关闭，U17 明确不交付该 hosted 产品面。

- `suite::v2::plugin_share::plugin_share_checkout_adds_personal_marketplace_entry`
- `suite::v2::plugin_share::plugin_share_checkout_cleans_up_path_when_marketplace_update_fails`
- `suite::v2::plugin_share::plugin_share_checkout_rejects_non_share_remote_plugin`
- `suite::v2::plugin_share::plugin_share_rejects_workspace_targets_from_client`
- `suite::v2::plugin_share::plugin_share_save_forwards_access_policy`
- `suite::v2::plugin_share::plugin_share_save_rejects_access_policy_for_existing_plugin`
- `suite::v2::plugin_share::plugin_share_save_rejects_listed_discoverability`
- `suite::v2::plugin_share::plugin_share_save_uploads_local_plugin`
- `suite::v2::plugin_share::plugin_share_update_targets_publishes_workspace_plugin`
- `suite::v2::plugin_share::plugin_share_update_targets_updates_share_targets`

### AS-RECOMMENDED-PLUGIN：外部登录后的 hosted 推荐插件等待（2）

映射：依赖本轮明确关闭的 OpenAI hosted recommended plugins。

- `suite::v2::recommended_plugins::first_turn_after_external_login_waits_for_recommended_plugins`
- `suite::v2::recommended_plugins::first_turn_after_external_login_waits_for_recommended_plugins_without_tool_suggest`

## 3. core lib：24 项

### CL-GUARDIAN：上游 Guardian/OpenAI 自动审查模型夹具（17）

映射：本轮不把上游 Guardian/OpenAI review model 产品面接到 DeepSeek 默认 provider。失败签名是缺少 `DEEPSEEK_API_KEY`、期望 GPT review model/catalog、请求未进入预置 OpenAI mock 或相应计数为 0；不涉及 TaskSpace fork/state 断言。

- `guardian::tests::guardian_ephemeral_retry_preserves_parallel_trunk_and_fork_history`
- `guardian::tests::guardian_reused_trunk_ignores_stale_prior_turn_completion`
- `guardian::tests::guardian_reuses_prompt_cache_key_and_appends_prior_reviews`
- `guardian::tests::guardian_review_does_not_retry_missing_assessment_payload`
- `guardian::tests::guardian_review_does_not_retry_valid_denial`
- `guardian::tests::guardian_review_exhausts_three_failures_with_one_terminal_event`
- `guardian::tests::guardian_review_records_missing_auto_review_model_in_analytics_metadata`
- `guardian::tests::guardian_review_request_layout_matches_model_visible_request_snapshot`
- `guardian::tests::guardian_review_retries_transient_session_failure_then_approves`
- `guardian::tests::guardian_review_retries_two_parse_failures_then_approves`
- `guardian::tests::guardian_review_session_config_clears_context_overrides_for_distinct_effective_model`
- `guardian::tests::guardian_review_surfaces_responses_api_errors_in_rejection_reason`
- `guardian::tests::guardian_review_uses_model_catalog_override_when_preferred_review_model_exists`
- `guardian::tests::guardian_review_uses_preferred_review_model_without_model_catalog_override`
- `mcp_tool_call::tests::guardian_mode_mcp_denial_returns_rationale_message`
- `session::tests::guardian_tests::request_permissions_routes_to_guardian_when_reviewer_is_enabled`
- `session::tests::guardian_tests::strict_auto_review_turn_grant_forces_guardian_for_shell_command_policy_skip`

### CL-REMOTE-PLUGIN：远程插件 discoverability（1）

- `plugins::discoverable::tests::list_tool_suggest_discoverable_plugins_includes_cached_remote_global_plugins`

映射：同 `AS-REMOTE-PLUGIN`。

### CL-HOST-ENV：宿主代理变量污染（1）

- `session::tests::user_shell_commands_do_not_inherit_managed_network_proxy`

首轮读取宿主 `HTTP_PROXY=http://127.0.0.1:7890`，期望 `not-set`；显式清除大小写代理变量后同一精确测试 1/1 通过。归因为测试进程未隔离宿主代理环境，不是产品回归。

### CL-OPENAI-MODEL-MANAGER：上游远程模型刷新（2）

- `thread_manager::tests::injected_models_manager_controls_refresh_policy`
- `thread_manager::tests::new_uses_active_provider_for_model_refresh`

映射：测试要求 OpenAI remote catalog refresh 计数；Whale 默认 DeepSeek-only catalog 不发起该刷新。

### CL-GPT-SUBAGENT：硬编码 GPT service-tier 夹具（2）

- `tools::handlers::multi_agents::tests::spawn_agent_service_tier_inheritance_preserves_supported_or_configured_tiers`
- `tools::handlers::multi_agents::tests::spawn_agent_service_tier_override_validates_the_effective_child_model`

映射：断言显式使用 `gpt-5.4`/`gpt-5.4-mini`，生产目录按 U6 只暴露 DeepSeek；失败发生在型号校验，不是 TaskSpace lineage。

### CL-HOSTED-IMAGE：独立 image generation 产品开关（1）

- `tools::spec_plan::tests::hosted_web_search_and_standalone_image_generation_follow_runtime_gates`

映射：Phase A–E 明确不启用独立 image generation 新产品能力。

## 4. core integration：37 项

### CI-GUARDIAN：上游 Guardian review model 选择（2）

- `suite::guardian_review::guardian_session_prewarms_and_is_reused_for_first_review::api_key_uses_luna_with_responses_lite`
- `suite::guardian_review::guardian_session_prewarms_and_is_reused_for_first_review::chatgpt_uses_codex_auto_review`

映射：测试期望 `gpt-5.6-luna` / `codex-auto-review`，当前 DeepSeek 默认路径使用其目录 fallback；同 `CL-GUARDIAN`。

### CI-OPENAI-CATALOG：OpenAI remote catalog/cache/selectors/personality（21）

映射：这些测试都向 OpenAI remote model cache 注入 GPT/remote model，并等待其进入公共目录、选择器、人格或 token-window 路径；U6 的 DeepSeek-only 公共目录有意拒绝该前提。

- `suite::injected_models_cache::injected_cache_error_falls_back_for_agent_model_selection`
- `suite::injected_models_cache::injected_cache_hit_drives_agent_model_selection`
- `suite::model_runtime_selectors::multi_agent_config_precedence_overrides_remote_model_selector`
- `suite::model_runtime_selectors::remote_code_mode_only_selector_fails_closed_when_host_is_disabled`
- `suite::model_runtime_selectors::remote_tool_mode_selector_overrides_feature_flags`
- `suite::model_runtime_selectors::unsupported_code_mode_warning_is_emitted_each_turn`
- `suite::model_switching::model_switch_to_smaller_model_updates_token_context_window`
- `suite::models_cache_ttl::refreshes_when_cache_version_differs`
- `suite::models_cache_ttl::refreshes_when_cache_version_missing`
- `suite::models_cache_ttl::renews_cache_ttl_on_matching_models_etag`
- `suite::models_cache_ttl::uses_cache_when_version_matches`
- `suite::personality::remote_model_friendly_personality_instructions_with_feature`
- `suite::personality::user_turn_personality_remote_model_template_includes_update_message`
- `suite::remote_models::remote_models_apply_legacy_instructions`
- `suite::remote_models::remote_models_do_not_append_removed_builtin_presets`
- `suite::remote_models::remote_models_hide_picker_only_models`
- `suite::remote_models::remote_models_merge_adds_new_high_priority_first`
- `suite::remote_models::remote_models_remote_model_uses_unified_exec`
- `suite::remote_models::remote_models_truncation_policy_with_tool_output_override`
- `suite::remote_models::remote_models_truncation_policy_without_override_preserves_remote`
- `suite::spawn_agent_description::spawn_agent_description_lists_visible_models_and_reasoning_efforts`

### CI-GPT-SUBAGENT：硬编码 GPT/Luna 子 Agent 与 summary 夹具（14）

映射：所有用例在请求或 mock 等待中显式使用 `gpt-5.4`、`gpt-5.6-sol`、Luna 或对应 GPT summary 元数据；失败签名为“Unknown model”或等待该 GPT 请求超时。当前 DeepSeek-only catalog 拒绝这些型号是 U6 合同，不表示通用 spawn/fork 路径失败。

- `suite::subagent_notifications::multi_agent_v2_spawn_sends_agent_message_to_child::legacy_encrypted_leaf`
- `suite::subagent_notifications::multi_agent_v2_spawn_sends_agent_message_to_child::luna_encrypted_leaf`
- `suite::subagent_notifications::spawn_agent_preserves_configured_defaults_through_unrelated_role`
- `suite::subagent_notifications::spawn_agent_rejects_reasoning_effort_unsupported_by_role_model`
- `suite::subagent_notifications::spawn_agent_requested_model_and_reasoning_override_inherited_settings_without_role`
- `suite::subagent_notifications::spawn_agent_role_overrides_requested_model_and_reasoning_settings`
- `suite::subagent_notifications::spawn_agent_uses_configured_subagent_defaults`
- `suite::subagent_notifications::spawn_agent_uses_independent_configured_subagent_defaults::model_only`
- `suite::subagent_notifications::spawned_agent_uses_summary_support_for_final_model::supported_child`
- `suite::subagent_notifications::spawned_agent_uses_summary_support_for_final_model::unsupported_child`
- `suite::subagent_notifications::spawned_child_receives_forked_parent_context::legacy`
- `suite::subagent_notifications::spawned_child_receives_forked_parent_context::paginated`
- `suite::subagent_notifications::spawned_full_history_v2_child_uses_model_precedence_without_dropping_context::configured_default_with_omitted_fork_turns`
- `suite::subagent_notifications::spawned_full_history_v2_child_uses_model_precedence_without_dropping_context::explicit_override_with_fork_turns_all`

## 5. 审计结论

- 三个矩阵共 94 个失败名，清单计数为 33 + 24 + 37，全部穷举。
- 93 项映射到已有产品/计划边界；1 项宿主代理污染经隔离精确复跑通过。
- 当前日志中没有 TaskSpace 测试失败，也没有未分类失败。
- IC-B01/IC-B02 的修复另有真实 SQLite migration、extension fork relation 与 process-level app-server `thread/fork` 回归通过证据；不能用本清单替代这些正向测试。
- 本清单不把延期能力表述为通过，也不证明 Windows、完整 TUI、live cache、OpenAI hosted、Bedrock 或 Guardian 产品面可发布。

## 6. 对抗性审查后的生产组合补证

Round 2 指出最初的 fork 测试只证明 production registry + SQLite binding，尚未把 RPC activation、reload 与 final-wire 放在同一条链中。随后扩展现有 `thread_fork_inherits_taskspace_through_production_extensions`：

1. 真实 app-server 发送 Standard turn 并捕获首个 Responses body；
2. 真实 SQLite 写入 canonical map，再通过 typed `thread/mapRuntimeMode/set` 与 `thread/taskspace/read` 验证激活；
3. 通过 typed `thread/fork` 验证 `Fork` binding；
4. graceful shutdown 后启动第二个 app-server，resume fork 并再次通过 typed read 验证 map；
5. fork 上发送 turn，捕获第二个 Responses body，验证 Standard 无 TaskSpace、fork/reload 后存在 `taskspace_control`、`<taskspace_map>` 与同一 map id。

定向命令 `just test -p codex-app-server thread_fork_inherits_taskspace_through_production_extensions --status-level fail --final-status-level fail` 为 1/1 passed。该补证只修改测试，不改变上文 exact production commit 和 94 项全量失败集合。
