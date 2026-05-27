# ConfigPilot

[![CI](https://github.com/itscyrus-dev/confpilot/actions/workflows/ci.yml/badge.svg)](https://github.com/itscyrus-dev/confpilot/actions/workflows/ci.yml)

ConfigPilot is a desktop configuration backup and synchronization app for macOS. It is built with Tauri 2, Rust, React, TypeScript, Vite, and Tailwind CSS.

The app focuses on personal shell and terminal configuration files, starting with zsh and Ghostty. It authenticates with GitHub, creates or reuses a private repository named `configpilot-dotfiles`, keeps a local Git-backed cache in the application data directory, and provides manual sync, restore, file watching, and conflict handling.

## Features

- Scan common zsh configuration files:
  - `~/.zshrc`
  - `~/.zprofile`
  - `~/.zshenv`
  - `~/.config/zsh/`
- Scan Ghostty configuration:
  - `~/.config/ghostty/config`
  - `~/.config/ghostty/`
- Authenticate with GitHub using OAuth browser login or device flow.
- Create or reuse a private GitHub repository named `configpilot-dotfiles`.
- Store a local Git workspace in the app data directory.
- Back up local configuration files to GitHub.
- Restore configuration files from the synced repository.
- Run bidirectional sync against the local cache and remote repository.
- Watch local configuration files and sync changes automatically.
- Preserve both local and remote copies when conflicts are detected.
- Store GitHub tokens through the operating system credential store.

## Tech Stack

- **Desktop runtime:** Tauri 2
- **Backend:** Rust
- **Frontend:** React 18, TypeScript, Vite
- **UI:** Tailwind CSS, Radix UI primitives, lucide-react icons
- **Sync engine:** Git repository cache plus GitHub API integration
- **Credential storage:** system keychain through the Rust `keyring` crate

## Repository Layout

```text
.
├── src/                  # React frontend
├── src-tauri/            # Tauri and Rust backend
├── docs/                 # Project documentation
├── index.html            # Vite entry document
├── package.json          # Frontend scripts and dependencies
├── pnpm-lock.yaml        # pnpm lockfile
├── vite.config.ts        # Vite configuration
└── README.md
```

## Requirements

- macOS for the primary desktop target.
- Node.js 20 or newer.
- pnpm 9 or newer.
- Rust stable.
- Tauri system dependencies for your operating system.
- A GitHub OAuth app if you want to test GitHub authentication locally.

For Linux CI or Linux development, Tauri also needs WebKit and GTK development packages. See `.github/workflows/ci.yml` for the exact Ubuntu packages used by the automated checks.

## GitHub OAuth Setup

Create a GitHub OAuth app and configure the callback URL as:

```text
http://127.0.0.1:39119/callback
```

ConfigPilot temporarily listens on `127.0.0.1:39119` during browser-based login so it can receive the OAuth callback.

Copy the example environment file:

```bash
cp .env.example .env
```

Then set your credentials:

```text
CONFIGPILOT_GITHUB_CLIENT_ID=your-github-oauth-client-id
CONFIGPILOT_GITHUB_CLIENT_SECRET=your-github-oauth-client-secret
```

You can also provide the same values as shell environment variables before starting the app.

## Development

Install dependencies:

```bash
pnpm install
```

Run the Tauri desktop app in development mode:

```bash
pnpm tauri:dev
```

Run only the Vite frontend:

```bash
pnpm dev
```

Build the frontend:

```bash
pnpm build
```

Build the desktop app:

```bash
pnpm tauri:build
```

## Releases

GitHub Actions can create draft releases and upload desktop bundles automatically.

Create and push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds:

- macOS Apple Silicon: `.app` and `.dmg`
- macOS Intel: `.app` and `.dmg`
- Windows x64: NSIS `.exe` and MSI `.msi`

You can also start the same workflow manually from the GitHub Actions tab and provide a release tag such as `v0.1.0`.

## Quality Checks

Run the frontend production build:

```bash
pnpm build
```

Check Rust formatting:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Run Clippy:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Run Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

The GitHub Actions workflow runs these checks automatically on pushes and pull requests.

## Synced Repository Format

ConfigPilot writes configuration files into the GitHub repository using a stable layout:

```text
zsh/
  .zshrc
  .zprofile
  .zshenv
  config-zsh/
ghostty/
  config
  ghostty/
manifest.json
```

`manifest.json` records source paths, target paths, file hashes, and sync timestamps.

## Security Model

ConfigPilot is intentionally conservative about what it syncs.

- It does not sync SSH keys, Git credentials, browser profiles, system keychains, or other high-risk secrets.
- GitHub tokens are stored in the operating system credential store.
- The default sync repository is private.
- Conflict resolution is explicit. The app keeps local and remote conflict copies instead of silently overwriting user files.
- The local repository cache is stored in the application data directory, not in the project source tree.

Review all configuration files before syncing them to any remote service.

## Roadmap

- Expand supported configuration sources.
- Add richer conflict diff and merge flows.
- Add signed release packaging.
- Add optional selective sync profiles.
- Add automated frontend tests.

## Contributing

Contributions are welcome.

1. Fork the repository.
2. Create a feature branch.
3. Install dependencies with `pnpm install`.
4. Make focused changes.
5. Run the quality checks listed above.
6. Open a pull request with a clear description of the change and any relevant screenshots.

Please keep changes small, tested, and aligned with the current Tauri 2 and React architecture.

## License

No license file is currently included. Add an open-source license before distributing or accepting external contributions.
