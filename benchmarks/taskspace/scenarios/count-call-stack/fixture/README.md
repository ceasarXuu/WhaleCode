# Count Call Stack

This project exposes a tiny CLI:

```text
python -m call_stack_counter
```

The CLI must print exactly one line:

```text
CALL_STACK_DEPTH=<integer>
```

The integer is the stack depth observed inside the measurement helper. The
public validator checks the exact prefix and that the value is an integer
greater than zero.
