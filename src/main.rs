// main.rs
use chrono::{Datelike, NaiveDate};
use clap::Parser;
use regex::Regex;
use std::{collections::HashMap, fs, path::PathBuf};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
struct Args {
    /// Template path like D:\data\参照{yyyy}年_{mm}月データ\Main
    #[arg(short, long)]
    template: String,

    /// Optional date list (e.g., 2024-12-01,2025-01-01)
    #[arg(short, long)]
    dates: Option<String>,
}

#[derive(Debug)]
struct FileInfo {
    actual_name: String,
    size: u64,
    created: String,
    date_str: String,
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
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|_| "N/A".into());

        let file_name = entry.file_name().to_string_lossy().to_string();
        let normalized = normalize_filename(&file_name, date.year(), date.month());

        map.insert(
            normalized.clone(),
            FileInfo {
                actual_name: file_name,
                size,
                created,
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

fn main() {
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

    println!("normalized_name,date,actual_name,size,created");
    for (norm_name, infos) in all {
        for info in infos {
            println!("{},{},{},{},{}", norm_name, info.date_str, info.actual_name, info.size, info.created);
        }
    }
}
