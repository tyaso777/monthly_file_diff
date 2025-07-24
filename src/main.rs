// main.rs
use chrono::{Datelike, NaiveDate, NaiveDateTime, DateTime, Local, Duration, Timelike, FixedOffset, TimeZone};
use clap::Parser;
use regex::Regex;
use std::{
    collections::{HashMap, BTreeMap},
    fs,
    io,
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
    created: String,
    date_str: String,
    modified: String,
}

#[derive(Serialize)]
struct ChartFile {
    name: String,
    id: String,
    dates_json: String,
    sizes_json: String,
    created_json: String,
    modified_json: String,
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

fn collect_files(path: &PathBuf, date: NaiveDate) -> HashMap<String, FileInfo> {
    let mut map = HashMap::new();

    for entry in WalkDir::new(path)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let meta = match fs::metadata(entry.path()) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = meta.len();
        let created = meta
            .created()
            .map(|t| {
                // Convert SystemTime to a local DateTime
                let mut dt: DateTime<Local> = DateTime::from(t);
                // Explorer-style rounding: if seconds >= 30, round up the minute
                if dt.second() >= 30 {
                    dt = dt + Duration::minutes(1);
                }
                // Format as "YYYY/MM/DD HH:MM"
                dt.format("%Y/%m/%d %H:%M").to_string()
            })
            .unwrap_or_else(|_| "N/A".into());
        let modified = meta
            .modified()
            .map(|t| {
                // Convert SystemTime to a local DateTime
                let mut dt: DateTime<Local> = DateTime::from(t);
                // Explorer-style rounding: if seconds >= 30, round up the minute
                if dt.second() >= 30 {
                    dt = dt + Duration::minutes(1);
                }
                // Format as "YYYY/MM/DD HH:MM"
                dt.format("%Y/%m/%d %H:%M").to_string()
            })
            .unwrap_or_else(|_| "N/A".into());
        let file_name = entry.file_name().to_string_lossy().to_string();
        let normalized = normalize_filename(&file_name, date.year(), date.month());

        map.insert(
            normalized.clone(),
            FileInfo {
                actual_name: file_name,
                size,
                created,
                modified,
                date_str: date.format("%Y-%m").to_string(),
            },
        );
    }

    map
}

fn extract_dates_from_template(template: &str) -> Vec<NaiveDate> {
    // Build a PathBuf and locate the directory that contains the date-folders.
    let tpl = PathBuf::from(template);
    let main_dir = tpl.parent().unwrap_or_else(|| Path::new("."));
    // The parent of "Main" is e.g. "参照{yyyy}_{mm}月データ"; its parent is TestData.
    let base_dir = main_dir.parent().unwrap_or_else(|| Path::new("."));

    // Prepare a folder-name template, e.g. "参照{yyyy}_{mm}月データ".
    let folder_tpl = main_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Build a regex by escaping the template and replacing placeholders with named groups.
    let mut re_str = regex::escape(&folder_tpl);
    re_str = re_str.replace(r"\{yyyy\}", r"(?P<yyyy>\d{4})");
    re_str = re_str.replace(r"\{mm\}",   r"(?P<mm>\d+)");
    re_str = re_str.replace(r"\{dd\}",   r"(?P<dd>\d+)");
    let re = Regex::new(&re_str).expect("Invalid regex from template");

    let mut dates = Vec::new();
    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(caps) = re.captures(name) {
                    if let (Some(y), Some(m)) = (
                        caps.name("yyyy").and_then(|m| m.as_str().parse::<i32>().ok()),
                        caps.name("mm").  and_then(|m| m.as_str().parse::<u32>().ok()),
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

fn write_html_report_with_tera(
    out_path: &Path,
    grouped: &BTreeMap<String, Vec<FileInfo>>,
) -> io::Result<()> {
    // Build view-model for Tera
    let files: Vec<ChartFile> = grouped
        .iter()
        .map(|(name, infos)| {
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

            // Prepare JSON strings to embed "as is" in JS code.
            let dates_json = to_json(&dates).unwrap();
            let sizes_json = to_json(&sizes).unwrap();
            let created_json = to_json(&created).unwrap();
            let modified_json = to_json(&modified).unwrap();

            let id = name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");

            ChartFile {
                name: name.clone(),
                id,
                dates_json,
                sizes_json,
                created_json,
                modified_json,
            }
        })
        .collect();

    // Load template(s)
    let tera = Tera::new("templates/**/*.html")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Build context
    let mut ctx = Context::new();
    ctx.insert("title", "File Info Charts");
    ctx.insert("files", &files);

    // Render
    let rendered = tera
        .render("report.html", &ctx)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Write out
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

    let mut all: HashMap<String, Vec<FileInfo>> = HashMap::new();

    for date in &dates {
        let path = resolve_template(&args.template, *date);
        if !path.exists() {
            eprintln!("Skipping missing path: {:?}", path);
            continue;
        }
        let files = collect_files(&path, *date);
        for (norm_name, info) in files {
            all.entry(norm_name).or_default().push(info);
        }
    }

    let enc_label = args.encoding
        .as_deref()
        .unwrap_or("utf8")
        .to_lowercase();

    let mut writer: Box<dyn Write> = match enc_label.as_str() {
        "shift_jis" => {
            // SHIFT_JIS encoding
            let stdout = io::stdout();
            let handle = stdout.lock();
            Box::new(EncodingWriter::new(handle, SHIFT_JIS.new_encoder()))
        }
        "utf16le" => {
            // UTF-16LE encoding
            let encoder = UTF_16LE.new_encoder();
            let stdout = io::stdout();
            let handle = stdout.lock();
            Box::new(EncodingWriter::new(handle, encoder))
        }
        _ => {
            // UTF-8 (no wrapper)
            let stdout = io::stdout();
            Box::new(stdout.lock())
        }
    };

    writeln!(
        writer,
        "normalized_name,date,actual_name,size,created,modified"
    )?;

    for (norm_name, infos) in &all {
        for info in infos {
            writeln!(
                writer,
                "{},{},{},{},{},{}",
                norm_name,
                info.date_str,
                info.actual_name,
                info.size,
                info.created,
                info.modified
            )?;
        }
    }
    writer.flush()?;

    use std::io::Write;
    // Convert HashMap -> BTreeMap to get stable ordering in HTML
    let grouped: BTreeMap<String, Vec<FileInfo>> =
        all.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    let html_path = PathBuf::from("output.html");
    write_html_report_with_tera(&html_path, &grouped)?;

    Ok(())

}
