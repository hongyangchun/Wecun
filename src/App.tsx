import { useCallback, useEffect, useState } from "react";
import StepIndicator from "./components/StepIndicator";
import StepLogin from "./components/StepLogin";
import StepTarget from "./components/StepTarget";
import StepOptions from "./components/StepOptions";
import StepExportSettings from "./components/StepExportSettings";
import StepProcessing from "./components/StepProcessing";
import { WizardProvider, useWizard } from "./state/wizard-context";
import {
  startDownload,
  cancelDownload,
  exportPosts,
  openOutputDir,
  clearSavedCookie,
  checkForAppUpdate,
  relaunchApp,
  type AppUpdate,
  type UpdateProgressEvent,
} from "./lib/tauri-bridge";
import { buildDownloadRequest, extractUidFromUrl, isValidProfileUrl } from "./lib/validation";
import type { ExportFormat } from "./types/contracts";
import "./App.css";

const STEPS = ["登录", "目标", "选项", "保存"];

type UpdateToastStatus = "hidden" | "available" | "downloading" | "installing" | "error";

interface UpdateToastState {
  status: UpdateToastStatus;
  version: string;
  notes: string;
  downloadedBytes: number;
  totalBytes: number;
  chunkCount: number;
  errorMessage: string;
}

const INITIAL_UPDATE_TOAST: UpdateToastState = {
  status: "hidden",
  version: "",
  notes: "",
  downloadedBytes: 0,
  totalBytes: 0,
  chunkCount: 0,
  errorMessage: "",
};

function formatLabel(fmt: ExportFormat): string {
  const labels: Record<ExportFormat, string> = {
    html: "HTML",
    "md-single": "Markdown",
    "md-multi": "Markdown（分文件）",
  };
  return labels[fmt];
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;

  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const rounded = value >= 100 ? value.toFixed(0) : value >= 10 ? value.toFixed(1) : value.toFixed(2);
  return `${rounded} ${units[unitIndex]}`;
}

function getUpdateProgressPercent(downloadedBytes: number, totalBytes: number): number {
  if (totalBytes <= 0) return 0;
  return Math.max(0, Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)));
}

function usePrefersReducedMotion() {
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined" || !("matchMedia" in window)) return;

    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const updatePreference = () => setPrefersReducedMotion(mediaQuery.matches);

    updatePreference();
    mediaQuery.addEventListener("change", updatePreference);

    return () => {
      mediaQuery.removeEventListener("change", updatePreference);
    };
  }, []);

  return prefersReducedMotion;
}

interface UpdateToastProps {
  state: UpdateToastState;
  prefersReducedMotion: boolean;
  onInstall: () => void;
  onDismiss: () => void;
}

function UpdateToast({ state, prefersReducedMotion, onInstall, onDismiss }: UpdateToastProps) {
  if (state.status === "hidden") return null;

  const progressPercent = getUpdateProgressPercent(state.downloadedBytes, state.totalBytes);
  const title = state.status === "available"
    ? `发现新版本 ${state.version}`
    : state.status === "downloading"
      ? "正在下载更新…"
      : state.status === "installing"
        ? "正在安装更新…"
        : "更新失败";

  const details = state.status === "available"
    ? state.notes || "新版本已可用，可立即下载并安装。"
    : state.status === "downloading"
      ? `${formatBytes(state.downloadedBytes)} / ${state.totalBytes > 0 ? formatBytes(state.totalBytes) : "未知大小"}`
      : state.status === "installing"
        ? "下载完成，正在应用更新…"
        : state.errorMessage;

  return (
    <div
      aria-live="polite"
      role="status"
      style={{
        position: "fixed",
        top: 16,
        right: 16,
        zIndex: 80,
        width: "min(360px, calc(100vw - 32px))",
        padding: 16,
        borderRadius: "var(--radius-lg)",
        border: "1px solid var(--color-border)",
        background: "var(--color-bg-elevated)",
        boxShadow: "0 20px 60px oklch(0 0 0 / 0.16)",
        display: "flex",
        flexDirection: "column",
        gap: 12,
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 4, minWidth: 0 }}>
          <strong style={{ fontSize: 14, color: "var(--color-text)" }}>{title}</strong>
          <span style={{ fontSize: 12, color: "var(--color-text-secondary)", lineHeight: 1.5 }}>{details}</span>
        </div>
        {state.status !== "downloading" && state.status !== "installing" && (
          <button className="btn btn-ghost" onClick={onDismiss} type="button" style={{ height: 28, padding: "0 10px", flexShrink: 0 }}>
            关闭
          </button>
        )}
      </div>

      {state.status === "downloading" && (
        <>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
            <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>{progressPercent}%</span>
            <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>已接收 {state.chunkCount} 个数据块</span>
          </div>
          <div style={{ width: "100%", height: 8, borderRadius: 999, background: "var(--color-bg-inset)", overflow: "hidden" }}>
            <div
              style={{
                width: `${progressPercent}%`,
                height: "100%",
                borderRadius: 999,
                background: "var(--color-accent)",
                transition: prefersReducedMotion ? "none" : "width 160ms cubic-bezier(0.16, 1, 0.3, 1)",
              }}
            />
          </div>
        </>
      )}

      {state.status === "available" && (
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
          <button className="btn btn-secondary" onClick={onDismiss} type="button">
            稍后
          </button>
          <button className="btn btn-primary" onClick={onInstall} type="button">
            Download & Install
          </button>
        </div>
      )}
    </div>
  );
}

function AppShell() {
  const { state, dispatch } = useWizard();
  const isProcessing = state.processStatus !== "idle";
  const [confirmLogout, setConfirmLogout] = useState(false);
  const [availableUpdate, setAvailableUpdate] = useState<AppUpdate | null>(null);
  const [updateToast, setUpdateToast] = useState<UpdateToastState>(INITIAL_UPDATE_TOAST);
  const prefersReducedMotion = usePrefersReducedMotion();

  const canGoNext = useCallback(() => {
    if (state.step === 0) return state.isLoggedIn;
    if (state.step === 1) {
      if (state.sourceType === "favorites") return true;
      return state.profileUrl.length > 0 && isValidProfileUrl(state.profileUrl);
    }
    if (state.step === 2) {
      if (state.sourceType === "favorites") return true;
      if (state.dateMode === "range") {
        return !!state.dateStart && !!state.dateEnd;
      }
      return true;
    }
    if (state.step === 3) return !!state.outputDir;
    return true;
  }, [state.step, state.isLoggedIn, state.profileUrl, state.outputDir, state.sourceType, state.dateMode, state.dateStart, state.dateEnd]);

  const handleNext = useCallback(async () => {
    if (state.step === 3) {
      dispatch({ type: "START_PROCESSING" });

      try {
        const uid = state.sourceType === "favorites" ? "" : extractUidFromUrl(state.profileUrl) || "";
        if (state.sourceType === "profile" && !extractUidFromUrl(state.profileUrl)) {
          dispatch({ type: "PROCESS_ERROR", message: "无法从链接中提取用户ID" });
          return;
        }

        const result = await startDownload(buildDownloadRequest({
          uid,
          cookie: state.cookie,
          filter: state.postFilter,
          includeImages: state.includeImages,
          dateMode: state.dateMode,
          dateStart: state.dateStart,
          dateEnd: state.dateEnd,
          outputDir: state.outputDir,
          ignoreDeleted: state.ignoreDeleted,
          sourceType: state.sourceType,
        }));

        dispatch({ type: "ADD_LOG", message: result });
        dispatch({ type: "DOWNLOAD_COMPLETE" });
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        if (msg.includes("已取消")) {
          dispatch({ type: "PROCESS_CANCEL" });
        } else {
          dispatch({ type: "PROCESS_ERROR", message: msg });
        }
      }
    } else if (canGoNext()) {
      dispatch({ type: "NEXT_STEP" });
    }
  }, [state, dispatch, canGoNext]);

  const handleExport = useCallback(async (format: ExportFormat) => {
    dispatch({ type: "EXPORT_START" });
    dispatch({ type: "ADD_LOG", message: `正在导出 ${formatLabel(format)}...` });

    try {
      const exportResult = await exportPosts({
        output_dir: state.outputDir,
        export_format: format,
      });
      dispatch({ type: "ADD_LOG", message: exportResult });
      dispatch({ type: "PROCESS_COMPLETE" });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      dispatch({ type: "PROCESS_ERROR", message: msg });
    }
  }, [state.outputDir, dispatch]);

  const handleContinueExport = useCallback(() => {
    dispatch({ type: "DOWNLOAD_COMPLETE" });
  }, [dispatch]);

  const handleBack = useCallback(() => {
    dispatch({ type: "PREV_STEP" });
  }, [dispatch]);

  const handleStop = useCallback(async () => {
    await cancelDownload();
    dispatch({ type: "ADD_LOG", message: "正在取消..." });
  }, [dispatch]);

  const handleReset = useCallback(() => {
    dispatch({ type: "RESET" });
  }, [dispatch]);

  const handleOpenOutputDir = useCallback(async () => {
    if (state.outputDir) await openOutputDir(state.outputDir);
  }, [state.outputDir]);

  const dismissUpdateToast = useCallback(() => {
    setUpdateToast(INITIAL_UPDATE_TOAST);
    setAvailableUpdate(null);
  }, []);

  const handleInstallUpdate = useCallback(async () => {
    if (!availableUpdate) return;

    try {
      let downloadedBytes = 0;
      let totalBytes = 0;
      let chunkCount = 0;

      setUpdateToast((current) => ({
        ...current,
        status: "downloading",
        errorMessage: "",
      }));

      await availableUpdate.downloadAndInstall((event: UpdateProgressEvent) => {
        switch (event.event) {
          case "Started": {
            totalBytes = event.data.contentLength ?? 0;
            setUpdateToast((current) => ({
              ...current,
              status: "downloading",
              totalBytes,
              downloadedBytes: 0,
              chunkCount: 0,
            }));
            break;
          }
          case "Progress": {
            downloadedBytes += event.data.chunkLength;
            chunkCount += 1;
            setUpdateToast((current) => ({
              ...current,
              status: "downloading",
              totalBytes,
              downloadedBytes,
              chunkCount,
            }));
            break;
          }
          case "Finished": {
            setUpdateToast((current) => ({
              ...current,
              status: "installing",
              downloadedBytes: totalBytes > 0 ? totalBytes : downloadedBytes,
              totalBytes,
              chunkCount,
            }));
            break;
          }
        }
      });

      await relaunchApp();
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setUpdateToast((current) => ({
        ...current,
        status: "error",
        errorMessage: message,
      }));
    }
  }, [availableUpdate]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
        e.preventDefault();
        handleNext();
      }
      if (e.key === "Escape" && state.step > 0 && !isProcessing) {
        e.preventDefault();
        handleBack();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleNext, handleBack, state.step, isProcessing]);

  useEffect(() => {
    let cancelled = false;

    void checkForAppUpdate()
      .then((update: AppUpdate | null) => {
        if (!update || cancelled) return;

        setAvailableUpdate(update);
        setUpdateToast({
          status: "available",
          version: update.version,
          notes: update.body?.trim() ?? "",
          downloadedBytes: 0,
          totalBytes: 0,
          chunkCount: 0,
          errorMessage: "",
        });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        console.warn("Failed to check for updates", err);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const showFooter = !isProcessing;

  return (
    <div className="app-shell">
      <UpdateToast
        state={updateToast}
        prefersReducedMotion={prefersReducedMotion}
        onInstall={handleInstallUpdate}
        onDismiss={dismissUpdateToast}
      />
      <div className="app-window">
        <header className="app-header">
          <h1 className="app-title">微存 <span style={{ fontSize: "0.5em", opacity: 0.6, fontWeight: 400, marginLeft: 8 }}>Wecun</span></h1>
          {state.isLoggedIn && (
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <span className="auth-badge">
                <span className="auth-dot" />
                已登录
              </span>
              {confirmLogout ? (
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>确认退出账号？</span>
                  <button
                    className="logout-btn"
                    style={{ color: "var(--color-danger)" }}
                    onClick={() => {
                      void clearSavedCookie().then(() => {
                        dispatch({ type: "LOGOUT" });
                        setConfirmLogout(false);
                      });
                    }}
                    type="button"
                  >
                    退出账号
                  </button>
                  <button
                    className="logout-btn"
                    onClick={() => setConfirmLogout(false)}
                    type="button"
                  >
                    取消
                  </button>
                </div>
              ) : (
                <button
                  className="logout-btn"
                  onClick={() => setConfirmLogout(true)}
                  type="button"
                >
                  退出账号
                </button>
              )}
            </div>
          )}
        </header>

        <StepIndicator steps={STEPS} currentStep={state.step} />

        <div className="step-content">
          {state.step === 0 && (
            <StepLogin
              isLoggedIn={state.isLoggedIn}
              restoreError={state.loginError}
            />
          )}
          {state.step === 1 && (
            <StepTarget
              profileUrl={state.profileUrl}
              onProfileUrlChange={(url) => dispatch({ type: "SET_PROFILE_URL", url })}
              onNext={() => dispatch({ type: "NEXT_STEP" })}
              sourceType={state.sourceType}
              onSourceTypeChange={(type) => dispatch({ type: "SET_SOURCE_TYPE", sourceType: type })}
            />
          )}
          {state.step === 2 && (
            <StepOptions
              postFilter={state.postFilter}
              onPostFilterChange={(f) => dispatch({ type: "SET_POST_FILTER", filter: f })}
              includeImages={state.includeImages}
              onIncludeImagesChange={(v) => dispatch({ type: "SET_INCLUDE_IMAGES", value: v })}
              dateMode={state.dateMode}
              onDateModeChange={(m) => dispatch({ type: "SET_DATE_MODE", mode: m })}
              dateStart={state.dateStart}
              onDateStartChange={(d) => dispatch({ type: "SET_DATE_START", date: d })}
              dateEnd={state.dateEnd}
              onDateEndChange={(d) => dispatch({ type: "SET_DATE_END", date: d })}
              ignoreDeleted={state.ignoreDeleted}
              onIgnoreDeletedChange={(v) => dispatch({ type: "SET_IGNORE_DELETED", value: v })}
              sourceType={state.sourceType}
            />
          )}
          {state.step === 3 && (
            <StepExportSettings
              outputDir={state.outputDir}
              onDirChange={(dir) => dispatch({ type: "SET_OUTPUT_DIR", dir })}
            />
          )}
        </div>

        {showFooter && (
          <>
            <div className="step-footer">
              <div>
                {state.step > 0 && (
                  <button className="btn btn-secondary" onClick={handleBack} type="button">
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true" style={{ flexShrink: 0 }}>
                      <path d="M8 3L4 7L8 11" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                    上一步
                  </button>
                )}
              </div>
              <button
                className="btn btn-primary"
                onClick={handleNext}
                disabled={!canGoNext()}
                type="button"
              >
                {state.step === 3 ? "开始下载" : (
                  <>
                    下一步
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                      <path d="M6 3L10.5 8L6 13" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                  </>
                )}
                {canGoNext() && (
                  <kbd style={{ fontSize: 10, opacity: 0.5, fontFamily: "var(--font-sans)", marginLeft: 4 }}>⏎</kbd>
                )}
              </button>
            </div>
            {!canGoNext() && (
              <div className="step-hint">
                {state.step === 0 && "请先登录微博账号"}
                {state.step === 1 && state.sourceType === "profile" && "请输入有效的微博主页地址"}
                {state.step === 3 && "请选择保存目录"}
              </div>
            )}
          </>
        )}

        {isProcessing && (
          <StepProcessing
            processStatus={state.processStatus as Exclude<typeof state.processStatus, "idle">}
            phase={state.phase}
            progress={state.progress}
            current={state.current}
            total={state.total}
            errorMessage={state.errorMessage}
            logs={state.logs}
            onStop={handleStop}
            onReset={handleReset}
            onExport={handleExport}
            onContinueExport={handleContinueExport}
            onOpenOutputDir={handleOpenOutputDir}
          />
        )}

        {updateToast.status === "downloading" && getUpdateProgressPercent(updateToast.downloadedBytes, updateToast.totalBytes) === 100 && (
          <div style={{ display: "none" }}>100%</div>
        )}
      </div>
    </div>
  );
}

function App() {
  return (
    <WizardProvider>
      <AppShell />
    </WizardProvider>
  );
}

export default App;
