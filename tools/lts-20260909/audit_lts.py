"""Read-only audit of imported stable deltas; reports, never edits kernel trees."""
import collections
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path('/home/yucca/work/kernel-update-20260909')
SERIES = {
    '8750': ('6.6.138', '6.6.156', 'cb0d022c9', '6d73e0995'),
    '8650': ('6.1.172', '6.1.187', 'd74232e2b', 'd972275aa'),
    '8550': ('5.15.207', '5.15.220', '971393298', None),
}

def blobs(repo, rev, paths):
    result = subprocess.run(['git', 'cat-file', '--batch'], cwd=repo,
                            input=''.join(f'{rev}:{p}\n' for p in paths).encode(),
                            stdout=subprocess.PIPE, check=True).stdout
    out, offset = {}, 0
    for path in paths:
        end = result.index(b'\n', offset)
        header = result[offset:end]
        offset = end + 1
        if header.endswith(b' missing'):
            out[path] = None
        else:
            size = int(header.split()[-1])
            out[path] = result[offset:offset + size]
            offset += size + 1
    return out

for soc in sys.argv[1:] or ['8750', '8650', '8550']:
    old, new, baseline, imported = SERIES[soc]
    repo = ROOT / f'sm{soc}'
    paths = (ROOT / f'sm{soc}-stable-files.txt').read_text().splitlines()
    before = blobs(repo, baseline, paths)
    now = blobs(repo, 'HEAD', paths)
    at_import = blobs(repo, imported or 'HEAD', paths)
    counts = collections.Counter()
    exceptions = []
    with tempfile.TemporaryDirectory() as tmp:
        temps = [pathlib.Path(tmp) / name for name in ('ours', 'base', 'new')]
        for path in paths:
            bp = ROOT / 'stable' / f'linux-{old}' / path
            up = ROOT / 'stable' / f'linux-{new}' / path
            b = bp.read_bytes() if bp.is_file() else None
            u = up.read_bytes() if up.is_file() else None
            o = before[path]
            if now[path] == o and o != u:
                print('UNCHANGED_FROM_BASELINE', soc, path, flush=True)
            if now[path] == u:
                counts['HEAD equals upstream file'] += 1
                continue
            if o == u:
                counts['already upstream at baseline, later customized'] += 1
                continue
            if o == b:
                expected = u
            elif None not in (o, b, u):
                for temp, data in zip(temps, (o, b, u)):
                    temp.write_bytes(data)
                merge = subprocess.run(['git', 'merge-file', '-p', *map(str, temps)],
                                       stdout=subprocess.PIPE)
                if merge.returncode:
                    counts['requires manual conflict review'] += 1
                    exceptions.append(('CONFLICT', path))
                    continue
                expected = merge.stdout
            else:
                counts['requires add/delete review'] += 1
                exceptions.append(('ADD_DELETE', path))
                continue
            if now[path] == expected:
                counts['HEAD equals clean three-way result'] += 1
            elif at_import[path] == expected:
                counts['cleanly imported then customized'] += 1
                exceptions.append(('POST_IMPORT', path))
            else:
                counts['different from clean three-way result'] += 1
                exceptions.append(('DIFFERENT', path))
    print(f'SM{soc}: {old} -> {new}; {len(paths)} changed upstream paths')
    print(dict(counts))
    for category, path in exceptions:
        print(category, path)

