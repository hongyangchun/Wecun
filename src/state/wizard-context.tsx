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
  hasSavedCookie,
  loadSavedCookie,
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
    const fn = onCookieReceived((cookie) => {
      dispatch({ type: "LOGIN_SUCCESS", cookie });
    });
    return () => { fn.then((u) => u()); };
  }, []);

  useEffect(() => {
    const fn = onLoginInvalid((message) => {
      dispatch({ type: "LOGIN_INVALID", message });
    });
    return () => { fn.then((u) => u()); };
  }, []);

  useEffect(() => {
    let cancelled = false;
    void hasSavedCookie().then((has) => {
      if (!has || cancelled) return;
      void loadSavedCookie()
        .then((c) => {
          if (!cancelled) dispatch({ type: "LOGIN_SUCCESS", cookie: c });
        })
        .catch(() => {});
    });
    return () => { cancelled = true; };
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
        if (s.dateMode) dispatch({ type: "SET_DATE_MODE", mode: s.dateMode });
        if (s.minTextLength !== undefined) dispatch({ type: "SET_MIN_TEXT_LENGTH", length: s.minTextLength });
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
        dateMode: state.dateMode,
        minTextLength: state.minTextLength,
      }));
    } catch {
      void 0;
    }
  }, [state.postFilter, state.includeImages, state.dateMode, state.minTextLength]);

  return (
    <WizardContext.Provider value={{ state, dispatch }}>
      {children}
    </WizardContext.Provider>
  );
}
