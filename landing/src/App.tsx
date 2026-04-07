import { Download, FileText, Zap, ShieldCheck, Github } from 'lucide-react';

export default function App() {
  return (
    <div className="min-h-screen selection:bg-indigo-100 selection:text-primary">
      {/* Navbar */}
      <nav className="fixed top-0 w-full z-50 bg-white/80 backdrop-blur-md border-b border-slate-200">
        <div className="max-w-7xl mx-auto px-4 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center text-white font-bold">微</div>
            <span className="font-bold text-xl text-slate-800">微光足迹</span>
          </div>
          <div className="hidden md:flex items-center gap-8 text-slate-600 font-medium">
            <a href="#features" className="hover:text-primary transition-colors">功能特点</a>
            <a href="#security" className="hover:text-primary transition-colors">安全隐私</a>
            <a href="#download" className="hover:text-primary transition-colors">下载地址</a>
          </div>
          <a href="#download" className="bg-primary text-white px-6 py-2 rounded-full font-semibold hover:bg-primary-hover shadow-lg shadow-indigo-200 transition-all active:scale-95">
            立即下载
          </a>
        </div>
      </nav>

      <main>
        {/* Hero Section */}
        <section className="pt-32 pb-20 lg:pt-48 lg:pb-32 px-4 overflow-hidden">
          <div className="max-w-7xl mx-auto grid lg:grid-cols-2 gap-12 items-center text-left">
            <div>
              <h1 className="text-5xl lg:text-6xl font-extrabold text-slate-900 leading-tight mb-6">
                留住每一份足迹<br />
                <span className="text-primary">备份微博回忆</span>
              </h1>
              <p className="text-xl text-slate-600 mb-10 leading-relaxed">
                高清、完整、私密。一键将你的微博导出为精美的 Markdown 或网页 HTML，长文自动展开，回忆永不丢失。
              </p>
              <div className="flex flex-wrap gap-4">
                <a href="#download" className="flex items-center gap-2 bg-primary text-white px-8 py-4 rounded-2xl font-bold text-lg hover:bg-primary-hover shadow-xl shadow-indigo-100 transition-all">
                  <Download size={20} />
                  下载 Windows 版 (v1.0.0)
                </a>
                <a href="#download" className="flex items-center gap-2 bg-slate-100 text-slate-700 px-8 py-4 rounded-2xl font-bold text-lg hover:bg-slate-200 transition-all">
                  其他平台
                </a>
              </div>
            </div>
            <div className="relative">
              <div className="absolute -inset-4 bg-gradient-to-tr from-indigo-500/10 to-transparent blur-3xl -z-10 rounded-full"></div>
              <div className="bg-white p-2 rounded-2xl shadow-2xl border border-slate-100 rotate-1 lg:rotate-2 hover:rotate-0 transition-transform duration-500">
                <div className="aspect-[16/10] bg-slate-100 rounded-xl flex items-center justify-center overflow-hidden">
                   <img src="/screenshot.png" alt="微光足迹 软件截图" className="w-full h-full object-cover" />
                </div>
              </div>
            </div>
          </div>
        </section>

        <section id="features" className="py-24 bg-white">
          <div className="max-w-7xl mx-auto px-4">
            <div className="text-center mb-16">
              <h2 className="text-3xl font-bold text-slate-900 mb-4">核心亮点</h2>
              <p className="text-slate-500 text-lg">专为微博回忆备份打造的极致体验</p>
            </div>
            <div className="grid md:grid-cols-3 gap-12">
              <div className="group p-8 rounded-3xl border border-slate-100 bg-slate-50/50 hover:bg-white hover:shadow-xl transition-all duration-300">
                <div className="w-14 h-14 bg-blue-100 text-blue-600 rounded-2xl flex items-center justify-center mb-6 group-hover:scale-110 transition-transform">
                  <Download size={28} />
                </div>
                <h3 className="text-xl font-bold text-slate-800 mb-4">高清完整备份</h3>
                <p className="text-slate-600 leading-relaxed">完美保留高清大图、动态表情、微博超话、甚至转发链条。不错过任何一个精彩瞬间。</p>
              </div>
              <div className="group p-8 rounded-3xl border border-slate-100 bg-slate-50/50 hover:bg-white hover:shadow-xl transition-all duration-300">
                <div className="w-14 h-14 bg-indigo-100 text-indigo-600 rounded-2xl flex items-center justify-center mb-6 group-hover:scale-110 transition-transform">
                  <FileText size={28} />
                </div>
                <h3 className="text-xl font-bold text-slate-800 mb-4">多格式沉浸阅读</h3>
                <p className="text-slate-600 leading-relaxed">支持导出为 Markdown、HTML、JSON。精美排版，让阅读回忆像读电子书一样享受。</p>
              </div>
              <div className="group p-8 rounded-3xl border border-slate-100 bg-slate-50/50 hover:bg-white hover:shadow-xl transition-all duration-300">
                <div className="w-14 h-14 bg-amber-100 text-amber-600 rounded-2xl flex items-center justify-center mb-6 group-hover:scale-110 transition-transform">
                  <Zap size={28} />
                </div>
                <h3 className="text-xl font-bold text-slate-800 mb-4">长文自动展开</h3>
                <p className="text-slate-600 leading-relaxed">智能识别并抓取微博全文。告别“展开全文”，让每一段感悟都完整留存。</p>
              </div>
            </div>
          </div>
        </section>

        <section id="security" className="py-24 px-4">
          <div className="max-w-4xl mx-auto bg-primary rounded-[3rem] p-12 lg:p-16 text-white text-center shadow-2xl shadow-indigo-200">
            <ShieldCheck size={64} className="mx-auto mb-8 opacity-90" />
            <h2 className="text-3xl font-bold mb-6">数据属于你，我们也尊重你的隐私</h2>
            <div className="grid sm:grid-cols-3 gap-8 text-indigo-50 font-medium">
              <div>
                <div className="text-2xl font-bold mb-1">100%</div>
                <div>本地运行，安全无忧</div>
              </div>
              <div>
                <div className="text-2xl font-bold mb-1">私密</div>
                <div>不上传任何个人数据</div>
              </div>
              <div>
                <div className="text-2xl font-bold mb-1">开源</div>
                <div>GitHub 代码透明可证</div>
              </div>
            </div>
          </div>
        </section>

        <section id="download" className="py-24 bg-slate-50">
          <div className="max-w-7xl mx-auto px-4 text-center">
            <h2 className="text-3xl font-bold text-slate-900 mb-12">选择你的平台</h2>
            <div className="flex flex-wrap justify-center gap-6">
              <div className="bg-white p-8 rounded-3xl border border-slate-200 w-64 hover:border-primary transition-colors cursor-pointer group">
                <div className="text-slate-400 group-hover:text-primary mb-4 transition-colors font-bold text-sm">Windows</div>
                <div className="font-bold text-xl mb-6">.exe 安装包</div>
                <button className="w-full bg-slate-900 text-white py-3 rounded-xl font-bold hover:bg-slate-800 transition-colors">下载</button>
              </div>
              <div className="bg-white p-8 rounded-3xl border border-slate-200 w-64 hover:border-primary transition-colors cursor-pointer group">
                <div className="text-slate-400 group-hover:text-primary mb-4 transition-colors font-bold text-sm">macOS</div>
                <div className="font-bold text-xl mb-6">.dmg 安装包</div>
                <button className="w-full bg-slate-100 text-slate-800 py-3 rounded-xl font-bold hover:bg-slate-200 transition-colors">下载</button>
              </div>
              <div className="bg-white p-8 rounded-3xl border border-slate-200 w-64 hover:border-primary transition-colors cursor-pointer group">
                <div className="text-slate-400 group-hover:text-primary mb-4 transition-colors font-bold text-sm">Linux</div>
                <div className="font-bold text-xl mb-6">.deb / .rpm</div>
                <button className="w-full bg-slate-100 text-slate-800 py-3 rounded-xl font-bold hover:bg-slate-200 transition-colors">下载</button>
              </div>
            </div>
          </div>
        </section>
      </main>

      <footer className="py-12 border-t border-slate-200">
        <div className="max-w-7xl mx-auto px-4 flex flex-col md:flex-row justify-between items-center gap-8">
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 bg-primary rounded flex items-center justify-center text-white font-bold text-xs">微</div>
            <span className="font-bold text-slate-800">微光足迹</span>
          </div>
          <p className="text-slate-400 text-sm">© 2026 微光足迹. Made with ❤️ for your memories.</p>
          <div className="flex items-center gap-6 text-slate-400">
            <a href="#" className="hover:text-slate-600 transition-colors"><Github size={20} /></a>
          </div>
        </div>
      </footer>
    </div>
  )
}
