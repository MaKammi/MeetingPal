"""Measure physical marker alignment in a MeetingPal recording. No network or devices are opened."""
import argparse
from array import array
import json
from pathlib import Path
import subprocess
import sys
import wave


def track_markers(folder, segments, key):
    peaks = []
    armed = True
    for segment in segments:
        name = segment[key]
        if not name or Path(name).name != name:
            raise ValueError(f"Missing or invalid {key} track")
        with wave.open(str(folder / name), "rb") as recording:
            rate = recording.getframerate()
            if recording.getnchannels() != 1 or recording.getsampwidth() != 2:
                raise ValueError("Expected mono 16-bit PCM")
            if abs(recording.getnframes() * 1000 / rate - (segment["endMs"] - segment["startMs"])) > 2:
                raise ValueError(f"Track duration mismatch in {name}")
            block = rate // 100
            offset = 0
            while data := recording.readframes(block):
                samples = array("h", data)
                if sys.byteorder != "little":
                    samples.byteswap()
                loud = max(max(samples), -min(samples)) >= 4096
                if loud and armed:
                    peaks.append(segment["startMs"] + round(offset * 1000 / rate))
                armed = not loud
                offset += len(samples)
    return peaks


def flashes(folder, ffmpeg):
    # Playback carries the presentation clock of the recorded video.
    cmd = [ffmpeg, "-v", "error", "-i", str(folder / "playback.webm"), "-map", "0:v:0",
           "-vf", "fps=60:start_time=0,scale=1:1,format=gray", "-f", "rawvideo", "-"]
    values = subprocess.run(cmd, capture_output=True, check=True).stdout
    return [round(i * 1000 / 60) for i, value in enumerate(values)
            if value >= 160 and (i == 0 or values[i - 1] < 160)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folder", type=Path)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--audio-only", action="store_true")
    parser.add_argument("--seconds", type=int, default=7200)
    parser.add_argument("--period", type=int, default=60, help="marker spacing in seconds")
    parser.add_argument("--device", required=True, help="OS, device and capture configuration")
    args = parser.parse_args()
    if args.seconds <= 0 or args.period <= 0:
        parser.error("Duration and marker spacing must be positive")
    manifest = json.loads((args.folder / "capture.json").read_text(encoding="utf-8"))
    segments = manifest["segments"]
    end = 0
    for segment in segments:
        if segment["startMs"] != end or segment["endMs"] <= end:
            raise ValueError("Gap or overlap in committed segments")
        end = segment["endMs"]
    tracks = {key: track_markers(args.folder, segments, key) for key in ("microphone", "desktop")}
    if not args.audio_only:
        tracks["video"] = flashes(args.folder, args.ffmpeg)
    reference = tracks["desktop"]
    step = args.period * 1000
    # Require repeated physical markers across the full requested measurement interval.
    valid = (end == manifest["durationMs"] and end >= args.seconds * 1000
             and len(reference) >= max(2, args.seconds // args.period)
             and reference[0] <= step and end - reference[-1] <= step
             and all(abs(b - a - step) <= 100 for a, b in zip(reference, reference[1:])))
    same_count = bool(reference) and all(len(track) == len(reference) for track in tracks.values())
    worst = max((max(points) - min(points) for points in zip(*tracks.values())), default=None) if same_count else None
    passed = valid and worst is not None and worst <= 100
    print(json.dumps({"device": args.device, "durationMs": end, "segments": len(segments),
                      "markerCounts": {k: len(v) for k, v in tracks.items()}, "maxPairwiseOffsetMs": worst,
                      "audioResolutionMs": 10, "videoResolutionMs": None if args.audio_only else round(1000 / 60, 2),
                      "passed": passed}, ensure_ascii=False, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
