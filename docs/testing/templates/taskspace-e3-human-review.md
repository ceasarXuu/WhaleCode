# TaskSpace E3 Artifact Audit Review

This review can be completed by Codex, an independent reviewer agent, or a human engineer. The reviewer must inspect the recorded artifacts rather than rely on the executing agent's self-report.

scenario:
pair:
reviewer:
date:

## Source Check

- sample_origin_type:
- source_is_real_or_faithfully_sanitized:
- privacy_review_ok:
- original_prompt_preserved:
- taskspace_methodology_leak:

## Outcome Check

- standard_business_success:
- taskspace_business_success:
- standard_key_failure_or_success_reason:
- taskspace_key_failure_or_success_reason:
- validator_result_clear:

## Utility Check

- taskspace_structural_benefit:
- taskspace_observability_useful:
- taskspace_unnecessary_edit_or_noise:
- taskspace_repeated_reading_or_cost_issue:
- cost_acceptable_for_claim_scope:

## Aggregate Decision

decision:

Allowed decisions:

- include_taskspace_better
- include_standard_better
- include_no_clear_delta
- exclude_harness_failure
- exclude_invalid_prompt
- exclude_validator_unclear
- exclude_privacy_or_sample_risk

claim_scope:
notes:
