import pytest

from tax_calc import calculate_tax, calculate_total


def test_calculate_tax_rounds_to_cents():
    assert calculate_tax(19.99, "CA") == 1.45
    assert calculate_tax(19.99, "NY") == 1.77


def test_calculate_total_uses_tax_amount():
    assert calculate_total(19.99, "CA") == 21.44
    assert calculate_total(10, "TX") == 10.62


def test_invalid_inputs_raise_value_error():
    with pytest.raises(ValueError):
        calculate_tax(-1, "CA")
    with pytest.raises(ValueError):
        calculate_tax(1, "WA")
