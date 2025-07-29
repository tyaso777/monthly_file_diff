// Test fixtures and helper functions for creating test data
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct TestDataFixture {
    pub temp_dir: TempDir,
    pub base_path: PathBuf,
}

impl TestDataFixture {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();
        
        Self {
            temp_dir,
            base_path,
        }
    }
    
    /// Creates a standard monthly directory structure for testing
    pub fn create_monthly_structure(&self) -> std::io::Result<()> {
        self.create_month_data(2024, 8, &[("InTheBox08-2024.xlsx", 1024), ("Summary08-2024.pdf", 2048)])?;
        self.create_month_data(2024, 12, &[("InTheBox12-2024.xlsx", 1536), ("Summary12-2024.pdf", 512)])?;
        self.create_month_data(2025, 1, &[("InTheBox01-2025.xlsx", 2048), ("Summary01-2025.pdf", 1024)])?;
        Ok(())
    }
    
    /// Creates monthly data directory with specified files
    pub fn create_month_data(&self, year: i32, month: u32, files: &[(&str, usize)]) -> std::io::Result<()> {
        let month_dir = self.base_path
            .join(format!("参照{year}_{month:02}月データ"))
            .join("Main");
        fs::create_dir_all(&month_dir)?;
        
        // Create subdirectory
        let sub_dir = month_dir.join("Sub");
        fs::create_dir_all(&sub_dir)?;
        
        for (filename, size) in files {
            let content = vec![b'X'; *size];
            
            // Place some files in root, some in subdirectory
            let file_path = if filename.contains("Summary") {
                sub_dir.join(filename)
            } else {
                month_dir.join(filename)
            };
            
            fs::write(file_path, content)?;
        }
        
        Ok(())
    }
    
    /// Creates a complex nested directory structure for depth testing
    pub fn create_nested_structure(&self, max_depth: usize) -> std::io::Result<PathBuf> {
        let nested_root = self.base_path.join("nested_test");
        fs::create_dir_all(&nested_root)?;
        
        let mut current_path = nested_root.clone();
        
        for depth in 1..=max_depth {
            current_path = current_path.join(format!("level_{depth}"));
            fs::create_dir_all(&current_path)?;
            
            // Create a file at each level
            let filename = format!("file_at_depth_{depth}.txt");
            let content = format!("Content at depth {depth}").into_bytes();
            fs::write(current_path.join(&filename), content)?;
        }
        
        Ok(nested_root)
    }
    
    /// Creates files with specific timestamps (mock data)
    pub fn create_timestamped_files(&self) -> std::io::Result<PathBuf> {
        let timestamp_dir = self.base_path.join("timestamp_test");
        fs::create_dir_all(&timestamp_dir)?;
        
        // Create files with different content sizes to test metadata collection
        let files = vec![
            ("small_file.txt", b"small".to_vec()),
            ("medium_file.txt", vec![b'M'; 1000]),
            ("large_file.txt", vec![b'L'; 10000]),
            ("empty_file.txt", vec![]),
        ];
        
        for (filename, content) in files {
            fs::write(timestamp_dir.join(filename), content)?;
        }
        
        Ok(timestamp_dir)
    }
    
    /// Creates invalid directory names that shouldn't match templates
    pub fn create_invalid_directories(&self) -> std::io::Result<()> {
        let invalid_dirs = vec![
            "invalid_format",
            "参照invalid_08月データ", 
            "参照2024_invalid月データ",
            "other_directory",
            "参照2024_13月データ", // Invalid month
        ];
        
        for dir_name in invalid_dirs {
            let dir_path = self.base_path.join(dir_name);
            fs::create_dir_all(&dir_path)?;
            
            // Add some files to make sure they're not accidentally picked up
            fs::write(dir_path.join("should_not_appear.txt"), b"ignored")?;
        }
        
        Ok(())
    }
    
    /// Creates files with special characters in names
    pub fn create_special_char_files(&self) -> std::io::Result<PathBuf> {
        let special_dir = self.base_path.join("special_chars");
        fs::create_dir_all(&special_dir)?;
        
        let special_files = vec![
            "file with spaces.txt",
            "file-with-dashes.txt", 
            "file_with_underscores.txt",
            "file.with.dots.txt",
            "ファイル日本語.txt", // Japanese characters
        ];
        
        for filename in special_files {
            fs::write(special_dir.join(filename), b"special content")?;
        }
        
        Ok(special_dir)
    }
    
    /// Returns the template path for the standard monthly structure
    pub fn monthly_template(&self) -> String {
        format!("{}/参照{{yyyy}}_{{mm}}月データ/Main", self.base_path.display())
    }
    
    /// Returns the base path for accessing created files
    pub fn path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    use monthly_file_diff::extract_dates_from_template;
    
    #[test]
    fn test_fixture_monthly_structure() {
        let fixture = TestDataFixture::new();
        fixture.create_monthly_structure().unwrap();
        
        let template = fixture.monthly_template();
        let dates = extract_dates_from_template(&template);
        
        assert_eq!(dates.len(), 3);
        
        // Verify directories exist
        assert!(fixture.path().join("参照2024_08月データ/Main").exists());
        assert!(fixture.path().join("参照2024_12月データ/Main").exists());
        assert!(fixture.path().join("参照2025_01月データ/Main").exists());
    }
    
    #[test]
    fn test_fixture_nested_structure() {
        let fixture = TestDataFixture::new();
        let nested_root = fixture.create_nested_structure(4).unwrap();
        
        // Check that files exist at different depths
        assert!(nested_root.join("level_1/file_at_depth_1.txt").exists());
        assert!(nested_root.join("level_1/level_2/file_at_depth_2.txt").exists());
        assert!(nested_root.join("level_1/level_2/level_3/file_at_depth_3.txt").exists());
        assert!(nested_root.join("level_1/level_2/level_3/level_4/file_at_depth_4.txt").exists());
    }
    
    #[test] 
    fn test_fixture_timestamped_files() {
        let fixture = TestDataFixture::new();
        let timestamp_dir = fixture.create_timestamped_files().unwrap();
        
        assert!(timestamp_dir.join("small_file.txt").exists());
        assert!(timestamp_dir.join("medium_file.txt").exists());
        assert!(timestamp_dir.join("large_file.txt").exists());
        assert!(timestamp_dir.join("empty_file.txt").exists());
        
        // Verify file sizes
        let metadata = fs::metadata(timestamp_dir.join("medium_file.txt")).unwrap();
        assert_eq!(metadata.len(), 1000);
        
        let empty_metadata = fs::metadata(timestamp_dir.join("empty_file.txt")).unwrap();
        assert_eq!(empty_metadata.len(), 0);
    }
    
    #[test]
    fn test_fixture_invalid_directories() {
        let fixture = TestDataFixture::new();
        fixture.create_invalid_directories().unwrap();
        
        let template = fixture.monthly_template();
        let dates = extract_dates_from_template(&template);
        
        // Should find no valid dates since we only created invalid directories
        assert_eq!(dates.len(), 0);
    }
    
    #[test]
    fn test_fixture_special_char_files() {
        let fixture = TestDataFixture::new();
        let special_dir = fixture.create_special_char_files().unwrap();
        
        assert!(special_dir.join("file with spaces.txt").exists());
        assert!(special_dir.join("file-with-dashes.txt").exists());
        assert!(special_dir.join("ファイル日本語.txt").exists());
    }
}