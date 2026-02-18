# batchi-demo-sounds

Example sound files for [batchi](https://github.com/pengo/batchi), a bat call analysis tool.

## File conventions

- Audio files live in `sounds/`
- Each audio file has a matching `.xc.json` metadata sidecar (e.g. `Recording.wav` + `Recording.xc.json`)
- `index.json` at the repo root lists all available demo files

## Licensing

Each recording is individually licensed. See the `.xc.json` metadata file for the `lic` (license URL), `rec` (recordist), and `attribution` fields. Most recordings from xeno-canto are Creative Commons licensed.

## xc-fetch tool

A Rust CLI tool in `tools/xc-fetch/` fetches recording metadata (and optionally audio) from the [xeno-canto API v3](https://xeno-canto.org/explore/api).

### Setup

You need a xeno-canto API key. Set it as an environment variable:

```bash
export XC_API_KEY=your_key_here
```

### Usage

```bash
cd tools/xc-fetch
cargo run -- 928094                          # fetch metadata only
cargo run -- 928094 --download               # fetch metadata + download audio
cargo run -- 928094 --output-dir ../../sounds # write to sounds directory
```

The tool generates:
- `XC{id} - {English name} - {Genus species}.xc.json` (metadata)
- `XC{id} - {English name} - {Genus species}.wav` (audio, with `--download`)

Remember to update `index.json` after adding new files.
