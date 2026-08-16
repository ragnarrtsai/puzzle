//! 選圖畫面。
//!
//! 依難度分區，難度就是拼片總數。每頁橫向四格，每格放完整原圖並標上幾乘幾。
//! 已完成的圖邊框是琥珀色，點進去是欣賞而不是重玩。

use std::path::Path;

use macroquad::prelude::*;

use crate::assets::{Textures, fit};
use crate::save::Completed;
use crate::scan::Entry;
use crate::ui::{self, Ui};

const COLS: usize = 4;
const ROWS: usize = 2;
const PER_PAGE: usize = COLS * ROWS;

struct Page {
    piece_count: usize,
    entries: Vec<usize>,
    /// 這是該難度的第幾頁、共幾頁。
    nth: usize,
    total: usize,
}

/// 難度總覽上的一項：一個片數，以及它的第一頁在 `pages` 裡的位置。
struct Difficulty {
    piece_count: usize,
    first_page: usize,
}

pub struct Select {
    pages: Vec<Page>,
    difficulties: Vec<Difficulty>,
    current: usize,
    /// 掃不到任何關卡時，要在畫面上告訴玩家該去哪裡放圖。
    root: String,
    root_exists: bool,
}

impl Select {
    /// `entries` 已經依片數排好序，所以連續相同片數的就是同一個難度區。
    pub fn new(entries: &[Entry], root: &Path) -> Self {
        let mut pages = Vec::new();
        let mut difficulties = Vec::new();
        let mut start = 0;
        while start < entries.len() {
            let count = entries[start].piece_count();
            let end = entries[start..]
                .iter()
                .position(|e| e.piece_count() != count)
                .map_or(entries.len(), |offset| start + offset);

            let chunks: Vec<Vec<usize>> = (start..end)
                .collect::<Vec<_>>()
                .chunks(PER_PAGE)
                .map(<[usize]>::to_vec)
                .collect();
            let total = chunks.len();
            difficulties.push(Difficulty {
                piece_count: count,
                first_page: pages.len(),
            });
            for (nth, entries) in chunks.into_iter().enumerate() {
                pages.push(Page {
                    piece_count: count,
                    entries,
                    nth: nth + 1,
                    total,
                });
            }
            start = end;
        }
        Self {
            pages,
            difficulties,
            current: 0,
            root: root.display().to_string(),
            root_exists: root.is_dir(),
        }
    }

    /// 翻到下一頁，已經是最後一頁就回傳 false。除錯截圖用。
    pub fn next_page(&mut self) -> bool {
        if self.current + 1 >= self.pages.len() {
            return false;
        }
        self.current += 1;
        true
    }

    /// 回傳這一幀被點下去的關卡。
    pub fn update_and_draw(
        &mut self,
        ui: &Ui,
        textures: &mut Textures,
        entries: &[Entry],
        completed: &Completed,
    ) -> Option<usize> {
        clear_background(ui::BG);

        if self.pages.is_empty() {
            self.draw_empty(ui);
            return None;
        }

        let picked = draw_grid(ui, textures, entries, completed, &self.pages[self.current]);
        self.draw_difficulty_bar(ui);
        self.draw_pager(ui);
        picked
    }

    /// 難度總覽。把所有存在的難度一次列出來，目前這個底下畫琥珀色底線，點了直接跳過去。
    /// 沒有這一排的話，玩家不會知道別的難度還有東西。
    fn draw_difficulty_bar(&mut self, ui: &Ui) {
        let current_count = self.pages[self.current].piece_count;
        let size = 26;
        let gap = 30.0;
        let y = 50.0;

        let labels: Vec<String> = self
            .difficulties
            .iter()
            .map(|d| difficulty_label(ui, d.piece_count))
            .collect();
        let widths: Vec<f32> = labels.iter().map(|l| ui.measure(l, size)).collect();
        let total: f32 = widths.iter().sum::<f32>() + gap * (labels.len() - 1) as f32;

        let mut x = (screen_width() - total) / 2.0;
        let mut jump = None;
        for (i, difficulty) in self.difficulties.iter().enumerate() {
            let w = widths[i];
            let hit = Rect::new(x - 10.0, y - 26.0, w + 20.0, 38.0);
            let hovered = hit.contains(mouse_position().into());
            let active = difficulty.piece_count == current_count;

            let color = match (hovered, active) {
                (true, _) => ui::HOVER,
                (false, true) => ui::TEXT,
                (false, false) => ui::TEXT_DIM,
            };
            ui.text(&labels[i], x, y, size, color);
            if active {
                draw_line(x, y + 9.0, x + w, y + 9.0, 3.0, ui::AMBER);
            }
            if hovered && is_mouse_button_pressed(MouseButton::Left) {
                jump = Some(difficulty.first_page);
            }
            x += w + gap;
        }

        if let Some(page) = jump {
            self.current = page;
        }
    }

    /// 同一個難度內的翻頁。按鈕位置固定，不能按的時候只是變暗，不會忽隱忽現。
    fn draw_pager(&mut self, ui: &Ui) {
        let page = &self.pages[self.current];
        if page.total > 1 {
            let sub = format!("{} / {}", page.nth, page.total);
            ui.text_centered(&sub, screen_width() / 2.0, screen_height() - 66.0, 18, ui::TEXT_DIM);
        }
        if self.pages.len() == 1 {
            return;
        }

        let y = screen_height() - 52.0;
        let prev = Rect::new(screen_width() / 2.0 - 150.0, y, 120.0, 40.0);
        let next = Rect::new(screen_width() / 2.0 + 30.0, y, 120.0, 40.0);
        let has_prev = self.current > 0;
        let has_next = self.current + 1 < self.pages.len();

        if ui.button_enabled(prev, ui.t("上一頁", "PREV"), has_prev) {
            self.current -= 1;
        }
        if ui.button_enabled(next, ui.t("下一頁", "NEXT"), has_next) {
            self.current += 1;
        }
    }

    /// 一個關卡都沒有的時候，把「該怎麼放圖」直接寫在畫面上，
    /// 而不是丟一句「找不到」讓玩家自己猜。
    fn draw_empty(&self, ui: &Ui) {
        let cx = screen_width() / 2.0;
        let left = cx - 300.0;
        let mut y = screen_height() / 2.0 - 190.0;

        ui.text_centered(
            ui.t("找不到任何拼圖", "NO PUZZLES FOUND"),
            cx,
            y,
            34,
            ui::TEXT,
        );
        y += 46.0;

        let reason = if self.root_exists {
            ui.t(
                "來源資料夾在，但裡面沒有合法的關卡：",
                "the source folder exists but has no valid puzzles:",
            )
        } else {
            ui.t("找不到來源資料夾：", "source folder not found:")
        };
        ui.text_centered(reason, cx, y, 20, ui::TEXT_DIM);
        y += 28.0;
        ui.text_centered(&self.root, cx, y, 20, ui::AMBER);
        y += 48.0;

        ui.text(
            ui.t(
                "把圖片照這個格式放進去，然後重開遊戲：",
                "put your images in this layout, then restart:",
            ),
            left,
            y,
            22,
            ui::TEXT,
        );
        y += 40.0;
        ui.text(
            ui.t(
                "images/<圖集>/<高_寬>/圖片檔",
                "images/<set>/<rows_cols>/image-file",
            ),
            left + 24.0,
            y,
            24,
            ui::AMBER,
        );
        y += 34.0;
        ui.text(
            ui.t("例如   images/貓/3_4/tabby.jpg", "e.g.   images/cats/3_4/tabby.jpg"),
            left + 24.0,
            y,
            22,
            ui::TEXT_DIM,
        );
        y += 46.0;

        let notes = [
            ui.t(
                "3_4 是 3 列 4 行、共 12 片。底線分隔，列數在前、行數在後。",
                "3_4 means 3 rows and 4 columns = 12 pieces. rows first, then columns.",
            ),
            ui.t(
                "圖集名稱隨你取，只是用來分類，遊戲裡不會顯示。",
                "the set name is yours to pick; it only groups files.",
            ),
            ui.t(
                "格數那層寫錯的資料夾會被整個略過。",
                "folders with an unparsable grid name are skipped entirely.",
            ),
            ui.t(
                "支援 jpg / jpeg / png / webp。",
                "supported formats: jpg / jpeg / png / webp.",
            ),
        ];
        for note in notes {
            ui.text("-", left + 24.0, y, 20, ui::TEXT_DIM);
            ui.text(note, left + 44.0, y, 20, ui::TEXT_DIM);
            y += 30.0;
        }

        y += 22.0;
        ui.text(
            ui.t(
                "也可以用環境變數 PUZZLE_IMAGES 指定別的資料夾。",
                "or point PUZZLE_IMAGES at a different folder.",
            ),
            left,
            y,
            20,
            ui::TEXT_DIM,
        );
    }
}

fn difficulty_label(ui: &Ui, piece_count: usize) -> String {
    if ui.cjk() {
        format!("{piece_count} 片")
    } else {
        format!("{piece_count}P")
    }
}

fn draw_grid(
    ui: &Ui,
    textures: &mut Textures,
    entries: &[Entry],
    completed: &Completed,
    page: &Page,
) -> Option<usize> {
    let pad = 24.0;
    let top = 96.0;
    let bottom = screen_height() - 76.0;
    let cell_w = (screen_width() - pad * (COLS as f32 + 1.0)) / COLS as f32;
    let cell_h = (bottom - top - pad * (ROWS as f32 - 1.0)) / ROWS as f32;
    let label_h = 30.0;

    let mut picked = None;
    for (slot, &index) in page.entries.iter().enumerate() {
        let entry = &entries[index];
        let cell = Rect::new(
            pad + (slot % COLS) as f32 * (cell_w + pad),
            top + (slot / COLS) as f32 * (cell_h + pad),
            cell_w,
            cell_h,
        );
        let hovered = cell.contains(mouse_position().into());
        let done = completed.contains(&entry.id);

        draw_rectangle(cell.x, cell.y, cell.w, cell.h, ui::PANEL);

        if let Some(texture) = textures.get(&entry.path) {
            let area = vec2(cell.w - 16.0, cell.h - label_h - 12.0);
            let size = fit(texture.width(), texture.height(), area.x, area.y);
            draw_texture_ex(
                texture,
                cell.x + (cell.w - size.x) / 2.0,
                cell.y + 8.0 + (area.y - size.y) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(size),
                    ..Default::default()
                },
            );
        }

        // 已完成是琥珀色，hover 蓋過它，兩者都沒有就是暗色。
        let border = match (hovered, done) {
            (true, _) => ui::HOVER,
            (false, true) => ui::AMBER,
            (false, false) => ui::TEXT_DIM,
        };
        draw_rectangle_lines(cell.x, cell.y, cell.w, cell.h, 3.0, border);

        ui.text_centered(
            &entry.grid_label(),
            cell.x + cell.w / 2.0,
            cell.y + cell.h - 9.0,
            22,
            if done { ui::AMBER } else { ui::TEXT },
        );

        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            picked = Some(index);
        }
    }
    picked
}
