use embedded_graphics_core::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    primitives::Rectangle,
};
use embedded_hal::{delay::DelayNs, digital::OutputPin, spi::SpiDevice};
use mipidsi::{
    Builder, Display, InitError,
    interface::{SpiError, SpiInterface},
    models::ILI9341Rgb565,
    options::{ColorOrder::Rgb, Orientation, Rotation::Deg180},
};

pub type ScreenInitError<SPI, DC, RST> = InitError<
    SpiError<
        <SPI as embedded_hal::spi::ErrorType>::Error,
        <DC as embedded_hal::digital::ErrorType>::Error,
    >,
    <RST as embedded_hal::digital::ErrorType>::Error,
>;

pub struct Screen<'a, SPI, DC, RST>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
{
    display: Display<SpiInterface<'a, SPI, DC>, ILI9341Rgb565, RST>,
}

impl<'a, SPI, DC, RST> Screen<'a, SPI, DC, RST>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
{
    pub fn init(
        spi_device: SPI,
        dc_pin: DC,
        rst_pin: RST,
        buffer: &'a mut [u8],
        delay: &mut impl DelayNs,
    ) -> Result<Self, ScreenInitError<SPI, DC, RST>> {
        let di = SpiInterface::new(spi_device, dc_pin, buffer);
        let display = Builder::new(ILI9341Rgb565, di)
            .reset_pin(rst_pin)
            .display_size(240, 320)
            .orientation(Orientation::new().rotate(Deg180))
            .color_order(Rgb)
            .init(delay)?;

        Ok(Self { display })
    }
}

impl<SPI, DC, RST> OriginDimensions for Screen<'_, SPI, DC, RST>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
{
    fn size(&self) -> Size {
        self.display.size()
    }
}

impl<'a, SPI, DC, RST> DrawTarget for Screen<'a, SPI, DC, RST>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
{
    type Color = <Display<SpiInterface<'a, SPI, DC>, ILI9341Rgb565, RST> as DrawTarget>::Color;

    type Error = <Display<SpiInterface<'a, SPI, DC>, ILI9341Rgb565, RST> as DrawTarget>::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.display.fill_contiguous(area, colors)
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.display.fill_solid(area, color)
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.display.clear(color)
    }
}
