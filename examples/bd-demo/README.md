# back-directory asciinema demo

This directory contains a reproducible asciinema demo for back-directory using a shell-based automation script.

## Files

- `demo.sh`: automated demo script
- `demo-magic.sh`: vendored helper for simulated typing
- `pv`: minimal local shim for typing speed control (avoids external dependency)

## Prerequisites

- `asciinema`
- `bash`
- `bd-core` available on PATH, or a local build at `target/release/bd-core`

## Record

```bash
asciinema rec examples/bd-demo/media/bd-demo.cast \
  --command "bash -lc 'cd examples/bd-demo && ./demo.sh'"
```

The recording starts in `examples/bd-demo`, and all demo paths are relative to that directory.

## Re-record

```bash
rm -f examples/bd-demo/media/bd-demo.cast
asciinema rec examples/bd-demo/media/bd-demo.cast \
  --command "bash -lc 'cd examples/bd-demo && ./demo.sh'"
```

## Play

```bash
asciinema play examples/bd-demo/media/bd-demo.cast
```

## Convert to WebP

Install `agg` (asciinema GIF/WebP generator):

```bash
# macOS (Homebrew)
brew install agg

# Linux (cargo)
cargo install --git https://github.com/asciinema/agg
```

Convert the recording directly to WebP:

```bash
agg --speed 1.25 --font-size 16 --cols 80 --fps-cap 30 \
  examples/bd-demo/media/bd-demo.cast \
  examples/bd-demo/media/bd-demo.webp
```

After converting, replace the demo image in the repository root `README.md` with
`examples/bd-demo/media/bd-demo.webp`.

## Notes

- The demo relies on directory structure only; no prompt customization is used.
