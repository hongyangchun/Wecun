# 微存 (Wecun)

一个跨平台的桌面应用，可以下载并导出为多种格式指定博主的全部微博内容。

## 功能特性

- 🔍 下载任意指定博主的原创微博/所有微博
- ⭐ 下载登录账号的收藏微博
- 🖼️ 可选是否包含图片
- 📅 可选全部微博或指定时间段内的微博
- 📄 导出为多种格式：HTML、Markdown（单文件）、Markdown（Obsidian兼容）、Markdown（分文件）
- 🪟 支持 Windows、macOS（Apple Silicon & Intel）和 Linux

## 快速开始

### 下载

前往 [Releases](../../releases) 页面下载对应平台的安装包：
- **macOS** (Apple Silicon & Intel): `.dmg` 文件
- **Windows**: `.exe` 安装包
- **Linux**: `.deb` 或 `.AppImage` 文件

### 使用说明

1. 打开应用
2. 在应用内登录微博
3. 选择下载来源：「博主主页」或「我的收藏」
   - **博主主页**：输入目标博主主页地址，例如 `https://www.weibo.com/u/2166767661`
   - **我的收藏**：无需输入，将下载当前登录账号的收藏微博
4. 选择下载选项（原创/全部、时间范围、是否含图片）
5. 选择保存目录并开始下载
6. 下载完成后选择导出格式（Markdown / HTML）并导出

> ⚠️ 如果登录失效，应用会提示重新登录。

## 从源码构建

### 前置条件

- Node.js 18+
- Rust 1.70+
- macOS: Xcode Command Line Tools
- Windows: Visual Studio Build Tools
- Linux: `libwebkit2gtk-4.1-dev`、`libgtk-3-dev`、`libappindicator3-dev`、`librsvg2-dev`、`patchelf`

### 构建步骤

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建安装包
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录下。

## 导出格式说明

| 格式 | 说明 |
|------|------|
| Markdown（单文件） | 所有微博合并在一个 `.md` 文件中 |
| Markdown（每条微博一个文件） | 每条微博一个 `.md` 文件，存放在 `posts/` 子目录中 |
| HTML（单文件） | 所有微博合并在一个 `.html` 文件中，可直接在浏览器中查看 |

## 测试与冒烟检查

### Rust 测试

```bash
cd src-tauri
cargo test
```

测试覆盖：
- 微博 AJAX 接口响应 fixture 解析
- 微博数据标准化逻辑
- 长微博归一化逻辑
- Markdown 单文件导出
- Markdown 按微博拆分导出
- HTML 单文件导出

### Smoke Checklist

- [ ] 应用可以正常启动
- [ ] 已登录状态下重新打开应用可显示"下一步"和"退出登录"
- [ ] 应用内登录状态可以成功保存并用于请求
- [ ] 输入有效微博主页地址后可以拉取用户信息
- [ ] "仅原创/全部微博"筛选符合预期
- [ ] 指定时间范围下载结果正确
- [ ] 长微博可以导出完整正文
- [ ] Markdown 单文件导出成功
- [ ] Markdown 每条微博单独导出成功
- [ ] Markdown 每条微博导出包含 `posts/index.md`
- [ ] HTML 导出成功且可在浏览器中正常显示
- [ ] 下载缓存正确写入，重复导出无需重新下载
- [ ] 勾选图片下载时 `images/` 目录生成成功
- [ ] "打开下载目录"按钮可用
- [ ] 取消下载后任务能及时停止
- [ ] 无效或过期登录状态会提示重新登录

## 技术栈

- **前端**: React + TypeScript + Vite
- **后端**: Rust (Tauri 2)
- **HTTP**: reqwest
- **打包**: Tauri Bundler

## 微博 API

本应用使用微博网页版 AJAX 接口，通过应用内登录后的会话进行身份验证。主要接口包括：
- `/ajax/profile/info` — 获取用户信息
- `/ajax/profile/mbloghistory` — 获取微博年月分布
- `/ajax/statuses/searchProfile` — 获取微博列表（分页）
- `/ajax/favorites/all` — 获取收藏微博列表（分页）
- `/ajax/statuses/longtext` — 获取长微博全文

## 注意事项

- 请遵守微博的使用条款，不要频繁大量下载
- 应用内置 1秒/次 的请求频率限制
- 下载过程中可以随时取消
- 图片会下载到 `images/` 子目录

## License

MIT
