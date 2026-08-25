"""Minimal worker entry point used by the foundation smoke test."""

from __future__ import annotations

import json
import sys
from typing import TextIO

from .contracts import CONTRACT_VERSION, IpcEnvelope, MessageType


def run(input_stream: TextIO, output_stream: TextIO) -> int:
    for line in input_stream:
        if not line.strip():
            continue
        try:
            request = IpcEnvelope.from_mapping(json.loads(line))
            if request.protocol_version != CONTRACT_VERSION:
                raise ValueError("Unsupported protocol version")
            response: dict[str, object] = {
                "protocol_version": CONTRACT_VERSION,
                "message_type": MessageType.RESPONSE.value,
                "request_id": request.request_id,
                "payload": {"status": "ready", "worker": "llm-wiki-engine"},
            }
            if request.wiki_id is not None:
                response["wiki_id"] = request.wiki_id
            if request.job_id is not None:
                response["job_id"] = request.job_id
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            response = {
                "protocol_version": CONTRACT_VERSION,
                "message_type": MessageType.ERROR.value,
                "request_id": "unknown",
                "payload": {"category": "invalid_request", "message": str(error)},
            }
        output_stream.write(json.dumps(response, separators=(",", ":")) + "\n")
        output_stream.flush()
    return 0


def main() -> int:
    return run(sys.stdin, sys.stdout)


if __name__ == "__main__":
    raise SystemExit(main())
