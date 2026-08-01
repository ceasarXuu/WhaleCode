#!/usr/bin/env python3
"""Drive one active-prefix app-server run without semantic intervention."""

from __future__ import annotations

import argparse
import json
import os
import queue
import subprocess
import threading
import time
from pathlib import Path
from typing import Any, Callable


class RpcFailure(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--thread-id", required=True)
    parser.add_argument("--mode", choices=("standard", "taskspace"), required=True)
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--events", required=True)
    parser.add_argument("--stderr", required=True)
    parser.add_argument("--summary", required=True)
    parser.add_argument("--last-message", required=True)
    parser.add_argument("--timeout-seconds", type=int, default=900)
    return parser.parse_args()


class Client:
    def __init__(self, args: argparse.Namespace) -> None:
        self.args = args
        self.started_at = time.monotonic()
        self.events_file = Path(args.events).open("w", encoding="utf-8")
        self.stderr_file = Path(args.stderr).open("w", encoding="utf-8")
        self.process = subprocess.Popen(
            [args.binary],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.stderr_file,
            text=True,
            encoding="utf-8",
            bufsize=1,
            env=os.environ.copy(),
        )
        self.output_queue: queue.Queue[str | None] = queue.Queue()
        self.output_thread = threading.Thread(target=self._read_output, daemon=True)
        self.output_thread.start()
        self.messages: list[dict[str, Any]] = []
        self.final_messages: list[str] = []

    def _read_output(self) -> None:
        if self.process.stdout is None:
            self.output_queue.put(None)
            return
        for line in self.process.stdout:
            self.output_queue.put(line)
        self.output_queue.put(None)

    def send(self, method: str, request_id: int | None, params: dict[str, Any]) -> None:
        if self.process.stdin is None:
            raise RpcFailure("app-server stdin is unavailable")
        payload: dict[str, Any] = {"method": method, "params": params}
        if request_id is not None:
            payload["id"] = request_id
        self.process.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        self.process.stdin.flush()

    def read(self, timeout: float) -> dict[str, Any]:
        try:
            line = self.output_queue.get(timeout=timeout)
        except queue.Empty as error:
            raise TimeoutError("timed out waiting for app-server output") from error
        if line is None:
            raise RpcFailure(f"app-server stream closed with {self.process.poll()}")
        self.events_file.write(line)
        self.events_file.flush()
        try:
            message = json.loads(line)
        except json.JSONDecodeError as error:
            raise RpcFailure(f"invalid app-server JSON: {error}") from error
        self.messages.append(message)
        self._validate(message)
        self._capture_message(message)
        return message

    def wait_for(
        self,
        label: str,
        predicate: Callable[[dict[str, Any]], bool],
    ) -> dict[str, Any]:
        deadline = self.started_at + self.args.timeout_seconds
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RpcFailure(f"timeout waiting for {label}")
            try:
                message = self.read(min(remaining, 30.0))
            except TimeoutError:
                continue
            if predicate(message):
                return message

    @staticmethod
    def _validate(message: dict[str, Any]) -> None:
        if message.get("error") is not None:
            raise RpcFailure(f"JSON-RPC error: {message['error']}")
        if message.get("method") == "error":
            params = message.get("params") or {}
            if not params.get("willRetry", False):
                raise RpcFailure(f"provider error: {params.get('error')}")
        if message.get("method") == "turn/completed":
            turn = (message.get("params") or {}).get("turn") or {}
            if turn.get("status") == "failed":
                raise RpcFailure(f"turn failed: {turn.get('error')}")

    def _capture_message(self, message: dict[str, Any]) -> None:
        if message.get("method") != "item/completed":
            return
        item = (message.get("params") or {}).get("item") or {}
        if item.get("type") != "agentMessage":
            return
        text = item.get("text")
        if isinstance(text, str):
            self.final_messages.append(text)

    def close(self) -> int:
        if self.process.stdin is not None:
            self.process.stdin.close()
        try:
            return self.process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            return self.process.wait(timeout=15)

    def finish_files(self, summary: dict[str, Any]) -> None:
        self.events_file.close()
        self.stderr_file.close()
        Path(self.args.summary).write_text(
            json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        last_message = self.final_messages[-1] if self.final_messages else ""
        Path(self.args.last_message).write_text(last_message + "\n", encoding="utf-8")


def is_response(request_id: int) -> Callable[[dict[str, Any]], bool]:
    return lambda message: message.get("id") == request_id


def is_turn_completed(message: dict[str, Any]) -> bool:
    return message.get("method") == "turn/completed"


def main() -> int:
    args = parse_args()
    client = Client(args)
    phase_times: dict[str, int] = {}
    try:
        client.send(
            "initialize",
            1,
            {
                "clientInfo": {
                    "name": "r5_map_compression_benchmark",
                    "title": "R5 Map Compression Benchmark",
                    "version": "1",
                },
                "capabilities": {"experimentalApi": True},
            },
        )
        client.wait_for("initialize", is_response(1))
        client.send("initialized", None, {})
        client.send(
            "thread/resume",
            2,
            {
                "threadId": args.thread_id,
                "excludeTurns": True,
                "cwd": "/workspace",
                "model": "deepseek-v4-flash",
                "modelProvider": "deepseek",
                "approvalPolicy": "never",
                "permissionProfile": {"type": "disabled"},
                "config": {
                    "model_reasoning_effort": "max",
                    "features": {"plugins": False},
                    "skills": {
                        "bundled": {"enabled": False},
                        "include_instructions": False,
                    },
                },
            },
        )
        client.wait_for("thread resume", is_response(2))
        if args.mode == "standard":
            client.send(
                "thread/mapRuntimeMode/set",
                3,
                {"threadId": args.thread_id, "mode": "standard"},
            )
            client.wait_for("standard mode", is_response(3))

        compact_started = time.monotonic()
        client.send("thread/compact/start", 4, {"threadId": args.thread_id})
        client.wait_for("compact accepted", is_response(4))
        compact_turn = client.wait_for("compact completed", is_turn_completed)
        phase_times["compact_ms"] = round((time.monotonic() - compact_started) * 1000)

        prompt = Path(args.prompt).read_text(encoding="utf-8")
        continuation_started = time.monotonic()
        client.send(
            "turn/start",
            5,
            {
                "threadId": args.thread_id,
                "input": [{"type": "text", "text": prompt}],
                "cwd": "/workspace",
                "approvalPolicy": "never",
                "permissionProfile": {"type": "disabled"},
            },
        )
        client.wait_for("continuation accepted", is_response(5))
        continuation_turn = client.wait_for("continuation completed", is_turn_completed)
        phase_times["continuation_ms"] = round(
            (time.monotonic() - continuation_started) * 1000
        )
        exit_code = client.close()
        if exit_code != 0:
            raise RpcFailure(f"app-server exited with {exit_code}")
        summary = {
            "schema_version": "taskspace-active-prefix-client-v1",
            "status": "completed",
            "mode": args.mode,
            "thread_id": args.thread_id,
            "event_count": len(client.messages),
            "phase_times": phase_times,
            "compact_turn": (compact_turn.get("params") or {}).get("turn"),
            "continuation_turn": (continuation_turn.get("params") or {}).get("turn"),
        }
        client.finish_files(summary)
        return 0
    except Exception as error:
        exit_code = client.close()
        summary = {
            "schema_version": "taskspace-active-prefix-client-v1",
            "status": "failed",
            "mode": args.mode,
            "error": str(error),
            "app_server_exit_code": exit_code,
            "event_count": len(client.messages),
            "phase_times": phase_times,
        }
        client.finish_files(summary)
        print(str(error), flush=True)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
