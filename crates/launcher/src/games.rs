//! Discovers games on the SD card, see `GAME_FORMAT.md`.

use alloc::vec::Vec;
use core::ops::ControlFlow;

use embedded_sdmmc::{
    Directory, File, Mode, SdCardError, ShortFileName, TimeSource, Timestamp, VolumeIdx,
    VolumeManager,
};
use log::{info, warn};

use crate::meta::{self, GameMeta};

pub const ICON_SIZE: u32 = 64;
pub const ICON_LEN: usize = (ICON_SIZE * ICON_SIZE * 2) as usize;
/// Size of the `ota_0` partition in `partitions.csv`.
pub const MAX_BIN_LEN: u32 = 0x6E_0000;

const GAMES_DIR: &str = "GAMES";
const META_FILE: &str = "GAME.TOM";
const BIN_FILE: &str = "GAME.BIN";
const ICON_FILE: &str = "ICON.RAW";

pub type Fs = VolumeManager<board::BoardSdCard, NoClock>;
pub type FsError = embedded_sdmmc::Error<SdCardError>;
pub type GameDir<'a> = Directory<'a, board::BoardSdCard, NoClock, 4, 4, 1>;
pub type GameFile<'a> = File<'a, board::BoardSdCard, NoClock, 4, 4, 1>;

/// No RTC on the board: every timestamp is the FAT epoch.
pub struct NoClock;

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

pub struct Game {
    pub id: ShortFileName,
    pub meta: GameMeta,
    pub bin_len: u32,
}

// Fields are only read through `Debug` when logging.
#[allow(dead_code)]
#[derive(Debug)]
enum GameError {
    Fs(FsError),
    MetaTooLong,
    Meta(meta::MetaError),
    BinTooLong(u32),
}

impl From<FsError> for GameError {
    fn from(e: FsError) -> Self {
        Self::Fs(e)
    }
}

/// Lists the valid games in `/GAMES`, sorted by name. Invalid ones are logged and skipped.
pub fn scan(fs: &Fs) -> Result<Vec<Game>, FsError> {
    let ids = with_dir(fs, None, |games_dir| {
        let mut ids = Vec::new();
        games_dir.iterate_dir(|entry| {
            let attrs = entry.attributes;
            let is_dot = entry.name.base_name().first() == Some(&b'.');
            if attrs.is_directory() && !attrs.is_hidden() && !attrs.is_volume() && !is_dot {
                ids.push(entry.name);
            }
            ControlFlow::Continue(())
        })?;
        Ok::<_, FsError>(ids)
    })?;

    let mut games: Vec<Game> = ids
        .into_iter()
        .filter_map(|id| match load_game(fs, &id) {
            Ok(game) => {
                info!("found game {id}: {}", game.meta.name);
                Some(game)
            }
            Err(e) => {
                warn!("skipping game {id}: {e:?}");
                None
            }
        })
        .collect();
    games.sort_by(|a, b| a.meta.name.cmp(&b.meta.name));
    Ok(games)
}

fn load_game(fs: &Fs, id: &ShortFileName) -> Result<Game, GameError> {
    with_dir(fs, Some(id), |dir| {
        let meta_file = dir.open_file_in_dir(META_FILE, Mode::ReadOnly)?;
        if meta_file.length() as usize > meta::MAX_LEN {
            return Err(GameError::MetaTooLong);
        }
        let mut buf = [0; meta::MAX_LEN];
        let len = read_full(&meta_file, &mut buf)?;
        let text = core::str::from_utf8(&buf[..len])
            .map_err(|_| GameError::Meta(meta::MetaError::Invalid("utf-8")))?;
        let meta = meta::parse(text).map_err(GameError::Meta)?;

        let bin_len = dir.open_file_in_dir(BIN_FILE, Mode::ReadOnly)?.length();
        if bin_len > MAX_BIN_LEN {
            return Err(GameError::BinTooLong(bin_len));
        }
        // Must exist; a wrong size only falls back to the placeholder icon.
        dir.open_file_in_dir(ICON_FILE, Mode::ReadOnly)?;

        Ok(Game {
            id: *id,
            meta,
            bin_len,
        })
    })
}

/// Reads the game's icon into `buf`. Returns `false` if it's missing, the wrong size or unreadable.
pub fn read_icon(fs: &Fs, game: &Game, buf: &mut [u8; ICON_LEN]) -> bool {
    let result = with_dir(fs, Some(&game.id), |dir| {
        let file = dir.open_file_in_dir(ICON_FILE, Mode::ReadOnly)?;
        Ok::<_, FsError>(file.length() as usize == ICON_LEN && read_full(&file, buf)? == ICON_LEN)
    });
    result.unwrap_or_else(|e| {
        warn!("reading icon of {}: {e:?}", game.id);
        false
    })
}

/// Opens the game's `GAME.BIN` for the duration of `f`.
pub fn with_bin<R, E: From<FsError>>(
    fs: &Fs,
    game: &Game,
    f: impl FnOnce(&GameFile) -> Result<R, E>,
) -> Result<R, E> {
    with_dir(fs, Some(&game.id), |dir| {
        let file = dir.open_file_in_dir(BIN_FILE, Mode::ReadOnly)?;
        f(&file)
    })
}

/// Reads until `buf` is full or the file ends, returning the number of bytes read.
pub fn read_full(file: &GameFile, buf: &mut [u8]) -> Result<usize, FsError> {
    let mut len = 0;
    while len < buf.len() && !file.is_eof() {
        len += file.read(&mut buf[len..])?;
    }
    Ok(len)
}

/// Runs `f` on `/GAMES`, or on `/GAMES/<id>` when `id` is given.
fn with_dir<R, E: From<FsError>>(
    fs: &Fs,
    id: Option<&ShortFileName>,
    f: impl FnOnce(&GameDir) -> Result<R, E>,
) -> Result<R, E> {
    let volume = fs.open_volume(VolumeIdx(0))?;
    let root = volume.open_root_dir()?;
    let games_dir = root.open_dir(GAMES_DIR)?;
    match id {
        Some(id) => f(&games_dir.open_dir(id)?),
        None => f(&games_dir),
    }
}
