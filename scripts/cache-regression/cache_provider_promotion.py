#!/usr/bin/env python3
"""Revalidate provider-route artifacts during cache baseline promotion."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from cache_json import exact_json_equal, strict_json_loads
from cache_provider_route import (
    PROFILE_ARTIFACTS,
    PROFILE_INSPECT_ARTIFACTS,
    validate_network_none_inspect,
    validate_resolved_provider_pair,
    validate_route_attestation,
    validate_route_summary,
)
from cache_source_evidence import (
    relative_path,
    require,
    source_bytes,
    source_json,
    source_sha256,
)


def validate_provider_route_evidence(
    repo: Path,
    result: dict[str, Any],
    source: str,
) -> tuple[dict[str, Any], str, list[str]]:
    model = result["observed_scope"]["model"]
    route = validate_route_summary(result.get("provider_route_attestation"), model)
    route_path = relative_path(repo, route["artifact_path"])
    route_prefix = (
        f"benchmarks/cache-regression/evidence/{result['record_id']}/"
        "provider-route-preflight/"
    )
    require(
        route_path.startswith(route_prefix),
        "provider route attestation is not bound to its record",
    )
    require(
        source_sha256(repo, route_path, source) == route["artifact_sha256"],
        "provider route attestation digest mismatch",
    )
    attestation = validate_route_attestation(
        source_json(repo, route_path, source), model
    )
    require(
        attestation["provider_descriptor_sha256"]
        == route["provider_descriptor_sha256"]
        and attestation["whale_binary_sha256"] == route["whale_binary_sha256"]
        and attestation["preflight_started_at"] == route["preflight_started_at"]
        and attestation["preflight_completed_at"]
        == route["preflight_completed_at"]
        and exact_json_equal(
            attestation["provider_routing"], route["provider_routing"]
        ),
        "provider route attestation summary mismatch",
    )

    evidence_paths = []
    route_dir = Path(route_path).parent
    for profile in attestation["profiles"]:
        profile_name = profile["profile"]
        alias_name, builtin_name = PROFILE_ARTIFACTS[profile_name]
        alias_inspect_name, builtin_inspect_name = PROFILE_INSPECT_ARTIFACTS[
            profile_name
        ]
        alias_path = relative_path(repo, (route_dir / alias_name).as_posix())
        builtin_path = relative_path(repo, (route_dir / builtin_name).as_posix())
        alias_inspect_path = relative_path(
            repo, (route_dir / alias_inspect_name).as_posix()
        )
        builtin_inspect_path = relative_path(
            repo, (route_dir / builtin_inspect_name).as_posix()
        )
        require(
            source_sha256(repo, alias_path, source)
            == profile["resolved_provider_artifact_sha256"]
            and source_sha256(repo, builtin_path, source)
            == profile["builtin_provider_artifact_sha256"],
            "resolved provider artifact digest mismatch",
        )
        require(
            source_sha256(repo, alias_inspect_path, source)
            == profile["container_inspect_artifact_sha256"]
            and source_sha256(repo, builtin_inspect_path, source)
            == profile["builtin_container_inspect_artifact_sha256"],
            "provider route container inspect digest mismatch",
        )
        for kind, inspect_path in (
            ("alias", alias_inspect_path),
            ("builtin", builtin_inspect_path),
        ):
            validate_network_none_inspect(
                strict_json_loads(
                    source_bytes(repo, inspect_path, source).decode("utf-8-sig")
                ),
                profile_name,
                kind,
            )
        alias, builtin = validate_resolved_provider_pair(
            source_json(repo, alias_path, source),
            source_json(repo, builtin_path, source),
            model,
        )
        require(
            alias["provider_descriptor_sha256"]
            == profile["provider_descriptor_sha256"]
            and builtin["provider_descriptor_sha256"]
            == profile["builtin_provider_descriptor_sha256"],
            "resolved provider descriptor digest mismatch",
        )
        evidence_paths.extend(
            (alias_path, builtin_path, alias_inspect_path, builtin_inspect_path)
        )

    return route, route_path, evidence_paths
