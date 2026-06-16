import pytest

from large_output_demo import normalize_status


def test_normalize_status_strips_and_lowercases():
    assert normalize_status(" Ready ") == "ready"
    assert normalize_status("BLOCKED") == "blocked"


def test_normalize_status_rejects_empty_after_strip():
    with pytest.raises(ValueError, match="status"):
        normalize_status("   ")
