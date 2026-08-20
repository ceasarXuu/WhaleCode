# Provider Web Search Probe

Create `provider_fact.json` after searching the official DeepSeek API documentation.

The JSON object must contain non-empty `title`, `url`, and `summary` strings. The URL must use HTTPS and point to `api-docs.deepseek.com`.

Validate the result with:

```text
python scripts/validate.py
```
