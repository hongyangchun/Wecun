# 微博下载器 (Weibo Downloader)

一个跨平台的桌面应用，可以下载并导出指定用户的全部微博内容。

## 功能特性

- 🔍 输入微博用户ID即可下载
- 📝 支持仅原创或全部微博
- 🖼️ 可选是否包含图片
- 📅 支持全部微博或指定时间段
- 📄 导出为 PDF 或 Markdown 格式
- 🪟 支持 Windows 和 macOS

## 快速开始

### 下载

前往 [Releases](../../releases) 页面下载对应平台的安装包：
- **macOS**: `.dmg` 文件
- **Windows**: `.exe` 安装包

### 使用说明

1. 打开应用
2. 获取你的微博 Cookie（见下方说明）
3. 输入要下载的用户ID（在其微博主页URL中可见）
4. 选择下载选项
5. 选择保存目录
6. 点击"开始下载"

## 如何获取 Cookie

1. 打开浏览器，访问 [weibo.com](https://weibo.com) 并登录
2. 按 `F12` 打开开发者工具
3. 切换到 **Network（网络）** 标签
4. 刷新页面
5. 在请求列表中找到任意一个请求
6. 查看 **Request Headers**，找到 **Cookie** 字段
7. 复制整个 Cookie 值粘贴到应用输入框

> ⚠️ Cookie 会过期，如果下载失败请重新获取。

## 从源码构建

### 前置条件

- Node.js 18+
- Rust 1.70+
- macOS: Xcode Command Line Tools
- Windows: Visual Studio Build Tools

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
| PDF（单文件） | 所有微博合并在一个PDF文件中 |
| Markdown（单文件） | 所有微博合并在一个 `.md` 文件中 |
| Markdown（每条微博一个文件） | 每条微博一个 `.md` 文件，存放在 `posts/` 子目录中 |

## 测试与冒烟检查

### Rust 测试

```bash
cd src-tauri
cargo test
```

测试覆盖：
- 微博 AJAX 接口响应 fixture 解析
- 微博数据标准化逻辑
- Markdown 单文件导出
- Markdown 按微博拆分导出

### Smoke Checklist

- [ ] 应用可以正常启动
- [ ] Cookie 可以成功保存并用于请求
- [ ] 输入有效用户 ID 后可以拉取用户信息
- [ ] “仅原创/全部微博”筛选符合预期
- [ ] 指定时间范围下载结果正确
- [ ] Markdown 单文件导出成功
- [ ] Markdown 每条微博单独导出成功
- [ ] PDF 导出成功
- [ ] 勾选图片下载时 `images/` 目录生成成功
- [ ] 取消下载后任务能及时停止
- [ ] 无效或过期 Cookie 会给出错误提示

## 技术栈

- **前端**: React + TypeScript + Vite
- **后端**: Rust (Tauri 2)
- **HTTP**: reqwest
- **PDF**: genpdf
- **打包**: Tauri Bundler

## 微博 API

本应用使用微博网页版 AJAX 接口，通过用户登录后的 Cookie 进行身份验证。主要接口包括：
- `/ajax/profile/info` — 获取用户信息
- `/ajax/profile/mbloghistory` — 获取微博年月分布
- `/ajax/statuses/searchProfile` — 获取微博列表（分页）
- `/ajax/statuses/longtext` — 获取长微博全文

## 注意事项

- 请遵守微博的使用条款，不要频繁大量下载
- 应用内置 1秒/次 的请求频率限制
- 下载过程中可以随时取消
- 图片会下载到 `images/` 子目录

## License

MIT
