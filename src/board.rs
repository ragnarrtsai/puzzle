//! 拼圖畫面。
//!
//! 沒有固定的板子，整個畫面就是一張桌子。拼片一開始隨便散、可以互相重疊，
//! 相鄰的兩片靠得夠近就自動扣起來變成一組、之後一起移動，而且不能再拆開。
//! 所有片併成一組就是完成。

use macroquad::prelude::*;

use crate::assets::fit;
use crate::scan::Entry;
use crate::ui::{self, Ui};

/// 上方 HUD 的高度，散落拼片時會避開這一條。
const TOP_BAR: f32 = 64.0;
/// 金光閃爍的總長度（秒）與閃爍次數。
const CELEBRATE_SECS: f32 = 5.0;
const CELEBRATE_FLASHES: f32 = 3.0;
/// 已經對齊的兩片之間允許的誤差，用來判斷「這兩片其實已經貼合了」。
const ALIGNED_EPS: f32 = 1.0;

pub enum Event {
    /// 按了返回，等於棄權，進度直接丟掉。
    Quit,
    /// 最後一片剛扣上。要在這個當下就存檔，不能等金光放完 ——
    /// 否則玩家在閃爍途中按返回或關視窗，這一次就白拼了。
    Solved,
    /// 金光放完了，可以切到欣賞畫面。
    Finished,
}

struct Piece {
    row: usize,
    col: usize,
    /// 左上角在畫面上的座標。
    pos: Vec2,
}

enum Phase {
    Playing,
    /// 拼完了，正在放金光。
    Celebrating(f32),
}

struct Drag {
    group: usize,
    last_mouse: Vec2,
}

pub struct Board {
    rows: usize,
    cols: usize,
    texture: Texture2D,
    pieces: Vec<Piece>,
    /// 每一組的成員。合併後被吃掉的那組會變成 `None`。
    groups: Vec<Option<Vec<usize>>>,
    piece_group: Vec<usize>,
    /// 還活著的組的繪製順序，越後面越上層。
    order: Vec<usize>,
    /// 一片在畫面上的大小。
    piece_size: Vec2,
    /// 一片在來源圖上對應的大小。
    src_size: Vec2,
    snap_tol: f32,
    drag: Option<Drag>,
    phase: Phase,
    /// 完成只回報一次，之後金光每一幀都在跑但不再重複發事件。
    solved_reported: bool,
}

impl Board {
    pub fn new(entry: &Entry, texture: Texture2D) -> Self {
        let (rows, cols) = (entry.rows, entry.cols);
        let n = rows * cols;

        // 整張圖只佔畫面的一部分，其餘空間留給散落的拼片。
        let disp = fit(
            texture.width(),
            texture.height(),
            screen_width() * 0.45,
            (screen_height() - TOP_BAR) * 0.45,
        );
        let piece_size = vec2(disp.x / cols as f32, disp.y / rows as f32);
        let src_size = vec2(
            texture.width() / cols as f32,
            texture.height() / rows as f32,
        );

        // 拼片依 row-major 排，所以 index 就是 row * cols + col。
        let pieces = (0..n)
            .map(|i| Piece {
                row: i / cols,
                col: i % cols,
                pos: random_scatter_pos(piece_size),
            })
            .collect();

        Self {
            rows,
            cols,
            texture,
            pieces,
            groups: (0..n).map(|i| Some(vec![i])).collect(),
            piece_group: (0..n).collect(),
            order: (0..n).collect(),
            piece_size,
            src_size,
            snap_tol: (piece_size.x.min(piece_size.y) * 0.28).max(12.0),
            drag: None,
            phase: Phase::Playing,
            solved_reported: false,
        }
    }

    pub fn update_and_draw(&mut self, ui: &Ui) -> Option<Event> {
        clear_background(ui::BG);

        let back = Rect::new(16.0, 12.0, 120.0, 40.0);
        let peek = Rect::new(screen_width() - 156.0, 12.0, 140.0, 40.0);
        let mouse: Vec2 = mouse_position().into();
        let on_hud = back.contains(mouse) || peek.contains(mouse);

        if matches!(self.phase, Phase::Playing) {
            self.handle_drag(mouse, on_hud);
        }

        let highlight = match &self.drag {
            Some(d) => Some(d.group),
            None if !on_hud && matches!(self.phase, Phase::Playing) => self.group_at(mouse),
            None => None,
        };

        self.draw_pieces();
        if let Some(g) = highlight {
            self.draw_group_outline(g, ui::HOVER);
        }

        // 金光。放完就換欣賞畫面。
        let mut event = None;
        if let Phase::Celebrating(t) = &mut self.phase {
            *t += get_frame_time();
            let t = *t;
            self.draw_celebration(t);
            if !self.solved_reported {
                self.solved_reported = true;
                event = Some(Event::Solved);
            } else if t >= CELEBRATE_SECS {
                event = Some(Event::Finished);
            }
        }

        // HUD 最後畫，才不會被拼片蓋住。
        if ui.button(back, ui.t("返回", "BACK")) {
            return Some(Event::Quit);
        }
        self.draw_peek(ui, peek);
        event
    }

    // ---- 操作 ----

    /// `on_hud` 只擋「開始拖曳」。放開滑鼠一定要處理 ——
    /// 不然游標剛好停在返回鍵或偷看區上面放手時，那一片就永遠扣不上。
    fn handle_drag(&mut self, mouse: Vec2, on_hud: bool) {
        if !on_hud
            && is_mouse_button_pressed(MouseButton::Left)
            && let Some(g) = self.group_at(mouse)
        {
            // 抓起來就移到最上層。
            self.order.retain(|&x| x != g);
            self.order.push(g);
            self.drag = Some(Drag {
                group: g,
                last_mouse: mouse,
            });
        }

        let Some(drag) = &mut self.drag else { return };
        let group = drag.group;

        if is_mouse_button_down(MouseButton::Left) {
            let delta = mouse - drag.last_mouse;
            drag.last_mouse = mouse;
            self.translate_group(group, delta);
            self.clamp_group(group);
        }

        if is_mouse_button_released(MouseButton::Left) {
            self.drag = None;
            self.try_snap(group);
            if self.order.len() == 1 {
                self.phase = Phase::Celebrating(0.0);
            }
        }
    }

    /// 游標下最上層的那一組。
    fn group_at(&self, p: Vec2) -> Option<usize> {
        self.order.iter().rev().copied().find(|&g| {
            self.members(g)
                .iter()
                .any(|&i| self.piece_rect(i).contains(p))
        })
    }

    fn try_snap(&mut self, g: usize) {
        // 先找出最接近的一個合法接合，把整組挪過去對齊。
        let mut best: Option<(f32, Vec2)> = None;
        for &p in self.members(g) {
            for (dr, dc) in NEIGHBORS {
                let Some(q) = self.neighbor(p, dr, dc) else {
                    continue;
                };
                if self.piece_group[q] == g {
                    continue;
                }
                let target = self.pieces[q].pos - self.offset(dr, dc);
                let delta = target - self.pieces[p].pos;
                let d = delta.length();
                if d <= self.snap_tol && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, delta));
                }
            }
        }
        let Some((_, delta)) = best else { return };
        self.translate_group(g, delta);

        // 對齊之後，凡是已經貼合的鄰居全部併進來。合併可能讓更多片變成鄰居，所以要反覆做。
        loop {
            let mut merged = false;
            for &p in &self.members(g).to_vec() {
                for (dr, dc) in NEIGHBORS {
                    let Some(q) = self.neighbor(p, dr, dc) else {
                        continue;
                    };
                    let other = self.piece_group[q];
                    if other == g {
                        continue;
                    }
                    let expected = self.pieces[p].pos + self.offset(dr, dc);
                    if (expected - self.pieces[q].pos).length() <= ALIGNED_EPS {
                        self.merge(g, other);
                        merged = true;
                    }
                }
            }
            if !merged {
                break;
            }
        }
    }

    /// 把 `other` 併進 `keep`。只合併、不分裂，所以被吃掉的那組直接標成 `None`。
    fn merge(&mut self, keep: usize, other: usize) {
        let Some(moved) = self.groups[other].take() else {
            return;
        };
        for &i in &moved {
            self.piece_group[i] = keep;
        }
        self.groups[keep]
            .as_mut()
            .expect("留下來的那組一定還活著")
            .extend(moved);
        self.order.retain(|&x| x != other);
    }

    fn translate_group(&mut self, g: usize, delta: Vec2) {
        for i in self.members(g).to_vec() {
            self.pieces[i].pos += delta;
        }
    }

    /// 不讓整組被拖出畫面外面找不回來，至少留一點在視窗裡。
    fn clamp_group(&mut self, g: usize) {
        let (min, max) = self.group_bounds(g);
        let keep = 40.0_f32.min(self.piece_size.x).min(self.piece_size.y);
        let mut delta = Vec2::ZERO;
        if max.x < keep {
            delta.x = keep - max.x;
        }
        if min.x > screen_width() - keep {
            delta.x = screen_width() - keep - min.x;
        }
        if max.y < TOP_BAR + keep {
            delta.y = TOP_BAR + keep - max.y;
        }
        if min.y > screen_height() - keep {
            delta.y = screen_height() - keep - min.y;
        }
        if delta != Vec2::ZERO {
            self.translate_group(g, delta);
        }
    }

    // ---- 繪製 ----

    fn draw_pieces(&self) {
        for &g in &self.order {
            for &i in self.members(g) {
                let piece = &self.pieces[i];
                draw_texture_ex(
                    &self.texture,
                    piece.pos.x,
                    piece.pos.y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(self.piece_size),
                        source: Some(Rect::new(
                            piece.col as f32 * self.src_size.x,
                            piece.row as f32 * self.src_size.y,
                            self.src_size.x,
                            self.src_size.y,
                        )),
                        ..Default::default()
                    },
                );
            }
        }
    }

    /// 只畫整組的外輪廓 —— 跟同組鄰居相接的那些邊不畫，看起來才像一整塊。
    fn draw_group_outline(&self, g: usize, color: Color) {
        for &i in self.members(g) {
            let r = self.piece_rect(i);
            for (dr, dc) in NEIGHBORS {
                let shared = self
                    .neighbor(i, dr, dc)
                    .is_some_and(|q| self.piece_group[q] == g);
                if shared {
                    continue;
                }
                let (a, b) = match (dr, dc) {
                    (-1, 0) => (vec2(r.x, r.y), vec2(r.x + r.w, r.y)),
                    (1, 0) => (vec2(r.x, r.y + r.h), vec2(r.x + r.w, r.y + r.h)),
                    (0, -1) => (vec2(r.x, r.y), vec2(r.x, r.y + r.h)),
                    _ => (vec2(r.x + r.w, r.y), vec2(r.x + r.w, r.y + r.h)),
                };
                draw_line(a.x, a.y, b.x, b.y, 3.0, color);
            }
        }
    }

    /// 完成的金光：沿著整張圖的外圍疊幾層琥珀色，亮度在五秒內閃三次。
    fn draw_celebration(&self, t: f32) {
        let phase = (t / CELEBRATE_SECS * CELEBRATE_FLASHES * std::f32::consts::TAU).cos();
        let intensity = (1.0 - phase) / 2.0;
        let g = self.order[0];
        let (min, max) = self.group_bounds(g);
        for layer in 0..6 {
            let spread = layer as f32 * 4.0;
            let alpha = intensity * (1.0 - layer as f32 / 6.0) * 0.9;
            draw_rectangle_lines(
                min.x - spread,
                min.y - spread,
                (max.x - min.x) + spread * 2.0,
                (max.y - min.y) + spread * 2.0,
                4.0,
                Color { a: alpha, ..ui::AMBER },
            );
        }
    }

    /// 右上角的偷看區，hover 時把原圖放大顯示當參考。
    fn draw_peek(&self, ui: &Ui, rect: Rect) {
        let hovered = rect.contains(mouse_position().into());
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, ui::PANEL);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.0,
            if hovered { ui::HOVER } else { ui::TEXT_DIM },
        );
        ui.text_centered(
            ui.t("偷看原圖", "PEEK"),
            rect.x + rect.w / 2.0,
            rect.y + rect.h / 2.0 + 7.0,
            20,
            ui::TEXT,
        );
        if !hovered {
            return;
        }

        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.0, 0.0, 0.0, 0.75),
        );
        let size = fit(
            self.texture.width(),
            self.texture.height(),
            screen_width() * 0.7,
            screen_height() * 0.7,
        );
        draw_texture_ex(
            &self.texture,
            (screen_width() - size.x) / 2.0,
            (screen_height() - size.y) / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                ..Default::default()
            },
        );
    }

    /// 除錯用：把每一片直接排到正確位置再走一次真正的扣合流程，
    /// 用來驗證「拼完 → 存檔 → 金光 → 欣賞畫面」這條路確實會通。
    pub fn debug_solve(&mut self) {
        let origin = vec2(140.0, TOP_BAR + 80.0);
        for i in 0..self.pieces.len() {
            let (row, col) = (self.pieces[i].row, self.pieces[i].col);
            self.pieces[i].pos = origin
                + vec2(
                    col as f32 * self.piece_size.x,
                    row as f32 * self.piece_size.y,
                );
        }
        self.try_snap(self.piece_group[0]);
        if self.order.len() == 1 {
            self.phase = Phase::Celebrating(0.0);
        }
    }

    // ---- 小工具 ----

    fn members(&self, g: usize) -> &[usize] {
        self.groups[g].as_deref().unwrap_or(&[])
    }

    fn piece_rect(&self, i: usize) -> Rect {
        let p = &self.pieces[i];
        Rect::new(p.pos.x, p.pos.y, self.piece_size.x, self.piece_size.y)
    }

    /// 相鄰兩片在畫面上「應該」相差多少。因為不能旋轉，這就是單純的位移。
    fn offset(&self, dr: i32, dc: i32) -> Vec2 {
        vec2(
            dc as f32 * self.piece_size.x,
            dr as f32 * self.piece_size.y,
        )
    }

    /// 原圖上位在 `i` 的第 (dr, dc) 個鄰居。
    fn neighbor(&self, i: usize, dr: i32, dc: i32) -> Option<usize> {
        let row = self.pieces[i].row.checked_add_signed(dr as isize)?;
        let col = self.pieces[i].col.checked_add_signed(dc as isize)?;
        (row < self.rows && col < self.cols).then(|| row * self.cols + col)
    }

    fn group_bounds(&self, g: usize) -> (Vec2, Vec2) {
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for &i in self.members(g) {
            let p = self.pieces[i].pos;
            min = min.min(p);
            max = max.max(p + self.piece_size);
        }
        (min, max)
    }
}

const NEIGHBORS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

fn random_scatter_pos(piece: Vec2) -> Vec2 {
    let pad = 8.0;
    vec2(
        rand::gen_range(pad, (screen_width() - piece.x - pad).max(pad + 1.0)),
        rand::gen_range(
            TOP_BAR + pad,
            (screen_height() - piece.y - pad).max(TOP_BAR + pad + 1.0),
        ),
    )
}
