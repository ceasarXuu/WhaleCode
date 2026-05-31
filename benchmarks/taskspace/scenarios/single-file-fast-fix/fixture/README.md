# Tax Calc

This tiny package computes US sales tax for a few supported regions.

Product rules:
- `calculate_tax(subtotal, region)` returns the tax amount rounded to cents.
- `calculate_total(subtotal, region)` returns subtotal plus tax rounded to cents.
- Supported regions are `CA`, `NY`, and `TX`.
- Unknown regions should raise `ValueError`.
- Negative subtotals should raise `ValueError`.
