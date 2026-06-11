All done. Here's a summary of the merge operation:

## Input Sources

| Source | Format | Users | Fields (original → unified) |
|--------|--------|-------|-----------------------------|
| **source_a** (highest) | `users.json` | 101, 102 | `id`→`user_id`, `full_name`→`name`, `registration_date`→`created_date`, `status`→`status` |
| **source_b** (secondary) | `users.csv` | 101, 103 | `user_id`→`user_id`, `name`→`name`, `email_address`→`email`, `created_at`→`created_date`, `is_active`→`status` |
| **source_c** (tertiary) | `users.parquet` | 101, 104 | `userId`→`user_id`, `userName`→`name`, `joined`→`created_date`, `active`→`status` |

## Merged Output (`W:\app\merged_users.parquet`)

| user_id | name | email | created_date | status |
|---------|------|-------|-------------|--------|
| 101 | John Doe | john@a.com | 2024-01-15 | active |
| 102 | Jane Smith | jane@a.com | 2024-02-20 | inactive |
| 103 | Bob Wilson | bob@b.com | 2024-03-01 | active |
| 104 | Alice Brown | alice@c.com | 2024-04-01 | active |

All 4 unique users across sources included. User 101's conflicts (email, name variant, date) resolved using **source_a** values per priority.

## Conflict Report (`W:\app\conflicts.json`)

**3 conflicts** found, all for user 101 (present in all 3 sources with differing values):

- **name**: `"John Doe"` (a/b) vs `"John D."` (c) → selected `"John Doe"` (source_a)
- **email**: `"john@a.com"` (a) vs `"john@b.com"` (b) vs `"john@c.com"` (c) → selected `"john@a.com"`
- **created_date**: `"2024-01-15"` (a) vs `"2024-01-10"` (b) vs `"2024-01-20"` (c) → selected `"2024-01-15"`