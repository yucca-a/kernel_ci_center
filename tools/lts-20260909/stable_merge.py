"""Three-way merge stable file deltas against each pristine Samsung/ACK snapshot."""
import concurrent.futures
import pathlib
import subprocess
import sys
import tarfile
import urllib.request

ROOT = pathlib.Path('/home/yucca/work/kernel-update-20260909')
VERSIONS = {'8550': ('5.15.207', '5.15.220'), '8650': ('6.1.172', '6.1.187'), '8750': ('6.6.138', '6.6.156')}

def fetch(version):
    dest = ROOT / 'stable' / f'linux-{version}'
    archive = ROOT / 'downloads' / f'linux-{version}.tar.xz'
    if dest.exists():
        return str(dest)
    archive.parent.mkdir(exist_ok=True)
    url = f'https://cdn.kernel.org/pub/linux/kernel/v{version.split(".")[0]}.x/linux-{version}.tar.xz'
    urllib.request.urlretrieve(url, archive)
    with tarfile.open(archive) as tar:
        tar.extractall(ROOT / 'stable', filter='data')
    return str(dest)

def merge(soc):
    old, new = VERSIONS[soc]
    tree = ROOT / f'sm{soc}'
    base, upstream = [ROOT / 'stable' / f'linux-{v}' for v in (old, new)]
    changed = []
    conflicts = []
    files = {p.relative_to(base) for p in base.rglob('*') if p.is_file()} | {p.relative_to(upstream) for p in upstream.rglob('*') if p.is_file()}
    for rel in sorted(files):
        b, u, dst = base / rel, upstream / rel, tree / rel
        oldbytes = b.read_bytes() if b.is_file() else None
        newbytes = u.read_bytes() if u.is_file() else None
        if oldbytes == newbytes:
            continue
        changed.append(str(rel))
        ours = dst.read_bytes() if dst.is_file() else None
        if ours == newbytes:
            continue
        if ours == oldbytes:
            if newbytes is None:
                dst.unlink()
            else:
                dst.parent.mkdir(parents=True, exist_ok=True)
                dst.write_bytes(newbytes)
                dst.chmod(u.stat().st_mode)
        elif ours is not None and oldbytes is not None and newbytes is not None:
            result = subprocess.run(['git', 'merge-file', '--diff3', '-L', f'Samsung-{soc}', '-L', old, '-L', new, str(dst), str(b), str(u)], capture_output=True, text=True)
            if result.returncode:
                conflicts.append(str(rel))
        else:
            conflicts.append(str(rel) + ' [add/delete conflict]')
    (ROOT / f'sm{soc}-stable-files.txt').write_text('\n'.join(changed) + '\n')
    (ROOT / f'sm{soc}-conflicts.txt').write_text('\n'.join(conflicts) + '\n')
    print(f'sm{soc}: {old} -> {new}: {len(changed)} upstream files, {len(conflicts)} conflicts', flush=True)
    print('\n'.join(conflicts), flush=True)

if sys.argv[1] == 'cleanup-increments':
    for soc in VERSIONS:
        tree = ROOT / f'sm{soc}'
        subprocess.run(['git', 'restore', '--worktree', '.'], cwd=tree, check=True)
        for path in tree.rglob('*.rej'):
            path.unlink()
elif sys.argv[1] == 'download':
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        for result in pool.map(fetch, [v for pair in VERSIONS.values() for v in pair]):
            print(result, flush=True)
else:
    merge(sys.argv[1])

