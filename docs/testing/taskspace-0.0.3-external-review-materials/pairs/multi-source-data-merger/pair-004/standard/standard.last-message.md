All done. Here's a summary of the merge operation:

## Results

### Merged Dataset (`/app/merged_users.parquet`)
| user_id | name | email | created_date | status |
|---------|------|-------|-------------|--------|
| 101 | John Doe | john@a.com | 2024-01-15 | active |
| 102 | Jane Smith | jane@a.com | 2024-02-20 | inactive |
| 103 | Bob Wilson | bob@b.com | 2024-03-01 | active |
| 104 | Alice Brown | alice@c.com | 2024-04-01 | active |

**4 unique users** across all sources, all conflicts resolved using source priority (a > b > c).

### Conflict Report (`/app/conflicts.json`)
**3 conflicts detected** — all for **user_id 101** who appeared in all three sources with differing values:

| Field | Source A (selected) | Source B | Source C |
|-------|-------------------|---------|---------|
| **name** | John Doe | John Doe | John D. |
| **email** | john@a.com | john@b.com | john@c.com |
| **created_date** | 2024-01-15 | 2024-01-10 | 2024-01-20 |

Users 102, 103, and 104 each appeared in only one source, so no conflicts arose for them.

### Field Mappings Applied
| Unified Field | source_a | source_b | source_c |
|--------------|----------|----------|----------|
| `user_id` | `id` | `user_id` | `userId` |
| `name` | `full_name` | `name` | `userName` |
| `email` | `email` | `email_address` | `email` |
| `created_date` | `registration_date` | `created_at` | `joined` |
| `status` | `status` (string) | `is_active` → "active/inactive" | `active` (bool) → "active/inactive" |

### Validation
- `user_id` is stored as `int64` ✓
- All dates in `YYYY-MM-DD` format ✓
- `total_conflicts` matches the conflict list count (3) ✓
- `status` is nullable string (optional) ✓