import json
import difflib
import pathlib
import re
import sys

root = pathlib.Path('/home/yucca/work/kernel-update-20260909')
soc = sys.argv[1]
files = (root / f'sm{soc}-conflicts.txt').read_text().splitlines()
start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
end = int(sys.argv[3]) if len(sys.argv) > 3 else len(files)
pattern = re.compile(r'^<<<<<<<[^\n]*\n(.*?)^\|\|\|\|\|\|\|[^\n]*\n(.*?)^=======\n(.*?)^>>>>>>>[^\n]*\n', re.M | re.S)
for index, file in enumerate(files[start:end], start):
    print(f'FILE {index}: {file}')
    path = root / f'sm{soc}' / file.split(' [')[0]
    if not path.is_file():
        print('not present')
        continue
    for block, match in enumerate(pattern.finditer(path.read_text())):
        if len(match[0].splitlines()) > 160:
            ours = match[1].splitlines(keepends=True)
            print(f'BLOCK {block} (large) OURS ({len(ours)} lines):\n' + ''.join(ours if len(ours) < 65 else ours[:30] + ['...\n'] + ours[-30:]))
            print('UPSTREAM CHANGE:\n' + ''.join(difflib.unified_diff(match[2].splitlines(keepends=True), match[3].splitlines(keepends=True), n=3)))
        else:
            print(f'BLOCK {block}\nOURS:\n{match[1]}BASE:\n{match[2]}UPSTREAM:\n{match[3]}')

