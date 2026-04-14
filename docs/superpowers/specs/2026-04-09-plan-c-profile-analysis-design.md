# Plan C — 博主画像分析功能设计文档

**日期**: 2026-04-09
**版本**: 0.1 (草案)
**状态**: 待确认

---

## 1. 概述

为微存(Wecun)新增博主画像分析功能，利用本地统计 + 智谱 GLM-4-Flash 免费 AI API，深度分析博主的内容特征、性格画像和生活方式。分析结果在应用内展示，并支持导出为 OpenClaw 兼容格式（identity.md + soul.md）。

### 1.1 用户确认的功能范围

- ✅ 基础统计（发博频率、时间分布、字数统计、活跃时段）
- ✅ 内容分析（高频词云、话题标签、@提及分析）
- ✅ 情感分析（正负面情绪比例、趋势）
- ✅ 影响力指标（转发/评论/点赞比率、互动分析）
- ✅ AI 深度个人分析（MBTI、性格、兴趣、价值观、政治立场、哲学理念等）
- ✅ OpenClaw 兼容导出（identity.md + soul.md）
- ✅ 应用内展示（非生成独立 HTML 文件）

### 1.2 核心技术决策

| 决策项 | 选择 | 说明 |
|--------|------|------|
| AI 引擎 | 智谱 GLM-4-Flash | 免费、不限额度 |
| API Key | **待确认** | 见 §2 |
| 分析结果展示 | 应用内 React 组件 | 新增分析面板 |
| 图表库 | 待定（Chart.js / ECharts） | 用于数据可视化 |
| 数据采样 | 智能抽样 | 多于 200 条时抽样 |

---

## 2. ⚠️ 待确认：API Key 管理方案

这是最关键的架构决策，直接影响整个实现方案。

### 方案 A：开发者内嵌 Key

```
应用内硬编码 API Key → 所有用户共享
```

- ✅ 用户零配置，体验最好
- ❌ Key 可被反编译提取
- ❌ 所有用户共享额度，高峰期可能限流
- ❌ Key 泄露风险

**适用场景**：快速验证 MVP

### 方案 B：用户自带 Key

```
设置页面输入 API Key → 本地加密存储 → 调用时使用
```

- ✅ 安全，无 Key 泄露风险
- ✅ 每个用户独立额度
- ❌ 增加使用门槛（需要注册智谱账号）
- ❌ 非技术用户可能不会操作

**适用场景**：面向开发者/技术用户

### 方案 C：后端代理（推荐长期方案）

```
应用 → 你的后端服务 → 智谱 API
后端做限流、计费、Key 保护
```

- ✅ 最安全，Key 不暴露
- ✅ 可做精细的免费额度控制
- ❌ 需要服务器成本
- ❌ 增加开发和运维复杂度

**适用场景**：正式商业化

### 建议

**阶段 1（MVP）**：方案 A，快速验证功能
**阶段 2（正式发布）**：迁移到方案 C

---

## 3. 技术架构

### 3.1 整体架构

```
┌─────────────────────────────────────────────────────┐
│                 前端 (React + TypeScript)             │
│  ┌──────────────┐  ┌───────────────┐                │
│  │ StepProcessing│  │ ProfilePanel  │                │
│  │ (+ 画像分析按钮)│  │ (分析结果展示) │                │
│  └──────────────┘  └───────────────┘                │
└────────────┬────────────────────────────────────────┘
             │ Tauri Commands
┌────────────▼────────────────────────────────────────┐
│              后端 (Rust + Tauri 2)                    │
│  ┌──────────────┐  ┌──────────────┐                 │
│  │ LocalAnalyzer │  │ GLMClient    │                 │
│  │ (本地统计计算) │  │ (AI 深度分析) │                 │
│  └──────┬───────┘  └──────┬───────┘                 │
│         │                  │                         │
│  ┌──────▼──────────────────▼──────┐                 │
│  │     ProfileService              │                 │
│  │     (编排本地+AI分析)            │                 │
│  └────────────────────────────────┘                 │
│  ┌──────────────┐                                   │
│  │ OpenClawExporter│                                │
│  │ (identity/soul) │                                │
│  └──────────────┘                                   │
└─────────────────────────────────────────────────────┘
```

### 3.2 新增文件清单

| 文件路径 | 职责 |
|---------|------|
| `src-tauri/src/services/profile/local_analyzer.rs` | 本地统计分析 |
| `src-tauri/src/services/profile/glm_client.rs` | 智谱 API 调用客户端 |
| `src-tauri/src/services/profile/mod.rs` | Profile 模块入口 |
| `src-tauri/src/services/profile/models.rs` | 分析结果数据模型 |
| `src-tauri/src/services/export_openclaw.rs` | OpenClaw identity.md + soul.md 导出 |
| `src/components/ProfilePanel.tsx` | 画像分析结果展示组件 |
| `src/components/ProfileCharts.tsx` | 图表可视化子组件 |

### 3.3 新增 Tauri Commands

```rust
#[tauri::command]
pub async fn analyze_profile(
    output_dir: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ProfileResult, String>

#[tauri::command]
pub async fn export_openclaw(
    output_dir: String,
    profile: ProfileResult,
    app: tauri::AppHandle,
) -> Result<String, String>
```

### 3.4 数据流

```
用户点击"画像分析"按钮
    ↓
Tauri command: analyze_profile(output_dir)
    ↓
加载 .weibo-cache/posts.json
    ↓
┌───────────────────────────────────────┐
│ Step 1: 本地分析 (LocalAnalyzer)       │
│ • 基础统计（频率、时间、字数）          │
│ • 内容分析（词频、标签、@提及）          │
│ • 影响力指标（互动率）                   │
│ → 输出 LocalStats                      │
└───────────────┬───────────────────────┘
                ↓
┌───────────────────────────────────────┐
│ Step 2: AI 深度分析 (GLMClient)        │
│ • 智能抽样微博内容（≤200条）            │
│ • 构建分析 prompt                       │
│ • 调用 GLM-4-Flash API                 │
│ • 解析 JSON 响应                        │
│ → 输出 DeepProfile                      │
└───────────────┬───────────────────────┘
                ↓
合并 LocalStats + DeepProfile → ProfileResult
    ↓
返回前端 → ProfilePanel 渲染展示
    ↓
（可选）用户点击"导出 OpenClaw"
    ↓
export_openclaw() → identity.md + soul.md
```

---

## 4. 本地分析模块 (LocalAnalyzer)

### 4.1 基础统计

```rust
struct BasicStats {
    total_posts: usize,
    total_original: usize,
    total_reposts: usize,
    avg_text_length: f64,
    max_text_length: usize,
    min_text_length: usize,
    posting_frequency: PostingFrequency,
    time_distribution: TimeDistribution,  // 24小时分布
    active_hours: (u8, u8),               // 最活跃时段
    weekly_distribution: Vec<usize>,       // 周一到周日
    monthly_distribution: HashMap<String, usize>, // 按月
}

struct PostingFrequency {
    posts_per_day: f64,
    posts_per_week: f64,
    posts_per_month: f64,
    most_active_date: String,     // 发博最多的一天
    total_active_days: usize,     // 有发博的天数
}
```

### 4.2 内容分析

```rust
struct ContentAnalysis {
    top_words: Vec<(String, usize)>,      // Top 50 高频词
    top_hashtags: Vec<(String, usize)>,    // 话题标签统计
    top_mentions: Vec<(String, usize)>,    // @提及统计
    avg_hashtags_per_post: f64,
    avg_mentions_per_post: f64,
    posts_with_images_ratio: f64,          // 含图微博比例
    emoji_frequency: Vec<(String, usize)>, // 常用表情
}
```

> **注意**：高频词提取需要中文分词。考虑两种方案：
> - 方案 A：引入 `jieba-rs` crate（增加二进制体积约 10MB，分词质量好）
> - 方案 B：简单的 n-gram 统计 + 停用词过滤（体积小但精度低）
> - **建议使用方案 A**，jieba-rs 是成熟的中文分词库

### 4.3 影响力指标

```rust
struct InfluenceMetrics {
    // 从微博 API 返回数据中提取（如果有 reposts/comments/likes 计数）
    // 注意：当前 WeiboPost 模型没有互动数据，需要确认是否可用
    source_distribution: HashMap<String, usize>, // 发博来源分布（iPhone、Android、网页等）
    region_distribution: Vec<(String, usize)>,   // 发博地区分布
}
```

> **⚠️ 数据限制**：当前 `WeiboPost` 模型不包含 reposts/comments/likes 计数。如果微博 API 返回这些字段，需要扩展模型。否则影响力指标只能做有限分析。

### 4.4 情感分析（本地部分）

```rust
struct SentimentAnalysis {
    positive_ratio: f64,
    negative_ratio: f64,
    neutral_ratio: f64,
    sentiment_trend: Vec<(String, f64)>,  // 按月的情感趋势
}
```

情感分析也可以结合 AI 分析来做，本地基于词典的方案精度有限。

---

## 5. AI 深度分析模块 (GLMClient)

### 5.1 智谱 API 调用

```
POST https://open.bigmodel.cn/api/paas/v4/chat/completions
Headers: Authorization: Bearer {API_KEY}
Body: {
    "model": "glm-4-flash",
    "messages": [
        { "role": "system", "content": "{系统提示词}" },
        { "role": "user", "content": "{微博内容样本}" }
    ],
    "temperature": 0.3,
    "response_format": { "type": "json_object" }
}
```

### 5.2 数据采样策略

当微博数量 > 200 条时，采用分层抽样：

```
1. 按互动量排序取 Top 50（最具代表性的内容）
2. 按时间均匀抽样 100 条（覆盖不同时期）
3. 随机抽样 50 条（增加随机性）
→ 总计约 200 条，约 40K-60K tokens
```

对于 ≤ 200 条的用户，发送全部内容。

### 5.3 System Prompt 设计

```
你是一位专业的内容分析师和人格心理学家。请根据以下微博内容，对博主进行深度画像分析。

请严格以 JSON 格式输出分析结果，包含以下字段：

{
  "personal_info": {
    "estimated_age_range": "推测年龄段",
    "zodiac_sign": "星座（如果能从内容推断出生日期）",
    "mbti_type": "MBTI 人格类型",
    "gender": "性别",
    "possible_cities": ["可能居住的城市"],
    "possible_occupation": "可能的职业/行业"
  },
  "personality": {
    "traits": ["主要性格特征（3-5个）"],
    "communication_style": "沟通风格描述",
    "social_orientation": "社交倾向（内向/外向/中间）",
    "humor_style": "幽默风格"
  },
  "interests": {
    "music": ["喜欢的音乐/歌手"],
    "books": ["喜欢的书籍/作者"],
    "movies": ["喜欢的电影/影视剧"],
    "hobbies": ["兴趣爱好"],
    "sports": ["运动偏好"],
    "food": ["美食偏好"]
  },
  "values": {
    "political_stance": "政治立场倾向",
    "philosophy": "人生哲学/理念",
    "worldview": "世界观描述",
    "core_values": ["核心价值观"],
    "attitude_toward_life": "生活态度"
  },
  "content_style": {
    "writing_style": "写作风格",
    "common_topics": ["常讨论的话题"],
    "emotional_tone": "整体情绪基调",
    "expression_habits": "表达习惯"
  },
  "sentiment_analysis": {
    "overall_sentiment": "positive/neutral/negative",
    "emotional_stability": "情绪稳定性评估",
    "emotional_triggers": ["情绪触发话题"],
    "happiness_index": 0.0-1.0
  }
}

注意：
1. 所有推测都应基于微博内容中的实际证据
2. 如果某些信息无法从内容中推断，请标注"无法判断"
3. 保持客观中立，避免刻板印象
4. 在 JSON 之外不要输出任何其他内容
```

### 5.4 DeepProfile 数据模型

```rust
struct DeepProfile {
    personal_info: PersonalInfo,
    personality: PersonalityAnalysis,
    interests: InterestAnalysis,
    values: ValueAnalysis,
    content_style: ContentStyleAnalysis,
    sentiment: AiSentimentAnalysis,
}
```

---

## 6. 前端展示 (ProfilePanel)

### 6.1 UI 设计

在 StepProcessing 组件的 "下载完成" 状态中，新增一个 "画像分析" 按钮，与现有的导出格式按钮并列。

点击后进入分析流程（显示进度），分析完成后展示 ProfilePanel：

```
┌─────────────────────────────────────────┐
│           博主画像分析报告                 │
│         @用户名 · 共 XXX 条微博           │
├─────────────────────────────────────────┤
│                                         │
│  ┌── 个人画像 ──────────────────────┐   │
│  │ 🎂 星座: 天秤座                   │   │
│  │ 🧠 MBTI: ENFP                    │   │
│  │ 🏙️ 城市: 上海/杭州               │   │
│  │ 💼 职业: 互联网产品经理            │   │
│  │ 🎭 性格: 开朗好奇、善于表达        │   │
│  └──────────────────────────────────┘   │
│                                         │
│  ┌── 发博统计 ──────────────────────┐   │
│  │ [时间分布柱状图 - 24小时]          │   │
│  │ 日均 X.X 条 · 活跃时段 XX:XX-XX:XX│   │
│  └──────────────────────────────────┘   │
│                                         │
│  ┌── 内容分析 ──────────────────────┐   │
│  │ [词云图]                          │   │
│  │ Top 话题: #标签1 #标签2 #标签3     │   │
│  │ 常提及: @用户1 @用户2              │   │
│  └──────────────────────────────────┘   │
│                                         │
│  ┌── 兴趣偏好 ──────────────────────┐   │
│  │ 🎵 音乐: 歌手A、歌手B             │   │
│  │ 📚 书籍: 书A、书B                  │   │
│  │ 🎬 电影: 电影A                     │   │
│  └──────────────────────────────────┘   │
│                                         │
│  ┌── 价值观 ────────────────────────┐   │
│  │ 政治立场: XXXX                    │   │
│  │ 人生哲学: XXXXXXXX                │   │
│  │ 核心价值观: XX、XX、XX            │   │
│  └──────────────────────────────────┘   │
│                                         │
│  [导出 OpenClaw]  [关闭]                │
│                                         │
│  ⚠️ 基于微博内容的AI推测分析，仅供参考    │
└─────────────────────────────────────────┘
```

### 6.2 前端组件结构

```
ProfilePanel
├── PersonalInfoCard      (个人画像卡片)
├── StatsCharts           (统计图表)
│   ├── TimeDistributionChart (时间分布柱状图)
│   ├── PostingFrequencyChart (发博频率折线图)
│   └── WordCloud            (词云)
├── ContentAnalysisCard   (内容分析)
├── InterestCard          (兴趣偏好)
├── ValuesCard            (价值观)
├── SentimentCard         (情感分析)
└── Disclaimer            (免责声明)
```

### 6.3 图表库选择

| 选项 | 体积 | 功能 | 推荐 |
|------|------|------|------|
| Chart.js | ~60KB | 基础图表，够用 | ✅ MVP 推荐 |
| ECharts | ~800KB | 功能全面，词云支持好 | 后续迭代 |
| Recharts | ~200KB | React 原生 | 备选 |

**MVP 阶段建议用 Chart.js**，后续如需词云再切换 ECharts。

---

## 7. OpenClaw 导出

### 7.1 identity.md

基于分析结果生成，遵循 OpenClaw 格式：

```markdown
---
name: {screen_name}
id: {uid}
description: {个人简介推断}
---

## 基础信息
- 微博总数: {total_posts} 条
- 原创微博: {total_original} 条
- 转发微博: {total_reposts} 条
- 平均字数: {avg_length} 字/条
- 发博频率: {posts_per_day} 条/天
- 活跃时段: {active_hours}

## 写作风格
- 语言特点: {communication_style}
- 常用表情: {top_emojis}
- 表达习惯: {expression_habits}

## 内容特征
- 高频词汇: {top_words}
- 话题标签: {top_hashtags}
- 常用@: {top_mentions}

## 个人画像
- MBTI: {mbti_type}
- 性格特征: {personality_traits}
- 可能职业: {possible_occupation}
- 可能城市: {possible_cities}

## 兴趣偏好
- 音乐: {music}
- 书籍: {books}
- 电影: {movies}
- 爱好: {hobbies}
```

### 7.2 soul.md

```markdown
---
name: {screen_name}
type: blogger_profile
---

## 价值观倾向
- 政治观点: {political_stance}
- 核心价值观: {core_values}
- 社会关注: {common_topics}

## 情绪特征
- 整体基调: {overall_sentiment}
- 情绪稳定性: {emotional_stability}
- 触发话题: {emotional_triggers}

## 语言模式
- 句式特点: {writing_style}
- 表达方式: {expression_style}
- 常见修辞: {communication_style}

## 人生哲学
- 生活态度: {attitude_toward_life}
- 世界观: {worldview}
- 人生理念: {philosophy}
```

---

## 8. 新增依赖

### Rust (Cargo.toml)

```toml
[dependencies]
# 中文分词
jieba-rs = "0.7"
# 已有: reqwest (HTTP client), serde, serde_json, chrono
```

### 前端 (package.json)

```json
{
  "dependencies": {
    "chart.js": "^4.4",
    "react-chartjs-2": "^5.2"
  }
}
```

---

## 9. 实现计划

### Phase 1: 基础架构 + 本地分析（Task 1-3）

**Task 1**: Rust 数据模型 + ProfileService 骨架
- 创建 `services/profile/` 模块
- 定义 ProfileResult、LocalStats、DeepProfile 等数据模型
- 创建 ProfileService 编排器
- 注册 Tauri commands
- 更新前端 contracts.ts

**Task 2**: LocalAnalyzer 本地统计分析
- 实现 BasicStats 计算（频率、时间分布、字数）
- 实现 ContentAnalysis（词频 via jieba-rs、标签、@提及）
- 实现 InfluenceMetrics（来源分布、地区分布）
- 单元测试

**Task 3**: 前端 ProfilePanel 组件 + 分析按钮
- StepProcessing 添加"画像分析"按钮
- 创建 ProfilePanel 组件
- 添加 chart.js 依赖
- 基础 UI 布局（不含 AI 分析部分）

### Phase 2: AI 分析集成（Task 4-5）

**Task 4**: GLMClient + 数据采样
- 实现数据抽样逻辑
- 实现 GLM API 调用客户端
- 实现 prompt 构建
- JSON 响应解析

**Task 5**: AI 分析前端展示
- ProfilePanel 集成 AI 分析结果
- 个人画像卡片
- 兴趣偏好卡片
- 价值观卡片
- 免责声明

### Phase 3: OpenClaw 导出 + 收尾（Task 6-7）

**Task 6**: OpenClaw 导出
- 实现 export_openclaw.rs
- identity.md 模板渲染
- soul.md 模板渲染
- 前端导出按钮

**Task 7**: 集成测试 + 优化
- 端到端测试（分析流程）
- 错误处理（API 超时、网络异常）
- 加载状态和进度展示
- 性能优化（大量微博场景）

---

## 10. ⚠️ 待确认问题清单

在开始实施前，需要用户确认以下问题：

| # | 问题 | 影响 | 选项 |
|---|------|------|------|
| 1 | **API Key 管理方案** | 架构核心 | A. 内嵌 / B. 用户自带 / C. 后端代理 |
| 2 | **WeiboPost 模型扩展** | 数据完整性 | 是否需要扩展互动数据（reposts/comments/likes）？ |
| 3 | **分词库选择** | 二进制体积 | jieba-rs (精确但大) vs 简单统计 (小但粗糙) |
| 4 | **图表库选择** | 前端体积 | Chart.js (轻量) vs ECharts (功能全) |
| 5 | **免责声明措辞** | 法务风险 | 需要确认 AI 推测分析的免责文本 |
| 6 | **分阶段交付** | 工期 | 是否接受先 MVP（本地统计+基础 AI）再迭代深度功能？ |
