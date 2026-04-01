<div align="center">

# trollstov

Your music is the database.

<figure>
<img src="https://github.com/user-attachments/assets/07913095-1160-4cd9-9eb9-fb6e80b3d95d"/>
<p><em>The music player with the Monokai Soda color scheme.</em></p>
</figure>

</div>

`trollstov` is a music player for the terminal that is built around a simple idea — your files and their metadata are all you need. Simply back up your music directory and you have backed up everything. Ratings are part of the metadata, so your favorite songs are always just a few keystrokes away.

The name is a norwegian word play for a substance with magical properties. You have "troll", a creature from Norse mythology, and "stov" which is actually "støv", meaning "dust" in english. Hence, you get "troll dust".

## 📌 Features

- Your music is the database — your files and their metadata are all you need.
- Portable rating — the rating is part of the metadata.

## ⚡ Usage

The `trollstov` command takes one mandatory argument, which is the path to your music directory. In addition, it comes with a few optional arguments.

```console
Usage: trollstov [OPTIONS] <MUSIC_DIR>

Example: trollstov --media-controls /path/to/my/music

Arguments:
  <MUSIC_DIR>  The directory for your music

Options:
      --settings <SETTINGS_FILE.toml>  Optional path for your settings file. If not set, the location will be determined by the conventions of your operating system.
      --media-controls                 Add system media controls for player interaction with media keys and your operating system.
  -h, --help                           Print help.
  -V, --version                        Print version.
```

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
cargo install --git https://github.com/hikikones/trollstov --tag v0.1.0 --features opus
```
