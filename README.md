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

## Setup

In order to use `tiny-dc`, you need to install it and then integrate it with
your shell.

### Step 1: Install `tiny-dc`

You can download the [latest
release](https://github.com/n1ghtmare/tiny-dc/releases)

__or__

you can install `tiny-dc` from cargo:

```bash
cargo install tiny-dc
```

### Step 2: Integrate it with your shell

#### > Bash/Zsh

For `bash` or `zsh`, add the following to your `~/.bashrc` or `~/.zshrc`:

```bash
# Change dc to whatever you want to use
dc() {
    local result=$(command tiny-dc "$@")
    [ -n "$result" ] && cd -- "$result"
}

# Change z to whatever you want to use
# This will allow you to use z as a shortcut for tiny-dc
# It will take you to the most "frecent" directory that you've visited before
z() {
    local result=$(command tiny-dc z "$@")
    [ -n "$result" ] && cd -- "$result"
}

# This is needed for the `z` command to work
# Every time you change directories, it will push the current directory to the
# tiny-dc index
cd() {
    builtin cd "$@" && tiny-dc push "$PWD"
}
```

After adding the above lines, run `source ~/.bashrc` or `source ~/.zshrc` to
reload your shell configuration.

#### > fish

For `fish`, add the following to your `~/.config/fish/config.fish`:

```fish
# Change dc to whatever you want to use
function dc
    set result (tiny-dc $argv)
    if test -n "$result"
        cd $result
    end
end

# Change z to whatever you want to use
# This will allow you to use z as a shortcut for tiny-dc
# It will take you to the most "frecent" directory that you've visited before
function z
    set result (tiny-dc z $argv)
    if test -n "$result"
        cd $result
    end
end

# This is needed for the `z` command to work
# Every time you change directories, it will push the current directory to the
# tiny-dc index
function cd
    builtin cd $argv; and tiny-dc push $PWD
end
```

#### > Nushell

For `nushell`, add the following to your `~/.config/nushell/config.nu`:

```nushell
# Change dc to whatever you want to use
def --env dc [...args: string] {
    let result = (tiny-dc ...$args)
    if $result != "" {
        cd $result
    }
}

# Change z to whatever you want to use
# This will allow you to use z as a shortcut for tiny-dc
# It will take you to the most "frecent" directory that you've visited before
def --env z [...args: string] {
    let result = (tiny-dc z ...$args)
    if $result != "" {
        cd $result
    }
}

# Make sure the built-in cd is saved so we can call it from within our new cd command.
alias cd-orig = cd

# This is needed for the `z` command to work
# Every time you change directories, it will push the current directory to the
# tiny-dc index
def --env cd [dir: path] {
    cd-orig $dir;
    tiny-dc push $dir
}
```

### Step 3: Profit

Now you can use `dc` to open the TUI and navigate through your directories. You can
also use `z` to quickly jump to the most "frecent" directory.

## Keybindings

At any given time when using `tiny-dc`, you can press `?` to see the help menu,
which will show you the keybindings that are available.

Here is a list of keybindings that you can use in the TUI:

| Keybinding | Action |
|------------|--------|
| `h` or `←`            | Go to the parent directory |
| `l` or `→` or `Enter` | Go to the selected directory |
| `j` or `↓`            | Move down one line |
| `k` or `↑`            | Move up one line |
| `gg` or `Home`        | Go to the top of the list |
| `G` or `End`          | Go to the bottom of the list |
| `?`                   | Show help menu |
| `q` or `Esc`          | Quit |
| `/`                   | Search for a directory |
| `_`                   | Clear search |
| `Ctrl + f`            | Switch to frecent category |
| `Ctrl + d`            | Switch to directories category |
