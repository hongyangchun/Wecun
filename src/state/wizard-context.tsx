import { createContext, useContext, useReducer, useRef, useEffect, type Dispatch, type ReactNode } from "react";
import {
  wizardReducer,
  INITIAL_STATE,
  type WizardState,
  type WizardAction,
} from "./wizard-reducer";
import {
  onProgress,
  onCookieReceived,
  onLoginInvalid,
  getSavedCookie,
  getCurrentUserInfo,
} from "../lib/tauri-bridge";

interface WizardContextValue {
  state: WizardState;
  dispatch: Dispatch<WizardAction>;
}

const WizardContext = createContext<WizardContextValue | null>(null);

export function useWizard(): WizardContextValue {
  const ctx = useContext(WizardContext);
  if (!ctx) throw new Error("useWizard must be used within WizardProvider");
  return ctx;
}

interface WizardProviderProps {
  children: ReactNode;
}

export function WizardProvider({ children }: WizardProviderProps) {
  const [state, dispatch] = useReducer(wizardReducer, INITIAL_STATE);
  const settingsRef = useRef({ loaded: false });

  useEffect(() => {
    let cancelled = false;
    onProgress((e) => {
      if (cancelled) return;
      dispatch({ type: "UPDATE_PROGRESS", ...e });
    }).then((fn) => {
      if (!cancelled) return;
      fn();
    });
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    const fn = onCookieReceived(async (cookie) => {
      dispatch({ type: "LOGIN_SUCCESS", cookie });
      // Fetch username after successful login
      try {
        const username = await getCurrentUserInfo();
        if (username) {
          dispatch({ type: "SET_USERNAME", username });
        } else {
          // No username returned - might indicate cookie not fully validated yet
          dispatch({ type: "USERNAME_FETCH_FAILED" });
        }
      } catch {
        // Username fetch failed - might indicate cookie validation issue
        dispatch({ type: "USERNAME_FETCH_FAILED" });
      }
    });
    return () => { fn.then((u) => u()); };
  }, []);

  useEffect(() => {
    const fn = onLoginInvalid((message) => {
      dispatch({ type: "LOGIN_INVALID", message });
    });
    return () => { fn.then((u) => u()); };
  }, []);

  // Load saved cookie on mount - single call instead of hasSavedCookie + loadSavedCookie
  useEffect(() => {
    let cancelled = false;
    // Use setTimeout to defer loading and allow UI to render first
    const timer = setTimeout(() => {
      void getSavedCookie().then(async (cookie) => {
        if (!cancelled && cookie) {
          dispatch({ type: "LOGIN_SUCCESS", cookie });
          // Fetch username after restoring cookie
          try {
            const username = await getCurrentUserInfo();
            if (username && !cancelled) {
              dispatch({ type: "SET_USERNAME", username });
            } else {
              if (!cancelled) dispatch({ type: "USERNAME_FETCH_FAILED" });
            }
          } catch {
            if (!cancelled) dispatch({ type: "USERNAME_FETCH_FAILED" });
          }
        }
      });
    }, 100); // Small delay to let UI to render first
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, []);

  useEffect(() => {
    if (settingsRef.current.loaded) return;
    settingsRef.current.loaded = true;
    try {
      const raw = localStorage.getItem("weibo-dl-settings");
      if (raw) {
        const s = JSON.parse(raw);
        if (s.postFilter) dispatch({ type: "SET_POST_FILTER", filter: s.postFilter });
        if (s.includeImages !== undefined) dispatch({ type: "SET_INCLUDE_IMAGES", value: s.includeImages });
        if (s.downloadRange) dispatch({ type: "SET_DOWNLOAD_RANGE", range: s.downloadRange });
        if (s.ignoreDeleted !== undefined) dispatch({ type: "SET_IGNORE_DELETED", value: s.ignoreDeleted });
        if (s.minTextLength !== undefined) dispatch({ type: "SET_MIN_TEXT_LENGTH", value: s.minTextLength });
      }
    } catch {
      void 0;
    }
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem("weibo-dl-settings", JSON.stringify({
        postFilter: state.postFilter,
        includeImages: state.includeImages,
        downloadRange: state.downloadRange,
        ignoreDeleted: state.ignoreDeleted,
        minTextLength: state.minTextLength,
      }));
    } catch {
      void 0;
    }
  }, [state.postFilter, state.includeImages, state.downloadRange, state.ignoreDeleted, state.minTextLength]);

  return (
    <WizardContext.Provider value={{ state, dispatch }}>
      {children}
    </WizardContext.Provider>
  );
}
