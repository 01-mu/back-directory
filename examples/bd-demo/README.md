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
asciinema rec examples/bd-demo/bd-demo.cast \
  --command "bash -lc 'cd examples/bd-demo && ./demo.sh'"
```

The recording starts in `examples/bd-demo`, and all demo paths are relative to that directory.

## Re-record

```bash
rm -f examples/bd-demo/bd-demo.cast
asciinema rec examples/bd-demo/bd-demo.cast \
  --command "bash -lc 'cd examples/bd-demo && ./demo.sh'"
```

## Play

```bash
asciinema play examples/bd-demo/bd-demo.cast
```

## Upload

```bash
asciinema upload --server-url https://asciinema.org examples/bd-demo/bd-demo.cast
```

If this machine is not linked to an asciinema.org account, authenticate first:

```bash
asciinema auth --server-url https://asciinema.org
```

After uploading, replace the asciinema URL in the repository root `README.md`.

## Notes

- The `bd-demo.cast` file is intentionally ignored by git.
- The demo relies on directory structure only; no prompt customization is used.
