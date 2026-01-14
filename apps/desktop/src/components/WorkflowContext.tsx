import { createContext, useContext } from "react";

interface WorkflowContextValue {
  onAddNode: (sourceNodeId: string) => void;
}

export const WorkflowContext = createContext<WorkflowContextValue | null>(null);

export function useWorkflow() {
  const context = useContext(WorkflowContext);
  if (!context) {
    throw new Error("useWorkflow must be used within a WorkflowContext provider");
  }
  return context;
}
