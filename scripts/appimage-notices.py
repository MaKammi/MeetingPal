"""Collect Ubuntu copyright files for libraries actually staged in the AppImage."""

import io
import subprocess
import tarfile
from pathlib import Path
from urllib.parse import quote


bundle = Path(__file__).resolve().parents[1] / 'src-tauri/target/release/bundle'
appdir = bundle / 'appimage/MeetingPal.AppDir'
libraries = {p.name for p in appdir.rglob('*.so*') if p.is_file()}
if not libraries:
    raise SystemExit('No staged AppImage libraries found')

metadata = subprocess.check_output([
    'dpkg-query', '-W', '-f=${binary:Package}\t${source:Package}\t${source:Version}\n',
], text=True)
packages = {name: (source, version) for name, source, version in
            (line.split('\t') for line in metadata.splitlines())}
owners = {}
matched = set()
for inventory in Path('/var/lib/dpkg/info').glob('*.list'):
    package = inventory.name.removesuffix('.list')
    if package not in packages:
        continue
    files = {name.rsplit('/', 1)[-1] for name in inventory.read_text(errors='replace').splitlines()}
    matches = files & libraries
    copyright_file = Path('/usr/share/doc') / package.split(':')[0] / 'copyright'
    if matches and copyright_file.is_file():
        owners[package] = (copyright_file, sorted(matches))
        matched |= matches
if not owners:
    raise SystemExit('Could not identify any Ubuntu library packages')

with tarfile.open(bundle / 'appimage-notices.tar.gz', 'w:gz', dereference=True) as archive:
    lines = ['MeetingPal AppImage – Ubuntu package sources and copyright notices', '']
    for package, (copyright_file, names) in sorted(owners.items()):
        source, version = packages[package]
        lines.extend([f'{package} | {source} {version}', ', '.join(names),
                      f'https://launchpad.net/ubuntu/+source/{source}/{quote(version, safe="")}', ''])
        archive.add(copyright_file, arcname=f'copyright/{package.replace(":", "-")}.txt')
    archive.add('/usr/share/common-licenses', arcname='common-licenses')
    if libraries - matched:
        lines.append('Additional staged names: ' + ', '.join(sorted(libraries - matched)))
    data = ('\n'.join(lines) + '\n').encode()
    index = tarfile.TarInfo('SOURCES.txt')
    index.size = len(data)
    archive.addfile(index, io.BytesIO(data))
print(f'Collected notices from {len(owners)} Ubuntu packages.')
