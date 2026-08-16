//! 存檔。只記錄哪些關卡已經拼完，不存拼到一半的進度。
//!
//! 格式就是純文字，一行一個 `Entry::id`。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct Completed {
    file: PathBuf,
    done: HashSet<String>,
}

impl Completed {
    pub fn load(file: PathBuf) -> Self {
        let done = std::fs::read_to_string(&file)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        Self { file, done }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.done.contains(id)
    }

    pub fn mark(&mut self, id: &str) {
        if !self.done.insert(id.to_string()) {
            return;
        }
        self.write();
    }

    /// 重置一個已完成的關卡，讓它變回沒拼過的樣子。
    pub fn unmark(&mut self, id: &str) {
        if !self.done.remove(id) {
            return;
        }
        self.write();
    }

    fn write(&self) {
        let mut lines: Vec<&str> = self.done.iter().map(String::as_str).collect();
        lines.sort_unstable();
        if let Err(e) = std::fs::write(&self.file, lines.join("\n") + "\n") {
            eprintln!("存檔寫入失敗 {}: {e}", self.file.display());
        }
    }
}

/// 存檔放在來源資料夾旁邊，這樣整包搬走進度也跟著走。
pub fn save_path(root: &Path) -> PathBuf {
    root.with_file_name(format!(
        "{}.completed",
        root.file_name().and_then(|n| n.to_str()).unwrap_or("puzzle")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("puzzle-save-{name}"));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn survives_restart() {
        let path = temp_file("restart");

        let mut first = Completed::load(path.clone());
        assert!(!first.contains("3_4|貓/a.png"));
        first.mark("3_4|貓/a.png");
        drop(first);

        // 重開遊戲：重新從檔案讀一次，紀錄要還在。
        let second = Completed::load(path.clone());
        assert!(second.contains("3_4|貓/a.png"));
        assert!(!second.contains("5_6|貓/a.png"));
    }

    #[test]
    fn reset_makes_a_level_unplayed_again() {
        let path = temp_file("reset");

        let mut save = Completed::load(path.clone());
        save.mark("3_4|a/x.png");
        save.mark("2_2|a/y.png");
        save.unmark("3_4|a/x.png");
        drop(save);

        let reloaded = Completed::load(path);
        assert!(!reloaded.contains("3_4|a/x.png"), "重置後不該還算完成");
        assert!(reloaded.contains("2_2|a/y.png"), "不該動到別的關卡");
    }

    #[test]
    fn keeps_earlier_entries_when_marking_more() {
        let path = temp_file("accumulate");

        let mut save = Completed::load(path.clone());
        save.mark("2_2|a/x.png");
        save.mark("3_4|b/y.png");
        save.mark("2_2|a/x.png"); // 重複標記不該弄丟東西
        drop(save);

        let reloaded = Completed::load(path);
        assert!(reloaded.contains("2_2|a/x.png"));
        assert!(reloaded.contains("3_4|b/y.png"));
    }

    #[test]
    fn save_file_sits_next_to_the_source_folder() {
        assert_eq!(
            save_path(Path::new("images")),
            PathBuf::from("images.completed")
        );
        assert_eq!(
            save_path(Path::new("/photos/puzzles")),
            PathBuf::from("/photos/puzzles.completed")
        );
    }
}
