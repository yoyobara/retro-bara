use esp_hal::gpio::{Input, InputConfig, InputPin, Pull};

pub struct Buttons {
    pub left: Input<'static>,
    pub up: Input<'static>,
    pub down: Input<'static>,
    pub right: Input<'static>,
    pub a: Input<'static>,
    pub b: Input<'static>,
    pub start: Input<'static>,
    pub select: Input<'static>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonState {
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
}

impl Buttons {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        left: impl InputPin + 'static,
        up: impl InputPin + 'static,
        down: impl InputPin + 'static,
        right: impl InputPin + 'static,
        a: impl InputPin + 'static,
        b: impl InputPin + 'static,
        start: impl InputPin + 'static,
        select: impl InputPin + 'static,
    ) -> Self {
        let config = InputConfig::default().with_pull(Pull::Up);
        Self {
            left: Input::new(left, config),
            up: Input::new(up, config),
            down: Input::new(down, config),
            right: Input::new(right, config),
            a: Input::new(a, config),
            b: Input::new(b, config),
            start: Input::new(start, config),
            select: Input::new(select, config),
        }
    }

    pub fn read(&self) -> ButtonState {
        ButtonState {
            left: self.left.is_low(),
            up: self.up.is_low(),
            down: self.down.is_low(),
            right: self.right.is_low(),
            a: self.a.is_low(),
            b: self.b.is_low(),
            start: self.start.is_low(),
            select: self.select.is_low(),
        }
    }
}
