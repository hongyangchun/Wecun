import { useState, useCallback } from "react";
import type { DownloadStatus } from "../types/contracts";

export function useDownloadRun() {
  const [isRunning, setIsRunning] = useState(false);
  const [isDone, setIsDone] = useState(false);
  const [status, setStatus] = useState<DownloadStatus>("ready");
  const [progress, setProgress] = useState(0);
  const [phase, setPhase] = useState("");
  const [current, setCurrent] = useState(0);
  const [total, setTotal] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | undefined>();
  const [logs, setLogs] = useState<string[]>([]);

  const start = useCallback(() => {
    setIsRunning(true);
    setIsDone(false);
    setStatus("downloading");
    setProgress(0);
    setPhase("正在获取用户信息...");
    setCurrent(0);
    setTotal(0);
    setErrorMessage(undefined);
    setLogs([]);
  }, []);

  const stop = useCallback(() => {
    setIsRunning(false);
    setIsDone(true);
    setStatus("cancelled");
    setPhase("已取消");
  }, []);

  const complete = useCallback(() => {
    setIsRunning(false);
    setIsDone(true);
    setStatus("done");
    setProgress(100);
    setPhase("下载完成");
  }, []);

  const fail = useCallback((msg: string) => {
    setIsRunning(false);
    setIsDone(true);
    setStatus("error");
    setErrorMessage(msg);
    setPhase("出错了");
  }, []);

  const addLog = useCallback((log: string) => {
    setLogs((prev) => [...prev, log]);
  }, []);

  return {
    isRunning, isDone, status, progress, phase, current, total,
    errorMessage, logs,
    start, stop, complete, fail, addLog,
    setProgress, setPhase, setCurrent, setTotal,
  };
}
