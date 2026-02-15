<div align="center">

# jukebox

A simple music player for the terminal

TODO: image

</div>

`jukebox` is a music player built around a simple idea: your music is the database. It treats your audio files — and their metadata — as the single source of truth. Your filesystem is the index, and your tags are the schema. Simply back up your music directory and you have backed up everything. Ratings are part of the metadata, so your favorite songs are always just a few keystrokes away.

## 📌 Features

- Your music is the database — your files and their metadata are the source of truth.
- Portable rating — the rating is part of the metadata.

## ⚡ Usage

The `jukebox` command takes one mandatory argument, which is the path to your music directory.

```
jukebox /path/to/my/music
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

- Any kind of external data; be it playlists, settings or something else.
- Supporting most audio formats.
- Metadata editing, except for the rating.
- Gapless playback.

## 🔖 Install

todo
