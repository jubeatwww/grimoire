import type { ReactNode } from "react";

interface AppShellProps {
  children: ReactNode;
  detail: ReactNode;
  chromeless?: boolean;
}

export function AppShell({ children, detail, chromeless }: AppShellProps) {
  if (chromeless) {
    return (
      <div className="app-shell app-shell-chromeless">
        <main className="main-pane">{children}</main>
      </div>
    );
  }
  if (!detail) {
    return (
      <div className="app-shell app-shell-no-detail">
        <main className="main-pane">{children}</main>
      </div>
    );
  }
  return (
    <div className="app-shell">
      <main className="main-pane">{children}</main>
      <aside className="detail-pane">{detail}</aside>
    </div>
  );
}
