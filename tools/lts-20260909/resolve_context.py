"""Resolve only mechanically applicable/already-present rejected stable hunks."""
import pathlib
import re
import subprocess
import sys

tree = pathlib.Path('/home/yucca/work/kernel-update-20260909') / ('sm' + sys.argv[1])
for reject in sorted(tree.rglob('*.rej')):
    text = reject.read_text()
    chunks = re.split(r'(?=^@@ )', text, flags=re.M)
    header = chunks[0]
    remaining = []
    for hunk in chunks[1:]:
        patch = header + hunk
        args = ['patch', '-p0', '--batch', '--fuzz=2', '--no-backup-if-mismatch']
        test = subprocess.run(args + ['--forward', '--dry-run'], cwd=tree, input=patch, text=True, capture_output=True)
        if test.returncode == 0:
            applied = subprocess.run(args + ['--forward'], cwd=tree, input=patch, text=True, capture_output=True)
            if applied.returncode:
                raise RuntimeError(applied.stdout + applied.stderr)
            print(f'{reject.relative_to(tree)} {hunk.splitlines()[0]}: context-only', flush=True)
        else:
            reverse = subprocess.run(args + ['--reverse', '--dry-run'], cwd=tree, input=patch, text=True, capture_output=True)
            if reverse.returncode == 0:
                print(f'{reject.relative_to(tree)} {hunk.splitlines()[0]}: already present', flush=True)
            else:
                remaining.append(hunk)
    if remaining:
        reject.write_text(header + ''.join(remaining))
        print(f'REMAINING {reject.relative_to(tree)}: {len(remaining)}', flush=True)
    else:
        reject.unlink()

