# Subscription Billing

The service converts usage rows into invoice totals.

Product rules:
- Usage rows are `account,plan,seats,billing_period`.
- Account, plan, and billing period are trimmed; plan and billing period are lowercased.
- Seats must be a positive integer.
- Monthly plan prices per seat: basic 10, pro 29, enterprise 99.
- Annual billing charges 10 months for 12 months of service.
- Supported billing periods are monthly and annual.
- Supported tax regions are US at 7 percent and EU at 20 percent.
- Unknown tax regions have no tax.
- Invoice totals are rounded to cents after subtotal and tax are combined.
