#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString as _};
use alloc::vec::Vec;
use core::ops::ControlFlow;

use embedded_graphics::Drawable as _;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::{Rgb565, RgbColor as _};
use embedded_graphics::text::{Baseline, Text};
use embedded_sdmmc::{LfnBuffer, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::clock::CpuClock;

esp_app_desc!();

const LINE_HEIGHT: i32 = 12;
const MARGIN: i32 = 4;

/// No RTC on the board: every timestamp is the FAT epoch.
struct NoClock;

impl TimeSource for NoClock {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 10, // 1980, the earliest FAT date
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

#[esp_hal::main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let board::Board {
        mut screen,
        sd_card,
        ..
    } = board::Board::new(peripherals);
    screen.clear(Rgb565::BLACK).unwrap();

    let lines = match list_root_dir(sd_card) {
        Ok(entries) if entries.is_empty() => alloc::vec!["SD card is empty".to_string()],
        Ok(entries) => entries,
        Err(e) => alloc::vec!["SD card error:".to_string(), e],
    };

    draw_lines(&mut screen, &lines);

    loop {}
}

/// Returns one display line per entry in the SD card's root directory.
fn list_root_dir(sd_card: board::BoardSdCard) -> Result<Vec<String>, String> {
    let volume_mgr = VolumeManager::new(sd_card, NoClock);
    let volume = volume_mgr
        .open_volume(VolumeIdx(0))
        .map_err(|e| format!("{e:?}"))?;
    let root = volume.open_root_dir().map_err(|e| format!("{e:?}"))?;

    let mut lfn_storage = [0u8; 255 * 3];
    let mut lfn_buffer = LfnBuffer::new(&mut lfn_storage);
    let mut entries = Vec::new();

    root.iterate_dir_lfn(&mut lfn_buffer, |entry, long_name| {
        if entry.attributes.is_volume() || entry.attributes.is_hidden() {
            return ControlFlow::Continue(());
        }
        let name = match long_name {
            Some(name) => name.to_string(),
            None => entry.name.to_string(),
        };
        entries.push(if entry.attributes.is_directory() {
            format!("{name}/")
        } else {
            format!("{name}  ({} B)", entry.size)
        });
        ControlFlow::Continue(())
    })
    .map_err(|e| format!("{e:?}"))?;

    Ok(entries)
}

fn draw_lines<D>(target: &mut D, lines: &[String])
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: core::fmt::Debug,
{
    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    let max_lines = (target.bounding_box().size.height as i32 - 2 * MARGIN) / LINE_HEIGHT;

    for (i, line) in lines.iter().take(max_lines as usize).enumerate() {
        let position = Point::new(MARGIN, MARGIN + i as i32 * LINE_HEIGHT);
        Text::with_baseline(line, position, style, Baseline::Top)
            .draw(target)
            .unwrap();
    }
}
