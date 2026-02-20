import type { ReactNode } from "react";

export interface PanelProps {
  title: string;
  actions?: ReactNode;
  children: ReactNode;
  noPad?: boolean;
}

export function Panel({ title, actions, children, noPad = false }: PanelProps) {
  return (
    <div
      className="flex flex-col rounded-2xl"
      style={{
        background: "var(--surface)",
        border: "1px solid var(--border)",
        boxShadow: "0 2px 8px rgba(0,0,0,0.08)",
      }}
    >
      <div
        className="flex items-center justify-between px-5 py-3.5 shrink-0"
        style={{ borderBottom: "1px solid var(--border)" }}
      >
        <h2
          className="font-semibold text-sm"
          style={{ color: "var(--textPrimary)" }}
        >
          {title}
        </h2>
        {actions}
      </div>
      <div className={noPad ? "" : "p-5"}>{children}</div>
    </div>
  );
}
