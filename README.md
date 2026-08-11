# Meety

**Local-first meeting transcription for macOS, Windows, and Linux.**
Captures system audio and microphone as separate streams.
Transcribes on-device with GPU acceleration where available.
Writes a markdown note per meeting to a vault you own.
Audio never leaves your machine on the default path.

Meety is a cross-platform fork of [Folio](https://github.com/woosal1337/folio)
by Ege Celebi.

This fork changes the project direction in several ways:

- **New maintainer.** Zinzan now maintains the project.
- **New license.** New code uses MIT. The original Folio code stays Apache-2.0.
- **Clean documentation.** The doc set is smaller. Each file has one purpose.
- **Stable build process.** CI tests run on every change. The release pipeline
  builds and signs the app automatically.
- **Linux support.** The Linux CI no longer crashes during teardown.
- **Renamed packages.** All crate names use the `meety` prefix.
- **Workspaces only.** The CLI and the desktop app share one workspace crate
  (`meety-core`). There is no duplicate code.

**Status:** `2026-08-10.R0` — Alpha. Build from source.

## What it does

- Sits in the system tray and waits for meetings.
- Records system audio and microphone as two separate WAV streams.
- Transcribes locally with whisper.cpp (Metal on macOS, Vulkan on
  Windows and Linux).
- Labels speakers on-device with pyannote segmentation and a
  speaker-embedding model. Your microphone is always labelled **You**.
- Writes a markdown note per meeting to your vault.
- Runs a local MCP server (`meety-mcp`). Tools like Claude Desktop and
  Cursor can read your transcripts, tasks, and memories over stdio.
- Audio never leaves your machine on the default path. Privacy Mode
  blocks all outbound HTTP except localhost.

## Privacy

No telemetry, no analytics, no crash reporting. CI enforces this.
Audio, transcripts, and notes stay on your machine on the default path.
The only network calls are the opt-in cloud transcription path, the
webhook path, and the one-time model download.
Full details are in [`docs/PRIVACY.md`](./docs/PRIVACY.md).

Recording a conversation can be illegal without consent from all
participants. The rules vary by US state and by country. Meety gives
you the tool. You must obtain consent. Tell people before you record.
See [`docs/PRIVACY.md`](./docs/PRIVACY.md#recording-consent).

## Requirements

- macOS 13+, Windows 10+, or Linux with PipeWire or PulseAudio.
- Rust 1.88 via [`rustup`](https://rustup.rs/).
- [Bun](https://bun.sh) 1.3+.
- On macOS: Xcode command-line tools (`xcode-select --install`).
- On Linux: development packages for PipeWire or PulseAudio.

The first time you use local transcription or diarization, Meety
downloads the model weights from Hugging Face. This is a few hundred MB.

## Build from source

```sh
git clone https://github.com/zinzan-vdm/app-meety.git
cd meety
bun install
pre-commit install
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push
```

### Run the desktop app

```sh
bun tauri dev
```

The first build compiles the Rust workspace. This takes about
30 seconds on a warm cache.

### Run the CLI test harness

```sh
# List audio input devices
cargo run -p meety-cli --release -- devices

# Record from the default device for 60 seconds
cargo run -p meety-cli --release -- record --seconds 60

# Record from a specific device
cargo run -p meety-cli --release -- record --seconds 60 --mic-device "Microphone Name"
```

Output: `./recordings/<timestamp>/mic.wav` (mono 16-bit PCM at the
device sample rate).

### Local checks

```sh
# Rust
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --bins

# Frontend
bun run typecheck
bun run lint
bun run format:check
bun run test
```

The pre-commit hooks run the relevant checks on each commit.
The full suite also runs in `.github/workflows/ci.yml`.

## Repository layout

```
meety/
├── Cargo.toml                # workspace root
├── crates/
│   ├── meety-core/          # audio capture, storage, transcription, diarization
│   ├── meety-cli/           # CLI test harness (binary: meety-cli)
│   └── meety-mcp/           # local MCP stdio server (binary: meety-mcp)
├── src-tauri/                # Tauri 2 desktop binary
├── src/                      # React + TypeScript + Tailwind frontend
├── docs/                     # design docs (see ARCHITECTURE.md)
└── .github/workflows/        # CI + release pipelines
```

**Stack:**
Rust core for audio, storage, and transcription.
Tauri 2 wraps a React + TypeScript + Tailwind frontend.
`cpal` for microphone capture.
`ScreenCaptureKit` (macOS) and PulseAudio monitor source (Linux) for
system audio capture.
`whisper-rs` for local Whisper inference.
`sherpa-onnx` for on-device speaker diarization.

## MCP server

Meety ships a local MCP server that runs over stdio.
Add this to your MCP client configuration:

```json
{
  "mcpServers": {
    "meety": {
      "command": "meety-mcp"
    }
  }
}
```

The server provides tools to search transcripts, find decisions, list
tasks, and query meeting notes. No network access is needed.

## Contributors

- [zinzan-vdm](https://github.com/zinzan-vdm) — maintainer and primary contributor

## License

MIT. See [LICENSE](./LICENSE).
Third-party attributions are in [NOTICE](./NOTICE).

Forked from [Folio](https://github.com/woosal1337/folio) by Ege Celebi (Apache-2.0).
New code is MIT.
