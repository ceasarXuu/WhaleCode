#!/usr/bin/env python3

from __future__ import annotations

import copy
import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from cache_evidence import file_sha256
from cache_process_control import BenchmarkTimeoutError
from cache_provider_route import (
    EXPECTED_ROUTE,
    SCHEMA_VERSION,
    require_deepseek_route_model,
    run_provider_route_preflight,
    validate_resolved_provider_pair,
    validate_route_artifact_directory,
    validate_route_attestation,
)
from cache_provider_route_test_support import materialize_route_summary


def attestation(model: str = "deepseek-v4-flash") -> dict:
    profiles = []
    for profile in ("standard", "taskspace"):
        profiles.append(
            {
                "profile": profile,
                "projection_policy": "map-request" if profile == "taskspace" else None,
                "multi_agent_v2_enabled": profile == "taskspace",
                "provider_descriptor_sha256": "d" * 64,
                "config_overrides_sha256": "a" * 64,
                "argv_sha256": "b" * 64,
                "resolved_provider_artifact": f"resolved-provider-{profile}.json",
                "resolved_provider_artifact_sha256": "c" * 64,
                "builtin_provider_artifact": f"builtin-provider-{profile}.json",
                "builtin_provider_artifact_sha256": "d" * 64,
                "builtin_provider_descriptor_sha256": "e" * 64,
                "builtin_config_overrides_sha256": "f" * 64,
                "builtin_argv_sha256": "1" * 64,
                "container_inspect_artifact": f"container-inspect-{profile}-alias.json",
                "container_inspect_artifact_sha256": "2" * 64,
                "builtin_container_inspect_artifact": f"container-inspect-{profile}-builtin.json",
                "builtin_container_inspect_artifact_sha256": "3" * 64,
                "equivalent_to_builtin_deepseek": True,
            }
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "status": "passed",
        "model": model,
        "preflight_started_at": "2026-07-31T00:00:00+00:00",
        "preflight_completed_at": "2026-07-31T00:00:01+00:00",
        "operation": "config_resolution_only",
        "network_mode": "none",
        "whale_binary_sha256": "e" * 64,
        "container_image_digest": "sha256:" + "f" * 64,
        "provider_descriptor_sha256": "d" * 64,
        "provider_routing": copy.deepcopy(EXPECTED_ROUTE),
        "profiles": profiles,
    }


class CacheProviderRouteTest(unittest.TestCase):
    def test_non_deepseek_model_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "requires a DeepSeek model"):
            require_deepseek_route_model("gpt-5")

    def test_attestation_rejects_operation_or_identity_drift(self) -> None:
        value = attestation()
        value["operation"] = "provider_dispatch"
        with self.assertRaisesRegex(ValueError, "incomplete"):
            validate_route_attestation(value, "deepseek-v4-flash")

        value = attestation()
        value["provider_routing"]["transport_provider_id"] = "deepseek"
        with self.assertRaisesRegex(ValueError, "reviewed contract"):
            validate_route_attestation(value, "deepseek-v4-flash")

    def test_attestation_rejects_non_hex_digest(self) -> None:
        value = attestation()
        value["provider_descriptor_sha256"] = "z" * 64
        with self.assertRaisesRegex(ValueError, "incomplete"):
            validate_route_attestation(value, "deepseek-v4-flash")

        value = attestation()
        value["preflight_completed_at"] = "2026-07-30T23:59:59+00:00"
        with self.assertRaisesRegex(ValueError, "precedes"):
            validate_route_attestation(value, "deepseek-v4-flash")

    def test_internally_consistent_alias_behavior_drift_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            output = repo / "evidence/provider-route-preflight.json"
            materialize_route_summary(repo, output, "deepseek-v4-flash")
            alias = json.loads(
                (output.parent / "resolved-provider-standard.json").read_text(
                    encoding="utf-8"
                )
            )
            builtin = json.loads(
                (output.parent / "builtin-provider-standard.json").read_text(
                    encoding="utf-8"
                )
            )
            alias["provider"]["query_params_hmac_sha256"] = "a" * 64
            encoded = json.dumps(
                alias["provider"],
                ensure_ascii=False,
                separators=(",", ":"),
                allow_nan=False,
            ).encode("utf-8")
            alias["provider_descriptor_sha256"] = hashlib.sha256(encoded).hexdigest()
            with self.assertRaisesRegex(ValueError, "beyond reviewed differences"):
                validate_resolved_provider_pair(
                    alias, builtin, "deepseek-v4-flash"
                )

        value = attestation()
        value["profiles"][1]["multi_agent_v2_enabled"] = False
        with self.assertRaisesRegex(ValueError, "incomplete"):
            validate_route_attestation(value, "deepseek-v4-flash")

    def test_artifact_directory_rejects_missing_or_tampered_original(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            output = repo / "evidence/provider-route-preflight.json"
            materialize_route_summary(repo, output, "deepseek-v4-flash")
            value = json.loads(output.read_text(encoding="utf-8"))
            missing = output.parent / "resolved-provider-standard.json"
            missing.unlink()
            with self.assertRaisesRegex(ValueError, "missing"):
                validate_route_artifact_directory(
                    output.parent, value, "deepseek-v4-flash"
                )

            materialize_route_summary(repo, output, "deepseek-v4-flash")
            tampered = output.parent / "resolved-provider-taskspace.json"
            tampered.write_text("{}\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "digest mismatch"):
                validate_route_artifact_directory(
                    output.parent, value, "deepseek-v4-flash"
                )

            materialize_route_summary(repo, output, "deepseek-v4-flash")
            value = json.loads(output.read_text(encoding="utf-8"))
            inspect = output.parent / "container-inspect-standard-alias.json"
            inspect_value = json.loads(inspect.read_text(encoding="utf-8"))
            inspect_value["network_mode"] = "bridge"
            inspect.write_text(json.dumps(inspect_value) + "\n", encoding="utf-8")
            value["profiles"][0]["container_inspect_artifact_sha256"] = file_sha256(
                inspect
            )
            with self.assertRaisesRegex(ValueError, "network=none"):
                validate_route_artifact_directory(
                    output.parent, value, "deepseek-v4-flash"
                )

            materialize_route_summary(repo, output, "deepseek-v4-flash")
            value = json.loads(output.read_text(encoding="utf-8"))
            inspect = output.parent / "container-inspect-standard-alias.json"
            inspect_value = json.loads(inspect.read_text(encoding="utf-8"))
            inspect_value["descriptor_key_mount_identity_confirmed"] = False
            inspect.write_text(json.dumps(inspect_value) + "\n", encoding="utf-8")
            value["profiles"][0]["container_inspect_artifact_sha256"] = file_sha256(
                inspect
            )
            with self.assertRaisesRegex(ValueError, "network=none"):
                validate_route_artifact_directory(
                    output.parent, value, "deepseek-v4-flash"
                )

            materialize_route_summary(repo, output, "deepseek-v4-flash")
            value = json.loads(output.read_text(encoding="utf-8"))
            inspect = output.parent / "container-inspect-standard-alias.json"
            inspect_value = json.loads(inspect.read_text(encoding="utf-8"))
            inspect_value["descriptor_key_source_mount_unique"] = False
            inspect.write_text(json.dumps(inspect_value) + "\n", encoding="utf-8")
            value["profiles"][0]["container_inspect_artifact_sha256"] = file_sha256(
                inspect
            )
            with self.assertRaisesRegex(ValueError, "network=none"):
                validate_route_artifact_directory(
                    output.parent, value, "deepseek-v4-flash"
                )

            materialize_route_summary(repo, output, "deepseek-v4-flash")
            value = json.loads(output.read_text(encoding="utf-8"))
            inspect = output.parent / "container-inspect-standard-alias.json"
            inspect_value = json.loads(inspect.read_text(encoding="utf-8"))
            inspect_value["descriptor_key_read_only"] = False
            inspect.write_text(json.dumps(inspect_value) + "\n", encoding="utf-8")
            value["profiles"][0]["container_inspect_artifact_sha256"] = file_sha256(
                inspect
            )
            with self.assertRaisesRegex(ValueError, "network=none"):
                validate_route_artifact_directory(
                    output.parent, value, "deepseek-v4-flash"
                )

    def test_preflight_returns_artifact_bound_summary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            whale = repo / "whale"
            whale.write_bytes(b"fixture")
            output = Path("target/provider-route.json")
            absolute_output = repo / output

            transient_paths: list[Path] = []

            def complete(command: list[str], **_: object):
                self.assertIn(str(whale), command)
                descriptor_key = Path(
                    command[command.index("-DescriptorKeyPath") + 1]
                )
                transient_dir = Path(command[command.index("-TransientDir") + 1])
                self.assertTrue(descriptor_key.is_file())
                self.assertEqual(descriptor_key.parent, transient_dir)
                transient_paths.extend((descriptor_key, transient_dir))
                materialize_route_summary(repo, absolute_output, "deepseek-v4-flash")
                return type("Completed", (), {"returncode": 0, "stderr": ""})()

            with (
                patch("cache_process_control.run_captured_command", side_effect=complete),
                patch(
                    "cache_process_control.cleanup_labeled_containers",
                    return_value={"status": "verified_absent", "error": ""},
                ),
            ):
                summary = run_provider_route_preflight(
                    repo, whale, output, "deepseek-v4-flash"
                )

            self.assertEqual(summary["provider_routing"], EXPECTED_ROUTE)
            self.assertEqual(summary["artifact_path"], "target/provider-route.json")
            self.assertEqual(summary["artifact_sha256"], file_sha256(absolute_output))
            self.assertTrue(all(not path.exists() for path in transient_paths))

    def test_parent_timeout_removes_key_and_transient_inspect(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            whale = repo / "whale"
            whale.write_bytes(b"fixture")
            observed: list[Path] = []

            def time_out(command: list[str], **_: object):
                descriptor_key = Path(
                    command[command.index("-DescriptorKeyPath") + 1]
                )
                transient_dir = Path(command[command.index("-TransientDir") + 1])
                self.assertTrue(descriptor_key.is_file())
                (transient_dir / "container-inspect-agent.json").write_text(
                    '{"Mounts":[{"Source":"/tmp/.container-secrets/key"}]}',
                    encoding="utf-8",
                )
                observed.extend((descriptor_key, transient_dir))
                raise BenchmarkTimeoutError(
                    command,
                    1,
                    {
                        "status": "terminated",
                        "descendants_guaranteed_terminated": True,
                    },
                )

            with (
                patch("cache_process_control.run_captured_command", side_effect=time_out),
                patch(
                    "cache_process_control.cleanup_labeled_containers",
                    return_value={"status": "verified_absent", "error": ""},
                ),
                self.assertRaisesRegex(ValueError, "timed out before authorization"),
            ):
                run_provider_route_preflight(
                    repo,
                    whale,
                    Path("target/provider-route.json"),
                    "deepseek-v4-flash",
                    timeout_seconds=1,
                )

            self.assertTrue(all(not path.exists() for path in observed))

    def test_unverified_process_tree_cleanup_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            whale = repo / "whale"
            whale.write_bytes(b"fixture")

            def time_out(command: list[str], **_: object):
                raise BenchmarkTimeoutError(
                    command,
                    1,
                    {
                        "status": "failed",
                        "descendants_guaranteed_terminated": False,
                    },
                )

            with (
                patch("cache_process_control.run_captured_command", side_effect=time_out),
                patch(
                    "cache_process_control.cleanup_labeled_containers",
                    return_value={"status": "verified_absent", "error": ""},
                ),
                self.assertRaisesRegex(ValueError, "process-tree cleanup"),
            ):
                run_provider_route_preflight(
                    repo,
                    whale,
                    Path("target/provider-route.json"),
                    "deepseek-v4-flash",
                    timeout_seconds=1,
                )

    def test_nonzero_preflight_removes_key_and_transient_inspect(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            whale = repo / "whale"
            whale.write_bytes(b"fixture")
            observed: list[Path] = []

            def fail(command: list[str], **_: object):
                descriptor_key = Path(
                    command[command.index("-DescriptorKeyPath") + 1]
                )
                transient_dir = Path(command[command.index("-TransientDir") + 1])
                self.assertTrue(descriptor_key.is_file())
                (transient_dir / "container-inspect-agent.json").write_text(
                    '{"Mounts":[{"Source":"/tmp/.container-secrets/key"}]}',
                    encoding="utf-8",
                )
                observed.extend((descriptor_key, transient_dir))
                return type(
                    "Completed", (), {"returncode": 7, "stderr": "mock failure"}
                )()

            with (
                patch("cache_process_control.run_captured_command", side_effect=fail),
                patch(
                    "cache_process_control.cleanup_labeled_containers",
                    return_value={"status": "verified_absent", "error": ""},
                ),
                self.assertRaisesRegex(ValueError, "mock failure"),
            ):
                run_provider_route_preflight(
                    repo,
                    whale,
                    Path("target/provider-route.json"),
                    "deepseek-v4-flash",
                )

            self.assertTrue(all(not path.exists() for path in observed))

    def test_real_parent_timeout_removes_caller_owned_transient_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            docker = fake_bin / "docker"
            docker.write_text(
                "#!/bin/sh\n"
                'if [ "$1" = "ps" ] || [ "$1" = "network" ]; then exit 0; fi\n'
                "sleep 4\n",
                encoding="utf-8",
            )
            docker.chmod(0o755)
            repo = Path.cwd()
            output = repo / "target/provider-route-timeout-fixture.json"
            environment = {"PATH": f"{fake_bin}:{os.environ['PATH']}"}

            with (
                patch.dict(os.environ, environment),
                patch("cache_provider_route.tempfile.tempdir", directory),
                self.assertRaisesRegex(ValueError, "timed out before authorization"),
            ):
                run_provider_route_preflight(
                    repo,
                    Path("/bin/false"),
                    output,
                    "deepseek-v4-flash",
                    timeout_seconds=1,
                )

            self.assertEqual(list(root.glob("whale-provider-route-*")), [])

    def test_failed_attestation_does_not_persist_host_paths(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            key = root / "descriptor.key"
            key.write_bytes(b"k" * 32)
            transient = root / "transient"
            transient.mkdir()
            output = root / "failed.json"
            script = (
                Path(__file__).resolve().parents[1]
                / "taskspace-benchmark/invoke-provider-route-preflight.ps1"
            )
            completed = subprocess.run(
                [
                    "pwsh",
                    "-NoProfile",
                    "-File",
                    str(script),
                    "-WhaleBin",
                    "/home/private-user/secret-project/bin/whale",
                    "-Model",
                    "deepseek-v4-flash",
                    "-OutputPath",
                    str(output),
                    "-RunId",
                    "provider-route-preflight-123456789abc",
                    "-DescriptorKeyPath",
                    str(key),
                    "-TransientDir",
                    str(transient),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(completed.returncode, 0)
            persisted = output.read_text(encoding="utf-8-sig")
            value = json.loads(persisted)
            self.assertNotIn("/home/private-user", persisted)
            self.assertNotIn("error", value)
            self.assertEqual(
                value["failure_summary"],
                "provider route preflight failed before authorization claim",
            )


if __name__ == "__main__":
    unittest.main()
