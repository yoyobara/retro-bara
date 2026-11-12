#![no_std]

mod buttons;
mod screen;

use core::cell::RefCell;

use embedded_hal_bus::spi::RefCellDevice;
use embedded_sdmmc::SdCard;
use esp_hal::Blocking;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin};
use esp_hal::peripherals::Peripherals;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use log::{info, warn};
use static_cell::{ConstStaticCell, StaticCell};

pub use buttons::{ButtonState, Buttons};
pub use screen::Screen;

pub type BoardSpiBus = Spi<'static, Blocking>;
pub type BoardSpiDevice = RefCellDevice<'static, BoardSpiBus, Output<'static>, Delay>;
pub type BoardScreen = Screen<'static, BoardSpiDevice, Output<'static>, Output<'static>>;
pub type BoardSdCard = SdCard<BoardSpiDevice, Delay>;

/// SD cards must be initialized at 100-400 kHz.
const SPI_INIT_FREQUENCY: Rate = Rate::from_khz(400);
/// Bus clock after init: shared by both devices, so kept under the SD card's 25 MHz limit.
const SPI_FREQUENCY: Rate = Rate::from_mhz(20);

pub struct Board {
    pub buttons: Buttons,
    pub screen: BoardScreen,
    /// May be missing: check `num_bytes()` before using it.
    pub sd_card: BoardSdCard,
}

impl Board {
    /// Panics if called more than once.
    pub fn new(peripherals: Peripherals) -> Self {
        let buttons = Buttons::new(
            peripherals.GPIO33,
            peripherals.GPIO39,
            peripherals.GPIO38,
            peripherals.GPIO40,
            peripherals.GPIO41,
            peripherals.GPIO42,
            peripherals.GPIO47,
            peripherals.GPIO48,
        );

        // TODO: real SD MISO/CS pins
        let sd_miso = peripherals.GPIO4;
        let sd_cs_pin = peripherals.GPIO15;

        let spi = Spi::new(peripherals.SPI2, spi_config(SPI_INIT_FREQUENCY))
            .expect("invalid SPI config")
            .with_sck(peripherals.GPIO5)
            .with_mosi(peripherals.GPIO6)
            .with_miso(sd_miso);

        static SPI_BUS: StaticCell<RefCell<BoardSpiBus>> = StaticCell::new();
        let bus = SPI_BUS.init(RefCell::new(spi));

        // Deselect both devices before any traffic, so the SD card never sees screen commands.
        let screen_cs = Output::new(peripherals.GPIO7, Level::High, OutputConfig::default());
        let sd_cs = Output::new(sd_cs_pin, Level::High, OutputConfig::default());

        let sd_card = init_sd_card(bus, sd_cs);

        bus.borrow_mut()
            .apply_config(&spi_config(SPI_FREQUENCY))
            .expect("invalid SPI config");

        let screen = init_screen(bus, screen_cs, peripherals.GPIO19, peripherals.GPIO20);

        info!("board initialized");

        Self {
            buttons,
            screen,
            sd_card,
        }
    }
}

fn init_sd_card(bus: &'static RefCell<BoardSpiBus>, cs: Output<'static>) -> BoardSdCard {
    let device = RefCellDevice::new(bus, cs, Delay::new()).unwrap();
    let sd_card = SdCard::new(device, Delay::new());
    match sd_card.num_bytes() {
        Ok(bytes) => info!("SD card: {} MiB", bytes / (1024 * 1024)),
        Err(e) => warn!("SD card init failed: {e:?}"),
    }
    sd_card
}

fn init_screen(
    bus: &'static RefCell<BoardSpiBus>,
    cs: Output<'static>,
    dc: impl OutputPin + 'static,
    rst: impl OutputPin + 'static,
) -> BoardScreen {
    let device = RefCellDevice::new(bus, cs, Delay::new()).unwrap();
    let dc = Output::new(dc, Level::Low, OutputConfig::default());
    let rst = Output::new(rst, Level::Low, OutputConfig::default());
    static SCREEN_BUFFER: ConstStaticCell<[u8; 512]> = ConstStaticCell::new([0; 512]);
    Screen::init(device, dc, rst, SCREEN_BUFFER.take(), &mut Delay::new())
        .expect("screen init failed")
}

fn spi_config(frequency: Rate) -> Config {
    Config::default()
        .with_frequency(frequency)
        .with_mode(Mode::_0)
}
