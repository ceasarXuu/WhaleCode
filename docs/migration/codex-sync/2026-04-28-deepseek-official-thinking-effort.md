# DeepSeek official thinking effort alignment

## Trigger

The TUI model picker showed `deepseek-v4-pro medium` immediately after model
selection because Whale's bundled DeepSeek presets exposed only one local
reasoning level: `medium`.

## Decision

DeepSeek model presets now advertise only the official Chat Completions
thinking efforts documented by DeepSeek:

- `high`
- `max`

Whale does not map local labels such as `medium` or `xhigh` onto DeepSeek
values for bundled DeepSeek models. The persisted and model-visible effort is
the same value sent to the API.

## Runtime contract

For Chat Completions providers, Whale now emits DeepSeek thinking controls in
the request body:

- `thinking: { "type": "enabled" }`
- `reasoning_effort: "high"` or `"max"`

`ReasoningEffort::None` remains a direct disabled-thinking escape hatch and is
serialized as `thinking: { "type": "disabled" }` without `reasoning_effort`.

## Source

- DeepSeek API `create-chat-completion` documents `thinking` and
  `reasoning_effort` as Chat Completions request fields:
  https://api-docs.deepseek.com/api/create-chat-completion
