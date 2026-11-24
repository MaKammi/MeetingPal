"""Check actual Orca speech output for MeetingPal in a disposable X11 session."""
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
    return [line.split("SPEECH OUTPUT:", 1)[1].strip() for line in log.read_text(errors="replace").splitlines()
            if "SPEECH OUTPUT:" in line]


def exercise(language, root, log):
    local = lambda model: {"provider": "local", "model": model}
    config = {"title": "", "language": "auto", "mode": "post", "offline": True,
              "video": {"kind": "none", "id": None, "crop": None}, "transcription": local("whisper-small"),
              "analysis": local("qwen-text"), "vision": local("qwen-vision"), "videoAnalysis": "none",
              "maxRequests": 20, "consent": False, "denoise": False}
    library = root / language / "de.meetingpal.desktop"
    library.mkdir(parents=True)
    (library / "settings.json").write_text(json.dumps({"locale":language,"defaultConfig":config,"retentionDays":None,"updateChecks":"manual"}))
    app = subprocess.Popen(["/usr/bin/meetingpal"], env={**os.environ, "XDG_DATA_HOME": str(library.parent)})
    texts = (["Neues Meeting starten","Meetings durchsuchen","Ein guter Start"] if language == "de"
             else ["Start a new meeting","Search meetings","A good start"])
    def button():
        for application in pyatspi.Registry.getDesktop(0):
            if application is not None and "meetingpal" in (application.name or "").lower():
                node = pyatspi.findDescendant(application, lambda element: element.name == texts[0] and element.getRole()==pyatspi.ROLE_PUSH_BUTTON)
                if node is not None:
                    return node
        return None
    def check(label, action):
        start = len(speech(log))
        action()
        lines = wait_for(lambda: [line for line in speech(log)[start:] if label.lower() in line.lower()], "Orca did not announce "+label)
        print(language, label, "=>", lines[0], flush=True)
    try:
        node = wait_for(button, "Start button missing from installed WebView AT-SPI tree")
        check(texts[0], lambda: node.queryComponent().grabFocus())
        check(texts[1], lambda: subprocess.run(["xdotool","key","ctrl+k"],check=True))
        check(texts[0], lambda: node.queryComponent().grabFocus())
        check(texts[2], lambda: subprocess.run(["xdotool","key","Return"],check=True))
        check(texts[0], lambda: subprocess.run(["xdotool","key","Escape"],check=True))
    finally:
        app.terminate()
        try:
            app.wait(10)
        except subprocess.TimeoutExpired:
            app.kill()
            app.wait()


if __name__ == "__main__":
    log = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="meetingpal-spoken-") as tmp:
        subprocess.run(["gsettings","set","org.gnome.desktop.interface","toolkit-accessibility","true"],check=True)
        manager = subprocess.Popen(["openbox"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        reader = subprocess.Popen(["orca","--replace","--enable=speech","--disable=braille","--debug-file="+str(log)],stdout=subprocess.DEVNULL)
        try:
            wait_for(lambda: log.is_file() and log.stat().st_size > 100, "Orca failed to initialize")
            time.sleep(3)
            for language in ["de","en"]:
                exercise(language,Path(tmp),log)
        finally:
            for process in [reader,manager]:
                process.terminate()
            print("Recent Orca output:", *speech(log)[-20:], sep="\n")
