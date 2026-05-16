# Tech Spec — Dock icon badge for terminals that rang the bell

## Problem

Implement the dock icon badge described in `PRODUCT.md`: a count on the
macOS dock icon of terminals that rang the terminal bell and have not yet
been viewed. The work has two parts:

1. A cross-platform way to set/clear a badge on the application icon, with
   a macOS implementation (and no-op elsewhere).
2. State tracking of belled-but-unviewed terminals, fed by the existing
   terminal bell event and cleared on view/exit.

## Relevant code

**Dock bounce (the existing signal this mirrors):**
- `app/src/terminal/view.rs` — `ModelEvent::Bell` handler. It rings the
  audible bell (gated by `TerminalSettings::use_audible_bell`) and calls
  `ctx.request_user_attention()` unconditionally. This is the *only* caller
  of `request_user_attention` on macOS.
- `crates/warpui_core/src/core/view/context.rs` — `ViewContext::request_user_attention`
- `crates/warpui_core/src/core/app.rs` — `AppContext::request_user_attention`
- `crates/warpui_core/src/platform/mod.rs` — `Delegate` trait
- `crates/warpui/src/platform/mac/delegate.rs` — macOS `AppDelegate`, which
  implements `request_user_attention` via `NSApp requestUserAttention:`.

**Notification model (the natural home for the new state):**
- `app/src/ai/agent_management/agent_management_model.rs` — `AgentNotificationsModel`,
  a singleton model that already subscribes to terminal/agent events and
  already holds the `is_terminal_view_visible(EntityId, &AppContext)` helper.

**Focus / activation signals (used to clear the count):**
- `app/src/workspace/view.rs` — the terminal-focus-change handler calls
  `AgentNotificationsModel::handle(ctx).update(...)` →
  `mark_items_from_terminal_view_read`. This handler is invoked both on a
  pane focus change *and* on app re-activation while the focused pane is
  unchanged (`handle_window_state_change` detects `app_became_active`).
- `crates/warpui_core/src/windowing/state.rs` — `WindowManager::app_is_active()`.

**Terminal lifecycle:**
- `app/src/terminal/view.rs` — `ModelEvent::Exit` handler (shell exited;
  also fires on tab close, since closing a tab terminates the pty).

## Design

### 1. Platform plumbing: `set_dock_badge`

Add a new method to the `Delegate` trait with a **default no-op body**, so
only macOS needs an override and all other platforms (test, integration,
headless, winit/Linux/Windows) inherit the no-op:

```rust
// crates/warpui_core/src/platform/mod.rs — trait Delegate
fn set_dock_badge(&self, _label: Option<String>) {}
```

macOS override sets the dock tile badge label (`None` clears it):

```rust
// crates/warpui/src/platform/mac/delegate.rs — impl Delegate for AppDelegate
fn set_dock_badge(&self, label: Option<String>) {
    unsafe {
        let dock_tile: id = msg_send![NSApp(), dockTile];
        let badge: id = match &label {
            Some(text) => make_nsstring(text),
            None => nil,
        };
        let () = msg_send![dock_tile, setBadgeLabel: badge];
    }
}
```

Wrappers thread it through the context types, mirroring how
`request_user_attention` is exposed:

- `AppContext::set_dock_badge(&self, Option<String>)` →
  `platform_delegate.set_dock_badge(..)`
- `ModelContext::set_dock_badge(&self, Option<String>)` →
  `self.app.set_dock_badge(..)`

`ModelContext` is the entry point needed, since the new state lives in a
model (below).

`NSDockTile.badgeLabel` is the standard AppKit API for the icon badge. It
does not require notification entitlements at the code level, though macOS
only *renders* the badge when the user has the app's "Badges" option
enabled in System Settings → Notifications — standard OS behavior, no
special handling required.

### 2. State: belled-but-unviewed terminals

`AgentNotificationsModel` gains:

```rust
belled_terminals: HashSet<EntityId>,   // terminal view ids
```

and three methods:

- `record_terminal_bell(terminal_view_id, ctx)` — called on every terminal
  bell. Returns early (does **not** count) when
  `ctx.windows().app_is_active() && is_terminal_view_visible(id, ctx)` —
  i.e. the user is genuinely looking at it. Otherwise inserts the id and
  recomputes the badge.
- `mark_terminal_viewed(terminal_view_id, ctx)` — removes the id and
  recomputes the badge. Idempotent.
- `update_dock_badge(ctx)` — private; sets the badge to
  `belled_terminals.len()` (or clears it at zero) via `ctx.set_dock_badge`.

The model is the right home: it is a singleton, already subscribed to
terminal/agent lifecycle, already reachable from both `terminal/view.rs`
and `workspace/view.rs` via `AgentNotificationsModel::handle(ctx)`, and
already owns `is_terminal_view_visible`.

### 3. Wiring

- **Record:** in the `terminal/view.rs` `ModelEvent::Bell` handler, right
  after the existing `ctx.request_user_attention()`, call
  `record_terminal_bell(ctx.view_id(), ..)`. Same event as the bounce, so
  the two are guaranteed consistent.
- **Clear on view:** in the `workspace/view.rs` focus-change handler, add a
  `mark_terminal_viewed(terminal_view_id, ..)` call alongside the existing
  `mark_items_from_terminal_view_read`. This handler already fires on both
  pane focus change and app re-activation, covering both clear triggers
  from `PRODUCT.md`.
- **Clear on exit:** in the `terminal/view.rs` `ModelEvent::Exit` handler,
  call `mark_terminal_viewed(ctx.view_id(), ..)`.

### Avoiding a stale count

There is no view-release hook in the UI framework, so the count cannot
rely on observing terminal destruction directly. Instead it is bounded by
`ModelEvent::Exit`: closing a tab terminates its pty, which emits `Exit`,
which clears the entry. A terminal cannot remain counted after it is gone.

## Alternatives considered

- **Drive the badge from `AgentNotificationsModel` notification items**
  (the agent "needs input" / "complete" mailbox) instead of the bell.
  Rejected: that set is disjoint from what bounces the dock — it would miss
  plain belling commands and depend on the Claude Code plugin — so the
  badge would routinely disagree with the bounce. The bell is the correct,
  consistent source.
- **Store bell state per `TerminalView` and recompute by traversing all
  workspaces.** Rejected as more invasive than a single `HashSet` in the
  existing singleton; the `HashSet` + `Exit` cleanup is simpler and leak-free.

## Testing

- Unit coverage for the `HashSet` insert/remove/count logic is thin because
  `record_terminal_bell` depends on `is_terminal_view_visible` /
  `app_is_active`, which need a running app context. The set arithmetic
  itself is trivial; the meaningful coverage is manual / integration:
  - Bell in a background tab (or with Warp backgrounded) → badge increments.
  - Bell in the visible terminal while Warp is frontmost → no change.
  - Focusing a counted terminal → badge decrements.
  - Re-activating Warp on a counted, visible terminal → badge decrements.
  - Closing a counted terminal's tab → badge decrements (no leak).
  - Multiple terminals across windows → badge sums correctly.
- No regression to the existing dock bounce (unchanged call site).

## Platform considerations

- macOS: full implementation via `NSDockTile.badgeLabel`.
- Linux / Windows / wasm / headless / tests: inherit the trait's no-op
  default. A Windows/Linux taskbar overlay equivalent is a possible
  follow-up (see `PRODUCT.md` Open questions).
