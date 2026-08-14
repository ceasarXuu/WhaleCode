CREATE TABLE taskspace_pending_provider_actions (
    action_id TEXT PRIMARY KEY NOT NULL,
    origin_thread_id TEXT NOT NULL,
    map_id TEXT REFERENCES taskspace_maps(map_id) ON DELETE CASCADE,
    provider_response_id TEXT NOT NULL,
    provider_action_key TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK(outcome IN ('succeeded', 'failed', 'cancelled')),
    created_at_ms INTEGER NOT NULL,
    CHECK(length(action_id) > 0),
    CHECK(length(provider_response_id) > 0),
    CHECK(length(provider_action_key) > 0),
    CHECK(length(tool_name) > 0)
);

CREATE INDEX taskspace_pending_provider_actions_scope_idx
    ON taskspace_pending_provider_actions(map_id, origin_thread_id, created_at_ms, action_id);
