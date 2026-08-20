from .inventory import needs_restock, parse_inventory_row
from .shipping import shipping_quote


def build_dispatch_summary(row):
    item = parse_inventory_row(row)
    return {
        "sku": item["sku"],
        "restock": needs_restock(item["on_hand"], item["reorder_point"]),
        "shipping": shipping_quote(
            item["weight_kg"],
            item["region"],
            item["service"],
        ),
    }
