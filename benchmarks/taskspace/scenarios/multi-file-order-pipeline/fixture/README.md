# Order Pipeline

This package parses simple order lines and calculates invoice totals.

Product rules:
- SKU values must be trimmed and lowercased.
- Quantity must be a positive integer.
- Unit price must be non-negative.
- Premium customers receive 10 percent off, case-insensitive.
- VIP customers receive 15 percent off, case-insensitive.
- Shipping adds 5 only when the discounted total is below 50.
- Invoice totals are rounded to cents after discount and shipping.
