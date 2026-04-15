import { Github, Download, ArrowRight, FileText, ImageIcon, Shield, Monitor, Globe } from 'lucide-react';

const PLATFORMS = [
  {
    icon: Monitor,
    name: 'macOS',
    desc: 'Apple Silicon & Intel',
    format: '.dmg',
    arch: 'Universal',
  },
  {
    icon: Monitor,
    name: 'Windows',
    desc: 'x64',
    format: '.exe',
    arch: 'x64',
  },
  {
    icon: Globe,
    name: 'Linux',
    desc: 'Ubuntu / Debian',
    format: '.AppImage / .deb',
    arch: 'x64',
  },
];

const SCREENSHOTS = [
  {
    src: '/screenshots/screenshot-1.png',
    alt: '微博导入',
  },
  {
    src: '/screenshots/screenshot-2.png',
    alt: '导出选项',
  },
  {
    src: '/screenshots/screenshot-3.png',
    alt: '导出进度',
  },
  {
    src: '/screenshots/screenshot-4.png',
    alt: '导出结果',
  },
];

const FEATURES = [
  {
    icon: Download,
    title: '完整下载',
    desc: '抓取任意博主的原创微博或全部微博，支持指定时间范围，自动处理长微博全文。',
  },
  {
    icon: ImageIcon,
    title: '高清图片',
    desc: '保留原图质量，拒绝压缩。图片单独存放于 images/ 目录，方便管理。',
  },
  {
    icon: FileText,
    title: '多格式导出',
    desc: 'Markdown（单文件）、Markdown（Obsidian 兼容）、HTML，单文件导出便于分享。',
  },
  {
    icon: Shield,
    title: '绝对隐私',
    desc: '纯本地运行，不上传任何数据。无需服务器，开源可验证。',
  },
];

const STEPS = [
  {
    num: '01',
    title: '下载安装',
    desc: '选择你的系统，下载对应安装包',
  },
  {
    num: '02',
    title: '登录微博',
    desc: '在应用内安全登录你的微博账号',
  },
  {
    num: '03',
    title: '下载导出',
    desc: '选择博主或收藏，设置选项，一键导出',
  },
];

export default function App() {
  return (
    <div className="min-h-screen">
      {/* Header */}
      <header className="fixed top-0 left-0 right-0 z-50 border-b border-[var(--color-border)] bg-[var(--color-bg)]/80 backdrop-blur-md">
        <div className="max-w-5xl mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <img src="/app-icon.png" alt="微存" className="w-8 h-8" />
            <span className="font-bold text-lg">微存</span>
            <span className="text-[var(--color-text-muted)] text-sm ml-1">Wecun</span>
          </div>
          <nav className="flex items-center gap-6">
            <a
              href="#features"
              className="text-sm text-[var(--color-text-muted)] hover:text-[var(--color-text)] transition-colors"
            >
              功能
            </a>
            <a
              href="#how"
              className="text-sm text-[var(--color-text-muted)] hover:text-[var(--color-text)] transition-colors"
            >
              使用方法
            </a>
            <a
              href="#download"
              className="text-sm font-medium text-[var(--color-accent)] hover:text-[#34e9bf] transition-colors"
            >
              下载
            </a>
            <a
              href="https://github.com/hongyangchun/Wecun"
              target="_blank"
              rel="noopener noreferrer"
              className="text-[var(--color-text-muted)] hover:text-[var(--color-text)] transition-colors"
            >
              <Github size={20} />
            </a>
          </nav>
        </div>
      </header>

      {/* Hero */}
      <section className="pt-40 pb-32 px-6">
        <div className="max-w-5xl mx-auto">
          <div className="max-w-3xl animate-fade-up">
            <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)] text-sm text-[var(--color-text-muted)] mb-8">
              <span className="w-2 h-2 rounded-full bg-[var(--color-accent)] pulse-dot"></span>
              <span className="mono text-xs">v1.0.0 现已可用</span>
            </div>

            <h1 className="text-5xl md:text-6xl lg:text-7xl font-bold tracking-tight mb-6 leading-[1.1]">
              纯粹的
              <br />
              <span className="text-[var(--color-accent)]">微博备份</span>工具
            </h1>

            <p className="text-xl text-[var(--color-text-muted)] leading-relaxed mb-12 max-w-2xl">
              本地运行，高清图片，多格式导出。保留你的数字记忆，不依赖任何服务器。
            </p>

            <div className="flex flex-wrap gap-4">
              <a href="#download" className="btn-primary text-base">
                <Download size={18} />
                立即下载
              </a>
              <a href="#how" className="btn-secondary text-base">
                了解使用方法
                <ArrowRight size={18} />
              </a>
            </div>
          </div>

          {/* Terminal Preview */}
          <div className="mt-20 animate-fade-up animate-delay-200">
            <div className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] overflow-hidden shadow-2xl shadow-black/50">
              {/* Window Controls */}
              <div className="flex items-center gap-2 px-4 py-3 border-b border-[var(--color-border)] bg-[var(--color-bg)]">
                <div className="w-3 h-3 rounded-full bg-[#ff5f57]"></div>
                <div className="w-3 h-3 rounded-full bg-[#febc2e]"></div>
                <div className="w-3 h-3 rounded-full bg-[#28c840]"></div>
                <span className="ml-4 mono text-xs text-[var(--color-text-muted)]">微存 — 微博备份</span>
              </div>
              {/* Content */}
              <div className="p-6 font-mono text-sm leading-relaxed">
                <div className="text-[var(--color-text-muted)]">
                  <span className="text-[var(--color-accent)]">$</span> wecun download --user 2166767661
                </div>
                <div className="mt-4 space-y-1">
                  <div>
                    <span className="text-[var(--color-accent)]">✓</span>{' '}
                    <span className="text-[var(--color-text)]">已连接微博账号</span>
                  </div>
                  <div>
                    <span className="text-[var(--color-accent)]">✓</span>{' '}
                    <span className="text-[var(--color-text)]">正在获取 @微博用户 的微博...</span>
                  </div>
                  <div>
                    <span className="text-[var(--color-accent)]">→</span>{' '}
                    <span className="text-[var(--color-text)]">已下载 1,234 条微博</span>
                  </div>
                  <div>
                    <span className="text-[var(--color-accent)]">→</span>{' '}
                    <span className="text-[var(--color-text)]">下载图片 567 张 (1.2 GB)</span>
                  </div>
                  <div className="mt-4 pt-4 border-t border-[var(--color-border)]">
                    <span className="text-[#28c840]">✓ 导出完成</span>
                    <span className="text-[var(--color-text-muted)] ml-4">→ ~/Documents/wecun/</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Features */}
      <section id="features" className="py-32 px-6 border-t border-[var(--color-border)]">
        <div className="max-w-5xl mx-auto">
          <div className="max-w-xl mb-16 animate-fade-up">
            <h2 className="text-3xl md:text-4xl font-bold mb-4">核心功能</h2>
            <p className="text-[var(--color-text-muted)] text-lg">
              每一个功能都经过精心设计，只为提供最佳的使用体验。
            </p>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            {FEATURES.map((feature, i) => (
              <div
                key={feature.title}
                className={`card animate-fade-up animate-delay-${(i + 1) * 100}`}
              >
                <div className="w-12 h-12 rounded-lg bg-[var(--color-accent-dim)] flex items-center justify-center mb-6">
                  <feature.icon size={24} className="text-[var(--color-accent)]" />
                </div>
                <h3 className="text-xl font-semibold mb-3">{feature.title}</h3>
                <p className="text-[var(--color-text-muted)] leading-relaxed">
                  {feature.desc}
                </p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Screenshots */}
      <section className="py-32 px-6 bg-[var(--color-surface)] border-y border-[var(--color-border)]">
        <div className="max-w-6xl mx-auto">
          <div className="max-w-xl mb-16 animate-fade-up">
            <h2 className="text-3xl md:text-4xl font-bold mb-4">界面预览</h2>
            <p className="text-[var(--color-text-muted)] text-lg">
              简洁直观的界面设计，轻松上手。
            </p>
          </div>

          <div className="grid grid-cols-2 gap-6">
            {SCREENSHOTS.map((screenshot, i) => (
              <div
                key={screenshot.src}
                className={`animate-fade-up animate-delay-${(i + 1) * 100}`}
              >
                <div className="rounded-xl border border-[var(--color-border)] bg-[var(--color-bg)] overflow-hidden shadow-lg shadow-black/30 hover:shadow-xl hover:shadow-black/40 hover:-translate-y-1 transition-all duration-300">
                  <img
                    src={screenshot.src}
                    alt={screenshot.alt}
                    className="w-full h-auto"
                    loading="lazy"
                  />
                </div>
                <div className="mt-3 text-center text-sm text-[var(--color-text-muted)]">
                  {screenshot.alt}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* How to Use */}
      <section id="how" className="py-32 px-6 bg-[var(--color-surface)] border-y border-[var(--color-border)]">
        <div className="max-w-5xl mx-auto">
          <div className="max-w-xl mb-16">
            <h2 className="text-3xl md:text-4xl font-bold mb-4">如何使用</h2>
            <p className="text-[var(--color-text-muted)] text-lg">
              三步完成微博备份，简单直接。
            </p>
          </div>

          <div className="grid md:grid-cols-3 gap-8">
            {STEPS.map((step, i) => (
              <div key={step.num} className="relative animate-fade-up animate-delay-100">
                <div className="text-6xl font-bold text-[var(--color-border)] mono mb-4">
                  {step.num}
                </div>
                <h3 className="text-xl font-semibold mb-2">{step.title}</h3>
                <p className="text-[var(--color-text-muted)]">{step.desc}</p>
                {i < STEPS.length - 1 && (
                  <div className="hidden md:block absolute top-8 right-0 translate-x-1/2 text-[var(--color-border)]">
                    <ArrowRight size={24} />
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Export Formats */}
      <section className="py-32 px-6">
        <div className="max-w-5xl mx-auto">
          <div className="grid lg:grid-cols-2 gap-16 items-center">
            <div className="animate-fade-up">
              <h2 className="text-3xl md:text-4xl font-bold mb-4">灵活导出格式</h2>
              <p className="text-[var(--color-text-muted)] text-lg mb-8">
                选择最适合你的导出方式，Markdown 兼容 Obsidian，HTML 可直接浏览器打开。
              </p>
              <div className="space-y-4">
                <div className="flex items-center gap-4">
                  <div className="w-10 h-10 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] flex items-center justify-center mono text-sm font-bold">
                    .md
                  </div>
                  <div>
                    <div className="font-medium">Markdown 单文件</div>
                    <div className="text-sm text-[var(--color-text-muted)]">所有微博合并为一个文件</div>
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <div className="w-10 h-10 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] flex items-center justify-center mono text-sm font-bold">
                    .md
                  </div>
                  <div>
                    <div className="font-medium">Obsidian 格式</div>
                    <div className="text-sm text-[var(--color-text-muted)]">每条微博一个文件，自动生成索引</div>
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <div className="w-10 h-10 rounded-lg bg-[var(--color-surface)] border border-[var(--color-border)] flex items-center justify-center mono text-sm font-bold">
                    .html
                  </div>
                  <div>
                    <div className="font-medium">HTML 单文件</div>
                    <div className="text-sm text-[var(--color-text-muted)]">精美排版，直接浏览器打开</div>
                  </div>
                </div>
              </div>
            </div>
            <div className="animate-fade-up animate-delay-200">
              <div className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] overflow-hidden">
                <div className="px-4 py-3 border-b border-[var(--color-border)] bg-[var(--color-bg)]">
                  <div className="flex items-center gap-2">
                    <div className="w-3 h-3 rounded-full bg-[var(--color-border)]"></div>
                    <span className="mono text-xs text-[var(--color-text-muted)]">preview.html</span>
                  </div>
                </div>
                <div className="p-6 space-y-4 text-sm">
                  <div className="text-[var(--color-text-muted)] text-xs">
                    2024年3月15日 14:32
                  </div>
                  <div className="text-[var(--color-text)] leading-relaxed">
                    今天分享一本最近在读的书，《原子习惯》。作者 James Clear
                    提出一个核心观点：you do not rise to the level of your goals, you fall to the level of your
                    systems。
                  </div>
                  <div className="text-[var(--color-text-muted)] text-xs">
                    来自 iPhone 客户端
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Download */}
      <section id="download" className="py-32 px-6 bg-[var(--color-surface)] border-t border-[var(--color-border)]">
        <div className="max-w-5xl mx-auto text-center">
          <h2 className="text-3xl md:text-4xl font-bold mb-4">开始使用</h2>
          <p className="text-[var(--color-text-muted)] text-lg mb-12 max-w-xl mx-auto">
            选择你的平台，下载并安装。完全免费，开源透明。
          </p>

          <div className="grid md:grid-cols-3 gap-6 max-w-3xl mx-auto mb-12">
            {PLATFORMS.map((platform) => (
              <div key={platform.name} className="card text-center group cursor-pointer">
                <platform.icon
                  size={32}
                  className="mx-auto mb-4 text-[var(--color-text-muted)] group-hover:text-[var(--color-accent)] transition-colors"
                />
                <div className="font-semibold text-lg mb-1">{platform.name}</div>
                <div className="text-sm text-[var(--color-text-muted)] mb-4">{platform.desc}</div>
                <button className="btn-primary w-full justify-center text-sm">
                  <Download size={16} />
                  {platform.format.split(' / ')[0]}
                </button>
              </div>
            ))}
          </div>

          <div className="flex flex-wrap justify-center gap-6 text-sm text-[var(--color-text-muted)]">
            <span>✓ 完全免费</span>
            <span>✓ 开源透明</span>
            <span>✓ 无需注册</span>
            <span>✓ 隐私安全</span>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="py-16 px-6 border-t border-[var(--color-border)]">
        <div className="max-w-5xl mx-auto flex flex-col md:flex-row items-center justify-between gap-6">
          <div className="flex items-center gap-3">
            <img src="/app-icon.png" alt="微存" className="w-8 h-8" />
            <span className="font-semibold">微存 Wecun</span>
          </div>
          <div className="flex items-center gap-8 text-sm text-[var(--color-text-muted)]">
            <a
              href="https://github.com/hongyangchun/Wecun"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-[var(--color-text)] transition-colors"
            >
              GitHub
            </a>
            <a
              href="https://github.com/hongyangchun/Wecun/releases"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-[var(--color-text)] transition-colors"
            >
              Releases
            </a>
          </div>
          <div className="text-sm text-[var(--color-text-muted)]">
            © 2024-{new Date().getFullYear()} Wecun. MIT License.
          </div>
        </div>
      </footer>
    </div>
  );
}
