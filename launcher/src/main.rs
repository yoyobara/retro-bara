#![no_std]
#![no_main]

use embedded_graphics::draw_target::DrawTarget as _;
use embedded_graphics::pixelcolor::{Rgb565, RgbColor as _};
use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::clock::CpuClock;

esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let mut board = board::Board::new(peripherals);
    board.screen.clear(Rgb565::BLUE).unwrap();

    loop {}
}
