#!/usr/bin/env python3
"""Read-only capture of the processes around a TEKKEN 8 session, in the fake-/proc fixture format of
crates/remmods-core/tests/fixtures/proc/ (docs/PLAN.md §9). Linux only, standard library only.

Usage: python3 -I scripts/proc-capture.py <label> > local/proc-<label>.json

A process is kept when (every match ignores ASCII case):
  - its comm is "reaper" or its argv[0] ends in "/reaper";
  - an argv element contains "steamapps/common" (or "steamapps\\common");
  - its comm or argv[0] ends in ".exe";
  - its comm or an argv element contains wineserver, pressure-vessel, steam-launch-wrapper,
    fossilize or modmanager;
  - or it descends from a kept reaper.
$HOME becomes /home/user (also in Wine's \\home\\<name> form). "expect" is left null for you to
fill in; "note" says why a process was kept. Vanished processes are skipped. Nothing is written
except the JSON on stdout.

Review and redact every capture by hand before it becomes a committed fixture: the repository is
public, and a capture can still hold personal paths, names or tools.
"""

import datetime
import errno
import json
import os
import re
import sys

PROC = "/proc"
ANON_HOME = "/home/user"
CONTAINS_TERMS = ("wineserver", "pressure-vessel", "steam-launch-wrapper", "fossilize", "modmanager")
STEAM_COMMON = re.compile(r"steamapps[/\\]common", re.IGNORECASE | re.ASCII)
LOWER_ASCII = str.maketrans("ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz")
VANISHED = (errno.ENOENT, errno.ESRCH)


class Process:
    def __init__(self, pid, ppid, comm_raw, cmdline_raw):
        self.pid = pid
        self.ppid = ppid
        self.cmdline_raw = cmdline_raw
        try:
            self.comm = comm_raw.decode("utf-8")
            self.comm_lossy = False
        except UnicodeDecodeError:
            self.comm = comm_raw.decode("utf-8", errors="replace")
            self.comm_lossy = True
        # Lossy and without the NUL padding, for matching only; the record keeps the exact bytes.
        trimmed = cmdline_raw.rstrip(b"\0")
        self.argv = (
            [arg.decode("utf-8", errors="replace") for arg in trimmed.split(b"\0")] if trimmed else []
        )


def lower_ascii(text):
    return text.translate(LOWER_ASCII)


def home_patterns():
    """(pattern, replacement) pairs that turn $HOME into /home/user in raw bytes."""
    home = os.path.expanduser("~").rstrip("/")
    if not home.startswith("/"):
        return []
    # The name must not continue (/home/ann must not touch /home/anna); Wine paths ignore case.
    not_continued = rb"(?![A-Za-z0-9._-])"
    unix = os.fsencode(home)
    wine = unix.replace(b"/", b"\\")
    anon = os.fsencode(ANON_HOME)
    return [
        (re.compile(re.escape(unix) + not_continued), anon),
        (re.compile(re.escape(wine) + not_continued, re.IGNORECASE), anon.replace(b"/", b"\\")),
    ]


def anonymised(raw, patterns):
    for pattern, replacement in patterns:
        raw = pattern.sub(lambda _match, text=replacement: text, raw)
    return raw


def read_bytes(path):
    with open(path, "rb") as file:
        return file.read()


def parent_pid(stat_raw):
    # "pid (comm) state ppid …": comm may hold spaces and parentheses, so split after the last ")".
    fields = stat_raw[stat_raw.rindex(b")") + 1 :].split()
    return int(fields[1])


def read_process(pid, patterns):
    """The process, or None when it vanished. Other read errors are reported on stderr."""
    base = os.path.join(PROC, str(pid))
    try:
        stat_raw = read_bytes(os.path.join(base, "stat"))
        comm_raw = read_bytes(os.path.join(base, "comm"))
        cmdline_raw = read_bytes(os.path.join(base, "cmdline"))
    except OSError as error:
        if error.errno not in VANISHED:
            print(f"proc-capture: skipped pid {pid}: {error.strerror}", file=sys.stderr)
        return None
    try:
        ppid = parent_pid(stat_raw)
    except (ValueError, IndexError):
        print(f"proc-capture: skipped pid {pid}: unreadable stat line", file=sys.stderr)
        return None
    if comm_raw.endswith(b"\n"):
        comm_raw = comm_raw[:-1]
    return Process(pid, ppid, anonymised(comm_raw, patterns), anonymised(cmdline_raw, patterns))


def snapshot(patterns):
    own_pid = os.getpid()
    processes = {}
    for name in os.listdir(PROC):
        if not name.isdigit() or int(name) == own_pid:
            continue
        process = read_process(int(name), patterns)
        if process is not None:
            processes[process.pid] = process
    return processes


def is_reaper(process):
    first = lower_ascii(process.argv[0]) if process.argv else ""
    return lower_ascii(process.comm) == "reaper" or first.endswith("/reaper")


def keep_reason(process):
    """Why the process matters, or None. Exact rules only: broad terms such as "proton" or the app id
    also caught a VPN daemon and the capturing shell's own command line."""
    if is_reaper(process):
        return "reaper"
    if any(STEAM_COMMON.search(arg) for arg in process.argv):
        return "steamapps/common in argv"
    first = lower_ascii(process.argv[0]) if process.argv else ""
    if lower_ascii(process.comm).endswith(".exe") or first.endswith(".exe"):
        return ".exe"
    texts = [lower_ascii(process.comm)] + [lower_ascii(arg) for arg in process.argv]
    for term in CONTAINS_TERMS:
        if any(term in text for text in texts):
            return term
    return None


def kept_processes(processes):
    """pid -> reason for every kept process."""
    reasons = {}
    for pid, process in processes.items():
        reason = keep_reason(process)
        if reason is not None:
            reasons[pid] = reason
    children = {}
    for pid, process in processes.items():
        children.setdefault(process.ppid, []).append(pid)
    for reaper in sorted(pid for pid, process in processes.items() if is_reaper(process)):
        stack = list(children.get(reaper, []))
        seen = {reaper}
        while stack:
            pid = stack.pop()
            if pid in seen:
                continue
            seen.add(pid)
            reasons.setdefault(pid, f"descends from reaper {reaper}")
            stack.extend(children.get(pid, []))
    return reasons


def cmdline_fields(raw):
    """The fixture's cmdline, nulPadding and cmdlineHex for raw /proc/<pid>/cmdline bytes, plus a
    note. The fixture builder writes args joined by NUL + one NUL + nulPadding NULs; bytes that
    form cannot reproduce exactly (no final NUL, only NULs, not UTF-8) are kept as hex instead."""
    if not raw:
        return {"cmdline": [], "nulPadding": 0, "cmdlineHex": None}, None
    trimmed = raw.rstrip(b"\0")
    as_hex = {"cmdline": None, "nulPadding": 0, "cmdlineHex": raw.hex()}
    if not trimmed:
        return as_hex, "cmdline only NULs"
    if not raw.endswith(b"\0"):
        return as_hex, "cmdline without final NUL"
    try:
        args = [arg.decode("utf-8") for arg in trimmed.split(b"\0")]
    except UnicodeDecodeError:
        return as_hex, "cmdline not UTF-8"
    padding = len(raw) - len(trimmed) - 1
    return {"cmdline": args, "nulPadding": padding, "cmdlineHex": None}, None


def record(process, reason):
    fields, anomaly = cmdline_fields(process.cmdline_raw)
    notes = [f"kept: {reason}"]
    if process.comm_lossy:
        notes.append("comm not UTF-8 (lossy)")
    if anomaly:
        notes.append(anomaly)
    return {
        "pid": process.pid,
        "ppid": process.ppid,
        "comm": process.comm,
        **fields,
        "note": "; ".join(notes),
    }


def capture(label):
    processes = snapshot(home_patterns())
    reasons = kept_processes(processes)
    taken = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    return {
        "version": 1,
        "source": f"{label}: scripts/proc-capture.py at {taken} on Linux {os.uname().release}; "
        "review and redact by hand before committing",
        "expect": {"running": None, "signal": None, "pid": None, "otherManager": None},
        "processes": [record(processes[pid], reasons[pid]) for pid in sorted(reasons)],
    }


def usage(stream):
    print(__doc__.strip(), file=stream)


def main(argv):
    if len(argv) == 2 and argv[1] in ("-h", "--help"):
        usage(sys.stdout)
        return 0
    if len(argv) != 2 or not argv[1]:
        usage(sys.stderr)
        return 2
    if not os.path.exists(os.path.join(PROC, "self", "stat")):
        print("proc-capture: needs the Linux /proc file system", file=sys.stderr)
        return 1
    document = capture(argv[1])
    sys.stdout.write(json.dumps(document, indent=2, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
