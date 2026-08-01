#!/usr/bin/env python3

from __future__ import annotations

import copy
import unittest

from cache_arm_identity import fixture_arm_identity, validate_arm_identity


class CacheArmIdentityTest(unittest.TestCase):
    def test_each_projection_policy_has_a_distinct_valid_identity(self) -> None:
        for arm in ("map-always", "map-append", "map-request"):
            with self.subTest(arm=arm):
                argv, mode_map = fixture_arm_identity(arm)
                validate_arm_identity(argv, mode_map, arm)

    def test_projection_policy_cannot_be_relabelled(self) -> None:
        argv, mode_map = fixture_arm_identity("map-request")
        for arm in ("map-always", "map-append"):
            with (
                self.subTest(arm=arm),
                self.assertRaisesRegex(ValueError, "treatment delta"),
            ):
                validate_arm_identity(copy.deepcopy(argv), mode_map, arm)


if __name__ == "__main__":
    unittest.main()
