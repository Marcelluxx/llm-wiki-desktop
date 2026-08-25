"""Versioned NDJSON worker entry point."""

from __future__ import annotations

import json
import sys
from typing import TextIO

from .contracts import CONTRACT_VERSION, ErrorCategory, IpcEnvelope, MessageType
from .runtime import WorkerRuntime


def run(input_stream: TextIO, output_stream: TextIO) -> int:
    runtime = WorkerRuntime(output_stream)
    for line in input_stream:
        if not line.strip():
            continue
        try:
            request = IpcEnvelope.from_mapping(json.loads(line))
            if request.protocol_version != CONTRACT_VERSION:
                raise ValueError("Unsupported protocol version")
            if not runtime.handle(request):
                break
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            response = {
                "protocol_version": CONTRACT_VERSION,
                "message_type": MessageType.ERROR.value,
                "request_id": "unknown",
                "payload": {
                    "category": ErrorCategory.INVALID_REQUEST.value,
                    "message": str(error),
                    "retryable": False,
                },
            }
            output_stream.write(json.dumps(response, separators=(",", ":")) + "\n")
            output_stream.flush()
    runtime.wait_for_jobs()
    return 0


def main() -> int:
    return run(sys.stdin, sys.stdout)


if __name__ == "__main__":
    raise SystemExit(main())
