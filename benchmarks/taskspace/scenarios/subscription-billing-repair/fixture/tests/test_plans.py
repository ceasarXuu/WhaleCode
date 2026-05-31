import pytest

from billing_service.plans import plan_subtotal


def test_plan_subtotal_uses_readme_prices():
    assert plan_subtotal("basic", 2, "monthly") == 20
    assert plan_subtotal("pro", 3, "annual") == 870


def test_enterprise_annual_uses_twelve_months():
    assert plan_subtotal("enterprise", 1, "annual") == 1188


def test_unknown_plan_is_rejected():
    with pytest.raises(ValueError, match="plan"):
        plan_subtotal("unknown", 1, "monthly")
