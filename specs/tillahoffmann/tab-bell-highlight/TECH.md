# Tech Spec — Tab highlight for terminals that rang the bell

## Problem

Implement the per-tab bell highlight described in `PRODUCT.md`: when a
terminal rings the bell while not being viewed, ring and/or tint the tab(s)
containing it, on both the horizontal tab bar and the vertical tabs panel,
until the user views that terminal.

The work is in three parts:

1. Track which terminal views have rung the bell and not been viewed.
2. Map a tab to the terminals it contains, so a tab can ask "does any of my
   terminals have a pending bell?".
3. Apply the highlight in both tab-rendering paths.

## Relevant code

**Terminal bell + view/exit signals:**
- `app/src/terminal/view.rs` — `ModelEvent::Bell` handler (rings the
  audible bell, then calls `ctx.request_user_attention()` for the dock
  bounce) and `ModelEvent::Exit` handler (shell exited / tab closed).
- `app/src/workspace/view.rs` — terminal-focus-change handler; invokes
  `AgentNotificationsModel::handle(ctx).update(...)` on both pane focus
  change and app re-activation (`handle_window_state_change` detects
  `app_became_active`).
- `crates/warpui_core/src/windowing/state.rs` — `WindowManager::app_is_active()`.
- `app/src/ai/agent_management/agent_management_model.rs` —
  `AgentNotificationsModel`, a singleton model already subscribed to
  terminal/agent events, already holding the
  `is_terminal_view_visible(EntityId, &AppContext)` helper, and already
  reachable from tab-rendering code via `AgentNotificationsModel::handle`.

**Horizontal tab bar:**
- `app/src/tab.rs` — `TabComponent` (renders one tab), `TabData`
  (`tab.pane_group`), `TabStyles`.
- `tab.rs` ~line 1328 — `let (background_color, border_fill) = ...`
  computes the per-tab background and border. Two branches behind
  `FeatureFlag::NewTabStyling`. Applied at ~1550 (`with_background`) and
  ~1557 / ~1562 (`with_border(... with_border_fill(border_fill))`).
- `tab.rs` ~line 661 — `enum Indicator` and the mutually-exclusive
  indicator-priority cascade (~833-848). **Not extended by this feature**
  (see Design).

**Vertical tabs panel:**
- `app/src/workspace/view/vertical_tabs.rs` — `pane_row_background()`
  (~line 272) returns the per-row `Option<ThemeFill>`; `render_pane_row*`
  functions render rows.

**Tab → terminals:**
- `app/src/pane_group/mod.rs` — `PaneGroup`; has methods that fold over
  the group's panes/sessions (e.g. `focused_shell_indicator_type`,
  `is_terminal_pane_being_shared`, `most_recent_pane_state`).

## Design

### 1. Bell-tracking state

`AgentNotificationsModel` tracks terminal views that have rung the bell and
not yet been viewed:

```rust
belled_terminals: HashSet<EntityId>,   // terminal view ids
```

with methods:

- `record_terminal_bell(terminal_view_id, ctx)` — inserts the id, *unless*
  the user is actively looking at it
  (`ctx.windows().app_is_active() && is_terminal_view_visible(id, ctx)`),
  mirroring macOS suppressing the dock bounce while the app is frontmost.
- `mark_terminal_viewed(terminal_view_id, ctx)` — removes the id.
- `is_terminal_belled(terminal_view_id) -> bool` — read accessor used by
  tab rendering.

The model emits an event (e.g. an existing/new `AgentManagementEvent`
variant) when the set changes so tab views re-render.

Wiring:

- **Record:** the `terminal/view.rs` `ModelEvent::Bell` handler calls
  `record_terminal_bell(ctx.view_id(), ..)` right after the existing
  `request_user_attention()` — same event as the bounce.
- **Clear on view:** the `workspace/view.rs` focus-change handler calls
  `mark_terminal_viewed(..)`. That handler already fires on both pane focus
  change and app re-activation, covering both clear triggers.
- **Clear on exit:** the `terminal/view.rs` `ModelEvent::Exit` handler
  calls `mark_terminal_viewed(ctx.view_id(), ..)`. Closing a tab terminates
  its pty, which emits `Exit`, so a closed terminal cannot stay tracked —
  this bounds the set with no view-release hook needed.

### 2. Tab → "has a belled terminal"

Add a `PaneGroup` helper that returns whether any terminal view in the
group is currently belled, e.g.:

```rust
// app/src/pane_group/mod.rs
pub fn has_belled_terminal(&self, ctx: &AppContext) -> bool
```

It folds over the group's terminal sessions and checks each terminal view
id against `AgentNotificationsModel`'s `is_terminal_belled`. This handles
split panes (highlight if any pane qualifies).

### 3. Rendering

**Horizontal tab bar (`tab.rs`).** Extend the existing
`(background_color, border_fill)` computation: when
`tab.pane_group.has_belled_terminal(ctx)` is true, override `border_fill`
(and/or `background_color`) with the bell-attention treatment. Both
`FeatureFlag::NewTabStyling` branches must be handled. No new `Indicator`
variant — the highlight is a border/background, orthogonal to the status
icon, so a belled tab still shows its agent/shell indicator.

**Vertical tabs panel (`vertical_tabs.rs`).** Extend `pane_row_background()`
(and/or the row border) to apply the bell treatment when the row's pane
group has a belled terminal, slotting in alongside the existing
selected/hover/tab-color cases.

**Color.** Per Warp UI guidelines, the highlight color must come from a
theme accessor — no hard-coded `ColorU`. The exact token is a design
decision (see `PRODUCT.md` open questions); the error color is semantically
wrong and the accent color collides with active-tab styling.

### Why not a new `Indicator` variant

The horizontal tab bar resolves a *single* indicator icon from a priority
cascade. Adding `Indicator::Bell` would suppress whatever icon
(agent/shell/error) was already shown. A ring/background is orthogonal and
preserves the existing icon — and matches the vertical panel, which has no
single-icon constraint.

## Feature flag

Ship behind a new `FeatureFlag` (e.g. `TabBellHighlight`) per the repo's
`add-feature-flag` process, so the visual can be rolled out gradually and
disabled if it interacts badly with `NewTabStyling`.

## Testing

- The `belled_terminals` set logic (`record`/`mark_viewed`/`is_belled`) is
  unit-testable; the visibility/`app_is_active` guard needs a running app
  context, so end-to-end coverage is manual/integration:
  - Bell in a background tab (or with Warp backgrounded) → that tab gains
    the highlight in both the horizontal bar and the vertical panel.
  - Bell in the visible terminal while Warp is frontmost → no highlight.
  - Focusing the tab, or re-activating Warp on it → highlight clears.
  - Closing a highlighted tab → no stale state.
  - Split pane: a bell in one pane highlights the tab; viewing the tab
    clears it.
  - The tab's existing status icon and active-tab styling are unaffected.

## Platform considerations

The highlight is rendered through the shared, cross-platform tab components
(`tab.rs`, `vertical_tabs.rs`), so it applies on all desktop platforms. The
bell event and the `app_is_active` guard are likewise cross-platform.
