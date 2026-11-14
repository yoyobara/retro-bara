use core::ops::{BitAnd, BitOr, Not};

use esp_hal::gpio::{Input, InputConfig, InputPin, Pull};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Button {
    Left,
    Up,
    Down,
    Right,
    A,
    B,
    Start,
    Select,
}

impl Button {
    pub const ALL: [Button; 8] = [
        Button::Left,
        Button::Up,
        Button::Down,
        Button::Right,
        Button::A,
        Button::B,
        Button::Start,
        Button::Select,
    ];

    const fn mask(self) -> u8 {
        1 << self as u8
    }
}

/// A set of buttons, one bit per [`Button`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonState(u8);

impl ButtonState {
    pub const NONE: Self = Self(0);

    pub const fn is_pressed(self, button: Button) -> bool {
        self.0 & button.mask() != 0
    }

    pub const fn any(self) -> bool {
        self.0 != 0
    }

    pub fn contains_any(self, buttons: &[Button]) -> bool {
        buttons.iter().any(|&b| self.is_pressed(b))
    }

    pub const fn with(self, button: Button) -> Self {
        Self(self.0 | button.mask())
    }
}

impl BitAnd for ButtonState {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for ButtonState {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl Not for ButtonState {
    type Output = Self;

    fn not(self) -> Self {
        Self(!self.0)
    }
}

/// What changed between two consecutive [`Buttons::update`] calls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonEvents {
    /// Buttons currently down.
    pub held: ButtonState,
    /// Buttons that went down since the previous update.
    pub pressed: ButtonState,
    /// Buttons that went up since the previous update.
    pub released: ButtonState,
}

pub struct Buttons {
    // Same order as `Button`.
    inputs: [Input<'static>; 8],
    prev: ButtonState,
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
            inputs: [
                Input::new(left, config),
                Input::new(up, config),
                Input::new(down, config),
                Input::new(right, config),
                Input::new(a, config),
                Input::new(b, config),
                Input::new(start, config),
                Input::new(select, config),
            ],
            prev: ButtonState::NONE,
        }
    }

    /// Samples the buttons without affecting [`Buttons::update`]'s edge detection.
    pub fn read(&self) -> ButtonState {
        Button::ALL
            .iter()
            .zip(&self.inputs)
            .filter(|(_, input)| input.is_low())
            .fold(ButtonState::NONE, |state, (&button, _)| state.with(button))
    }

    /// Samples the buttons and reports edges relative to the previous call.
    ///
    /// Poll every ~10 ms; that interval is also enough to debounce.
    pub fn update(&mut self) -> ButtonEvents {
        let held = self.read();
        let events = ButtonEvents {
            held,
            pressed: held & !self.prev,
            released: self.prev & !held,
        };
        self.prev = held;
        events
    }
}
