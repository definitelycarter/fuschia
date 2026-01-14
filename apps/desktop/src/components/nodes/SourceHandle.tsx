import { memo, useState } from "react";
import { Handle, Position, useNodeId } from "@xyflow/react";
import { Plus } from "lucide-react";
import { useWorkflow } from "../WorkflowContext";

interface SourceHandleProps {
  accent?: boolean;
}

function SourceHandleComponent({ accent = false }: SourceHandleProps) {
  const [hovered, setHovered] = useState(false);
  const nodeId = useNodeId();
  const { onAddNode } = useWorkflow();

  return (
    <div
      className="absolute -bottom-3 left-1/2 -translate-x-1/2 flex flex-col items-center"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <Handle
        type="source"
        position={Position.Bottom}
        className={`
          w-3! h-3! border-2!
          ${
            accent
              ? "bg-primary-500! border-[var(--color-bg-surface)]!"
              : "bg-[var(--color-bg-elevated)]! border-[var(--color-border-default)]!"
          }
        `}
      />

      {/* Add button - appears on hover */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          if (nodeId) {
            onAddNode(nodeId);
          }
        }}
        className={`
          mt-1 w-5 h-5 rounded-full flex items-center justify-center
          bg-primary-500 text-white
          transition-all duration-150
          hover:bg-primary-600 hover:scale-110
          ${hovered ? "opacity-100 translate-y-0" : "opacity-0 -translate-y-1 pointer-events-none"}
        `}
      >
        <Plus size={12} strokeWidth={3} />
      </button>
    </div>
  );
}

export const SourceHandle = memo(SourceHandleComponent);
