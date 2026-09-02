-- TaskSpace v2 and R8 use incompatible canonical models. Preserve v2 rows
-- losslessly for offline recovery instead of inventing a lossy conversion.
DROP INDEX IF EXISTS taskspace_map_bindings_map_id_idx;
DROP INDEX IF EXISTS taskspace_map_commits_map_revision_idx;
ALTER TABLE taskspace_map_commits RENAME TO taskspace_v2_map_commits;
ALTER TABLE taskspace_map_bindings RENAME TO taskspace_v2_map_bindings;
ALTER TABLE taskspace_maps RENAME TO taskspace_v2_maps;

CREATE TABLE taskspace_maps (
    map_id TEXT PRIMARY KEY NOT NULL,
    owner_thread_id TEXT NOT NULL,
    schema_version TEXT,
    map_revision INTEGER CHECK(map_revision >= 1),
    canonical_sha256 TEXT NOT NULL,
    store_revision INTEGER NOT NULL CHECK(store_revision >= 1),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    CHECK((schema_version IS NULL AND map_revision IS NULL)
       OR (schema_version IS NOT NULL AND map_revision IS NOT NULL))
);

CREATE TABLE taskspace_map_nodes (
    map_id TEXT NOT NULL REFERENCES taskspace_maps(map_id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('root', 'work', 'finish')),
    position INTEGER NOT NULL CHECK(position >= 0),
    goal TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('waiting', 'ready', 'in_flight', 'blocked', 'completed')),
    content TEXT NOT NULL,
    PRIMARY KEY(map_id, node_id),
    UNIQUE(map_id, role, position)
);

CREATE UNIQUE INDEX taskspace_map_single_root_idx
    ON taskspace_map_nodes(map_id) WHERE role = 'root';

CREATE UNIQUE INDEX taskspace_map_single_finish_idx
    ON taskspace_map_nodes(map_id) WHERE role = 'finish';

CREATE TABLE taskspace_map_node_parents (
    map_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    parent_node_id TEXT NOT NULL,
    position INTEGER NOT NULL CHECK(position >= 0),
    PRIMARY KEY(map_id, node_id, parent_node_id),
    UNIQUE(map_id, node_id, position),
    FOREIGN KEY(map_id, node_id)
        REFERENCES taskspace_map_nodes(map_id, node_id) ON DELETE CASCADE,
    FOREIGN KEY(map_id, parent_node_id)
        REFERENCES taskspace_map_nodes(map_id, node_id) ON DELETE CASCADE
);

CREATE TABLE taskspace_map_node_actions (
    map_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    position INTEGER NOT NULL CHECK(position >= 0),
    tool_name TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK(outcome IN ('pending', 'succeeded', 'failed', 'cancelled')),
    PRIMARY KEY(map_id, node_id, action_id),
    UNIQUE(map_id, node_id, position),
    FOREIGN KEY(map_id, node_id)
        REFERENCES taskspace_map_nodes(map_id, node_id) ON DELETE CASCADE
);

CREATE INDEX taskspace_map_node_parents_parent_idx
    ON taskspace_map_node_parents(map_id, parent_node_id);

CREATE INDEX taskspace_map_node_actions_node_idx
    ON taskspace_map_node_actions(map_id, node_id);

CREATE INDEX taskspace_map_node_actions_identity_idx
    ON taskspace_map_node_actions(map_id, action_id);

CREATE TABLE taskspace_map_bindings (
    thread_id TEXT PRIMARY KEY NOT NULL,
    map_id TEXT NOT NULL REFERENCES taskspace_maps(map_id) ON DELETE CASCADE,
    relation TEXT NOT NULL CHECK(relation IN ('owner', 'resume', 'fork', 'child')),
    parent_thread_id TEXT,
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
    canonical_sha256 TEXT NOT NULL,
    request_sha256 TEXT NOT NULL,
    operation TEXT NOT NULL,
    actor_thread_id TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX taskspace_map_commits_map_revision_idx
    ON taskspace_map_commits(map_id, result_store_revision);
