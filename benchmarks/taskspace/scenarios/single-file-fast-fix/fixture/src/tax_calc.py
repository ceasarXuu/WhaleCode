RATES = {
    "CA": 0.0725,
    "NY": 0.08875,
    "TX": 0.0625,
}


def calculate_tax(subtotal, region):
    if subtotal < 0:
        raise ValueError("subtotal must be non-negative")
    if region not in RATES:
        raise ValueError(f"unsupported region: {region}")
    return round(subtotal * RATES[region], 1)


def calculate_total(subtotal, region):
    return round(subtotal + calculate_tax(subtotal, region), 2)
