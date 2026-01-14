import { memo } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { GitMerge } from "lucide-react";
import { SourceHandle } from "./SourceHandle";

export interface JoinNodeData {
  label: string;
  strategy?: "all" | "any";
}

function JoinNodeComponent({
  data,
  selected,
}: NodeProps & { data: JoinNodeData }) {
  return (
    <div
      className={`
        relative px-3 py-2 min-w-[120px] rounded-md
        bg-[var(--color-bg-surface)] border border-[var(--color-border-default)]
        transition-all duration-150
        ${selected ? "ring-2 ring-primary-500 border-primary-500" : ""}
        hover:shadow-md
      `}
    >
      {/* Content */}
      <div className="flex items-center gap-2">
        <span className="text-secondary-500">
          <GitMerge size={14} />
        </span>
        <span className="text-xs font-medium text-[var(--color-text-primary)]">
          {data.label}
        </span>
        {data.strategy && (
          <span className="text-xs px-1.5 py-0.5 rounded bg-secondary-500/10 text-secondary-500">
            {data.strategy}
          </span>
        )}
      </div>

      {/* Handles */}
      <Handle
        type="target"
        position={Position.Top}
        className="w-3! h-3! bg-secondary-500! border-2! border-[var(--color-bg-surface)]!"
      />
      <SourceHandle />
    </div>
  );
}

export const JoinNode = memo(JoinNodeComponent);
