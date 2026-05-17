<p align="center">
  <img src="src-tauri/icons/logo.svg" alt="Codex Switcher" width="128" height="128">
</p>

<h1 align="center">Codex Switcher</h1>

<p align="center">
  A Desktop Application for Managing Multiple OpenAI <a href="https://github.com/openai/codex">Codex CLI</a> Accounts<br>
  Easily switch between accounts, monitor usage limits, and stay in control of your quota
</p>

## About This Fork

This repository is a downstream fork of the original
[Lampese/codex-switcher](https://github.com/Lampese/codex-switcher) project.

The goal of this fork is to keep the original multi-account workflow while
adding features that make the app more practical for restricted networks,
headless/remote setups, and newer ChatGPT login flows.

Notable additions in this fork:

- Configurable HTTP proxy support for ChatGPT/OpenAI requests
- ChatGPT device-code login as an alternative to browser OAuth
- Proxy-aware device login, token refresh, usage checks, and warm-up requests
- LAN/browser dashboard mode through the `codex-web` helper
- Masked accounts for hiding selected accounts from the main list
- Full encrypted account backup/import and slim text export/import
- Subscription expiry tracking in ChatGPT account metadata

## Features

- **Multi-Account Management** – Add and manage multiple Codex accounts in one place
- **Quick Switching** – Switch between accounts with a single click
- **Usage Monitoring** – View real-time usage for both 5-hour and weekly limits
- **Multiple Login Modes** – Browser OAuth, ChatGPT device-code login, or import existing `auth.json` files
- **Proxy Settings** – Configure an HTTP proxy for auth, usage, warm-up, and token refresh requests
- **Backup and Import** – Export/import slim text payloads or full encrypted backups
- **LAN Dashboard** – Run the same dashboard in a browser through the bundled web helper

## Installation

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [pnpm](https://pnpm.io/)
- [Rust](https://rustup.rs/)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/petrovichest/codex-switcher.git
cd codex-switcher

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

### Run the Dashboard in a Browser

You can also serve the built dashboard over HTTP instead of opening the Tauri shell.

```bash
# Build the frontend and start the web server on 0.0.0.0:3210
pnpm lan
```

Optional environment variables:

- `CODEX_SWITCHER_WEB_HOST` to override the bind host
- `CODEX_SWITCHER_WEB_PORT` to override the port

The browser dashboard serves the same UI and backend actions through `/api/invoke/*`, which makes it usable over LAN, Tailscale, or a remote host tunnel when you expose the chosen port safely.

## Disclaimer

This tool is designed **exclusively for individuals who personally own multiple OpenAI/ChatGPT accounts**. It is intended to help users manage their own accounts more conveniently.

**This tool is NOT intended for:**

- Sharing accounts between multiple users
- Circumventing OpenAI's terms of service
- Any form of account pooling or credential sharing

By using this software, you agree that you are the rightful owner of all accounts you add to the application. The authors are not responsible for any misuse or violations of OpenAI's terms of service.

## Versioning

Use the version bump helper to keep app versions in sync across Tauri, Cargo, and the frontend.

```bash
# Exact version
pnpm version:bump 0.2.1

# Semver bumps
pnpm version:patch
pnpm version:minor
pnpm version:major

# Prepare a release commit and tag
# This automatically runs the version bump first.
pnpm release patch

# Prepare and push a release
# This automatically runs the version bump first.
pnpm release patch -- --push
```
