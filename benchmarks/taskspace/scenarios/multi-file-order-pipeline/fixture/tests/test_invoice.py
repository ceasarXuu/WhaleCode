from order_pipeline.invoice import invoice_total


def test_invoice_total_combines_parser_discount_and_shipping():
    lines = [" SKU-1 , 2 , 20.00 ", "sku-2,1,10.00"]
    assert invoice_total(lines, "Premium") == 45.0


def test_invoice_total_vip_large_order_gets_free_shipping():
    lines = ["sku-1,3,25.00"]
    assert invoice_total(lines, "vip") == 63.75
