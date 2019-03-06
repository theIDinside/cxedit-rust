# cxedit - editor written in rust
A text editor completely written in rust.

Uses termios for terminal settings etc.

## Functionality
Some basic functionality that is currently working:

    - Editing of characters
    - Displaying text properly (scrolling not implemented yet... will look absurd when linecount longer than screen height)
    - Undo operations (redo operations is basically just popping from the "forward history" list, so it's about 95% done
    - Save & open from status line with Ctrl + s and Ctrl + o
    

Functionality I'm working on:

    - Input modes a lá Vim
    - Redo
    - Jump to <line>
    - Find <char range>
    - Inter process communication (for use to implement calls to RLS or racer for example and other language servers)
    - Syntax coloring (not relly hard at all)
    - Configuration (begun on, no real functionality yet though)
    - Multiple buffers & multiple views
    - Overlays / "frames" inside the views


How to open Cargo.toml form root folder:
```bash
    cargo run -- Cargo.toml
```
or
```bash
    ./target/debug/cxedit Cargo.toml
```
![Example color setup](one_color_setup.png)
