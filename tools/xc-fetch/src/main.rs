use chrono::Utc;
use clap::Parser;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "xc-fetch", about = "Fetch recording metadata from xeno-canto API v3")]
struct Args {
    /// Xeno-canto catalogue number or recording (e.g. 928094, XC928094, or https://xeno-canto.org/928094)
    recording: String,

    /// Fetch metadata only (skip audio download)
    #[arg(long)]
    metadata_only: bool,

    /// Output directory for sounds (default: ../../sounds relative to this tool)
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Path to index.json to update (default: ../../index.json relative to this tool)
    #[arg(long)]
    index: Option<PathBuf>,

    /// Skip updating index.json
    #[arg(long)]
    no_index: bool,

    /// API key (overrides XC_API_KEY env var)
    #[arg(long)]
    key: Option<String>,
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect()
}

/// Parse an XC number from various formats:
/// "928094", "XC928094", "xc928094", "https://xeno-canto.org/928094", "https://www.xeno-canto.org/928094"
fn parse_xc_number(input: &str) -> Result<u64, String> {
    let s = input.trim();

    // Try plain number first
    if let Ok(n) = s.parse::<u64>() {
        return Ok(n);
    }

    // Strip "XC" or "xc" prefix
    if let Some(rest) = s.strip_prefix("XC").or_else(|| s.strip_prefix("xc")) {
        return rest.parse::<u64>().map_err(|_| format!("Invalid XC number: {s}"));
    }

    // Try URL: extract trailing number from path
    if s.starts_with("http://xeno-canto.org/") || s.starts_with("https://xeno-canto.org/") {
        if let Some(last) = s.trim_end_matches('/').rsplit('/').next() {
            if let Ok(n) = last.parse::<u64>() {
                return Ok(n);
            }
        }
    }

    Err(format!("Can't parse XC number from: {s}"))
}

/// Resolve default paths relative to the tool's own location (tools/xc-fetch/ -> repo root)
fn repo_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    // exe is in tools/xc-fetch/target/debug/ or tools/xc-fetch/target/release/
    // Walk up to find index.json
    let mut dir = exe.parent()?;
    for _ in 0..6 {
        if dir.join("index.json").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

fn update_index(index_path: &PathBuf, xc_id: u64, en: &str, genus: &str, sp: &str, audio_filename: &str, meta_filename: &str) {
    let mut index: Value = if index_path.exists() {
        let content = fs::read_to_string(index_path).expect("Failed to read index.json");
        serde_json::from_str(&content).expect("Failed to parse index.json")
    } else {
        json!({ "version": 1, "sounds": [] })
    };

    let sounds = index["sounds"].as_array_mut().expect("index.json 'sounds' is not an array");

    // Check if this XC ID already exists
    if sounds.iter().any(|s| s["xc_id"].as_u64() == Some(xc_id)) {
        eprintln!("XC{} already in index.json, skipping update", xc_id);
        return;
    }

    sounds.push(json!({
        "filename": audio_filename,
        "metadata": meta_filename,
        "xc_id": xc_id,
        "en": en,
        "species": format!("{} {}", genus, sp),
        "source": "xeno-canto"
    }));

    let json_str = serde_json::to_string_pretty(&index).expect("Failed to serialize index.json");
    fs::write(index_path, format!("{}\n", json_str)).expect("Failed to write index.json");
    eprintln!("Updated {}", index_path.display());
}

fn main() {
    // Load .env from current dir or any parent (walks up to repo root)
    let _ = dotenvy::dotenv();

    let args = Args::parse();

    let xc_number = parse_xc_number(&args.recording)
        .unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1); });

    let api_key = args
        .key
        .or_else(|| std::env::var("XC_API_KEY").ok())
        .expect("API key required: pass --key, set XC_API_KEY env var, or add it to .env");

    let url = format!(
        "https://xeno-canto.org/api/3/recordings?query=nr:{}&key={}",
        xc_number, api_key
    );

    eprintln!("Fetching XC{}...", xc_number);

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .send()
        .expect("Failed to send request");

    if !resp.status().is_success() {
        eprintln!("HTTP error: {}", resp.status());
        let body = resp.text().unwrap_or_default();
        eprintln!("{body}");
        std::process::exit(1);
    }

    let body: Value = resp.json().expect("Failed to parse JSON response");

    if let Some(err) = body.get("error") {
        eprintln!("API error: {}", err);
        std::process::exit(1);
    }

    let recordings = body["recordings"]
        .as_array()
        .expect("Expected 'recordings' array in response");

    if recordings.is_empty() {
        eprintln!("No recordings found for XC{}", xc_number);
        std::process::exit(1);
    }

    let rec = &recordings[0];

    let id = rec["id"].as_str().unwrap_or("");
    let genus = rec["gen"].as_str().unwrap_or("");
    let sp = rec["sp"].as_str().unwrap_or("");
    let en = rec["en"].as_str().unwrap_or("");
    let recordist = rec["rec"].as_str().unwrap_or("");
    let lic = rec["lic"].as_str().unwrap_or("");

    let base_name = sanitize_filename(&format!("XC{} - {} - {} {}", id, en, genus, sp));

    // Determine extension from file-name field or default to .wav
    let ext = rec["file-name"]
        .as_str()
        .and_then(|name| name.rsplit('.').next())
        .unwrap_or("wav");

    let audio_filename = format!("{}.{}", base_name, ext);
    let meta_filename = format!("{}.xc.json", base_name);

    let attribution = format!(
        "{}, XC{}. Accessible at www.xeno-canto.org/{}",
        recordist, id, id
    );

    let metadata = json!({
        "source": "xeno-canto",
        "xc_id": rec["id"].as_str().and_then(|s| s.parse::<u64>().ok()).unwrap_or(xc_number),
        "url": format!("https://www.xeno-canto.org/{}", id),
        "file_url": rec["file"],
        "gen": genus,
        "sp": sp,
        "en": en,
        "rec": recordist,
        "cnt": rec["cnt"],
        "loc": rec["loc"],
        "lat": rec["lat"],
        "lon": rec["lon"],
        "date": rec["date"],
        "time": rec["time"],
        "type": rec["type"],
        "q": rec["q"],
        "length": rec["length"],
        "smp": rec["smp"].as_str().and_then(|s| s.parse::<u64>().ok()),
        "lic": lic,
        "attribution": attribution,
        "retrieved": Utc::now().format("%Y-%m-%d").to_string(),
        "raw_response": rec,
    });

    // Resolve output directory
    let output_dir = args.output_dir.unwrap_or_else(|| {
        repo_root()
            .map(|r| r.join("sounds"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Write metadata JSON
    let json_path = output_dir.join(&meta_filename);
    let json_bytes = serde_json::to_string_pretty(&metadata).expect("Failed to serialize JSON");
    fs::write(&json_path, format!("{}\n", json_bytes)).expect("Failed to write metadata JSON");
    eprintln!("Wrote {}", json_path.display());

    // Download audio
    if !args.metadata_only {
        let file_url = rec["file"]
            .as_str()
            .expect("No 'file' URL in recording data");

        let audio_path = output_dir.join(&audio_filename);

        eprintln!("Downloading audio...");
        let audio_resp = client
            .get(file_url)
            .send()
            .expect("Failed to download audio");

        if !audio_resp.status().is_success() {
            eprintln!("Failed to download audio: HTTP {}", audio_resp.status());
            std::process::exit(1);
        }

        let audio_bytes = audio_resp.bytes().expect("Failed to read audio bytes");
        let mut file = fs::File::create(&audio_path).expect("Failed to create audio file");
        file.write_all(&audio_bytes)
            .expect("Failed to write audio file");
        eprintln!("Wrote {} ({:.1} MB)", audio_path.display(), audio_bytes.len() as f64 / 1_048_576.0);
    }

    // Update index.json
    if !args.no_index {
        let index_path = args.index.unwrap_or_else(|| {
            repo_root()
                .map(|r| r.join("index.json"))
                .unwrap_or_else(|| output_dir.join("../index.json"))
        });

        let xc_id = rec["id"].as_str().and_then(|s| s.parse::<u64>().ok()).unwrap_or(xc_number);
        update_index(&index_path, xc_id, en, genus, sp, &audio_filename, &meta_filename);
    }

    // Print summary
    println!("XC{}: {} ({} {})", id, en, genus, sp);
    println!("Recordist: {}", recordist);
    println!("License: {}", lic);
    println!("Attribution: {}", attribution);
}
