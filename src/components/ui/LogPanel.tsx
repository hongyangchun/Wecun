import { useEffect, useRef } from "react";

interface LogPanelProps {
  logs: string[];
  maxVisible?: number;
}

export default function LogPanel({ logs, maxVisible = 30 }: LogPanelProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (ref.current) {
      ref.current.scrollTop = ref.current.scrollHeight;
    }
  }, [logs]);

  const visible = logs.slice(-maxVisible);

  return (
    <div ref={ref} className="log-panel">
      {visible.map((log, i) => (
        <div key={i} className="log-entry">{log}</div>
      ))}
    </div>
  );
}
