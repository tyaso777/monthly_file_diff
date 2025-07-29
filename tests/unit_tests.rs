use chrono::NaiveDate;
use monthly_file_diff::{
    resolve_template, normalize_filename, normalize_rel_path, 
    datetime_str_to_iso8601_jst, sanitize_id
};

#[test]
fn test_resolve_template() {
    let template = "D:/data/参照{yyyy}_{mm}月データ/Main";
    let date = NaiveDate::from_ymd_opt(2024, 8, 1).unwrap();
    
    let result = resolve_template(template, date);
    let expected = "D:/data/参照2024_08月データ/Main";
    
    assert_eq!(result.to_string_lossy(), expected);
}

#[test]
fn test_resolve_template_with_day() {
    let template = "D:/backup/{yyyy}/{mm}/{dd}/files";
    let date = NaiveDate::from_ymd_opt(2024, 12, 5).unwrap();
    
    let result = resolve_template(template, date);
    let expected = "D:/backup/2024/12/05/files";
    
    assert_eq!(result.to_string_lossy(), expected);
}

#[test]
fn test_normalize_filename() {
    // Test year and month replacement
    let filename = "InTheBox08-2024.xlsx";
    let result = normalize_filename(filename, 2024, 8);
    assert_eq!(result, "InTheBox{mm}-{yyyy}.xlsx");
    
    // Test with double digit month
    let filename2 = "Report12-2025.pdf";
    let result2 = normalize_filename(filename2, 2025, 12);
    assert_eq!(result2, "Report{mm}-{yyyy}.pdf");
    
    // Test no replacements needed
    let filename3 = "document.txt";
    let result3 = normalize_filename(filename3, 2024, 8);
    assert_eq!(result3, "document.txt");
}

#[test]
fn test_normalize_filename_edge_cases() {
    // Test year appears multiple times
    let filename = "2024_report_2024.xlsx";
    let result = normalize_filename(filename, 2024, 1);
    assert_eq!(result, "{yyyy}_report_{yyyy}.xlsx");
    
    // Test month zero-padding
    let filename2 = "data01file.txt";
    let result2 = normalize_filename(filename2, 2024, 1);
    assert_eq!(result2, "data{mm}file.txt");
}

#[test]
fn test_normalize_rel_path() {
    // Test root level file
    let rel_path = "InTheBox08-2024.xlsx";
    let result = normalize_rel_path(rel_path, 2024, 8);
    assert_eq!(result, "InTheBox{mm}-{yyyy}.xlsx");
    
    // Test subdirectory file
    let rel_path2 = "Sub/Folder/InTheBox12-2025.xlsx";
    let result2 = normalize_rel_path(rel_path2, 2025, 12);
    assert_eq!(result2, "Sub/Folder/InTheBox{mm}-{yyyy}.xlsx");
    
    // Test Windows-style path
    let rel_path3 = "Sub\\InTheBox01-2024.xlsx";
    let result3 = normalize_rel_path(rel_path3, 2024, 1);
    assert_eq!(result3, "Sub/InTheBox{mm}-{yyyy}.xlsx");
}

#[test]
fn test_normalize_rel_path_directory_not_normalized() {
    // Ensure directories are not normalized, only filenames
    let rel_path = "2024/08/InTheBox08-2024.xlsx";
    let result = normalize_rel_path(rel_path, 2024, 8);
    // Directory "2024/08" should remain unchanged
    assert_eq!(result, "2024/08/InTheBox{mm}-{yyyy}.xlsx");
}

#[test]
fn test_datetime_str_to_iso8601_jst() {
    let datetime_str = "2024/08/15 14:30";
    let result = datetime_str_to_iso8601_jst(datetime_str);
    assert_eq!(result, "2024-08-15T14:30:00");
    
    // Test invalid format
    let invalid = "invalid-date";
    let result2 = datetime_str_to_iso8601_jst(invalid);
    assert_eq!(result2, "null");
}

#[test]
fn test_sanitize_id() {
    let input = "Sub/InTheBox{mm}-{yyyy}.xlsx";
    let result = sanitize_id(input);
    assert_eq!(result, "Sub_InTheBox_mm___yyyy__xlsx");
    
    // Test alphanumeric only
    let input2 = "file123ABC";
    let result2 = sanitize_id(input2);
    assert_eq!(result2, "file123ABC");
    
    // Test special characters
    let input3 = "test@#$%file.txt";
    let result3 = sanitize_id(input3);
    assert_eq!(result3, "test____file_txt");
}

#[test]
fn test_sanitize_id_empty() {
    let result = sanitize_id("");
    assert_eq!(result, "");
}

#[cfg(test)]
mod date_parsing_tests {
    use super::*;
    
    #[test]
    fn test_valid_dates() {
        let date1 = NaiveDate::from_ymd_opt(2024, 1, 1);
        assert!(date1.is_some());
        
        let date2 = NaiveDate::from_ymd_opt(2024, 12, 31);
        assert!(date2.is_some());
    }
    
    #[test]
    fn test_invalid_dates() {
        let invalid_date = NaiveDate::from_ymd_opt(2024, 13, 1);
        assert!(invalid_date.is_none());
        
        let invalid_date2 = NaiveDate::from_ymd_opt(2024, 2, 30);
        assert!(invalid_date2.is_none());
    }
}