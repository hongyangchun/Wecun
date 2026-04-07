import { Terminal, Download, ArrowRight, Github, FileJson, Image as ImageIcon, HardDrive, Lock } from 'lucide-react';

export default function App() {
  return (
    <div className="min-h-screen relative overflow-hidden font-sans">
      {/* Background Glow */}
      <div className="absolute top-[-10%] left-1/2 -translate-x-1/2 w-[800px] h-[400px] bg-[var(--color-accent)]/15 rounded-full blur-[120px] -z-10 pointer-events-none"></div>
      
      {/* Navbar */}
      <nav className="fixed top-0 w-full z-50 glass-panel border-x-0 border-t-0">
        <div className="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-3 font-bold text-lg tracking-tight">
            <div className="w-8 h-8 rounded-md bg-[var(--color-bg-elevated)] border border-[var(--color-border)] flex items-center justify-center">
              <Terminal size={16} className="text-[var(--color-accent)]" />
            </div>
            <span>微存</span>
          </div>
          <div className="flex items-center gap-6">
            <a href="https://github.com" className="text-[var(--color-text-muted)] hover:text-white transition-colors">
              <Github size={20} />
            </a>
            <a href="#download" className="px-4 py-1.5 text-sm font-medium rounded-md bg-white/5 hover:bg-white/10 border border-white/10 transition-all">
              获取应用
            </a>
          </div>
        </div>
      </nav>

      <main className="max-w-6xl mx-auto px-6 pt-40 pb-24 text-center relative z-10">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full border border-[var(--color-border-subtle)] bg-[var(--color-bg-elevated)]/50 text-[var(--color-text-muted)] text-xs font-mono mb-8">
          <span className="w-2 h-2 rounded-full bg-[var(--color-accent)] animate-pulse"></span>
          v1.0.0 is now available
        </div>
        
        <h1 className="text-6xl md:text-7xl font-extrabold tracking-tighter mb-8 glow-text">
          重塑微博备份体验
        </h1>
        
        <p className="text-xl text-[var(--color-text-muted)] max-w-2xl mx-auto mb-12 leading-relaxed">
          摒弃臃肿，回归纯粹。以极客的标准，完整抓取、智能解析、精美导出。将你的数字记忆安全封存在本地。
        </p>

        <div className="flex items-center justify-center gap-4">
          <a href="#download" className="group flex items-center gap-2 px-6 py-3 rounded-lg bg-[var(--color-accent)] text-[var(--color-bg-base)] font-bold hover:bg-[var(--color-accent-hover)] transition-all">
            <Download size={18} />
            下载 Windows 版
          </a>
          <a href="#features" className="group flex items-center gap-2 px-6 py-3 rounded-lg glass-panel text-[var(--color-text-main)] hover:bg-white/5 transition-all">
            探索功能
            <ArrowRight size={18} className="group-hover:translate-x-1 transition-transform" />
          </a>
        </div>
        
        <div id="features" className="mt-32">
          <div className="grid md:grid-cols-3 gap-6 text-left">
            
            {/* Bento 1: Large */}
            <div className="md:col-span-2 glass-panel p-8 rounded-2xl flex flex-col justify-between group hover:border-[var(--color-border)] transition-colors">
              <div className="mb-12">
                <div className="w-10 h-10 rounded-lg bg-[var(--color-bg-elevated)] border border-[var(--color-border-subtle)] flex items-center justify-center mb-4">
                  <ImageIcon size={20} className="text-[var(--color-accent)]" />
                </div>
                <h3 className="text-2xl font-bold mb-2">不妥协的高清与完整</h3>
                <p className="text-[var(--color-text-muted)] leading-relaxed">突破 API 限制，不仅抓取长文，更深度还原超话、表情和转发链条。原图质量保存，拒绝压缩渣画质。</p>
              </div>
              <div className="h-32 rounded-lg bg-[var(--color-bg-base)] border border-[var(--color-border-subtle)] p-4 font-mono text-sm text-[var(--color-text-muted)] flex flex-col gap-2 overflow-hidden relative">
                <div className="absolute inset-0 bg-gradient-to-t from-[var(--color-bg-base)] to-transparent z-10"></div>
                <div>[GET] /api/statuses/show?id=... <span className="text-green-400">200 OK</span></div>
                <div>Parsing retweeted_status... <span className="text-green-400">Done</span></div>
                <div>Downloading original_pic... <span className="text-[var(--color-accent)]">8.2MB</span></div>
                <div>Extracting full_text...</div>
              </div>
            </div>

            {/* Bento 2: Small */}
            <div className="glass-panel p-8 rounded-2xl flex flex-col group hover:border-[var(--color-border)] transition-colors">
              <div className="w-10 h-10 rounded-lg bg-[var(--color-bg-elevated)] border border-[var(--color-border-subtle)] flex items-center justify-center mb-4">
                <FileJson size={20} className="text-[var(--color-text-main)]" />
              </div>
              <h3 className="text-xl font-bold mb-2">沉浸式导出</h3>
              <p className="text-[var(--color-text-muted)] leading-relaxed mb-6">Markdown、HTML 完美排版。像阅读电子书一样回顾你的数字生活。</p>
              <div className="mt-auto flex gap-2">
                <span className="px-2 py-1 rounded bg-[var(--color-bg-elevated)] border border-[var(--color-border-subtle)] text-xs font-mono">.md</span>
                <span className="px-2 py-1 rounded bg-[var(--color-bg-elevated)] border border-[var(--color-border-subtle)] text-xs font-mono">.html</span>
                <span className="px-2 py-1 rounded bg-[var(--color-bg-elevated)] border border-[var(--color-border-subtle)] text-xs font-mono">.json</span>
              </div>
            </div>

            {/* Bento 3: Wide Security */}
            <div className="md:col-span-3 glass-panel p-8 rounded-2xl flex flex-col md:flex-row items-center gap-8 border-[var(--color-border)] relative overflow-hidden">
              <div className="absolute right-0 top-0 w-64 h-64 bg-[var(--color-accent)]/5 rounded-full blur-[80px]"></div>
              <div className="w-16 h-16 rounded-2xl bg-[var(--color-bg-elevated)] border border-[var(--color-border-subtle)] flex items-center justify-center shrink-0 z-10">
                <Lock size={28} className="text-emerald-400" />
              </div>
              <div className="flex-1 z-10">
                <h3 className="text-2xl font-bold mb-2">绝对隐私：你的数据只属于你</h3>
                <p className="text-[var(--color-text-muted)] max-w-2xl">
                  微存是一个 100% 纯本地运行的客户端。无需经过任何第三方服务器，不上传 Cookie，断网导出。代码完全开源，安全透明可验证。
                </p>
              </div>
            </div>

          </div>
        </div>

        <div id="download" className="mt-32 pt-24 border-t border-[var(--color-border-subtle)] text-left">
          <h2 className="text-3xl font-bold mb-12 text-center">选择平台，立即部署</h2>
          <div className="grid md:grid-cols-3 gap-6 max-w-4xl mx-auto">
            <div className="glass-panel p-6 rounded-xl hover:border-[var(--color-accent)]/50 transition-colors group cursor-pointer text-center">
              <HardDrive size={24} className="mx-auto mb-4 text-[var(--color-text-muted)] group-hover:text-[var(--color-accent)] transition-colors" />
              <div className="font-bold text-lg mb-1">Windows</div>
              <div className="text-sm text-[var(--color-text-muted)] font-mono mb-6">x64 / ARM64</div>
              <button className="w-full py-2 rounded-md bg-white/5 border border-white/10 group-hover:bg-[var(--color-accent)] group-hover:text-[var(--color-bg-base)] group-hover:border-transparent transition-all font-medium">
                下载 .exe
              </button>
            </div>
            <div className="glass-panel p-6 rounded-xl hover:border-white/50 transition-colors group cursor-pointer text-center">
              <HardDrive size={24} className="mx-auto mb-4 text-[var(--color-text-muted)] group-hover:text-white transition-colors" />
              <div className="font-bold text-lg mb-1">macOS</div>
              <div className="text-sm text-[var(--color-text-muted)] font-mono mb-6">Apple Silicon / Intel</div>
              <button className="w-full py-2 rounded-md bg-white/5 border border-white/10 group-hover:bg-white group-hover:text-[var(--color-bg-base)] transition-all font-medium">
                下载 .dmg
              </button>
            </div>
            <div className="glass-panel p-6 rounded-xl hover:border-white/50 transition-colors group cursor-pointer text-center">
              <Terminal size={24} className="mx-auto mb-4 text-[var(--color-text-muted)] group-hover:text-white transition-colors" />
              <div className="font-bold text-lg mb-1">Linux</div>
              <div className="text-sm text-[var(--color-text-muted)] font-mono mb-6">AppImage / DEB</div>
              <button className="w-full py-2 rounded-md bg-white/5 border border-white/10 group-hover:bg-white group-hover:text-[var(--color-bg-base)] transition-all font-medium">
                下载包
              </button>
            </div>
          </div>
        </div>

        <footer className="mt-32 pt-8 border-t border-[var(--color-border-subtle)] flex flex-col md:flex-row items-center justify-between gap-4 text-sm text-[var(--color-text-muted)]">
          <div className="flex items-center gap-2">
            <div className="w-5 h-5 rounded bg-[var(--color-bg-elevated)] border border-[var(--color-border)] flex items-center justify-center font-bold text-[10px] text-[var(--color-text-main)]">微</div>
            <span>© 2026 Wecun.</span>
          </div>
          <div>Built for privacy. Crafted with code.</div>
        </footer>
      </main>
    </div>
  )
}
