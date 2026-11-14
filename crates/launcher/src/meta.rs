//! Parser for `GAME.TOM`, see `GAME_FORMAT.md`.

use alloc::string::String;

use serde::Deserialize;

pub const MAX_LEN: usize = 1024;

const FORMAT_VERSION: u32 = 1;
const MAX_NAME_LEN: usize = 24;
const MAX_VERSION_LEN: usize = 16;
const MAX_AUTHOR_LEN: usize = 24;
const MAX_DESCRIPTION_LEN: usize = 128;

#[derive(Debug, Clone)]
pub struct GameMeta {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub sha256: [u8; 32],
}

// Fields are only read through `Debug` when logging.
#[allow(dead_code)]
#[derive(Debug)]
pub enum MetaError {
    Toml(toml::de::Error),
    Invalid(&'static str),
    UnsupportedFormat(u32),
}

/// `GAME.TOM` as written; unknown keys are ignored.
#[derive(Deserialize)]
struct RawMeta {
    format: u32,
    name: String,
    version: String,
    author: Option<String>,
    description: Option<String>,
    sha256: String,
}

pub fn parse(text: &str) -> Result<GameMeta, MetaError> {
    let raw: RawMeta = toml::from_str(text).map_err(MetaError::Toml)?;

    if raw.format != FORMAT_VERSION {
        return Err(MetaError::UnsupportedFormat(raw.format));
    }

    if raw.name.is_empty()
        || raw.name.len() > MAX_NAME_LEN
        || !raw.name.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
    {
        return Err(MetaError::Invalid("name"));
    }
    check_len(&raw.version, MAX_VERSION_LEN, "version")?;
    if let Some(author) = &raw.author {
        check_len(author, MAX_AUTHOR_LEN, "author")?;
    }
    if let Some(description) = &raw.description {
        check_len(description, MAX_DESCRIPTION_LEN, "description")?;
    }
    let sha256 = parse_sha256(&raw.sha256).ok_or(MetaError::Invalid("sha256"))?;

    Ok(GameMeta {
        name: raw.name,
        version: raw.version,
        author: raw.author,
        description: raw.description,
        sha256,
    })
}

fn check_len(value: &str, max: usize, field: &'static str) -> Result<(), MetaError> {
    if value.chars().count() > max {
        Err(MetaError::Invalid(field))
    } else {
        Ok(())
    }
}

fn parse_sha256(hex: &str) -> Option<[u8; 32]> {
    let hex = hex.as_bytes();
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0; 32];
    for (byte, pair) in out.iter_mut().zip(hex.chunks_exact(2)) {
        *byte = hex_digit(pair[0])? << 4 | hex_digit(pair[1])?;
    }
    Some(out)
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}
