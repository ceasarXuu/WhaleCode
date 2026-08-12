import json
import pathlib
from urllib.parse import urlparse


ROOT = pathlib.Path(__file__).resolve().parents[1]


def main() -> int:
    result_path = ROOT / "provider_fact.json"
    if not result_path.is_file():
        raise AssertionError("provider_fact.json is missing")
    payload = json.loads(result_path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise AssertionError("provider_fact.json must contain an object")
    for field in ("title", "url", "summary"):
        value = payload.get(field)
        if not isinstance(value, str) or not value.strip():
            raise AssertionError(f"{field} must be a non-empty string")
    parsed = urlparse(payload["url"])
    if parsed.scheme != "https" or parsed.hostname != "api-docs.deepseek.com":
        raise AssertionError("url must point to the official DeepSeek API docs")
    print("validator_contract=passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
