#!/usr/bin/env python3
"""Explainable multi-label classification for the Whale Codex overlay."""

from __future__ import annotations

import difflib
import fnmatch
from dataclasses import dataclass


CATEGORIES = (
    "brand_home",
    "provider_model",
    "wire_sse",
    "cache_observability",
    "taskspace_domain",
    "taskspace_host_hooks",
    "multi_agent",
    "web_tools",
    "app_server_protocol",
    "apply_patch",
    "cli_surface",
    "cloud_remote",
    "configuration",
    "instructions_skills",
    "mcp",
    "permission_safety",
    "protocol_contract",
    "provider_transport",
    "sandbox_exec",
    "session_context",
    "tool_runtime",
    "tui_experience",
    "runtime_utilities",
    "upstream_backport",
    "generated_artifact",
    "build_release",
    "developer_tooling",
    "test_fixture",
    "documentation",
    "unclassified",
)


@dataclass(frozen=True)
class Classification:
    categories: tuple[str, ...]
    rule_ids: tuple[str, ...]


TASKSPACE_DOMAIN_GLOBS = (
    "codex-rs/core/src/action_map/**",
    "codex-rs/core/src/context/prompts/taskspace_*",
    "codex-rs/core/src/context/taskspace_contract.rs",
    "codex-rs/core/src/session/taskspace_*.rs",
    "codex-rs/core/src/taskspace_skill*.rs",
    "codex-rs/core/src/tools/handlers/taskspace_control*.rs",
    "codex-rs/protocol/src/taskspace.rs",
    "codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md",
    "codex-rs/state/migrations/0030_taskspace_maps.sql",
    "codex-rs/state/migrations/0031_taskspace_canonical_maps.sql",
    "codex-rs/state/src/model/taskspace_map.rs",
    "codex-rs/state/src/runtime/taskspace_map*.rs",
    "codex-rs/tools/src/taskspace_tool*.rs",
    "codex-rs/skills/src/assets/samples/taskspace-advanced/**",
    "codex-rs/tui/src/app/action_map_viewer.rs",
    "codex-rs/cli/tests/debug_taskspace_map.rs",
    "codex-rs/app-server-protocol/schema/typescript/ActionMapSnapshot*.ts",
    "codex-rs/app-server-protocol/schema/typescript/MapRuntimeMode.ts",
)

TASKSPACE_TOKENS = (
    "TaskSpace",
    "taskspace_",
    "TASKSPACE_",
    "taskspace_control",
    "ActionMap",
    "action_map",
    "MapRuntimeMode",
    "TaskSpaceProjectionPolicy",
    "TaskSpaceTerminalCarrier",
    "--taskspace",
    "thread/taskspace",
    "thread/mapRuntimeMode",
)


def _matches(path: str, patterns: tuple[str, ...]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def _changed_text(before: bytes | None, after: bytes | None) -> str:
    if (before and b"\0" in before) or (after and b"\0" in after):
        return ""
    before_lines = (before or b"").decode("utf-8", "replace").splitlines()
    after_lines = (after or b"").decode("utf-8", "replace").splitlines()
    changed: list[str] = []
    for line in difflib.ndiff(before_lines, after_lines):
        if line.startswith(("+ ", "- ")):
            changed.append(line[2:])
    return "\n".join(changed)


def classify(
    path: str,
    before: bytes | None,
    after: bytes | None,
    backport_paths: set[str],
) -> Classification:
    labels: set[str] = set()
    rules: set[str] = set()
    changed = _changed_text(before, after)

    def add(label: str, rule: str) -> None:
        labels.add(label)
        rules.add(rule)

    if path in backport_paths:
        add("upstream_backport", "ledger.exact_path")

    if _matches(path, TASKSPACE_DOMAIN_GLOBS) or (
        ("/tests/" in path or "/snapshots/" in path)
        and ("taskspace" in path or "action_map_viewer" in path)
    ):
        add("taskspace_domain", "taskspace.domain_path")
    elif any(token in changed for token in TASKSPACE_TOKENS):
        add("taskspace_host_hooks", "taskspace.changed_symbol")

    if path in {
        "codex-rs/Cargo.toml",
        "codex-rs/core/Cargo.toml",
        "codex-rs/state/Cargo.toml",
    }:
        add("taskspace_host_hooks", "taskspace.manifest_dependency")

    if _matches(
        path,
        (
            "codex-cli/bin/whale.js",
            "codex-rs/utils/home-dir/**",
            "codex-rs/secrets/**",
            "codex-rs/login/**",
            "codex-rs/responses-api-proxy/src/bin/whale_*",
            "codex-rs/protocol/src/prompts/base_instructions/whalecode_*",
            "codex-rs/tui/src/onboarding/**",
        ),
    ) or any(token in changed for token in ("WhaleCode", "WHALE_HOME", "~/.whale")):
        add("brand_home", "brand.strong_path_or_symbol")

    if (
        _matches(
            path,
            (
                "codex-rs/model-provider-info/**",
                "codex-rs/models-manager/**",
                "codex-rs/core/src/web_tools/providers/**",
                "codex-rs/tui/src/model_*",
                "codex-rs/tui/src/bottom_pane/model_*",
            ),
        )
        or path
        in {
            "codex-rs/core/src/config/mod.rs",
            "codex-rs/core/src/config/config_tests.rs",
            "codex-rs/core/config.schema.json",
            "codex-rs/protocol/src/models.rs",
            "codex-rs/protocol/src/openai_models.rs",
            "codex-rs/protocol/src/config_types.rs",
            "codex-rs/analytics/src/facts.rs",
            "codex-rs/app-server/tests/suite/v2/model_list.rs",
            "codex-rs/cli/tests/debug_models.rs",
        }
        or "deepseek" in changed.lower()
    ):
        add("provider_model", "provider.strong_path_or_symbol")

    if _matches(path, ("codex-rs/codex-api/src/sse/**",)) or path in {
        "codex-rs/codex-api/src/endpoint/chat_completions.rs",
        "codex-rs/codex-api/src/endpoint/responses.rs",
        "codex-rs/codex-api/src/common.rs",
        "codex-rs/codex-api/src/provider.rs",
        "codex-rs/codex-api/src/lib.rs",
        "codex-rs/core/src/client.rs",
        "codex-rs/core/src/client_common.rs",
        "codex-rs/core/src/client_tests.rs",
        "codex-rs/protocol/src/models.rs",
    }:
        add("wire_sse", "wire.exact_surface")

    if _matches(
        path,
        (
            "codex-rs/core/src/provider_wire*.rs",
            "codex-rs/core/tests/suite/cache_*",
            "codex-rs/core/tests/suite/snapshots/*cache_*",
        ),
    ) or path in {
        "codex-rs/core/src/client.rs",
        "codex-rs/core/src/client_tests.rs",
        "codex-rs/codex-api/src/endpoint/responses.rs",
        "codex-rs/codex-api/src/sse/responses.rs",
        "codex-rs/codex-api/src/sse/chat_completions.rs",
        "codex-rs/codex-api/tests/fixtures/provider_usage_contract.json",
    }:
        add("cache_observability", "cache.exact_surface")

    if _matches(
        path,
        (
            "codex-rs/core/src/agent/**",
            "codex-rs/core/src/multi_agents_v2/**",
            "codex-rs/core/src/tools/handlers/multi_agents*.rs",
            "codex-rs/core/src/tools/handlers/collab*.rs",
        ),
    ) or any(
        token in changed for token in ("AgentPath", "spawn_agent", "MultiAgentV2")
    ):
        add("multi_agent", "multi_agent.path_or_symbol")

    if _matches(path, ("codex-rs/core/src/web_tools/**",)) or any(
        token in changed for token in ("WebSearchProvider", "WebFetchProvider")
    ):
        add("web_tools", "web_tools.path_or_symbol")

    subsystem_rules = (
        (
            "app_server_protocol",
            "subsystem.app_server",
            (
                "codex-rs/app-server/**",
                "codex-rs/app-server-protocol/**",
                "codex-rs/app-server-test-client/**",
            ),
        ),
        (
            "apply_patch",
            "subsystem.apply_patch",
            (
                "codex-rs/apply-patch/**",
                "codex-rs/core/src/tools/handlers/apply_patch*",
                "codex-rs/core/src/tools/runtimes/apply_patch.rs",
            ),
        ),
        (
            "cli_surface",
            "subsystem.cli",
            ("codex-rs/cli/**", "codex-rs/exec/**", "codex-rs/utils/cli/**"),
        ),
        (
            "cloud_remote",
            "subsystem.cloud",
            ("codex-rs/cloud-requirements/**", "codex-rs/cloud-tasks/**"),
        ),
        (
            "configuration",
            "subsystem.configuration",
            (
                "codex-rs/config/**",
                "codex-rs/core/src/config/**",
                "codex-rs/core/src/config_loader/**",
                "codex-rs/features/src/lib.rs",
                "codex-rs/features/src/tests.rs",
            ),
        ),
        (
            "instructions_skills",
            "subsystem.instructions_skills",
            (
                "codex-rs/core-skills/**",
                "codex-rs/skills/**",
                "codex-rs/core/src/agents_md*",
                "codex-rs/core/src/plugins/**",
            ),
        ),
        (
            "mcp",
            "subsystem.mcp",
            (
                "codex-rs/codex-mcp/**",
                "codex-rs/mcp-server/**",
                "codex-rs/rmcp-client/**",
                "codex-rs/core/src/*mcp*",
                "codex-rs/core/src/session/mcp.rs",
                "codex-rs/core/src/tools/handlers/mcp*",
            ),
        ),
        (
            "permission_safety",
            "subsystem.permission_safety",
            (
                "codex-rs/core/src/guardian/**",
                "codex-rs/core/src/tools/network_approval.rs",
                "codex-rs/protocol/src/permissions.rs",
            ),
        ),
        (
            "protocol_contract",
            "subsystem.protocol",
            ("codex-rs/protocol/**", "sdk/typescript/src/**"),
        ),
        (
            "provider_transport",
            "subsystem.provider_transport",
            (
                "codex-rs/codex-api/**",
                "codex-rs/responses-api-proxy/**",
                "codex-rs/model-provider/**",
                "codex-rs/core/src/client*",
                "codex-rs/core/src/realtime_conversation.rs",
            ),
        ),
        (
            "sandbox_exec",
            "subsystem.sandbox_exec",
            (
                "codex-rs/windows-sandbox-rs/**",
                "codex-rs/linux-sandbox/**",
                "codex-rs/exec-server/**",
                "codex-rs/stdio-to-uds/**",
                "codex-rs/core/src/exec*",
                "codex-rs/core/src/shell.rs",
                "codex-rs/core/src/unified_exec/**",
                "codex-rs/core/src/user_shell_command*",
                "codex-rs/core/src/tools/runtimes/shell/**",
                "codex-rs/core/src/tools/runtimes/unified_exec.rs",
                "codex-rs/core/src/tools/sandboxing.rs",
            ),
        ),
        (
            "session_context",
            "subsystem.session_context",
            (
                "codex-rs/core/src/compact*",
                "codex-rs/core/src/context/**",
                "codex-rs/core/src/context_manager/**",
                "codex-rs/core/src/memories/**",
                "codex-rs/core/src/session/**",
                "codex-rs/core/src/tasks/**",
                "codex-rs/rollout/**",
                "codex-rs/rollout-trace/**",
            ),
        ),
        (
            "tool_runtime",
            "subsystem.tool_runtime",
            ("codex-rs/tools/**", "codex-rs/core/src/tools/**"),
        ),
        ("tui_experience", "subsystem.tui", ("codex-rs/tui/**",)),
    )
    for label, rule, patterns in subsystem_rules:
        if _matches(path, patterns):
            add(label, rule)

    if path in {
        "codex-rs/core/src/commit_attribution.rs",
        "codex-rs/core/src/commit_attribution_tests.rs",
        "codex-rs/tui/src/terminal_title.rs",
        "codex-rs/tui/src/update_action.rs",
        "codex-rs/tui/src/version.rs",
        "codex-rs/tui/tooltips.txt",
        "codex-cli/bin/codex.js",
    }:
        add("brand_home", "brand.exact_surface")

    if path in {
        "codex-rs/core/src/test_support.rs",
        "codex-rs/core/src/util.rs",
        "codex-rs/core/src/util_tests.rs",
    }:
        add("runtime_utilities", "runtime_utilities.exact_surface")

    if "/schema/" in path or "/snapshots/" in path or path.endswith(".snap"):
        add("generated_artifact", "generated.path")

    if (
        path.startswith((".github/", ".vscode/"))
        or path in {"BUILD_NUMBER", "MODULE.bazel.lock", "codex-rs/arg0/src/lib.rs"}
        or path.endswith(("Cargo.toml", "Cargo.lock", "BUILD.bazel", ".bzl"))
        or path.startswith("codex-cli/scripts/")
        or path == "codex-cli/package.json"
    ):
        add("build_release", "build.strong_path")

    if path.startswith(".vscode/"):
        add("developer_tooling", "developer_tooling.vscode")

    if "/tests/" in path or "/test/" in path or "/fixtures/" in path:
        add("test_fixture", "test.path")

    if path.endswith((".md", ".mdx")) or "/docs/" in path or path == "UPSTREAM.md":
        add("documentation", "documentation.path")

    if not labels:
        add("unclassified", "fallback.unclassified")
    return Classification(tuple(sorted(labels)), tuple(sorted(rules)))
