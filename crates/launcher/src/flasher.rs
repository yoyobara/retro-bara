//! Copies a game's `GAME.BIN` into `ota_0` and makes it the boot partition.

use esp_bootloader_esp_idf::ota::Ota;
use esp_bootloader_esp_idf::partitions::{
    self, AppPartitionSubType, DataPartitionSubType, FlashRegion, FlashStorage, PartitionEntry,
    PartitionType,
};
use log::info;
use sha2::{Digest as _, Sha256};

use crate::games::{self, Fs, FsError, Game};

const CHUNK_LEN: usize = 4096;
const SECTOR_LEN: u32 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Hashing `GAME.BIN` on the SD card.
    Verifying,
    /// Hashing what `ota_0` currently holds.
    Comparing,
    Flashing,
    /// Hashing `ota_0` after writing.
    Checking,
}

// Fields are only read through `Debug` when logging.
#[allow(dead_code)]
#[derive(Debug)]
pub enum LaunchError {
    Fs(FsError),
    Flash(partitions::Error),
    MissingPartition(&'static str),
    SdHashMismatch,
    FlashHashMismatch,
}

impl From<FsError> for LaunchError {
    fn from(e: FsError) -> Self {
        Self::Fs(e)
    }
}

impl From<partitions::Error> for LaunchError {
    fn from(e: partitions::Error) -> Self {
        Self::Flash(e)
    }
}

impl LaunchError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Fs(_) => "SD card read error",
            Self::Flash(_) => "Flash error",
            Self::MissingPartition(_) => "Bad partition table",
            Self::SdHashMismatch => "GAME.BIN checksum mismatch",
            Self::FlashHashMismatch => "Flash verification failed",
        }
    }
}

/// Installs `game` into `ota_0` and selects it for the next boot. The caller resets afterwards.
///
/// `progress` is called with the current stage and a percentage.
pub fn install(
    fs: &Fs,
    game: &Game,
    flash: &mut FlashStorage<'_>,
    mut progress: impl FnMut(Stage, u32),
) -> Result<(), LaunchError> {
    let mut pt_buf = [0; partitions::PARTITION_TABLE_MAX_LEN];
    let pt = partitions::read_partition_table(flash, &mut pt_buf)?;
    let ota_0 = find(&pt, PartitionType::App(AppPartitionSubType::Ota0), "ota_0")?;
    let otadata = find(
        &pt,
        PartitionType::Data(DataPartitionSubType::Ota),
        "otadata",
    )?;

    let len = game.bin_len;
    let expected = game.meta.sha256;
    let mut buf = [0; CHUNK_LEN];

    if hash_sd(fs, game, &mut buf, |pct| progress(Stage::Verifying, pct))? != expected {
        return Err(LaunchError::SdHashMismatch);
    }

    let mut region = ota_0.as_flash_region(flash);
    if hash_region(&mut region, len, &mut buf, |pct| {
        progress(Stage::Comparing, pct)
    })? == expected
    {
        info!("ota_0 already holds {}, skipping flash", game.id);
    } else {
        write_region(fs, game, &mut region, &mut buf, |pct| {
            progress(Stage::Flashing, pct)
        })?;
        if hash_region(&mut region, len, &mut buf, |pct| {
            progress(Stage::Checking, pct)
        })? != expected
        {
            return Err(LaunchError::FlashHashMismatch);
        }
        info!("flashed {} ({len} bytes) to ota_0", game.id);
    }

    // There is only one OTA app partition, but `Ota` can't select it with a count of 1: coming
    // from factory it would write sequence number 0. With a count of 2 it writes a proper odd
    // sequence number, and the bootloader (which knows there's one slot) maps any number to ota_0.
    let mut ota = Ota::new(otadata.as_flash_region(flash), 2)?;
    ota.set_current_app_partition(AppPartitionSubType::Ota0)?;
    Ok(())
}

fn find(
    pt: &partitions::PartitionTable<'_>,
    ty: PartitionType,
    name: &'static str,
) -> Result<PartitionEntry, LaunchError> {
    pt.find_partition(ty)?
        .ok_or(LaunchError::MissingPartition(name))
}

fn hash_sd(
    fs: &Fs,
    game: &Game,
    buf: &mut [u8; CHUNK_LEN],
    mut progress: impl FnMut(u32),
) -> Result<[u8; 32], LaunchError> {
    games::with_bin(fs, game, |file| {
        let mut hasher = Sha256::new();
        let mut done = 0;
        loop {
            let n = games::read_full(file, buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            done += n as u32;
            progress(percent(done, game.bin_len));
        }
        Ok(hasher.finalize().into())
    })
}

fn hash_region(
    region: &mut FlashRegion<'_, '_>,
    len: u32,
    buf: &mut [u8; CHUNK_LEN],
    mut progress: impl FnMut(u32),
) -> Result<[u8; 32], LaunchError> {
    let mut hasher = Sha256::new();
    let mut offset = 0;
    while offset < len {
        // Read whole chunks so reads stay word-aligned; the partition is a multiple of 4 KiB.
        region.read(offset, buf)?;
        let n = (len - offset).min(CHUNK_LEN as u32);
        hasher.update(&buf[..n as usize]);
        offset += n;
        progress(percent(offset, len));
    }
    Ok(hasher.finalize().into())
}

fn write_region(
    fs: &Fs,
    game: &Game,
    region: &mut FlashRegion<'_, '_>,
    buf: &mut [u8; CHUNK_LEN],
    mut progress: impl FnMut(u32),
) -> Result<(), LaunchError> {
    let len = game.bin_len;
    region.erase(0, len.div_ceil(SECTOR_LEN) * SECTOR_LEN)?;

    games::with_bin(fs, game, |file| {
        let mut offset = 0;
        while offset < len {
            // Pad the last chunk with the erased value so writes stay word-aligned.
            buf.fill(0xFF);
            let n = games::read_full(file, buf)?;
            if n == 0 {
                break;
            }
            region.write(offset, buf)?;
            offset += n as u32;
            progress(percent(offset, len));
        }
        Ok(())
    })
}

fn percent(done: u32, total: u32) -> u32 {
    if total == 0 {
        100
    } else {
        (done as u64 * 100 / total as u64) as u32
    }
}
