import pytest

from release_dispatch.inventory import needs_restock, parse_inventory_row


def test_inventory_row_is_normalized():
    assert parse_inventory_row(" SKU-7 , 4 , 4 , 1.2 , EU , Express ") == {
        "sku": "sku-7",
        "on_hand": 4,
        "reorder_point": 4,
        "weight_kg": 1.2,
        "region": "eu",
        "service": "express",
    }


def test_restock_includes_threshold():
    assert needs_restock(3, 4)
    assert needs_restock(4, 4)
    assert not needs_restock(5, 4)


@pytest.mark.parametrize(
    "row, field",
    [
        ("sku,-1,2,1.0,eu,standard", "on_hand"),
        ("sku,1,-2,1.0,eu,standard", "reorder_point"),
        ("sku,1,2,0,eu,standard", "weight"),
    ],
)
def test_inventory_row_rejects_invalid_numbers(row, field):
    with pytest.raises(ValueError, match=field):
        parse_inventory_row(row)
