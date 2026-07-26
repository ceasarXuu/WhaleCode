#!/usr/bin/env python3
"""R7.1 A2-B0 provider-wire function contracts."""

from __future__ import annotations

from typing import Any


def function_tool(
    name: str, description: str, properties: dict[str, Any], required: list[str]
) -> dict[str, Any]:
    return {
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "strict": False,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": False,
            },
        },
    }


def node_schema() -> dict[str, Any]:
    return {
        "type": "object",
        "properties": {
            "node_id": {"type": "string"},
            "goal": {"type": "string"},
        },
        "required": ["node_id", "goal"],
        "additionalProperties": False,
    }


def action_schema() -> dict[str, Any]:
    return {
        "type": "object",
        "properties": {
            "node_id": {"type": "string"},
            "tool": {"type": "string"},
        },
        "required": ["node_id", "tool"],
        "additionalProperties": False,
    }


def control_tool() -> dict[str, Any]:
    edge = {
        "type": "object",
        "properties": {
            "from": {"type": "string"},
            "to": {"type": "string"},
        },
        "required": ["from", "to"],
        "additionalProperties": False,
    }
    mutation = {
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["complete_node"]},
            "node_id": {"type": "string"},
        },
        "required": ["action", "node_id"],
        "additionalProperties": False,
    }
    initialize = {
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["initialize_and_execute"]},
            "root": node_schema(),
            "work_nodes": {"type": "array", "items": node_schema()},
            "finish": node_schema(),
            "edges": {"type": "array", "items": edge},
            "actions": {"type": "array", "items": action_schema()},
        },
        "required": ["action", "root", "work_nodes", "finish", "edges", "actions"],
        "additionalProperties": False,
    }
    execute = {
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["execute"]},
            "expected_revision": {"type": "integer"},
            "mutations": {"type": "array", "items": mutation},
            "actions": {"type": "array", "items": action_schema()},
        },
        "required": ["action", "expected_revision", "mutations", "actions"],
        "additionalProperties": False,
    }
    return {
        "type": "function",
        "function": {
            "name": "taskspace_control",
            "description": (
                "Declare TaskSpace Map mutations and the ordered node ownership "
                "of native sibling tool calls emitted in this same response."
            ),
            "strict": False,
            "parameters": {
                "type": "object",
                "anyOf": [initialize, execute],
            },
        },
    }
