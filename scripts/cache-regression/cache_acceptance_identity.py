#!/usr/bin/env python3
"""Source identity checks for accepted cache-run evidence."""

import subprocess
from pathlib import Path
from typing import Any

from cache_budget import validate_budget_proposal
from cache_json import exact_json_equal
from cache_source_evidence import require


def validate_proposal_identity(
    repo: Path,
    proposal: dict[str, Any],
    result: dict[str, Any],
    require_current_head: bool,
) -> None:
    validate_budget_proposal(proposal)
    require(
        proposal.get("proposal_id") == result.get("proposal_id")
        and proposal.get("proposal_sha256") == result.get("proposal_sha256"),
        "cache proposal identity mismatch",
    )
    require(
        proposal.get("subject_commit") == result.get("subject_commit")
        and proposal.get("surface_sha256") == result.get("surface_sha256")
        and exact_json_equal(proposal.get("selection"), result.get("observed_scope")),
        "cache proposal source or scope mismatch",
    )
    if require_current_head:
        head = subprocess.check_output(
            ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
        ).strip()
        require(
            proposal["subject_commit"] == head, "cache proposal is not current HEAD"
        )
