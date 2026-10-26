"""Private POSIX engine lifetime guard. No network, profiles or log output."""
import os
import signal
import sys
import time


def terminate_group(group):
    try:
        os.killpg(group, signal.SIGTERM)
    except ProcessLookupError:
        return
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        try:
            os.killpg(group, 0)
        except ProcessLookupError:
            return
        time.sleep(0.05)
    try:
        os.killpg(group, signal.SIGKILL)
    except ProcessLookupError:
        pass


def main():
    if os.name != 'posix' or len(sys.argv) != 2:
        return 2
    group = int(sys.argv[1])
    if group <= 1 or group == os.getpgrp():
        return 2
    try:
        if os.getpgid(group) != group:
            return 2
    except ProcessLookupError:
        # An engine may exit immediately, before the handshake completes.
        pass
    sys.stdout.buffer.write(b'R')
    sys.stdout.buffer.flush()
    try:
        # The application owns the only writer. EOF also covers SIGKILL/crash.
        while sys.stdin.buffer.read(1):
            pass
    finally:
        terminate_group(group)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
