#!/usr/bin/env python3
"""Verify the self-contained Run Boundary contract fixture."""

from __future__ import annotations

import json
from pathlib import Path
import re
import sys


HERE = Path(__file__).resolve().parent
SCHEMA_PATH = HERE / "run-boundary-v1.schema.json"
EXAMPLE_PATH = HERE / "worked-example-v1.json"
DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")


def fail(message: str) -> None:
    raise ValueError(message)


def load(path: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        fail(f"{path.name} must contain an object")
    return value


def walk_keys(value: object):
    if isinstance(value, dict):
        for key, child in value.items():
            yield key
            yield from walk_keys(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk_keys(child)


def main() -> int:
    schema = load(SCHEMA_PATH)
    example = load(EXAMPLE_PATH)

    if schema.get("title") != "RunBoundaryV1":
        fail("schema title mismatch")
    required = set(schema.get("required", []))
    missing = sorted(required - set(example))
    if missing:
        fail(f"worked example missing top-level fields: {missing}")
    if example.get("schema_version") != "RunBoundaryV1":
        fail("worked example schema mismatch")

    decision = example.get("decision", {})
    if decision.get("status") != "abstain":
        fail("worked example must abstain")
    if decision.get("evidence_status") != "invalid":
        fail("zero-measurement incomplete fixture must remain invalid")

    source_run_key = example.get("derivation", {}).get("source_run_key")
    if not isinstance(source_run_key, str) or not source_run_key.startswith("synthetic-"):
        fail("worked example must be explicitly synthetic")

    semantics = example.get("scope", {}).get("execution_semantics", {})
    if semantics.get("measured_requests", {}).get("value") != 0:
        fail("worked example must preserve zero measured requests")
    if example.get("scope", {}).get("workload", {}).get("attributes", {}).get("fixture") is not True:
        fail("worked example must preserve its fixture label")

    boundaries = example.get("boundaries", [])
    if not isinstance(boundaries, list) or not boundaries:
        fail("at least one boundary is required")
    codes = [item.get("code") for item in boundaries]
    if len(codes) != len(set(codes)):
        fail("boundary codes must be unique")
    required_codes = {"run_incomplete", "no_measured_evidence"}
    if not required_codes.issubset(set(codes)):
        fail("decisive blockers are missing")
    for item in boundaries:
        refs = item.get("evidence_refs")
        if not isinstance(refs, list) or not refs:
            fail(f"boundary {item.get('code')} lacks evidence")
        if item.get("disposition") == "blocking" and not item.get("clearance"):
            fail(f"blocking boundary {item.get('code')} lacks clearance")

    privacy = example.get("privacy", {})
    if privacy != {
        "content_mode": "metadata_only",
        "local_only": True,
        "raw_prompts_included": False,
        "raw_outputs_included": False,
        "redaction": "not_performed_content_omitted",
    }:
        fail("privacy envelope drifted")
    forbidden_keys = {"prompt", "prompt_text", "output", "raw_output", "judge_output"}
    leaked = sorted(forbidden_keys & set(walk_keys(example)))
    if leaked:
        fail(f"metadata-only receipt contains raw-content keys: {leaked}")

    unsupported = set(example.get("unsupported_claims", []))
    if "deployment candidate selection" not in unsupported:
        fail("deployment selection must remain unsupported")

    proof_record_count = 0
    for section in ("observations", "exclusions", "unknowns", "boundaries"):
        for item in example.get(section, []):
            for ref in item.get("evidence_refs", []):
                digest = ref.get("digest")
                if digest is not None and not DIGEST.fullmatch(digest):
                    fail(f"invalid digest in {section}: {digest}")
                if ref.get("kind") == "proof_record":
                    proof_record_count += 1
    if proof_record_count:
        fail("self-contained fixture must not depend on external proof records")

    print(
        json.dumps(
            {
                "schema": "RunBoundaryFixtureVerificationV1",
                "decision": "PASS",
                "boundary_decision": decision["status"],
                "blocking_boundaries": sum(
                    1 for item in boundaries if item.get("disposition") == "blocking"
                ),
                "fixture_scope": "synthetic_local_contract",
                "external_proof_record_count": proof_record_count,
                "raw_content_included": False,
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, json.JSONDecodeError, KeyError, TypeError, ValueError) as exc:
        print(json.dumps({"schema": "RunBoundaryFixtureVerificationErrorV1", "error": str(exc)}), file=sys.stderr)
        raise SystemExit(2)
