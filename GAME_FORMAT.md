# SD Card Game Format

This document describes how games must be laid out on the SD card so the launcher
can list them, show them in the menu, and flash them into the `ota_0` partition.

## Card layout

The card must be formatted as **FAT16 or FAT32** (MBR partition table, first partition).
Every game lives in its own directory under `GAMES/` in the card's root:

```
/
└── GAMES/
    ├── SNAKE/
    │   ├── GAME.TOM
    │   ├── GAME.BIN
    │   └── ICON.RAW
    └── TETRIS/
        ├── GAME.TOM
        ├── GAME.BIN
        └── ICON.RAW
```

### Naming rules

The launcher uses `embedded-sdmmc`, which only understands **8.3 short file names**:

- Directory names: 1–8 characters, `A–Z`, `0–9`, `_` or `-`, no extension.
- Write all names in **upper case**. Long or lower-case names get a mangled short
  alias (e.g. `SUPERM~1`) that the launcher won't recognise reliably.
- The directory name is the game's **id**. It must be unique on the card and must
  not change between versions of the same game.
- Hidden entries and entries that aren't directories are ignored inside `GAMES/`.

A game directory **must** contain the three files below. A directory missing any of
them is skipped (and logged) by the launcher.

| File       | Required | Contents                                   |
| ---------- | -------- | ------------------------------------------ |
| `GAME.TOM` | yes      | Metadata, TOML-like `key = value` text     |
| `GAME.BIN` | yes      | ESP32-S3 application image                 |
| `ICON.RAW` | yes      | 64×64 icon, raw RGB565 big-endian pixels   |

Any other files in the directory are ignored and may be used by the game itself
(save data, levels, assets).

## `GAME.TOM` — metadata

A small UTF-8 [TOML](https://toml.io) file with the keys below at the top level:

```toml
format = 1
name = "Snake"
version = "1.2.0"
author = "Yotam Barak"
description = "Eat apples, don't eat yourself."
sha256 = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
```

Rules:

- Any valid TOML is accepted: comments, basic or literal strings, escapes, etc.
- Values must have the types in the table below (e.g. `format = "1"` is rejected).
- Unknown keys and tables are ignored, so future fields can be added without breaking
  older launchers.
- The whole file must be at most **1 KiB**.

| Key           | Type    | Required | Description                                                     |
| ------------- | ------- | -------- | --------------------------------------------------------------- |
| `format`      | integer | yes      | Version of this spec. Currently `1`. Other values are rejected. |
| `name`        | string  | yes      | Display name in the menu. Max 24 characters, printable ASCII.   |
| `version`     | string  | yes      | Free-form version string, e.g. `1.2.0`. Max 16 characters.      |
| `sha256`      | string  | yes      | Lower-case hex SHA-256 of `GAME.BIN` (64 characters).           |
| `author`      | string  | no       | Max 24 characters.                                              |
| `description` | string  | no       | Max 128 characters. Shown on the game's detail screen.          |

The `sha256` is used to:

1. Verify `GAME.BIN` was read from the card correctly before and after flashing.
2. Skip re-flashing when `ota_0` already holds the exact same image.

Generate it with:

```sh
sha256sum GAME.BIN | cut -d' ' -f1
```

## `GAME.BIN` — the binary

A standard **ESP-IDF application image** (the format with the `0xE9` magic byte),
not a raw ELF and not a merged image containing a bootloader or partition table.

Requirements:

- Built for `esp32s3` (target `xtensa-esp32s3-none-elf`).
- Must include an app descriptor (`esp_app_desc!()` from `esp-bootloader-esp-idf`),
  otherwise the second-stage bootloader refuses to boot it.
- Size must not exceed the `ota_0` partition: **0x6E0000 bytes (7 040 KiB)**.
  See `partitions.csv`.
- Should use the `board` crate for hardware access so pin assignments stay in sync
  with the launcher.

Producing it from an ELF with `espflash`:

```sh
cargo build --release -p <your-game>
espflash save-image --chip esp32s3 \
    target/xtensa-esp32s3-none-elf/release/<your-game> GAME.BIN
```

The launcher writes `GAME.BIN` byte-for-byte to the start of `ota_0`, marks
`ota_0` as the boot partition in `otadata`, and resets.

## `ICON.RAW` — the icon

Headerless pixel data in the screen's native format, so the launcher can stream it
straight to the display without decoding:

- Size: **64 × 64** pixels.
- Pixel format: **RGB565, big-endian** (high byte first), 2 bytes per pixel.
- Order: row-major, top-left pixel first, no padding between rows.
- File size: exactly **8 192 bytes** (64 × 64 × 2). Any other size is rejected
  and a placeholder icon is shown instead.
- No transparency: draw the background colour you want into the icon itself
  (the menu background is black, `0x0000`).

Converting a PNG with `ffmpeg`:

```sh
ffmpeg -i icon.png -vf scale=64:64:flags=neighbor \
    -f rawvideo -pix_fmt rgb565be ICON.RAW
```

Or with Python + Pillow:

```python
from PIL import Image

img = Image.open("icon.png").convert("RGB").resize((64, 64), Image.NEAREST)
with open("ICON.RAW", "wb") as f:
    for r, g, b in img.getdata():
        px = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)
        f.write(px.to_bytes(2, "big"))
```

## Checklist

Before copying a game to the card, check that:

- [ ] Directory is `GAMES/<ID>/` with an upper-case id of at most 8 characters.
- [ ] `GAME.TOM` has `format = 1`, `name`, `version` and `sha256`.
- [ ] `GAME.BIN` is an ESP32-S3 app image no larger than 7 040 KiB.
- [ ] `sha256` in `GAME.TOM` matches `sha256sum GAME.BIN`.
- [ ] `ICON.RAW` is exactly 8 192 bytes.
