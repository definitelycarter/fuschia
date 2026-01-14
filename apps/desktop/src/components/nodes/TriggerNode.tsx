import { memo } from "react";
import { type NodeProps } from "@xyflow/react";
import { Zap } from "lucide-react";
import { SourceHandle } from "./SourceHandle";

export interface TriggerNodeData {
  label: string;
  triggerType?: "manual" | "poll" | "webhook";
}

function TriggerNodeComponent({
  data,
  selected,
}: NodeProps & { data: TriggerNodeData }) {
  return (
    <div
      className={`
        relative px-4 py-3 min-w-[180px] rounded-md
        bg-[var(--color-bg-surface)] border border-[var(--color-border-default)]
        transition-all duration-150
        ${selected ? "ring-2 ring-primary-500 border-primary-500" : ""}
        hover:shadow-md
      `}
    >
      {/* Accent bar - fuchsia for triggers */}
      <div className="absolute top-0 left-0 right-0 h-1 rounded-t-md bg-primary-500" />

      {/* Content */}
      <div className="flex items-center gap-2">
        <span className="text-primary-500">
          <Zap size={16} />
        </span>
        <span className="text-sm font-medium text-[var(--color-text-primary)]">
          {data.label}
        </span>
      </div>

      {/* Trigger type badge */}
      {data.triggerType && (
        <div className="mt-2">
          <span className="text-xs px-2 py-0.5 rounded-full bg-primary-500/10 text-primary-500">
            {data.triggerType}
          </span>
        </div>
      )}

      {/* Source handle only - triggers are entry points */}
      <SourceHandle accent />
    </div>
  );
}

export const TriggerNode = memo(TriggerNodeComponent);
