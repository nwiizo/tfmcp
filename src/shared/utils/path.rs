use dirs::home_dir;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub fn expand_tilde(path: &str) -> PathBuf {
    if !path.starts_with('~') {
        return PathBuf::from(path);
    }

    if let Some(home) = home_dir() {
        if path == "~" {
            return home;
        }

        if let Some(rest) = path.strip_prefix("~/") {
            return home.join(rest);
        }
    }

    PathBuf::from(path)
}

#[allow(dead_code)]
pub fn contract_tilde(path: &Path) -> String {
    if let Some(home) = home_dir() {
        let home_str = home.to_string_lossy();
        let path_str = path.to_string_lossy();

        if path_str == home_str {
            return "~".to_string();
        }

        // Create a path object for comparing to handle different path separators
        let path_buf = PathBuf::from(path_str.as_ref());
        
        // Check if path_buf starts with home directory
        if let Ok(relative) = path_buf.strip_prefix(&home) {
            if relative.as_os_str().is_empty() {
                return "~".to_string();
            } else {
                // Convert to forward slashes for consistency across platforms
                let relative_str = relative.to_string_lossy().replace('\\', "/");
                return format!("~/{}", relative_str);
            }
        }
        
        path_str.into_owned()
    } else {
        path.to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_expand_tilde_no_tilde() {
        let path = "/usr/bin";
        let expanded = expand_tilde(path);
        assert_eq!(expanded, PathBuf::from("/usr/bin"));
    }

    #[test]
    fn test_expand_tilde_home_only() {
        let expanded = expand_tilde("~");
        let home = home_dir().unwrap();
        assert_eq!(expanded, home);
    }

    #[test]
    fn test_expand_tilde_home_slash() {
        let expanded = expand_tilde("~/Documents");
        let home = home_dir().unwrap();
        assert_eq!(expanded, home.join("Documents"));
    }

    #[test]
    fn test_expand_tilde_user_not_supported() {
        let expanded = expand_tilde("~username/bin");
        assert_eq!(expanded, PathBuf::from("~username/bin"));
    }

    #[test]
    fn test_contract_tilde_outside_home() {
        let path = Path::new("/var/log");
        let contracted = contract_tilde(path);
        assert_eq!(contracted, "/var/log");
    }

    #[test]
    fn test_contract_tilde_exact_home() {
        let home = home_dir().unwrap();
        let contracted = contract_tilde(&home);
        assert_eq!(contracted, "~");
    }

    #[test]
    fn test_contract_tilde_home_subdir() {
        let home = home_dir().unwrap();
        let sub_path = home.join("Pictures");
        let contracted = contract_tilde(&sub_path);
        assert_eq!(contracted, "~/Pictures");
    }
}
