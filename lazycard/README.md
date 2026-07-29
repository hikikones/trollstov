<div align="center">

# lazycard

A simple flashcard application for the terminal.

<figure>
<img src="https://github.com/user-attachments/assets/dadf08a3-072f-4b51-8435-16e28ec06472"/>
<p><em>The flashcard application in the Monokai Soda color scheme.</em></p>
</figure>

</div>

`lazycard` is a flashcard application for the terminal that helps you retain information using [spaced repetition][spaced_repetition]. Cards you find difficult are reviewed more often, while cards you know well appear less frequently, helping you focus your study time where it matters most.

## 📌 Features

- Write cards in a light markup
- Add tags to your cards
- Card scheduling using the [Free Spaced Repetition Scheduler][fsrs]
- Image support (including animated) with [kitty graphics protocol][kitty_graphics_protocol]
- Code blocks with syntax highlighting
- Mathematics with [KaTeX][katex] rendered as images

## 📜 Markup

Cards are written in a custom lightweight markup language, inspired by both [Markdown](https://en.wikipedia.org/wiki/Markdown) and [Djot](https://djot.net/). It provides a small set of syntax for formatting text in the terminal.

Use `---` to mark reveal points in a card. You can have as many as you like. During review, cards are revealed one section at a time until the entire card is visible. At that point, you simply answer yes or no based on whether you successfully recalled everything.

A card with no reveal marks is considered a plain note, and will not show up in reviews.

The following table shows the entire syntax available.

<table align="center">
<tr>
<th>Markup</th>
<th>Result</th>
</tr>
<tr>
<td>

<pre>
A normal paragraph with *bold* and _italic_ text.

| Center paragraph

> Right paragraph

# This is a reveal marker
---

- item 1
- item 2

![image description](image.jpeg)

# Another reveal marker
---

$$ \int x^2 \ dx $$

```python
def add(a, b):
    return a + b
```
</pre>

</td>
<td>

<p>A normal paragraph with <b>bold</b> and <i>italic</i> text.</p>
<p align="center">Center paragraph</p>
<p align="right">Right paragraph</p>

<hr>

- item 1
- item 2

<div align="center">
<figure>
<img src="https://github.com/user-attachments/assets/269f45c2-3164-4310-a7c3-0ad898e951e7"/>
<p>image description</p>
</figure>
</div>

<hr>

$$ \int x^2 \ dx $$

<pre>
def add(a, b):
    return a + b
</pre>

</td>
</tr>
</table>

## 🔖 Install

```sh
cargo install --bin lazycard --git https://github.com/hikikones/trollstov
```

## ⚡ Usage

The `lazycard` command takes no mandatory arguments, but you can supply it with options for where your database file should be, along with the assets directory and the settings file.

```console
Usage: lazycard [OPTIONS]

Example: lazycard --database /path/to/my/database.db --assets /path/to/my/assets

Options:
      --database <FILE.db>    Optional path for your database file. By default, the location will be determined by the conventions of your operating system.
      --assets <DIR>          Optional path for your assets directory. By default, the location will be determined by the conventions of your operating system.
      --settings <FILE.toml>  Optional path for your settings file. By default, the location will be determined by the conventions of your operating system.
  -h, --help                  Print help.
  -V, --version               Print version.
```

## ⬛ Terminal Support

Currently the only supported terminal is [kitty][kitty_terminal], as that is the only terminal that fully supports the [kitty graphics protocol][kitty_graphics_protocol]. This includes animated images with scaling and cropping.

## 💻 Platform Support

The application is mainly developed on Linux, as that is what I use, but hopefully it also works on Windows and macOS. I try to keep cross-platform in mind when developing, but have no means of testing it.

## ⚠️ Non-goals

- Supporting most terminals.

[spaced_repetition]: https://en.wikipedia.org/wiki/Spaced_repetition
[fsrs]: https://github.com/open-spaced-repetition/awesome-fsrs/wiki/The-Algorithm
[kitty_terminal]: https://sw.kovidgoyal.net/kitty
[kitty_graphics_protocol]: https://sw.kovidgoyal.net/kitty/graphics-protocol
[katex]: https://katex.org