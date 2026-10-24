import { useState, type FormEvent } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deleteCode,
  deleteKeywordRule,
  deleteWorkTypeRule,
  fetchCommonTags,
  fetchOtherFiles,
  fetchRefetchStatus,
  fetchRules,
  saveKeywordRule,
  saveWorkTypeRule,
  startRefetch,
  upsertCode,
} from "../api/client";
import { SUGGESTIONS_KEY, useVocabulary, VOCABULARY_KEY } from "../api/queries";
import type { CategoryDef, OtherFilesReport, SourceName, WorkTypeRule } from "../api/types";
import { codeOptions, SOURCE_LABELS } from "../vocabulary";
import { CodePill, SourceTagPill } from "./Pills";

const SOURCES: SourceName[] = ["dlsite", "vndb", "steam"];
const RULES_KEY = ["classification-rules"] as const;
const COMMON_TAGS_KEY = ["common-tags"] as const;

/** Settings: genre codes, classification rules, and the source re-fetch. */
export function SettingsView() {
  return (
    <div className="settings-view">
      <CodeSettings />
      <RuleSettings />
      <RefetchSettings />
      <OtherFilesReportSection />
    </div>
  );
}

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** Rules and common tags feed the suggestions; refresh all three together. */
function useInvalidateRules() {
  const queryClient = useQueryClient();
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: RULES_KEY }),
      queryClient.invalidateQueries({ queryKey: COMMON_TAGS_KEY }),
      queryClient.invalidateQueries({ queryKey: SUGGESTIONS_KEY }),
    ]);
}

interface KeywordDraft {
  source: SourceName | "";
  keyword: string;
  code: string;
  contestsType: boolean;
}

const EMPTY_KEYWORD: KeywordDraft = { source: "", keyword: "", code: "", contestsType: false };

function RuleSettings() {
  const vocab = useVocabulary();
  const rules = useQuery({ queryKey: RULES_KEY, queryFn: fetchRules }).data;
  const commonTags = useQuery({ queryKey: COMMON_TAGS_KEY, queryFn: fetchCommonTags }).data;
  const invalidate = useInvalidateRules();
  const [error, setError] = useState<string | null>(null);
  const [keywordDraft, setKeywordDraft] = useState<KeywordDraft>(EMPTY_KEYWORD);
  const [typeDraft, setTypeDraft] = useState<WorkTypeRule>({
    source: "dlsite",
    workType: "",
    code: "",
    confident: true,
  });

  const run = async (action: () => Promise<unknown>) => {
    setError(null);
    try {
      await action();
      await invalidate();
    } catch (e) {
      setError(errorText(e));
    }
  };

  const submitKeyword = (e: FormEvent) => {
    e.preventDefault();
    void run(async () => {
      await saveKeywordRule({
        source: keywordDraft.source || null,
        keyword: keywordDraft.keyword,
        code: keywordDraft.code,
        contestsType: keywordDraft.contestsType,
      });
      setKeywordDraft(EMPTY_KEYWORD);
    });
  };

  const submitType = (e: FormEvent) => {
    e.preventDefault();
    void run(async () => {
      await saveWorkTypeRule({ ...typeDraft, code: typeDraft.code || null });
      setTypeDraft({ ...typeDraft, workType: "", code: "" });
    });
  };

  if (!rules) return <section className="settings-section">Loading rules…</section>;

  return (
    <section className="settings-section">
      <header>
        <h2>Classification rules</h2>
        <p>
          A source's work type proposes a code. Keywords match source tags by substring and add
          codes. Only keywords marked "contests type" make a suggestion need review when they
          point to another category.
        </p>
      </header>
      {error && <p className="settings-error">{error}</p>}

      <h3 className="settings-subhead">Work types</h3>
      <table className="settings-table">
        <thead>
          <tr>
            <th>Source</th>
            <th>Work type</th>
            <th>Code</th>
            <th>Confident</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {rules.workTypeRules.map((rule) => (
            <WorkTypeRuleRow
              key={`${rule.source}:${rule.workType}`}
              rule={rule}
              onSave={(next) => run(() => saveWorkTypeRule(next))}
              onDelete={() => run(() => deleteWorkTypeRule(rule.source, rule.workType))}
            />
          ))}
        </tbody>
      </table>
      <form className="settings-add-code" onSubmit={submitType}>
        <select
          value={typeDraft.source}
          onChange={(e) => setTypeDraft({ ...typeDraft, source: e.target.value as SourceName })}
        >
          {SOURCES.map((s) => (
            <option key={s} value={s}>
              {SOURCE_LABELS[s]}
            </option>
          ))}
        </select>
        <input
          value={typeDraft.workType}
          onChange={(e) => setTypeDraft({ ...typeDraft, workType: e.target.value })}
          placeholder="Work type, e.g. RPG"
          required
        />
        <input
          value={typeDraft.code ?? ""}
          onChange={(e) => setTypeDraft({ ...typeDraft, code: e.target.value })}
          placeholder="Code (empty = by tags)"
          list="genre-code-options-settings"
        />
        <label>
          <input
            type="checkbox"
            checked={typeDraft.confident}
            onChange={(e) => setTypeDraft({ ...typeDraft, confident: e.target.checked })}
          />
          Confident
        </label>
        <button type="submit">Add or update work type</button>
      </form>

      <h3 className="settings-subhead">Keywords</h3>
      <ul className="settings-keywords">
        {rules.keywordRules.map((rule) => (
          <li key={rule.id}>
            <span className="settings-keyword">{rule.keyword}</span>
            <span className="settings-arrow">→</span>
            <CodePill code={rule.code} vocab={vocab} />
            <small>{rule.source ? SOURCE_LABELS[rule.source] : "any source"}</small>
            {rule.contestsType && <small className="settings-contests">contests type</small>}
            <button
              type="button"
              className="settings-delete"
              onClick={() => void run(() => deleteKeywordRule(rule.id))}
              aria-label={`Remove keyword ${rule.keyword}`}
            >
              ×
            </button>
          </li>
        ))}
      </ul>
      <form className="settings-add-code" onSubmit={submitKeyword}>
        <select
          value={keywordDraft.source}
          onChange={(e) =>
            setKeywordDraft({ ...keywordDraft, source: e.target.value as SourceName | "" })
          }
        >
          <option value="">Any source</option>
          {SOURCES.map((s) => (
            <option key={s} value={s}>
              {SOURCE_LABELS[s]}
            </option>
          ))}
        </select>
        <input
          value={keywordDraft.keyword}
          onChange={(e) => setKeywordDraft({ ...keywordDraft, keyword: e.target.value })}
          placeholder="Keyword"
          required
        />
        <input
          value={keywordDraft.code}
          onChange={(e) => setKeywordDraft({ ...keywordDraft, code: e.target.value })}
          placeholder="Code"
          list="genre-code-options-settings"
          required
        />
        <label>
          <input
            type="checkbox"
            checked={keywordDraft.contestsType}
            onChange={(e) => setKeywordDraft({ ...keywordDraft, contestsType: e.target.checked })}
          />
          Contests type
        </label>
        <button type="submit">Add keyword</button>
      </form>
      <datalist id="genre-code-options-settings">
        {codeOptions(vocab).map((o) => (
          <option key={o.code} value={o.code}>
            {o.label}
          </option>
        ))}
      </datalist>

      <h3 className="settings-subhead">Common tags without a keyword</h3>
      <p className="settings-hint">
        The most frequent source tags on works that still need a category. Click one to start a
        keyword rule from it.
      </p>
      <div className="settings-common-tags">
        {(commonTags ?? []).map((tag) => (
          <button
            type="button"
            key={`${tag.source}:${tag.label}`}
            className="settings-common-tag"
            onClick={() =>
              setKeywordDraft({ ...keywordDraft, keyword: tag.label, source: tag.source })
            }
            title="Use as keyword"
          >
            <SourceTagPill tag={tag} />
            <span className="chip-count">{tag.count}</span>
          </button>
        ))}
        {commonTags && commonTags.length === 0 && (
          <p className="settings-hint">Every tag on unclassified works already has a keyword.</p>
        )}
      </div>
    </section>
  );
}

function WorkTypeRuleRow({
  rule,
  onSave,
  onDelete,
}: {
  rule: WorkTypeRule;
  onSave: (rule: WorkTypeRule) => Promise<unknown>;
  onDelete: () => Promise<unknown>;
}) {
  const [code, setCode] = useState(rule.code ?? "");
  const [confident, setConfident] = useState(rule.confident);
  const dirty = code !== (rule.code ?? "") || confident !== rule.confident;
  return (
    <tr>
      <td>{SOURCE_LABELS[rule.source]}</td>
      <td>
        <code>{rule.workType}</code>
      </td>
      <td>
        <input
          className="settings-rule-code"
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder="by tags"
          list="genre-code-options-settings"
        />
      </td>
      <td>
        <input
          type="checkbox"
          checked={confident}
          onChange={(e) => setConfident(e.target.checked)}
          aria-label="Confident"
        />
      </td>
      <td className="settings-row-actions">
        {dirty && (
          <button
            type="button"
            onClick={() => void onSave({ ...rule, code: code || null, confident })}
          >
            Save
          </button>
        )}
        <button
          type="button"
          className="settings-delete"
          onClick={() => void onDelete()}
          aria-label={`Remove rule ${rule.workType}`}
        >
          ×
        </button>
      </td>
    </tr>
  );
}

function CodeSettings() {
  const vocab = useVocabulary();
  const queryClient = useQueryClient();
  const [error, setError] = useState<string | null>(null);

  const refresh = () => queryClient.invalidateQueries({ queryKey: VOCABULARY_KEY });

  const handleDelete = async (code: string) => {
    setError(null);
    try {
      await deleteCode(code);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  if (!vocab) return <section className="settings-section">Loading codes…</section>;

  return (
    <section className="settings-section">
      <header>
        <h2>Genre codes</h2>
        <p>
          The closed list from the forum rules. T- and X- variants are accepted automatically for
          codes marked as animation codes.
        </p>
      </header>
      {error && <p className="settings-error">{error}</p>}
      <div className="settings-code-groups">
        {vocab.categories.map((category) => (
          <div key={category.id} className="settings-code-group">
            <h3>{category.folderName}</h3>
            <ul>
              {vocab.codes
                .filter((c) => c.categoryId === category.id)
                .map((c) => (
                  <li key={c.code}>
                    <code>{c.code}</code>
                    <span>{c.label}</span>
                    {c.acceptsPrefix && <small title="T- and X- variants allowed">T-/X-</small>}
                    {c.aliases.length > 0 && <small>= {c.aliases.join(", ")}</small>}
                    <button
                      type="button"
                      className="settings-delete"
                      onClick={() => void handleDelete(c.code)}
                      title="Remove this code (only when no game uses it)"
                      aria-label={`Remove ${c.code}`}
                    >
                      ×
                    </button>
                  </li>
                ))}
            </ul>
          </div>
        ))}
      </div>
      <AddCodeForm categories={vocab.categories} onSaved={refresh} />
    </section>
  );
}

function AddCodeForm({
  categories,
  onSaved,
}: {
  categories: CategoryDef[];
  onSaved: () => Promise<unknown>;
}) {
  const [code, setCode] = useState("");
  const [label, setLabel] = useState("");
  const [categoryId, setCategoryId] = useState(categories[0]?.id ?? "");
  const [acceptsPrefix, setAcceptsPrefix] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await upsertCode({ code, label, categoryId, acceptsPrefix });
      setCode("");
      setLabel("");
      setAcceptsPrefix(false);
      await onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <form className="settings-add-code" onSubmit={submit}>
      <input value={code} onChange={(e) => setCode(e.target.value)} placeholder="Code" required />
      <input value={label} onChange={(e) => setLabel(e.target.value)} placeholder="Label" required />
      <select value={categoryId} onChange={(e) => setCategoryId(e.target.value)}>
        {categories.map((c) => (
          <option key={c.id} value={c.id}>
            {c.folderName}
          </option>
        ))}
      </select>
      <label>
        <input
          type="checkbox"
          checked={acceptsPrefix}
          onChange={(e) => setAcceptsPrefix(e.target.checked)}
        />
        T-/X- variants
      </label>
      <button type="submit">Add or update code</button>
      {error && <p className="settings-error">{error}</p>}
    </form>
  );
}

function RefetchSettings() {
  const queryClient = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const status = useQuery({
    queryKey: ["refetch-status"],
    queryFn: fetchRefetchStatus,
    refetchInterval: (q) => (q.state.data?.running ? 2000 : false),
  }).data;

  const start = async () => {
    setError(null);
    try {
      await startRefetch();
      await queryClient.invalidateQueries({ queryKey: ["refetch-status"] });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <section className="settings-section">
      <header>
        <h2>Source re-fetch</h2>
        <p>
          Fetches every linked work again, one request per second, and stores each source's tags
          and work type separately. Titles, circles, descriptions and images are not touched.
        </p>
      </header>
      {status && (status.running || status.finishedAt) && (
        <p className="settings-progress">
          {status.running ? "Running" : "Finished"}: {status.done} / {status.total}
          {status.failed > 0 && `, ${status.failed} failed`}
          {status.lastError && <small> · last error: {status.lastError}</small>}
        </p>
      )}
      <button type="button" onClick={start} disabled={status?.running}>
        {status?.running ? "Re-fetch running…" : "Start re-fetch"}
      </button>
      {error && <p className="settings-error">{error}</p>}
    </section>
  );
}

function formatSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

/** Read-only list of what the scanner ignores, so it can be handled by hand. */
function OtherFilesReportSection() {
  const [report, setReport] = useState<OtherFilesReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const build = async () => {
    setBusy(true);
    setError(null);
    try {
      setReport(await fetchOtherFiles());
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="settings-section">
      <header>
        <h2>Files grimoire does not move</h2>
        <p>
          Everything under the library root that is not a zip or rar. A folder with no archive
          anywhere inside is shown as one line. Nothing here is changed by grimoire.
        </p>
      </header>
      <button type="button" onClick={build} disabled={busy}>
        {busy ? "Walking the library…" : report ? "Rebuild report" : "Build report"}
      </button>
      {error && <p className="settings-error">{error}</p>}
      {report && (
        <>
          <p className="settings-progress">
            {report.entries.length} entries, {report.totalFiles} files,{" "}
            {formatSize(report.totalBytes)}
          </p>
          <table className="settings-table settings-table-wide">
            <thead>
              <tr>
                <th>Path</th>
                <th>Files</th>
                <th>Size</th>
              </tr>
            </thead>
            <tbody>
              {report.entries.map((e) => (
                <tr key={e.path}>
                  <td>
                    <code>
                      {e.path}
                      {e.isDir ? "/" : ""}
                    </code>
                  </td>
                  <td>{e.files}</td>
                  <td>{formatSize(e.bytes)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </section>
  );
}
