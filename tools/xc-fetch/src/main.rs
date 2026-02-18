use chrono::Utc;
use clap::Parser;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "xc-fetch", about = "Fetch recording metadata from xeno-canto API v3")]
struct Args {
    /// Xeno-canto catalogue number (e.g. 928094)
    xc_number: u64,

    /// Also download the audio file
    #[arg(long)]
    download: bool,

    /// Output directory (default: current directory)
    #[arg(long, default_value = ".")]
    output_dir: PathBuf,

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

fn main() {
    let args = Args::parse();

    let api_key = args
        .key
        .or_else(|| std::env::var("XC_API_KEY").ok())
        .expect("API key required: pass --key or set XC_API_KEY env var");

    let url = format!(
        "https://xeno-canto.org/api/3/recordings?query=nr:{}&key={}",
        args.xc_number, api_key
    );

    eprintln!("Fetching XC{}...", args.xc_number);

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
        eprintln!("No recordings found for XC{}", args.xc_number);
        std::process::exit(1);
    }

    let rec = &recordings[0];

    let id = rec["id"].as_str().unwrap_or("");
    let gen = rec["gen"].as_str().unwrap_or("");
    let sp = rec["sp"].as_str().unwrap_or("");
    let en = rec["en"].as_str().unwrap_or("");
    let recordist = rec["rec"].as_str().unwrap_or("");
    let lic = rec["lic"].as_str().unwrap_or("");

    let base_name = sanitize_filename(&format!("XC{} - {} - {} {}", id, en, gen, sp));

    let attribution = format!(
        "{}, XC{}. Accessible at www.xeno-canto.org/{}",
        recordist, id, id
    );

    let metadata = json!({
        "source": "xeno-canto",
        "xc_id": rec["id"].as_str().and_then(|s| s.parse::<u64>().ok()).unwrap_or(args.xc_number),
        "url": format!("https://www.xeno-canto.org/{}", id),
        "file_url": rec["file"],
        "gen": gen,
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

    fs::create_dir_all(&args.output_dir).expect("Failed to create output directory");

    let json_path = args.output_dir.join(format!("{}.xc.json", base_name));
    let json_bytes = serde_json::to_string_pretty(&metadata).expect("Failed to serialize JSON");
    fs::write(&json_path, format!("{}\n", json_bytes)).expect("Failed to write metadata JSON");
    eprintln!("Wrote {}", json_path.display());

    if args.download {
        let file_url = rec["file"]
            .as_str()
            .expect("No 'file' URL in recording data");

        // Determine extension from file-name field or default to .wav
        let ext = rec["file-name"]
            .as_str()
            .and_then(|name| name.rsplit('.').next())
            .unwrap_or("wav");

        let audio_path = args.output_dir.join(format!("{}.{}", base_name, ext));

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

    // Print summary
    println!("XC{}: {} ({} {})", id, en, gen, sp);
    println!("Recordist: {}", recordist);
    println!("License: {}", lic);
    println!("Attribution: {}", attribution);
}
