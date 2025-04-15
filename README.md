# Tiny DC

Tiny DC (which stands for Tiny Directory Changer), is a tiny TUI aimed to make
changing directories as efficient as possible.

https://github.com/user-attachments/assets/9db48319-101c-4cdb-ac79-113cc2b78b77

Tiny DC is inspired by [tere](https://github.com/mgunyho/tere) and
[z](https://github.com/rupa/z). It's faster than doing `cd` + `ls` and more
convenient than using a GUI file explorer if you spend your time in the
terminal. It also combines the functionality of `z` to quickly jump to
directories that you've visited before. It's designed to be minimal and fast,
with a focus on usability and performance.

Tiny DC is not a "file manager", it can only be used to browse and change
directories. It doesn't have any manipulation features like copying, moving, or
deleting files/directories.

__NOTE:__ This was only tested on Linux, but it should work on MacOS and WSL as
well. Windows support is still work in progress (PRs are welcome).

## Features

- Fast and responsive TUI interface
- Quickly jump to frequently visited directories (similar to `z`)
- Search for directories by name
- Quickly and efficiently navigate through directories
    - vim-like keybindings + arrow keys
    - dynamic shortcuts for each listed directory (simply press the highlighted
      key/key combination on the right side of the directory name and you will
      be taken to that directory)

## Installation

TODO
