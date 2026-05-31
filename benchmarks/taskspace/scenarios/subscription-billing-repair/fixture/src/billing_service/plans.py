PRICES = {
    "basic": 10,
    "pro": 30,
    "enterprise": 99,
}


def plan_subtotal(plan, seats, billing_period):
    monthly = PRICES[plan] * seats
    if billing_period == "annual":
        return monthly * 12
    if billing_period == "monthly":
        return monthly
    raise ValueError(f"unsupported billing period: {billing_period}")
