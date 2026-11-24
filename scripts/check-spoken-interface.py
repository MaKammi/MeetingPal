"""Verify Orca's spoken output from an installed MeetingPal in a disposable X11 session."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

import pyatspi


def wait_for(probe, reason):
    deadline = time.monotonic() + 35
    while time.monotonic() < deadline:
        try:
            found = probe()
            if found:
                return found
        except (LookupError, RuntimeError):
            pass
        time.sleep(0.25)
    raise RuntimeError(reason)


def speech(log):
    if not log.is_file():
        return []
    return [line.split("SPEECH OUTPUT:", 1)[1].strip()
            for line in log.read_text(errors="replace").splitlines() if "SPEECH OUTPUT:" in line]


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def find_control(label, role, focused=False):
    for app in pyatspi.Registry.getDesktop(0):
        if app is None or "meetingpal" not in (app.name or "").lower():
            continue
        def matches(control):
            return (control.name == label and control.getRole() == role
                    and (not focused or control.getState().contains(pyatspi.STATE_FOCUSED)))
        node = pyatspi.findDescendant(app, matches)
        if node is not None:
            return node
    return None


def key(chord):
    subprocess.run(["xdotool", "key", "--clearmodifiers", chord], check=True)


def exercise(locale, root):
    config = {
        "title": "", "language": "auto", "mode": "post", "offline": True,
        "video": {"kind": "none", "id": None, "crop": None},
        "transcription": {"provider": "local", "model": "whisper-small"},
        "analysis": {"provider": "local", "model": "qwen-text"},
        "vision": {"provider": "local", "model": "qwen-vision"},
        "videoAnalysis": "none", "maxRequests": 20, "consent": False, "denoise": False,
    }
    library = root / locale / "de.meetingpal.desktop"
    library.mkdir(parents=True)
    (library / "settings.json").write_text(json.dumps({
        "locale": locale, "defaultConfig": config, "retentionDays": None, "updateChecks": "manual",
    }))
    labels = {
        "de": ("Neues Meeting starten", "Meetings durchsuchen", "Ein guter Start."),
        "en": ("Start a new meeting", "Search meetings", "A good start."),
    }
    start, search, dialog = labels[locale]
    app = subprocess.Popen(["/usr/bin/meetingpal"], env={**os.environ, "XDG_DATA_HOME": str(library.parent)})
    try:
        button = wait_for(lambda: find_control(start, pyatspi.ROLE_PUSH_BUTTON), start)
        steps = [
            (button.queryComponent().grabFocus, start, pyatspi.ROLE_PUSH_BUTTON, True),
            (lambda: key("ctrl+k"), search, pyatspi.ROLE_ENTRY, True),
            (button.queryComponent().grabFocus, start, pyatspi.ROLE_PUSH_BUTTON, True),
            (lambda: key("Return"), dialog, pyatspi.ROLE_DIALOG, False),
            (lambda: key("Escape"), start, pyatspi.ROLE_PUSH_BUTTON, True),
        ]
        for action, label, role, focused in steps:
            action()
            wait_for(lambda: find_control(label, role, focused), "AT-SPI did not reach " + label)
            # Orca handles focus asynchronously on its own GLib event loop.
            time.sleep(1)
        return [label for _, label, _, _ in steps]
    finally:
        stop(app)


def main(log):
    subprocess.run(["gsettings", "set", "org.gnome.desktop.interface", "toolkit-accessibility", "true"], check=True)
    manager = subprocess.Popen(["openbox"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    reader = subprocess.Popen(["orca", "--replace", "--enable=speech", "--disable=braille",
                               "--debug-file=" + str(log)], stdout=subprocess.DEVNULL)
    expected = []
    try:
        with tempfile.TemporaryDirectory(prefix="meetingpal-spoken-") as tmp:
            wait_for(lambda: log.is_file() and log.stat().st_size, "Orca failed to initialize")
            time.sleep(3)
            for locale in ("de", "en"):
                expected.extend(exercise(locale, Path(tmp)))
    finally:
        # Orca buffers its debug log. A normal shutdown flushes its final utterances.
        stop(reader)
        stop(manager)
        lines = speech(log)
        print("Recent Orca output:", *lines[-35:], sep="\n")
    utterances = iter(lines)
    for label in expected:
        match = next((line for line in utterances if label.casefold() in line.casefold()), None)
        if match is None:
            raise RuntimeError("Missing spoken label in expected order: " + label)
        print(label, "=>", match)


if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve())
