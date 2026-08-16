//! 掃描來源資料夾。
//!
//! 目錄格式固定為 `<root>/<name>/<高_寬>/圖片檔`，例如 `images/貓/3_4/tabby.jpg`
//! 代表這張圖要切成 3 列 4 行、共 12 片。
//!
//! `<高_寬>` 那層寫錯就整個不讀，不提示也不給預設值。

use std::path::{Path, PathBuf};

/// 一個可以挑戰的關卡：一張圖 + 一種切法。
///
/// 同一張圖放在不同的 `<高_寬>` 資料夾下算兩個不同的關卡。
#[derive(Clone, Debug)]
pub struct Entry {
    /// 存檔用的識別字串，格式為 `<列>_<行>|<相對於 root 的路徑>`。
    pub id: String,
    pub path: PathBuf,
    /// `<name>` 那層的圖集名。目前只用來排序，不顯示在畫面上。
    pub set_name: String,
    pub rows: usize,
    pub cols: usize,
}

impl Entry {
    pub fn piece_count(&self) -> usize {
        self.rows * self.cols
    }

    /// 顯示用的「幾乘幾」，沿用資料夾的高×寬順序。
    pub fn grid_label(&self) -> String {
        format!("{}x{}", self.rows, self.cols)
    }
}

const IMAGE_EXTS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

/// 掃描整個來源資料夾。開遊戲時呼叫一次，之後不再重掃。
pub fn scan(root: &Path) -> Vec<Entry> {
    let mut entries = Vec::new();

    for set_dir in sorted_subdirs(root) {
        let set_name = match set_dir.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        for grid_dir in sorted_subdirs(&set_dir) {
            // 資料夾名稱不是合法的 `<高_寬>` 就跳過整個資料夾。
            let Some((rows, cols)) = grid_dir
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(parse_grid)
            else {
                continue;
            };

            for image in sorted_files(&grid_dir) {
                if !has_image_ext(&image) {
                    continue;
                }
                let rel = image.strip_prefix(root).unwrap_or(&image);
                entries.push(Entry {
                    id: format!("{rows}_{cols}|{}", rel.display()),
                    path: image,
                    set_name: set_name.clone(),
                    rows,
                    cols,
                });
            }
        }
    }

    // 選圖畫面依難度（片數）分區，同一區裡再依圖集、檔名排。
    entries.sort_by(|a, b| {
        a.piece_count()
            .cmp(&b.piece_count())
            .then_with(|| a.rows.cmp(&b.rows))
            .then_with(|| a.set_name.cmp(&b.set_name))
            .then_with(|| a.path.cmp(&b.path))
    });
    entries
}

/// 解析 `3_4` 這種資料夾名，回傳 (列, 行)。底線分隔，列在前、行在後。
fn parse_grid(name: &str) -> Option<(usize, usize)> {
    let (rows, cols) = name.split_once('_')?;
    let rows: usize = rows.parse().ok()?;
    let cols: usize = cols.parse().ok()?;
    // 1x1 不算拼圖，至少要能切成兩片。
    if rows == 0 || cols == 0 || rows * cols < 2 {
        return None;
    }
    Some((rows, cols))
}

fn has_image_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| IMAGE_EXTS.contains(&e.as_str()))
}

fn sorted_subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = read_dir(dir).filter(|p| p.is_dir()).collect();
    out.sort();
    out
}

fn sorted_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = read_dir(dir).filter(|p| p.is_file()).collect();
    out.sort();
    out
}

fn read_dir(dir: &Path) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 依 `<圖集>/<格數資料夾>/<檔名>` 的清單搭出一個暫時的來源資料夾。
    fn fixture(name: &str, files: &[(&str, &str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("puzzle-scan-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        for (set, grid, file) in files {
            let dir = root.join(set).join(grid);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(file), b"not a real image").unwrap();
        }
        root
    }

    #[test]
    fn parses_rows_then_cols() {
        assert_eq!(parse_grid("3_4"), Some((3, 4)));
        assert_eq!(parse_grid("4_3"), Some((4, 3)));
    }

    #[test]
    fn rejects_malformed_grid_names() {
        for bad in ["亂寫", "3x4", "3_", "_4", "3_4_5", "0_5", "1_1", ""] {
            assert_eq!(parse_grid(bad), None, "{bad} 不該被接受");
        }
    }

    #[test]
    fn skips_folders_with_bad_grid_names() {
        let root = fixture(
            "bad-grid",
            &[("貓", "3_4", "a.png"), ("貓", "亂寫", "b.png")],
        );
        let found = scan(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].grid_label(), "3x4");
    }

    #[test]
    fn skips_non_image_files() {
        let root = fixture(
            "non-image",
            &[("貓", "2_2", "a.png"), ("貓", "2_2", "notes.txt")],
        );
        assert_eq!(scan(&root).len(), 1);
    }

    #[test]
    fn sorts_by_piece_count_so_pages_group_by_difficulty() {
        let root = fixture(
            "sorting",
            &[
                ("b", "4_6", "big.png"),
                ("a", "2_2", "small.png"),
                ("a", "3_4", "mid.png"),
                ("a", "4_3", "mid2.png"),
            ],
        );
        let counts: Vec<usize> = scan(&root).iter().map(Entry::piece_count).collect();
        assert_eq!(counts, vec![4, 12, 12, 24]);
    }

    #[test]
    fn same_image_in_two_grids_is_two_entries_with_distinct_ids() {
        let root = fixture(
            "dup",
            &[("貓", "3_4", "cat.png"), ("貓", "5_6", "cat.png")],
        );
        let found = scan(&root);
        assert_eq!(found.len(), 2);
        assert_ne!(found[0].id, found[1].id);
    }
}
