import inspect


def count_stack_depth() -> int:
    return len(inspect.stack())


def format_depth() -> str:
    return f"depth: {count_stack_depth()}"


def main() -> None:
    print(format_depth())


if __name__ == "__main__":
    main()
