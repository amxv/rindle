//! Safari history backend selection. The signed, sandboxed Swift process is an
//! opt-in experimental backend until its packaging and stable Developer ID
//! signing are finished. It owns exactly one security-scoped Safari folder.
//! The fallback is the existing Rust SQLite adapter (requiring FDA for launchd).
use crate::macos::safari;
use crate::state::OutboundUrl;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const HELPER_ENV: &str = "RINDLE_SCOPED_SAFARI_HELPER";
const MAX_HELPER_RESPONSE: usize = 8 * 1024 * 1024;

/// Shells and launchd have different environments. Honor the installed job's
/// helper when a user runs `rindle status` or `rindle sync` interactively.
/// An explicitly isolated RINDLE_HOME never consults the user's job.
fn configured_scoped_helper() -> Option<PathBuf> {
    if let Some(explicit) = env::var_os(HELPER_ENV) {
        return Some(PathBuf::from(explicit));
    }
    if env::var_os("RINDLE_HOME").is_some() {
        return None;
    }
    let plist =
        PathBuf::from(env::var_os("HOME")?).join("Library/LaunchAgents/xyz.rindle.sync.plist");
    if !plist.is_file() {
        return None;
    }
    let output = Command::new("plutil")
        .args([
            "-extract",
            "EnvironmentVariables.RINDLE_SCOPED_SAFARI_HELPER",
            "raw",
            "-o",
            "-",
        ])
        .arg(plist)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(PathBuf::from(value))
    }
}

pub enum Backend {
    Scoped(Scoped),
    Direct(safari::Safari),
}

pub struct Scoped {
    executable: PathBuf,
}

impl Scoped {
    pub fn new(executable: PathBuf) -> Result<Self> {
        if !executable.is_absolute() || !executable.is_file() {
            bail!("Scoped Safari helper must be an existing, absolute executable path")
        }
        let helper = Self { executable };
        let response = helper.send(json!({ "version": 1, "action": "checkpoint" }))?;
        if response
            .get("last_synced_generation")
            .and_then(Value::as_i64)
            .is_none()
        {
            bail!("Scoped Safari helper did not return a valid history checkpoint")
        }
        Ok(helper)
    }

    fn send(&self, request: Value) -> Result<Value> {
        let mut child = Command::new(&self.executable)
            .arg("--serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Unable to launch sandboxed Safari helper")?;
        {
            let input = child
                .stdin
                .take()
                .context("Missing Safari helper request pipe")?;
            let mut writer = std::io::BufWriter::new(input);
            serde_json::to_writer(&mut writer, &request)?;
            writer.flush()?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            // The Swift helper emits only errors, never browser URLs or paths.
            bail!(
                "Sandboxed Safari helper failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
        }
        if output.stdout.len() > MAX_HELPER_RESPONSE {
            bail!("Sandboxed Safari helper response too large")
        }
        let response: Value = serde_json::from_slice(&output.stdout)
            .context("Invalid JSON returned by sandboxed Safari helper")?;
        if response.get("ok") != Some(&Value::Bool(true)) {
            bail!("Sandboxed Safari helper rejected the request")
        }
        Ok(response)
    }

    fn inspect(&self) -> Result<Value> {
        self.send(json!({ "version": 1, "action": "inspect", "limit": 20000 }))
    }

    fn generations(&self) -> Result<(i64, i64)> {
        let value = self.send(json!({ "version": 1, "action": "checkpoint" }))?;
        Ok((
            value["current_generation"]
                .as_i64()
                .context("No current generation")?,
            value["last_synced_generation"]
                .as_i64()
                .context("No synced generation")?,
        ))
    }

    fn remote_urls(&self) -> Result<Vec<String>> {
        let response = self.inspect()?;
        let urls = response["remote_urls"]
            .as_array()
            .context("Scoped helper returned no remote URL list")?;
        let mut seen = std::collections::HashSet::new();
        Ok(urls
            .iter()
            .filter_map(Value::as_str)
            .filter_map(crate::urls::clean)
            .filter(|url| seen.insert(url.clone()))
            .collect())
    }

    fn add_urls(&mut self, urls: &[OutboundUrl]) -> Result<Vec<(String, Option<i64>)>> {
        if urls.is_empty() {
            return Ok(Vec::new());
        }
        let visits: Vec<_> = urls
            .iter()
            .filter_map(|item| {
                crate::urls::clean(&item.url).map(|url| {
                    json!({
                        "url": url, "title": item.title.chars().take(512).collect::<String>()
                    })
                })
            })
            .collect();
        let result = self.send(json!({ "version": 1, "action": "insert", "visits": visits }))?;
        let imports = result["imports"]
            .as_array()
            .context("Scoped helper returned no import results")?;
        if imports.len() != visits.len() {
            bail!("Scoped Safari helper returned inconsistent import count")
        }
        let mut output = Vec::new();
        for (returned, requested) in imports.iter().zip(visits.iter()) {
            let url = returned["url"]
                .as_str()
                .context("Missing imported Safari URL")?;
            if requested["url"].as_str() != Some(url) {
                bail!("Scoped Safari helper returned a different URL than requested")
            }
            output.push((url.to_owned(), returned["generation"].as_i64()));
        }
        Ok(output)
    }

    fn visit_generation(&self, url: &str) -> Result<Option<i64>> {
        let result = self.send(json!({"version":1,"action":"visit_generation","url":url}))?;
        Ok(result["generation"].as_i64())
    }
}

impl Backend {
    pub fn open() -> Result<Self> {
        if let Some(executable) = configured_scoped_helper() {
            return Ok(Self::Scoped(Scoped::new(executable)?));
        }
        Ok(Self::Direct(safari::Safari::open()?))
    }

    pub fn is_scoped(&self) -> bool {
        matches!(self, Self::Scoped(_))
    }

    pub fn generations(&self) -> Result<(i64, i64)> {
        match self {
            Self::Scoped(helper) => helper.generations(),
            Self::Direct(direct) => direct.generations(),
        }
    }

    pub fn remote_urls(&self) -> Result<Vec<String>> {
        match self {
            Self::Scoped(helper) => helper.remote_urls(),
            Self::Direct(direct) => direct.remote_urls(),
        }
    }

    pub fn add_urls(
        &mut self,
        urls: &[OutboundUrl],
        backups: &Path,
    ) -> Result<Vec<(String, Option<i64>)>> {
        match self {
            Self::Scoped(helper) => helper.add_urls(urls),
            Self::Direct(direct) => direct.add_urls(urls, backups),
        }
    }

    pub fn synced_generation_read_only() -> Result<i64> {
        if let Some(executable) = configured_scoped_helper() {
            return Ok(Scoped::new(executable)?.generations()?.1);
        }
        safari::Safari::synced_generation_read_only()
    }

    pub fn current_generation_read_only() -> Result<i64> {
        if let Some(executable) = configured_scoped_helper() {
            return Ok(Scoped::new(executable)?.generations()?.0);
        }
        safari::Safari::current_generation_read_only()
    }

    pub fn visit_generation_read_only(url: &str) -> Result<Option<i64>> {
        if let Some(executable) = configured_scoped_helper() {
            return Scoped::new(executable)?.visit_generation(url);
        }
        safari::Safari::visit_generation_read_only(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fake_scoped_helper(dir: &Path) -> Result<PathBuf> {
        let helper = dir.join("fake-scoped-helper.sh");
        fs::write(
            &helper,
            r#"#!/bin/sh
read -r request
case "$request" in
  *'"action":"checkpoint"'*) printf '{"ok":true,"current_generation":42,"last_synced_generation":41}\n' ;;
  *'"action":"inspect"'*) printf '{"ok":true,"current_generation":42,"last_synced_generation":41,"remote_urls":["https://example.com/from-phone","https://example.com/from-phone","file:///etc/passwd"]}\n' ;;
  *'"action":"visit_generation"'*) printf '{"ok":true,"generation":42}\n' ;;
  *'"action":"insert"'*) printf '{"ok":true,"imports":[{"url":"https://example.com/new","generation":43}]}\n' ;;
  *) exit 8 ;;
esac
"#,
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&helper, fs::Permissions::from_mode(0o700))?;
        }
        Ok(helper)
    }

    #[test]
    fn reject_missing_scoped_executable() {
        assert!(Scoped::new(PathBuf::from("/this-helper-does-not-exist")).is_err());
        assert!(Scoped::new(PathBuf::from("relative-helper")).is_err());
    }

    #[test]
    fn scoped_helper_protocol_matches_safari_backend_and_filters_urls() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut scoped = Scoped::new(fake_scoped_helper(dir.path())?)?;
        assert_eq!(scoped.generations()?, (42, 41));
        assert_eq!(
            scoped.remote_urls()?,
            vec!["https://example.com/from-phone"]
        );
        assert_eq!(
            scoped.visit_generation("https://example.com/from-phone")?,
            Some(42)
        );
        assert_eq!(
            scoped.add_urls(&[OutboundUrl {
                url: "https://example.com/new".into(),
                title: "New".into(),
            }])?,
            [("https://example.com/new".to_owned(), Some(43))]
        );
        Ok(())
    }
}
