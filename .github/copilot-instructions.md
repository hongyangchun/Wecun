# Copilot Instructions for Wecun

## Build, test, and run commands

### Frontend and Tauri

- `npm install` — install Node dependencies
- `npm run dev` — run the Vite frontend only
- `npm run build` — TypeScript check + production frontend build
- `npm run tauri dev` — run the desktop app in development
- `npm run tauri build` — build desktop bundles

### Rust tests

- `cd src-tauri && cargo test` — run the Rust test suite
- `cd src-tauri && cargo test test_markdown_single` — run a single named test
- `cd src-tauri && cargo test markdown_export_preserves_link_targets` — another targeted test example

### Linting

- There is currently **no dedicated lint script** in `package.json`.
- Frontend type-checking is enforced as part of `npm run build`.

## High-level architecture

This is a **Tauri 2 desktop app** with a **React + TypeScript** wizard frontend and a **Rust backend** that owns login state, Weibo API access, download orchestration, caching, and export.

### Frontend flow

- `src/App.tsx` is the main stateful controller. It implements a 6-step wizard: login → target URL → options → output path → download → export.
- Most files in `src/components/` are presentational step screens. The real app flow stays centralized in `App.tsx`.
- `src/lib/tauri-bridge.ts` is the frontend boundary to Tauri. React code should call Tauri commands and event listeners through this bridge, not inline `invoke()` calls scattered through components.
- `src/types/contracts.ts` defines the request, export, and progress payloads expected by the frontend.

### Backend flow

- `src-tauri/src/lib.rs` wires Tauri plugins, registers commands, and manages a shared `AppState`.
- `src-tauri/src/commands/download.rs` is the command layer for:
  - login window launch
  - download start/cancel
  - export
  - saved-cookie lifecycle
- `src-tauri/src/state.rs` stores cross-command runtime state such as the saved cookie, cancellation flag, and progress counters.
- `src-tauri/src/services/downloader.rs` is the main orchestration path:
  1. fetch user info
  2. fetch paginated post data
  3. expand long-text posts
  4. normalize/filter posts
  5. optionally download images
  6. write the cache for later export
- `src-tauri/src/services/weibo_api.rs` encapsulates Weibo HTTP access, request throttling, login invalidation detection, cookie persistence, and raw-to-normalized post conversion.
- Export is split by format:
  - `export_markdown.rs`
  - `export_html.rs`
  - `export_pdf.rs`

### Download/export data flow

- The download step writes normalized posts to `OUTPUT_DIR/.weibo-cache/posts.json`.
- Export commands do **not** re-fetch Weibo data; they load that cache and render files from it.
- If images were downloaded, they are stored under `OUTPUT_DIR/images/` and exporters prefer local paths over remote URLs.

## Key conventions

### Keep TypeScript and Rust contracts in sync

- The frontend contract in `src/types/contracts.ts` mirrors Rust request/progress models in `src-tauri/src/models/`.
- When changing request fields, progress phase names, or export format values, update **both sides** together.
- Pay attention to serialization details:
  - Rust `ExportFormat` uses serde renames such as `md-single`, `md-multi`, and `html`
  - download request fields are snake_case on the wire (`include_images`, `output_dir`, `date_range`)

### Use Tauri events as the long-running job boundary

- Download and export progress are pushed through the `"download-progress"` event.
- Login/session events use `"cookie-received"` and `"login-invalid"`.
- If you change long-running backend behavior, preserve or intentionally update the emitted phases/messages because the React wizard UI depends on them.

### Login state lives in Tauri, not just the form

- The backend uses `AppState` as the actual source of truth for the cookie.
- Saved cookies are persisted to the app data directory as `weibo_cookie.dat`.
- `start_download` validates against the cookie stored in `AppState`; frontend request payloads alone are not enough.

### Cached export is a core product behavior

- The intended workflow is **download once, export many times**.
- Avoid changes that force export paths to depend on a fresh network fetch.
- Cache compatibility matters because `export_posts` reads `.weibo-cache/posts.json` directly.

### Preserve repo-specific export behavior

- Markdown single-file export writes `<author>-微博导出.md`.
- Per-post Markdown export writes into `posts/` and must keep `posts/index.md`.
- Per-post Markdown image paths are intentionally relative as `../images/...`.
- HTML export sanitizes Weibo HTML with `ammonia` instead of rendering raw HTML directly.
- PDF export depends on loading a CJK font family from `src-tauri/assets/fonts` or system font directories.

### Text normalization rules matter

- Weibo post bodies are stored and exported as HTML-derived content, not plain text from the start.
- Long-text expansion is handled before export normalization.
- The short-post filter uses `count_plain_text_chars()` from `src-tauri/src/utils/html.rs`, so filtering is based on visible text length after stripping HTML.

### Test suite conventions

- Most tests are in `src-tauri/tests/` and exercise parsing, long-text normalization, markdown export behavior, and session restoration.
- Fixture-based parsing tests use `src-tauri/tests/fixtures/`.
- `live_export_smoke.rs` is not a hermetic unit test: it expects a real saved Weibo cookie and network access.

### UI/content conventions

- User-facing copy is primarily in **Simplified Chinese**. Keep new UI text, progress messages, export labels, and filenames consistent with the existing Chinese wording.
- The frontend follows a wizard-style flow with the main state in `App.tsx`; avoid spreading step coordination logic across many components unless there is a strong reason.
