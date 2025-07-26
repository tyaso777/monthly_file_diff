// main.rs
use chrono::NaiveDate;
use clap::Parser;
use std::{
    collections::{HashMap, BTreeMap},
    fs,
    io::{self, Write},
    path::{PathBuf, Path},
};
use encoding_rs::{SHIFT_JIS, UTF_16LE};
use encoding_rs_rw::EncodingWriter;

use serde::Serialize;
use serde_json::to_string as to_json;
use tera::{Context, Tera};

use monthly_file_diff::{
    FileInfo, resolve_template, collect_files, extract_dates_from_template,
    datetime_str_to_iso8601_jst, sanitize_id
};

#[derive(Parser, Debug)]
struct Args {
    /// Template path like D:\data\参照{yyyy}年_{mm}月データ\Main
    #[arg(short, long)]
    template: String,

    /// Optional date list (e.g., 2024-12-01,2025-01-01)
    #[arg(short, long)]
    dates: Option<String>,

    /// Output encoding for CSV: "utf8", "shift_jis", or "utf16le". Default is utf8.
    #[arg(short, long)]
    encoding: Option<String>,

    /// Max directory depth to search (default: 2)
    #[arg(long, default_value_t = 2)]
    max_depth: usize,

    /// Output HTML file path (default: output.html)
    #[arg(long, default_value = "")]
    html_file: String,
}


#[derive(Serialize)]
struct ChartFile {
    name: String,
    id: String,
    dates_json: String,
    sizes_json: String,
    created_json: String,
    modified_json: String,
    display_path: String,
    display_file_name: String,
}



fn write_html_report_with_tera(
    out_path: &Path,
    grouped: &BTreeMap<String, Vec<FileInfo>>,
) -> io::Result<()> {
    let files: Vec<ChartFile> = grouped
        .iter()
        .map(|(norm_rel_path, infos)| {
            // time series data
            let dates: Vec<String> = infos.iter().map(|i| i.date_str.clone()).collect();
            let sizes: Vec<u64> = infos.iter().map(|i| i.size).collect();
            let created: Vec<String> = infos
                .iter()
                .map(|i| datetime_str_to_iso8601_jst(&i.created))
                .collect();
            let modified: Vec<String> = infos
                .iter()
                .map(|i| datetime_str_to_iso8601_jst(&i.modified))
                .collect();

            // display: split path & filename from normalized_rel_path
            let p = Path::new(norm_rel_path);
            let display_file_name = p
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| norm_rel_path.clone());
            let display_path = p
                .parent()
                .map(|pp| pp.display().to_string().replace('\\', "/"))
                .unwrap_or_else(|| ".".to_string());

            ChartFile {
                name: norm_rel_path.clone(),
                id: sanitize_id(norm_rel_path),
                dates_json: to_json(&dates).unwrap(),
                sizes_json: to_json(&sizes).unwrap(),
                created_json: to_json(&created).unwrap(),
                modified_json: to_json(&modified).unwrap(),
                display_path,
                display_file_name,
            }
        })
        .collect();

    let tera = Tera::new("templates/**/*.html")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let mut ctx = Context::new();
    ctx.insert("title", "File Info Charts");
    ctx.insert("files", &files);

    let rendered = tera
        .render("report.html", &ctx)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    fs::write(out_path, rendered)
}


fn main() -> io::Result<()> {
    let args = Args::parse();

    let dates: Vec<NaiveDate> = if let Some(date_str) = args.dates {
        date_str
            .split(',')
            .filter_map(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
            .collect()
    } else {
        extract_dates_from_template(&args.template)
    };

    // normalized_rel_path -> vec<FileInfo>
    let mut grouped_by_norm_rel: HashMap<String, Vec<FileInfo>> = HashMap::new();

    for date in &dates {
        let path = resolve_template(&args.template, *date);
        if !path.exists() {
            eprintln!("Skipping missing path: {:?}", path);
            continue;
        }
        for info in collect_files(&path, *date, args.max_depth) {
            grouped_by_norm_rel
                .entry(info.normalized_rel_path.clone())
                .or_default()
                .push(info);
        }
    }

    // CSV output (same as before, but using the new grouping)
    let enc_label = args.encoding.as_deref().unwrap_or("utf8").to_lowercase();
    let mut writer: Box<dyn Write> = match enc_label.as_str() {
        "shift_jis" => {
            let stdout = io::stdout();
            let handle = stdout.lock();
            Box::new(EncodingWriter::new(handle, SHIFT_JIS.new_encoder()))
        }
        "utf16le" => {
            let encoder = UTF_16LE.new_encoder();
            let stdout = io::stdout();
            let handle = stdout.lock();
            Box::new(EncodingWriter::new(handle, encoder))
        }
        _ => {
            let stdout = io::stdout();
            Box::new(stdout.lock())
        }
    };

    writeln!(
        writer,
        "normalized_rel_path,date,actual_name,size,created,modified,rel_path"
    )?;

    for (norm_rel, infos) in &grouped_by_norm_rel {
        for info in infos {
            writeln!(
                writer,
                "{},{},{},{},{},{},{}",
                norm_rel,
                info.date_str,
                info.actual_name,
                info.size,
                info.created,
                info.modified,
                info.rel_path
            )?;
        }
    }
    writer.flush()?;

    // stable ordering for HTML
    let grouped: BTreeMap<String, Vec<FileInfo>> =
        grouped_by_norm_rel.into_iter().collect();

    let html_path = PathBuf::from(&args.html_file);
    if !args.html_file.trim().is_empty() {
        write_html_report_with_tera(&html_path, &grouped)?;
    }

    Ok(())
}