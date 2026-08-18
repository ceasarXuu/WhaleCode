def parse_inventory_row(row):
    sku, on_hand, reorder_point, weight_kg, region, service = row.split(",")
    return {
        "sku": sku,
        "on_hand": int(on_hand),
        "reorder_point": int(reorder_point),
        "weight_kg": float(weight_kg),
        "region": region,
        "service": service,
    }


def needs_restock(on_hand, reorder_point):
    return on_hand < reorder_point
