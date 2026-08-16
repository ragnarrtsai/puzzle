//! 共用的畫面元件：配色、字型、按鈕、邊框。

use macroquad::prelude::*;

/// 完成的代表色。選圖畫面已完成的邊框、拼完的金光閃爍都用它。
pub const AMBER: Color = Color::new(1.00, 0.74, 0.25, 1.00);
pub const BG: Color = Color::new(0.10, 0.10, 0.12, 1.00);
pub const PANEL: Color = Color::new(0.16, 0.16, 0.19, 1.00);
pub const TEXT: Color = Color::new(0.90, 0.90, 0.93, 1.00);
pub const TEXT_DIM: Color = Color::new(0.55, 0.55, 0.60, 1.00);
/// hover 到可以搬動的東西時的邊框色。
pub const HOVER: Color = Color::new(0.45, 0.85, 1.00, 1.00);
/// 不能按的東西。
pub const DISABLED: Color = Color::new(0.32, 0.32, 0.36, 1.00);

/// macOS 上找得到的含中文字型。找不到就退回內建字型並改用英文介面。
const CJK_FONT_CANDIDATES: [&str; 2] = [
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/Library/Fonts/Arial Unicode.ttf",
];

pub struct Ui {
    font: Option<Font>,
}

impl Ui {
    pub async fn load() -> Self {
        for path in CJK_FONT_CANDIDATES {
            if let Ok(font) = load_ttf_font(path).await {
                return Self { font: Some(font) };
            }
        }
        eprintln!("找不到中文字型，介面改用英文");
        Self { font: None }
    }

    /// 有沒有載到中文字型。決定介面用中文還是英文。
    pub fn cjk(&self) -> bool {
        self.font.is_some()
    }

    /// 有中文字型就用中文，沒有就用英文。
    pub fn t<'a>(&self, zh: &'a str, en: &'a str) -> &'a str {
        if self.cjk() { zh } else { en }
    }

    pub fn text(&self, s: &str, x: f32, y: f32, size: u16, color: Color) {
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font: self.font.as_ref(),
                font_size: size,
                color,
                ..Default::default()
            },
        );
    }

    /// 以 (x, y) 為基準畫置中對齊的文字，y 是文字底線。
    pub fn text_centered(&self, s: &str, cx: f32, y: f32, size: u16, color: Color) {
        let w = self.measure(s, size);
        self.text(s, cx - w / 2.0, y, size, color);
    }

    pub fn measure(&self, s: &str, size: u16) -> f32 {
        measure_text(s, self.font.as_ref(), size, 1.0).width
    }

    /// 畫一顆按鈕，回傳這一幀是否被按下。hover 時邊框變色。
    pub fn button(&self, rect: Rect, label: &str) -> bool {
        self.button_enabled(rect, label, true)
    }

    /// 同上，但可以畫成不能按的樣子。位置固定、只是變暗，按鈕才不會忽隱忽現跳來跳去。
    pub fn button_enabled(&self, rect: Rect, label: &str, enabled: bool) -> bool {
        let hovered = enabled && rect.contains(mouse_position().into());
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.0,
            match (enabled, hovered) {
                (false, _) => DISABLED,
                (true, true) => HOVER,
                (true, false) => TEXT_DIM,
            },
        );
        let size = 22;
        let ty = rect.y + rect.h / 2.0 + size as f32 * 0.35;
        self.text_centered(
            label,
            rect.x + rect.w / 2.0,
            ty,
            size,
            if enabled { TEXT } else { DISABLED },
        );
        hovered && is_mouse_button_pressed(MouseButton::Left)
    }
}
