import { useState } from "react";

interface InlineTextProps {
  value: string | null;
  onSave: (next: string) => Promise<void>;
  placeholder?: string;
  className?: string;
  multiline?: boolean;
}

/// Click to edit. Enter commits, Esc cancels. Blur commits unless the user
/// pressed Esc first. Submitting an empty string is allowed — the API layer
/// treats it as "clear" (so the user can blank out a wrong VN label without
/// inventing a placeholder).
export function InlineText({
  value,
  onSave,
  placeholder,
  className,
  multiline,
}: InlineTextProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value ?? "");
  const [saving, setSaving] = useState(false);
  const [cancelled, setCancelled] = useState(false);

  const start = () => {
    setDraft(value ?? "");
    setCancelled(false);
    setEditing(true);
  };

  const commit = async () => {
    if (cancelled) {
      setEditing(false);
      return;
    }
    if (draft === (value ?? "")) {
      setEditing(false);
      return;
    }
    setSaving(true);
    try {
      await onSave(draft);
      setEditing(false);
    } catch (e) {
      console.error("inline edit save failed", e);
    } finally {
      setSaving(false);
    }
  };

  const cancel = () => {
    setCancelled(true);
    setDraft(value ?? "");
    setEditing(false);
  };

  if (editing) {
    const common = {
      autoFocus: true,
      className: `inline-edit-input ${className ?? ""}`,
      value: draft,
      disabled: saving,
      onChange: (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) =>
        setDraft(e.target.value),
      onBlur: commit,
      onKeyDown: (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && !multiline) {
          e.preventDefault();
          void commit();
        }
        if (e.key === "Escape") {
          e.preventDefault();
          cancel();
        }
      },
    };
    return multiline ? <textarea rows={3} {...common} /> : <input type="text" {...common} />;
  }

  return (
    <span
      className={`inline-edit ${className ?? ""}`}
      onClick={start}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          start();
        }
      }}
      title="Click to edit"
    >
      {value ? value : <em className="inline-edit-placeholder">{placeholder ?? "—"}</em>}
    </span>
  );
}
