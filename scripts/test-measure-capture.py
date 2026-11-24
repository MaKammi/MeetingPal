"""Regression check for missing markers and drift with identical WAV durations."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import wave

script = Path(__file__).with_name("measure-capture.py")
with tempfile.TemporaryDirectory() as scratch:
    root = Path(scratch)
    (root / "capture.json").write_text(json.dumps({"durationMs": 4000, "segments": [
        {"startMs": 0, "endMs": 4000, "microphone": "mic.wav", "desktop": "system.wav"}]}))
    for delay, expected in [(0, True), (200, False), (None, False)]:
        for name, offset in [("mic.wav", delay), ("system.wav", 0)]:
            samples = bytearray(8000)
            if offset is not None:
                for start in range(250 + offset, 4000, 1000):
                    samples[start*2:(start+100)*2] = b"\0\x40" * 100
            with wave.open(str(root / name), "wb") as wav:
                wav.setparams((1, 2, 1000, 0, "NONE", ""))
                wav.writeframes(samples)
        result = subprocess.run([sys.executable, script, root, "--audio-only", "--period", "1",
                                "--seconds", "4", "--device", "synthetic"], capture_output=True, text=True)
        assert json.loads(result.stdout)["passed"] == expected and result.returncode == (0 if expected else 1)
print("Aligned, delayed and absent markers classified correctly.")
