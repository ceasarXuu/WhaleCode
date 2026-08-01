from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPT_DIR))

from tui_baseline import (  # noqa: E402
    add_ignored_tests,
    classify_failure,
    compare_runs,
    parse_ignored_tests,
    parse_junit,
)


class TuiBaselineTests(unittest.TestCase):
    def test_parse_junit_normalizes_and_sorts(self) -> None:
        document = parse_junit(
            b"""
            <testsuites>
              <testsuite name="suite">
                <testcase classname="crate::module" name="z_test" />
                <testcase classname="crate::module" name="a_test">
                  <failure>snapshot assertion failed; wrote case.snap.new</failure>
                </testcase>
                <testcase classname="crate::module" name="ignored"><skipped /></testcase>
              </testsuite>
            </testsuites>
            """
        )
        self.assertEqual(
            [
                "crate::module::a_test",
                "crate::module::ignored",
                "crate::module::z_test",
            ],
            [entry["name"] for entry in document["entries"]],
        )
        self.assertEqual(
            {"failed": 1, "ignored": 1, "passed": 1},
            document["summary"]["by_result"],
        )
        self.assertEqual("snapshot_review", document["entries"][0]["classification"])

    def test_environment_failure_precedes_generic_panic(self) -> None:
        result = classify_failure(
            "case", "thread has overflowed its stack\npanicked at"
        )
        self.assertEqual("environment", result)

    def test_plain_assertion_is_functional(self) -> None:
        result = classify_failure("case", "assertion failed: left == right")
        self.assertEqual("functional_assertion", result)

    def test_unknown_failure_stays_unknown(self) -> None:
        self.assertEqual("unknown", classify_failure("case", "process exited with 2"))

    def test_duplicate_normalized_names_are_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate"):
            parse_junit(
                b"<testsuite><testcase name='x'/><testcase name='x'/></testsuite>"
            )

    def test_ignored_tests_from_nextest_list_are_added(self) -> None:
        ignored = parse_ignored_tests(
            b'{"rust-suites":{"codex-tui":{"testcases":'
            b'{"a":{"ignored":false},"b":{"ignored":true},'
            b'"c":{"ignored":true,"filter-match":{"status":"mismatch"}}}}}}'
        )
        document = parse_junit(b"<testsuite><testcase name='a'/></testsuite>")
        augmented = add_ignored_tests(document, ignored)
        self.assertEqual({"ignored": 1, "passed": 1}, augmented["summary"]["by_result"])
        self.assertEqual("codex-tui::b", augmented["entries"][1]["name"])

    def test_compare_runs_reports_result_drift(self) -> None:
        first = parse_junit(b"<testsuite><testcase name='a'/></testsuite>")
        second = parse_junit(
            b"<testsuite><testcase name='a'><failure>boom</failure></testcase></testsuite>"
        )
        self.assertEqual(["a"], compare_runs([first, second]))

    def test_compare_runs_accepts_identical_results(self) -> None:
        document = parse_junit(b"<testsuite><testcase name='a'/></testsuite>")
        self.assertEqual([], compare_runs([document, document]))


if __name__ == "__main__":
    unittest.main()
