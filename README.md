# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Release Hardening Notes

- Rust release builds now use stronger optimization and symbol stripping (`src-tauri/Cargo.toml`).
- Frontend production builds disable sourcemaps and drop `console` / `debugger` (`vite.config.js`).
- Frontend verbose logs are disabled in production by default.
  - Browser side: set `localStorage.MR_VERBOSE_LOG = "1"` to re-enable non-error logs.
  - Rust side: set `MIRRATIV_VERBOSE_FRONTEND_LOG=1` to print full forwarded frontend logs.
