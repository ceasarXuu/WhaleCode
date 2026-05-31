import pytest

from order_pipeline.parser import parse_order_line


def test_parse_order_line_normalizes_sku_and_numbers():
    item = parse_order_line(" SKU-1 , 2 , 19.50 ")
    assert item == {"sku": "sku-1", "quantity": 2, "unit_price": 19.50}


def test_parse_order_line_rejects_non_positive_quantity():
    with pytest.raises(ValueError, match="quantity"):
        parse_order_line("sku-1,0,19.50")
