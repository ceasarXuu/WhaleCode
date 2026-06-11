## Merge Complete — All Validations Passed

### Data Sources Merged
Three data sources with different schemas were read, mapped, and merged:

| Source | Format | Records | Priority |
|--------|--------|---------|----------|
| `data/source_a/users.json` | JSON | 2 users (101, 102) | **Highest** |
| `data/source_b/users.csv` | CSV | 2 users (101, 103) | Secondary |
| `data/source_c/users.parquet` | Parquet | 2 users (101, 104) | Tertiary |

### Field Mappings
- `id` / `user_id` / `userId` → **`user_id`** (integer)
- `full_name` / `name` / `userName` → **`name`** (string)
- `email` / `email_address` → **`email`** (string)
- `registration_date` / `created_at` / `joined` → **`created_date`** (YYYY-MM-DD)
- `status` / `is_active` / `active` → **`status`** (string: "active"/"inactive")

### Merged Dataset (`merged_users.parquet`)
| user_id | name | email | created_date | status |
|---------|------|-------|-------------|--------|
| **101** | John Doe | john@a.com | 2024-01-15 | active |
| **102** | Jane Smith | jane@a.com | 2024-02-20 | inactive |
| **103** | Bob Wilson | bob@b.com | 2024-03-01 | active |
| **104** | Alice Brown | alice@c.com | 2024-04-01 | active |

All 4 unique users included. Conflicts resolved by priority (source_a values used when available).

### Conflicts Detected (`conflicts.json`)
**3 conflicts** — all for user 101 (present in all three sources):

| Field | source_a ✓ | source_b | source_c |
|-------|-----------|---------|---------|
| **name** | John Doe | John Doe | John D. |
| **email** | john@a.com | john@b.com | john@c.com |
| **created_date** | 2024-01-15 | 2024-01-10 | 2024-01-20 |

No conflicts for users 102, 103, 104 (each appears in only one source).

### Validation
All **13 pytest tests passed**, verifying:
- Correct row count (4), column names, and data types
- All field values match expected merged output
- Conflict report structure, `total_conflicts` == list length
- All selected values come from the highest-priority source (source_a)

### Output Files (in `W:\app\`)
| File | Size | Description |
|------|------|-------------|
| `merged_users.parquet` | 3.4 KB | Merged dataset, 4 rows, 5 columns |
| `conflicts.json` | 732 B | Conflict report with 3 entries |
| `merge_users.py` | 7.4 KB | Reusable merge script |
| `test_merge_output.py` | 2.4 KB | Pytest validation suite |