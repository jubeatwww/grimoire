import { useState, type ReactNode } from "react";
import {
  QUICK_FILTERS,
  type FilterGroup,
  type FilterOption,
  type FilterOptions,
  type Filters,
} from "../App";

interface AppShellProps {
  children: ReactNode;
  detail: ReactNode;
  filters: Filters;
  options: FilterOptions;
  onToggleFilter: (group: FilterGroup, value: string) => void;
  chromeless?: boolean;
}

const TAGS_INITIAL_LIMIT = 20;

export function AppShell({
  children,
  detail,
  filters,
  options,
  onToggleFilter,
  chromeless,
}: AppShellProps) {
  const isOn = (group: FilterGroup, value: string) => filters[group].has(value);

  if (chromeless) {
    return (
      <div className="app-shell app-shell-chromeless">
        <main className="main-pane">{children}</main>
      </div>
    );
  }

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
          values={options.primary}
          group="primary"
          isOn={isOn}
          onToggle={onToggleFilter}
        />
        <FilterSection
          title="Work type"
          values={options.workType}
          group="workType"
          isOn={isOn}
          onToggle={onToggleFilter}
        />
        <FilterSection
          title="Quick filters"
          values={QUICK_FILTERS.map((q) => ({ id: q.id, count: 0, label: q.label }))}
          group="quick"
          isOn={isOn}
          onToggle={onToggleFilter}
          hideCounts
        />
        <TagsSection
          values={options.tags}
          isOn={isOn}
          onToggle={onToggleFilter}
        />
        <FilterSection
          title="Legacy location"
          values={options.legacy}
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
  values: (FilterOption & { label?: string })[];
  group: FilterGroup;
  isOn: (group: FilterGroup, value: string) => boolean;
  onToggle: (group: FilterGroup, value: string) => void;
  muted?: boolean;
  hideCounts?: boolean;
}

function FilterSection({
  title,
  values,
  group,
  isOn,
  onToggle,
  muted,
  hideCounts,
}: FilterSectionProps) {
  if (values.length === 0) return null;
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
              {v.label ?? v.id}
              {!hideCounts && <span className="chip-count">{v.count}</span>}
            </button>
          );
        })}
      </div>
    </section>
  );
}

interface TagsSectionProps {
  values: FilterOption[];
  isOn: (group: FilterGroup, value: string) => boolean;
  onToggle: (group: FilterGroup, value: string) => void;
}

function TagsSection({ values, isOn, onToggle }: TagsSectionProps) {
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState(false);

  if (values.length === 0) return null;

  const q = search.trim().toLowerCase();
  // Always show selected tags so they stay clickable even if filtered out.
  const selected = values.filter((v) => isOn("tags", v.id));
  const selectedIds = new Set(selected.map((v) => v.id));
  const rest = values.filter((v) => !selectedIds.has(v.id));
  const matching = q
    ? rest.filter((v) => v.id.toLowerCase().includes(q))
    : rest;
  const visibleRest = expanded || q ? matching : matching.slice(0, TAGS_INITIAL_LIMIT);
  const hiddenCount = matching.length - visibleRest.length;

  return (
    <section>
      <h3>
        Tags
        <span className="tags-total">{values.length}</span>
      </h3>
      <input
        className="tags-search"
        type="search"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        placeholder="Filter tags…"
      />
      <div className="chips">
        {selected.map((v) => (
          <TagChip key={v.id} option={v} isOn onToggle={onToggle} />
        ))}
        {visibleRest.map((v) => (
          <TagChip
            key={v.id}
            option={v}
            isOn={false}
            onToggle={onToggle}
          />
        ))}
      </div>
      {!q && hiddenCount > 0 && (
        <button
          type="button"
          className="tags-expand"
          onClick={() => setExpanded((x) => !x)}
        >
          {expanded ? "Show less" : `Show ${hiddenCount} more`}
        </button>
      )}
    </section>
  );
}

function TagChip({
  option,
  isOn,
  onToggle,
}: {
  option: FilterOption;
  isOn: boolean;
  onToggle: (group: FilterGroup, value: string) => void;
}) {
  return (
    <button
      type="button"
      className={`chip ${isOn ? "chip-on" : ""}`}
      onClick={() => onToggle("tags", option.id)}
      aria-pressed={isOn}
      title={option.id}
    >
      {option.id}
      <span className="chip-count">{option.count}</span>
    </button>
  );
}
