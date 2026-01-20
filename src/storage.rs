use crate::model::RunResult;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Get the base directory for storing application data.
fn base_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cloudflare-speed-cli")
}

/// Get the directory for storing test run results.
fn runs_dir() -> PathBuf {
    base_dir().join("runs")
}

/// Ensure the necessary directories exist for storing data.
pub fn ensure_dirs() -> Result<()> {
    std::fs::create_dir_all(runs_dir()).context("create runs dir")?;
    Ok(())
}

pub fn save_run(result: &RunResult) -> Result<PathBuf> {
    ensure_dirs()?;
    let path = get_run_path(result)?;
    let data = serde_json::to_vec_pretty(result)?;
    std::fs::write(&path, data).context("write run json")?;
    Ok(path)
}

pub fn get_run_path(result: &RunResult) -> Result<PathBuf> {
    let ts = &result.timestamp_utc;
    let safe_ts = ts.replace(':', "-").replace('T', "_");
    Ok(runs_dir().join(format!("run-{safe_ts}-{}.json", result.meas_id)))
}

pub fn delete_run(result: &RunResult) -> Result<()> {
    let path = get_run_path(result)?;
    if path.exists() {
        std::fs::remove_file(&path).context("delete run file")?;
    }
    Ok(())
}

pub fn export_json(path: &Path, result: &RunResult) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create export directory")?;
    }
    let data = serde_json::to_vec_pretty(result)?;
    std::fs::write(path, data).context("write export json")?;
    Ok(())
}

pub fn export_csv(path: &Path, result: &RunResult) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create export directory")?;
    }
    let mut out = String::new();
    // Header row with all fields including diagnostics
    out.push_str("timestamp_utc,base_url,meas_id,comments,server,download_mbps,upload_mbps,idle_mean_ms,idle_median_ms,idle_p25_ms,idle_p75_ms,idle_loss,dl_loaded_mean_ms,dl_loaded_median_ms,dl_loaded_p25_ms,dl_loaded_p75_ms,dl_loaded_loss,ul_loaded_mean_ms,ul_loaded_median_ms,ul_loaded_p25_ms,ul_loaded_p75_ms,ul_loaded_loss,ip,colo,asn,as_org,interface_name,network_name,is_wireless,interface_mac,local_ipv4,local_ipv6,external_ipv4,external_ipv6,dns_resolution_ms,dns_ipv4_count,dns_ipv6_count,dns_servers,tls_handshake_ms,tls_protocol,tls_cipher,ipv4_download_mbps,ipv4_upload_mbps,ipv4_latency_ms,ipv6_download_mbps,ipv6_upload_mbps,ipv6_latency_ms,traceroute_hops\n");

    // Extract diagnostic values
    let dns_resolution_ms = result.dns.as_ref().map(|d| d.resolution_time_ms);
    let dns_ipv4_count = result.dns.as_ref().map(|d| d.ipv4_count);
    let dns_ipv6_count = result.dns.as_ref().map(|d| d.ipv6_count);
    let dns_servers = result
        .dns
        .as_ref()
        .map(|d| d.dns_servers.join("; "))
        .unwrap_or_default();
    let tls_handshake_ms = result.tls.as_ref().map(|t| t.handshake_time_ms);
    let tls_protocol = result.tls.as_ref().and_then(|t| t.protocol_version.clone());
    let tls_cipher = result.tls.as_ref().and_then(|t| t.cipher_suite.clone());

    // IPv4 results
    let ipv4_download = result
        .ip_comparison
        .as_ref()
        .and_then(|c| c.ipv4_result.as_ref())
        .filter(|r| r.available)
        .map(|r| r.download_mbps);
    let ipv4_upload = result
        .ip_comparison
        .as_ref()
        .and_then(|c| c.ipv4_result.as_ref())
        .filter(|r| r.available)
        .map(|r| r.upload_mbps);
    let ipv4_latency = result
        .ip_comparison
        .as_ref()
        .and_then(|c| c.ipv4_result.as_ref())
        .filter(|r| r.available)
        .map(|r| r.latency_ms);

    // IPv6 results
    let ipv6_download = result
        .ip_comparison
        .as_ref()
        .and_then(|c| c.ipv6_result.as_ref())
        .filter(|r| r.available)
        .map(|r| r.download_mbps);
    let ipv6_upload = result
        .ip_comparison
        .as_ref()
        .and_then(|c| c.ipv6_result.as_ref())
        .filter(|r| r.available)
        .map(|r| r.upload_mbps);
    let ipv6_latency = result
        .ip_comparison
        .as_ref()
        .and_then(|c| c.ipv6_result.as_ref())
        .filter(|r| r.available)
        .map(|r| r.latency_ms);

    // Traceroute hop count
    let traceroute_hops = result.traceroute.as_ref().map(|t| t.hops.len());

    out.push_str(&format!(
        "{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.6},{:.3},{:.3},{:.3},{:.3},{:.6},{:.3},{:.3},{:.3},{:.3},{:.6},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        csv_escape(&result.timestamp_utc),
        csv_escape(&result.base_url),
        csv_escape(&result.meas_id),
        csv_escape(result.comments.as_deref().unwrap_or("")),
        csv_escape(result.server.as_deref().unwrap_or("")),
        result.download.mbps,
        result.upload.mbps,
        result.idle_latency.mean_ms.unwrap_or(f64::NAN),
        result.idle_latency.median_ms.unwrap_or(f64::NAN),
        result.idle_latency.p25_ms.unwrap_or(f64::NAN),
        result.idle_latency.p75_ms.unwrap_or(f64::NAN),
        result.idle_latency.loss,
        result.loaded_latency_download.mean_ms.unwrap_or(f64::NAN),
        result.loaded_latency_download.median_ms.unwrap_or(f64::NAN),
        result.loaded_latency_download.p25_ms.unwrap_or(f64::NAN),
        result.loaded_latency_download.p75_ms.unwrap_or(f64::NAN),
        result.loaded_latency_download.loss,
        result.loaded_latency_upload.mean_ms.unwrap_or(f64::NAN),
        result.loaded_latency_upload.median_ms.unwrap_or(f64::NAN),
        result.loaded_latency_upload.p25_ms.unwrap_or(f64::NAN),
        result.loaded_latency_upload.p75_ms.unwrap_or(f64::NAN),
        result.loaded_latency_upload.loss,
        csv_escape(result.ip.as_deref().unwrap_or("")),
        csv_escape(result.colo.as_deref().unwrap_or("")),
        csv_escape(result.asn.as_deref().unwrap_or("")),
        csv_escape(result.as_org.as_deref().unwrap_or("")),
        csv_escape(result.interface_name.as_deref().unwrap_or("")),
        csv_escape(result.network_name.as_deref().unwrap_or("")),
        result.is_wireless.map(|w| if w { "true" } else { "false" }).unwrap_or(""),
        csv_escape(result.interface_mac.as_deref().unwrap_or("")),
        csv_escape(result.local_ipv4.as_deref().unwrap_or("")),
        csv_escape(result.local_ipv6.as_deref().unwrap_or("")),
        csv_escape(result.external_ipv4.as_deref().unwrap_or("")),
        csv_escape(result.external_ipv6.as_deref().unwrap_or("")),
        // Diagnostic fields
        dns_resolution_ms.map(|v| format!("{:.3}", v)).unwrap_or_default(),
        dns_ipv4_count.map(|v| v.to_string()).unwrap_or_default(),
        dns_ipv6_count.map(|v| v.to_string()).unwrap_or_default(),
        csv_escape(&dns_servers),
        tls_handshake_ms.map(|v| format!("{:.3}", v)).unwrap_or_default(),
        csv_escape(tls_protocol.as_deref().unwrap_or("")),
        csv_escape(tls_cipher.as_deref().unwrap_or("")),
        ipv4_download.map(|v| format!("{:.3}", v)).unwrap_or_default(),
        ipv4_upload.map(|v| format!("{:.3}", v)).unwrap_or_default(),
        ipv4_latency.map(|v| format!("{:.3}", v)).unwrap_or_default(),
        ipv6_download.map(|v| format!("{:.3}", v)).unwrap_or_default(),
        ipv6_upload.map(|v| format!("{:.3}", v)).unwrap_or_default(),
        ipv6_latency.map(|v| format!("{:.3}", v)).unwrap_or_default(),
        traceroute_hops.map(|v| v.to_string()).unwrap_or_default(),
    ));
    std::fs::write(path, out).context("write export csv")?;
    Ok(())
}

/// Escape a string for CSV format (handles commas, quotes, and newlines).
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn load_recent(limit: usize) -> Result<Vec<RunResult>> {
    ensure_dirs()?;
    let dir = runs_dir();
    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for e in std::fs::read_dir(&dir).context("read runs dir")? {
        let e = e?;
        let p = e.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let m = e.metadata()?;
        let mt = m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        entries.push((mt, p));
    }
    entries.sort_by_key(|(t, _)| *t);
    entries.reverse();

    let mut out = Vec::new();
    for (_, p) in entries.into_iter().take(limit) {
        let data = std::fs::read(&p).with_context(|| format!("read {}", p.display()))?;
        let r: RunResult =
            serde_json::from_slice(&data).with_context(|| format!("parse {}", p.display()))?;
        out.push(r);
    }
    Ok(out)
}
