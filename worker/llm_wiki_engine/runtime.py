"""Concurrent NDJSON worker runtime with cancellable jobs."""

from __future__ import annotations

import os
import threading
import traceback
from dataclasses import dataclass
from typing import Any, TextIO

from .contracts import CONTRACT_VERSION, ErrorCategory, IpcEnvelope, JobState, MessageType
from .ingestion import IngestionCancelled, JobProcessor

WORKER_VERSION = "0.9.0"
CAPABILITIES = ("handshake", "progress", "cancellation", "ingest_documents", "job_logs")


@dataclass(slots=True)
class ActiveJob:
    cancellation: threading.Event
    thread: threading.Thread


class WorkerRuntime:
    def __init__(self, output_stream: TextIO) -> None:
        self._output_stream = output_stream
        self._write_lock = threading.Lock()
        self._jobs_lock = threading.Lock()
        self._jobs: dict[str, ActiveJob] = {}

    def handle(self, request: IpcEnvelope) -> bool:
        action = str(request.payload.get("action", ""))
        if action in {"health", "handshake"}:
            self._write(
                request,
                MessageType.RESPONSE,
                {
                    "status": "ready",
                    "worker": "llm-wiki-engine",
                    "worker_version": WORKER_VERSION,
                    "capabilities": list(CAPABILITIES),
                },
            )
        elif action == "start_job":
            self._start_job(request)
        elif action == "cancel_job":
            self._cancel_job(request)
        elif action == "fake_crash":
            os._exit(17)
        elif action == "shutdown":
            self.cancel_all()
            self._write(request, MessageType.RESPONSE, {"status": "shutting_down"})
            return False
        else:
            self._write_error(
                request,
                ErrorCategory.INVALID_REQUEST,
                f"Unknown action: {action or '<empty>'}",
                retryable=False,
            )
        return True

    def wait_for_jobs(self) -> None:
        with self._jobs_lock:
            jobs = list(self._jobs.values())
        for job in jobs:
            job.thread.join()

    def cancel_all(self) -> None:
        with self._jobs_lock:
            jobs = list(self._jobs.values())
        for job in jobs:
            job.cancellation.set()

    def _start_job(self, request: IpcEnvelope) -> None:
        if request.job_id is None:
            self._write_error(
                request,
                ErrorCategory.INVALID_REQUEST,
                "start_job requires job_id",
                retryable=False,
            )
            return

        source_paths = request.payload.get("source_paths")
        is_real_job = isinstance(source_paths, list)
        steps = bounded_integer(request.payload.get("steps", 8), minimum=1, maximum=100)
        delay_ms = bounded_integer(request.payload.get("delay_ms", 180), minimum=1, maximum=60_000)
        cancellation = threading.Event()
        thread = threading.Thread(
            target=self._run_ingestion_job if is_real_job else self._run_fake_job,
            args=(request, cancellation)
            if is_real_job
            else (request, cancellation, steps, delay_ms),
            name=f"llm-wiki-job-{request.job_id}",
        )
        with self._jobs_lock:
            if request.job_id in self._jobs:
                self._write_error(
                    request,
                    ErrorCategory.INVALID_REQUEST,
                    "A job with this id is already active",
                    retryable=False,
                )
                return
            self._jobs[request.job_id] = ActiveJob(cancellation, thread)
        thread.start()

    def _run_ingestion_job(self, request: IpcEnvelope, cancellation: threading.Event) -> None:
        assert request.job_id is not None
        processor: JobProcessor | None = None
        try:
            source_values = request.payload.get("source_paths")
            if not isinstance(source_values, list) or not all(
                isinstance(value, str) for value in source_values
            ):
                raise ValueError("source_paths must be a list of paths")
            wiki_root = request.payload.get("wiki_root")
            if not isinstance(wiki_root, str) or not wiki_root:
                raise ValueError("wiki_root is required")
            from pathlib import Path

            processor = JobProcessor(
                job_id=request.job_id,
                wiki_id=request.wiki_id or "unknown",
                wiki_root=Path(wiki_root),
                source_paths=[Path(value) for value in source_values],
                ocr_language=str(request.payload.get("ocr_language", "ita+eng")),
                cancellation=cancellation,
                emit=lambda state, progress, message, level, source, detail: self._write(
                    request,
                    MessageType.PROGRESS,
                    {
                        "state": state.value,
                        "progress": progress,
                        "message": message,
                        "log_level": level,
                        "source": source,
                        "detail": detail,
                    },
                ),
            )
            processed = processor.run()
            self._write(
                request,
                MessageType.RESPONSE,
                {"status": "completed", "processed_sources": processed},
            )
        except IngestionCancelled:
            if processor is not None:
                processor.log_cancelled()
            self._write_error(
                request,
                ErrorCategory.CANCELLED,
                "Job cancelled",
                retryable=True,
            )
        except Exception as error:
            detail = "".join(traceback.format_exception_only(type(error), error)).strip()
            if processor is not None:
                processor.log_failure(detail)
            self._write_error(
                request,
                ErrorCategory.INTERNAL,
                "Document processing failed",
                retryable=True,
                detail=detail,
            )
        finally:
            with self._jobs_lock:
                self._jobs.pop(request.job_id, None)

    def _cancel_job(self, request: IpcEnvelope) -> None:
        job_id = str(request.payload.get("job_id", ""))
        with self._jobs_lock:
            job = self._jobs.get(job_id)
        if job is None:
            self._write_error(
                request,
                ErrorCategory.INVALID_REQUEST,
                "The requested job is not active",
                retryable=False,
            )
            return
        job.cancellation.set()
        self._write(
            request,
            MessageType.RESPONSE,
            {"status": "cancellation_requested", "job_id": job_id},
        )

    def _run_fake_job(
        self,
        request: IpcEnvelope,
        cancellation: threading.Event,
        steps: int,
        delay_ms: int,
    ) -> None:
        states = (
            JobState.ACQUIRING,
            JobState.EXTRACTING,
            JobState.INGESTING,
            JobState.VALIDATING,
            JobState.STAGING,
        )
        try:
            for step in range(1, steps + 1):
                if cancellation.wait(delay_ms / 1000):
                    self._write_error(
                        request,
                        ErrorCategory.CANCELLED,
                        "Job cancelled",
                        retryable=True,
                    )
                    return
                state = states[min((step - 1) * len(states) // steps, len(states) - 1)]
                self._write(
                    request,
                    MessageType.PROGRESS,
                    {
                        "state": state.value,
                        "progress": step / steps,
                        "message": f"stage.{state.value}",
                    },
                )
            self._write(
                request,
                MessageType.RESPONSE,
                {
                    "status": "completed",
                    "processed_sources": request.payload.get("source_count", 0),
                },
            )
        finally:
            if request.job_id is not None:
                with self._jobs_lock:
                    self._jobs.pop(request.job_id, None)

    def _write_error(
        self,
        request: IpcEnvelope,
        category: ErrorCategory,
        message: str,
        *,
        retryable: bool,
        detail: str | None = None,
    ) -> None:
        payload: dict[str, Any] = {
            "category": category.value,
            "message": message,
            "retryable": retryable,
        }
        if detail is not None:
            payload["detail"] = detail
        self._write(
            request,
            MessageType.ERROR,
            payload,
        )

    def _write(
        self,
        request: IpcEnvelope,
        message_type: MessageType,
        payload: dict[str, Any],
    ) -> None:
        import json

        response: dict[str, object] = {
            "protocol_version": CONTRACT_VERSION,
            "message_type": message_type.value,
            "request_id": request.request_id,
            "payload": payload,
        }
        if request.wiki_id is not None:
            response["wiki_id"] = request.wiki_id
        if request.job_id is not None:
            response["job_id"] = request.job_id
        with self._write_lock:
            self._output_stream.write(json.dumps(response, separators=(",", ":")) + "\n")
            self._output_stream.flush()


def bounded_integer(value: object, *, minimum: int, maximum: int) -> int:
    if isinstance(value, bool) or not isinstance(value, (int, str)):
        raise ValueError(f"Expected an integer between {minimum} and {maximum}")
    parsed = int(value)
    if not minimum <= parsed <= maximum:
        raise ValueError(f"Expected an integer between {minimum} and {maximum}")
    return parsed
