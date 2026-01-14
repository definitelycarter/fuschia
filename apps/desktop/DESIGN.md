# Desktop App Design

Design decisions and architecture for the Fuchsia desktop application.

## Technology Stack

| Layer | Choice | Notes |
|-------|--------|-------|
| Desktop framework | Tauri 2 | Rust backend, web frontend |
| Frontend framework | React 19 | With TypeScript strict mode |
| Styling | Tailwind CSS | Utility-first, no component libraries |
| Workflow canvas | React Flow | For node-based workflow editing |
| Icons | Lucide React | Tree-shakeable SVG icons |
| Fonts | Work Sans + JetBrains Mono | Sans + mono pairing |

## Design Principles

1. **Minimal dependencies** - Build custom components, avoid bloated UI libraries
2. **Performance first** - No CSS-in-JS runtime, tree-shake everything
3. **Accessibility** - Proper focus states, keyboard navigation
4. **Responsive** - Support various window sizes (800px to 1600px+)

## Theme

### Colors

**Primary: Fuchsia**
- Used for primary actions, active states, brand elements
- Dark mode: `fuchsia-500`
- Light mode: `fuchsia-600`

**Secondary: Cyan**
- Used for secondary actions, info states, accents
- Dark mode: `cyan-500`
- Light mode: `cyan-600`

**Backgrounds**

| Context | Dark Mode | Light Mode |
|---------|-----------|------------|
| Base | `slate-900` | `slate-50` |
| Surface | `slate-800` | `white` |
| Elevated | `slate-700` | `slate-100` |

**Text**

| Context | Dark Mode | Light Mode |
|---------|-----------|------------|
| Primary | `slate-50` | `slate-900` |
| Secondary | `slate-400` | `slate-600` |
| Muted | `slate-500` | `slate-500` |

**Borders**

| Context | Dark Mode | Light Mode |
|---------|-----------|------------|
| Default | `slate-700` | `slate-200` |
| Subtle | `slate-800` | `slate-100` |

**Semantic Colors**

| State | Color |
|-------|-------|
| Success | `emerald-500` |
| Warning | `amber-500` |
| Error | `red-500` |
| Info | `cyan-500` |

### Typography

**Font Families**
```css
--font-sans: 'Work Sans', system-ui, sans-serif;
--font-mono: 'JetBrains Mono', 'Fira Code', monospace;
```

**Scale** - Tailwind defaults (text-xs through text-4xl)

### Spacing & Sizing

- **Base unit:** 4px (Tailwind default)
- **Border radius:** 6px (`rounded-md`) for most elements
- **Transitions:** 150ms ease-out

### Shadows

- Dark mode: Subtle, low opacity
- Light mode: More pronounced, soft edges

### Focus States

- Ring style using primary color (fuchsia)
- `focus:ring-2 focus:ring-fuchsia-500 focus:ring-offset-2`
- Ring offset matches background

## Layout

### Breakpoints

| Name | Width | Use Case |
|------|-------|----------|
| Compact | 800px - 1200px | Small laptops, split screen |
| Default | 1200px - 1600px | Most laptops |
| Wide | 1600px+ | External monitors |

### App Structure

```
+------------------------------------------+
| Toolbar / Header                         |
+----------+-------------------------------+
|          |                               |
| Sidebar  |  Main Content                 |
| (nav)    |  (canvas / list / detail)    |
|          |                               |
+----------+-------------------------------+
| Status Bar (optional)                    |
+------------------------------------------+
```

- Sidebar collapsible at compact widths
- Main content adapts based on view

## Workflow Canvas (React Flow)

### Custom Node Types

| Node Type | Visual Treatment |
|-----------|------------------|
| Trigger | Fuchsia accent, lightning icon, entry point styling |
| Component | Slate background, subtle border, task icon |
| Join | Compact, diamond or merge icon |
| Loop | Cycle indicator, contains nested structure |

### Node Anatomy

```
+----------------------------------+
| [Icon]  Node Label        [Menu] |
+----------------------------------+
| Input preview or status          |
+----------------------------------+
     |
     v (handle)
```

### Interaction States

- **Default:** Subtle border
- **Hover:** Elevated shadow, brighter border
- **Selected:** Fuchsia ring/border
- **Running:** Pulse animation
- **Success:** Emerald indicator
- **Error:** Red indicator

## Component Patterns

### Buttons

- **Primary:** Fuchsia background, white text
- **Secondary:** Transparent, fuchsia text/border
- **Ghost:** Transparent, slate text, hover background

### Inputs

- Slate background (darker than surface)
- Subtle border, focus ring on interaction
- Monospace font for code inputs

### Cards/Panels

- Surface background
- Subtle border
- Consistent padding (p-4)

## Icons

Using Lucide React. Import individually for tree-shaking:

```tsx
import { Play, Settings, Workflow } from 'lucide-react'
```

Common icons:
- Workflow: `Workflow`
- Trigger: `Zap`
- Component: `Box`
- Join: `GitMerge`
- Loop: `Repeat`
- Play/Run: `Play`
- Settings: `Settings`
- Add: `Plus`

## Node Interactions

### Adding Nodes

Users add nodes by clicking the `+` button that appears on hover below a node's source handle. This opens a drawer from the right side.

**Flow:**
1. Click `+` on any node → drawer slides in
2. Drawer shows component picker (hierarchical: component → actions)
3. Select an action → node created, edge connected, drawer closes
4. New node shows warning state (unconfigured)
5. Click node later to configure in detail panel

**Design decisions:**
- Drawer instead of popover - more room for hierarchy and search
- Drawer is scoped to main content area (doesn't cover header/footer)
- No auto-open of detail panel on add - one action at a time
- Supports "sketch first, configure later" workflow

### New Workflows

New workflows start with a Manual trigger node pre-placed. Users can change the trigger type but cannot add additional triggers (for now).

### Component Picker

The drawer shows installed components from the registry, not internal node types. Users see "Google Sheets", "Send Email", etc. - not "Component".

**Structure:**
- Components grouped by service with section headers
- Each component expands to show available actions (tasks)
- Flow control (Join, Loop) shown in separate section
- Search filters across all items

### Node States (Runtime)

Status indicators appear as small icons in the top-right corner of nodes.

| State | Visual | Meaning |
|-------|--------|---------|
| Idle | No indicator | Ready to run |
| Running | Pulsing cyan dot | Currently executing |
| Success | Green checkmark | Completed successfully |
| Error (non-critical) | Amber X | Failed, but workflow continued |
| Error (critical) | Red X | Failed, workflow stopped |

### Node Attributes

Attribute badges appear at the bottom of nodes, separated by a subtle border.

| Attribute | Icon | Meaning |
|-----------|------|---------|
| Critical | `Ban` (⊘) | Workflow fails if this node fails |
| Non-critical | `Circle` (○) | Workflow continues if this node fails |
| Timeout | `Clock` + duration | Node timeout (e.g., "30s") |

Icons use muted color (`--color-text-muted`) to avoid competing with status indicators.

## Edge Connection Rules

Most nodes should only have one incoming edge. Join nodes are the exception - they're designed to merge multiple branches.

| Node Type | Incoming Edges | Outgoing Edges |
|-----------|----------------|----------------|
| Trigger | 0 (entry point) | 1+ |
| Component | 1 | 1+ |
| Join | 2+ | 1+ |
| Loop | 1 | 1+ |

**Why:** Each node's input templates reference its single upstream node's output. Multiple incoming edges would be ambiguous - which upstream does `{{ field }}` refer to? Join nodes solve this by keying context by upstream node ID: `{{ fetch_user.email }}`.

### Handling Invalid Connections

When a user tries to connect a second edge to a non-Join node:

1. **Show warning** - "This node already has an incoming connection"
2. **Offer fix** - "Insert Join node" button
3. **On click** - automatically insert a Join node between the sources and target

**Power user setting:** "Auto-insert Join nodes" (opt-in)
- When enabled, skip the warning and automatically insert Join nodes
- Default: off (guide the user first)

## Outstanding UI Work

- [ ] Component picker content in drawer
- [ ] Actually adding nodes and edges on selection
- [ ] Detail panel for node configuration
- [ ] Warning state for unconfigured nodes
- [ ] Search in component picker
- [ ] Keyboard navigation (Tab, Enter, Escape)
- [ ] Connect to Rust backend via Tauri commands
- [ ] Enforce edge connection rules (single incoming edge for non-Join nodes)
- [ ] "Insert Join node" prompt when invalid connection attempted
- [ ] Settings panel with "Auto-insert Join nodes" option
