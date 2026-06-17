import json
import os
import pathlib
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
EXPECTED_FORMAT = "CALL_STACK_DEPTH=<positive integer>"


def write_probe_artifact() -> None:
    proof_dir = os.environ.get("TASKSPACE_VALIDATION_ARTIFACT_DIR", "")
    if not proof_dir:
        return
    path = pathlib.Path(proof_dir) / "expected-format-decision.json"
    path.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "task": "count-call-stack",
                "expected_format": EXPECTED_FORMAT,
                "local_checker": "scripts/validate.py",
            },
            indent=2,
        ),
        encoding="utf-8",
    )


def run_cli() -> str:
    env = dict(os.environ)
    src = str(ROOT / "src")
    env["PYTHONPATH"] = src + os.pathsep + env.get("PYTHONPATH", "")
    completed = subprocess.run(
        [sys.executable, "-m", "call_stack_counter"],
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise AssertionError(completed.stderr.strip() or "CLI failed")
    return completed.stdout.strip()


def assert_contract(text: str) -> None:
    if not text.startswith("CALL_STACK_DEPTH="):
        raise AssertionError(f"expected {EXPECTED_FORMAT}, got {text!r}")
    value = text.split("=", 1)[1]
    if not value.isdigit() or int(value) <= 0:
        raise AssertionError(f"depth must be a positive integer, got {value!r}")


def main() -> int:
    if "-ProbeOnly" in sys.argv:
        write_probe_artifact()
        print(f"expected_format={EXPECTED_FORMAT}")
        print("local_checker=scripts/validate.py")
        return 0
    write_probe_artifact()
    assert_contract(run_cli())
    print("validator_contract=passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
