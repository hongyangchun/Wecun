# 下载历史管理功能设计文档

**日期:** 2026-04-09
**状态:** 设计草稿

## 概述

在当前的下载微博功能之外增加一个完整的下载历史管理功能。用户曾经下载过的博主微博可以直接进行格式转换导出，无需重新下载。同时支持增量更新和覆盖下载两种重新下载模式。

## 用户需求

1. **快速重新导出** - 对已下载的微博直接选择不同格式导出，跳过重新下载步骤
2. **完整历史管理** - 查看和管理所有下载记录
3. **增量更新** - 可选择仅下载新内容，而非完全覆盖
4. **灵活操作** - 支持删除记录、打开保存目录等操作

## 设计方案：侧边栏历史面板

### 整体布局

应用顶部导航栏新增历史按钮，点击后右侧滑出历史面板：

```
┌─────────────────────────────────────────────────┐
│ 微存 Wecun              [新建下载▼] [📚历史]    │
├─────────────────────────────────────────────────┤
│                                                 │
│  当前主内容区域...    │  📚 历史记录         ×   │
│  （4步向导或处理界面）  │  ┌─────────────────┐   │
│                       │  │ 🔍 搜索...       │   │
│                       │  ├─────────────────┤   │
│                       │  │ 张三  @zhangsan │   │
│                       │  │ 123条 · 2天前   │   │
│                       │  │ [导出] [重下]   │   │
│                       │  ├─────────────────┤   │
│                       │  │ 李四  @lisi     │   │
│                       │  │ 456条 · 1周前   │   │
│                       │  │ [导出] [重下]   │   │
│                       │  └─────────────────┘   │
│                       │                        │
└─────────────────────────────────────────────────┘
```

### 历史记录列表项

**单条记录展示：**

```
┌────────────────────────────────────┐
│ 张三  @zhangsan          2天前      │
│ 123条微博 · /Users/.../张三微博     │
│ 选项: 原创 · 含图片 · 全部时间      │
│                                    │
│ [📤导出] [🔄重下] [📁打开] [🗑️删除] │
└────────────────────────────────────┘
```

**详细信息展开（点击记录主体）：**

```
┌────────────────────────────────────┐
│ 张三  @zhangsan          2天前      │
│ 123条微博 · /Users/.../张三微博     │
│ ──────────────────────────────────  │
│ 下载选项:                          │
│ • 过滤: 仅原创                     │
│ • 图片: 已下载                     │
│ • 时间: 全部                       │
│ • 最小字数: 0                      │
│                                    │
│ [📤导出] [🔄重下] [📁打开] [🗑️删除] │
└────────────────────────────────────┘
```

### 快速导出流程

1. 用户点击历史记录项的"导出"按钮
2. 弹出格式选择对话框
3. 用户选择格式后直接导出（使用缓存数据）
4. 显示进度，完成后可继续导出其他格式

**导出格式选择对话框：**

```
┌────────────────────────────────────┐
│ 选择导出格式                        │
├────────────────────────────────────┤
│                                    │
│  [HTML]        浏览器查看          │
│                                    │
│  [Markdown]   单文件备份           │
│                                    │
│  [Obsidian]   带 frontmatter       │
│                                    │
│  [PDF]        精美排版             │
│                                    │
├────────────────────────────────────┤
│            [取消]                  │
└────────────────────────────────────┘
```

### 重新下载流程

**模式选择对话框：**

```
┌────────────────────────────────────┐
│ 重新下载模式                        │
├────────────────────────────────────┤
│                                    │
│  🔄 覆盖模式                       │
│     清空旧数据，重新下载所有微博    │
│                                    │
│  ➕ 增量模式                       │
│     保留已下载的微博，仅下载新内容  │
│                                    │
├────────────────────────────────────┤
│        [取消]        [确认]        │
└────────────────────────────────────┘
```

- **覆盖模式**：删除现有缓存，按照原选项（或新选项）重新下载所有内容
- **增量模式**：保留现有数据，仅下载自上次以来的新微博并合并

## 技术实现

### 后端修改（Rust）

#### 新增数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedownloadMode {
    Overwrite,
    Incremental,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRangeOption {
    pub mode: String,  // "all" | "range"
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntryDetail {
    pub uid: String,
    pub screen_name: String,
    pub output_dir: String,
    pub last_download: String,
    pub post_count: usize,
    pub source_type: SourceType,
    // 新增：保存完整下载选项
    pub filter: String,  // "original" | "all"
    pub include_images: bool,
    pub date_range: DateRangeOption,
    pub ignore_deleted: bool,
    pub min_text_length: usize,
}
```

#### 新增命令

```rust
#[tauri::command]
pub async fn export_from_history(
    uid: String,
    export_format: ExportFormat,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // 1. 从历史记录中查找该 uid 的 output_dir
    // 2. 使用 export_posts 命令的逻辑导出
}

#[tauri::command]
pub async fn redownload_with_mode(
    uid: String,
    mode: RedownloadMode,
    new_options: Option<DownloadOptions>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // 1. 从历史记录中查找该 uid 的信息
    // 2. overwrite: 删除缓存，重新下载
    // 3. incremental: 读取缓存最新时间，增量下载
}

#[tauri::command]
pub async fn get_history_entry(
    uid: String,
    app: tauri::AppHandle,
) -> Result<HistoryEntryDetail, String> {
    // 获取单条历史记录的详细信息
}
```

#### 增量下载逻辑

在 `DownloadService` 中添加增量下载支持：

1. **读取现有缓存**：从 `output_dir` 加载 `cache_bundle.json`，解析现有微博列表
2. **获取最新时间**：找到所有微博中最新的 `created_at` 时间戳
3. **过滤新微博**：API 请求使用该时间戳作为 `since` 参数，仅获取新发布的微博
4. **数据合并**：
   - 新微博追加到现有列表
   - 更新 `export_context` 中的用户信息（可能变化）
   - 更新 `last_download` 时间
5. **保存缓存**：将合并后的数据重新序列化保存
6. **历史记录更新**：更新 `download_history.json` 中的 `post_count` 和 `last_download`

**边界情况处理**：
- 如果缓存文件损坏或不存在，回退到完全重新下载
- 如果没有新微博，提示用户并取消操作

### 前端修改（TypeScript/React）

#### 新增组件

1. **`HistoryPanel.tsx`** - 历史记录侧边栏主组件
   - 显示历史记录列表
   - 处理搜索、筛选
   - 处理各项操作（导出、重下、删除等）

2. **`HistoryExportDialog.tsx`** - 快速导出格式选择对话框

3. **`RedownloadDialog.tsx`** - 重新下载模式选择对话框

4. **`HistoryListItem.tsx`** - 单条历史记录项组件
   - 显示摘要/详细信息
   - 操作按钮

#### 修改组件

1. **`App.tsx`** - 添加侧边栏状态和切换逻辑
2. **`StepIndicator.tsx`** 或顶部导航 - 添加历史按钮

#### 新增 TypeScript 类型

```typescript
export interface DateRangeOption {
  mode: "all" | "range";
  start_timestamp: number | null;
  end_timestamp: number | null;
}

export interface HistoryEntry {
  uid: string;
  screen_name: string;
  output_dir: string;
  last_download: string;
  post_count: number;
  source_type: "profile" | "favorites";
  filter: "original" | "all";
  include_images: boolean;
  date_range: DateRangeOption;
  ignore_deleted: boolean;
  min_text_length: number;
}

export type RedownloadMode = "overwrite" | "incremental";
```

#### 新增 Tauri Bridge 函数

```typescript
export function exportFromHistory(uid: string, format: ExportFormat): Promise<string>;
export function redownloadWithMode(uid: string, mode: RedownloadMode, options?: DownloadRequest): Promise<string>;
export function getHistoryEntry(uid: string): Promise<HistoryEntry>;
export function openHistoryDirectory(outputDir: string): Promise<void>;
```

## UI/UX 细节

1. **侧边栏宽度**：340px 固定宽度
2. **动画**：侧边栏从右侧滑入，使用 240ms ease-out 过渡
3. **状态禁用**：侧边栏打开时，主区域显示半透明遮罩，交互被禁用
4. **响应式设计**：
   - 窗口宽度 < 600px 时：侧边栏占满整个内容区
   - 窗口宽度 >= 600px 时：侧边栏宽度为 340px，主区域自适应
5. **空状态**：无历史记录时显示提示信息"暂无下载历史，先去下载一些微博吧"
6. **加载状态**：历史记录加载时显示骨架屏或加载指示器

## 实现优先级

### 第一阶段（核心功能）
1. 基础侧边栏 UI
2. 历史记录列表展示
3. 快速导出功能
4. 删除记录功能

### 第二阶段（增强功能）
5. 重新下载（覆盖模式）
6. 打开保存目录
7. 详细信息展开

### 第三阶段（高级功能）
8. 增量下载模式
9. 搜索功能
10. 历史记录筛选

## 兼容性考虑

- 现有下载流程完全不受影响
- 历史记录数据格式向后兼容（新字段可选）
- 增量下载作为可选功能，不影响核心下载逻辑

## 未来扩展

- 收藏夹功能（标记重要的历史记录）
- 历史记录导出/导入
- 自动备份策略
- 历史记录统计（总微博数、总占用空间等）
