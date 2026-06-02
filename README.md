# kernel_ci_center

A small, typed **CI orchestrator** (in Rust) for the Yucca Samsung GKI kernels
— `sm8550` (S23), `sm8650` (S24), `sm8750` (S25). It drives each device's own
`build.sh`, collects the packed AnyKernel3 zip, and publishes a GitHub Release.

It replaces a sprawl of shell scripts with one config-driven binary: devices,
modes, toolchains and upstream pins live in `config/devices.toml`; the logic
lives in `src/*.rs`.

> **Scope, honestly:** the orchestration layer is 100% Rust. It still *invokes*
> external build tools — `git`, `make`/`clang`, `patch`, `zip`, `gh` — because
> nobody reimplements the kernel build system or a C compiler in Rust. The
> per-kernel feature application + `make` lives in each kernel repo's
> `build.sh`; this repo decides *what/when/where* and publishes the result.

## Usage

```sh
# list devices and their status
cargo run -- list

# show the plan without building
cargo run -- build --device sm8750 --mode resukisu --dry-run

# build locally (uses the local toolchain path in devices.toml)
cargo run --release -- build --device sm8750 --mode resukisu

# build + publish a GitHub Release
cargo run --release -- build --device sm8750 --mode lkm --release
```

The CLI generates a random `abogki<number>` build id per run and passes it
through to the kernel `build.sh` (which bakes it into the version string
`6.6.138-android15-8-YuccaA-abogki<number>-4k`).

## Cloud builds (GitHub Actions)

`.github/workflows/build.yml` runs a **resukisu + lkm matrix** on
`workflow_dispatch`, builds with `kci`, attaches the zips as workflow
artifacts and (optionally) creates a Release.

Two things must be provided before cloud builds work:

1. **`secrets.KERNEL_REPO_TOKEN`** — a PAT (repo scope) so the runner can clone
   the *private* kernel repos. The workflow rewrites the kernel git URLs to use
   it. (`GITHUB_TOKEN` is automatic and only used for the Release.)
2. **`toolchain.url`** in `config/devices.toml` — a hosted Samsung clang
   prebuilts tarball. Cloud runners don't have the local toolchain path, so the
   resolver falls back to `url` (with optional `subdir`). Until a URL is set,
   cloud builds stop at the toolchain step (local builds are unaffected).

## Device status

| Device | SoC | Android | Source on GitHub | CI-ready |
|---|---|---|---|---|
| Galaxy S25 | sm8750 | android15-6.6 | ✅ `kernel_samsung_sm8750` (private) | local ✅ / cloud needs `toolchain.url` |
| Galaxy S23 | sm8550 | android13-5.15 | ❌ local only — push to enable | disabled in config |
| Galaxy S24 | sm8650 | android14-6.1 | ❌ not built yet | placeholder |

Flip `enabled = true` in `config/devices.toml` once a device's repo is on
GitHub and its toolchain is reachable.

## Layout

```
config/devices.toml      device table (repo, branch, build_script, modes, toolchain)
src/main.rs              clap CLI (list / build)
src/config.rs            serde config model
src/pipeline.rs          fetch source+toolchain+anykernel -> build -> collect -> sha256
src/release.rs           gh release create/upload
src/util.rs              process runner, sha256, zip discovery
.github/workflows/build.yml   resukisu+lkm matrix on dispatch
```

GPL-2.0.
