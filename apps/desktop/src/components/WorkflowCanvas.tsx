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
      label: "Send Confirmation",
      componentName: "email/send",
      status: "success",
      critical: false,
    },
  },
  // Right branch - payment processing chain (critical path)
  {
    id: "process_payment",
    type: "component",
    position: { x: 550, y: 120 },
    data: {
      label: "Process Payment",
      componentName: "payment/process",
      status: "success",
      critical: true,
      timeoutMs: 30000,
    },
  },
  {
    id: "allocate_inventory",
    type: "component",
    position: { x: 550, y: 240 },
    data: {
      label: "Allocate Inventory",
      componentName: "inventory/allocate",
      status: "success",
      critical: true,
    },
  },
  {
    id: "stage_for_shipping",
    type: "component",
    position: { x: 550, y: 360 },
    data: {
      label: "Stage for Shipping",
      componentName: "shipping/stage",
      status: "success",
      critical: true,
    },
  },
  {
    id: "send_shipping_email",
    type: "component",
    position: { x: 550, y: 480 },
    data: {
      label: "Send Shipping Email",
      componentName: "email/send",
      status: "error",
      critical: false,
    },
  },
  // Join node - waits for all branches
  {
    id: "join",
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
