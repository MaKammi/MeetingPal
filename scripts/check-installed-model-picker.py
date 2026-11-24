"""Exercise provider selection through the installed WebKit accessibility tree on a clean VM."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import time
import pyatspi

spec = importlib.util.spec_from_file_location("spoken", Path(__file__).with_name("check-spoken-interface.py"))
ui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ui)


def find(predicate):
    for app in pyatspi.Registry.getDesktop(0):
        if app is not None and (app.name or "").lower() == "meetingpal":
            node = pyatspi.findDescendant(app, predicate)
            if node is not None:
                return node


def accessible_text(node):
    try:
        return node.queryText().getText(0, -1)
    except (NotImplementedError, RuntimeError):
        return ""


def key_error(node):
    return "für gemini fehlt der api-schlüssel" in ((node.name or "") + " " + accessible_text(node)).casefold()


def choose(field, letter):
    combo = ui.wait_for(lambda: ui.find_control(field, pyatspi.ROLE_COMBO_BOX), field)
    combo.queryComponent().grabFocus()
    ui.key(letter)
    time.sleep(0.4)


def exercise():
    start = ui.wait_for(lambda: ui.find_control("Neues Meeting starten", pyatspi.ROLE_PUSH_BUTTON), "Start")
    start.queryComponent().grabFocus()
    ui.key("Return")
    ui.wait_for(lambda: ui.find_control("Transkription Anbieter", pyatspi.ROLE_COMBO_BOX), "Transcription field")
    assert not ui.find_control("Vollständig lokal verarbeiten", pyatspi.ROLE_CHECK_BOX)
    for field in ("Auswertung & Assistenz Anbieter", "Transkription Anbieter"):
        choose(field, "g")
        ui.wait_for(lambda: find(key_error), "Missing-key feedback for " + field)
        button = ui.wait_for(lambda: ui.find_control("Modelle vom Anbieter laden", pyatspi.ROLE_PUSH_BUTTON), "Refresh")
        assert button.getState().contains(pyatspi.STATE_SENSITIVE), "Refresh button disabled"
        button.queryComponent().grabFocus()
        ui.key("Return")
        ui.wait_for(lambda: find(key_error), "Retry did not report missing credentials")
        choose(field, "Home")
        ui.wait_for(lambda: not find(key_error), "Cloud controls did not reset for local mode")
        print("Installed WebKit:", field, "selected Gemini, allowed refresh, reported missing key, returned to local", flush=True)


subprocess.run(["gsettings","set","org.gnome.desktop.interface","toolkit-accessibility","true"],check=True)
with tempfile.TemporaryDirectory(prefix="meetingpal-picker-") as folder:
    manager = subprocess.Popen(["openbox"],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    app = subprocess.Popen(["/usr/bin/meetingpal"],env={**os.environ,"XDG_DATA_HOME":folder})
    try:
        exercise()
    except Exception:
        def describe(node):
            content = ((node.name or "") + " " + accessible_text(node)).strip()
            if content:
                print(node.getRoleName(), content[:180], flush=True)
            return False
        find(describe)
        raise
    finally:
        ui.stop(app)
        ui.stop(manager)
