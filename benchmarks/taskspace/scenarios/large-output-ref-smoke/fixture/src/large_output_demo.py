def normalize_status(value: str) -> str:
    if value == "":
        raise ValueError("status must not be empty")
    return value
