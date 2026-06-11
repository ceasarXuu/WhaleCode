Merge complete. Here's the summary:

## Results

**Merged dataset** → `W:\app\merged_users.parquet` (4 rows)

| user_id | name | email | created_date | status |
|---------|------|-------|-------------|--------|
| 101 | John Doe | john@a.com | 2024-01-15 | active |
| 102 | Jane Smith | jane@a.com | 2024-02-20 | inactive |
| 103 | Bob Wilson | bob@b.com | 2024-03-01 | null |
| 104 | Alice Brown | alice@c.com | 2024-04-01 | null |

**Conflict report** → `W:\app\conflicts.json` (3 conflicts)

All 3 conflicts involved user **101** (appeared in all 3 sources):

| Field | source_a | source_b | source_c | Selected |
|-------|----------|----------|----------|----------|
| name | John Doe | John Doe | John D. | John Doe |
| email | john@a.com | john@b.com | john@c.com | john@a.com |
| created_date | 2024-01-15 | 2024-01-10 | 2024-01-20 | 2024-01-15 |

All requirements met:
- ✅ All 4 unique users included (101, 102, 103, 104)
- ✅ Field mappings applied: `id`/`userId` → `user_id`, `full_name`/`userName` → `name`, `email_address` → `email`, `registration_date`/`joined` → `created_date`
- ✅ Priority resolution: source_a > source_b > source_c
- ✅ `user_id` as integer, dates as `YYYY-MM-DD`, `status` optional/nullable
- ✅ Parquet and JSON outputs in correct format with matching conflict count