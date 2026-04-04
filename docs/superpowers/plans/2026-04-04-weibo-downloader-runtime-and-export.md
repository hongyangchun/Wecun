# Weibo Downloader Runtime and Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix restored-session correctness, long-post expansion, open-folder behavior, Markdown readability, and low-risk theme consistency for the Weibo Downloader, then verify the app and rebuild installers.

**Architecture:** Keep the current reqwest + saved-cookie design and make the backend the single authority for session state. Normalize post content in the downloader before export, improve Markdown rendering with a shared formatter path, use the official Tauri opener plugin for the output-directory action, and limit theme work to stable CSS-token/icon changes unless WebView theming has a proven safe hook.

**Tech Stack:** React, TypeScript, Vite, Rust, Tauri 2, reqwest, serde, cargo test, TypeScript compiler, Tauri bundler.

---

## File Structure

- `src/App.tsx`
  - Own startup restore flow, login state transitions, and open-output-directory action wiring.
- `src/components/StepLogin.tsx`
  - Own login step messaging and already-logged-in navigation affordance.
- `src/components/StepDownload.tsx`
  - Own completed-state action button behavior and copy.
- `src/App.css`
  - Own theme tokens and shared icon color variables.
- `src/lib/tauri-bridge.ts`
  - Own frontend bridge commands for session restore and open-folder behavior.
- `src-tauri/src/state.rs`
  - Own backend in-memory session state and remove dead bridge-era fields/helpers.
- `src-tauri/src/commands/download.rs`
  - Own download command auth guard and restore/clear command surface.
- `src-tauri/src/services/weibo_api.rs`
  - Own cookie persistence, restore path, auth-invalid handling, and API fetch logic.
- `src-tauri/src/services/downloader.rs`
  - Own post normalization and long-text expansion before export.
- `src-tauri/src/services/export_markdown.rs`
  - Own shared Markdown formatting for single/per-post output.
- `src-tauri/src/services/export_pdf.rs`
  - Possibly consume shared text-normalization helper if needed without broad refactor.
- `src-tauri/src/models/weibo.rs`
  - Own any model changes needed for normalized long-text handling.
- `src-tauri/src/lib.rs`
  - Own Tauri plugin registration.
- `src-tauri/Cargo.toml`
  - Own opener plugin dependency.
- `src-tauri/capabilities/default.json`
  - Own opener permission.
- `README.md`
  - Own user-facing workflow and smoke-checklist updates.
- `src-tauri/tests/`
  - Own regression coverage for session restore, long-text normalization, and Markdown formatting.

### Task 1: Session restore authority and logged-in step flow

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/StepLogin.tsx`
- Modify: `src/lib/tauri-bridge.ts`
- Modify: `src-tauri/src/commands/download.rs`
- Modify: `src-tauri/src/services/weibo_api.rs`
- Modify: `src-tauri/src/state.rs`
- Test: `src-tauri/tests/session_restore.rs` (new if no existing fit)

- [ ] **Step 1: Write the failing backend regression test for restored cookie state**

```rust
use weibo_downloader::state::AppState;

#[test]
fn restored_cookie_is_loaded_into_backend_state() {
    let state = AppState::default();

    state.set_cookie("SUB=restored_cookie".to_string());

    assert_eq!(state.get_cookie(), "SUB=restored_cookie");
}
```

- [ ] **Step 2: Add a command-level failing test for restore-path behavior**

```rust
#[test]
fn clear_cookie_resets_backend_session_state() {
    let state = AppState::default();
    state.set_cookie("SUB=restored_cookie".to_string());

    state.set_cookie(String::new());

    assert!(state.get_cookie().is_empty());
}
```

- [ ] **Step 3: Run the targeted Rust test file to verify the red state**

Run: `cargo test --test session_restore`

Expected: FAIL if the restore test file or behavior does not exist yet.

- [ ] **Step 4: Implement a single authoritative backend restore path**

```rust
pub fn restore_saved_cookie_into_state(app: &tauri::AppHandle, state: &AppState) -> Result<Option<String>, String> {
    let cookie = load_saved_cookie(app).map_err(|err| err.to_string())?;
    if let Some(value) = cookie.filter(|value| !value.trim().is_empty()) {
        state.set_cookie(value.clone());
        return Ok(Some(value));
    }

    state.set_cookie(String::new());
    Ok(None)
}
```

- [ ] **Step 5: Update the frontend startup flow to use backend restore, then unlock the next step**

```tsx
useEffect(() => {
  let cancelled = false;

  void (async () => {
    const restored = await restoreSavedCookie();
    if (cancelled || !restored) {
      return;
    }

    setCookie(restored);
    setIsLoggedIn(true);
    setStep((prev) => (prev < 1 ? 1 : prev));
  })();

  return () => {
    cancelled = true;
  };
}, []);
```

- [ ] **Step 6: Update login-step UI so already-restored sessions still expose the forward path**

```tsx
{isLoggedIn ? (
  <div className="step-login__actions">
    <button type="button" className="primary-button" onClick={onNext}>
      下一步
    </button>
    <button type="button" className="secondary-button" onClick={onLogout}>
      退出登录
    </button>
  </div>
) : (
  <button type="button" className="primary-button" onClick={onLogin}>
    登录微博
  </button>
)}
```

- [ ] **Step 7: Handle auth-invalid responses by clearing persisted and in-memory session state**

```rust
fn clear_session(app: &tauri::AppHandle, state: &AppState) -> Result<(), String> {
    clear_saved_cookie(app).map_err(|err| err.to_string())?;
    state.set_cookie(String::new());
    Ok(())
}
```

- [ ] **Step 8: Re-run the targeted session restore tests**

Run: `cargo test --test session_restore`

Expected: PASS

- [ ] **Step 9: Run diagnostics on changed frontend/backend files**

Run LSP diagnostics for:
- `src/App.tsx`
- `src/components/StepLogin.tsx`
- `src/lib/tauri-bridge.ts`
- `src-tauri/src/commands/download.rs`
- `src-tauri/src/services/weibo_api.rs`
- `src-tauri/src/state.rs`

Expected: no new errors.

### Task 2: Open output directory button via official opener plugin

**Files:**
- Modify: `src/lib/tauri-bridge.ts`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/capabilities/default.json`
- Test: `src/App.tsx` / manual QA path

- [ ] **Step 1: Write the failing capability/dependency checklist as a manual regression note in the plan execution context**

```text
Current failure expectation:
- openOutputDir invokes opener path from frontend
- Tauri plugin dependency/init/permission are missing
- runtime button click fails to open folder
```

- [ ] **Step 2: Add the opener plugin dependency**

```toml
tauri-plugin-opener = "2"
```

- [ ] **Step 3: Register the opener plugin during Tauri app setup**

```rust
.plugin(tauri_plugin_opener::init())
```

- [ ] **Step 4: Add opener capability permission**

```json
{
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:default",
    "http:default",
    "opener:default"
  ]
}
```

- [ ] **Step 5: Keep the frontend open-folder bridge minimal and explicit**

```ts
export async function openOutputDir(path: string): Promise<void> {
  await invoke("plugin:opener|open_path", { path });
}
```

- [ ] **Step 6: Run backend compile verification for plugin wiring**

Run: `cargo check`

Expected: PASS

- [ ] **Step 7: Run diagnostics on changed files**

Run LSP diagnostics for:
- `src/lib/tauri-bridge.ts`
- `src-tauri/src/lib.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/capabilities/default.json`

Expected: no new errors.

### Task 3: Long-post normalization before export

**Files:**
- Modify: `src-tauri/src/services/downloader.rs`
- Modify: `src-tauri/src/services/weibo_api.rs`
- Modify: `src-tauri/src/models/weibo.rs` (if needed)
- Test: `src-tauri/tests/downloader_long_text.rs` (new if no existing fit)

- [ ] **Step 1: Write a failing long-post test for top-level expansion**

```rust
#[test]
fn expands_top_level_long_post_before_export() {
    let raw = fixture_long_post_summary_only();
    let long_text = "完整长微博正文";

    let normalized = normalize_post_text(raw, Some(long_text.to_string()), None);

    assert_eq!(normalized.text, "完整长微博正文");
}
```

- [ ] **Step 2: Write a failing long-post test for retweeted expansion**

```rust
#[test]
fn expands_retweeted_long_post_before_export() {
    let raw = fixture_retweeted_long_post_summary_only();

    let normalized = normalize_post_text(raw, None, Some("转发里的完整正文".to_string()));

    assert_eq!(
        normalized.retweeted_status.unwrap().text,
        "转发里的完整正文"
    );
}
```

- [ ] **Step 3: Run the targeted long-text test file and verify it fails correctly**

Run: `cargo test --test downloader_long_text`

Expected: FAIL because normalization helper/behavior is incomplete.

- [ ] **Step 4: Add a single normalization helper that prefers structured long-text expansion over string heuristics**

```rust
fn needs_long_text(raw: &RawPost) -> bool {
    raw.is_long_text || raw.text.contains("展开") || raw.text.contains("全文")
}

async fn normalize_post(
    api: &WeiboApiClient,
    raw: RawPost,
) -> Result<RawPost, AppError> {
    let mut normalized = raw;
    if needs_long_text(&normalized) {
        if let Ok(full_text) = api.get_long_text(&normalized.mblogid).await {
            normalized.text = full_text;
        }
    }

    if let Some(retweeted) = normalized.retweeted_status.take() {
        normalized.retweeted_status = Some(Box::new(normalize_post(api, *retweeted).await?));
    }

    Ok(normalized)
}
```

- [ ] **Step 5: Route export preparation through the normalization helper before Markdown/PDF generation**

```rust
let normalized_posts = stream::iter(raw_posts)
    .then(|post| normalize_post(&api, post))
    .try_collect::<Vec<_>>()
    .await?;
```

- [ ] **Step 6: Re-run the targeted long-text tests**

Run: `cargo test --test downloader_long_text`

Expected: PASS

- [ ] **Step 7: Run diagnostics on changed files**

Run LSP diagnostics for:
- `src-tauri/src/services/downloader.rs`
- `src-tauri/src/services/weibo_api.rs`
- `src-tauri/src/models/weibo.rs`

Expected: no new errors.

### Task 4: Hybrid Markdown formatter and per-post index

**Files:**
- Modify: `src-tauri/src/services/export_markdown.rs`
- Modify: `src-tauri/src/services/export_pdf.rs` (only if shared text/link helper is extracted)
- Modify: `src-tauri/src/services/file_naming.rs` (if naming improvement needs it)
- Test: `src-tauri/tests/export_markdown_hybrid.rs` (new if no existing fit)

- [ ] **Step 1: Write a failing Markdown test for preserving link URLs**

```rust
#[test]
fn markdown_export_preserves_link_targets() {
    let post = fixture_post_with_html_link();

    let rendered = format_post(&post);

    assert!(rendered.contains("[查看原文](https://weibo.com/example)"));
}
```

- [ ] **Step 2: Write a failing Markdown test for metadata placement after the body**

```rust
#[test]
fn markdown_export_places_metadata_below_body() {
    let post = fixture_basic_post();

    let rendered = format_post(&post);

    let body_index = rendered.find("微博正文").unwrap();
    let metadata_index = rendered.find("- 发布时间:").unwrap();

    assert!(body_index < metadata_index);
}
```

- [ ] **Step 3: Write a failing per-post export test for index generation**

```rust
#[test]
fn per_post_export_writes_index_file() {
    let dir = tempdir().unwrap();
    let service = MarkdownExportService::new();
    let posts = vec![fixture_basic_post()];

    service.export_per_post(&dir.path().to_path_buf(), &posts).unwrap();

    assert!(dir.path().join("posts").join("index.md").exists());
}
```

- [ ] **Step 4: Run the targeted Markdown regression tests and verify the red state**

Run: `cargo test --test export_markdown_hybrid`

Expected: FAIL because formatter/index behavior is not implemented yet.

- [ ] **Step 5: Implement the shared hybrid formatter with readable body-first layout**

```rust
fn format_post(post: &WeiboPost) -> String {
    format!(
        "## {}\n\n{}\n\n- 发布时间: {}\n- 来源: {}\n- 链接: {}\n",
        post.title_line(),
        render_markdown_body(post),
        post.created_at,
        post.source.as_deref().unwrap_or("未知来源"),
        post.url
    )
}
```

- [ ] **Step 6: Preserve links during HTML-to-Markdown cleanup**

```rust
fn convert_links(input: &str) -> String {
    LINK_RE.replace_all(input, "[$text]($href)").to_string()
}
```

- [ ] **Step 7: Add per-post `index.md` generation**

```rust
let index = posts
    .iter()
    .map(|post| format!("- [{}](./{})", post.title_line(), post_filename(post)))
    .collect::<Vec<_>>()
    .join("\n");

fs::write(posts_dir.join("index.md"), index)?;
```

- [ ] **Step 8: Use user-specific naming for single-file export when profile metadata exists**

```rust
let filename = format!("{}-微博导出.md", sanitize_filename(&profile.screen_name));
```

- [ ] **Step 9: Re-run the targeted Markdown tests**

Run: `cargo test --test export_markdown_hybrid`

Expected: PASS

- [ ] **Step 10: Run diagnostics on changed files**

Run LSP diagnostics for:
- `src-tauri/src/services/export_markdown.rs`
- `src-tauri/src/services/export_pdf.rs`
- `src-tauri/src/services/file_naming.rs`

Expected: no new errors.

### Task 5: Theme-token cleanup and icon consistency

**Files:**
- Modify: `src/App.css`
- Modify: `src/components/StepLogin.tsx`
- Modify: `src/components/StepTarget.tsx`
- Modify: `src/components/StepOptions.tsx`
- Modify: `src/components/StepPath.tsx`
- Modify: `src/components/StepDownload.tsx`

- [ ] **Step 1: Write a focused frontend regression checklist for theme consistency**

```text
Expected after change:
- main UI light/dark colors derive from shared CSS tokens
- step illustrations/icons no longer embed hardcoded theme-breaking fills/strokes
- no behavior change beyond color consistency
```

- [ ] **Step 2: Add shared icon color variables to `App.css`**

```css
:root {
  --icon-accent: #4f6ef7;
  --icon-accent-soft: #e8f0fe;
  --icon-success: #30a46c;
  --icon-success-soft: #e9f9ee;
  --icon-danger: #d14343;
  --icon-danger-soft: #ffebeb;
}
```

- [ ] **Step 3: Update step components to consume CSS-driven icon colors instead of hardcoded literals**

```tsx
<svg className="step-illustration step-illustration--success" viewBox="0 0 80 80">
  <path className="step-illustration__soft" d="..." />
  <path className="step-illustration__accent" d="..." />
</svg>
```

- [ ] **Step 4: Explicitly avoid platform-specific WebView theme forcing unless a verified safe hook is found during implementation**

```text
Decision checkpoint:
- if there is no reliable Tauri/WebView theme API in current project context, do not add a workaround
- document the limitation instead
```

- [ ] **Step 5: Run TypeScript diagnostics on the touched components**

Run LSP diagnostics for:
- `src/App.css`
- `src/components/StepLogin.tsx`
- `src/components/StepTarget.tsx`
- `src/components/StepOptions.tsx`
- `src/components/StepPath.tsx`
- `src/components/StepDownload.tsx`

Expected: no new errors.

### Task 6: Remove dead bridge-era state and update docs

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `README.md`

- [ ] **Step 1: Write a failing compile-oriented expectation for dead-code cleanup**

```text
Current problem:
- state.rs still contains request-result storage for removed WebView bridge
- README still describes manual cookie copy and raw user ID flow
```

- [ ] **Step 2: Remove dead request-result state/helpers that are no longer used**

```rust
#[derive(Default)]
pub struct AppState {
    pub cancel_flag: AtomicBool,
    pub current_page: Mutex<u32>,
    pub total_pages: Mutex<u32>,
    pub posts_fetched: Mutex<u32>,
    pub total_posts: Mutex<u32>,
    cookie: Mutex<String>,
}
```

- [ ] **Step 3: Update README to match the current guided login + homepage URL flow**

```md
1. 打开应用
2. 在应用内登录微博
3. 输入目标博主主页地址，例如 `https://www.weibo.com/u/2166767661`
4. 选择下载选项和保存目录
5. 开始下载并导出
```

- [ ] **Step 4: Update README smoke checklist for current behavior**

```md
- [ ] 已登录状态下重新打开应用可直接进入下一步
- [ ] 长微博可以导出完整正文
- [ ] “打开下载目录”按钮可用
```

- [ ] **Step 5: Run compile/docs verification for cleanup**

Run: `cargo check && npx tsc --noEmit`

Expected: PASS

- [ ] **Step 6: Run diagnostics on changed files**

Run LSP diagnostics for:
- `src-tauri/src/state.rs`
- `src-tauri/src/lib.rs`
- `README.md`

Expected: no new errors in code files.

### Task 7: Full verification, manual QA, and package rebuild

**Files:**
- Modify only if verification reveals issues
- Test: whole project

- [ ] **Step 1: Run the full Rust test suite**

Run: `cargo test`

Expected: PASS, or identify pre-existing failures explicitly.

- [ ] **Step 2: Run frontend typecheck**

Run: `npx tsc --noEmit`

Expected: PASS

- [ ] **Step 3: Run backend compile verification**

Run: `cargo check`

Expected: PASS

- [ ] **Step 4: Perform manual QA against the approved checklist**

```text
Manual QA checklist:
1. Saved valid login survives app relaunch and shows next-step path
2. Logout clears persisted session and returns to login state
3. Invalid/stale login falls back cleanly
4. Long-post export includes full text
5. Open-folder button opens the real output directory
6. Markdown single-file output is readable and named appropriately
7. Markdown per-post output includes index.md
8. Main UI light/dark styling remains consistent
9. If WebView theme sync is unsupported, confirm limitation is documented rather than hacked around
```

- [ ] **Step 5: Build the installer package after verification passes**

Run: `npm run tauri build`

Expected: PASS and bundle artifacts created under `src-tauri/target/release/bundle/`

- [ ] **Step 6: Summarize evidence and any remaining known limitations**

```text
Report must include:
- exact commands run
- pass/fail results
- package artifact paths
- whether WebView theme sync was implemented or intentionally left unchanged
- any pre-existing failures not caused by this work
```
