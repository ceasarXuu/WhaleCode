All done. Here's a summary of what was accomplished:

## Results

### Input Sources
| Source | Format | Records | Fields Mapped |
|--------|--------|---------|---------------|
| Source A (priority 1) | `users.json` | 2 users (101, 102) | `id`→`user_id`, `full_name`→`name`, `registration_date`→`created_date` |
| Source B (priority 2) | `users.csv` | 2 users (101, 103) | `email_address`→`email`, `created_at`→`created_date`, `is_active`→`status` |
| Source C (priority 3) | `users.parquet` | 2 users (101, 104) | `userId`→`user_id`, `userName`→`name`, `joined`→`created_date`, `active`→`status` |

### Merged Dataset (`merged_users.parquet`)
- **4 unique users**, all sources combined
- User 101 (John Doe) from source A wins conflicts on name, email, and created_date
- Users 102, 103, 104 each appear in only one source

### Conflict Report (`conflicts.json`)
- **3 conflicts** detected — all on user 101 (present in all 3 sources):
  - **name**: "John Doe" (A/B) vs "John D." (C) → selected "John Doe"
  - **email**: "john@a.com" (A) vs "john@b.com" (B) vs "john@c.com" (C) → selected "john@a.com"
  - **created_date**: "2024-01-15" (A) vs "2024-01-10" (B) vs "2024-01-20" (C) → selected "2024-01-15"

Key implementation details:
- Source B's `is_active` was parsed as boolean by pandas → mapped `{True: "active", False: "inactive"}`
- Source C's `active` was boolean → mapped `{True: "active", False: "inactive"}`
- All source A, B, C agree on `status` = "active" for user 101 → no conflict there
- Dates are preserved as `YYYY-MM-DD` strings
- `user_id` is stored as integer type in parquet