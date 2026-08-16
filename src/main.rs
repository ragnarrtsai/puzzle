//! 一個本機拼圖小遊戲：玩家把自己的圖片放進來源資料夾，遊戲就把它們變成拼圖。
//!
//! 來源資料夾預設是工作目錄下的 `images/`，可以用 `PUZZLE_IMAGES` 環境變數指定別的位置。
//! 目錄格式是 `<圖集>/<高_寬>/圖片檔`，例如 `images/貓/3_4/tabby.jpg` 就是 12 片。

mod assets;
mod board;
mod save;
mod scan;
mod select;
mod ui;
mod view;

use std::path::PathBuf;

use macroquad::prelude::*;

use assets::Textures;
use board::Board;
use save::Completed;
use select::Select;
use ui::Ui;

enum Screen {
    Select,
    Playing { entry: usize, board: Board },
    Viewing { entry: usize },
}

fn window() -> Conf {
    Conf {
        window_title: "Puzzle".to_owned(),
        window_width: 1280,
        window_height: 800,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window)]
async fn main() {
    // 每次開遊戲拼片散落的位置都不一樣。
    rand::srand(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    );

    let root = PathBuf::from(
        std::env::var("PUZZLE_IMAGES").unwrap_or_else(|_| "images".to_string()),
    );
    let entries = scan::scan(&root);
    println!("從 {} 掃到 {} 個關卡", root.display(), entries.len());

    let ui = Ui::load().await;
    let mut textures = Textures::default();
    let mut completed = Completed::load(save::save_path(&root));

    let done = entries.iter().filter(|e| completed.contains(&e.id)).count();
    println!("其中 {done} 個已完成");

    let shot = std::env::var("PUZZLE_SHOT").ok();
    let mut frame = 0u32;
    let mut shots = 0u32;

    let mut select = Select::new(&entries, &root);
    let mut screen = Screen::Select;

    // 除錯用：直接開第一個關卡並自動拼完，驗證完成到存檔那條路。
    let selftest = std::env::var("PUZZLE_SELFTEST").is_ok();
    if selftest && let Some(texture) = textures.get(&entries[0].path) {
        let mut board = Board::new(&entries[0], texture.clone());
        board.debug_solve();
        screen = Screen::Playing { entry: 0, board };
    }

    loop {
        match &mut screen {
            Screen::Select => {
                if let Some(index) =
                    select.update_and_draw(&ui, &mut textures, &entries, &completed)
                {
                    let entry = &entries[index];
                    // 已完成的點進去是欣賞，不是重玩。
                    screen = if completed.contains(&entry.id) {
                        Screen::Viewing { entry: index }
                    } else if let Some(texture) = textures.get(&entry.path) {
                        Screen::Playing {
                            entry: index,
                            board: Board::new(entry, texture.clone()),
                        }
                    } else {
                        Screen::Select
                    };
                }
            }

            Screen::Playing { entry, board } => match board.update_and_draw(&ui) {
                // 返回等於棄權，拼到一半的進度不留。
                Some(board::Event::Quit) => screen = Screen::Select,
                // 一拼完就先存檔，金光還在閃的時候關掉也算數。
                Some(board::Event::Solved) => completed.mark(&entries[*entry].id),
                Some(board::Event::Finished) => screen = Screen::Viewing { entry: *entry },
                None => {}
            },

            Screen::Viewing { entry } => {
                let index = *entry;
                match textures.get(&entries[index].path).cloned() {
                    None => screen = Screen::Select,
                    Some(texture) => match view::update_and_draw(&ui, &texture) {
                        view::Action::Back => screen = Screen::Select,
                        // 重置：清掉完成紀錄，然後直接重拼一次。
                        view::Action::Reset => {
                            completed.unmark(&entries[index].id);
                            screen = Screen::Playing {
                                entry: index,
                                board: Board::new(&entries[index], texture),
                            };
                        }
                        view::Action::None => {}
                    },
                }
            }
        }

        // 除錯用：設了 PUZZLE_SHOT 就把選圖畫面每一頁截圖存檔然後結束。
        if let Some(prefix) = &shot {
            // 自測要涵蓋五秒金光，所以按時間截；翻頁模式跟畫面內容無關，按幀數就好。
            let due = if selftest {
                get_time() >= f64::from(shots) + 1.0
            } else {
                frame += 1;
                frame.is_multiple_of(15)
            };
            if due {
                shots += 1;
                let path = format!("{prefix}-{shots}.png");
                get_screen_data().export_png(&path);
                println!("截圖 {path}（t={:.1}s）", get_time());
                let done = if selftest { shots >= 8 } else { !select.next_page() };
                if done {
                    return;
                }
            }
        }

        next_frame().await;
    }
}
