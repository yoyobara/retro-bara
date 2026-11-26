#![no_std]
#![no_main]

extern crate alloc;

mod flasher;
mod games;
mod meta;
mod ui;

use alloc::string::ToString;
use alloc::vec::Vec;

use board::{BoardScreen, Button, Buttons};
use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use log::{error, info};

use crate::games::{Fs, Game, ICON_LEN, NoClock};
use crate::ui::VISIBLE_ROWS;

esp_app_desc!();

const POLL_INTERVAL_MS: u32 = 10;
const LAUNCH_BUTTONS: &[Button] = &[Button::A, Button::Start];
const DETAILS_BUTTONS: &[Button] = &[Button::B, Button::Select];

#[derive(Clone, Copy)]
enum Mode {
    List,
    Details,
}

struct Launcher {
    buttons: Buttons,
    screen: BoardScreen,
    flash: esp_storage::FlashStorage<'static>,
    fs: Fs,
    games: Vec<Game>,
    selected: usize,
    /// Index of the game shown in the first row.
    top: usize,
    icon_buf: [u8; ICON_LEN],
}

#[esp_hal::main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let board::Board {
        buttons,
        screen,
        sd_card,
        flash,
    } = board::Board::new(peripherals);

    let mut launcher = Launcher {
        buttons,
        screen,
        flash,
        fs: Fs::new(sd_card, NoClock),
        games: Vec::new(),
        selected: 0,
        top: 0,
        icon_buf: [0; ICON_LEN],
    };
    launcher.run()
}

impl Launcher {
    fn run(&mut self) -> ! {
        self.load_games();
        self.draw_list();

        let mut mode = Mode::List;
        loop {
            Delay::new().delay_millis(POLL_INTERVAL_MS);
            let pressed = self.buttons.update().pressed;

            mode = match mode {
                _ if pressed.contains_any(LAUNCH_BUTTONS) => {
                    self.launch();
                    Mode::List
                }
                Mode::List if pressed.contains_any(DETAILS_BUTTONS) => {
                    ui::draw_modal(&mut self.screen, &self.games[self.selected].meta);
                    Mode::Details
                }
                Mode::List => {
                    if pressed.is_pressed(Button::Up) && self.selected > 0 {
                        self.select(self.selected - 1);
                    } else if pressed.is_pressed(Button::Down)
                        && self.selected + 1 < self.games.len()
                    {
                        self.select(self.selected + 1);
                    }
                    Mode::List
                }
                Mode::Details if pressed.any() => {
                    self.draw_list();
                    Mode::List
                }
                Mode::Details => Mode::Details,
            };
        }
    }

    /// Scans the SD card until at least one game is found.
    fn load_games(&mut self) {
        loop {
            match games::scan(&self.fs) {
                Ok(games) if !games.is_empty() => {
                    info!("{} games found", games.len());
                    self.games = games;
                    self.selected = 0;
                    self.top = 0;
                    return;
                }
                Ok(_) => ui::draw_message(
                    &mut self.screen,
                    &[
                        "No games",
                        "Put games in /GAMES on the SD card",
                        "Press any button to retry",
                    ],
                    false,
                ),
                Err(e) => {
                    error!("scanning SD card: {}", e.to_string());
                    ui::draw_message(
                        &mut self.screen,
                        &[
                            "SD card error",
                            "Check the card is inserted",
                            "Press any button to retry",
                        ],
                        true,
                    );
                }
            }
            self.wait_for_press();
        }
    }

    fn select(&mut self, index: usize) {
        let previous = self.selected;
        self.selected = index;

        if index < self.top {
            self.top = index;
            self.draw_rows();
        } else if index >= self.top + VISIBLE_ROWS {
            self.top = index + 1 - VISIBLE_ROWS;
            self.draw_rows();
        } else {
            // Same window: only the highlight moves, so the icons don't have to be re-read.
            self.draw_label(previous);
            self.draw_label(index);
        }
        ui::draw_header(&mut self.screen, self.selected, self.games.len());
    }

    fn draw_list(&mut self) {
        ui::draw_header(&mut self.screen, self.selected, self.games.len());
        self.draw_rows();
    }

    fn draw_rows(&mut self) {
        for slot in 0..VISIBLE_ROWS {
            let index = self.top + slot;
            let Some(game) = self.games.get(index) else {
                ui::clear_row(&mut self.screen, slot);
                continue;
            };
            let icon =
                games::read_icon(&self.fs, game, &mut self.icon_buf).then_some(&self.icon_buf);
            ui::draw_row_icon(&mut self.screen, slot, icon);
            self.draw_label(index);
        }
    }

    fn draw_label(&mut self, index: usize) {
        let name = &self.games[index].meta.name;
        ui::draw_row_label(
            &mut self.screen,
            index - self.top,
            name,
            index == self.selected,
        );
    }

    /// Installs and boots the selected game. Only returns if that fails.
    fn launch(&mut self) {
        let game = &self.games[self.selected];
        info!("launching {}", game.id);
        ui::draw_progress_frame(&mut self.screen, &game.meta.name);

        let screen = &mut self.screen;
        let mut last = None;
        let result = flasher::install(&self.fs, game, &mut self.flash, |stage, percent| {
            if last != Some((stage, percent)) {
                last = Some((stage, percent));
                ui::draw_progress(screen, stage, percent);
            }
        });

        match result {
            Ok(()) => esp_hal::system::software_reset(),
            Err(e) => {
                error!("launching {}: {e:?}", game.id);
                ui::draw_message(
                    &mut self.screen,
                    &["Launch failed", e.message(), "Press any button"],
                    true,
                );
                self.wait_for_press();
                self.draw_list();
            }
        }
    }

    fn wait_for_press(&mut self) {
        while !self.buttons.update().pressed.any() {
            Delay::new().delay_millis(POLL_INTERVAL_MS);
        }
    }
}
