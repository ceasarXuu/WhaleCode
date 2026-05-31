from .plans import plan_subtotal
from .tax import apply_tax
from .usage import parse_usage_row


def invoice_total(row, tax_region):
    usage = parse_usage_row(row)
    subtotal = plan_subtotal(
        usage["plan"],
        usage["seats"],
        usage["billing_period"],
    )
    return round(apply_tax(subtotal, tax_region), 2)
