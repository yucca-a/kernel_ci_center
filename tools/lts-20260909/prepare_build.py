import pathlib
import shutil
import subprocess
import sys

root = pathlib.Path('/home/yucca/work/kernel-update-20260909')
soc, mode = sys.argv[1:3]
src = root / ('sm' + soc)
dst = root / ('build-sm' + soc + '-' + mode)
subprocess.run(['git', '-C', str(src), 'worktree', 'add', '--detach', str(dst), 'HEAD'], check=True)
diff = subprocess.check_output(['git', '-C', str(src), 'diff', '--binary', 'HEAD'])
subprocess.run(['git', '-C', str(dst), 'apply', '--whitespace=nowarn'], input=diff, check=True)
others = subprocess.check_output(['git', '-C', str(src), 'ls-files', '--others', '--exclude-standard', '-z']).decode().split('\0')
for name in filter(None, others):
    target = dst / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src / name, target)
subprocess.run(['git', 'clone', '--quiet', '--shared', str(root / 'sources/resukisu'), str(dst / 'KernelSU')], check=True)
cache = dst / ('.features_cache' if soc == '8550' else '.build_cache')
cache.mkdir(exist_ok=True)
for source, target in [('susfs' + {'8550':'13','8650':'14','8750':'15'}[soc], 'susfs4ksu'), ('super-builders', 'super_builders')]:
    subprocess.run(['git', 'clone', '--quiet', '--shared', str(root / 'sources' / source), str(cache / target)], check=True)
print(dst)

