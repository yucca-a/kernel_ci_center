use anyhow::{Context, Result};
use std::path::Path;

use crate::config::Device;
use crate::pipeline::Artifact;

/// Publish the built zip as a GitHub Release on the device's OWN kernel repo
/// (so artifacts land next to their source and stay private), using the `gh`
/// CLI. Requires GH_TOKEN with write access to that repo (set RELEASE_TOKEN in
/// the workflow). Idempotent: creates the release, or uploads/clobbers the
/// asset if the tag already exists.
pub fn upload(dev: &Device, mode: &str, art: &Artifact) -> Result<()> {
    let slug = repo_slug(&dev.repo)
        .with_context(|| format!("cannot derive owner/repo from `{}`", dev.repo))?;
    let tag = format!("{}-{}-{}", dev.id, mode, art.build_num);
    let title = format!("{} — {} (build {})", dev.name, mode, art.build_num);
    let notes = format!(
        "Device: {} ({})\nMode: {}\nBuild number: {}\nsha256: {}",
        dev.name, dev.id, mode, art.build_num, art.sha256
    );
    let zip = art.zip.to_string_lossy().into_owned();
    let here = Path::new(".");

    println!("== releasing to {slug} :: tag {tag} ==");
    let created = crate::util::run(
        "gh",
        &["release", "create", &tag, &zip, "-R", &slug, "-t", &title, "-n", &notes],
        here,
        &[],
    );
    if created.is_err() {
        crate::util::run("gh", &["release", "upload", &tag, &zip, "-R", &slug, "--clobber"], here, &[])?;
    }
    println!("== released :: {slug} {tag} ==");
    Ok(())
}

/// `git@github.com:OWNER/NAME.git` or `https://github.com/OWNER/NAME.git` -> `OWNER/NAME`
fn repo_slug(url: &str) -> Option<String> {
    let s = url.trim().trim_end_matches(".git");
    let s = s.rsplit("github.com").next()?;
    let s = s.trim_start_matches([':', '/']);
    if s.split('/').filter(|p| !p.is_empty()).count() == 2 {
        Some(s.to_string())
    } else {
        None
    }
}
