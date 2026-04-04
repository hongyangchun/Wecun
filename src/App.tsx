import { useState, useEffect, useRef } from "react";
import StepIndicator from "./components/StepIndicator";
import StepLogin from "./components/StepLogin";
import StepTarget from "./components/StepTarget";
import StepOptions, { PostFilter, DateMode, ExportFormat } from "./components/StepOptions";
import StepPath from "./components/StepPath";
import StepDownload from "./components/StepDownload";
import { startDownload, cancelDownload, onProgress, onCookieReceived, openOutputDir } from "./lib/tauri-bridge";
import { buildDownloadRequest, extractUidFromUrl } from "./lib/validation";
import type { ProgressPhase } from "./types/contracts";
import "./App.css";

const STEPS = ["登录", "目标", "选项", "保存", "下载"];

const PHASE_TEXT: Record<ProgressPhase, string> = {
  FetchingUserInfo: "正在获取用户信息",
  FetchingPostList: "正在获取微博列表",
  FetchingLongText: "正在获取长文内容",
  DownloadingImages: "正在下载图片",
  Exporting: "正在导出文件",
  Complete: "下载完成",
  Error: "出错了",
  Cancelled: "已取消",
};

interface PersistedSettings {
  postFilter: PostFilter;
  includeImages: boolean;
  dateMode: DateMode;
  exportFormat: ExportFormat;
  minTextLength: number;
}

function loadSettings(): PersistedSettings {
  try {
    const raw = localStorage.getItem("weibo-dl-settings");
    if (raw) return JSON.parse(raw);
  } catch {
    void 0;
  }
  return {
    postFilter: "original",
    includeImages: true,
    dateMode: "all",
    exportFormat: "md-single",
    minTextLength: 0,
  };
}

function saveSettings(s: PersistedSettings) {
  try {
    localStorage.setItem("weibo-dl-settings", JSON.stringify(s));
  } catch {
    void 0;
  }
}

function App() {
  const saved = useRef(loadSettings());
  const [step, setStep] = useState(0);
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [cookie, setCookie] = useState("");
  const [profileUrl, setProfileUrl] = useState("");
  const [postFilter, setPostFilter] = useState<PostFilter>(saved.current.postFilter);
  const [includeImages, setIncludeImages] = useState(saved.current.includeImages);
  const [dateMode, setDateMode] = useState<DateMode>(saved.current.dateMode);
  const [dateStart, setDateStart] = useState("");
  const [dateEnd, setDateEnd] = useState("");
  const [exportFormat, setExportFormat] = useState<ExportFormat>(saved.current.exportFormat);
  const [minTextLength, setMinTextLength] = useState(saved.current.minTextLength);
  const [outputDir, setOutputDir] = useState("");

  const [dlStatus, setDlStatus] = useState<"downloading" | "done" | "cancelled" | "error">("downloading");
  const [progress, setProgress] = useState(0);
  const [phase, setPhase] = useState("");
  const [current, setCurrent] = useState(0);
  const [total, setTotal] = useState(0);
  const [errorMsg, setErrorMsg] = useState<string | undefined>();
  const [logs, setLogs] = useState<string[]>([]);
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    let cancelled = false;
    onProgress((e) => {
      if (cancelled) return;
      setCurrent(e.current);
      setTotal(e.total);
      setLogs((p) => [...p, e.message]);

      if (e.phase === "FetchingPostList") {
        setProgress(0);
        setPhase(`${e.message}`);
      } else if (e.phase === "Complete") {
        setProgress(100);
        setPhase(PHASE_TEXT[e.phase]);
        setDlStatus("done");
      } else if (e.phase === "Cancelled") {
        setDlStatus("cancelled");
        setPhase(PHASE_TEXT[e.phase]);
      } else if (e.phase === "Error") {
        setDlStatus("error");
        setErrorMsg(e.message);
        setPhase(PHASE_TEXT[e.phase]);
      } else {
        const pct = e.total > 0 ? Math.round((e.current / e.total) * 100) : 0;
        setProgress(pct);
        setPhase(PHASE_TEXT[e.phase] || e.message);
      }
    }).then((fn) => { if (!cancelled) unlistenRef.current = fn; });
    return () => { cancelled = true; unlistenRef.current?.(); };
  }, []);

  useEffect(() => {
    const fn = onCookieReceived((c) => {
      setCookie(c);
      setIsLoggedIn(true);
      setStep(1);
    });
    return () => { fn.then((u) => u()); };
  }, []);

  useEffect(() => {
    saveSettings({ postFilter, includeImages, dateMode, exportFormat, minTextLength });
  }, [postFilter, includeImages, dateMode, exportFormat, minTextLength]);

  const handleStart = async () => {
    setStep(4);
    setDlStatus("downloading");
    setProgress(0);
    setPhase("正在获取用户信息");
    setCurrent(0);
    setTotal(0);
    setErrorMsg(undefined);
    setLogs(["开始下载..."]);

    try {
      const uid = extractUidFromUrl(profileUrl);
      if (!uid) {
        setDlStatus("error");
        setErrorMsg("无法从链接中提取用户ID");
        return;
      }

      const result = await startDownload(buildDownloadRequest({
        uid, cookie, filter: postFilter, includeImages,
        dateMode, dateStart, dateEnd, exportFormat, outputDir,
        minTextLength,
      }));
      setLogs((p) => [...p, result]);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes("已取消")) {
        setDlStatus("cancelled");
      } else {
        setDlStatus("error");
        setErrorMsg(msg);
      }
    }
  };

  const handleStop = async () => {
    await cancelDownload();
    setLogs((p) => [...p, "正在取消..."]);
  };

  const handleReset = () => {
    setStep(0);
    setDlStatus("downloading");
    setProgress(0);
    setPhase("");
    setCurrent(0);
    setTotal(0);
    setErrorMsg(undefined);
    setLogs([]);
  };

  const handleOpenOutputDir = async () => {
    if (outputDir) {
      await openOutputDir(outputDir);
    }
  };

  return (
    <div className="app-root">
      <header className="app-header">
        <h1>微博下载器</h1>
      </header>

      <div className="wizard-container">
        <div className="wizard-card">
          <StepIndicator steps={STEPS} currentStep={step} />

          {step === 0 && <StepLogin isLoggedIn={isLoggedIn} />}
          {step === 1 && <StepTarget profileUrl={profileUrl} onProfileUrlChange={setProfileUrl} onNext={() => setStep(2)} />}
          {step === 2 && (
            <StepOptions
              postFilter={postFilter} onPostFilterChange={setPostFilter}
              includeImages={includeImages} onIncludeImagesChange={setIncludeImages}
              dateMode={dateMode} onDateModeChange={setDateMode}
              dateStart={dateStart} onDateStartChange={setDateStart}
              dateEnd={dateEnd} onDateEndChange={setDateEnd}
              exportFormat={exportFormat} onExportFormatChange={setExportFormat}
              minTextLength={minTextLength} onMinTextLengthChange={setMinTextLength}
              onBack={() => setStep(1)} onNext={() => setStep(3)}
            />
          )}
          {step === 3 && (
            <StepPath
              outputDir={outputDir} onDirChange={setOutputDir}
              onBack={() => setStep(2)} onStart={handleStart}
            />
          )}
          {step === 4 && (
            <StepDownload
              phase={phase} progress={progress}
              current={current} total={total}
              status={dlStatus} errorMessage={errorMsg} logs={logs}
              onStop={handleStop} onReset={handleReset} onOpenOutputDir={handleOpenOutputDir}
            />
          )}
        </div>
      </div>
    </div>
  );
}

export default App;
