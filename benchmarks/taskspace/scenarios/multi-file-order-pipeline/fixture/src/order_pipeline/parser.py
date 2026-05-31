def parse_order_line(line):
    sku, quantity, unit_price = line.split(",")
    return {
        "sku": sku,
        "quantity": int(quantity),
        "unit_price": float(unit_price),
    }
