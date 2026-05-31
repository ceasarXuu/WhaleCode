def parse_usage_row(row):
    account, plan, seats, billing_period = row.split(",")
    return {
        "account": account,
        "plan": plan,
        "seats": int(seats),
        "billing_period": billing_period,
    }
