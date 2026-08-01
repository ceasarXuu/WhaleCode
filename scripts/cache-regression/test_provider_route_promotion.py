#!/usr/bin/env python3

from __future__ import annotations

import copy
import json
import unittest

from cache_surface import write_json
from promote_cache_baseline_test_support import PromoteCacheBaselineFixture


class ProviderRoutePromotionTest(PromoteCacheBaselineFixture, unittest.TestCase):
    def test_rejects_tampered_provider_route_attestation(self) -> None:
        route = self.result["provider_route_attestation"]
        route_path = self.repo / route["artifact_path"]
        value = json.loads(route_path.read_text(encoding="utf-8"))
        value["operation"] = "provider_dispatch"
        write_json(route_path, value)

        with self.assertRaisesRegex(ValueError, "route attestation digest mismatch"):
            self.validate()

    def test_rejects_missing_resolved_provider_original(self) -> None:
        route = self.result["provider_route_attestation"]
        route_dir = (self.repo / route["artifact_path"]).parent
        (route_dir / "resolved-provider-standard.json").unlink()

        with self.assertRaisesRegex(ValueError, "cache evidence is missing"):
            self.validate()

    def test_rejects_provider_route_attestation_from_another_record(self) -> None:
        result = copy.deepcopy(self.result)
        result["provider_route_attestation"]["artifact_path"] = (
            "benchmarks/cache-regression/evidence/WAR-OTHER/"
            "provider-route-preflight/provider-route-preflight.json"
        )

        with self.assertRaisesRegex(ValueError, "not bound to its record"):
            self.validate(result=result)
