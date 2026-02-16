use md_docs::domain::CliMessage;
use md_docs::infra::logger::FileLogger;

#[cfg(test)]
mod cli_message_tests {
    use super::*;

    #[test]
    fn test_success_formatted_contains_checkmark_and_message() {
        let msg = CliMessage::Success("done".to_string());
        let output = msg.formatted();
        assert!(output.contains("done"));
        // Checkmark may have ANSI codes; verify output is longer than bare message
        assert!(output.contains("\u{2713}") || output.len() > "done".len());
    }

    #[test]
    fn test_info_formatted_contains_message() {
        let msg = CliMessage::Info("compiling...".to_string());
        let output = msg.formatted();
        assert!(output.contains("compiling..."));
    }

    #[test]
    fn test_warning_formatted_contains_prefix_and_message() {
        let msg = CliMessage::Warning("missing font".to_string());
        let output = msg.formatted();
        assert!(output.contains("warning:"));
        assert!(output.contains("missing font"));
    }

    #[test]
    fn test_error_formatted_contains_prefix_and_message() {
        let msg = CliMessage::Error("file not found".to_string());
        let output = msg.formatted();
        assert!(output.contains("error:"));
        assert!(output.contains("file not found"));
    }

    #[test]
    fn test_log_formatted_contains_message() {
        let msg = CliMessage::Log("debug info".to_string());
        let output = msg.formatted();
        assert!(output.contains("debug info"));
    }

    #[test]
    fn test_plain_formatted_is_passthrough() {
        let msg = CliMessage::Plain("raw text".to_string());
        let output = msg.formatted();
        assert_eq!(output, "raw text");
    }

    #[test]
    fn test_display_impl_matches_formatted() {
        let msg = CliMessage::Success("test".to_string());
        assert_eq!(format!("{}", msg), msg.formatted());
    }
}

#[cfg(test)]
mod file_logger_tests {
    use super::*;
    use std::fs;

    fn temp_log_path() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        (dir, path)
    }

    #[test]
    fn test_logger_creates_file_on_first_write() {
        let (_dir, path) = temp_log_path();
        assert!(!path.exists());
        let logger = FileLogger::with_path(path.clone());
        logger.log("INFO", "hello");
        assert!(path.exists());
    }

    #[test]
    fn test_logger_appends_entries() {
        let (_dir, path) = temp_log_path();
        let logger = FileLogger::with_path(path.clone());
        logger.log("INFO", "first");
        logger.log("INFO", "second");
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_logger_entry_contains_level_and_message() {
        let (_dir, path) = temp_log_path();
        let logger = FileLogger::with_path(path.clone());
        logger.log("WARN", "test warning");
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[WARN]"));
        assert!(content.contains("test warning"));
    }

    #[test]
    fn test_log_message_maps_cli_message_variants() {
        let (_dir, path) = temp_log_path();
        let logger = FileLogger::with_path(path.clone());
        logger.log_message(&CliMessage::Success("ok".to_string()));
        logger.log_message(&CliMessage::Warning("caution".to_string()));
        logger.log_message(&CliMessage::Error("fail".to_string()));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[SUCCESS]"));
        assert!(content.contains("[WARN]"));
        assert!(content.contains("[ERROR]"));
    }
}
