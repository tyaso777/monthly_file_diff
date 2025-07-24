// main.rs
use chrono::{Datelike, NaiveDate, NaiveDateTime, DateTime, Local, Duration, Timelike};
use clap::Parser;
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    fs::File,
    io::{self, Write, BufWriter},
    path::PathBuf,
};
use std::path::Path;
use walkdir::WalkDir;
use encoding_rs::{SHIFT_JIS, UTF_16LE};
use encoding_rs_rw::EncodingWriter;

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

fn parse_datetime_to_timestamp(s: &str) -> i64 {
    NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M")
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0)
}

fn write_html_report(all: &HashMap<String, Vec<FileInfo>>, out_path: &Path) -> io::Result<()> {
    let file = File::create(out_path)?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, r#"
    <!DOCTYPE html>
    <html>
    <head>
      <meta charset="utf-8">
      <title>File Info Report</title>
      <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
      <style>
        body {{ font-family: sans-serif; padding: 2em; }}
        h2 {{ margin-top: 2em; }}
        .row {{ display: flex; gap: 2em; margin-bottom: 4em; }}
        canvas {{ max-width: 480px; height: 300px; }}
      </style>
    </head>
    <body>
    <h1>File Info Charts</h1>
    "#)?;

    for (norm_name, infos) in all {
        let mut infos = infos.clone();
        infos.sort_by_key(|i| i.date_str.clone());

        let dates: Vec<String> = infos.iter().map(|i| i.date_str.clone()).collect();
        let sizes: Vec<String> = infos.iter().map(|i| i.size.to_string()).collect();
        let created_ts: Vec<String> = infos.iter()
            .map(|i| parse_datetime_to_timestamp(&i.created).to_string())
            .collect();
        let modified_ts: Vec<String> = infos.iter()
            .map(|i| parse_datetime_to_timestamp(&i.modified).to_string())
            .collect();

        let safe_id = norm_name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");

        let size_id = format!("chart_size_{}", safe_id);
        let time_id = format!("chart_time_{}", safe_id);

        writeln!(writer, r#"<h2>{}</h2>"#, norm_name)?;
        writeln!(writer, r#"<div class="row">"#)?;
        writeln!(writer, r#"<canvas id="{}"></canvas>"#, size_id)?;
        writeln!(writer, r#"<canvas id="{}"></canvas>"#, time_id)?;
        writeln!(writer, r#"</div>"#)?;

        // Size chart
        writeln!(writer, r#"<script>
new Chart(document.getElementById("{size_id}").getContext("2d"), {{
  type: "line",
  data: {{
    labels: {labels:?},
    datasets: [
      {{
        label: "size",
        data: {size:?},
        borderColor: "blue",
        fill: false
      }}
    ]
  }},
  options: {{
    responsive: true,
    plugins: {{ title: {{ display: true, text: "Size" }} }},
    scales: {{
      x: {{ title: {{ display: true, text: "Date" }} }},
      y: {{ title: {{ display: true, text: "Size" }} }}
    }}
  }}
}});
"#,
        size_id = size_id,
        labels = dates,
        size = sizes)?;

        // Time chart with style distinction
        writeln!(writer, r#"
new Chart(document.getElementById("{time_id}").getContext("2d"), {{
  type: "line",
  data: {{
    labels: {labels:?},
    datasets: [
      {{
        label: "created",
        data: {created:?},
        borderColor: "green",
        borderDash: [4, 2],
        pointStyle: "circle",
        pointRadius: 5,
        fill: false
      }},
      {{
        label: "modified",
        data: {modified:?},
        borderColor: "orange",
        borderDash: [],
        pointStyle: "triangle",
        pointRadius: 5,
        fill: false
      }}
    ]
  }},
  options: {{
    responsive: true,
    plugins: {{ title: {{ display: true, text: "Created / Modified (timestamp)" }} }},
    scales: {{
      x: {{ title: {{ display: true, text: "Date" }} }},
      y: {{ title: {{ display: true, text: "Unix Timestamp" }} }}
    }}
  }}
}});
</script>"#,
        time_id = time_id,
        labels = dates,
        created = created_ts,
        modified = modified_ts)?;
    }

    writeln!(writer, "</body></html>")?;
    Ok(())
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

    let html_path = PathBuf::from("output.html");
    write_html_report(&all, &html_path)?;

    Ok(())

}
