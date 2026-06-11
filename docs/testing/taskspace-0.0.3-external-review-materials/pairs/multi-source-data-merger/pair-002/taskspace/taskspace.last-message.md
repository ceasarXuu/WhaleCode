## Merge Complete — Final Summary

All three data sources have been successfully merged, validated, and output to the workspace.

### Data Sources

| Source | Format | Priority | Records | Users |
|--------|--------|----------|---------|-------|
| source_a/users.json | JSON | Highest | 2 | 101, 102 |
| source_b/users.csv | CSV | Medium | 2 | 101, 103 |
| source_c/users.parquet | Parquet | Lowest | 2 | 101, 104 |

### Field Mappings Applied

- `id` / `userId` / `user_id` → **user_id** (integer)
- `full_name` / `name` / `userName` → **name** (string)
- `email` / `email_address` → **email** (string)
- `registration_date` / `created_at` / `joined` → **created_date** (YYYY-MM-DD)
- `status` / `is_active` / `active` → **status** (string, optional)

### Output Files

**`merged_users.parquet`** — 4 unique users with conflicts resolved by priority:

| user_id | name | email | created_date | status |
|---------|------|-------|-------------|--------|
| 101 | John Doe | john@a.com | 2024-01-15 | active |
| 102 | Jane Smith | jane@a.com | 2024-02-20 | inactive |
| 103 | Bob Wilson | bob@b.com | 2024-03-01 | true |
| 104 | Alice Brown | alice@c.com | 2024-04-01 | true |

**`conflicts.json`** — 4 conflicts detected (all for user 101, who appeared in all three sources):

| Field | source_a | source_b | source_c | Selected |
|-------|----------|----------|----------|----------|
| name | John Doe | John Doe | John D. | John Doe |
| email | john@a.com | john@b.com | john@c.com | john@a.com |
| created_date | 2024-01-15 | 2024-01-10 | 2024-01-20 | 2024-01-15 |
| status | active | true | true | active |

### Validation

**9/9 pytest tests passed** covering:
- Parquet file schema and data types
- All 4 unique user IDs present
- Date format compliance (YYYY-MM-DD)
- Conflict report structure and count
- Conflict resolution priority (source_a > source_b > source_c)
- User 101 field values confirmed from highest priority source