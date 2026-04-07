import { Terminal, Download, ArrowRight, Github } from 'lucide-react';

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
        
        {/* Placeholder for Bento grid */}
        <div id="features" className="mt-40"></div>
      </main>
    </div>
  )
}
