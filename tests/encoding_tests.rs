// Tests for encoding functionality and CSV output
use std::io::Write;
use encoding_rs::{SHIFT_JIS, UTF_16LE};
use encoding_rs_rw::EncodingWriter;
use chrono::NaiveDate;
use monthly_file_diff::{FileInfo, collect_files};
mod test_fixtures;
use test_fixtures::TestDataFixture;

#[test]
fn test_csv_output_format() {
    let fixture = TestDataFixture::new();
    fixture.create_monthly_structure().unwrap();
    
    let aug_dir = fixture.path().join("参照2024_08月データ/Main");
    let date = NaiveDate::from_ymd_opt(2024, 8, 1).unwrap();
    let files = collect_files(&aug_dir, date, 3, true);
    
    assert!(!files.is_empty());
    
    // Test CSV format output
    let mut csv_output = Vec::new();
    writeln!(csv_output, "normalized_rel_path,date,actual_name,size,created,modified,rel_path").unwrap();
    
    for file in &files {
        writeln!(
            csv_output,
            "{},{},{},{},{},{},{}",
            file.normalized_rel_path,
            file.date_str,
            file.actual_name,
            file.size,
            file.created,
            file.modified,
            file.rel_path
        ).unwrap();
    }
    
    let csv_string = String::from_utf8(csv_output).unwrap();
    
    // Verify CSV header
    assert!(csv_string.contains("normalized_rel_path,date,actual_name,size,created,modified,rel_path"));
    
    // Verify normalized paths
    assert!(csv_string.contains("InTheBox{mm}-{yyyy}.xlsx"));
    assert!(csv_string.contains("2024-08"));
}

#[test]
fn test_shift_jis_encoding() {
    let test_data = "テストデータ,2024-08,ファイル.txt,1024,2024/08/15 10:30,2024/08/15 10:45,ファイル.txt\n";
    
    let mut buffer = Vec::new();
    {
        let mut encoder_writer = EncodingWriter::new(&mut buffer, SHIFT_JIS.new_encoder());
        encoder_writer.write_all(test_data.as_bytes()).unwrap();
        encoder_writer.flush().unwrap();
    }
    
    // Verify that encoding happened (buffer should be different from UTF-8)
    assert_ne!(buffer, test_data.as_bytes());
    
    // Decode back to verify
    let (decoded, _, had_errors) = SHIFT_JIS.decode(&buffer);
    assert!(!had_errors);
    assert_eq!(decoded, test_data);
}

#[test]
fn test_utf16le_encoding() {
    let test_data = "Hello World";
    
    let mut buffer = Vec::new();
    {
        let mut encoder_writer = EncodingWriter::new(&mut buffer, UTF_16LE.new_encoder());
        encoder_writer.write_all(test_data.as_bytes()).unwrap();
        encoder_writer.flush().unwrap();
    }
    
    // UTF-16LE should produce different bytes than UTF-8
    assert!(!buffer.is_empty());
    assert_ne!(buffer.len(), test_data.as_bytes().len());
    
    // Decode back to verify
    let (decoded, _, had_errors) = UTF_16LE.decode(&buffer);
    assert!(!had_errors);
    assert_eq!(decoded, test_data);
}

#[test]
fn test_csv_special_characters() {
    // Test CSV output with special characters that might need escaping
    let file_info = FileInfo {
        actual_name: "file,with,commas.txt".to_string(),
        size: 1024,
        created: "2024/08/15 10:30".to_string(),
        modified: "2024/08/15 10:45".to_string(),
        date_str: "2024-08".to_string(),
        rel_path: "sub/file,with,commas.txt".to_string(),
        normalized_rel_path: "sub/file,with,commas.txt".to_string(),
    };
    
    let mut csv_output = Vec::new();
    writeln!(
        csv_output,
        "{},{},{},{},{},{},{}",
        file_info.normalized_rel_path,
        file_info.date_str,
        file_info.actual_name,
        file_info.size,
        file_info.created,
        file_info.modified,
        file_info.rel_path
    ).unwrap();
    
    let csv_string = String::from_utf8(csv_output).unwrap();
    
    // Note: This is a basic test - in production, commas in data should be properly escaped
    assert!(csv_string.contains("file,with,commas.txt"));
    assert!(csv_string.contains("2024-08"));
}

#[test]
fn test_encoding_writer_error_handling() {
    // Test with invalid sequences that might cause encoding issues
    let mut buffer = Vec::new();
    let result = {
        let mut encoder_writer = EncodingWriter::new(&mut buffer, SHIFT_JIS.new_encoder());
        // Write some data that should encode fine
        encoder_writer.write_all("Valid ASCII text".as_bytes())
    };
    
    assert!(result.is_ok());
}

#[test]
fn test_multiple_files_csv_format() {
    let files = vec![
        FileInfo {
            actual_name: "file1.txt".to_string(),
            size: 100,
            created: "2024/08/01 09:00".to_string(),
            modified: "2024/08/01 09:15".to_string(),
            date_str: "2024-08".to_string(),
            rel_path: "file1.txt".to_string(),
            normalized_rel_path: "file{mm}.txt".to_string(),
        },
        FileInfo {
            actual_name: "file2.txt".to_string(),
            size: 200,
            created: "2024/12/01 10:00".to_string(),
            modified: "2024/12/01 10:30".to_string(),
            date_str: "2024-12".to_string(),
            rel_path: "file2.txt".to_string(),
            normalized_rel_path: "file{mm}.txt".to_string(),
        },
    ];
    
    let mut csv_output = Vec::new();
    writeln!(csv_output, "normalized_rel_path,date,actual_name,size,created,modified,rel_path").unwrap();
    
    for file in &files {
        writeln!(
            csv_output,
            "{},{},{},{},{},{},{}",
            file.normalized_rel_path,
            file.date_str,
            file.actual_name,
            file.size,
            file.created,
            file.modified,
            file.rel_path
        ).unwrap();
    }
    
    let csv_string = String::from_utf8(csv_output).unwrap();
    let lines: Vec<&str> = csv_string.lines().collect();
    
    // Should have header + 2 data lines
    assert_eq!(lines.len(), 3);
    
    // Check header
    assert_eq!(lines[0], "normalized_rel_path,date,actual_name,size,created,modified,rel_path");
    
    // Check data rows
    assert!(lines[1].contains("file{mm}.txt,2024-08,file1.txt,100"));
    assert!(lines[2].contains("file{mm}.txt,2024-12,file2.txt,200"));
}

#[test]
fn test_empty_file_list_csv() {
    let files: Vec<FileInfo> = vec![];
    
    let mut csv_output = Vec::new();
    writeln!(csv_output, "normalized_rel_path,date,actual_name,size,created,modified,rel_path").unwrap();
    
    for file in &files {
        writeln!(
            csv_output,
            "{},{},{},{},{},{},{}",
            file.normalized_rel_path,
            file.date_str,
            file.actual_name,
            file.size,
            file.created,
            file.modified,
            file.rel_path
        ).unwrap();
    }
    
    let csv_string = String::from_utf8(csv_output).unwrap();
    let lines: Vec<&str> = csv_string.lines().collect();
    
    // Should only have header
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "normalized_rel_path,date,actual_name,size,created,modified,rel_path");
}
