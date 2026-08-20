import pytest

from release_dispatch.shipping import shipping_quote


def test_shipping_uses_started_kilograms_and_service_surcharge():
    assert shipping_quote(1.2, "eu", "express") == 17
    assert shipping_quote(2.0, "domestic", "standard") == 7


@pytest.mark.parametrize(
    "region, service, field",
    [
        ("moon", "standard", "region"),
        ("eu", "overnight", "service"),
    ],
)
def test_shipping_rejects_unknown_values(region, service, field):
    with pytest.raises(ValueError, match=field):
        shipping_quote(1, region, service)
