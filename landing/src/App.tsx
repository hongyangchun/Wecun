export default function App() {
  return (
    <div className="min-h-screen relative overflow-hidden">
      <div className="absolute top-[-20%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-[var(--color-accent)]/10 rounded-full blur-[120px] -z-10 pointer-events-none"></div>
      <main className="max-w-6xl mx-auto px-6 relative z-10 pt-32">
        <h1 className="text-white text-4xl font-bold glow-text">Testing Theme</h1>
      </main>
    </div>
  )
}
