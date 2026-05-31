from order_pipeline.pricing import add_shipping, apply_discount


def test_premium_discount_is_percent_and_case_insensitive():
    assert apply_discount(100, "Premium") == 90


def test_vip_discount_is_percent_and_case_insensitive():
    assert apply_discount(200, "VIP") == 170


def test_shipping_uses_discounted_total():
    assert add_shipping(49.99) == 54.99
    assert add_shipping(50) == 50
