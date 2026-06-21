import { useEffect, useState } from "react";

type Theme = "auto" | "light" | "dark";

const STORAGE_KEY = "grimoire:theme";

function loadStoredTheme(): Theme {
  const v = localStorage.getItem(STORAGE_KEY);
  return v === "light" || v === "dark" ? v : "auto";
}

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "auto") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", theme);
  }
}

/// Best called from main.tsx before React renders so first paint has the
/// correct palette and we avoid a light-mode flash.
export function initThemeFromStorage() {
  applyTheme(loadStoredTheme());
}

export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(() => loadStoredTheme());

  useEffect(() => {
    applyTheme(theme);
    if (theme === "auto") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

  const next: Record<Theme, Theme> = { auto: "dark", dark: "light", light: "auto" };
  const label: Record<Theme, string> = {
    auto: "Theme · auto (follows system)",
    light: "Theme · light",
    dark: "Theme · dark",
  };

  return (
    <button
      type="button"
      className="theme-toggle"
      onClick={() => setTheme(next[theme])}
      title={label[theme]}
      aria-label={label[theme]}
    >
      {theme === "dark" ? <Moon /> : theme === "light" ? <Sun /> : <Auto />}
    </button>
  );
}

function Sun() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
    </svg>
  );
}

function Moon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
    </svg>
  );
}

function Auto() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor" />
    </svg>
  );
}
