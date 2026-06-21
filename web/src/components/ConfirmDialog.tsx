import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

export interface ConfirmOpts {
  title: string;
  message?: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}

type Ask = (opts: ConfirmOpts) => Promise<boolean>;

const ConfirmContext = createContext<Ask | null>(null);

export function ConfirmProvider({ children }: { children: ReactNode }) {
  const [opts, setOpts] = useState<ConfirmOpts | null>(null);
  const resolveRef = useRef<((ok: boolean) => void) | null>(null);
  const cancelBtnRef = useRef<HTMLButtonElement | null>(null);

  const ask = useCallback<Ask>((o) => {
    return new Promise<boolean>((resolve) => {
      // Reject any prior pending dialog so we never silently swallow a
      // confirmation when the user spam-clicks before finishing the last one.
      resolveRef.current?.(false);
      resolveRef.current = resolve;
      setOpts(o);
    });
  }, []);

  const close = useCallback((ok: boolean) => {
    resolveRef.current?.(ok);
    resolveRef.current = null;
    setOpts(null);
  }, []);

  // Esc cancels.
  useEffect(() => {
    if (!opts) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [opts, close]);

  // Default focus on Cancel (the safe choice for destructive dialogs).
  useEffect(() => {
    if (opts) cancelBtnRef.current?.focus();
  }, [opts]);

  return (
    <ConfirmContext.Provider value={ask}>
      {children}
      {opts && (
        <div
          className="confirm-backdrop"
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) close(false);
          }}
          role="dialog"
          aria-modal="true"
        >
          <div className={`confirm-dialog ${opts.danger ? "confirm-danger" : ""}`}>
            <h2 className="confirm-title">{opts.title}</h2>
            {opts.message && <div className="confirm-message">{opts.message}</div>}
            <div className="confirm-actions">
              <button
                ref={cancelBtnRef}
                type="button"
                className="confirm-btn confirm-cancel"
                onClick={() => close(false)}
              >
                {opts.cancelLabel ?? "Cancel"}
              </button>
              <button
                type="button"
                className={`confirm-btn ${opts.danger ? "confirm-danger-btn" : "confirm-primary-btn"}`}
                onClick={() => close(true)}
              >
                {opts.confirmLabel ?? (opts.danger ? "Delete" : "Confirm")}
              </button>
            </div>
          </div>
        </div>
      )}
    </ConfirmContext.Provider>
  );
}

export function useConfirm(): Ask {
  const ctx = useContext(ConfirmContext);
  if (!ctx) throw new Error("useConfirm must be used within ConfirmProvider");
  return ctx;
}
