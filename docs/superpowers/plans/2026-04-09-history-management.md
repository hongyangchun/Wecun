# 下载历史管理功能实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 添加侧边栏历史面板，用户可以查看已下载的微博记录并快速导出，无需重新下载。

**架构:** 在现有下载/导出流程基础上，增强历史记录存储（包含下载选项），添加后端命令支持从历史导出，前端新增侧边栏UI组件。

**Tech Stack:** Rust (Tauri backend), TypeScript/React (frontend), CSS animations

---

## 文件结构

### 后端（Rust）
- `src-tauri/src/services/history.rs` - 增强历史记录数据结构，保存完整下载选项
- `src-tauri/src/commands/download.rs` - 添加 `export_from_history` 和 `get_history_entry` 命令
- `src-tauri/src/models/mod.rs` - 可能需要添加新模型类型

### 前端（TypeScript/React/CSS）
- `src/types/contracts.ts` - 添加新的 TypeScript 类型定义
- `src/lib/tauri-bridge.ts` - 添加 Tauri 命令封装
- `src/components/HistoryPanel.tsx` - 新建：历史记录侧边栏主组件
- `src/components/HistoryListItem.tsx` - 新建：单条历史记录项组件
- `src/components/HistoryExportDialog.tsx` - 新建：导出格式选择对话框
- `src/App.tsx` - 修改：添加侧边栏状态和历史按钮
- `src/App.css` - 修改：添加侧边栏动画样式

---

## 任务分解

### Task 1: 增强后端历史记录数据结构

**Files:**
- Modify: `src-tauri/src/services/history.rs`

- [ ] **Step 1: 扩展 HistoryEntry 结构体添加下载选项字段**

在 `HistoryEntry` 结构体中添加新的可选字段，保持向后兼容：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub uid: String,
    pub screen_name: String,
    pub output_dir: String,
    pub last_download: String,
    pub post_count: usize,
    #[serde(default)]
    pub source_type: Option<SourceType>,
    // 新增字段 - 用于保存完整下载选项
    #[serde(default)]
    pub filter: Option<String>,
    #[serde(default)]
    pub include_images: Option<bool>,
    #[serde(default)]
    pub date_mode: Option<String>,
    #[serde(default)]
    pub date_start: Option<String>,
    #[serde(default)]
    pub date_end: Option<String>,
    #[serde(default)]
    pub ignore_deleted: Option<bool>,
    #[serde(default)]
    pub min_text_length: Option<usize>,
}
```

- [ ] **Step 2: 更新测试用例中的 sample_entry 函数**

更新 `src-tauri/src/services/history.rs` 中的测试辅助函数：

```rust
fn sample_entry(
    uid: &str,
    screen_name: &str,
    last_download: &str,
    post_count: usize,
) -> HistoryEntry {
    HistoryEntry {
        uid: uid.to_string(),
        screen_name: screen_name.to_string(),
        output_dir: format!("/tmp/{uid}"),
        last_download: last_download.to_string(),
        post_count,
        source_type: None,
        filter: Some("all".to_string()),
        include_images: Some(true),
        date_mode: Some("all".to_string()),
        date_start: None,
        date_end: None,
        ignore_deleted: Some(false),
        min_text_length: Some(0),
    }
}
```

- [ ] **Step 3: 运行测试验证向后兼容性**

运行: `cd src-tauri && cargo test history`

预期: 所有测试通过，新字段有默认值

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/services/history.rs
git commit -m "feat(history): add download options fields to HistoryEntry for future use"
```

---

### Task 2: 添加后端导出命令

**Files:**
- Modify: `src-tauri/src/commands/download.rs`

- [ ] **Step 1: 添加 get_history_entry 命令**

在 `src-tauri/src/commands/download.rs` 中添加新命令：

```rust
#[tauri::command]
pub fn get_history_entry(
    uid: String,
    app: tauri::AppHandle,
) -> Result<HistoryEntry, String> {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("wecun"));
    let entries = HistoryService::list(&dir);
    entries
        .into_iter()
        .find(|e| e.uid == uid)
        .ok_or_else(|| "未找到该历史记录".to_string())
}
```

- [ ] **Step 2: 添加 export_from_history 命令**

在 `src-tauri/src/commands/download.rs` 中添加导出命令：

```rust
#[tauri::command]
pub async fn export_from_history(
    uid: String,
    export_format: ExportFormat,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // 获取历史记录以找到 output_dir
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("wecun"));
    let history_entry = HistoryService::list(&dir)
        .into_iter()
        .find(|e| e.uid == uid)
        .ok_or_else(|| "未找到该历史记录".to_string())?;

    // 使用 export_posts 的逻辑
    let bundle = crate::services::cache::load_cache_bundle_sync(&history_entry.output_dir)
        .map_err(|e| {
            if matches!(e, AppError::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::NotFound) {
                "找不到缓存文件，该记录可能已失效".to_string()
            } else if matches!(e, AppError::Parse(_)) {
                "缓存文件已损坏，请重新下载".to_string()
            } else {
                format!("读取缓存失败: {e}")
            }
        })?;

    let export_context = bundle.export_context;
    let posts = bundle.posts;

    if posts.is_empty() {
        return Err("没有可导出的微博数据".to_string());
    }

    let _ = app.emit(
        "download-progress",
        ProgressEvent::new(
            ProgressPhase::Exporting,
            1,
            1,
            &format!("正在导出 {}...", format_label(&export_format)),
        ),
    );

    let posts_count = posts.len();

    match export_format {
        ExportFormat::MarkdownSingle => MarkdownExportService::new()
            .export(&posts, &history_entry.output_dir, &export_context, true)
            .await
            .map_err(|e| format!("导出Markdown失败: {e}"))?,
        ExportFormat::MarkdownObsidian => MarkdownExportService::new()
            .export(&posts, &history_entry.output_dir, &export_context, false)
            .await
            .map_err(|e| format!("导出Markdown(Obsidian)失败: {e}"))?,
        ExportFormat::Html => HtmlExportService::new()
            .export(&posts, &history_entry.output_dir, &export_context)
            .await
            .map_err(|e| format!("导出HTML失败: {e}"))?,
        ExportFormat::Pdf => PdfExportService::new()
            .export(&posts, &history_entry.output_dir, &export_context)
            .await
            .map_err(|e| format!("导出PDF失败: {e}"))?,
    }

    let _ = app.emit(
        "download-progress",
        ProgressEvent::new(ProgressPhase::Complete, posts_count, posts_count, "导出完成！"),
    );

    Ok(format!("导出完成！已生成 {}", format_label(&export_format)))
}
```

- [ ] **Step 3: 在 lib.rs 中注册新命令**

在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中添加：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 现有命令 ...
    get_history_entry,
    export_from_history,
])
```

- [ ] **Step 4: 编译检查**

运行: `cd src-tauri && cargo check`

预期: 无编译错误

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/commands/download.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add get_history_entry and export_from_history commands"
```

---

### Task 3: 添加前端 TypeScript 类型

**Files:**
- Modify: `src/types/contracts.ts`

- [ ] **Step 1: 扩展 HistoryEntry 接口**

在 `src/types/contracts.ts` 中找到现有的 HistoryEntry 接口（如果有），或在文件末尾添加：

```typescript
export interface HistoryEntry {
  uid: string;
  screen_name: string;
  output_dir: string;
  last_download: string;
  post_count: number;
  source_type?: "profile" | "favorites";
  // 新增字段
  filter?: "original" | "all";
  include_images?: boolean;
  date_mode?: "all" | "range";
  date_start?: string;
  date_end?: string;
  ignore_deleted?: boolean;
  min_text_length?: number;
}

export type RedownloadMode = "overwrite" | "incremental";
```

- [ ] **Step 2: 类型检查**

运行: `npm run tsc -- --noEmit`

预期: 无类型错误

- [ ] **Step 3: 提交**

```bash
git add src/types/contracts.ts
git commit -m "feat(types): extend HistoryEntry with download options"
```

---

### Task 4: 添加前端 Tauri Bridge 函数

**Files:**
- Modify: `src/lib/tauri-bridge.ts`

- [ ] **Step 1: 添加新函数**

在 `src/lib/tauri-bridge.ts` 中添加：

```typescript
export async function getHistoryEntry(uid: string): Promise<HistoryEntry> {
  return await invoke<HistoryEntry>("get_history_entry", { uid });
}

export async function exportFromHistory(uid: string, exportFormat: ExportFormat): Promise<string> {
  return await invoke<string>("export_from_history", { uid, exportFormat });
}
```

- [ ] **Step 2: 更新 HistoryEntry 类型导入**

确保文件顶部有正确的导入（如果使用 contracts.ts 中的类型）：

```typescript
import type { HistoryEntry } from "../types/contracts";
```

- [ ] **Step 3: 类型检查**

运行: `npm run tsc -- --noEmit`

预期: 无类型错误

- [ ] **Step 4: 提交**

```bash
git add src/lib/tauri-bridge.ts
git commit -m "feat(bridge): add getHistoryEntry and exportFromHistory functions"
```

---

### Task 5: 创建 HistoryExportDialog 组件

**Files:**
- Create: `src/components/HistoryExportDialog.tsx`

- [ ] **Step 1: 创建组件文件**

创建 `src/components/HistoryExportDialog.tsx`：

```typescript
import type { ExportFormat } from "../types/contracts";

interface HistoryExportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onExport: (format: ExportFormat) => void;
  isExporting: boolean;
}

const exportOptions: Array<{ format: ExportFormat; label: string; desc: string }> = [
  { format: "html", label: "HTML", desc: "浏览器查看" },
  { format: "md-single", label: "Markdown", desc: "单文件备份" },
  { format: "md-obsidian", label: "Obsidian", desc: "带 frontmatter" },
  { format: "pdf", label: "PDF", desc: "精美排版" },
];

export default function HistoryExportDialog({
  isOpen,
  onClose,
  onExport,
  isExporting,
}: HistoryExportDialogProps) {
  if (!isOpen) return null;

  return (
    <div className="dialog-overlay">
      <div className="dialog-card" style={{ maxWidth: 360 }}>
        <h3 style={{ fontSize: 16, fontWeight: 600, marginBottom: 16 }}>选择导出格式</h3>

        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {exportOptions.map((option) => (
            <button
              key={option.format}
              className="btn btn-secondary"
              type="button"
              onClick={() => onExport(option.format)}
              disabled={isExporting}
              style={{
                width: "100%",
                textAlign: "left",
                padding: "12px 16px",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <span style={{ fontSize: 14, fontWeight: 600 }}>{option.label}</span>
              <span style={{ fontSize: 12, opacity: 0.7 }}>{option.desc}</span>
            </button>
          ))}
        </div>

        <div style={{ marginTop: 16 }}>
          <button
            className="btn btn-secondary"
            type="button"
            onClick={onClose}
            disabled={isExporting}
            style={{ width: "100%" }}
          >
            取消
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 类型检查**

运行: `npm run tsc -- --noEmit`

预期: 无类型错误

- [ ] **Step 3: 提交**

```bash
git add src/components/HistoryExportDialog.tsx
git commit -m "feat(components): add HistoryExportDialog component"
```

---

### Task 6: 创建 HistoryListItem 组件

**Files:**
- Create: `src/components/HistoryListItem.tsx`

- [ ] **Step 1: 创建组件文件**

创建 `src/components/HistoryListItem.tsx`：

```typescript
import type { HistoryEntry } from "../types/contracts";

interface HistoryListItemProps {
  entry: HistoryEntry;
  onExport: () => void;
  onDelete: () => void;
  onOpenDir: () => void;
}

function formatDate(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) return "今天";
  if (diffDays === 1) return "昨天";
  if (diffDays < 7) return `${diffDays} 天前`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)} 周前`;
  return `${Math.floor(diffDays / 30)} 月前`;
}

function formatFilter(filter?: string): string {
  if (!filter) return "全部";
  if (filter === "original") return "仅原创";
  return "全部";
}

function formatDirPath(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export default function HistoryListItem({
  entry,
  onExport,
  onDelete,
  onOpenDir,
}: HistoryListItemProps) {
  return (
    <div className="history-list-item">
      <div className="history-item-header">
        <span className="history-item-name">{entry.screen_name}</span>
        <span className="history-item-time">{formatDate(entry.last_download)}</span>
      </div>

      <div className="history-item-meta">
        <span>{entry.post_count} 条微博</span>
        <span>·</span>
        <span className="history-item-path" title={entry.output_dir}>
          {formatDirPath(entry.output_dir)}
        </span>
      </div>

      <div className="history-item-options">
        选项: {formatFilter(entry.filter)}
        {entry.include_images && " · 含图片"}
        · {entry.date_mode === "all" ? "全部时间" : "指定时间范围"}
      </div>

      <div className="history-item-actions">
        <button
          className="btn-small btn-small--primary"
          type="button"
          onClick={onExport}
          title="导出"
        >
          📤 导出
        </button>
        <button
          className="btn-small"
          type="button"
          onClick={onOpenDir}
          title="打开目录"
        >
          📁 打开
        </button>
        <button
          className="btn-small btn-small--danger"
          type="button"
          onClick={onDelete}
          title="删除记录"
        >
          🗑️ 删除
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 类型检查**

运行: `npm run tsc -- --noEmit`

预期: 无类型错误

- [ ] **Step 3: 提交**

```bash
git add src/components/HistoryListItem.tsx
git commit -m "feat(components): add HistoryListItem component"
```

---

### Task 7: 创建 HistoryPanel 主组件

**Files:**
- Create: `src/components/HistoryPanel.tsx`

- [ ] **Step 1: 创建组件文件**

创建 `src/components/HistoryPanel.tsx`：

```typescript
import { useEffect, useState } from "react";
import HistoryListItem from "./HistoryListItem";
import HistoryExportDialog from "./HistoryExportDialog";
import {
  listDownloadHistory,
  deleteHistoryEntry,
  exportFromHistory,
  openOutputDir,
  type HistoryEntry,
  type ExportFormat,
} from "../lib/tauri-bridge";

interface HistoryPanelProps {
  isOpen: boolean;
  onClose: () => void;
  onLog: (message: string) => void;
}

export default function HistoryPanel({ isOpen, onClose, onLog }: HistoryPanelProps) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [exportingUid, setExportingUid] = useState<string | null>(null);
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [selectedUid, setSelectedUid] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadHistory();
    }
  }, [isOpen]);

  async function loadHistory() {
    setLoading(true);
    try {
      const history = await listDownloadHistory();
      setEntries(history);
    } catch (error) {
      onLog(`加载历史记录失败: ${error}`);
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(uid: string) {
    if (!confirm("确定要删除这条历史记录吗？这不会删除已下载的文件。")) {
      return;
    }

    try {
      await deleteHistoryEntry(uid);
      setEntries((prev) => prev.filter((e) => e.uid !== uid));
      onLog("历史记录已删除");
    } catch (error) {
      onLog(`删除失败: ${error}`);
    }
  }

  async function handleExport(format: ExportFormat) {
    if (!selectedUid) return;

    setExportingUid(selectedUid);
    setShowExportDialog(false);

    try {
      onLog(`正在导出 ${format}...`);
      const result = await exportFromHistory(selectedUid, format);
      onLog(result);
    } catch (error) {
      onLog(String(error));
    } finally {
      setExportingUid(null);
      setSelectedUid(null);
    }
  }

  function openExportDialog(uid: string) {
    setSelectedUid(uid);
    setShowExportDialog(true);
  }

  async function handleOpenDir(outputDir: string) {
    try {
      await openOutputDir(outputDir);
    } catch (error) {
      onLog(`打开目录失败: ${error}`);
    }
  }

  if (!isOpen) return null;

  return (
    <>
      <div className="history-overlay" onClick={onClose} />
      <aside className="history-panel">
        <div className="history-header">
          <h2>📚 历史记录</h2>
          <button className="history-close" onClick={onClose} type="button">
            ×
          </button>
        </div>

        {loading ? (
          <div className="history-loading">加载中...</div>
        ) : entries.length === 0 ? (
          <div className="history-empty">
            <p>暂无下载历史</p>
            <p style={{ fontSize: 12, opacity: 0.7 }}>先去下载一些微博吧</p>
          </div>
        ) : (
          <div className="history-list">
            {entries.map((entry) => (
              <HistoryListItem
                key={entry.uid}
                entry={entry}
                onExport={() => openExportDialog(entry.uid)}
                onDelete={() => handleDelete(entry.uid)}
                onOpenDir={() => handleOpenDir(entry.output_dir)}
              />
            ))}
          </div>
        )}
      </aside>

      <HistoryExportDialog
        isOpen={showExportDialog}
        onClose={() => {
          setShowExportDialog(false);
          setSelectedUid(null);
        }}
        onExport={handleExport}
        isExporting={exportingUid !== null}
      />
    </>
  );
}
```

- [ ] **Step 2: 类型检查**

运行: `npm run tsc -- --noEmit`

预期: 无类型错误

- [ ] **Step 3: 提交**

```bash
git add src/components/HistoryPanel.tsx
git commit -m "feat(components): add HistoryPanel sidebar component"
```

---

### Task 8: 添加侧边栏 CSS 样式

**Files:**
- Modify: `src/App.css`

- [ ] **Step 1: 添加侧边栏相关样式**

在 `src/App.css` 文件末尾添加：

```css
/* 历史记录侧边栏 */
.history-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.3);
  z-index: 90;
  animation: fadeIn 160ms ease-out;
}

.history-panel {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  width: 340px;
  background: var(--color-bg-elevated);
  border-left: 1px solid var(--color-border);
  z-index: 91;
  display: flex;
  flex-direction: column;
  animation: slideInRight 240ms ease-out;
}

.history-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid var(--color-border);
}

.history-header h2 {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}

.history-close {
  background: none;
  border: none;
  font-size: 24px;
  line-height: 1;
  padding: 0;
  width: 32px;
  height: 32px;
  cursor: pointer;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-md);
}

.history-close:hover {
  background: var(--color-bg-inset);
  color: var(--color-text);
}

.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.history-loading,
.history-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--color-text-secondary);
  font-size: 14px;
}

/* 历史记录列表项 */
.history-list-item {
  padding: 12px;
  border-radius: var(--radius-md);
  background: var(--color-bg-inset);
  margin-bottom: 8px;
  transition: background-color 120ms ease;
}

.history-list-item:hover {
  background: var(--color-border-subtle);
}

.history-item-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.history-item-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--color-text);
}

.history-item-time {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.history-item-meta {
  display: flex;
  gap: 6px;
  font-size: 12px;
  color: var(--color-text-secondary);
  margin-bottom: 4px;
}

.history-item-path {
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.history-item-options {
  font-size: 11px;
  color: var(--color-text-tertiary);
  margin-bottom: 8px;
}

.history-item-actions {
  display: flex;
  gap: 6px;
}

/* 小按钮样式 */
.btn-small {
  flex: 1;
  padding: 6px 8px;
  font-size: 12px;
  border: 1px solid var(--color-border);
  background: var(--color-bg);
  color: var(--color-text);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 120ms ease;
}

.btn-small:hover {
  background: var(--color-bg-inset);
}

.btn-small--primary {
  background: var(--color-accent);
  color: white;
  border-color: var(--color-accent);
}

.btn-small--primary:hover {
  opacity: 0.9;
}

.btn-small--danger {
  color: var(--color-danger);
}

.btn-small--danger:hover {
  background: rgba(255, 69, 58, 0.1);
}

/* 动画 */
@keyframes slideInRight {
  from {
    transform: translateX(100%);
  }
  to {
    transform: translateX(0);
  }
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

/* 响应式 */
@media (max-width: 600px) {
  .history-panel {
    width: 100%;
  }
}
```

- [ ] **Step 2: 提交**

```bash
git add src/App.css
git commit -m "feat(styles): add history panel sidebar styles"
```

---

### Task 9: 在 App.tsx 中集成历史面板

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: 导入 HistoryPanel 组件**

在 `src/App.tsx` 顶部添加导入：

```typescript
import HistoryPanel from "./components/HistoryPanel";
```

- [ ] **Step 2: 添加历史面板状态**

在 `AppShell` 函数组件内添加状态：

```typescript
const [isHistoryOpen, setIsHistoryOpen] = useState(false);
```

- [ ] **Step 3: 在头部添加历史按钮**

找到 `app-header` 部分，修改添加历史按钮：

```typescript
<header className="app-header">
  <h1 className="app-title">微存 <span style={{ fontSize: "0.5em", opacity: 0.6, fontWeight: 400, marginLeft: 8 }}>Wecun</span></h1>
  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
    {state.isLoggedIn && (
      <>
        <button
          className="btn btn-ghost"
          onClick={() => setIsHistoryOpen(true)}
          type="button"
          style={{ padding: "6px 12px", fontSize: 13 }}
        >
          📚 历史
        </button>
        <span className="auth-badge">
          <span className="auth-dot" />
          已登录
        </span>
        {/* ... 现有退出登录按钮 ... */}
      </>
    )}
  </div>
</header>
```

- [ ] **Step 4: 在返回的 JSX 中添加 HistoryPanel**

在 `return` 语句的 JSX 中，在 `</div>` 闭合标签前添加：

```typescript
{isHistoryOpen && (
  <HistoryPanel
    isOpen={isHistoryOpen}
    onClose={() => setIsHistoryOpen(false)}
    onLog={(msg) => dispatch({ type: "ADD_LOG", message: msg })}
  />
)}
```

- [ ] **Step 5: 类型检查**

运行: `npm run tsc -- --noEmit`

预期: 无类型错误

- [ ] **Step 6: 提交**

```bash
git add src/App.tsx
git commit -m "feat(app): integrate history panel with sidebar button"
```

---

### Task 10: 端到端测试

**Files:**
- (No new files, manual testing)

- [ ] **Step 1: 编译项目**

运行: `npm run tauri build`

预期: 成功编译，无错误

- [ ] **Step 2: 启动开发服务器**

运行: `npm run tauri dev`

- [ ] **Step 3: 测试基本流程**

1. 登录微博账号
2. 下载一个博主的微博（确保有历史记录）
3. 点击顶部"📚 历史"按钮
4. 验证侧边栏正确滑出
5. 验证历史记录显示正确（博主名、微博数量、时间等）
6. 点击"导出"按钮
7. 选择一种格式并导出
8. 验证导出成功且未重新下载
9. 点击"打开"按钮，验证文件夹正确打开
10. 点击"删除"按钮，验证记录被删除

- [ ] **Step 4: 测试边界情况**

1. 无历史记录时打开侧边栏 - 应显示空状态
2. 导出时点击遮罩 - 应关闭侧边栏
3. 删除记录后取消 - 记录应保留
4. 快速连续点击导出 - 应正常处理

- [ ] **Step 5: 测试响应式**

1. 缩小窗口到 < 600px
2. 打开历史面板
3. 验证侧边栏占满整个内容区

- [ ] **Step 6: 记录测试结果**

如有问题，创建对应的 bug 修复任务。

- [ ] **Step 7: 提交**

```bash
git add .
git commit -m "test: complete end-to-end testing for history panel phase 1"
```

---

## 验收标准

完成所有任务后：

1. ✅ 用户可以通过顶部"历史"按钮打开侧边栏
2. ✅ 侧边栏显示所有下载历史记录
3. ✅ 每条记录显示博主名、微博数量、下载时间、保存目录
4. ✅ 用户可以点击"导出"按钮直接导出，无需重新下载
5. ✅ 用户可以点击"打开"按钮打开保存目录
6. ✅ 用户可以删除历史记录
7. ✅ 空状态正确显示
8. ✅ 侧边栏动画流畅
9. ✅ 响应式设计在窄窗口下正常工作

---

## 下一步（第二阶段）

完成第一阶段后，可以继续实现：

1. 重新下载功能（覆盖模式）
2. 详细信息展开
3. 搜索功能
4. 增量下载模式

这些功能将在独立的实施计划中定义。
