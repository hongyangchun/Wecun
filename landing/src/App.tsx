export default function App() {
  return (
    <div className="min-h-screen">
      <header className="fixed top-0 w-full z-50 bg-white/80 backdrop-blur-md border-b border-slate-200">
        <nav className="max-w-7xl mx-auto px-4 h-16 flex items-center justify-between">
          <div className="font-bold text-xl text-primary">微光足迹</div>
          <button className="bg-primary text-white px-4 py-2 rounded-full font-medium hover:bg-primary-hover transition-colors">
            立即下载
          </button>
        </nav>
      </header>
      <main className="pt-32">
        <section className="max-w-7xl mx-auto px-4 text-center">
          <h1 className="text-5xl font-bold tracking-tight text-slate-900">留住每一份足迹，备份微博回忆</h1>
        </section>
      </main>
    </div>
  )
}
