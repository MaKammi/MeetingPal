"""Install and exercise a downloaded MeetingPal release on a clean package-test runner."""
import argparse
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import subprocess
import tarfile
import tempfile
import time


def run(args, **kwargs):
    return subprocess.run([str(arg) for arg in args], check=True, timeout=600, **kwargs)


def audit_linux(trace):
    lines = trace.read_text(errors="replace").splitlines()
    if not any("execve(" in line and "/ffmpeg" in line for line in lines):
        raise RuntimeError("The network trace did not follow the bundled FFmpeg child")
    addresses = set()
    for line in lines:
        if not re.search(r"\b(connect|sendto|sendmsg|sendmmsg)\(", line):
            continue
        for ip in re.findall(r'(?:inet_addr\(|inet_pton\(AF_INET6,\s*)"([^"]+)"', line):
            address = ipaddress.ip_address(ip)
            address = getattr(address, "ipv4_mapped", None) or address
            if not address.is_loopback:
                addresses.add(str(address))
    if addresses:
        raise RuntimeError(f"External network destinations observed: {sorted(addresses)}")
    return "strace followed application and child processes; no external IP destination"


def prepare_models(folder):
    catalog = json.loads((Path(__file__).resolve().parents[1] / "resources/models.json").read_text(encoding="utf8"))
    for model in catalog:
        if model["id"] not in {"whisper-small", "qwen-text"}:
            continue
        directory = folder / model["id"]
        directory.mkdir(parents=True)
        for item in model["files"]:
            target = directory / item["filename"]
            run(["curl", "--fail", "--silent", "--show-error", "--location", "--retry", "3",
                 "--output", target, item["url"]])
            with target.open("rb") as file:
                digest = hashlib.file_digest(file, "sha256").hexdigest()
            if digest != item["sha256"] or target.stat().st_size != item["bytes"]:
                raise ValueError("Test model failed verification")
        (directory / "verified.json").write_text(json.dumps(model), encoding="utf8")


def check_bundle(directory):
    root = Path(__file__).resolve().parents[1]
    lock = json.loads((root / "resources/runtimes.json").read_text(encoding="utf8"))
    expected = ["THIRD_PARTY_NOTICES.txt", "MeetingPal-LICENSE.txt", "NotoSans-LICENSE.txt",
                "DISTRIBUTION-NOTES.md", "build-codecs.mjs", "ffmpeg-config.mak", "codec-build.json", "runtime-versions.json"]
    licenses = {"whisper": "LICENSE", "llama": "LICENSE", "ffmpeg": "COPYING.LGPLv2.1", "opus": "COPYING", "libvpx": "LICENSE"}
    for runtime in licenses:
        expected += [f"{runtime}-source.tar.gz", f"{runtime}-LICENSE.txt"]
    missing = [name for name in expected if not (directory / name).is_file() or not (directory / name).stat().st_size]
    if missing:
        raise RuntimeError(f"Missing installed distribution materials: {missing}")
    if json.loads((directory / "runtime-versions.json").read_text()) != lock:
        raise ValueError("The installed runtime manifest differs from the release source")
    if json.loads((directory / "codec-build.json").read_text())["sources"] != lock["codecs"]:
        raise ValueError("Codec build records disagree with the release source")
    actual = (directory / "ffmpeg-config.mak").read_text().splitlines()
    for flag in ["GPL", "VERSION3", "NONFREE", "NETWORK"]:
        if f"!CONFIG_{flag}=yes" not in actual or f"CONFIG_{flag}=yes" in actual:
            raise ValueError(f"The packaged FFmpeg configuration enabled {flag} or lacks its setting")
    for runtime, license_name in licenses.items():
        source = (lock.get(runtime) if runtime in ("llama", "whisper") else lock["codecs"][runtime])
        with tarfile.open(directory / f"{runtime}-source.tar.gz") as archive:
            if archive.pax_headers.get("comment") != source["commit"]:
                raise ValueError(f"Wrong upstream revision for {runtime}")
            upstream = archive.extractfile(license_name).read().splitlines()
        if (directory / f"{runtime}-LICENSE.txt").read_bytes().splitlines() != upstream:
            raise ValueError(f"Installed license differs from upstream {runtime}")
    for name, path in [("MeetingPal-LICENSE.txt", "LICENSE"), ("NotoSans-LICENSE.txt", "resources/NotoSans-LICENSE.txt"),
                       ("build-codecs.mjs", "scripts/build-codecs.mjs")]:
        if (directory / name).read_bytes().splitlines() != (root / path).read_bytes().splitlines():
            raise ValueError(f"Installed file differs from the release source: {name}")
    return len(expected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("version")
    parser.add_argument("--test-ai", action="store_true", help="Stage the pinned public models before starting network observation")
    args = parser.parse_args()
    package = args.package.resolve(strict=True)
    output = args.output.resolve()
    with tempfile.TemporaryDirectory(prefix="meetingpal-package-") as tmp:
        temp = Path(tmp)
        report = temp / "report.json"
        models = temp / "models"
        if args.test_ai:
            prepare_models(models)
        system = platform.system()
        mounted = None
        try:
            if system == "Windows":
                install = temp / "installed"
                run([package, "/S", f"/D={install}"])
                exe = next(install.rglob("meetingpal.exe"))
                materials = exe.parent / "binaries"
                command = ["pwsh", "-NoProfile", "-File", Path(__file__).with_name("windows-network-check.ps1"),
                           "-Executable", exe, "-Report", report]
                evidence = "Windows Filtering Platform and process-creation audit of application and children"
            elif system == "Darwin":
                mounted = temp / "volume"
                run(["hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", mounted, package])
                bundle = next(mounted.glob("*.app"))
                install = temp / bundle.name
                run(["ditto", bundle, install])
                run(["hdiutil", "detach", mounted])
                mounted = None
                app = install / "Contents"
                with (app / "Info.plist").open("rb") as file:
                    exe = app / "MacOS" / plistlib.load(file)["CFBundleExecutable"]
                # The child processes inherit this profile. Network reachability is restricted for the whole run.
                profile = '(version 1)(allow default)(deny network-outbound (remote ip "*:*"))(allow network-outbound (remote ip "localhost:*"))'
                materials = app / "Resources" / "binaries"
                command = ["sandbox-exec", "-p", profile, exe, "--check-offline", report]
                evidence = "installed macOS app with off-device IP traffic denied"
            else:
                if package.suffix == ".AppImage":
                    package.chmod(0o755)
                    # Exercise AppRun from the published image without depending on runner FUSE policy.
                    run([package, "--appimage-extract"], cwd=temp, stdout=subprocess.DEVNULL)
                    image = temp / "squashfs-root"
                    exe = image / "AppRun"
                    materials = next(image.rglob("runtime-versions.json")).parent
                    with tarfile.open(package.with_name(f"MeetingPal_{args.version}_AppImage-notices.tar.gz")) as notices:
                        index = notices.extractfile("SOURCES.txt").read().decode()
                        if "https://launchpad.net/ubuntu/+source/" not in index or "Additional staged names:" in index:
                            raise ValueError("AppImage has unaccounted distribution libraries")
                        for entry in index.splitlines():
                            if " | " in entry:
                                owner = entry.split(" | ", 1)[0].replace(":", "-")
                                if not notices.getmember(f"copyright/{owner}.txt").size:
                                    raise ValueError(f"Missing AppImage license for {owner}")
                else:
                    run(["sudo", "apt-get", "install", "-y", str(package)])
                    name = run(["dpkg-deb", "-f", package, "Package"], capture_output=True, text=True).stdout.strip()
                    files = [Path(line) for line in run(["dpkg-query", "-L", name], capture_output=True, text=True).stdout.splitlines()]
                    exe = next(path for path in files if path.parent == Path("/usr/bin") and path.is_file())
                    materials = next(path.parent for path in files if path.name == "runtime-versions.json")
                trace = temp / "network.trace"
                command = ["xvfb-run", "-a", "dbus-run-session", "--", "strace", "-f", "-s", "256",
                           "-e", "trace=network,execve", "-o", trace, exe, "--check-offline", report]
            if args.test_ai:
                if system == "Windows":
                    command.extend(["-Models", models])
                else:
                    command.append(models)
            count = check_bundle(materials)
            started = time.monotonic()
            try:
                run(command)
            except (subprocess.SubprocessError, OSError):
                if report.is_file():
                    print("Application report:", report.read_text(encoding="utf8"), flush=True)
                raise
            result = json.loads(report.read_text(encoding="utf8"))
            if result.get("version") != args.version or result.get("cloudRequests") != 0:
                raise RuntimeError(f"Installed product check failed: {result}")
            if system == "Linux":
                evidence = audit_linux(trace)
            result.update(launchToExitMs=round((time.monotonic() - started) * 1000),
                          systemVersion=platform.platform(), cpu=os.environ.get("PROCESSOR_IDENTIFIER",platform.machine()),
                          distributionFiles=count, networkEvidence=evidence, package=package.name)
            output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf8")
            print(output.read_text(encoding="utf8"))
        finally:
            if mounted is not None and mounted.is_mount():
                run(["hdiutil","detach",mounted])


if __name__ == "__main__":
    main()
