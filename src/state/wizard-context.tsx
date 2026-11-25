import { createContext, useContext, useReducer, useRef, useEffect, useState, type Dispatch, type ReactNode } from "react";
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
  setCookie,
} from "../lib/tauri-bridge";

interface WizardContextValue {
  state: WizardState;
  dispatch: Dispatch<WizardAction>;
  initialized: boolean;
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
  const [initialized, setInitialized] = useState(false);
  const settingsRef = useRef({ loaded: false });

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    onProgress((e) => {
      if (cancelled) return;
      dispatch({ type: "UPDATE_PROGRESS", ...e });
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    onCookieReceived(async (cookie) => {
      if (cancelled) return;
      dispatch({ type: "LOGIN_SUCCESS", cookie });
      // Fetch username after successful login
      try {
        const username = await getCurrentUserInfo();
        if (cancelled) return;
        if (username) {
          dispatch({ type: "SET_USERNAME", username });
        } else {
          // No username returned - might indicate cookie not fully validated yet
          dispatch({ type: "USERNAME_FETCH_FAILED" });
        }
      } catch {
        if (cancelled) return;
        // Username fetch failed - might indicate cookie validation issue
        dispatch({ type: "USERNAME_FETCH_FAILED" });
      }
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    onLoginInvalid((message) => {
      if (cancelled) return;
      dispatch({ type: "LOGIN_INVALID", message });
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  // Load saved cookie on mount - prefill cookie and silently validate
  useEffect(() => {
    const loadCookie = async () => {
      try {
        const cookie = await getSavedCookie();
        if (cookie) {
          dispatch({ type: "SET_COOKIE", cookie });
          // Set cookie in backend state first, then validate
          try {
            await setCookie(cookie);
            const username = await getCurrentUserInfo();
            if (username) {
              dispatch({ type: "LOGIN_SUCCESS", cookie, username });
            }
          } catch {
            // Cookie invalid or expired, user will need to login manually
          }
        }
      } catch {
        // Ignore storage read failure on startup so UI still initializes
      } finally {
        setInitialized(true);
      }
    };
    loadCookie();
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
    <WizardContext.Provider value={{ state, dispatch, initialized }}>
      {children}
    </WizardContext.Provider>
  );
}
