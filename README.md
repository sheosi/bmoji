# Bmoji - Emoji Picker

Bmoji is a simple desktop-agnostic and fast emoji picker.

Made with [Rust](https://www.rust-lang.org/) and [Iced](https://iced.rs/).

## Features:
* Simple: Just launch, select your emoji will be copied to the clipboard
* Automatically exits: Once an emoji is selected, when you press ESC or when focus is lost bmoji will understand it is not needed anymore and close itself.
* Fast: The GUI is written in the fast compiled languaje [Rust](https://www.rust-lang.org/) and the lightweight toolkit [Iced](https://iced.rs/). The search is provided with the search engine [Tantivy](https://github.com/quickwit-oss/tantivy).
* Desktop-agnostic: It does not load any desktop-specific framework, and the GUI is lightweight enough that can fit anywhere. While right now only GNOME settings are taking effect feel free to open an issue on how bmoji can take into account your desktop environment.
* Dark mode: Supports dark mode and will automatically load it when set on GNOME desktops (fill an issue on how to read the value on another desktop).  
