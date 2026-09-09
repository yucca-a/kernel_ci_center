"""Download and apply official stable increments in isolated kernel worktrees."""
import concurrent.futures
import lzma
import pathlib
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path('/home/yucca/work/kernel-update-20260909')
SERIES = {'8550': ('5.15', 207, 220), '8650': ('6.1', 172, 187), '8750': ('6.6', 138, 156)}

def download(item):
    series, prev, nxt = item
    name = f'patch-{series}.{prev}-{nxt}'
    dest = ROOT / 'patches' / name
    if not dest.exists():
        url = f'https://cdn.kernel.org/pub/linux/kernel/v{series.split(".")[0]}.x/incr/{name}.xz'
        with urllib.request.urlopen(url, timeout=90) as response:
            data = lzma.decompress(response.read())
        dest.write_bytes(data)
    return str(dest)

def main():
    ROOT.mkdir(exist_ok=True)
    (ROOT / 'patches').mkdir(exist_ok=True)
    if sys.argv[1] == 'download':
        jobs = [(series, n, n + 1) for series, start, end in SERIES.values() for n in range(start, end)]
        with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
            for result in executor.map(download, jobs):
                print(result, flush=True)
        return
    soc = sys.argv[1]
    series, start, end = SERIES[soc]
    tree = ROOT / f'sm{soc}'
    if len(sys.argv) > 2:
        start = int(sys.argv[2])
    for n in range(start, end):
        patch = ROOT / 'patches' / f'patch-{series}.{n}-{n + 1}'
        result = subprocess.run(['patch', '-p1', '--batch', '--forward', '--fuzz=0', '--no-backup-if-mismatch', '-i', str(patch)], cwd=tree, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (ROOT / f'sm{soc}-{n + 1}.log').write_text(result.stdout)
        print(f'sm{soc} {series}.{n + 1}: exit={result.returncode}', flush=True)
        if result.returncode:
            print('\n'.join(line for line in result.stdout.splitlines() if 'FAIL' in line or 'reject' in line or 'Reversed' in line), flush=True)
            sys.exit(result.returncode)

if __name__ == '__main__':
    main()

