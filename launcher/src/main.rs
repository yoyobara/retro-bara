#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;

esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default());
    esp_alloc::heap_allocator!(size: 72 * 1024);

    loop {}
}
