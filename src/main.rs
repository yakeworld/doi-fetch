use anyhow::Result;
use clap::Parser;
use regex::Regex;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

// ====== CLI ======

#[derive(Parser)]
#[command(name = "doi-fetch", version = "0.1.0", about = "Multi-source DOI full-text PDF downloader",
    long_about = "Cascade: bban.top CDN → Sci-Hub → LibGen → Anna's Archive")]
struct Cli {
    /// DOI to download (e.g. 10.1016/j.jcrs.2019.04.024)
    doi: String,

    /// Output PDF path
    #[arg(short, long)]
    output: Option<String>,

    /// Direct download without rproxy proxy rotation
    #[arg(long)]
    no_proxy: bool,

    /// Proxy pool file for rproxy
    #[arg(long, default_value = "/tmp/proxy_pool.txt")]
    proxy_pool: String,

    /// Per-phase timeout in seconds
    #[arg(long, default_value = "120")]
    timeout: u64,
}

// ====== HTTP helper ======

// Phase function type: (doi, output, &dyn Fn(&str) -> Result<HttpResp>) -> Result<bool>
type PhaseFn = fn(&str, &str, &dyn Fn(&str) -> Result<HttpResp>) -> Result<bool>;

fn make_http(timeout_secs: u64, proxy_pool: Option<&str>) -> impl Fn(&str) -> Result<HttpResp> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0")
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .build()
        .expect("HTTP client");

    let pool = proxy_pool
        .filter(|p| Path::new(p).exists())
        .map(|s| s.to_string());

    move |url: &str| {
        if let Some(ref pool) = pool {
            // Use rproxy exec for proxy rotation
            let code = format!(
                "import sys; from curl_cffi import requests as cr; \
                 r=cr.get({url:?}, impersonate='chrome120', timeout={timeout}); \
                 sys.stdout.buffer.write(r.content)",
                url = url,
                timeout = timeout_secs.min(30)
            );
            match Command::new("rproxy")
                .args(["exec", "-i", pool, "-r", "6", "--"])
                .arg("python3")
                .args(["-c", &code])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    let mut buf = Vec::new();
                    child.stdout.take().unwrap().read_to_end(&mut buf).ok();
                    Ok(HttpResp { content: buf, url: url.to_string() })
                }
                Err(_) => {
                    // Fallback: direct request (rproxy not available)
                    let resp = client.get(url).send().ok();
                    let content = resp.and_then(|r| r.bytes().ok()).map(|b| b.to_vec()).unwrap_or_default();
                    Ok(HttpResp { content, url: url.to_string() })
                }
            }
        } else {
            let resp = client.get(url).send().ok();
            let content = resp.and_then(|r| r.bytes().ok()).map(|b| b.to_vec()).unwrap_or_default();
            Ok(HttpResp { content, url: url.to_string() })
        }
    }
}

struct HttpResp {
    content: Vec<u8>,
    url: String,
}

fn is_pdf(content: &[u8]) -> bool {
    content.len() > 5 && content[..5] == *b"%PDF-"
}

// ====== Phases ======

/// Phase 1: bban.top CDN direct download
fn phase_cdn(doi: &str, output: &str, get: &dyn Fn(&str) -> Result<HttpResp>) -> Result<bool> {
    let url = format!("https://sci.bban.top/pdf/{}.pdf", doi.to_lowercase());
    let resp = get(&url)?;
    if is_pdf(&resp.content) {
        fs::write(output, &resp.content)?;
        println!("  ✅ {} bytes", resp.content.len());
        return Ok(true);
    }
    println!("  not PDF ({} bytes)", resp.content.len());
    Ok(false)
}

/// Phase 2: Sci-Hub frontend (sci-hub.vg iframe extraction)
fn phase_frontend(doi: &str, output: &str, get: &dyn Fn(&str) -> Result<HttpResp>) -> Result<bool> {
    let url = format!("https://sci-hub.vg/{}", doi);
    let resp = get(&url)?;
    let html = String::from_utf8_lossy(&resp.content);
    let re = Regex::new(r#"src="([^"]*\.pdf[^"]*)"#)?;
    if let Some(caps) = re.captures(&html) {
        let mut pdf_url = caps[1].to_string();
        if pdf_url.starts_with("//") {
            pdf_url = format!("https:{}", pdf_url);
        }
        pdf_url = pdf_url.split('#').next().unwrap_or(&pdf_url).to_string();
        let pdf_resp = get(&pdf_url)?;
        if is_pdf(&pdf_resp.content) {
            fs::write(output, &pdf_resp.content)?;
            println!("  ✅ {} bytes", pdf_resp.content.len());
            return Ok(true);
        }
        println!("  not PDF");
    } else {
        println!("  NO_IFRAME");
    }
    Ok(false)
}

/// Phase 3: LibGen search + download
fn phase_libgen(doi: &str, output: &str, get: &dyn Fn(&str) -> Result<HttpResp>) -> Result<bool> {
    let encoded = doi.replace('/', "%2F");
    let search_url = format!(
        "https://libgen.li/index.php?req={}&columns%5B%5D=d", encoded
    );
    let resp = get(&search_url)?;
    let html = String::from_utf8_lossy(&resp.content);
    let re_id = Regex::new(r"edition\.php\?id=(\d+)")?;
    if let Some(caps) = re_id.captures(&html) {
        let ed_url = format!("https://libgen.li/edition.php?id={}", &caps[1]);
        let ed_resp = get(&ed_url)?;
        let ed_html = String::from_utf8_lossy(&ed_resp.content);
        let re_href = Regex::new(r#"href="([^"]*md5=[0-9a-f]{32}[^"]*)"#)?;

        for href_cap in re_href.captures_iter(&ed_html) {
            let dl_url = if href_cap[1].starts_with('/') {
                format!("https://libgen.li{}", &href_cap[1])
            } else {
                href_cap[1].to_string()
            };
            let dl = get(&dl_url)?;
            if is_pdf(&dl.content) {
                fs::write(output, &dl.content)?;
                println!("  ✅ {} bytes", dl.content.len());
                return Ok(true);
            }
        }

        // Try alternate MD5 mirrors
        let re_md5 = Regex::new(r"md5=([0-9a-f]{32})")?;
        if let Some(md5_cap) = re_md5.captures(&ed_url) {
            let md5 = &md5_cap[1];
            for base in &["https://libgen.li/main/", "https://library.lol/main/"] {
                let mirror_url = format!("{}{}", base, md5);
                if let Ok(m) = get(&mirror_url) {
                    if is_pdf(&m.content) {
                        fs::write(output, &m.content)?;
                        println!("  ✅ {} bytes (mirror)", m.content.len());
                        return Ok(true);
                    }
                }
            }
        }
        println!("  NO_DL");
    } else {
        println!("  NO_RESULT");
    }
    Ok(false)
}

/// Phase 4: Anna's Archive scidb (MD5 discovery only)
fn phase_annas(doi: &str, _output: &str, get: &dyn Fn(&str) -> Result<HttpResp>) -> Result<bool> {
    let domains = ["annas-archive.gd", "annas-archive.gl"];
    let re = Regex::new(r"md5/([0-9a-f]{32})")?;
    for dom in &domains {
        let url = format!("https://{}/scidb/{}/", dom, doi);
        if let Ok(resp) = get(&url) {
            let html = String::from_utf8_lossy(&resp.content);
            if let Some(caps) = re.captures(&html) {
                println!("  🔍 MD5={} (use LibGen to download)", &caps[1]);
                return Ok(false);
            }
        }
    }
    println!("  NONE");
    Ok(false)
}

// ====== Main ======

fn main() -> Result<()> {
    let cli = Cli::parse();
    let doi = &cli.doi;
    let output = cli.output.unwrap_or_else(|| doi.replace('/', "_") + ".pdf");

    let proxy_pool = if cli.no_proxy { None } else { Some(cli.proxy_pool.as_str()) };
    let get = make_http(cli.timeout, proxy_pool);

    println!("📄 {}", doi);
    println!("   → {}", output);

    // Skip if PDF already exists
    if Path::new(&output).exists() {
        let data = fs::read(&output).unwrap_or_default();
        if is_pdf(&data) {
            println!("   ✅ 已有: {} bytes", data.len());
            return Ok(());
        }
    }

    let phases: [(&str, fn(&str, &str, &dyn Fn(&str) -> Result<HttpResp>) -> Result<bool>); 4] = [
        ("bban.top CDN", phase_cdn),
        ("Sci-Hub frontend", phase_frontend),
        ("LibGen", phase_libgen),
        ("Anna's Archive", phase_annas),
    ];

    for (name, phase) in &phases {
        print!("   [{}]", name);
        std::io::Write::flush(&mut std::io::stdout())?;
        if phase(doi, &output, &get)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    println!("   ❌ 全部失败");
    Ok(())
}
