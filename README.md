<div align="center">

# trollstov

Your music is the database

TODO: image

</div>

`trollstov` is a music player for the terminal that is built around a simple idea: your music is the database. It treats your audio files — and their metadata — as the single source of truth. Your filesystem is the index, and your tags are the schema. Simply back up your music directory and you have backed up everything. Ratings are part of the metadata, so your favorite songs are always just a few keystrokes away.

The name is a norwegian word play for a substance with magical properties. You have "troll", a creature from Norse mythology, and "stov" which is actually "støv", meaning "dust" in english. Hence, you get "troll dust".

## 📌 Features

- Your music is the database — your files and their metadata are all you need.
- Portable rating — the rating is part of the metadata.

## ⚡ Usage

The `trollstov` command takes one mandatory argument, which is the path to your music directory.

```sh
trollstov /path/to/my/music
```

In addition, it comes with one optional argument.

| Option | Description |
| ------ | ----------- |
| `--mpris` | Add media controls through the Media Player Remote Interfacing Specification (MPRIS). |

## 💡 Supported Audio Formats

| Format | Metadata | Playback |
| ------ | -------- | -------- |
| FLAC | Vorbis Comments | Yes |
| Opus | Vorbis Comments | No |
| Ogg Vorbis | Vorbis Comments | Yes |
| MP3 | ID3v2 | Yes |

## ⚠️ Non-goals

- Supporting most audio formats.
- Metadata editing, except for the rating.
- Gapless playback.

## 🔖 Install

todo
