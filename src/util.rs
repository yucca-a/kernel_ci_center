use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// Run a command, streaming its stdio to ours (so CI logs stay live).
pub fn run(program: &str, args: &[&str], cwd: &Path, envs: &[(&str, &str)]) -> Result<()> {
    eprintln!("+ {program} {}", args.join(" "));
    let mut c = Command::new(program);
    c.args(args).current_dir(cwd);
    for (k, v) in envs {
        c.env(k, v);
    }
    let status = c
        .status()
        .with_context(|| format!("spawning `{program}`"))?;
    if !status.success() {
        bail!("`{program} {}` failed ({status})", args.join(" "));
    }
    Ok(())
}

pub fn sha256_file(p: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(p).with_context(|| format!("reading {}", p.display()))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    let digest = h.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// All *.zip files directly under `dir` (non-recursive).
pub fn list_zips(dir: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "zip").unwrap_or(false) {
                v.push(p);
            }
        }
    }
    v
}

/// Newest *.zip under `dir` by mtime, if any.
pub fn newest_zip(dir: &Path) -> Option<PathBuf> {
    list_zips(dir)
        .into_iter()
        .max_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
}
