# Weibo Downloader Runtime and Export Design

**Goal:** Fix the current post-login/runtime regressions in the Weibo Downloader, improve Markdown export readability, and apply low-risk theme consistency improvements without expanding scope beyond the user-reported issues.

**Current state:** The app is a Tauri 2 + React + TypeScript desktop client with a Rust backend. Recent uncommitted work replaced the earlier WebView-JS API bridge with a reqwest-based backend flow plus persisted cookies, but several runtime gaps remain: restored login state is inconsistent between frontend and backend, the open-folder button is unwired at the Tauri plugin level, long-post expansion is still unreliable, Markdown output is structurally weak, and theme handling is only partially consistent.

## Scope

This design covers only the currently approved follow-up work:

1. Fix login/session restore behavior so a previously logged-in user can continue directly.
2. Fix the download output directory button.
3. Improve long-post expansion correctness.
4. Redesign Markdown export for better reading experience.
5. Apply safe theme consistency improvements.
6. Clean up dead state left from the removed WebView bridge.
7. Run manual QA, tests, and rebuild packaging after the fixes.

Out of scope:

- Rebranding, logo redesign, or app-store style marketing polish
- Large UI rearchitecture beyond the already-approved wizard flow
- New export formats
- Forced WebView theme hacks if the platform support is brittle or non-portable
- Broad refactors unrelated to the issues above

## Constraints

- Keep the solution aligned with the current reqwest + saved-cookie architecture.
- Do not reintroduce the removed WebView API bridge.
- Keep bug fixes focused; do not bundle unrelated refactors.
- Prefer low-risk theme improvements over fragile platform-specific tricks.
- Preserve the current user-facing wizard flow.

## Problem Summary

### 1. Session restore is split across frontend and backend

The frontend can load a saved cookie and mark the user as logged in on startup, but the backend `AppState` is still the authority used by the download command and API client. That means a restored session can appear valid in the UI while still failing in the backend. The startup flow also does not reliably advance the wizard when the cookie is restored from disk.

### 2. The open-folder button is wired in the UI but unsupported in Tauri

The frontend invokes an opener command, but the backend dependencies/plugin registration/permissions are not wired for that path. The button is therefore present but non-functional.

### 3. Long-post expansion is still heuristic-heavy

The downloader currently relies in part on string checks such as `展开` and `全文`, which is not a robust primary signal. Nested retweeted posts can also miss expansion.

### 4. Markdown export is structurally weak

Current Markdown output strips too much information, uses a generic single-file name, loses link URLs, and provides weak navigation for per-post export. The formatting is also not optimized for reading.

### 5. Theme consistency is incomplete

The app shell uses CSS variables and OS theme detection, but several step components still use hardcoded icon colors. There is no reliable confirmed mechanism yet for fully forcing the embedded login WebView into the same appearance mode as the main UI.

## Design

### A. Session and authentication model

The backend remains the single source of truth for whether a session is available for downloading.

#### Startup flow

1. App startup checks whether a saved cookie exists.
2. If one exists, the frontend requests a backend restore command instead of only reading the saved cookie for its own state.
3. The backend restore path loads the persisted cookie into `AppState` and returns the restored value or a simple success payload.
4. The frontend sets local logged-in state from that result and unlocks the next wizard step immediately.
5. The login step still shows `退出登录` and also shows the path forward without requiring a fresh login.

#### Validity handling

The app should not assume that a persisted cookie is still valid forever.

- If the backend later detects an auth-invalid API response, the app should:
  1. clear the saved cookie,
  2. clear backend in-memory cookie state,
  3. return the user to the login step with a clear re-login message.

This keeps the failure close to the auth boundary instead of surfacing as a confusing later-stage runtime error.

#### Why this approach

This removes the current split-brain problem where the frontend believes the user is logged in but the backend rejects the request. It also keeps the current architecture intact instead of adding another session source.

### B. Open output directory behavior

The existing UI behavior stays the same: after download completes, the button opens the output directory.

Implementation design:

1. Add the real Tauri opener plugin dependency.
2. Register the opener plugin during app initialization.
3. Add the required capability permission for opening a local path.
4. Keep the existing frontend action shape if possible so the UI surface remains unchanged.

This is intentionally narrow: the goal is to make the current button work, not redesign post-download actions.

### C. Long-post content normalization

Long-post handling should be moved to a content-normalization step before export formatting.

#### Normalization rules

For each downloaded post:

1. Decide whether the post requires long-text expansion.
2. Use the dedicated long-text API as the primary source.
3. If the top-level post is expanded, apply the same logic to `retweeted_status` when present.
4. Only use existing text heuristics as fallback signals when structured signals are insufficient.
5. Store normalized final text into the exportable post structure before Markdown/PDF formatting starts.

#### Why this approach

Exports should not need to guess whether a post was truncated. The downloader should hand exporters a normalized representation of the final content.

### D. Markdown export redesign

The default target style is **hybrid**: prioritize reading comfort while still preserving useful metadata.

#### Shared formatting rules

Both single-file and per-post export should use one shared formatting path so content rendering stays consistent.

The formatter should:

- present the readable body first,
- preserve line breaks sensibly,
- retain URLs from links instead of stripping them entirely,
- show metadata below the body rather than before it,
- include clearer author/date/source metadata,
- use a user-specific export title/filename where available.

#### Single-file export

The single-file export should:

- use a descriptive filename tied to the blogger/user when possible,
- include a document header with user identity and export scope,
- render posts in chronological order consistent with current downloader expectations.

#### Per-post export

The per-post export should:

- keep one file per post,
- continue using sanitized filenames,
- add an index file for navigation,
- make each post file self-contained and readable on its own.

#### Metadata layout

Recommended per-post structure:

1. Title/header line
2. Main post body
3. Embedded image references if included
4. Retweet/forwarded section if present
5. Metadata block at the bottom

This keeps the reading flow clean while preserving archive value.

### E. Theme handling

Theme work should stay intentionally conservative.

#### Safe improvements

1. Centralize color usage through shared CSS variables/tokens.
2. Replace hardcoded inline icon colors in step components with theme-aware values.
3. Keep the app shell consistent in light/dark mode using the existing CSS-driven model.

#### WebView treatment

The embedded login WebView should only be theme-synced if there is a clean, low-risk, cross-platform path available in current Tauri/WebView support.

If such a path is not available or is fragile:

- do not force a brittle workaround,
- leave the WebView behavior unchanged,
- explicitly document that limitation to the user.

This prevents introducing unstable platform-specific behavior for a cosmetic improvement.

### F. Cleanup

The removed WebView API bridge left stale state and code paths behind. Cleanup should remove dead structures that are no longer used by the current reqwest-based architecture.

Likely cleanup targets include old request-result storage state and any startup/event wiring that only existed for the removed bridge.

README should also be updated so it does not reference removed commands or stale architecture.

## Files Expected to Change

Frontend:

- `src/App.tsx`
- `src/components/StepLogin.tsx`
- `src/components/StepDownload.tsx`
- `src/App.css`
- `src/lib/tauri-bridge.ts`

Backend:

- `src-tauri/src/lib.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands/download.rs`
- `src-tauri/src/services/weibo_api.rs`
- `src-tauri/src/services/downloader.rs`
- `src-tauri/src/services/export_markdown.rs`
- `src-tauri/src/services/export_pdf.rs` (if shared text/link normalization is extracted)
- `src-tauri/src/models/weibo.rs` (if normalization needs model changes)
- `src-tauri/Cargo.toml`
- `src-tauri/capabilities/default.json`

Docs:

- `README.md`

## Data Flow

### Login restore flow

1. App boots.
2. Frontend asks backend whether a saved cookie exists.
3. Backend restore command loads cookie into `AppState`.
4. Frontend updates wizard/login state from backend response.
5. User proceeds directly to target/options/path steps.

### Download flow

1. User starts download.
2. Backend validates cookie presence from `AppState`.
3. Downloader fetches posts.
4. Downloader expands long text and retweeted long text.
5. Downloader passes normalized content to export layer.
6. Export layer renders Markdown/PDF.
7. Completion UI offers open-folder action.

## Error Handling

- Missing/invalid restored cookie should downgrade to logged-out state cleanly.
- Open-folder failure should surface a direct user-facing error instead of silently failing.
- Long-text fetch failure should fall back predictably and preserve partial content only when necessary.
- Markdown formatting should avoid dropping URLs or destroying line breaks when HTML cleanup runs.

## Testing and Verification Strategy

### Automated

- Add focused regression tests for session restore behavior where practical.
- Add tests for long-text normalization, especially retweeted long posts.
- Add tests for Markdown formatting so links, metadata placement, and single/per-post selection stay correct.
- Keep `cargo test`, `cargo check`, and `npx tsc --noEmit` in the verification path.

### Manual QA

Manual QA is required before rebuilding installers.

Minimum checklist:

1. Launch app with saved valid cookie → login step shows logged-in state and next-step path.
2. Launch app with invalid/stale cookie → app returns cleanly to login.
3. Complete a download with long posts → exported content includes full text.
4. Export and click open-folder button → local output directory opens.
5. Verify Markdown readability in both single-file and per-post modes.
6. Verify light/dark theme behavior in main UI and confirm whether WebView theming is supported or intentionally left unchanged.
7. Run final package build after verification passes.

## Risks and Decisions

### Risk: stale cookie looks valid at startup

Mitigation: backend-auth failure clears session and pushes user back to login.

### Risk: WebView appearance cannot be reliably synchronized

Decision: treat WebView theme sync as optional and low-priority; do not ship a fragile workaround.

### Risk: duplicated text-cleaning logic between Markdown and PDF exporters

Decision: only extract a shared helper if needed by the approved fixes. Do not broaden scope into a large exporter refactor.

## Success Criteria

This design is successful when:

1. Restored login state works end-to-end, not just visually.
2. The next-step path appears correctly for already logged-in users.
3. The open-folder button works on the packaged app.
4. Long posts export with full content, including retweeted long posts when present.
5. Markdown exports are noticeably easier to read and preserve important links/metadata.
6. Main UI theme consistency improves without introducing brittle WebView hacks.
7. Dead bridge state is removed and verification/build/package steps pass.
