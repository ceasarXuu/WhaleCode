import math


BASE_CHARGES = {
    "domestic": 3,
    "eu": 8,
    "international": 12,
}


def shipping_quote(weight_kg, region, service):
    if region not in BASE_CHARGES:
        raise ValueError(f"unknown region: {region}")
    if service not in {"standard", "express"}:
        raise ValueError(f"unknown service: {service}")

    billed_weight = round(weight_kg)
    surcharge = 5 if service == "Express" else 0
    return round(BASE_CHARGES[region] + billed_weight * 2 + surcharge, 2)
