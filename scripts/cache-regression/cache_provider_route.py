#!/usr/bin/env python3
"""No-dispatch provider-route preflight and identity validation."""

from __future__ import annotations

import copy
import hashlib
import json
import secrets
import tempfile
import uuid
from pathlib import Path
from typing import Any

from cache_evidence import file_sha256
from cache_json import exact_json_equal, strict_json_loads
from cache_time import parse_timestamp, require_ordered


SCHEMA_VERSION = "whalecode-provider-route-preflight-v1"
EXPECTED_ROUTE = {
    "route_kind": "custom_provider_transport_alias",
    "logical_provider_id": "deepseek",
    "transport_provider_id": "deepseek-boundary",
    "provider_name": "DeepSeek",
    "base_url": "http://provider-proxy:8080",
    "env_key": "DEEPSEEK_API_KEY",
    "wire_api": "responses",
    "intentional_differences": ["provider_id", "base_url"],
}
PROFILE_ARTIFACTS = {
    "standard": ("resolved-provider-standard.json", "builtin-provider-standard.json"),
    "taskspace": (
        "resolved-provider-taskspace.json",
        "builtin-provider-taskspace.json",
    ),
}
PROFILE_INSPECT_ARTIFACTS = {
    "standard": ("container-inspect-standard-alias.json", "container-inspect-standard-builtin.json"),
    "taskspace": (
        "container-inspect-taskspace-alias.json",
        "container-inspect-taskspace-builtin.json",
    ),
}
PROVIDER_FIELDS = {
    "name",
    "base_url_hmac_sha256",
    "env_key",
    "env_key_instructions_hmac_sha256",
    "experimental_bearer_token_hmac_sha256",
    "auth_hmac_sha256",
    "aws_hmac_sha256",
    "wire_api",
    "query_params_hmac_sha256",
    "http_headers_hmac_sha256",
    "env_http_headers_hmac_sha256",
    "request_max_retries",
    "stream_max_retries",
    "stream_idle_timeout_ms",
    "websocket_connect_timeout_ms",
    "requires_openai_auth",
    "supports_websockets",
    "is_deepseek",
}
OPTIONAL_HMAC_FIELDS = {
    "env_key_instructions_hmac_sha256",
    "experimental_bearer_token_hmac_sha256",
    "auth_hmac_sha256",
    "aws_hmac_sha256",
    "query_params_hmac_sha256",
    "http_headers_hmac_sha256",
    "env_http_headers_hmac_sha256",
}


def _is_sha256(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _is_container_digest(value: object) -> bool:
    return (
        isinstance(value, str)
        and value.startswith("sha256:")
        and _is_sha256(value.removeprefix("sha256:"))
    )


def require_deepseek_route_model(model: str) -> None:
    if not isinstance(model, str) or not model.startswith("deepseek-"):
        raise ValueError("provider boundary route requires a DeepSeek model")


def validate_route_identity(value: object) -> dict[str, Any]:
    if not isinstance(value, dict) or not exact_json_equal(value, EXPECTED_ROUTE):
        raise ValueError("provider route identity does not match the reviewed contract")
    return value


def _provider_json_sha256(value: dict[str, Any]) -> str:
    encoded = json.dumps(
        value, ensure_ascii=False, separators=(",", ":"), allow_nan=False
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def validate_resolved_provider(
    value: object, expected_provider_id: str, expected_model: str
) -> dict[str, Any]:
    if not isinstance(value, dict) or not isinstance(value.get("provider"), dict):
        raise ValueError("resolved provider artifact must be an object")
    valid = (
        value.get("schema_version") == "whalecode-resolved-provider-v1"
        and value.get("model_provider_id") == expected_provider_id
        and value.get("model") == expected_model
        and _is_sha256(value.get("provider_descriptor_sha256"))
        and value["provider_descriptor_sha256"]
        == _provider_json_sha256(value["provider"])
        and set(value["provider"]) == PROVIDER_FIELDS
        and _is_sha256(value["provider"].get("base_url_hmac_sha256"))
        and all(
            value["provider"].get(field) is None
            or _is_sha256(value["provider"].get(field))
            for field in OPTIONAL_HMAC_FIELDS
        )
    )
    if not valid:
        raise ValueError("resolved provider artifact is inconsistent")
    return value


def validate_resolved_provider_pair(
    alias: object, builtin: object, expected_model: str
) -> tuple[dict[str, Any], dict[str, Any]]:
    alias = validate_resolved_provider(
        alias, EXPECTED_ROUTE["transport_provider_id"], expected_model
    )
    builtin = validate_resolved_provider(
        builtin, EXPECTED_ROUTE["logical_provider_id"], expected_model
    )
    alias_provider = alias["provider"]
    builtin_provider = builtin["provider"]
    identity_is_valid = (
        alias_provider.get("name") == EXPECTED_ROUTE["provider_name"]
        and alias_provider.get("env_key") == EXPECTED_ROUTE["env_key"]
        and alias_provider.get("wire_api") == EXPECTED_ROUTE["wire_api"]
        and alias_provider.get("is_deepseek") is True
        and builtin_provider.get("is_deepseek") is True
    )
    normalized_alias = copy.deepcopy(alias_provider)
    normalized_alias["base_url_hmac_sha256"] = builtin_provider.get(
        "base_url_hmac_sha256"
    )
    if not identity_is_valid or not exact_json_equal(normalized_alias, builtin_provider):
        raise ValueError(
            "provider alias differs from built-in DeepSeek beyond reviewed differences"
        )
    return alias, builtin


def validate_route_attestation(value: object, expected_model: str) -> dict[str, Any]:
    require_deepseek_route_model(expected_model)
    if not isinstance(value, dict):
        raise ValueError("provider route attestation must be an object")
    profiles = value.get("profiles")
    profiles_are_objects = isinstance(profiles, list) and all(
        isinstance(profile, dict) for profile in profiles
    )
    valid = (
        value.get("schema_version") == SCHEMA_VERSION
        and value.get("status") == "passed"
        and value.get("model") == expected_model
        and value.get("operation") == "config_resolution_only"
        and value.get("network_mode") == "none"
        and _is_sha256(value.get("whale_binary_sha256"))
        and _is_container_digest(value.get("container_image_digest"))
        and _is_sha256(value.get("provider_descriptor_sha256"))
        and profiles_are_objects
        and [profile.get("profile") for profile in profiles]
        == ["standard", "taskspace"]
        and all(
            profile.get("provider_descriptor_sha256")
            == value["provider_descriptor_sha256"]
            and profile.get("resolved_provider_artifact")
            == PROFILE_ARTIFACTS[profile["profile"]][0]
            and profile.get("builtin_provider_artifact")
            == PROFILE_ARTIFACTS[profile["profile"]][1]
            and profile.get("container_inspect_artifact")
            == PROFILE_INSPECT_ARTIFACTS[profile["profile"]][0]
            and profile.get("builtin_container_inspect_artifact")
            == PROFILE_INSPECT_ARTIFACTS[profile["profile"]][1]
            and profile.get("equivalent_to_builtin_deepseek") is True
            and profile.get("multi_agent_v2_enabled")
            is (profile["profile"] == "taskspace")
            and profile.get("projection_policy")
            == ("map-request" if profile["profile"] == "taskspace" else None)
            and all(
                _is_sha256(profile.get(field))
                for field in (
                    "config_overrides_sha256",
                    "argv_sha256",
                    "resolved_provider_artifact_sha256",
                    "builtin_provider_artifact_sha256",
                    "builtin_provider_descriptor_sha256",
                    "builtin_config_overrides_sha256",
                    "builtin_argv_sha256",
                    "container_inspect_artifact_sha256",
                    "builtin_container_inspect_artifact_sha256",
                )
            )
            for profile in profiles
        )
    )
    if not valid:
        raise ValueError("provider route attestation is incomplete")
    started = parse_timestamp(
        value.get("preflight_started_at"), "provider route preflight start"
    )
    completed = parse_timestamp(
        value.get("preflight_completed_at"), "provider route preflight completion"
    )
    require_ordered(started, completed, "provider route preflight start", "completion")
    validate_route_identity(value.get("provider_routing"))
    return value


def validate_route_artifact_directory(
    artifact_dir: Path, attestation: dict[str, Any], expected_model: str
) -> list[Path]:
    paths: list[Path] = []
    for profile in attestation["profiles"]:
        alias_name, builtin_name = PROFILE_ARTIFACTS[profile["profile"]]
        alias_inspect_name, builtin_inspect_name = PROFILE_INSPECT_ARTIFACTS[
            profile["profile"]
        ]
        alias_path = artifact_dir / alias_name
        builtin_path = artifact_dir / builtin_name
        alias_inspect_path = artifact_dir / alias_inspect_name
        builtin_inspect_path = artifact_dir / builtin_inspect_name
        if not all(
            path.is_file()
            for path in (
                alias_path,
                builtin_path,
                alias_inspect_path,
                builtin_inspect_path,
            )
        ):
            raise ValueError("resolved provider artifact is missing")
        if file_sha256(alias_path) != profile["resolved_provider_artifact_sha256"]:
            raise ValueError("resolved provider artifact digest mismatch")
        if file_sha256(builtin_path) != profile["builtin_provider_artifact_sha256"]:
            raise ValueError("built-in provider artifact digest mismatch")
        if (
            file_sha256(alias_inspect_path)
            != profile["container_inspect_artifact_sha256"]
            or file_sha256(builtin_inspect_path)
            != profile["builtin_container_inspect_artifact_sha256"]
        ):
            raise ValueError("provider route container inspect digest mismatch")
        for kind, inspect_path in (
            ("alias", alias_inspect_path),
            ("builtin", builtin_inspect_path),
        ):
            inspect = strict_json_loads(inspect_path.read_text(encoding="utf-8-sig"))
            validate_network_none_inspect(inspect, profile["profile"], kind)
        alias = strict_json_loads(alias_path.read_text(encoding="utf-8-sig"))
        builtin = strict_json_loads(builtin_path.read_text(encoding="utf-8-sig"))
        alias, builtin = validate_resolved_provider_pair(alias, builtin, expected_model)
        if (
            alias["provider_descriptor_sha256"]
            != profile["provider_descriptor_sha256"]
            or builtin["provider_descriptor_sha256"]
            != profile["builtin_provider_descriptor_sha256"]
        ):
            raise ValueError("resolved provider descriptor digest mismatch")
        paths.extend(
            (alias_path, builtin_path, alias_inspect_path, builtin_inspect_path)
        )
    return paths


def validate_route_summary(value: object, expected_model: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError("provider route summary must be an object")
    valid = (
        value.get("schema_version") == SCHEMA_VERSION
        and value.get("status") == "passed"
        and value.get("model") == expected_model
        and all(
            _is_sha256(value.get(field))
            for field in (
                "provider_descriptor_sha256",
                "whale_binary_sha256",
                "artifact_sha256",
            )
        )
        and isinstance(value.get("artifact_path"), str)
        and bool(value["artifact_path"])
    )
    if not valid:
        raise ValueError("provider route summary is incomplete")
    started = parse_timestamp(
        value.get("preflight_started_at"), "provider route preflight start"
    )
    completed = parse_timestamp(
        value.get("preflight_completed_at"), "provider route preflight completion"
    )
    require_ordered(started, completed, "provider route preflight start", "completion")
    validate_route_identity(value.get("provider_routing"))
    return value


def route_profile_binding(route: dict[str, Any], arm: str) -> dict[str, Any]:
    return {
        "profile": "standard" if arm == "standard" else "taskspace",
        "attestation_path": route["artifact_path"],
        "attestation_sha256": route["artifact_sha256"],
        "provider_descriptor_sha256": route["provider_descriptor_sha256"],
    }


def validate_route_profile_binding(
    value: object, route: dict[str, Any], arm: str
) -> dict[str, Any]:
    expected = route_profile_binding(route, arm)
    if not isinstance(value, dict) or not exact_json_equal(value, expected):
        raise ValueError("cache arm is not bound to its resolved provider profile")
    return value


def validate_network_none_inspect(
    value: object, expected_profile: str, expected_kind: str
) -> dict[str, Any]:
    expected = {
        "schema_version": "whalecode-provider-route-container-inspect-v1",
        "profile": expected_profile,
        "provider_kind": expected_kind,
        "network_mode": "none",
        "workspace_read_only": True,
        "descriptor_key_secret_mounted": True,
        "descriptor_key_read_only": True,
        "descriptor_key_source_mount_unique": True,
        "descriptor_key_mount_identity_confirmed": True,
        "descriptor_key_env_file": "/run/secrets/deepseek_api_key",
    }
    if not isinstance(value, dict) or not exact_json_equal(value, expected):
        raise ValueError("provider route container did not use network=none")
    return value


def run_provider_route_preflight(
    repo: Path,
    whale_bin: Path,
    output_path: Path,
    model: str,
    timeout_seconds: int = 180,
) -> dict[str, Any]:
    from cache_process_control import (
        BenchmarkTimeoutError,
        cleanup_labeled_containers,
        run_captured_command,
    )

    require_deepseek_route_model(model)
    repo = repo.resolve()
    output_path = (
        output_path.resolve()
        if output_path.is_absolute()
        else (repo / output_path).resolve()
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="whale-provider-route-") as transient:
        transient_dir = Path(transient)
        preflight_run_id = f"provider-route-preflight-{uuid.uuid4().hex[:12]}"
        descriptor_key_path = transient_dir / "descriptor-hmac.key"
        descriptor_key_path.write_bytes(secrets.token_bytes(32))
        descriptor_key_path.chmod(0o600)
        command = [
            "pwsh",
            "-NoProfile",
            "-File",
            str(repo / "scripts/taskspace-benchmark/invoke-provider-route-preflight.ps1"),
            "-WhaleBin",
            str(whale_bin),
            "-Model",
            model,
            "-OutputPath",
            str(output_path),
            "-RunId",
            preflight_run_id,
            "-DescriptorKeyPath",
            str(descriptor_key_path),
            "-TransientDir",
            str(transient_dir),
            "-TimeoutSeconds",
            str(timeout_seconds),
        ]
        try:
            completed = run_captured_command(
                command,
                cwd=repo,
                timeout_seconds=timeout_seconds * 2,
            )
        except BenchmarkTimeoutError as error:
            if not error.process_tree_termination.get(
                "descendants_guaranteed_terminated", False
            ):
                raise ValueError(
                    "provider route preflight process-tree cleanup could not be verified"
                ) from error
            raise ValueError(
                "provider route preflight timed out before authorization claim"
            ) from error
        finally:
            cleanup = cleanup_labeled_containers(
                preflight_run_id, grace_seconds=10, run_root=transient_dir
            )
            if cleanup["status"] == "failed":
                raise ValueError(
                    "provider route preflight container cleanup could not be verified: "
                    + cleanup["error"]
                )
        if completed.returncode != 0:
            detail = completed.stderr.strip().splitlines()[-1:] or ["unknown failure"]
            raise ValueError(
                f"provider route preflight failed before authorization claim: {detail[0]}"
            )
    if not output_path.is_file():
        raise ValueError("provider route preflight did not write its attestation")
    attestation = strict_json_loads(output_path.read_text(encoding="utf-8-sig"))
    validate_route_attestation(attestation, model)
    validate_route_artifact_directory(output_path.parent, attestation, model)
    return {
        "schema_version": SCHEMA_VERSION,
        "status": "passed",
        "model": model,
        "provider_routing": attestation["provider_routing"],
        "provider_descriptor_sha256": attestation["provider_descriptor_sha256"],
        "whale_binary_sha256": attestation["whale_binary_sha256"],
        "preflight_started_at": attestation["preflight_started_at"],
        "preflight_completed_at": attestation["preflight_completed_at"],
        "artifact_path": output_path.relative_to(repo).as_posix(),
        "artifact_sha256": file_sha256(output_path),
    }
