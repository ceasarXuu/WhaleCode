import pytest

from billing_service.usage import parse_usage_row


def test_parse_usage_row_normalizes_fields():
    assert parse_usage_row(" acct-7 , Pro , 3 , Annual ") == {
        "account": "acct-7",
        "plan": "pro",
        "seats": 3,
        "billing_period": "annual",
    }


def test_parse_usage_row_rejects_non_positive_seats():
    with pytest.raises(ValueError, match="seats"):
        parse_usage_row("acct,basic,0,monthly")
