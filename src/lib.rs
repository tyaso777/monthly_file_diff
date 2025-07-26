// lib.rs - Extract functions for testing
use chrono::{Datelike, NaiveDate, NaiveDateTime, DateTime, Local, Duration, Timelike, FixedOffset, TimeZone};
use regex::Regex;
use std::{
    fs,
    path::{PathBuf, Path},
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub actual_name: String,
    pub size: u64,
    pub created: String,      // "YYYY/MM/DD HH:MM"
    pub modified: String,     // "YYYY/MM/DD HH:MM"
    pub date_str: String,     // "YYYY-MM"
    /// Path relative to the resolved monthly root (e.g. "Sub/InTheBox08-2024.xlsx")
    pub rel_path: String,
    /// Relative path where yyyy/mm are normalized to {yyyy}/{mm} on the file name part
    pub normalized_rel_path: String,
}

pub fn resolve_template(path_template: &str, date: NaiveDate) -> PathBuf {
    let replaced = path_template
        .replace("{yyyy}", &format!("{}", date.year()))
        .replace("{mm}", &format!("{:02}", date.month()))
        .replace("{dd}", &format!("{:02}", date.day()));
    PathBuf::from(replaced)
}

pub fn normalize_filename(name: &str, yyyy: i32, mm: u32) -> String {
    // Replace the four-digit year first
    let with_year = name.replace(&yyyy.to_string(), "{yyyy}");
    // Then replace the zero-padded month
    let month_str = format!("{:02}", mm);
    with_year.replace(&month_str, "{mm}")
}

pub fn normalize_rel_path(rel_path: &str, yyyy: i32, mm: u32) -> String {
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

pub fn collect_files(root: &Path, date: NaiveDate, max_depth: usize) -> Vec<FileInfo> {
    let mut out = Vec::new();

    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(max_depth)
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

pub fn extract_dates_from_template(template: &str) -> Vec<NaiveDate> {
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

pub fn datetime_str_to_iso8601_jst(s: &str) -> String {
    let jst = FixedOffset::east_opt(9 * 3600).unwrap();
    NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M")
        .ok()
        .and_then(|naive| jst.from_local_datetime(&naive).single())
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_else(|| "null".to_string())
}

pub fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}