import { useCallback } from "react";
import {
  ReactFlow,
  Controls,
  Background,
  BackgroundVariant,
  useNodesState,
  useEdgesState,
  addEdge,
  reconnectEdge,
  type Connection,
  type Edge,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import { TriggerNode, ComponentNode, JoinNode } from "./nodes";

const nodeTypes = {
  trigger: TriggerNode,
  component: ComponentNode,
  join: JoinNode,
};

// Order Processing Workflow (Use Case #4)
// Demonstrates parallel branches, sequential chains, and join nodes
const initialNodes: Node[] = [
  // Trigger
  {
    id: "order_created",
    type: "trigger",
    position: { x: 300, y: 0 },
    data: { label: "Order Created", triggerType: "webhook" },
  },
  // Left branch - confirmation email (non-critical)
  {
    id: "send_order_confirmation",
    type: "component",
    position: { x: 50, y: 120 },
    data: {
      label: "Fetch User",
      componentName: "http/fetch",
      status: "success",
    },
  },
  {
    id: "component-2",
    type: "component",
    position: { x: 400, y: 180 },
    data: {
      label: "Get Config",
      componentName: "config/get",
      status: "success",
    },
  },
  {
    id: "join-1",
    type: "join",
    position: { x: 300, y: 600 },
    data: { label: "Join", strategy: "all" },
  },
  // Final node
  {
    id: "complete_order",
    type: "component",
    position: { x: 300, y: 720 },
    data: {
      label: "Complete Order",
      componentName: "order/complete",
      status: "success",
    },
  },
];

const initialEdges: Edge[] = [
  // From trigger - two parallel branches
  { id: "e1", source: "order_created", target: "send_order_confirmation" },
  { id: "e2", source: "order_created", target: "process_payment" },
  // Right branch chain
  { id: "e3", source: "process_payment", target: "allocate_inventory" },
  { id: "e4", source: "allocate_inventory", target: "stage_for_shipping" },
  { id: "e5", source: "stage_for_shipping", target: "send_shipping_email" },
  // Into join
  { id: "e6", source: "send_order_confirmation", target: "join" },
  { id: "e7", source: "stage_for_shipping", target: "join" },
  { id: "e8", source: "send_shipping_email", target: "join" },
  // Final
  { id: "e9", source: "join", target: "complete_order" },
];

export function WorkflowCanvas() {
  const [nodes, , onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges],
  );

  const onReconnect = useCallback(
    (oldEdge: Edge, newConnection: Connection) =>
      setEdges((eds) => reconnectEdge(oldEdge, newConnection, eds)),
    [setEdges],
  );

  return (
    <div className="w-full h-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onReconnect={onReconnect}
        nodeTypes={nodeTypes}
        fitView
        defaultEdgeOptions={{
          style: {
            stroke: "var(--color-border-default)",
            strokeWidth: 2,
          },
          type: "smoothstep",
        }}
        proOptions={{ hideAttribution: true }}
      >
        <Controls className="bg-[var(--color-bg-surface)]! border-[var(--color-border-default)]! shadow-md! [&>button]:bg-[var(--color-bg-surface)]! [&>button]:border-[var(--color-border-default)]! [&>button]:text-[var(--color-text-secondary)]! [&>button:hover]:bg-[var(--color-bg-elevated)]!" />
        <Background
          variant={BackgroundVariant.Dots}
          gap={20}
          size={1}
          color="var(--color-border-subtle)"
        />
      </ReactFlow>
    </div>
  );
}
