//! Launcher screens. The display is 240×320 portrait.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use board::BoardScreen;
use embedded_graphics::Drawable as _;
use embedded_graphics::draw_target::DrawTarget as _;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::mono_font::ascii::{FONT_6X13, FONT_10X20};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::{Rgb565, RgbColor as _, WebColors as _};
use embedded_graphics::primitives::{Primitive as _, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

use crate::flasher::Stage;
use crate::games::{ICON_LEN, ICON_SIZE};
use crate::meta::GameMeta;

const WIDTH: u32 = 240;
const HEIGHT: u32 = 320;

const HEADER_HEIGHT: u32 = 24;
const ROW_HEIGHT: u32 = 74;
pub const VISIBLE_ROWS: usize = ((HEIGHT - HEADER_HEIGHT) / ROW_HEIGHT) as usize;
const ROW_PADDING: u32 = (ROW_HEIGHT - ICON_SIZE) / 2;
const LABEL_X: i32 = (ROW_PADDING + ICON_SIZE + 4) as i32;

const MODAL: Rectangle = Rectangle::new(Point::new(12, 40), Size::new(WIDTH - 24, 240));
const MODAL_PADDING: i32 = 8;

const PROGRESS_BAR: Rectangle = Rectangle::new(Point::new(20, 170), Size::new(WIDTH - 40, 16));

const BACKGROUND: Rgb565 = Rgb565::BLACK;
const FOREGROUND: Rgb565 = Rgb565::WHITE;
const DIM: Rgb565 = Rgb565::CSS_GRAY;
const ACCENT: Rgb565 = Rgb565::CSS_DODGER_BLUE;
const HIGHLIGHT: Rgb565 = Rgb565::CSS_MIDNIGHT_BLUE;
const PANEL: Rgb565 = Rgb565::new(3, 6, 3);
const ERROR: Rgb565 = Rgb565::CSS_TOMATO;

pub fn draw_header(screen: &mut BoardScreen, selected: usize, total: usize) {
    let area = Rectangle::new(Point::zero(), Size::new(WIDTH, HEADER_HEIGHT));
    fill(screen, area, HIGHLIGHT);
    let y = (HEADER_HEIGHT / 2) as i32;
    text(
        screen,
        "RETRO-BARA",
        Point::new(6, y),
        &FONT_10X20,
        FOREGROUND,
        Alignment::Left,
    );
    let counter = format!("{}/{}", selected + 1, total);
    text(
        screen,
        &counter,
        Point::new(WIDTH as i32 - 6, y),
        &FONT_6X13,
        DIM,
        Alignment::Right,
    );
}

/// Draws the icon of the game shown in `slot` (0..VISIBLE_ROWS), or a placeholder.
pub fn draw_row_icon(screen: &mut BoardScreen, slot: usize, icon: Option<&[u8; ICON_LEN]>) {
    let area = Rectangle::new(
        row_origin(slot) + Point::new(ROW_PADDING as i32, ROW_PADDING as i32),
        Size::new(ICON_SIZE, ICON_SIZE),
    );
    match icon {
        Some(icon) => {
            let pixels = icon
                .chunks_exact(2)
                .map(|px| Rgb565::from(RawU16::new(u16::from_be_bytes([px[0], px[1]]))));
            screen.fill_contiguous(&area, pixels).unwrap();
        }
        None => {
            fill(screen, area, Rgb565::CSS_DIM_GRAY);
            text(
                screen,
                "?",
                area.center(),
                &FONT_10X20,
                FOREGROUND,
                Alignment::Center,
            );
        }
    }
}

/// Draws the name next to the icon in `slot`, highlighted if `selected`.
pub fn draw_row_label(screen: &mut BoardScreen, slot: usize, name: &str, selected: bool) {
    let origin = row_origin(slot);
    let area = Rectangle::new(
        origin + Point::new(LABEL_X, ROW_PADDING as i32),
        Size::new(WIDTH - LABEL_X as u32 - ROW_PADDING, ICON_SIZE),
    );
    if selected {
        fill(screen, area, HIGHLIGHT);
        area.into_styled(PrimitiveStyle::with_stroke(ACCENT, 2))
            .draw(screen)
            .unwrap();
    } else {
        fill(screen, area, BACKGROUND);
    }
    let position = Point::new(area.top_left.x + 6, area.center().y);
    text(
        screen,
        name,
        position,
        &FONT_6X13,
        FOREGROUND,
        Alignment::Left,
    );
}

pub fn clear_row(screen: &mut BoardScreen, slot: usize) {
    fill(
        screen,
        Rectangle::new(row_origin(slot), Size::new(WIDTH, ROW_HEIGHT)),
        BACKGROUND,
    );
}

pub fn draw_modal(screen: &mut BoardScreen, meta: &GameMeta) {
    fill(screen, MODAL, PANEL);
    MODAL
        .into_styled(PrimitiveStyle::with_stroke(ACCENT, 2))
        .draw(screen)
        .unwrap();

    let inner_width = MODAL.size.width - 2 * MODAL_PADDING as u32;
    let left = MODAL.top_left.x + MODAL_PADDING;
    let mut y = MODAL.top_left.y + MODAL_PADDING;

    let name_font = if fits(&meta.name, &FONT_10X20, inner_width) {
        &FONT_10X20
    } else {
        &FONT_6X13
    };
    y = line(screen, &meta.name, left, y, name_font, FOREGROUND);
    y += 4;
    y = line(
        screen,
        &format!("v{}", meta.version),
        left,
        y,
        &FONT_6X13,
        DIM,
    );
    if let Some(author) = &meta.author {
        y = line(screen, &format!("by {author}"), left, y, &FONT_6X13, DIM);
    }
    y += 10;
    if let Some(description) = &meta.description {
        let max_chars = (inner_width / FONT_6X13.character_size.width) as usize;
        for l in wrap(description, max_chars) {
            y = line(screen, &l, left, y, &FONT_6X13, FOREGROUND);
        }
    }

    let footer_y = MODAL.top_left.y + MODAL.size.height as i32 - MODAL_PADDING - 2 * 13;
    line(
        screen,
        "A/START: launch",
        left,
        footer_y,
        &FONT_6X13,
        ACCENT,
    );
    line(
        screen,
        "Other: back",
        left,
        footer_y + 13,
        &FONT_6X13,
        ACCENT,
    );
}

/// Full-screen centered message; the first line is the title.
pub fn draw_message(screen: &mut BoardScreen, lines: &[&str], is_error: bool) {
    screen.clear(BACKGROUND).unwrap();
    let center_x = (WIDTH / 2) as i32;
    let mut y = (HEIGHT / 2) as i32 - (lines.len() as i32 * 16) / 2;
    for (i, l) in lines.iter().enumerate() {
        let (font, color) = match i {
            0 if is_error => (&FONT_10X20, ERROR),
            0 => (&FONT_10X20, FOREGROUND),
            _ => (&FONT_6X13, DIM),
        };
        text(
            screen,
            l,
            Point::new(center_x, y),
            font,
            color,
            Alignment::Center,
        );
        y += if i == 0 { 26 } else { 16 };
    }
}

pub fn draw_progress_frame(screen: &mut BoardScreen, name: &str) {
    screen.clear(BACKGROUND).unwrap();
    let center_x = (WIDTH / 2) as i32;
    text(
        screen,
        "Launching",
        Point::new(center_x, 110),
        &FONT_10X20,
        FOREGROUND,
        Alignment::Center,
    );
    text(
        screen,
        name,
        Point::new(center_x, 134),
        &FONT_6X13,
        DIM,
        Alignment::Center,
    );
    PROGRESS_BAR
        .into_styled(PrimitiveStyle::with_stroke(FOREGROUND, 1))
        .draw(screen)
        .unwrap();
}

pub fn draw_progress(screen: &mut BoardScreen, stage: Stage, percent: u32) {
    let label = match stage {
        Stage::Verifying => "Verifying SD card",
        Stage::Comparing => "Checking installed game",
        Stage::Flashing => "Flashing",
        Stage::Checking => "Verifying flash",
    };
    let label_area = Rectangle::new(Point::new(0, 196), Size::new(WIDTH, 16));
    fill(screen, label_area, BACKGROUND);
    let label = format!("{label}... {percent}%");
    text(
        screen,
        &label,
        label_area.center(),
        &FONT_6X13,
        DIM,
        Alignment::Center,
    );

    let inner = PROGRESS_BAR.offset(-2);
    let filled = inner.size.width * percent.min(100) / 100;
    let (done, rest) = (
        Rectangle::new(inner.top_left, Size::new(filled, inner.size.height)),
        Rectangle::new(
            inner.top_left + Point::new(filled as i32, 0),
            Size::new(inner.size.width - filled, inner.size.height),
        ),
    );
    fill(screen, done, ACCENT);
    fill(screen, rest, BACKGROUND);
}

fn row_origin(slot: usize) -> Point {
    Point::new(0, (HEADER_HEIGHT + slot as u32 * ROW_HEIGHT) as i32)
}

fn fill(screen: &mut BoardScreen, area: Rectangle, color: Rgb565) {
    screen.fill_solid(&area, color).unwrap();
}

/// Draws `s` vertically centered on `position.y`.
fn text(
    screen: &mut BoardScreen,
    s: &str,
    position: Point,
    font: &MonoFont,
    color: Rgb565,
    alignment: Alignment,
) {
    let character_style = MonoTextStyle::new(font, color);
    let text_style = TextStyleBuilder::new()
        .alignment(alignment)
        .baseline(Baseline::Middle)
        .build();
    Text::with_text_style(s, position, character_style, text_style)
        .draw(screen)
        .unwrap();
}

/// Draws one top-aligned line with its background filled, and returns the next line's y.
fn line(screen: &mut BoardScreen, s: &str, x: i32, y: i32, font: &MonoFont, color: Rgb565) -> i32 {
    let style = MonoTextStyleBuilder::new()
        .font(font)
        .text_color(color)
        .background_color(PANEL)
        .build();
    Text::with_baseline(s, Point::new(x, y), style, Baseline::Top)
        .draw(screen)
        .unwrap();
    y + font.character_size.height as i32
}

fn fits(s: &str, font: &MonoFont, width: u32) -> bool {
    s.chars().count() as u32 * (font.character_size.width + font.character_spacing) <= width
}

/// Greedy word wrap to `max_chars` per line, hard-breaking words that are too long.
fn wrap(s: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in s.split_whitespace() {
        let mut word = word;
        loop {
            let current_len = current.chars().count();
            let word_len = word.chars().count();
            let sep = usize::from(current_len > 0);
            if current_len + sep + word_len <= max_chars {
                if sep == 1 {
                    current.push(' ');
                }
                current.push_str(word);
                break;
            }
            if current_len > 0 {
                lines.push(core::mem::take(&mut current));
                continue;
            }
            // The word alone is longer than a line.
            let split = word
                .char_indices()
                .nth(max_chars)
                .map_or(word.len(), |(i, _)| i);
            lines.push(String::from(&word[..split]));
            word = &word[split..];
            if word.is_empty() {
                break;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
