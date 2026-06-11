## Results: User Data Merge Complete ✓

### What was done

I created a Python script **`merge_users.py`** that reads three data sources with different schemas, maps fields to a unified schema, merges records by `user_id`, resolves conflicts by source priority (source_a > source_b > source_c), and produces two output files.

### Output files

| File | Path | Description |
|------|------|-------------|
| **Merged dataset** | `W:\app\merged_users.parquet` | 4 users, unified schema |
| **Conflict report** | `W:\app\conflicts.json` | 3 conflicts detected |
| **Script** | `W:\app\merge_users.py` | Reusable merge + validation |

### Merged dataset (4 users)

| user_id | name | email | created_date | status |
|---------|------|-------|-------------|--------|
| 101 | John Doe | john@a.com | 2024-01-15 | active |
| 102 | Jane Smith | jane@a.com | 2024-02-20 | inactive |
| 103 | Bob Wilson | bob@b.com | 2024-03-01 | active |
| 104 | Alice Brown | alice@c.com | 2024-04-01 | active |

### Field mapping applied

| Unified field | source_a | source_b | source_c |
|--------------|----------|----------|----------|
| `user_id` | `id` | `user_id` | `userId` |
| `name` | `full_name` | `name` | `userName` |
| `email` | `email` | `email_address` | `email` |
| `created_date` | `registration_date` | `created_at` | `joined` |
| `status` | `status` | `is_active` | `active` |

### Conflicts resolved (3 for user 101)

- **name**: source_c had "John D." → selected "John Doe" (source_a)
- **email**: all three had different emails → selected "john@a.com" (source_a)
- **created_date**: all three had different dates → selected "2024-01-15" (source_a)

All conflicts correctly use source_a's values due to highest priority. The script includes automated validation with exit code 0 passing all assertions.