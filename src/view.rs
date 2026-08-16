//! 欣賞畫面。把那張圖完整地看一遍。
//!
//! 兩個入口：剛拼完金光放完之後，以及從選圖畫面點已完成的圖。

use macroquad::prelude::*;

use crate::assets::fit;
use crate::ui::{self, Ui};

pub enum Action {
    None,
    Back,
    /// 把這張圖變回沒拼過的狀態，然後馬上重拼一次。
    Reset,
}

pub fn update_and_draw(ui: &Ui, texture: &Texture2D) -> Action {
    clear_background(ui::BG);

    let size = fit(
        texture.width(),
        texture.height(),
        screen_width() * 0.82,
        screen_height() * 0.82,
    );
    let pos = vec2(
        (screen_width() - size.x) / 2.0,
        (screen_height() - size.y) / 2.0,
    );
    draw_texture_ex(
        texture,
        pos.x,
        pos.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(size),
            ..Default::default()
        },
    );
    draw_rectangle_lines(pos.x, pos.y, size.x, size.y, 3.0, ui::AMBER);

    if ui.button(Rect::new(16.0, 12.0, 120.0, 40.0), ui.t("返回", "BACK")) {
        return Action::Back;
    }
    if ui.button(
        Rect::new(screen_width() - 136.0, 12.0, 120.0, 40.0),
        ui.t("重置", "RESET"),
    ) {
        return Action::Reset;
    }
    Action::None
}
