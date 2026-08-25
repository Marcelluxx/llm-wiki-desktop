from __future__ import annotations

import json
from io import StringIO
from pathlib import Path
from typing import Any, cast

from jsonschema import Draft202012Validator, FormatChecker
from llm_wiki_engine.cli import run
from llm_wiki_engine.contracts import (
    ERROR_CATEGORIES,
    INGEST_STRATEGIES,
    JOB_STATES,
    MESSAGE_TYPES,
    PROVIDER_IDS,
    REVIEW_SEVERITIES,
    SOURCE_FORMATS,
    IpcEnvelope,
)

REPOSITORY_ROOT = Path(__file__).parents[2]
FIXTURES = REPOSITORY_ROOT / "tests" / "fixtures" / "contracts"
SCHEMAS = REPOSITORY_ROOT / "schemas" / "v1"


def read_json(name: str) -> dict[str, Any]:
    return cast(dict[str, Any], json.loads((FIXTURES / name).read_text(encoding="utf-8")))


def test_python_enums_match_language_neutral_fixture() -> None:
    values = read_json("contract-enums.json")

    assert values["message_types"] == list(MESSAGE_TYPES)
    assert values["job_states"] == list(JOB_STATES)
    assert values["source_formats"] == list(SOURCE_FORMATS)
    assert values["error_categories"] == list(ERROR_CATEGORIES)
    assert values["review_severities"] == list(REVIEW_SEVERITIES)
    assert values["provider_ids"] == list(PROVIDER_IDS)
    assert values["ingest_strategies"] == list(INGEST_STRATEGIES)


def test_all_shared_examples_validate_against_their_schema() -> None:
    examples = cast(
        list[dict[str, str]],
        json.loads((FIXTURES / "schema-examples.json").read_text(encoding="utf-8")),
    )

    for entry in examples:
        schema = read_schema(entry["schema"])
        Draft202012Validator.check_schema(schema)
        Draft202012Validator(schema, format_checker=FormatChecker()).validate(
            read_json(entry["example"])
        )


def read_schema(name: str) -> dict[str, Any]:
    return cast(dict[str, Any], json.loads((SCHEMAS / name).read_text(encoding="utf-8")))


def test_parses_shared_ipc_fixture() -> None:
    envelope = IpcEnvelope.from_mapping(read_json("ipc-request.json"))

    assert envelope.protocol_version == "1.0"
    assert envelope.payload == {"action": "health"}


def test_worker_returns_a_versioned_ready_response() -> None:
    input_stream = StringIO(json.dumps(read_json("ipc-request.json")) + "\n")
    output_stream = StringIO()

    assert run(input_stream, output_stream) == 0
    response = json.loads(output_stream.getvalue())
    Draft202012Validator(read_schema("ipc-envelope.schema.json")).validate(response)
    assert response["protocol_version"] == "1.0"
    assert response["message_type"] == "response"
    assert response["payload"]["status"] == "ready"


def test_worker_streams_progress_and_completion_for_a_job() -> None:
    request = {
        "protocol_version": "1.0",
        "message_type": "request",
        "request_id": "request-1",
        "wiki_id": "wiki-1",
        "job_id": "job-1",
        "payload": {"action": "start_job", "steps": 3, "delay_ms": 1, "source_count": 2},
    }
    output_stream = StringIO()

    assert run(StringIO(json.dumps(request) + "\n"), output_stream) == 0
    responses = [json.loads(line) for line in output_stream.getvalue().splitlines()]

    assert [response["message_type"] for response in responses] == [
        "progress",
        "progress",
        "progress",
        "response",
    ]
    assert responses[-1]["payload"] == {"status": "completed", "processed_sources": 2}


def test_worker_can_cancel_an_active_job() -> None:
    start = {
        "protocol_version": "1.0",
        "message_type": "request",
        "request_id": "request-start",
        "wiki_id": "wiki-1",
        "job_id": "job-1",
        "payload": {"action": "start_job", "steps": 20, "delay_ms": 100},
    }
    cancel = {
        "protocol_version": "1.0",
        "message_type": "request",
        "request_id": "request-cancel",
        "wiki_id": "wiki-1",
        "payload": {"action": "cancel_job", "job_id": "job-1"},
    }
    output_stream = StringIO()

    assert run(StringIO(json.dumps(start) + "\n" + json.dumps(cancel) + "\n"), output_stream) == 0
    responses = [json.loads(line) for line in output_stream.getvalue().splitlines()]

    assert any(
        response["request_id"] == "request-cancel"
        and response["payload"]["status"] == "cancellation_requested"
        for response in responses
    )
    assert any(
        response["request_id"] == "request-start"
        and response["message_type"] == "error"
        and response["payload"]["category"] == "cancelled"
        for response in responses
    )


def test_transaction_schema_rejects_paths_outside_the_wiki() -> None:
    transaction = read_json("wiki-transaction.json")
    transaction["writes"][0]["relative_path"] = "C:\\Users\\Example\\outside.md"

    errors = list(
        Draft202012Validator(read_schema("wiki-transaction.schema.json")).iter_errors(transaction)
    )

    assert errors
