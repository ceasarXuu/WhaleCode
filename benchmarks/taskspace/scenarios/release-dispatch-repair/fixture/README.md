# Release Dispatch

The package converts one inventory row into a dispatch decision and shipping quote.

Inventory rules:
- Rows use `sku,on_hand,reorder_point,weight_kg,region,service`.
- Text fields are trimmed; SKU, region, and service are lowercased.
- On-hand and reorder-point values are non-negative integers.
- Weight must be positive.
- Restock is required when on-hand stock is less than or equal to the reorder point.

Shipping rules:
- Region base charges are domestic 3, EU 8, and international 12.
- Weight is billed by each started kilogram at 2 per kilogram.
- Express service adds 5; standard service has no surcharge.
- Unknown regions and services are rejected.
- Quotes are rounded to cents.

The dispatch summary must preserve the normalized SKU, restock decision, and shipping quote.
