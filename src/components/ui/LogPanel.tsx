import { useEffect, useRef, useMemo } from "react";

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

  // 使用日志内容的 hash 作为稳定的 key，避免索引变化导致的 DOM 重排
  const visible = logs.slice(-maxVisible);
  const logKeys = useMemo(() => {
    const start = Math.max(0, logs.length - maxVisible);
    return logs.slice(start).map((log, i) => `${start + i}-${log.length}`);
  }, [logs, maxVisible]);

  return (
    <div ref={ref} className="log-panel">
      {visible.map((log, i) => (
        <div key={logKeys[i]} className="log-entry">{log}</div>
      ))}
    </div>
  );
}
