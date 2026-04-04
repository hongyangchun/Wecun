import { useCallback, useState, useEffect } from "react";
import StepIndicator from "./components/StepIndicator";
import StepLogin from "./components/StepLogin";
import StepTarget from "./components/StepTarget";
import StepOptions from "./components/StepOptions";
import StepExportSettings from "./components/StepExportSettings";
import StepProcessing from "./components/StepProcessing";
import { WizardProvider, useWizard } from "./state/wizard-context";
import { startDownload, cancelDownload, exportPosts, openOutputDir, clearSavedCookie } from "./lib/tauri-bridge";
import { buildDownloadRequest, extractUidFromUrl, isValidProfileUrl } from "./lib/validation";
import type { ExportFormat } from "./types/contracts";
import "./App.css";

const STEPS = ["登录", "目标", "选项", "导出"];

function formatLabel(fmt: ExportFormat): string {
  const labels: Record<ExportFormat, string> = {
    html: "HTML",
    "md-single": "Markdown",
    "md-multi": "Markdown（分文件）",
  };
  return labels[fmt];
}

function AppShell() {
  const { state, dispatch } = useWizard();
  const isProcessing = state.processStatus !== "idle";
  const [confirmLogout, setConfirmLogout] = useState(false);

  const canGoNext = useCallback(() => {
    if (state.step === 0) return state.isLoggedIn;
    if (state.step === 1) return state.profileUrl.length > 0 && isValidProfileUrl(state.profileUrl);
    if (state.step === 3) return !!state.outputDir;
    return true;
  }, [state.step, state.isLoggedIn, state.profileUrl, state.outputDir]);

  const handleNext = useCallback(async () => {
    if (state.step === 3) {
      dispatch({ type: "START_PROCESSING" });

      try {
        const uid = extractUidFromUrl(state.profileUrl);
        if (!uid) {
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
          minTextLength: state.minTextLength,
        }));

        dispatch({ type: "ADD_LOG", message: result });
        dispatch({ type: "EXPORT_START" });
        dispatch({ type: "ADD_LOG", message: `正在导出 ${formatLabel(state.exportFormat)}...` });

        const exportResult = await exportPosts({
          output_dir: state.outputDir,
          export_format: state.exportFormat,
        });

        dispatch({ type: "ADD_LOG", message: exportResult });
        dispatch({ type: "PROCESS_COMPLETE" });
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

  const showFooter = !isProcessing;

  return (
    <div className="app-shell">
      <div className="app-window">
        <header className="app-header">
          <h1 className="app-title">微博备份助手</h1>
          {state.isLoggedIn && (
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <span className="auth-badge">
                <span className="auth-dot" />
                已登录
              </span>
              {confirmLogout ? (
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>确认退出？</span>
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
                    退出
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
                  退出
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
              minTextLength={state.minTextLength}
              onMinTextLengthChange={(n) => dispatch({ type: "SET_MIN_TEXT_LENGTH", length: n })}
            />
          )}
          {state.step === 3 && (
            <StepExportSettings
              exportFormat={state.exportFormat}
              onExportFormatChange={(f) => dispatch({ type: "SET_EXPORT_FORMAT", format: f })}
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
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" style={{ flexShrink: 0 }}>
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
                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
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
                {state.step === 1 && "请输入有效的微博主页地址"}
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
            onOpenOutputDir={handleOpenOutputDir}
          />
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
