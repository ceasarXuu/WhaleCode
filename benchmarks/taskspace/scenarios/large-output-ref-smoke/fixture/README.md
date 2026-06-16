# Large Output Ref Smoke

`normalize_status` should normalize a status string for downstream routing.

Expected behavior:

- Trim surrounding whitespace.
- Convert the value to lowercase.
- Reject an empty normalized value with `ValueError`.
