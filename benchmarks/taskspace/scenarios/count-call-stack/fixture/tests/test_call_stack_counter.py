from call_stack_counter import count_stack_depth, format_depth


def test_count_stack_depth_is_positive():
    assert count_stack_depth() > 0


def test_format_depth_contract():
    formatted = format_depth()
    assert formatted.startswith("CALL_STACK_DEPTH=")
    value = formatted.split("=", 1)[1]
    assert value.isdigit()
    assert int(value) > 0
