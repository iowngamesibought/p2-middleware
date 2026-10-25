"""Build the release zip from compiled binaries, and optionally a clean source tree.

Writes dist/release/p2-middleware-v<version>-windows-x64.zip plus SHA256SUMS.txt.
With --source, also copies the publishable source files to dist/source/p2-middleware.
Refuses binaries that still contain this machine's user-profile path.
"""
from pathlib import Path
import argparse
import hashlib
import os
import shutil
import tomllib
import zipfile

root = Path(__file__).resolve().parents[1]
BINARIES = ('p2-middleware.exe', 'p2-bridge.exe', 'p2-route.exe')
DOCS = ('README.md', 'LICENSE', 'SOURCE_NOTICES.md')
SOURCE_FILES = ('.gitattributes', '.gitignore', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
                'README.md', 'LICENSE', 'SOURCE_NOTICES.md', 'scripts/package.py')
SOURCE_DIRS = ('src', 'crates')


def personal_markers():
    markers = []
    profile = os.environ.get('USERPROFILE')
    if profile:
        for form in {profile, profile.replace('\\', '/'), profile.lower(), profile.lower().replace('\\', '/')}:
            markers += [form.encode(), form.encode('utf-16-le')]
    return markers


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--build-dir', type=Path, required=True)
    parser.add_argument('--source', action='store_true', help='also copy the clean source tree')
    args = parser.parse_args()
    version = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
    name = f'p2-middleware-v{version}-windows-x64'
    out = root / 'dist' / 'release'
    archive = out / f'{name}.zip'
    if archive.exists():
        raise SystemExit(f'{archive} already exists; remove it first.')

    binaries = [args.build_dir / b for b in BINARIES]
    missing = [str(b) for b in binaries if not b.is_file()]
    if missing:
        raise SystemExit('Missing binaries: ' + ', '.join(missing))
    markers = personal_markers()
    for b in binaries:
        data = b.read_bytes()
        if any(m in data for m in markers):
            raise SystemExit(f'{b.name} contains a local user-profile path; rebuild with path remapping.')

    out.mkdir(parents=True, exist_ok=True)
    notices = sorted((root / 'crates/bridge-core/notices').iterdir())
    with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED) as z:
        for b in binaries:
            z.write(b, f'{name}/{b.name}')
        for d in DOCS:
            z.write(root / d, f'{name}/{d}')
        for n in notices:
            z.write(n, f'{name}/notices/{n.name}')
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (out / 'SHA256SUMS.txt').write_text(f'{digest}  {archive.name}\n', encoding='utf-8')
    print(f'{archive}\nsha256 {digest}')

    if args.source:
        tree = root / 'dist' / 'source' / 'p2-middleware'
        if tree.exists():
            raise SystemExit(f'{tree} already exists; remove it first.')
        files = [root / f for f in SOURCE_FILES]
        for d in SOURCE_DIRS:
            files += [p for p in (root / d).rglob('*') if p.is_file() and '__pycache__' not in p.parts]
        for p in files:
            target = tree / p.relative_to(root)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(p, target)
        print(f'{tree} ({len(files)} files)')


if __name__ == '__main__':
    main()
