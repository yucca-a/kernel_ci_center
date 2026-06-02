use anyhow::{bail, Context, Result};
use rand::Rng;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Config, Device};
use crate::util::{list_zips, newest_zip, run, sha256_file};

pub struct Artifact {
    pub zip: PathBuf,
    pub sha256: String,
    pub build_num: u64,
}

pub fn run_build(
    cfg: &Config,
    device_id: &str,
    mode: &str,
    work: &str,
    do_release: bool,
    dry_run: bool,
) -> Result<()> {
    let dev = cfg
        .device(device_id)
        .with_context(|| format!("unknown device `{device_id}` (see `kci list`)"))?;
    if !dev.enabled {
        bail!("device `{device_id}` is disabled in config (its source is not on GitHub yet)");
    }
    if !dev.supports(mode) {
        bail!("device `{device_id}` does not support mode `{mode}` (modes: {:?})", dev.modes);
    }

    let build_num: u64 = rand::thread_rng().gen_range(100_000_000..1_000_000_000);
    println!(
        "== kci build :: {} ({}) :: mode={} :: build={} ==",
        dev.id, dev.name, mode, build_num
    );

    // Absolute workdir: build.sh runs with cwd = srcdir, so any TOOLCHAIN_DIR /
    // ANYKERNEL_DIR we hand it must be absolute, not relative to our cwd.
    let workdir = {
        let w = PathBuf::from(work);
        if w.is_absolute() {
            w
        } else {
            std::env::current_dir().context("current_dir")?.join(w)
        }
    };
    let srcdir = workdir.join(&dev.id);
    // build.sh writes the packed zip to the kernel-repo parent dir (= workdir).
    let artifacts = workdir.clone();

    if dry_run {
        println!("[dry-run] plan:");
        println!("  1. sync source : {} @ {} -> {}", dev.repo, dev.branch, srcdir.display());
        println!("  2. toolchain   : {}", describe_toolchain(dev));
        match &dev.anykernel_repo {
            Some(ak) => println!("  3. anykernel   : {} @ {}", ak, dev.anykernel_branch),
            None => println!("  3. anykernel   : (build script default)"),
        }
        println!(
            "  4. build       : bash {} {}  [env MODE,TOOLCHAIN_DIR,PACK=1,BUILD_NUM={}]",
            dev.build_script, mode, build_num
        );
        println!("  5. collect zip : newest *.zip in {}", artifacts.display());
        if do_release {
            println!("  6. release     : gh release create {}-{}-{}", dev.id, mode, build_num);
        }
        return Ok(());
    }

    fs::create_dir_all(&workdir).with_context(|| format!("mkdir {}", workdir.display()))?;

    // 1. source
    sync_repo(&dev.repo, &dev.branch, &srcdir)?;

    // 2. toolchain
    let toolchain_dir = ensure_toolchain(dev, &workdir)?;
    if !toolchain_dir.exists() {
        bail!("resolved toolchain dir does not exist: {}", toolchain_dir.display());
    }

    // 3. anykernel (optional)
    let anykernel_dir = match &dev.anykernel_repo {
        Some(ak) => {
            let akd = workdir.join(format!("{}-anykernel", dev.id));
            sync_repo(ak, &dev.anykernel_branch, &akd)?;
            Some(akd)
        }
        None => None,
    };

    // 4. build (kernel repo owns the feature application + make via its build_script)
    let bn = build_num.to_string();
    let tc = toolchain_dir.to_string_lossy().into_owned();
    let mut envs: Vec<(&str, &str)> = vec![
        ("MODE", mode),
        ("PACK", "1"),
        ("BUILD_NUM", bn.as_str()),
        ("TOOLCHAIN_DIR", tc.as_str()),
    ];
    let ak_s;
    if let Some(akd) = &anykernel_dir {
        ak_s = akd.to_string_lossy().into_owned();
        envs.push(("ANYKERNEL_DIR", ak_s.as_str()));
    }

    let before = list_zips(&artifacts);
    run("bash", &[dev.build_script.as_str(), mode], &srcdir, &envs)
        .with_context(|| format!("running build script for {}", dev.id))?;

    // 5. collect artifact
    let after = list_zips(&artifacts);
    let zip = after
        .into_iter()
        .find(|z| !before.contains(z))
        .or_else(|| newest_zip(&artifacts))
        .context("no output .zip found after build (is PACK honored by the build script?)")?;
    let sha = sha256_file(&zip)?;
    let art = Artifact { zip, sha256: sha, build_num };
    println!("== artifact :: {} ==", art.zip.display());
    println!("   sha256   :: {}", art.sha256);

    // 6. release
    if do_release {
        crate::release::upload(dev, mode, &art)?;
    }
    Ok(())
}

fn describe_toolchain(dev: &Device) -> String {
    if let Some(p) = &dev.toolchain.local_path {
        format!("local_path {p}")
    } else if let Some(u) = &dev.toolchain.url {
        format!("fetch {u} (subdir {:?})", dev.toolchain.subdir)
    } else {
        "<unconfigured>".to_string()
    }
}

fn sync_repo(repo: &str, branch: &str, dest: &Path) -> Result<()> {
    let here = Path::new(".");
    let dest_s = dest.to_string_lossy().into_owned();
    if dest.join(".git").exists() {
        run("git", &["-C", &dest_s, "fetch", "--depth", "1", "origin", branch], here, &[])?;
        run("git", &["-C", &dest_s, "checkout", "-f", branch], here, &[])
            .or_else(|_| run("git", &["-C", &dest_s, "checkout", "-f", "-B", branch, &format!("origin/{branch}")], here, &[]))?;
        run("git", &["-C", &dest_s, "reset", "--hard", &format!("origin/{branch}")], here, &[])?;
    } else {
        run("git", &["clone", "--depth", "1", "--branch", branch, "--single-branch", repo, &dest_s], here, &[])?;
    }
    Ok(())
}

fn ensure_toolchain(dev: &Device, workdir: &Path) -> Result<PathBuf> {
    if let Some(p) = &dev.toolchain.local_path {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Ok(pb);
        }
        eprintln!("note: local_path {p} not present (cloud runner?), falling back to url");
    }
    let url = dev
        .toolchain
        .url
        .as_ref()
        .context("toolchain local_path is absent and no url set; set toolchain.url in devices.toml for cloud builds")?;
    let here = Path::new(".");
    let root = workdir.join(format!("{}-toolchain", dev.id));
    if !root.exists() {
        fs::create_dir_all(&root)?;
        let tarball = workdir.join(format!("{}-toolchain.tar", dev.id));
        let tb = tarball.to_string_lossy().into_owned();
        run("curl", &["-fL", "--retry", "3", "-o", &tb, url], here, &[])?;
        let root_s = root.to_string_lossy().into_owned();
        run("tar", &["-xf", &tb, "-C", &root_s], here, &[])?;
        let _ = fs::remove_file(&tarball);
    }
    Ok(match &dev.toolchain.subdir {
        Some(s) => root.join(s),
        None => root,
    })
}
