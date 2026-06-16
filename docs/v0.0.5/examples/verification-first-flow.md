# Example: Verification-First Flow for Format-Sensitive Tasks

## Intended use

Use for tasks like `count-call-stack`, where output format and validator expectations are decisive.

## Flow

```text
1. Route task as verification-first.
2. Read tests / validator / expected output contract.
3. Commit expected-format decision.
4. Generate local checker.
5. Generate output artifact.
6. Run local checker.
7. If checker passes, run public validator.
8. If validator fails, create revised decision from failure text.
```

## State commits

First commit:

```text
fact: validator expects exact first-line stack trace count
criterion: output.txt matches test_outputs.py
validation decision: local checker must pass before public validation
```

Second commit:

```text
result: local checker pass/fail
patch decision: revise output format if needed
next action: public validation or correction
```
