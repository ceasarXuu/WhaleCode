---

## Data Merge Complete

Three user data sources have been successfully merged with priority-based conflict resolution.

### Input Sources

| Source | File | Priority | Format |
|--------|------|----------|--------|
| A | `data/source_a/users.json` | Highest | JSON |
| B | `data/source_b/users.csv` | Secondary | CSV |
| C | `data/source_c/users.parquet` | Tertiary | Parquet |

### Field Mappings Applied

| Unified Field | Source A | Source B | Source C |
|---|---|---|---|
| `user_id` | `id` | `user_id` | `userId` |
| `name` | `full_name` | `name` | `userName` |
| `email` | `email` | `email_address` | `email` |
| `created_date` | `registration_date` | `created_at` | `joined` |
| `status` | `status` | `is_active` | `active` |

Status values were normalized to `"active"` / `"inactive"` strings across all sources.

### Merged Dataset (`merged_users.parquet`)

| user_id | name | email | created_date | status |
|---|---|---|---|---|
| 101 | John Doe | john@a.com | 2024-01-15 | active |
| 102 | Jane Smith | jane@a.com | 2024-02-20 | inactive |
| 103 | Bob Wilson | bob@b.com | 2024-03-01 | active |
| 104 | Alice Brown | alice@c.com | 2024-04-01 | active |

All 4 unique users from all sources included. Data types: `user_id` (int64), `name`/`email`/`created_date`/`status` (string). All dates in YYYY-MM-DD format.

### Conflicts (`conflicts.json`)

3 conflicts detected for **user 101** (present in all 3 sources):

| Field | Source A | Source B | Source C | Selected |
|---|---|---|---|---|
| `name` | John Doe | John Doe | John D. | John D. |
| `email` | john@a.com | john@b.com | john@c.com | john@a.com |
| `created_date` | 2024-01-15 | 2024-01-10 | 2024-01-20 | 2024-01-15 |

No conflicts for status (all sources resolved to "active") or for users 102, 103, 104 (each present in only one source).

### Verification

- **9/9 pytest tests passed**, validating: file existence, column schema, data types, date format, all unique users included, priority resolution (source A values used for user 101), conflict report format and content.

### Files Created

- `merge_users.py` - Data merge pipeline script
- `test_merge_pipeline.py` - Comprehensive validation tests
- `merged_users.parquet` - Final merged dataset
- `conflicts.json` - Conflict report