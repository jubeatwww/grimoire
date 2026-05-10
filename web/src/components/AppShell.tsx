import type { ReactNode } from "react";

interface AppShellProps {
  children: ReactNode;
  detail: ReactNode;
}

export function AppShell({ children, detail }: AppShellProps) {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">HG</div>
        <nav>
          <button className="active">作品庫</button>
          <button>整理工作台</button>
          <button>匯入 staging</button>
          <button>下載紀錄</button>
          <button>設定</button>
        </nav>
        <section>
          <h3>Primary category</h3>
          <div className="chips">
            {["Visual Novel", "Action", "RPG", "Simulation", "Strategy", "3D"].map((item) => (
              <span className="chip" key={item}>{item}</span>
            ))}
          </div>
        </section>
        <section>
          <h3>Genre facets</h3>
          <div className="chips">
            {["Needs review", "Has DLsite", "Missing cover", "Downloaded"].map((item) => (
              <span className="chip" key={item}>{item}</span>
            ))}
          </div>
        </section>
        <section>
          <h3>Legacy location</h3>
          <div className="chips">
            {["ADV", "ACT", "RPG", "舊 SIM+SLG", "未分類"].map((item) => (
              <span className="chip muted" key={item}>{item}</span>
            ))}
          </div>
        </section>
      </aside>
      <main className="main-pane">{children}</main>
      <aside className="detail-pane">{detail}</aside>
    </div>
  );
}
