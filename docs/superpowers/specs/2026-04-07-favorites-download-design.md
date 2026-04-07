# Favorites Download Feature Design

**Goal:** Add support for downloading the logged-in user's Weibo favorites (收藏) alongside the existing blogger profile download mode.

**Current state:** The app downloads posts from a specified blogger's profile page via `/ajax/statuses/searchProfile`. Users must input a profile URL or user ID to start a download.

**Key insight:** Favorites and profile downloads share 90%+ of the pipeline (long-text expansion, normalization, filtering, image download, caching, export). The only difference is the data source API.

## Scope

**In scope:**
1. New "download source" toggle in the Target step (博主主页 / 我的收藏)
2. Backend favorites API integration (`/ajax/favorites/all`)
3. `source_type` field added to `DownloadRequest`
4. Favorites mode reuses all existing: long-text expansion, post normalization, image download, cache, export
5. README update

**Out of scope:**
- Favorites management (add/remove/unfavorite)
- Favorites filtering by tag or category
- Downloading other users' favorites
- Any changes to the export formats or pipeline

## Constraints

- Reuse existing download pipeline; do not duplicate normalization/export logic
- Follow existing code conventions (Rust backend, React frontend, Tauri 2)
- All user-facing text in Simplified Chinese
- Preserve existing rate limiting (1s/request)
- Cache compatibility: favorites data writes to the same `.weibo-cache/posts.json`

## Design

### A. Frontend Changes

#### A.1: StepTarget Component

Add a source type toggle at the top of the target step:

```
下载来源：
  ◉ 博主主页    ○ 我的收藏
```

- **博主主页 mode** (default): identical to current behavior — show URL input field, validate profile URL, extract uid
- **我的收藏 mode**: hide URL input, show informational text "将下载当前登录账号的收藏微博"，no uid required, "下一步" button always enabled

#### A.2: Wizard State

Add `sourceType: "profile" | "favorites"` to the wizard state (in `state/wizard-context.tsx`).

Default value: `"profile"` (existing behavior unchanged).

#### A.3: Download Request

`DownloadRequest` in `src/types/contracts.ts` gains a new field:

```typescript
export interface DownloadRequest {
  uid: string;
  source_type: "profile" | "favorites";  // NEW
  cookie: string;
  filter: PostFilter;
  include_images: boolean;
  date_range: { start_timestamp: number | null; end_timestamp: number | null };
  output_dir: string;
  min_text_length: number;
}
```

In favorites mode, `uid` is sent as an empty string.

#### A.4: Validation Logic

- Profile mode: validate URL format, extract uid (existing behavior)
- Favorites mode: skip URL validation, only require login state
- `canGoNext()` in `App.tsx` updated: favorites mode on step 1 always returns `true` if logged in

### B. Backend Changes

#### B.1: Request Model

`src-tauri/src/models/request.rs` — `DownloadRequest` struct gains:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Profile,
    Favorites,
}

pub struct DownloadRequest {
    pub uid: String,
    pub source_type: SourceType,  // NEW
    // ... existing fields unchanged
}
```

#### B.2: Weibo API Client

`src-tauri/src/services/weibo_api.rs` — new method:

```rust
pub async fn fetch_favorites_page(
    &self,
    page: i64,
    starttime: Option<i64>,
    endtime: Option<i64>,
) -> Result<(Vec<RawPost>, i64), AppError>
```

Calls `/ajax/favorites/all` with pagination params. Response structure is expected to be compatible with `RawSearchProfile` (contains `list` of posts and `total` count). If the response shape differs, a new `RawFavoritesResponse` model will be created in `models/weibo.rs`.

#### B.3: Download Service

`src-tauri/src/services/downloader.rs` — `run_inner()` modified:

```
if request.source_type == SourceType::Profile {
    // existing: client.fetch_posts_page(&request.uid, page, ...)
} else {
    // new: client.fetch_favorites_page(page, ...)
}
```

All subsequent steps (long-text expansion, normalization, filtering, image download, cache save) remain unchanged.

#### B.4: Cache Checkpoint

`src-tauri/src/models/cache.rs` — `CheckpointMeta` gains optional `source_type` field for resume support:

```rust
pub struct CheckpointMeta {
    pub uid: String,
    pub source_type: Option<SourceType>,  // NEW, optional for backward compat
    pub last_page: usize,
    pub total_fetched: usize,
    pub total_posts: i64,
}
```

### C. Error Handling

| Scenario | Behavior |
|----------|----------|
| Favorites mode, no login | Reuse existing: "请先登录微博" |
| Favorites API returns empty list | "收藏夹为空，没有可下载的微博" |
| Favorites API returns 401/403 | Reuse existing auth invalid logic: clear cookie, prompt re-login |
| Favorites API response shape unexpected | Parse error logged, user sees "收藏接口响应异常，请反馈" |

### D. Data Flow

```
User selects "我的收藏" in StepTarget
    → wizard state.sourceType = "favorites"
    → DownloadRequest { source_type: Favorites, uid: "", ... }
    → Tauri invoke("start_download", request)
    → DownloadService::run_inner()
        → WeiboApiClient::fetch_favorites_page(page, ...)
            → GET /ajax/favorites/all?page=N
        → [reuse] long-text expansion
        → [reuse] post normalization
        → [reuse] filter (original/all)
        → [reuse] image download (if enabled)
        → [reuse] cache save
        → [reuse] export (md-single / md-multi / html)
```

### E. Files Expected to Change

**Frontend:**
- `src/types/contracts.ts` — add `source_type` to `DownloadRequest`
- `src/state/wizard-context.ts` — add `sourceType` to state
- `src/components/StepTarget.tsx` — add source type toggle UI
- `src/App.tsx` — update `canGoNext()` and `buildDownloadRequest`
- `src/lib/validation.ts` — update `buildDownloadRequest` to include `source_type`

**Backend:**
- `src-tauri/src/models/request.rs` — add `SourceType` enum and field
- `src-tauri/src/models/cache.rs` — add `source_type` to `CheckpointMeta`
- `src-tauri/src/models/mod.rs` — export `SourceType`
- `src-tauri/src/services/weibo_api.rs` — add `fetch_favorites_page()`
- `src-tauri/src/services/downloader.rs` — branch on `source_type` in `run_inner()`

**Docs:**
- `README.md` — add favorites feature to feature list and usage instructions

## Testing and Verification

### Automated
- Add fixture for favorites API response in `src-tauri/tests/fixtures/favorites.json`
- Add test for `fetch_favorites_page` parsing
- Add test for favorites mode download request serialization

### Manual QA
- [ ] Login → select "我的收藏" → next step works without URL input
- [ ] Favorites download fetches correct posts
- [ ] Long-text posts in favorites expand correctly
- [ ] Image download works for favorites
- [ ] Export (md/html) works for favorites data
- [ ] Cancel during favorites download works
- [ ] Resume from checkpoint works for favorites

## Risks and Decisions

### Risk: Favorites API response structure differs from profile API

**Mitigation:** Create a separate `RawFavoritesResponse` model if needed. The post data itself (`RawPost`) should be reusable since Weibo returns the same mblog structure for both endpoints.

### Risk: Favorites API endpoint path unknown

**Mitigation:** The most likely endpoint is `/ajax/favorites/all` based on Weibo's web UI patterns. Will verify by inspecting network traffic from weibo.com favorites page. If this endpoint doesn't work, alternative paths include `/ajax/favorites/timeline` or `/favourites/`.

### Risk: Favorites have no total count in API response

**Mitigation:** If the API doesn't return a `total` field, use empty-page detection (stop when `list` is empty) as the termination condition. This is already handled in the existing download loop.

## Success Criteria

1. User can select "我的收藏" as download source in the Target step
2. Favorites download fetches all favorited posts with correct content
3. Long-text favorites posts expand correctly
4. Images in favorites download correctly when enabled
5. Export works identically for favorites and profile data
6. Existing profile download behavior is completely unchanged
7. All tests pass (`cargo test`, `npx tsc --noEmit`)
