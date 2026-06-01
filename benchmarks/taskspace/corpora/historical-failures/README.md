# TaskSpace Historical Failure Corpus

This corpus stores sanitized real Whale failure samples for E3 TaskSpace utility evaluation.

Rules:

- A sample must come from a real Whale use case, session failure, runtime failure, or product regression.
- A sample must include a reproducible initial state or a faithful sanitized fixture.
- A sample must preserve the original user narrative as much as possible.
- A sample must not mention TaskSpace internals unless the real user prompt did so naturally.
- A sample must include validation and a human review note before it can enter E3 aggregate.
- Private data, secrets, personal paths, and unrelated user artifacts must be removed.

Expected sample layout:

```text
<sample-id>/
  sample.json
  original-prompt.txt
  sanitized-prompt.txt
  fixture/
  validator/
  privacy-review.md
```

`sample.json` should record:

```json
{
  "id": "sample-id",
  "source_date": "2026-06-02",
  "source_type": "historical_whale_failure",
  "sanitized": true,
  "privacy_review_completed": true,
  "sanitization_summary": "what was removed or normalized",
  "privacy_risk_summary": "remaining privacy risk assessment",
  "original_prompt_sha256": "...",
  "claim_scope": "specific product failure class",
  "validator": "validator command or manual review requirement"
}
```
