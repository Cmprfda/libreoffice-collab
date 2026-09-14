/**
 * In-app toast notifications (the flyout kind, bottom-right, Win11 style).
 * Native Windows toasts are sent separately from Rust for events that must be
 * visible even when the window is hidden in the tray.
 */
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { AlertIcon, CheckIcon } from "./Icons";

type ToastKind = "info" | "success" | "error";

interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

interface ToastApi {
  push: (message: string, kind?: ToastKind) => void;
}

const ToastContext = createContext<ToastApi | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextId = useRef(1);

  const push = useCallback((message: string, kind: ToastKind = "info") => {
    const id = nextId.current++;
    setToasts((current) => [...current, { id, kind, message }]);
    window.setTimeout(
      () => setToasts((current) => current.filter((toast) => toast.id !== id)),
      kind === "error" ? 6000 : 3500,
    );
  }, []);

  const api = useMemo<ToastApi>(() => ({ push }), [push]);

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="toasts">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast toast--${toast.kind}`}>
            {toast.kind === "error" ? (
              <AlertIcon style={{ color: "var(--danger)", flex: "none" }} />
            ) : (
              <CheckIcon style={{ color: "var(--success)", flex: "none" }} />
            )}
            <span>{toast.message}</span>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastApi {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used inside <ToastProvider>");
  return ctx;
}
