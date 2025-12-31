# back-directory (bd)

A zsh wrapper plus a Rust core for fast, correct directory backtracking with a single-step cancel.

## User guide

### Install

If you're not sure, use the one-liner. The other options are for advanced users.

#### Recommended: one-liner (GitHub Releases)

```zsh
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/scripts/install.sh | sh
```

This installs the core binary to `${XDG_BIN_HOME:-$HOME/.local/bin}` (creates it if needed),
installs the wrapper to `${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.zsh`, and adds
`source "${XDG_CONFIG_HOME:-$HOME/.config}/back-directory/bd.zsh"` to your `.zshrc`.
If `${XDG_BIN_HOME:-$HOME/.local/bin}` is not on your `PATH`, add this to your shell config:

```zsh
export PATH="$HOME/.local/bin:$PATH"
```

#### Manual: download from Releases

1) Download the matching `.tar.gz` for your OS/arch from the latest GitHub Release:
   - `bd-core-aarch64-apple-darwin.tar.gz`
   - `bd-core-x86_64-apple-darwin.tar.gz`
   - `bd-core-x86_64-unknown-linux-gnu.tar.gz`

2) Extract and install:

```zsh
mkdir -p ~/.local/bin
tar -xzf bd-core-<target>.tar.gz
mv bd-core ~/.local/bin/bd-core
```

#### Developer install (cargo)

```zsh
# from a local clone
cargo install --path .

# or from git
# cargo install --git https://github.com/01-mu/back-directory
```

Ensure `bd-core` is on your `PATH` (default is `~/.cargo/bin`).

#### Wrapper configuration

Start a new shell or `source ~/.zshrc`.

If the core binary lives elsewhere, set `BD_CORE_BIN` before sourcing:

```zsh
export BD_CORE_BIN=/path/to/bd-core
```

### Usage

```zsh
bd       # same as: bd 1
bd 3     # go back 3 directories (1 <= N <= 99)
bd c     # cancel the last bd command in the current session
```

Optional alias:

```zsh
bd cancel
```

Session semantics: `bd` tracks history per session key. By default this is derived from
your terminal TTY and the shell PID, so each shell is its own session. Set
`BD_SESSION_ID` before sourcing if you want to override:

```zsh
export BD_SESSION_ID=work-logs
```

## Developer guide

See `docs/development.md` for implementation details and development workflow.

## Layout

- scripts/: distribution scripts (install.sh, bd.zsh)
- src/: Rust implementation (bd-core)

### Uninstall

Since installation is done by placing files on your PATH and adding shell config,
uninstall is a manual cleanup of those files and changes.

1) Locate the installed binary (the path will be used below):

```zsh
command -v bd
# or
which bd
```

2) Remove the binary at the path printed above (examples):

```zsh
rm -f ~/.local/bin/bd
# or
sudo rm -f /usr/local/bin/bd
```

3) Remove optional config/state/data if present (only if you created them):

```zsh
rm -rf ~/.config/back-directory
rm -rf ~/.local/share/back-directory
rm -rf ~/.cache/back-directory
```

4) Revert shell setup changes:
   - Remove any PATH, alias, or `source .../bd.zsh` lines you added to `.zshrc` or
     `.bashrc`, then restart your shell.
