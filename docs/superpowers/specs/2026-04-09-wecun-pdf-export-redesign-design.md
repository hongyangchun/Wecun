# Wecun PDF Export Redesign

**Goal:** Redesign and implement Wecun's PDF export so the result feels like a readable personal archive instead of a raw dump, while staying within the existing Tauri + Rust + Typst architecture and only changing `src-tauri/src/services/export_pdf.rs` plus style/layout updates in `src/components/ProfilePanel.tsx`.

## Assumptions

- The visual direction defaults to a **mixed style**: a book-like cover with restrained, archive-style interior pages.
- The existing export API shape remains unchanged.
- No new Rust or frontend dependencies will be added.
- Typst 0.14 remains the rendering engine and existing compile flow stays intact.

## Scope

This design covers:

1. A dedicated cover page for PDF export.
2. A manual month-grouped table of contents without numeric prefixes.
3. Real image embedding only when local files actually exist.
4. More readable page layout, typography, and metadata presentation.
5. Cleanup of city/region text so `发布于` prefixes and HTML entities do not leak into the PDF.
6. Lightweight visual alignment in `ProfilePanel.tsx` so the profile analysis panel better matches the upgraded export presentation.

Out of scope:

- Adding new export commands or changing frontend export flow.
- Changing data contracts between Rust and TypeScript.
- Broad refactors outside `export_pdf.rs` and `ProfilePanel.tsx`.
- Complex responsive re-architecture in the frontend.
- Advanced Typst graphics features that would significantly increase implementation risk.

## Current Problems

### 1. The PDF opens without ceremony

The current export starts with a plain title block and immediately moves into generated content. It does not feel like a personal document or archive.

### 2. The built-in outline structure is the wrong model

The current output relies on Typst heading/outline behavior, which produces numbered, flat directory items like `1. 2024-01-01`. That does not match the desired month-grouped Chinese archive structure.

### 3. Image handling is only partially safe

The renderer checks whether `post.images` exists, but the logical behavior still treats the image list as content even when referenced local files are missing. This causes misleading image sections or broken-looking output.

### 4. Reading comfort is weak

The current page treatment is functionally correct but visually plain. Typography hierarchy is weak, metadata is too raw, and the content does not read like a polished export.

### 5. Region text leaks unwanted prefixes/entities

Region strings can still contain `发布于`-style prefixes or decoded/encoded fragments that should not appear in the final archive.

## Design Summary

The redesigned PDF uses a **four-part document structure**:

1. **Cover page** — emotional entry point, book-like and centered.
2. **Manual table of contents** — grouped by month with Chinese month titles and date items.
3. **Body pages** — archive-style post blocks with cleaner hierarchy and real image embedding.
4. **Closing page** — minimal, consistent end page.

This separates emotion, navigation, and reading into distinct layers and fits the current exporter architecture without changing external behavior.

## Detailed Design

### A. Cover page

The cover is rendered as a dedicated first page before the table of contents.

#### Content

- Blogger avatar only if a usable local image path is already available from existing cached/export data; otherwise a styled fallback circle with initials or a neutral placeholder block.
- Nickname / display name.
- A subtitle such as `微博存档` or `个人微传`.
- Bio/description only if already available through the current export context or cached author data; if unavailable, omit the field rather than inventing placeholder prose or expanding scope.
- Summary stats:
  - total post count
  - export date range
  - export source label (for example, 收藏微博 vs user export)
- Export signature: `由微存 Wecun 导出` and the export date.

#### Visual treatment

- Soft gradient or gently tinted background using Typst primitives already available in the document.
- Large central title block.
- Thin divider line separating identity from archive metadata.
- Round avatar presentation if possible; otherwise a restrained square/rounded block fallback.
- Generous whitespace so it reads like the front matter of a personal collection.

#### Why this approach

This creates the sense of a curated archive without requiring new rendering infrastructure or risky visual effects.

### B. Manual table of contents

The new directory structure will be rendered manually instead of using Typst's default `outline` numbering flow.

#### Target structure

```text
封面页
├── 博主信息
├── 目录
│   ├── 2024年1月
│   │   ├── 2024-01-01
│   │   ├── 2024-01-02
│   │   └── ...
│   └── 2024年2月
│       ├── 2024-02-01
│       └── ...
```

#### Rendering model

- Group posts by `YYYY-MM` extracted from `created_at`.
- Convert group titles into Chinese month labels like `2024年1月`.
- Render month headings as stronger text blocks.
- Render each post entry beneath the month using `YYYY-MM-DD` text only rather than literal `.md` filenames. This keeps the PDF directory elegant while still mapping one entry to one post/day.
- Do not prefix items with `1.` or any numeric numbering.
- Allow natural page flow if the contents span multiple pages.

#### Why this approach

It solves the numbering problem completely and better matches the user's mental model of an archive organized by time.

### C. Body page layout

Each post becomes a consistent archive block rather than a minimally formatted chunk.

#### Page setup

- Keep A4.
- Increase overall reading comfort by using a more intentional margin model and type scale. The redesign will not increase left/right margins to 4cm because that would further narrow the content area and conflict with the stated readability goal on mobile PDF viewers.
- Use stronger type hierarchy:
  - document/month title: ~18pt
  - post date title: ~14pt
  - body text: ~12.5–14pt
  - metadata/tags: ~10pt
- Maintain readable line spacing and avoid cramped vertical rhythm.

#### Post block structure

For each post:

1. Date heading (`YYYY-MM-DD` or date + time if desired and available).
2. Region / repost source line in a subdued style.
3. Main body text.
4. Image section, only when valid images exist.
5. Metadata row(s): source URL, tags.
6. Divider line.

#### Styling principles

- Prefer subtle separators over heavy card borders.
- Use soft background or fill only for small metadata tags, not full-width panels.
- Keep forwarded/repost text visually secondary but still readable.

#### Why this approach

This keeps the document elegant on desktop and mobile PDF viewers while preserving archive value.

### D. Image embedding behavior

Image handling should become strict and truthful.

#### Rules

For each image in a post:

1. If `local_path` is missing, skip the image.
2. If `local_path` exists but the file does not exist on disk, skip the image.
3. If the file exists, embed it with Typst `image()`.
4. If no images remain after validation, omit the image section entirely.

#### Layout behavior

- Single image: wide display at comfortable width.
- Multiple images: render in a simple vertical sequence for stability.
- Do not create fake placeholder figures that look like real content.

#### Why this approach

The export should only claim to include images when it truly has them.

### E. Text normalization and region cleanup

The exporter already strips HTML, but region and body cleanup should become more explicit.

#### Required cleanup

- Decode common HTML entities during plain-text conversion.
- Trim whitespace and collapse redundant empty lines.
- Remove leading `发布于` and `发布于 ` from region strings.
- Normalize any entity-derived spacing so values such as `发布于&nbsp;上海` become `上海`.
- Omit the region line entirely if the cleaned value becomes empty.

#### Why this approach

This ensures PDF text looks authored and intentional rather than scraped.

### F. `export_pdf.rs` implementation structure

The file should be improved through targeted extraction, not broad rearchitecture.

#### Expected internal helpers

- `render_cover_page(...)`
- `render_toc(...)`
- `render_post_block(...)`
- `group_posts_by_month(...)`
- `clean_region_text(...)`
- `resolve_existing_image_paths(...)`

These helpers can remain private functions in `export_pdf.rs` so the public API stays unchanged.

#### Why this structure

The current file is still small enough to keep local, but these helpers make the new responsibilities easier to reason about and test.

### G. `ProfilePanel.tsx` design alignment

The frontend change is intentionally narrow: make the profile panel visually feel closer to the upgraded PDF export style.

#### Adjustments

- Strengthen top-level title and supporting subtitle spacing.
- Make key metric/chart containers feel more editorial and less utilitarian.
- Soften chart box presentation and align spacing with the new PDF cover/body rhythm.
- Preserve hooks-based structure and existing data flow.
- Do not add export state or new business logic.

#### Why this approach

It creates product-level coherence without expanding scope into a frontend redesign.

## Error Handling

- Missing images are silently excluded from embed output instead of producing misleading placeholders.
- Missing optional cover data (avatar/bio) falls back to omission or restrained placeholder styling.
- Invalid/empty cleaned region text is not rendered.
- Typst compile behavior remains unchanged: existing compile errors still surface through `AppError`.

## Testing and Verification

Implementation should be verified incrementally:

1. `cargo check` in `src-tauri/`
2. `tsc --noEmit` or the project's frontend type-check/build equivalent
3. LSP diagnostics clean for changed files
4. Manual PDF spot-check for:
   - cover page exists
   - TOC has Chinese month grouping and no numbering prefix
   - valid local images appear
   - missing images do not render fake placeholders
   - region text no longer includes `发布于`

## Files Expected to Change

- `src-tauri/src/services/export_pdf.rs`
- `src/components/ProfilePanel.tsx`

## Recommended Implementation Order

1. Build cover page rendering.
2. Replace default outline with manual month-grouped TOC.
3. Tighten image validation and embedding.
4. Improve body typography/layout.
5. Add region/text cleanup helpers.
6. Align `ProfilePanel.tsx` styling.

## Success Criteria

The redesign is complete when:

- PDF exports begin with a dedicated cover page.
- The contents are grouped by month using Chinese month labels and no numeric prefixes.
- Only real existing local images are embedded.
- Interior pages feel more readable and polished.
- `发布于` prefixes and leaked HTML entity artifacts are removed from region text.
- Frontend profile presentation feels visually closer to the upgraded export output without changing behavior.
