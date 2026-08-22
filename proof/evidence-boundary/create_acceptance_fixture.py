#!/usr/bin/env python3
"""Create deterministic Run Boundary app-acceptance data in an isolated DB."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import sys
from typing import Any


FIXTURE_SCHEMA = "RunBoundaryAcceptanceFixtureV1"
FIXTURE_SUITE = "[ACCEPTANCE FIXTURE — NOT PRODUCT EVIDENCE] Run Boundary V1"
FIXTURE_NOTE = "Synthetic acceptance fixture only; not measured model evidence."
LIVE_DIR = (Path.home() / ".model-colosseum").resolve()
DIGEST_A = "a" * 64
DIGEST_B = "b" * 64


def fail(message: str) -> None:
    raise ValueError(message)


def admitted_db_path(raw: str) -> Path:
    path = Path(raw).expanduser()
    if not path.is_absolute() or path.suffix != ".db":
        fail("acceptance database must be an absolute .db path")
    resolved = path.resolve(strict=False)
    if resolved == LIVE_DIR or LIVE_DIR in resolved.parents:
        fail("refusing to target the live ModelColosseum data directory")
    return resolved


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def canonical_pretty(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False)


def manifest(
    *,
    run_key: str,
    created_at_unix_ms: int,
    suite_id: int,
    prompt_id: int,
    prompt_text: str,
    model_a_id: int,
    model_b_id: int,
    measured_trial_count: int,
) -> tuple[str, str]:
    prompt_digest = sha256_text(prompt_text)
    value: dict[str, Any] = {
        "schema_version": 1,
        "run_key": run_key,
        "created_at_unix_ms": created_at_unix_ms,
        "suite": {
            "id": suite_id,
            "name": FIXTURE_SUITE,
            "description": FIXTURE_NOTE,
            "digest": sha256_text(f"{FIXTURE_SCHEMA}:suite:v1"),
            "prompts": [
                {
                    "id": prompt_id,
                    "category": "coding",
                    "title": "Synthetic acceptance case",
                    "text": prompt_text,
                    "system_prompt": None,
                    "ideal_answer": None,
                    "eval_criteria": "Synthetic deterministic score separation.",
                    "sort_order": 0,
                    "digest": prompt_digest,
                }
            ],
        },
        "models": [
            {
                "database_id": model_a_id,
                "exact_tag": "acceptance-model-a:fixture",
                "digest": DIGEST_A,
                "size_bytes": 1,
                "parameter_size": "fixture",
                "quantization": "FIXTURE",
                "family": "acceptance",
                "modified_at": "2026-08-13T00:00:00Z",
                "capabilities": ["synthetic_acceptance_only"],
            },
            {
                "database_id": model_b_id,
                "exact_tag": "acceptance-model-b:fixture",
                "digest": DIGEST_B,
                "size_bytes": 1,
                "parameter_size": "fixture",
                "quantization": "FIXTURE",
                "family": "acceptance",
                "modified_at": "2026-08-13T00:00:00Z",
                "capabilities": ["synthetic_acceptance_only"],
            },
        ],
        "ollama": {
            "server_version": "acceptance-fixture-no-runtime",
            "endpoint": "http://localhost:11434",
        },
        "hardware": {
            "os_name": "synthetic acceptance host",
            "os_version": "fixture",
            "kernel_version": None,
            "architecture": "fixture",
            "cpu_brand": "not measured",
            "logical_cpu_count": 1,
            "total_memory_bytes": 1,
        },
        "generation": {
            "repetitions": 3,
            "warmup_repetitions": 0,
            "timeout_seconds": 120,
            "temperature": 0.2,
            "num_predict": 32,
            "think": False,
            "seed": 424242,
        },
        "measured_trial_count": measured_trial_count,
        "warmup_trial_count": 0,
    }
    encoded = canonical_pretty(value)
    return encoded, sha256_text(encoded)


def ensure_initialized_empty(conn: sqlite3.Connection) -> None:
    required = {
        "models",
        "test_suites",
        "prompts",
        "benchmark_runs",
        "benchmark_results",
        "benchmark_scores",
        "benchmark_trials",
        "evaluation_run_manifests",
    }
    tables = {
        row[0]
        for row in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table'")
    }
    missing = sorted(required - tables)
    if missing:
        fail(f"database is not initialized; missing tables: {missing}")
    run_count = conn.execute("SELECT COUNT(*) FROM benchmark_runs").fetchone()[0]
    if run_count != 0:
        fail("acceptance database must contain zero runs before fixture creation")


def insert_run(
    conn: sqlite3.Connection,
    *,
    suite_id: int,
    run_key: str | None,
    manifest_json: str | None,
    manifest_digest: str | None,
    started_at: str,
    outcome_status: str,
    comparable: bool,
    repetitions: int,
    notes: str,
) -> int:
    cursor = conn.execute(
        """
        INSERT INTO benchmark_runs
            (suite_id, status, notes, started_at, completed_at, run_key,
             manifest_digest, repetitions, warmup_repetitions, random_seed,
             timeout_seconds, generation_settings_json, outcome_status,
             comparable, comparability_notes)
        VALUES (?, 'completed', ?, ?, ?, ?, ?, ?, 0, 424242, 120, ?, ?, ?, ?)
        """,
        (
            suite_id,
            f"{FIXTURE_NOTE} {notes}",
            started_at,
            started_at,
            run_key,
            manifest_digest,
            repetitions,
            '{"fixture":"RunBoundaryAcceptanceFixtureV1"}',
            outcome_status,
            1 if comparable else 0,
            f"{FIXTURE_NOTE} {notes}",
        ),
    )
    run_id = int(cursor.lastrowid)
    if manifest_json is not None and manifest_digest is not None:
        conn.execute(
            """
            INSERT INTO evaluation_run_manifests
                (run_id, schema_version, manifest_json, manifest_digest, created_at)
            VALUES (?, 1, ?, ?, ?)
            """,
            (run_id, manifest_json, manifest_digest, started_at),
        )
    return run_id


def insert_trial_result(
    conn: sqlite3.Connection,
    *,
    run_id: int,
    prompt_id: int,
    model_id: int,
    trial_key: str,
    repetition_index: int,
    execution_order: int,
    output: str,
    score: int | None,
    scoring_method: str = "manual",
    judge_model_id: int | None = None,
) -> int:
    trial = conn.execute(
        """
        INSERT INTO benchmark_trials
            (run_id, trial_key, prompt_id, model_id, repetition_index,
             trial_kind, execution_order, generation_seed, status,
             started_at, completed_at)
        VALUES (?, ?, ?, ?, ?, 'measured', ?, ?, 'completed', ?, ?)
        """,
        (
            run_id,
            trial_key,
            prompt_id,
            model_id,
            repetition_index,
            execution_order,
            424242 + execution_order,
            "2026-08-13 01:00:00",
            "2026-08-13 01:00:01",
        ),
    )
    trial_id = int(trial.lastrowid)
    result = conn.execute(
        """
        INSERT INTO benchmark_results
            (run_id, prompt_id, model_id, output, tokens_generated,
             time_to_first_token_ms, total_time_ms, tokens_per_second,
             created_at, trial_id, repetition_index, trial_kind, generation_seed)
        VALUES (?, ?, ?, ?, 8, 10, 100, 80.0, ?, ?, ?, 'measured', ?)
        """,
        (
            run_id,
            prompt_id,
            model_id,
            output,
            "2026-08-13 01:00:01",
            trial_id,
            repetition_index,
            424242 + execution_order,
        ),
    )
    result_id = int(result.lastrowid)
    conn.execute(
        "UPDATE benchmark_trials SET result_id = ? WHERE id = ?",
        (result_id, trial_id),
    )
    if score is not None:
        conn.execute(
            """
            INSERT INTO benchmark_scores
                (result_id, score, scoring_method, judge_model_id, notes,
                 judge_manifest_json, status)
            VALUES (?, ?, ?, ?, ?, ?, 'completed')
            """,
            (
                result_id,
                score,
                scoring_method,
                judge_model_id,
                FIXTURE_NOTE,
                '{"fixture":"no judge was run"}',
            ),
        )
    return result_id


def create_fixture(conn: sqlite3.Connection) -> dict[str, Any]:
    ensure_initialized_empty(conn)
    conn.execute("BEGIN IMMEDIATE")
    try:
        conn.execute(
            "UPDATE settings SET value = 'http://127.0.0.1:1' WHERE key = 'ollama_url'"
        )
        conn.execute("DELETE FROM models")
        suite = conn.execute(
            "INSERT INTO test_suites (name, description, is_default) VALUES (?, ?, 0)",
            (FIXTURE_SUITE, FIXTURE_NOTE),
        )
        suite_id = int(suite.lastrowid)
        prompt_text = "SYNTHETIC ACCEPTANCE FIXTURE. No model was called."
        prompt = conn.execute(
            """
            INSERT INTO prompts
                (suite_id, category, title, text, eval_criteria, sort_order)
            VALUES (?, 'coding', 'Synthetic acceptance case', ?, ?, 0)
            """,
            (suite_id, prompt_text, "Synthetic deterministic score separation."),
        )
        prompt_id = int(prompt.lastrowid)

        model_a = conn.execute(
            """
            INSERT INTO models
                (name, display_name, parameter_count, quantization, family,
                 digest, size_bytes, modified_at, capabilities_json)
            VALUES (?, ?, 0, 'FIXTURE', 'acceptance', ?, 1, ?, ?)
            """,
            (
                "acceptance-model-a:fixture",
                "[FIXTURE] Model A",
                DIGEST_A,
                "2026-08-13T00:00:00Z",
                '["synthetic_acceptance_only"]',
            ),
        )
        model_a_id = int(model_a.lastrowid)
        model_b = conn.execute(
            """
            INSERT INTO models
                (name, display_name, parameter_count, quantization, family,
                 digest, size_bytes, modified_at, capabilities_json)
            VALUES (?, ?, 0, 'FIXTURE', 'acceptance', ?, 1, ?, ?)
            """,
            (
                "acceptance-model-b:fixture",
                "[FIXTURE] Model B",
                DIGEST_B,
                "2026-08-13T00:00:00Z",
                '["synthetic_acceptance_only"]',
            ),
        )
        model_b_id = int(model_b.lastrowid)

        directional_manifest, directional_digest = manifest(
            run_key="acceptance-directional-v1",
            created_at_unix_ms=1786579200000,
            suite_id=suite_id,
            prompt_id=prompt_id,
            prompt_text=prompt_text,
            model_a_id=model_a_id,
            model_b_id=model_b_id,
            measured_trial_count=6,
        )
        directional_run = insert_run(
            conn,
            suite_id=suite_id,
            run_key="acceptance-directional-v1",
            manifest_json=directional_manifest,
            manifest_digest=directional_digest,
            started_at="2026-08-13 01:00:00",
            outcome_status="completed",
            comparable=True,
            repetitions=3,
            notes="Directional state.",
        )
        directional_results: list[int] = []
        order = 0
        for model_id, label, score in (
            (model_a_id, "A", 5),
            (model_b_id, "B", 2),
        ):
            for repetition in range(3):
                directional_results.append(
                    insert_trial_result(
                        conn,
                        run_id=directional_run,
                        prompt_id=prompt_id,
                        model_id=model_id,
                        trial_key=f"acceptance-directional-{label.lower()}-{repetition}",
                        repetition_index=repetition,
                        execution_order=order,
                        output=(
                            f"SYNTHETIC ACCEPTANCE FIXTURE output {label}, "
                            f"repetition {repetition + 1}. No model was called."
                        ),
                        score=score,
                    )
                )
                order += 1

        incomplete_manifest, incomplete_digest = manifest(
            run_key="acceptance-incomplete-v1",
            created_at_unix_ms=1786579260000,
            suite_id=suite_id,
            prompt_id=prompt_id,
            prompt_text=prompt_text,
            model_a_id=model_a_id,
            model_b_id=model_b_id,
            measured_trial_count=5,
        )
        incomplete_run = insert_run(
            conn,
            suite_id=suite_id,
            run_key="acceptance-incomplete-v1",
            manifest_json=incomplete_manifest,
            manifest_digest=incomplete_digest,
            started_at="2026-08-13 01:01:00",
            outcome_status="completed_with_failures",
            comparable=False,
            repetitions=3,
            notes="Incomplete and missing-content state.",
        )
        for order, (status, reason) in enumerate(
            (
                ("completed", None),
                ("failed", "synthetic runtime failure"),
                ("timeout", "synthetic timeout"),
                ("cancelled", "synthetic cancellation"),
                ("excluded", "empty output"),
            )
        ):
            conn.execute(
                """
                INSERT INTO benchmark_trials
                    (run_id, trial_key, prompt_id, model_id, repetition_index,
                     trial_kind, execution_order, generation_seed, status,
                     error_message, exclusion_reason, started_at, completed_at)
                VALUES (?, ?, ?, ?, ?, 'measured', ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    incomplete_run,
                    f"acceptance-incomplete-{status}",
                    prompt_id,
                    model_a_id if order % 2 == 0 else model_b_id,
                    order,
                    order,
                    525252 + order,
                    status,
                    reason if status in {"failed", "timeout", "cancelled"} else None,
                    reason if status == "excluded" else None,
                    "2026-08-13 01:01:00",
                    "2026-08-13 01:01:01",
                ),
            )

        legacy_run = insert_run(
            conn,
            suite_id=suite_id,
            run_key=None,
            manifest_json=None,
            manifest_digest=None,
            started_at="2026-08-13 01:02:00",
            outcome_status="legacy",
            comparable=False,
            repetitions=1,
            notes="Legacy missing-provenance state.",
        )
        conn.execute(
            """
            INSERT INTO benchmark_results
                (run_id, prompt_id, model_id, output, tokens_generated,
                 time_to_first_token_ms, total_time_ms, tokens_per_second,
                 created_at, repetition_index, trial_kind, generation_seed)
            VALUES (?, ?, ?, ?, 8, 10, 100, 80.0, ?, 0, 'legacy', 626262)
            """,
            (
                legacy_run,
                prompt_id,
                model_a_id,
                "SYNTHETIC ACCEPTANCE FIXTURE legacy output. No model was called.",
                "2026-08-13 01:02:01",
            ),
        )

        error_manifest, error_digest = manifest(
            run_key="acceptance-load-error-v1",
            created_at_unix_ms=1786579380000,
            suite_id=suite_id,
            prompt_id=prompt_id,
            prompt_text=prompt_text,
            model_a_id=model_a_id,
            model_b_id=model_b_id,
            measured_trial_count=1,
        )
        error_run = insert_run(
            conn,
            suite_id=suite_id,
            run_key="acceptance-load-error-v1",
            manifest_json=error_manifest,
            manifest_digest=error_digest,
            started_at="2026-08-13 01:03:00",
            outcome_status="completed",
            comparable=True,
            repetitions=1,
            notes="Schema-valid evidence-row load-error state.",
        )
        insert_trial_result(
            conn,
            run_id=error_run,
            prompt_id=prompt_id,
            model_id=model_a_id,
            trial_key="acceptance-load-error",
            repetition_index=0,
            execution_order=0,
            output="SYNTHETIC ACCEPTANCE FIXTURE load-error output. No model was called.",
            score=8,
            scoring_method="auto_judge",
            judge_model_id=None,
        )

        conn.commit()
    except Exception:
        conn.rollback()
        raise

    return {
        "schema": FIXTURE_SCHEMA,
        "suite_id": suite_id,
        "run_ids": {
            "directional": directional_run,
            "incomplete": incomplete_run,
            "legacy": legacy_run,
            "load_error": error_run,
        },
        "directional_result_ids": directional_results,
    }


def verify_fixture(conn: sqlite3.Connection, db_path: Path) -> dict[str, Any]:
    rows = conn.execute(
        "SELECT id, notes, outcome_status FROM benchmark_runs ORDER BY id"
    ).fetchall()
    if len(rows) != 4:
        fail(f"expected 4 fixture runs, found {len(rows)}")
    if any(FIXTURE_NOTE not in row[1] for row in rows):
        fail("every fixture run must carry the synthetic-evidence label")
    if conn.execute("SELECT COUNT(*) FROM evaluation_run_manifests").fetchone()[0] != 3:
        fail("expected three immutable fixture manifests")
    if conn.execute("SELECT COUNT(*) FROM benchmark_results").fetchone()[0] != 8:
        fail("expected eight fixture results")
    if conn.execute("SELECT COUNT(*) FROM benchmark_trials").fetchone()[0] != 12:
        fail("expected twelve fixture trials")
    raw_values = [
        value
        for row in conn.execute("SELECT output FROM benchmark_results")
        for value in row
    ]
    if any("SYNTHETIC ACCEPTANCE FIXTURE" not in value for value in raw_values):
        fail("every fixture output must be visibly synthetic")
    model_rows = conn.execute("SELECT name FROM models ORDER BY name").fetchall()
    if model_rows != [
        ("acceptance-model-a:fixture",),
        ("acceptance-model-b:fixture",),
    ]:
        fail("acceptance database contains non-fixture model rows")
    ollama_url = conn.execute(
        "SELECT value FROM settings WHERE key = 'ollama_url'"
    ).fetchone()[0]
    if ollama_url != "http://127.0.0.1:1":
        fail("acceptance database must not contact the configured local runtime")
    conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    digest = hashlib.sha256(db_path.read_bytes()).hexdigest()
    return {
        "schema": "RunBoundaryAcceptanceFixtureVerificationV1",
        "decision": "PASS",
        "database_sha256": digest,
        "run_ids": {
            "directional": rows[0][0],
            "incomplete": rows[1][0],
            "legacy": rows[2][0],
            "load_error": rows[3][0],
        },
        "run_count": len(rows),
        "manifest_count": 3,
        "result_count": 8,
        "trial_count": 12,
        "product_inference_used": False,
        "fixture_label": FIXTURE_SUITE,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("database", help="Absolute path to an initialized disposable .db")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    db_path = admitted_db_path(args.database)
    if not db_path.is_file():
        fail("acceptance database must be initialized by the debug app before seeding")
    conn = sqlite3.connect(db_path)
    try:
        conn.execute("PRAGMA foreign_keys=ON")
        created = None if args.verify_only else create_fixture(conn)
        verified = verify_fixture(conn, db_path)
    finally:
        conn.close()
    print(
        json.dumps(
            {"created": created, "verified": verified},
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, sqlite3.Error, ValueError) as exc:
        print(
            json.dumps(
                {"schema": "RunBoundaryAcceptanceFixtureErrorV1", "error": str(exc)},
                sort_keys=True,
            ),
            file=sys.stderr,
        )
        raise SystemExit(2)
