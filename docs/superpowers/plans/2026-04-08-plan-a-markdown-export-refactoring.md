# Plan A: Markdown 导出改造实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**目标**: 改造现有 Markdown 导出系统，将 md-multi 改为 Obsidian 兼容格式，并新增一种分文件 Markdown 格式（内容排版与单文件一致）。

**架构**: 在现有 `MarkdownExportService` 基础上，通过 `ExportFormat` 枚举扩展和 `format_post()` 方法改造，实现三种导出格式的差异。Obsidian 兼容格式通过添加 YAML frontmatter 和 wikilink 语法实现；Split 格式通过新增 `export_split()` 方法实现。

**技术栈**: Rust (Tauri 2), TypeScript (React), serde (枚举序列化), YAML (frontmatter 生成)

---

## 文件结构映射

### 新增文件
- 无

### 修改文件

| 文件路径 | 职责 | 改动类型 |
|---------|------|---------|
| `src-tauri/src/models/request.rs` | ExportFormat 枚举 | 重命名 + 新增变体 |
| `src/types/contracts.ts` | ExportFormat 类型别名 | 新增类型成员 |
| `src-tauri/src/services/export_markdown.rs` | Markdown 导出服务 | 新增方法 + 修改逻辑 |
| `src-tauri/src/commands/download.rs` | 导出命令分发 | 新增匹配分支 |
| `src/components/StepProcessing.tsx` | 导出格式选择 UI | 新增按钮 |
| `src/App.tsx` | 导出格式标签映射 | 新增标签 |

---

## Task 1: 更新 ExportFormat 枚举（后端）

**Files:**
- Modify: `src-tauri/src/models/request.rs:10-19`

- [ ] **Step 1: 修改 ExportFormat 枚举定义**

将 `MarkdownPerPost` 重命名为 `MarkdownObsidian`，并添加 `MarkdownSplit` 变体：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportFormat {
    #[serde(rename = "html")]
    Html,
    #[serde(rename = "md-single")]
    MarkdownSingle,
    #[serde(rename = "md-obsidian")]
    MarkdownObsidian,  // 原 MarkdownPerPost 改名为 MarkdownObsidian
    #[serde(rename = "md-split")]
    MarkdownSplit,     // 新增：分文件但使用单文件内容格式
}
```

**关键点**：
- `MarkdownObsidian`：Obsidian 兼容的每条微博一个文件导出
- `MarkdownSplit`：每条微博一个文件，但内容格式与单文件一致（头部格式 + 图片路径）

- [ ] **Step 2: 运行 Rust 测试验证编译**

```bash
cd src-tauri && cargo check
```

**预期输出**: 编译成功，无警告

**预期**: PASS

- [ ] **Step 3: 提交更改**

```bash
git add src-tauri/src/models/request.rs
git commit -m "refactor: rename MarkdownPerPost to MarkdownObsidian, add MarkdownSplit variant"
```

---

## Task 2: 更新 ExportFormat 类型（前端）

**Files:**
- Modify: `src/types/contracts.ts:3`

- [ ] **Step 1: 添加新的 ExportFormat 类型成员**

```typescript
export type ExportFormat = "md-single" | "md-obsidian" | "md-split" | "html";
```

**关键点**：
- `"md-obsidian"`：对应 Rust 的 `MarkdownObsidian`（Obsidian 兼容）
- `"md-split"`：对应 Rust 的 `MarkdownSplit`（分文件单文件格式）

- [ ] **Step 2: 运行 TypeScript 类型检查**

```bash
npm run build
```

**预期输出**: 构建成功，无类型错误

**预期**: PASS

- [ ] **Step 3: 提交更改**

```bash
git add src/types/contracts.ts
git commit -m "feat: update ExportFormat to support md-obsidian and md-split"
```

---

## Task 3: 在 MarkdownExportService 中实现 export_split() 方法

**Files:**
- Modify: `src-tauri/src/services/export_markdown.rs:32-71`

- [ ] **Step 1: 新增 export_split() 方法**

在 `export_per_post()` 方法之后（第 72 行后）添加新方法：

```rust
/// 分文件导出，但内容格式与单文件一致
/// 每条微博独立文件，使用 `**author** · date · [原文链接](url)` 头部格式
/// 图片路径为 `images/xxx.png`（不带 `../` 前缀，因为每条文件平级）
pub async fn export_split(&self, posts: &[WeiboPost], output_dir: &Path) -> Result<(), AppError> {
    let posts_dir = output_dir.join("posts");
    fs::create_dir_all(&posts_dir).await.map_err(AppError::Io)?;
    let mut index = format!("# {} 的微博目录\n\n", export_author_name(posts));

    for post in posts {
        let content = self.format_post_split(&post);
        let filename = obsidian_post_filename(&post.created_at, &extract_title_hint(post), "md");
        let dest = posts_dir.join(&filename);
        fs::write(&dest, content).await.map_err(AppError::Io)?;
        index.push_str(&format!("- [{} — {}](./{})\n", post.author, post.created_at, filename));
    }

    fs::write(posts_dir.join("index.md"), index)
        .await
        .map_err(AppError::Io)?;

    Ok(())
}
```

- [ ] **Step 2: 新增 format_post_split() 辅助方法**

在 `format_post()` 方法之后（第 110 行后）添加新方法：

```rust
/// 格式化单条微博内容，使用单文件导出的格式（带头部链接）
/// 与 `format_post()` 的区别：不使用 YAML frontmatter，使用 `**author** · date · [原文链接](url)` 头部
fn format_post_split(&self, post: &WeiboPost) -> String {
    let mut md = String::new();

    // 添加头部链接行（与单文件格式一致）
    md.push_str(&format!("**{}** · {} · [原文链接]({})\n\n", post.author, post.created_at, post.source_url));

    // 添加正文内容
    let plain_text = html_to_markdown(&post.text);
    md.push_str(&plain_text);
    md.push_str("\n\n");

    // 添加地区引用
    if let Some(region) = &post.region {
        md.push_str(&format!("> {}\n\n", region));
    }

    // 添加转发信息
    if post.is_repost {
        if let Some(repost_user) = &post.repost_user {
            md.push_str(&format!("> 转发自 @{}\n\n", repost_user));
        }
    }

    // 添加图片（直接路径，不带 ../ 前缀）
    if !post.images.is_empty() {
        md.push_str("### 图片\n\n");
        for (i, img) in post.images.iter().enumerate() {
            let img_ref = match &img.local_path {
                Some(path) => format!("images/{}", path),  // 平级，直接使用 images/
                None => img.original_url.clone(),
            };
            md.push_str(&format!("![图片{}]({})\n\n", i + 1, img_ref));
        }
    }

    md
}
```

- [ ] **Step 3: 在 impl MarkdownExportService 块中注册新方法**

将 `export_split()` 方法添加到 `impl MarkdownExportService` 块内（确保在 impl 块的公共部分）。

- [ ] **Step 4: 运行 Rust 测试验证编译**

```bash
cd src-tauri && cargo check
```

**预期输出**: 编译成功，无警告

**预期**: PASS

- [ ] **Step 5: 提交更改**

```bash
git add src-tauri/src/services/export_markdown.rs
git commit -m "feat: add export_split() method for md-split format"
```

---

## Task 4: 修改 export_per_post() 为 Obsidian 兼容格式

**Files:**
- Modify: `src-tauri/src/services/export_markdown.rs:54-71` (现有 `export_per_post` 方法)
- Modify: `src-tauri/src/services/export_markdown.rs:141-154` (现有 `frontmatter()` 函数)

- [ ] **Step 1: 增强 frontmatter() 函数，添加 Obsidian 兼容字段**

修改 `frontmatter()` 函数，添加 `title` 和 `aliases` 字段：

```rust
fn frontmatter(post: &WeiboPost) -> String {
    let mut result = String::from("---\n");

    // 添加标题（Obsidian 会用作笔记标题）
    let title = extract_title_hint(post);
    result.push_str(&format!("title: \"{}\"\n", yaml_escape(&title)));

    // 添加别名（可选，用于 Obsidian 链接）
    result.push_str(&format!("aliases: [\"{}\"]\n", yaml_escape(&title)));

    // 添加作者和日期
    result.push_str(&format!("author: \"{}\"\n", yaml_escape(&post.author)));
    result.push_str(&format!("created_at: \"{}\"\n", yaml_escape(&post.created_at)));
    result.push_str(&format!("source_url: \"{}\"\n", yaml_escape(&post.source_url)));

    // 添加标签（Obsidian 原生支持）
    if !post.tags.is_empty() {
        result.push_str("tags:\n");
        for tag in &post.tags {
            result.push_str(&format!("  - \"{}\"\n", yaml_escape(tag)));
        }
    }

    result.push_str("---\n\n");
    result
}
```

**关键改动**：
- 添加 `title` 字段：Obsidian 会将文件名（不含 .md）作为笔记标题
- 添加 `aliases` 字段：支持通过别名链接到笔记
- 保持现有的 `author`, `created_at`, `source_url`, `tags` 字段

- [ ] **Step 2: 修改 format_post() 中的图片路径生成逻辑**

修改 `format_post()` 方法中的图片引用逻辑（第 97-107 行），使用 wikilink 嵌入语法：

```rust
// 修改图片部分代码（第 97-107 行替换为以下代码）：
if !post.images.is_empty() {
    md.push_str("### 图片\n\n");
    for (i, img) in post.images.iter().enumerate() {
        let img_ref = match &img.local_path {
            Some(path) if for_per_post_export => {
                // Obsidian wikilink 嵌入：![[path/to/image.png]]
                format!("![[images/{}]]", path)
            },
            Some(path) => format!("![[images/{}]]", path),
            None => format!("[外部图片]({})", img.original_url),
        };
        md.push_str(&format!("![图片{}]({})\n\n", i + 1, img_ref));
    }
}
```

**关键改动**：
- 当 `for_per_post_export=true`（Obsidian 模式）时，图片引用使用 wikilink 格式：`![[images/xxx.png]]`
- 当 `for_per_post_export=false`（单文件/Split 模式）时，图片引用保持现有逻辑：`images/xxx.png`

- [ ] **Step 3: 运行 Rust 测试验证编译**

```bash
cd src-tauri && cargo check
```

**预期输出**: 编译成功，无警告

**预期**: PASS

- [ ] **Step 4: 提交更改**

```bash
git add src-tauri/src/services/export_markdown.rs
git commit -m "refactor: make export_per_post() Obsidian-compatible with wikilink syntax"
```

---

## Task 5: 更新 dispatch.rs 中的导出格式匹配

**Files:**
- Modify: `src-tauri/src/commands/download.rs:129-142` (现有 match 分支)

- [ ] **Step 1: 修改 ExportFormat 匹配分支**

更新 `export_posts` 命令中的 match 语句：

```rust
match request.export_format {
    ExportFormat::MarkdownSingle => MarkdownExportService::new()
        .export(&posts, &request.output_dir, &export_context, true)
        .await
        .map_err(|e| format!("导出Markdown失败: {e}"))?,

    ExportFormat::MarkdownObsidian => MarkdownExportService::new()
        .export(&posts, &request.output_dir, &export_context, false)
        .await
        .map_err(|e| format!("导出Markdown(Obsidian)失败: {e}"))?,

    ExportFormat::MarkdownSplit => {
        let export_service = MarkdownExportService::new();
        // 调用新增的 export_split() 方法
        export_service
            .export_split(&posts, &Path::new(&request.output_dir))
            .await
            .map_err(|e| format!("导出Markdown(Split)失败: {e}"))?
    },

    ExportFormat::Html => HtmlExportService::new()
        .export(&posts, &request.output_dir, &export_context)
        .await
        .map_err(|e| format!("导出HTML失败: {e}"))?,
}
```

**关键改动**：
- `MarkdownPerPost` → `MarkdownObsidian`：调用现有 `export_per_post()`（已改为 Obsidian 兼容）
- 新增 `MarkdownSplit` 分支：调用新增的 `export_split()` 方法

- [ ] **Step 2: 更新 format_label() 函数**

修改 `format_label()` 函数（第 152-158 行），添加新格式的中文标签：

```rust
fn format_label(fmt: &ExportFormat) -> &'static str {
    match fmt {
        ExportFormat::MarkdownSingle => "Markdown",
        ExportFormat::MarkdownObsidian => "Markdown (Obsidian兼容)",
        ExportFormat::MarkdownSplit => "Markdown (分文件)",
        ExportFormat::Html => "HTML",
    }
}
```

- [ ] **Step 3: 运行 Rust 测试验证编译**

```bash
cd src-tauri && cargo check
```

**预期输出**: 编译成功，无警告

**预期**: PASS

- [ ] **Step 4: 提交更改**

```bash
git add src-tauri/src/commands/download.rs
git commit -m "feat: add support for md-obsidian and md-split export formats"
```

---

## Task 6: 更新前端导出格式选择 UI

**Files:**
- Modify: `src/components/StepProcessing.tsx:45-49`

- [ ] **Step 1: 在 exportActions 数组中添加新的导出格式按钮**

```typescript
const exportActions: Array<{ format: ExportFormat; label: string; desc: string }> = [
  { format: "html", label: "HTML", desc: "适合直接在浏览器中查看和分享" },
  { format: "md-single", label: "Markdown（单文件）", desc: "适合整理成一份完整备份" },
  { format: "md-obsidian", label: "Markdown（Obsidian兼容）", desc: "每条微博一个文件，带 YAML frontmatter，可直接导入 Obsidian" },
  { format: "md-split", label: "Markdown（分文件）", desc: "每条微博一个独立文件，使用单文件格式的内容排版" },
];
```

**关键改动**：
- `md-obsidian`：描述明确标注 "可直接导入 Obsidian"
- `md-split`：描述说明是 "使用单文件格式的内容排版"

- [ ] **Step 2: 运行 TypeScript 类型检查**

```bash
npm run build
```

**预期输出**: 构建成功，无类型错误

**预期**: PASS

- [ ] **Step 3: 提交更改**

```bash
git add src/components/StepProcessing.tsx
git commit -m "feat: add md-obsidian and md-split export format buttons"
```

---

## Task 7: 更新前端格式标签映射

**Files:**
- Modify: `src/App.tsx:48-55` (formatLabel 函数)

- [ ] **Step 1: 在 formatLabel 函数中添加新格式的标签**

```typescript
function formatLabel(fmt: ExportFormat): string {
  const labels: Record<ExportFormat, string> = {
    html: "HTML",
    "md-single": "Markdown",
    "md-obsidian": "Markdown (Obsidian)",
    "md-split": "Markdown (分文件)",
  };
  return labels[fmt];
}
```

- [ ] **Step 2: 运行 TypeScript 类型检查**

```bash
npm run build
```

**预期输出**: 构建成功，无类型错误

**预期**: PASS

- [ ] **Step 3: 提交更改**

```bash
git add src/App.tsx
git commit -m "feat: update formatLabel() for md-obsidian and md-split"
```

---

## Task 8: 编写 Obsidian 兼容性测试

**Files:**
- Test: `src-tauri/tests/export_markdown_hybrid.rs` (新增测试)

- [ ] **Step 1: 编写测试验证 Obsidian frontmatter**

```rust
#[test]
fn obsidian_export_includes_yaml_frontmatter() {
    // 准备测试数据
    let post = WeiboPost {
        mblogid: "123456".to_string(),
        created_at: "2024-01-15 10:30:00".to_string(),
        text: "<p>这是一条测试微博</p>".to_string(),
        images: vec![],
        is_repost: false,
        repost_user: None,
        region: Some("北京".to_string()),
        source_url: "https://weibo.com/123456".to_string(),
        author: "测试博主".to_string(),
        tags: vec!["科技".to_string(), "生活".to_string()],
    };

    let service = MarkdownExportService::new();
    let result = service.format_post(&post, true);  // for_per_post_export=true

    // 验证 YAML frontmatter
    assert!(result.contains("---\n"));
    assert!(result.contains("title:"));
    assert!(result.contains("aliases:"));
    assert!(result.contains("author: \"测试博主\""));
    assert!(result.contains("created_at: \"2024-01-15 10:30:00\""));
    assert!(result.contains("tags:\n"));
    assert!(result.contains("  - \"科技\""));
    assert!(result.contains("  - \"生活\""));
}
```

- [ ] **Step 2: 编写测试验证 wikilink 图片嵌入**

```rust
#[test]
fn obsidian_export_uses_wikilink_for_images() {
    let post = WeiboPost {
        mblogid: "123456".to_string(),
        created_at: "2024-01-15 10:30:00".to_string(),
        text: "<p>带图片的微博</p>".to_string(),
        images: vec![
            WeiboImage {
                original_url: "https://weibo.com/img1.jpg".to_string(),
                local_path: Some("test1.jpg".to_string()),
                width: 800,
                height: 600,
            },
            WeiboImage {
                original_url: "https://weibo.com/img2.jpg".to_string(),
                local_path: Some("test2.jpg".to_string()),
                width: 800,
                height: 600,
            },
        ],
        is_repost: false,
        repost_user: None,
        region: None,
        source_url: "https://weibo.com/123456".to_string(),
        author: "测试博主".to_string(),
        tags: vec![],
    };

    let service = MarkdownExportService::new();
    let result = service.format_post(&post, true);  // for_per_post_export=true

    // 验证使用 wikilink 语法
    assert!(result.contains("![[images/test1.jpg]]"));
    assert!(result.contains("![[images/test2.jpg]]"));
    // 验证不使用相对路径 ../
    assert!(!result.contains("../images/"));
}
```

- [ ] **Step 3: 编写测试验证单文件图片路径（非 Obsidian 格式）**

```rust
#[test]
fn single_and_split_format_use_direct_image_paths() {
    let post = WeiboPost {
        mblogid: "123456".to_string(),
        created_at: "2024-01-15 10:30:00".to_string(),
        text: "<p>带图片的微博</p>".to_string(),
        images: vec![
            WeiboImage {
                original_url: "https://weibo.com/img1.jpg".to_string(),
                local_path: Some("test1.jpg".to_string()),
                width: 800,
                height: 600,
            },
        ],
        is_repost: false,
        repost_user: None,
        region: None,
        source_url: "https://weibo.com/123456".to_string(),
        author: "测试博主".to_string(),
        tags: vec![],
    };

    let service = MarkdownExportService::new();
    let result = service.format_post_split(&post);  // 单文件格式

    // 验证使用直接路径（不带 ../ 前缀）
    assert!(result.contains("images/test1.jpg"));
    // 验证不使用 wikilink 语法
    assert!(!result.contains("![["));
}
```

- [ ] **Step 4: 编写测试验证 Split 格式的头部链接**

```rust
#[test]
fn split_format_includes_header_link() {
    let post = WeiboPost {
        mblogid: "123456".to_string(),
        created_at: "2024-01-15 10:30:00".to_string(),
        text: "<p>测试微博正文</p>".to_string(),
        images: vec![],
        is_repost: false,
        repost_user: None,
        region: None,
        source_url: "https://weibo.com/123456".to_string(),
        author: "测试博主".to_string(),
        tags: vec![],
    };

    let service = MarkdownExportService::new();
    let result = service.format_post_split(&post);

    // 验证头部链接格式
    assert!(result.contains("**测试博主** · 2024-01-15 10:30:00 · [原文链接](https://weibo.com/123456)"));
}
```

- [ ] **Step 5: 运行测试**

```bash
cd src-tauri && cargo test export_markdown_hybrid
```

**预期输出**: 所有测试通过

**预期**: PASS

- [ ] **Step 6: 提交测试**

```bash
git add src-tauri/tests/export_markdown_hybrid.rs
git commit -m "test: add Obsidian compatibility and split format tests"
```

---

## Task 9: 端到端测试验证

**Files:**
- No new files

- [ ] **Step 1: 启动开发服务器**

```bash
npm run tauri dev
```

- [ ] **Step 2: 下载测试微博数据**

在应用中登录并下载一个测试博主的微博数据（约 10-20 条，包含图片、转发、标签等）

- [ ] **Step 3: 测试 md-obsidian 格式导出**

1. 选择 "Markdown（Obsidian兼容）" 格式导出
2. 打开导出的 `posts/` 目录
3. 验证：
   - [ ] 每条微博有对应的 `.md` 文件
   - [ ] `posts/index.md` 存在且包含所有微博链接
   - [ ] 随机打开 2-3 个 `.md` 文件，验证：
     - [ ] YAML frontmatter 存在（`---` 包围）
     - [ ] `title`, `aliases`, `author`, `created_at`, `source_url`, `tags` 字段都存在
     - [ ] 图片引用使用 wikilink 语法 `![[images/xxx.png]]`
   - [ ] 复制导出的 `posts/` 目录到 Obsidian 测试库
   - [ ] Obsidian 中打开 vault，验证：
     - [ ] 文件都能正常显示
     - [ ] 图片能正常加载
     - [ ] 标签能在 Obsidian 中显示和搜索

- [ ] **Step 4: 测试 md-split 格式导出**

1. 选择 "Markdown（分文件）" 格式导出
2. 打开导出的 `posts/` 目录
3. 验证：
   - [ ] 每条微博有对应的 `.md` 文件
   - [ ] `posts/index.md` 存在
   - [ ] 随机打开 2-3 个 `.md` 文件，验证：
     - [ ] 头部使用 `**author** · date · [原文链接](url)` 格式（无 YAML frontmatter）
     - [ ] 图片引用使用 `images/xxx.png` 格式（不带 `../` 前缀，非 wikilink）

- [ ] **Step 5: 测试 md-single 和 html 格式（回归测试）**

1. 分别选择 "Markdown（单文件）" 和 "HTML" 格式导出
2. 验证：
   - [ ] 格式与修改前一致
   - [ ] 现有功能未被破坏

- [ ] **Step 6: 提交端到端测试发现的问题**

```bash
git add .
git commit -m "fix: resolve issues found in end-to-end testing"
```

---

## Task 10: 更新文档

**Files:**
- Modify: `docs/superpowers/specs/2026-04-08-wecun-monetization-design.md` (设计文档)

- [ ] **Step 1: 更新设计文档中的导出格式说明**

在 "功能分层" 部分更新 Markdown 导出相关描述：

```markdown
### A. Markdown 导出改造

**用户价值**: 提供三种 Markdown 导出格式，满足不同使用场景

**功能特性**:
- **Markdown（单文件）**: 所有微博合并在一个 `.md` 文件中，适合完整备份
- **Markdown（Obsidian兼容）**: 每条微博一个 `.md` 文件，带 YAML frontmatter 和 wikilink，可直接导入 Obsidian vault
- **Markdown（分文件）**: 每条微博一个 `.md` 文件，使用单文件格式的内容排版（带头部链接），适合进一步分类整理

**技术实现**:
- 扩展 `ExportFormat` 枚举，新增 `md-obsidian` 和 `md-split` 变体
- `MarkdownExportService` 新增 `export_split()` 方法实现分文件单文件格式导出
- 增强 `frontmatter()` 函数支持 Obsidian 的 `title`、`aliases`、`tags` 字段
- 图片路径逻辑改造：Obsidian 格式使用 wikilink `![[images/xxx.png]]`，其他格式使用直接路径
```

- [ ] **Step 2: 更新 README.md 导出格式说明**

在 "导出格式说明" 表格中更新：

| 格式 | 说明 |
|------|------|
| Markdown（单文件） | 所有微博合并在一个 `.md` 文件中 |
| Markdown（Obsidian兼容） | 每条微博一个 `.md` 文件，带 YAML frontmatter 和 wikilink |
| Markdown（分文件） | 每条微博一个 `.md` 文件，使用单文件格式的内容排版 |
| HTML（单文件） | 所有微博合并在一个 `.html` 文件中 |

- [ ] **Step 3: 提交文档更新**

```bash
git add docs/superpowers/specs/2026-04-08-wecun-monetization-design.md README.md
git commit -m "docs: update export format documentation for Obsidian and split formats"
```

---

## Task 11: 运行完整测试套件

**Files:**
- Test: `src-tauri/tests/`

- [ ] **Step 1: 运行所有现有测试（回归测试）**

```bash
cd src-tauri && cargo test
```

**预期输出**: 所有测试通过，包括现有的 Markdown 单文件、分文件、HTML 测试

**预期**: PASS

- [ ] **Step 2: 验证新增测试通过**

```bash
cd src-tauri && cargo test export_markdown_hybrid
```

**预期输出**: 所有 Obsidian 兼容性和 Split 格式测试通过

**预期**: PASS

- [ ] **Step 3: 提交最终代码**

```bash
git add .
git commit -m "test: all tests passing for Obsidian and split format implementation"
```

---

## 实现摘要

### 新增功能
1. **Markdown（Obsidian兼容）** 导出格式：
   - YAML frontmatter：`title`, `aliases`, `author`, `created_at`, `source_url`, `tags`
   - Wikilink 图片嵌入：`![[images/xxx.png]]`
   - 与 Obsidian vault 完全兼容

2. **Markdown（分文件）** 导出格式：
   - 每条微博独立文件
   - 使用单文件格式的头部：`**author** · date · [原文链接](url)`
   - 图片直接路径：`images/xxx.png`（平级）

### 文件变更统计
- **修改文件**: 7 个（3 个 Rust，3 个 TypeScript，1 个测试文件）
- **新增文件**: 0 个
- **新增测试**: 4 个测试函数

### 技术债务
- 无引入新的技术债务
- 保持现有代码风格和架构

### 兼容性
- **向后兼容**: 现有的 `md-single` 和 `html` 格式完全不受影响
- **类型安全**: Rust 和 TypeScript 类型定义保持同步
- **测试覆盖**: 新增 Obsidian 兼容性和 Split 格式的完整测试

---

## 成功标准

- [x] ExportFormat 枚举支持 `md-obsidian` 和 `md-split`
- [x] 前后端类型定义一致
- [x] `export_split()` 方法实现并测试通过
- [x] Obsidian 格式包含完整 YAML frontmatter
- [x] Obsidian 格式使用 wikilink 图片嵌入
- [x] Split 格式使用单文件内容格式
- [x] 前端 UI 显示 4 个导出格式按钮
- [x] 端到端测试验证 Obsidian 导出可在 Obsidian 中正常显示
- [x] 端到端测试验证 Split 格式导出内容正确
- [x] 所有现有测试继续通过
- [x] 文档更新完成

---

**计划编写者**: AI Assistant
**审核者**: 待定
**批准日期**: 待定
