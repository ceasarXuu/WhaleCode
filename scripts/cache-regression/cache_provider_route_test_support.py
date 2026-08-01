#!/usr/bin/env python3
"""Provider-route fixtures shared by cache regression tests."""

from __future__ import annotations

import copy
import hashlib
import hmac
import json
from pathlib import Path

from cache_evidence import file_sha256
from cache_provider_route import EXPECTED_ROUTE, SCHEMA_VERSION, route_profile_binding


def route_summary(model: str = "deepseek-v4-flash") -> dict:
    return {
        "schema_version": SCHEMA_VERSION,
        "status": "passed",
        "model": model,
        "provider_routing": copy.deepcopy(EXPECTED_ROUTE),
        "provider_descriptor_sha256": "d" * 64,
        "whale_binary_sha256": "e" * 64,
        "preflight_started_at": "2026-07-31T00:00:00+00:00",
        "preflight_completed_at": "2026-07-31T00:00:01+00:00",
        "artifact_path": "target/provider-route-preflight.json",
        "artifact_sha256": "f" * 64,
    }


def _provider(base_url: str, key: bytes) -> dict:
    def digest(value: str) -> str:
        return hmac.new(key, value.encode("utf-8"), hashlib.sha256).hexdigest()

    return {
        "name": "DeepSeek",
        "base_url_hmac_sha256": digest(base_url),
        "env_key": "DEEPSEEK_API_KEY",
        "env_key_instructions_hmac_sha256": digest(
            "Set DEEPSEEK_API_KEY to a DeepSeek API key before starting Whale."
        ),
        "experimental_bearer_token_hmac_sha256": None,
        "auth_hmac_sha256": None,
        "aws_hmac_sha256": None,
        "wire_api": "responses",
        "query_params_hmac_sha256": None,
        "http_headers_hmac_sha256": None,
        "env_http_headers_hmac_sha256": None,
        "request_max_retries": None,
        "stream_max_retries": None,
        "stream_idle_timeout_ms": None,
        "websocket_connect_timeout_ms": None,
        "requires_openai_auth": False,
        "supports_websockets": False,
        "is_deepseek": True,
    }


def _resolved(provider_id: str, model: str, base_url: str, key: bytes) -> dict:
    provider = _provider(base_url, key)
    encoded = json.dumps(
        provider, ensure_ascii=False, separators=(",", ":"), allow_nan=False
    ).encode("utf-8")
    return {
        "schema_version": "whalecode-resolved-provider-v1",
        "model_provider_id": provider_id,
        "model": model,
        "provider": provider,
        "provider_descriptor_sha256": hashlib.sha256(encoded).hexdigest(),
    }


def materialize_route_summary(repo: Path, path: Path, model: str) -> dict:
    path.parent.mkdir(parents=True, exist_ok=True)
    profiles = []
    descriptor_key = b"provider-route-fixture-key"
    for profile in ("standard", "taskspace"):
        alias_name = f"resolved-provider-{profile}.json"
        builtin_name = f"builtin-provider-{profile}.json"
        alias_path = path.parent / alias_name
        builtin_path = path.parent / builtin_name
        alias = _resolved(
            "deepseek-boundary", model, EXPECTED_ROUTE["base_url"], descriptor_key
        )
        builtin = _resolved(
            "deepseek", model, "https://api.deepseek.com", descriptor_key
        )
        alias_path.write_text(json.dumps(alias, indent=2) + "\n", encoding="utf-8")
        builtin_path.write_text(json.dumps(builtin, indent=2) + "\n", encoding="utf-8")
        alias_inspect_name = f"container-inspect-{profile}-alias.json"
        builtin_inspect_name = f"container-inspect-{profile}-builtin.json"
        alias_inspect_path = path.parent / alias_inspect_name
        builtin_inspect_path = path.parent / builtin_inspect_name
        def inspect(kind: str) -> dict:
            return {
                "schema_version": "whalecode-provider-route-container-inspect-v1",
                "profile": profile,
                "provider_kind": kind,
                "network_mode": "none",
                "workspace_read_only": True,
                "descriptor_key_secret_mounted": True,
                "descriptor_key_read_only": True,
                "descriptor_key_source_mount_unique": True,
                "descriptor_key_mount_identity_confirmed": True,
                "descriptor_key_env_file": "/run/secrets/deepseek_api_key",
            }

        alias_inspect_path.write_text(
            json.dumps(inspect("alias")) + "\n", encoding="utf-8"
        )
        builtin_inspect_path.write_text(
            json.dumps(inspect("builtin")) + "\n", encoding="utf-8"
        )
        profiles.append(
            {
                "profile": profile,
                "projection_policy": "map-request" if profile == "taskspace" else None,
                "multi_agent_v2_enabled": profile == "taskspace",
                "provider_descriptor_sha256": alias["provider_descriptor_sha256"],
                "config_overrides_sha256": "a" * 64,
                "argv_sha256": "b" * 64,
                "resolved_provider_artifact": alias_name,
                "resolved_provider_artifact_sha256": file_sha256(alias_path),
                "builtin_provider_artifact": builtin_name,
                "builtin_provider_artifact_sha256": file_sha256(builtin_path),
                "builtin_provider_descriptor_sha256": builtin[
                    "provider_descriptor_sha256"
                ],
                "builtin_config_overrides_sha256": "c" * 64,
                "builtin_argv_sha256": "d" * 64,
                "container_inspect_artifact": alias_inspect_name,
                "container_inspect_artifact_sha256": file_sha256(alias_inspect_path),
                "builtin_container_inspect_artifact": builtin_inspect_name,
                "builtin_container_inspect_artifact_sha256": file_sha256(
                    builtin_inspect_path
                ),
                "equivalent_to_builtin_deepseek": True,
            }
        )
    value = {
        "schema_version": SCHEMA_VERSION,
        "status": "passed",
        "model": model,
        "preflight_started_at": "2026-07-31T00:00:00+00:00",
        "preflight_completed_at": "2026-07-31T00:00:01+00:00",
        "operation": "config_resolution_only",
        "network_mode": "none",
        "whale_binary_sha256": "e" * 64,
        "container_image_digest": "sha256:" + "f" * 64,
        "provider_descriptor_sha256": profiles[0]["provider_descriptor_sha256"],
        "provider_routing": copy.deepcopy(EXPECTED_ROUTE),
        "profiles": profiles,
    }
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    summary = route_summary(model)
    summary["provider_descriptor_sha256"] = value["provider_descriptor_sha256"]
    summary["artifact_path"] = path.relative_to(repo).as_posix()
    summary["artifact_sha256"] = file_sha256(path)
    return summary


def bind_route(entry: dict, result: dict, route: dict | None = None) -> None:
    route = route or route_summary()
    identity = route["provider_routing"]
    execution = entry.setdefault("execution", {})
    execution.update(
        {
            "provider": identity["logical_provider_id"],
            "transport_provider": identity["transport_provider_id"],
            "provider_descriptor_sha256": route["provider_descriptor_sha256"],
            "model": route["model"],
        }
    )
    evidence = entry.setdefault("evidence", {})
    evidence.update(
        {
            "provider_route_attestation_path": route["artifact_path"],
            "provider_route_attestation_sha256": route["artifact_sha256"],
        }
    )
    result["provider_route_attestation"] = route
    result.setdefault("observed_scope", {})["model"] = route["model"]
    for observation in result.get("observations", []):
        observation["provider_routing"] = copy.deepcopy(identity)
        observation["provider_route_profile"] = route_profile_binding(
            route, observation["arm"]
        )
