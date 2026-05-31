from billing_service.billing import invoice_total


def test_invoice_total_applies_annual_discount_and_us_tax():
    assert invoice_total("acct-1,pro,3,annual", "US") == 930.9


def test_invoice_total_applies_eu_tax():
    assert invoice_total("acct-2,basic,2,monthly", "EU") == 24.0


def test_unknown_tax_region_has_no_tax():
    assert invoice_total("acct-3,basic,1,monthly", "CA") == 10
