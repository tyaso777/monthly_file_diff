use std::fs;
use std::path::{Path, PathBuf};
use chrono::NaiveDate;
use tempfile::TempDir;

use monthly_file_diff::{
    collect_files, extract_dates_from_template, resolve_template
};

fn create_test_file_structure(base_dir: &Path) -> std::io::Result<()> {
    // Create directory structure: 参照2024_08月データ/Main/
    let aug_dir = base_dir.join("参照2024_08月データ").join("Main");
    fs::create_dir_all(&aug_dir)?;
    
    // Create directory structure: 参照2024_12月データ/Main/
    let dec_dir = base_dir.join("参照2024_12月データ").join("Main");
    fs::create_dir_all(&dec_dir)?;
    
    // Create directory structure: 参照2025_01月データ/Main/
    let jan_dir = base_dir.join("参照2025_01月データ").join("Main");
    fs::create_dir_all(&jan_dir)?;
    
    // Create files in August directory
    fs::write(aug_dir.join("InTheBox08-2024.xlsx"), b"August data")?;
    
    // Create subdirectory and file
    let aug_sub = aug_dir.join("Sub");
    fs::create_dir_all(&aug_sub)?;
    fs::write(aug_sub.join("Report08-2024.pdf"), b"August report")?;
    
    // Create files in December directory
    fs::write(dec_dir.join("InTheBox12-2024.xlsx"), b"December data")?;
    let dec_sub = dec_dir.join("Sub");
    fs::create_dir_all(&dec_sub)?;
    fs::write(dec_sub.join("Report12-2024.pdf"), b"December report")?;
    
    // Create files in January directory
    fs::write(jan_dir.join("InTheBox01-2025.xlsx"), b"January data")?;
    let jan_sub = jan_dir.join("Sub");
    fs::create_dir_all(&jan_sub)?;
    fs::write(jan_sub.join("Report01-2025.pdf"), b"January report")?;
    
    Ok(())
}

#[test]
fn test_collect_files_integration() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create test structure
    create_test_file_structure(base_path).unwrap();
    
    let aug_main_dir = base_path.join("参照2024_08月データ").join("Main");
    let date = NaiveDate::from_ymd_opt(2024, 8, 1).unwrap();
    
    let files = collect_files(&aug_main_dir, date, 3);
    
    // Should find 2 files (root + subdirectory)
    assert_eq!(files.len(), 2);
    
    // Check normalization works correctly
    let xlsx_file = files.iter().find(|f| f.actual_name.contains("InTheBox")).unwrap();
    assert_eq!(xlsx_file.normalized_rel_path, "InTheBox{mm}-{yyyy}.xlsx");
    assert_eq!(xlsx_file.date_str, "2024-08");
    
    let pdf_file = files.iter().find(|f| f.actual_name.contains("Report")).unwrap();
    assert_eq!(pdf_file.normalized_rel_path, "Sub/Report{mm}-{yyyy}.pdf");
    assert_eq!(pdf_file.rel_path, "Sub/Report08-2024.pdf");
}

#[test]
fn test_collect_files_max_depth() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    let test_dir = base_path.join("test_main");
    fs::create_dir_all(&test_dir).unwrap();
    
    // Create nested structure: test_main/level1/level2/file.txt
    let level1 = test_dir.join("level1");
    let level2 = level1.join("level2");
    fs::create_dir_all(&level2).unwrap();
    
    fs::write(test_dir.join("root_file.txt"), b"root").unwrap();
    fs::write(level1.join("level1_file.txt"), b"level1").unwrap();
    fs::write(level2.join("level2_file.txt"), b"level2").unwrap();
    
    let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    
    // Test max_depth = 1 (should only find root_file.txt)
    let files_depth1 = collect_files(&test_dir, date, 1);
    assert_eq!(files_depth1.len(), 1);
    assert_eq!(files_depth1[0].actual_name, "root_file.txt");
    
    // Test max_depth = 2 (should find root + level1)
    let files_depth2 = collect_files(&test_dir, date, 2);
    assert_eq!(files_depth2.len(), 2);
    
    // Test max_depth = 3 (should find all files)
    let files_depth3 = collect_files(&test_dir, date, 3);
    assert_eq!(files_depth3.len(), 3);
}

#[test]
fn test_extract_dates_from_template_integration() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create test directory structure
    create_test_file_structure(base_path).unwrap();
    
    let template = format!("{}/参照{{yyyy}}_{{mm}}月データ/Main", base_path.display());
    let dates = extract_dates_from_template(&template);
    
    // Should find 3 dates: 2024-08, 2024-12, 2025-01
    assert_eq!(dates.len(), 3);
    
    assert_eq!(dates[0], NaiveDate::from_ymd_opt(2024, 8, 1).unwrap());
    assert_eq!(dates[1], NaiveDate::from_ymd_opt(2024, 12, 1).unwrap());
    assert_eq!(dates[2], NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
}

#[test]
fn test_extract_dates_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let template = format!("{}/nonexistent_{{yyyy}}_{{mm}}/Main", temp_dir.path().display());
    
    let dates = extract_dates_from_template(&template);
    assert_eq!(dates.len(), 0);
}

#[test]
fn test_extract_dates_invalid_format() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create directories that don't match the template
    fs::create_dir_all(base_path.join("invalid_format")).unwrap();
    fs::create_dir_all(base_path.join("参照invalid_08月データ")).unwrap();
    
    let template = format!("{}/参照{{yyyy}}_{{mm}}月データ/Main", base_path.display());
    let dates = extract_dates_from_template(&template);
    
    assert_eq!(dates.len(), 0);
}

#[test]
fn test_resolve_template_integration() {
    let template = "/base/参照{yyyy}_{mm}月データ/Main";
    let date = NaiveDate::from_ymd_opt(2024, 8, 15).unwrap();
    
    let resolved = resolve_template(template, date);
    let expected = PathBuf::from("/base/参照2024_08月データ/Main");
    
    assert_eq!(resolved, expected);
}

#[test]
fn test_full_workflow_integration() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create test structure
    create_test_file_structure(base_path).unwrap();
    
    // Extract dates from template
    let template = format!("{}/参照{{yyyy}}_{{mm}}月データ/Main", base_path.display());
    let dates = extract_dates_from_template(&template);
    
    assert_eq!(dates.len(), 3);
    
    // Collect files for each date
    let mut all_files = Vec::new();
    for date in dates {
        let resolved_path = resolve_template(&template, date);
        if resolved_path.exists() {
            let files = collect_files(&resolved_path, date, 3);
            all_files.extend(files);
        }
    }
    
    // Should have collected files from all 3 months (2 files per month)
    assert_eq!(all_files.len(), 6);
    
    // Verify normalization across different months
    let normalized_paths: Vec<_> = all_files.iter()
        .map(|f| f.normalized_rel_path.clone())
        .collect();
    
    // Should have normalized versions of the files
    assert!(normalized_paths.contains(&"InTheBox{mm}-{yyyy}.xlsx".to_string()));
    assert!(normalized_paths.contains(&"Sub/Report{mm}-{yyyy}.pdf".to_string()));
}

#[test]
fn test_file_metadata_collection() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("metadata_test");
    fs::create_dir_all(&test_dir).unwrap();
    
    // Create a test file
    let test_file = test_dir.join("test_file.txt");
    fs::write(&test_file, b"test content for metadata").unwrap();
    
    let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let files = collect_files(&test_dir, date, 2);
    
    assert_eq!(files.len(), 1);
    let file_info = &files[0];
    
    // Check basic metadata
    assert_eq!(file_info.actual_name, "test_file.txt");
    assert_eq!(file_info.size, 25); // "test content for metadata".len()
    assert_eq!(file_info.date_str, "2024-06");
    assert_eq!(file_info.rel_path, "test_file.txt");
    
    // Check timestamp format (should be YYYY/MM/DD HH:MM or "N/A")
    assert!(file_info.created.contains("/") || file_info.created == "N/A");
    assert!(file_info.modified.contains("/") || file_info.modified == "N/A");
}

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir_all(&empty_dir).unwrap();
    
    let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let files = collect_files(&empty_dir, date, 2);
    
    assert_eq!(files.len(), 0);
}

#[test]
fn test_nonexistent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("does_not_exist");
    
    let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let files = collect_files(&nonexistent, date, 2);
    
    // Should handle gracefully and return empty vec
    assert_eq!(files.len(), 0);
}