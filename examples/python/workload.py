#!/usr/bin/env python3
"""
CPU, lock, and syscall workload for testing Aperture profiling in OrbStack.
- CPU: math, recursion, string/hash work (flamegraph).
- Lock: threads contending on a shared lock (lock / futex profiling).
- Syscall: file I/O and sleep (syscall tracer).
Runs ~5 minutes so you can attach the agent (e.g. --mode all).
"""
import os
import sys
import time
import math
import hashlib
import json
import threading
import tempfile


def _compute_chunk(n: int) -> float:
    """Leaf: pure number crunching."""
    x = 0.0
    for i in range(n):
        x += math.sqrt(i + 1) * math.sin(i * 0.01)
    return x


def _hash_chunk(data: bytes) -> str:
    """Leaf: hash work (shows up as different stack)."""
    h = hashlib.sha256()
    for i in range(0, len(data), 8192):
        h.update(data[i : i + 8192])
    return h.hexdigest()


def busy_math(depth: int, n: int) -> float:
    """Recursive math path: busy_math -> _compute_chunk."""
    if depth <= 0:
        return _compute_chunk(n)
    return busy_math(depth - 1, n) + _compute_chunk(n // 2)


def busy_strings(rounds: int) -> str:
    """String building and JSON (different stack shape)."""
    out = []
    for i in range(rounds):
        obj = {"i": i, "x": "y" * (i % 100), "nested": {"a": 1, "b": 2}}
        out.append(json.dumps(obj))
    return "".join(out)


def phase_math(_t: float) -> float:
    """Phase: math-heavy call chain (CPU flamegraph)."""
    return busy_math(3, 80_000) + busy_math(2, 120_000)


def phase_io_like(t: float) -> str:
    """Phase: hash + string work (CPU)."""
    data = ("x" * 1000 + str(t)).encode()  # use t so data varies
    return _hash_chunk(data) + busy_strings(50)


def phase_mixed(_t: float) -> float:
    """Phase: mix of math and shallow recursion (CPU)."""
    a = busy_math(1, 100_000)
    b = _compute_chunk(60_000)
    return a + b


# Shared lock for contention (futex under the hood; shows up in lock profiling).
_contend_lock = threading.Lock()


def _lock_worker(rounds: int) -> None:
    """Repeatedly acquire/release shared lock and do a little work (lock contention)."""
    for _ in range(rounds):
        with _contend_lock:
            _compute_chunk(5_000)  # small work while holding lock


def phase_lock(_t: float) -> None:
    """Phase: multiple threads contending on one lock (lock / futex profiling)."""
    num_threads = 4
    rounds = 200
    threads = [threading.Thread(target=_lock_worker, args=(rounds,)) for _ in range(num_threads)]
    for th in threads:
        th.start()
    for th in threads:
        th.join()


def phase_syscall(t: float) -> None:
    """Phase: file I/O and sleep to generate syscalls (syscall tracer)."""
    # File I/O: open, write, read, close (openat, write, read, close syscalls)
    with tempfile.NamedTemporaryFile(mode="w+", delete=False, suffix=".tmp") as f:
        path = f.name
        for i in range(100):
            f.write(f"line {i} {t}\n")
        f.flush()
        f.seek(0)
        f.read()
    os.unlink(path)
    # Sleep triggers nanosleep (or similar) syscalls
    for _ in range(20):
        time.sleep(0.001)


def run_phase(phase_id: int, t: float) -> None:
    """Dispatch so we get CPU, lock, and syscall activity over time."""
    k = phase_id % 5
    if k == 0:
        phase_math(t)
    elif k == 1:
        phase_io_like(t)
    elif k == 2:
        phase_mixed(t)
    elif k == 3:
        phase_lock(t)
    else:
        phase_syscall(t)


def main() -> None:
    duration = 5 * 60  # 5 minutes
    start = time.monotonic()
    phase = 0
    print(
        "workload.py: running 5 min (CPU + lock + syscall). PID:",
        os.getpid(),
        file=sys.stderr,
    )
    while time.monotonic() - start < duration:
        run_phase(phase, start)
        phase += 1
        elapsed = time.monotonic() - start
        if phase % 100 == 0:
            print(f"{elapsed:.1f}s ...", flush=True)
    print("Done.", flush=True)


if __name__ == "__main__":
    main()
