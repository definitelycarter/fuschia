import { memo, type ReactNode } from "react";
import { Handle, Position } from "@xyflow/react";

export interface BaseNodeData extends Record<string, unknown> {
  label: string;
  icon?: ReactNode;
  status?: "idle" | "running" | "success" | "error";
}

interface BaseNodeProps {
  data: BaseNodeData;
  selected?: boolean;
  accentColor?: string;
  showTargetHandle?: boolean;
  showSourceHandle?: boolean;
}

function BaseNodeComponent({
  data,
  selected,
  accentColor,
  showTargetHandle = true,
  showSourceHandle = true,
}: BaseNodeProps) {
  const statusColors = {
    idle: "",
    running: "animate-pulse ring-2 ring-secondary-400",
    success: "ring-2 ring-emerald-500",
    error: "ring-2 ring-red-500",
  };

  const statusIndicator = {
    idle: null,
    running: "bg-secondary-400",
    success: "bg-emerald-500",
    error: "bg-red-500",
  };

  return (
    <div
      className={`
        relative px-4 py-3 min-w-[180px] rounded-md
        bg-[var(--color-bg-surface)] border border-[var(--color-border-default)]
        transition-all duration-150
        ${selected ? "ring-2 ring-primary-500 border-primary-500" : ""}
        ${data.status ? statusColors[data.status] : ""}
        hover:border-[var(--color-border-default)] hover:shadow-md
      `}
    >
      {/* Accent bar */}
      {accentColor && (
        <div
          className={`absolute top-0 left-0 right-0 h-1 rounded-t-md ${accentColor}`}
        />
      )}

      {/* Status indicator */}
      {data.status && data.status !== "idle" && (
        <div
          className={`absolute -top-1 -right-1 w-3 h-3 rounded-full ${statusIndicator[data.status]}`}
        />
      )}

      {/* Content */}
      <div className="flex items-center gap-2">
        {data.icon && (
          <span className="text-[var(--color-text-muted)]">{data.icon}</span>
        )}
        <span className="text-sm font-medium text-[var(--color-text-primary)]">
          {data.label}
        </span>
      </div>

      {/* Handles */}
      {showTargetHandle && (
        <Handle
          type="target"
          position={Position.Top}
          className="w-3! h-3! bg-[var(--color-bg-elevated)]! border-2! border-[var(--color-border-default)]!"
        />
      )}
      {showSourceHandle && (
        <Handle
          type="source"
          position={Position.Bottom}
          className="w-3! h-3! bg-[var(--color-bg-elevated)]! border-2! border-[var(--color-border-default)]!"
        />
      )}
    </div>
  );
}

export const BaseNode = memo(BaseNodeComponent);
