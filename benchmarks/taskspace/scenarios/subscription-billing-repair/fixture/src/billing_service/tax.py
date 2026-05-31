RATES = {
    "US": 0.07,
    "EU": 0.19,
}


def apply_tax(subtotal, region):
    return subtotal + subtotal * RATES.get(region, 0)
