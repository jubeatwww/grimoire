import type { ReactNode } from "react";
import {
  LEGACY_LOCATIONS,
  PRIMARY_CATEGORIES,
  QUICK_FILTERS,
  type FilterGroup,
  type Filters,
} from "../App";

interface AppShellProps {
  children: ReactNode;
  detail: ReactNode;
  filters: Filters;
  onToggleFilter: (group: FilterGroup, value: string) => void;
}

export function AppShell({ children, detail, filters, onToggleFilter }: AppShellProps) {
  const isOn = (group: FilterGroup, value: string) => filters[group].has(value);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">HG</div>
        <nav>
          <button className="active">作品庫</button>
          <button>匯入 staging</button>
          <button>下載紀錄</button>
          <button>設定</button>
        </nav>
        <FilterSection
          title="Primary category"
          values={PRIMARY_CATEGORIES.map((label) => ({ id: label, label }))}
          group="primary"
          isOn={isOn}
          onToggle={onToggleFilter}
        />
        <FilterSection
          title="Quick filters"
          values={QUICK_FILTERS}
          group="quick"
          isOn={isOn}
          onToggle={onToggleFilter}
        />
        <FilterSection
          title="Legacy location"
          values={LEGACY_LOCATIONS.map((label) => ({ id: label, label }))}
          group="legacy"
          isOn={isOn}
          onToggle={onToggleFilter}
          muted
        />
      </aside>
      <main className="main-pane">{children}</main>
      <aside className="detail-pane">{detail}</aside>
    </div>
  );
}

interface FilterSectionProps {
  title: string;
  values: { id: string; label: string }[];
  group: FilterGroup;
  isOn: (group: FilterGroup, value: string) => boolean;
  onToggle: (group: FilterGroup, value: string) => void;
  muted?: boolean;
}

function FilterSection({ title, values, group, isOn, onToggle, muted }: FilterSectionProps) {
  return (
    <section>
      <h3>{title}</h3>
      <div className="chips">
        {values.map((v) => {
          const on = isOn(group, v.id);
          const classes = ["chip"];
          if (on) classes.push("chip-on");
          if (muted && !on) classes.push("muted");
          return (
            <button
              key={v.id}
              type="button"
              className={classes.join(" ")}
              onClick={() => onToggle(group, v.id)}
              aria-pressed={on}
            >
              {v.label}
            </button>
          );
        })}
      </div>
    </section>
  );
}
