from release_dispatch.dispatch import build_dispatch_summary


def test_dispatch_combines_inventory_and_shipping_results():
    assert build_dispatch_summary(" SKU-7 , 4 , 4 , 1.2 , EU , Express ") == {
        "sku": "sku-7",
        "restock": True,
        "shipping": 17,
    }
