# batchi-demo-sounds

Example sound files for [batchi](https://github.com/pengo/batchi), a bat call analysis tool.

## File conventions

- Audio files live in `sounds/`
- Each audio file has a matching `.xc.json` metadata sidecar (e.g. `Recording.wav` + `Recording.xc.json`)
- `index.json` at the repo root lists all available demo files

## Licensing

Each recording is individually licensed. See the `.xc.json` metadata file for the `lic` (license URL), `rec` (recordist), and `attribution` fields. Most recordings from xeno-canto are Creative Commons licensed.

## Adding recordings with xc-fetch

The `xc-fetch` CLI tool (in the main [batchi](https://github.com/pengo/batchi) repo under `xc-cli/`) fetches recordings from the [xeno-canto API v3](https://xeno-canto.org/explore/api).

### Setup

```bash
# From the batchi repo root:
cargo run -p xc-cli -- set-key YOUR_XC_API_KEY
```

Or set `XC_API_KEY` as an environment variable, or add it to a `.env` file.

### Usage

```bash
# Fetch a recording into this repo:
cargo run -p xc-cli -- fetch 928094 --cache-dir batchi-demo-sounds

# Metadata only (no audio download):
cargo run -p xc-cli -- fetch 928094 --metadata-only --cache-dir batchi-demo-sounds

# Browse bat species:
cargo run -p xc-cli -- browse bats
```

The tool generates:

- `sounds/XC{id} - {English name} - {Genus species}.{ext}` (audio)
- `sounds/XC{id} - {English name} - {Genus species}.xc.json` (metadata sidecar)
- Updates `index.json` automatically
