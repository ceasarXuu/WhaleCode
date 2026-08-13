CREATE TABLE taskspace_maps (
    map_id TEXT PRIMARY KEY NOT NULL,
    owner_thread_id TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    snapshot_sha256 TEXT NOT NULL,
    store_revision INTEGER NOT NULL CHECK(store_revision >= 1),
    graph_revision INTEGER NOT NULL CHECK(graph_revision >= 0),
    complete INTEGER NOT NULL CHECK(complete IN (0, 1)),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE taskspace_map_bindings (
    thread_id TEXT PRIMARY KEY NOT NULL,
    map_id TEXT NOT NULL REFERENCES taskspace_maps(map_id) ON DELETE CASCADE,
    relation TEXT NOT NULL CHECK(relation IN ('owner', 'resume', 'fork', 'child')),
    parent_thread_id TEXT,
    node_id TEXT,
    lease_id TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX taskspace_map_bindings_map_id_idx
    ON taskspace_map_bindings(map_id);

CREATE TABLE taskspace_map_commits (
    commit_id TEXT PRIMARY KEY NOT NULL,
    map_id TEXT NOT NULL REFERENCES taskspace_maps(map_id) ON DELETE CASCADE,
    expected_store_revision INTEGER NOT NULL CHECK(expected_store_revision >= 0),
    result_store_revision INTEGER NOT NULL CHECK(result_store_revision >= 1),
    snapshot_sha256 TEXT NOT NULL,
    request_sha256 TEXT NOT NULL,
    operation TEXT NOT NULL,
    actor_thread_id TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX taskspace_map_commits_map_revision_idx
    ON taskspace_map_commits(map_id, result_store_revision);
