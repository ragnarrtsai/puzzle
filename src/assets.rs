//! 圖片載入與快取。
//!
//! 拼片不會真的把圖切成很多張，而是共用同一張 texture、畫的時候各自指定來源矩形，
//! 所以一個關卡只需要一張 texture。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use macroquad::prelude::*;

/// 載入時縮到長邊不超過這個尺寸。玩家可能丟 6000px 的原圖進來，
/// 但畫面根本放不下，先縮小可以省下大量記憶體。
const MAX_DIM: u32 = 2048;

#[derive(Default)]
pub struct Textures {
    cache: HashMap<PathBuf, Option<Texture2D>>,
}

impl Textures {
    /// 取得（必要時載入）一張圖。載入失敗會記成 `None`，之後不再重試。
    pub fn get(&mut self, path: &Path) -> Option<&Texture2D> {
        if !self.cache.contains_key(path) {
            let loaded = load(path);
            if loaded.is_none() {
                eprintln!("讀不到圖片，略過：{}", path.display());
            }
            self.cache.insert(path.to_path_buf(), loaded);
        }
        self.cache[path].as_ref()
    }
}

fn load(path: &Path) -> Option<Texture2D> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;

    let (w, h) = (img.width(), img.height());
    let img = if w.max(h) > MAX_DIM {
        let scale = MAX_DIM as f32 / w.max(h) as f32;
        let (nw, nh) = (
            ((w as f32 * scale).round() as u32).max(1),
            ((h as f32 * scale).round() as u32).max(1),
        );
        img.resize_exact(nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img
    };

    let rgba = img.to_rgba8();
    let texture = Texture2D::from_rgba8(rgba.width() as u16, rgba.height() as u16, &rgba);
    texture.set_filter(FilterMode::Linear);
    Some(texture)
}

/// 把一個 w×h 的東西等比例縮放塞進 `max_w`×`max_h`，回傳縮放後的尺寸。
pub fn fit(w: f32, h: f32, max_w: f32, max_h: f32) -> Vec2 {
    let scale = (max_w / w).min(max_h / h);
    vec2(w * scale, h * scale)
}
