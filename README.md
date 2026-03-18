<div align="center">

# trollstov

Your music is the database.

<figure>
<img src="https://github.com/user-attachments/assets/ef3c9576-e668-41de-8848-9062a2f95c9e"/>
<p><em>The music player with the Monokai Soda color scheme.</em></p>
</figure>

</div>

`trollstov` is a music player for the terminal that is built around a simple idea — your files and their metadata are all you need. Simply back up your music directory and you have backed up everything. Ratings are part of the metadata, so your favorite songs are always just a few keystrokes away.

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
| Opus | Vorbis Comments | Yes[^1] |
| Ogg Vorbis | Vorbis Comments | Yes |
| MP3 | ID3v2 | Yes |

[^1]: Requires the `opus` feature.

## ⚠️ Non-goals

- Supporting most audio formats.
- Metadata editing, except for the rating.
- Gapless playback.

## 🔖 Install

The application is only available on GitHub for now, but will probably come to `crates.io` at a later time. Do note that for `opus` support you need `libopus` installed along with `cmake`.

```sh
cargo install --git https://github.com/hikikones/trollstov --features opus
```
