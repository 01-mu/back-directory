# back-directory (bd)

A tiny zsh utility to jump back through directory history, with a single-step cancel of the most recent `bd` command.

## Install

```zsh
curl -fsSL https://raw.githubusercontent.com/01-mu/back-directory/main/bd.zsh -o ~/.zsh/bd.zsh

echo 'source ~/.zsh/bd.zsh' >> ~/.zshrc
```

## Usage

```zsh
bd       # same as: bd 1
bd 3     # go back 3 directories
bd c     # cancel the last bd command (only if the previous command was bd ...)
```

Optional alias:

```zsh
bd cancel
```

## Notes

- History and state live in `~/.cache/back-directory/`.
- State includes a cursor, last `bd` delta, and last command tracking for cancel logic.
- Directory changes are captured via zsh hooks (`chpwd` and `precmd`), no `cd` wrapper.
- Multiple shells share the same history; last writer wins if concurrent updates happen.
