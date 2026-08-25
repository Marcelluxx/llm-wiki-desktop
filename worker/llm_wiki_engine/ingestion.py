"""Immutable source acquisition and local document extraction."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import socket
import sqlite3
import subprocess
import sys
import time
from collections.abc import Callable
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from threading import Event

from docx import Document

from .contracts import JobState

EmitEvent = Callable[[JobState, float, str, str | None, str | None, str | None], None]
SUPPORTED_SUFFIXES = {".pdf", ".docx", ".txt", ".md"}


class IngestionCancelled(Exception):
    """Raised when the user cancels an active ingestion."""


@dataclass(frozen=True, slots=True)
class AcquiredSource:
    source_id: str
    original_name: str
    source_format: str
    content_sha256: str
    byte_size: int
    raw_path: Path


class JobLogger:
    def __init__(self, wiki_root: Path, job_id: str, emit: EmitEvent) -> None:
        self._job_id = job_id
        self._emit = emit
        log_directory = wiki_root / ".llm-wiki" / "logs"
        log_directory.mkdir(parents=True, exist_ok=True)
        self.path = log_directory / f"{job_id}.jsonl"

    def write(
        self,
        level: str,
        message: str,
        *,
        state: JobState,
        progress: float,
        source: str | None = None,
        detail: str | None = None,
    ) -> None:
        entry = {
            "timestamp": datetime.now(UTC).isoformat(),
            "level": level,
            "job_id": self._job_id,
            "state": state.value,
            "message": message,
            "source": source,
            "detail": detail,
        }
        with self.path.open("a", encoding="utf-8", newline="\n") as stream:
            stream.write(json.dumps(entry, ensure_ascii=False, separators=(",", ":")) + "\n")
        console_line = f"[LLM Wiki][{level.upper()}][{self._job_id}] {message}"
        if source:
            console_line += f" ({source})"
        if detail:
            console_line += f": {detail}"
        print(console_line, file=sys.stderr, flush=True)
        self._emit(state, progress, message, level, source, detail)


class JobProcessor:
    def __init__(
        self,
        *,
        job_id: str,
        wiki_id: str,
        wiki_root: Path,
        source_paths: list[Path],
        ocr_language: str,
        cancellation: Event,
        emit: EmitEvent,
    ) -> None:
        self._job_id = job_id
        self._wiki_id = wiki_id
        self._wiki_root = wiki_root.resolve()
        self._source_paths = source_paths
        self._ocr_language = normalize_ocr_languages(ocr_language)
        self._cancellation = cancellation
        self._emit = emit
        self._logger = JobLogger(self._wiki_root, job_id, emit)

    def run(self) -> int:
        self._logger.write(
            "info",
            "job.started",
            state=JobState.ACQUIRING,
            progress=0.0,
            detail=f"sources={len(self._source_paths)}",
        )
        acquired = [
            self._acquire(path, index, len(self._source_paths))
            for index, path in enumerate(self._source_paths, start=1)
        ]
        self._check_cancelled()

        pdf_sources = [source for source in acquired if source.source_format == "pdf"]
        direct_sources = [source for source in acquired if source.source_format != "pdf"]
        processed = 0
        for index, source in enumerate(direct_sources, start=1):
            progress = 0.35 + (0.35 * index / max(len(acquired), 1))
            self._extract_direct(source, progress)
            processed += 1
        if pdf_sources:
            self._extract_pdfs(pdf_sources, 0.72)
            processed += len(pdf_sources)

        self._validate(acquired)
        self._logger.write(
            "info",
            "job.completed",
            state=JobState.COMPLETED,
            progress=1.0,
            detail=f"processed={processed}",
        )
        return processed

    def log_failure(self, detail: str) -> None:
        self._logger.write(
            "error",
            "job.failed",
            state=JobState.FAILED,
            progress=0.0,
            detail=detail,
        )

    def log_cancelled(self) -> None:
        self._logger.write(
            "warning",
            "job.cancelled",
            state=JobState.CANCELLED,
            progress=0.0,
        )

    def _acquire(self, source_path: Path, index: int, total: int) -> AcquiredSource:
        self._check_cancelled()
        resolved = source_path.resolve(strict=True)
        suffix = resolved.suffix.lower()
        if not resolved.is_file() or suffix not in SUPPORTED_SUFFIXES:
            raise ValueError(f"Unsupported or missing source: {resolved.name}")
        byte_size = resolved.stat().st_size
        if byte_size > 2 * 1024 * 1024 * 1024:
            raise ValueError(f"Source exceeds the 2 GB limit: {resolved.name}")
        content_hash = sha256_file(resolved, self._cancellation)
        source_id = content_hash
        raw_directory = self._wiki_root / ".llm-wiki" / "raw" / content_hash
        raw_directory.mkdir(parents=True, exist_ok=True)
        safe_name = sanitize_filename(resolved.name)
        raw_path = raw_directory / safe_name
        if not raw_path.exists():
            temporary = raw_directory / f".{safe_name}.{self._job_id}.tmp"
            shutil.copyfile(resolved, temporary)
            os.replace(temporary, raw_path)

        acquired = AcquiredSource(
            source_id=source_id,
            original_name=resolved.name,
            source_format=suffix.removeprefix("."),
            content_sha256=content_hash,
            byte_size=byte_size,
            raw_path=raw_path,
        )
        self._record_source(acquired)
        progress = 0.3 * index / total
        self._logger.write(
            "info",
            "source.acquired",
            state=JobState.ACQUIRING,
            progress=progress,
            source=resolved.name,
            detail=f"sha256={content_hash[:12]} bytes={byte_size}",
        )
        return acquired

    def _extract_direct(self, source: AcquiredSource, progress: float) -> None:
        self._check_cancelled()
        if source.source_format == "docx":
            markdown = docx_to_markdown(source.raw_path)
            extractor = "python-docx"
        else:
            markdown = source.raw_path.read_text(encoding="utf-8-sig", errors="replace")
            extractor = "direct-text"
        if not markdown.strip():
            raise ValueError(f"No readable content extracted from {source.original_name}")
        self._write_artifacts(source, markdown, extractor)
        self._logger.write(
            "info",
            "source.extracted",
            state=JobState.EXTRACTING,
            progress=progress,
            source=source.original_name,
            detail=f"extractor={extractor}",
        )

    def _extract_pdfs(self, sources: list[AcquiredSource], progress: float) -> None:
        self._check_cancelled()
        job_directory = self._wiki_root / ".llm-wiki" / "artifacts" / self._job_id
        input_directory = job_directory / "pdf-input"
        output_directory = job_directory / "pdf-output"
        input_directory.mkdir(parents=True, exist_ok=True)
        output_directory.mkdir(parents=True, exist_ok=True)
        staged_sources: list[Path] = []
        for index, source in enumerate(sources, start=1):
            staged_path = input_directory / (
                f"{index:04d}-{source.content_sha256[:12]}-{sanitize_filename(source.original_name)}"
            )
            if not staged_path.exists():
                try:
                    os.link(source.raw_path, staged_path)
                except OSError:
                    shutil.copyfile(source.raw_path, staged_path)
            staged_sources.append(staged_path)
        port = available_local_port()
        server_log_path = self._logger.path.with_name(f"{self._job_id}-ocr-server.log")
        server_executable = Path(sys.executable).with_name("opendataloader-pdf-hybrid.exe")
        client_executable = Path(sys.executable).with_name("opendataloader-pdf.exe")
        if not server_executable.is_file() or not client_executable.is_file():
            raise RuntimeError("OpenDataLoader hybrid executables are not installed")

        self._logger.write(
            "info",
            "ocr.server_starting",
            state=JobState.EXTRACTING,
            progress=progress,
            detail=f"force_ocr=true languages={self._ocr_language}",
        )
        with server_log_path.open("ab") as server_log:
            server = subprocess.Popen(
                [
                    str(server_executable),
                    "--host",
                    "127.0.0.1",
                    "--port",
                    str(port),
                    "--force-ocr",
                    "--ocr-lang",
                    self._ocr_language,
                ],
                stdin=subprocess.DEVNULL,
                stdout=server_log,
                stderr=subprocess.STDOUT,
                creationflags=hidden_process_flags(),
            )
            try:
                wait_for_server(server, port, self._cancellation, timeout_seconds=300)
                self._logger.write(
                    "info",
                    "ocr.server_ready",
                    state=JobState.EXTRACTING,
                    progress=progress,
                )
                command = [
                    str(client_executable),
                    *[str(path) for path in staged_sources],
                    "--output-dir",
                    str(output_directory),
                    "--format",
                    "markdown,json",
                    "--hybrid",
                    "docling-fast",
                    "--hybrid-url",
                    f"http://127.0.0.1:{port}",
                    "--hybrid-mode",
                    "full",
                ]
                client = subprocess.Popen(
                    command,
                    stdin=subprocess.DEVNULL,
                    stdout=server_log,
                    stderr=subprocess.STDOUT,
                    creationflags=hidden_process_flags(),
                )
                wait_for_process(client, self._cancellation)
                if client.returncode != 0:
                    raise RuntimeError(
                        "OpenDataLoader exited with code "
                        f"{client.returncode}; see {server_log_path.name}"
                    )
            finally:
                terminate_process(server)

        for source, staged_path in zip(sources, staged_sources, strict=True):
            markdown_path = find_pdf_output(output_directory, staged_path.stem, ".md")
            if markdown_path is None:
                raise RuntimeError(
                    f"OpenDataLoader did not produce Markdown for {source.original_name}"
                )
            markdown = markdown_path.read_text(encoding="utf-8", errors="replace")
            if not markdown.strip():
                raise RuntimeError(
                    f"OpenDataLoader produced empty Markdown for {source.original_name}"
                )
            semantic_json_path = find_pdf_output(output_directory, staged_path.stem, ".json")
            self._write_artifacts(
                source,
                markdown,
                "opendataloader-pdf-hybrid-force-ocr",
                semantic_json_path=semantic_json_path,
            )
            self._logger.write(
                "info",
                "source.ocr_completed",
                state=JobState.EXTRACTING,
                progress=0.88,
                source=source.original_name,
            )

    def _write_artifacts(
        self,
        source: AcquiredSource,
        markdown: str,
        extractor: str,
        *,
        semantic_json_path: Path | None = None,
    ) -> None:
        artifact_directory = self._wiki_root / ".llm-wiki" / "artifacts" / source.content_sha256
        artifact_directory.mkdir(parents=True, exist_ok=True)
        atomic_write_text(artifact_directory / "document.md", markdown)
        if semantic_json_path is not None:
            atomic_copy_file(semantic_json_path, artifact_directory / "semantic.json")
        manifest = {
            "schema_version": "1.0",
            "source_id": source.source_id,
            "original_name": source.original_name,
            "source_format": source.source_format,
            "content_sha256": source.content_sha256,
            "byte_size": source.byte_size,
            "extractor": extractor,
            "semantic_json": semantic_json_path is not None,
        }
        atomic_write_text(
            artifact_directory / "manifest.json",
            json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        )
        note_name = f"{slugify(Path(source.original_name).stem)}-{source.content_sha256[:8]}.md"
        note = (
            "---\n"
            f"source_id: {source.source_id}\n"
            f"original_name: {json.dumps(source.original_name, ensure_ascii=False)}\n"
            f"source_format: {source.source_format}\n"
            f"extractor: {extractor}\n"
            "---\n\n"
            f"{markdown.strip()}\n"
        )
        atomic_write_text(self._wiki_root / "sources" / note_name, note)

    def _validate(self, acquired: list[AcquiredSource]) -> None:
        self._logger.write(
            "info",
            "job.validating",
            state=JobState.VALIDATING,
            progress=0.94,
        )
        missing = [
            source.original_name
            for source in acquired
            if not (
                self._wiki_root / ".llm-wiki" / "artifacts" / source.content_sha256 / "document.md"
            ).is_file()
        ]
        if missing:
            raise RuntimeError(f"Missing extraction artifacts: {', '.join(missing)}")

    def _record_source(self, source: AcquiredSource) -> None:
        database_path = self._wiki_root / ".llm-wiki" / "catalog.sqlite3"
        with sqlite3.connect(database_path, timeout=30) as connection:
            connection.execute("PRAGMA foreign_keys=ON")
            connection.execute(
                "INSERT OR REPLACE INTO source_records "
                "(source_id, job_id, original_name, source_format, content_sha256, byte_size) "
                "VALUES (?, ?, ?, ?, ?, ?)",
                (
                    source.source_id,
                    self._job_id,
                    source.original_name,
                    source.source_format,
                    source.content_sha256,
                    source.byte_size,
                ),
            )

    def _check_cancelled(self) -> None:
        if self._cancellation.is_set():
            raise IngestionCancelled


def sha256_file(path: Path, cancellation: Event) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            if cancellation.is_set():
                raise IngestionCancelled
            digest.update(chunk)
    return digest.hexdigest()


def docx_to_markdown(path: Path) -> str:
    document = Document(str(path))
    lines: list[str] = []
    for paragraph in document.paragraphs:
        text = paragraph.text.strip()
        if not text:
            continue
        style = paragraph.style.name.lower() if paragraph.style is not None else ""
        if style.startswith("heading"):
            digits = "".join(character for character in style if character.isdigit())
            level = min(max(int(digits or "1"), 1), 6)
            lines.append(f"{'#' * level} {text}")
        elif "list" in style:
            lines.append(f"- {text}")
        else:
            lines.append(text)
    for table in document.tables:
        rows = [[cell.text.strip().replace("\n", " ") for cell in row.cells] for row in table.rows]
        if not rows:
            continue
        lines.append("| " + " | ".join(rows[0]) + " |")
        lines.append("| " + " | ".join("---" for _ in rows[0]) + " |")
        lines.extend("| " + " | ".join(row) + " |" for row in rows[1:])
    return "\n\n".join(lines).strip() + "\n"


def available_local_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def wait_for_server(
    process: subprocess.Popen[bytes], port: int, cancellation: Event, *, timeout_seconds: int
) -> None:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        if cancellation.is_set():
            terminate_process(process)
            raise IngestionCancelled
        if process.poll() is not None:
            raise RuntimeError(f"OpenDataLoader OCR server exited with code {process.returncode}")
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
            probe.settimeout(0.25)
            if probe.connect_ex(("127.0.0.1", port)) == 0:
                return
        time.sleep(0.25)
    raise TimeoutError("OpenDataLoader OCR server did not become ready within 5 minutes")


def wait_for_process(process: subprocess.Popen[bytes], cancellation: Event) -> None:
    while process.poll() is None:
        if cancellation.wait(0.2):
            terminate_process(process)
            raise IngestionCancelled


def terminate_process(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def find_pdf_output(directory: Path, source_stem: str, suffix: str) -> Path | None:
    candidates = [path for path in directory.rglob(f"*{suffix}") if path.stem == source_stem]
    if not candidates:
        candidates = list(directory.rglob(f"{source_stem}*{suffix}"))
    return candidates[0] if candidates else None


def normalize_ocr_languages(value: str) -> str:
    mapping = {"ita": "it", "eng": "en"}
    values = value.replace("+", ",").split(",")
    normalized = [mapping.get(item.strip().lower(), item.strip().lower()) for item in values]
    return ",".join(item for item in normalized if item) or "it,en"


def sanitize_filename(value: str) -> str:
    forbidden = '<>:"/\\|?*'
    cleaned = "".join("_" if character in forbidden else character for character in value)
    return cleaned.strip(" .")[:180] or "document"


def slugify(value: str) -> str:
    cleaned = "".join(character.lower() if character.isalnum() else "-" for character in value)
    return "-".join(part for part in cleaned.split("-") if part)[:80] or "document"


def atomic_write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(content, encoding="utf-8", newline="\n")
    os.replace(temporary, path)


def atomic_copy_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(f".{destination.name}.{os.getpid()}.tmp")
    shutil.copyfile(source, temporary)
    os.replace(temporary, destination)


def hidden_process_flags() -> int:
    return int(getattr(subprocess, "CREATE_NO_WINDOW", 0))
