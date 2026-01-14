import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { Box, Ban, Circle, Clock, Check, X } from "lucide-react";
import { SourceHandle } from "./SourceHandle";

export interface ComponentNodeData {
  label: string;
  componentName?: string;
  status?: "idle" | "running" | "success" | "error";
  critical?: boolean;
  timeoutMs?: number;
}

function ComponentNodeComponent({
  data,
  selected,
}: NodeProps & { data: ComponentNodeData }) {
  // Runtime status styles (for the node ring)
  const statusRing = {
    idle: "",
    running: "animate-pulse ring-2 ring-secondary-400",
    success: "",
    error: "",
  };

  // Runtime status indicator icon in corner
  const StatusIcon = () => {
    if (!data.status || data.status === "idle") return null;

    if (data.status === "running") {
      return (
        <div className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-secondary-400 flex items-center justify-center">
          <div className="w-2 h-2 rounded-full bg-white animate-pulse" />
        </div>
      );
    }

    if (data.status === "success") {
      return (
        <div className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-emerald-500 flex items-center justify-center">
          <Check size={10} className="text-white" strokeWidth={3} />
        </div>
      );
    }

    if (data.status === "error") {
      // Use orange for non-critical failures, red for critical
      const bgColor = data.critical ? "bg-red-500" : "bg-amber-500";
      return (
        <div
          className={`absolute -top-1 -right-1 w-4 h-4 rounded-full ${bgColor} flex items-center justify-center`}
        >
          <X size={10} className="text-white" strokeWidth={3} />
        </div>
      );
    }

    return null;
  };

  // Format timeout for display
  const formatTimeout = (ms: number) => {
    if (ms >= 60000) return `${ms / 60000}m`;
    if (ms >= 1000) return `${ms / 1000}s`;
    return `${ms}ms`;
  };

  const hasAttributes = data.critical !== undefined || data.timeoutMs;

  return (
    <div
      className={`
        relative px-4 py-3 min-w-[180px] rounded-md
        bg-[var(--color-bg-surface)] border border-[var(--color-border-default)]
        transition-all duration-150
        ${selected ? "ring-2 ring-primary-500 border-primary-500" : ""}
        ${data.status ? statusRing[data.status] : ""}
        hover:shadow-md
      `}
    >
      <StatusIcon />

      {/* Content */}
      <div className="flex items-center gap-2">
        <span className="text-[var(--color-text-muted)]">
          <Box size={16} />
        </span>
        <span className="text-sm font-medium text-[var(--color-text-primary)]">
          {data.label}
        </span>
      </div>

      {/* Component name subtitle */}
      {data.componentName && (
        <div className="mt-1">
          <span className="text-xs text-[var(--color-text-muted)] font-mono">
            {data.componentName}
          </span>
        </div>
      )}

      {/* Attribute badges */}
      {hasAttributes && (
        <div className="mt-2 pt-2 border-t border-[var(--color-border-subtle)] flex items-center gap-2">
          {data.critical !== undefined && (
            <div
              className="flex items-center gap-1 text-[var(--color-text-muted)]"
              title={
                data.critical
                  ? "Critical - workflow fails if this fails"
                  : "Non-critical - workflow continues if this fails"
              }
            >
              {data.critical ? <Ban size={12} /> : <Circle size={12} />}
            </div>
          )}
          {data.timeoutMs && (
            <div
              className="flex items-center gap-1 text-[var(--color-text-muted)]"
              title={`Timeout: ${formatTimeout(data.timeoutMs)}`}
            >
              <Clock size={12} />
              <span className="text-xs">{formatTimeout(data.timeoutMs)}</span>
            </div>
          )}
        </div>
      )}

      {/* Handles */}
      <Handle
        type="target"
        position={Position.Top}
        className="w-3! h-3! bg-[var(--color-bg-elevated)]! border-2! border-[var(--color-border-default)]!"
      />
      <SourceHandle />
    </div>
  );
}

export const ComponentNode = memo(ComponentNodeComponent);
