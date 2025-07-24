// main.rs
use chrono::{Datelike, NaiveDate, NaiveDateTime, DateTime, Local, Duration, Timelike, FixedOffset, TimeZone};
use clap::Parser;
use regex::Regex;
use std::{
    collections::{HashMap, BTreeMap},
    fs,
    io::{self, Write},
    path::{PathBuf, Path},
};
use walkdir::WalkDir;
use encoding_rs::{SHIFT_JIS, UTF_16LE};
use encoding_rs_rw::EncodingWriter;

use serde::Serialize;
use serde_json::to_string as to_json;
use tera::{Context, Tera};

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
}

#[derive(Debug, Clone)]
struct FileInfo {
    actual_name: String,
    size: u64,
    created: String,      // "YYYY/MM/DD HH:MM"
    modified: String,     // "YYYY/MM/DD HH:MM"
    date_str: String,     // "YYYY-MM"
    /// Path relative to the resolved monthly root (e.g. "Sub/InTheBox08-2024.xlsx")
    rel_path: String,
    /// Relative path where yyyy/mm are normalized to {yyyy}/{mm} on the file name part
    normalized_rel_path: String,
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

fn resolve_template(path_template: &str, date: NaiveDate) -> PathBuf {
    let replaced = path_template
        .replace("{yyyy}", &format!("{}", date.year()))
        .replace("{mm}", &format!("{:02}", date.month()))
        .replace("{dd}", &format!("{:02}", date.day()));
    PathBuf::from(replaced)
}

fn normalize_filename(name: &str, yyyy: i32, mm: u32) -> String {
    // Replace the four-digit year first
    let with_year = name.replace(&yyyy.to_string(), "{yyyy}");
    // Then replace the zero-padded month
    let month_str = format!("{:02}", mm);
    with_year.replace(&month_str, "{mm}")
}

fn normalize_rel_path(rel_path: &str, yyyy: i32, mm: u32) -> String {
    // Only normalize the file name part, keep directories as they are
    let p = Path::new(rel_path);
    let file = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let normalized_file = normalize_filename(&file, yyyy, mm);
    if let Some(parent) = p.parent() {
        if parent.as_os_str().is_empty() {
            normalized_file
        } else {
            format!("{}/{}", parent.to_string_lossy().replace('\\', "/"), normalized_file)
        }
    } else {
        normalized_file
    }
}

fn collect_files(root: &Path, date: NaiveDate) -> Vec<FileInfo> {
    let mut out = Vec::new();

    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(64) // allow deeper subdirs if you want
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let meta = match fs::metadata(entry.path()) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // relative path from root
        let rel_path = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");

        let size = meta.len();
        let created = meta
            .created()
            .map(|t| {
                let mut dt: DateTime<Local> = DateTime::from(t);
                if dt.second() >= 30 {
                    dt = dt + Duration::minutes(1);
                }
                dt.format("%Y/%m/%d %H:%M").to_string()
            })
            .unwrap_or_else(|_| "N/A".into());
        let modified = meta
            .modified()
            .map(|t| {
                let mut dt: DateTime<Local> = DateTime::from(t);
                if dt.second() >= 30 {
                    dt = dt + Duration::minutes(1);
                }
                dt.format("%Y/%m/%d %H:%M").to_string()
            })
            .unwrap_or_else(|_| "N/A".into());

        let file_name = entry.file_name().to_string_lossy().to_string();
        let normalized_rel_path = normalize_rel_path(&rel_path, date.year(), date.month());

        out.push(FileInfo {
            actual_name: file_name,
            size,
            created,
            modified,
            date_str: date.format("%Y-%m").to_string(),
            rel_path,
            normalized_rel_path,
        });
    }

    out
}

fn extract_dates_from_template(template: &str) -> Vec<NaiveDate> {
    let tpl = PathBuf::from(template);
    let main_dir = tpl.parent().unwrap_or_else(|| Path::new("."));
    let base_dir = main_dir.parent().unwrap_or_else(|| Path::new("."));

    let folder_tpl = main_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut re_str = regex::escape(&folder_tpl);
    re_str = re_str.replace(r"\{yyyy\}", r"(?P<yyyy>\d{4})");
    re_str = re_str.replace(r"\{mm\}", r"(?P<mm>\d+)");
    re_str = re_str.replace(r"\{dd\}", r"(?P<dd>\d+)");
    let re = Regex::new(&re_str).expect("Invalid regex from template");

    let mut dates = Vec::new();
    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(caps) = re.captures(name) {
                    if let (Some(y), Some(m)) = (
                        caps.name("yyyy").and_then(|m| m.as_str().parse::<i32>().ok()),
                        caps.name("mm").and_then(|m| m.as_str().parse::<u32>().ok()),
                    ) {
                        if let Some(d) = NaiveDate::from_ymd_opt(y, m, 1) {
                            dates.push(d);
                        }
                    }
                }
            }
        }
    }

    dates.sort_unstable();
    dates
}


fn datetime_str_to_iso8601_jst(s: &str) -> String {
    let jst = FixedOffset::east_opt(9 * 3600).unwrap();
    NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M")
        .ok()
        .and_then(|naive| jst.from_local_datetime(&naive).single())
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
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
        for info in collect_files(&path, *date) {
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

    let html_path = PathBuf::from("output.html");
    write_html_report_with_tera(&html_path, &grouped)?;

    Ok(())
}