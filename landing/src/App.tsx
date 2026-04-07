import { Download } from 'lucide-react';

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
        
        {/* Placeholder for future tasks */}
      </main>
    </div>
  )
}
