import pathlib
import sys

repo = pathlib.Path(sys.argv[1]).resolve()
sys.path.insert(0, str(repo / "src"))

from tax_calc import calculate_tax, calculate_total

assert calculate_tax(19.99, "CA") == 1.45
assert calculate_tax(19.99, "NY") == 1.77
assert calculate_tax(10, "TX") == 0.62
assert calculate_total(19.99, "CA") == 21.44
assert calculate_total(10, "TX") == 10.62
try:
    calculate_tax(-1, "CA")
except ValueError as exc:
    assert "subtotal" in str(exc).lower()
else:
    raise AssertionError("negative subtotal should fail")
try:
    calculate_tax(1, "WA")
except ValueError as exc:
    assert "unsupported" in str(exc).lower()
else:
    raise AssertionError("unknown region should fail")
print("hidden oracle passed")
