# Tab highlight for terminals that rang the bell

## Summary

Highlight a tab — with a colored ring around it and/or a subtle background
tint — when a terminal in that tab rings the terminal bell and has not yet
been viewed. This tells the user *which* tab wants attention, complementing
the existing dock icon bounce, which is transient and conveys no location.

## Problem

When a terminal rings the bell (`\a`), Warp bounces the macOS dock icon to
request user attention. The bounce signals *that* a terminal wants
attention, but not *which* one. With several tabs and windows open, the
user still has to click through tabs to find the one that belled.

The terminal bell handler itself carries an explicit acknowledgement of
this gap:

```rust
// TODO(vorporeal): Remove this once we have a visual bell
// indicator in terminal tabs.
ctx.request_user_attention();
```

A per-tab visual bell indicator is a known, wanted-but-unbuilt feature.
Requests for per-tab bell indication are consolidated by the Warp team
under issue #8851 (issue #8782 "bells per tab" was closed as a duplicate
of it).

## Goals

- Visually highlight the tab(s) containing a terminal that rang the bell
  and has not yet been viewed.
- Apply consistently to both tab surfaces: the horizontal tab bar and the
  vertical tabs panel.
- Clear a tab's highlight as soon as the user views that terminal.
- Coexist with existing per-tab visuals (active-tab styling, tab color,
  status indicator icons) without replacing or obscuring them.

## Non-goals

- **A new "needs input" signal.** This reflects the terminal *bell*, not a
  semantic agent-input state. The bell does not distinguish "needs input"
  from "command finished" — out of scope.
- **Replacing the tab status-indicator icon.** The tab bar already shows a
  mutually-exclusive status icon (agent/shell/error/etc.). The bell
  highlight is deliberately a *ring/background*, orthogonal to that icon —
  see Design rationale.
- **Non-bell command completion.** Commands that finish without emitting a
  bell do not highlight a tab.
- **A settings toggle.** Consistent with the dock bounce, which has none.
- **Windows/Linux-specific work** beyond what the shared tab rendering
  already provides cross-platform.

## Behavior

A tab is **highlighted** when at least one terminal in it has rung the bell
and has not yet been viewed:

- The terminal rang the bell while the user was not looking at it (Warp not
  frontmost, or the terminal not visible).
- The highlight persists until the user views that terminal (focuses the
  tab, or re-activates Warp on it) or the terminal's shell exits.

A tab with split panes is highlighted if *any* of its terminals qualifies.
The highlight is removed the moment the qualifying terminal is viewed.

Because a terminal the user is currently looking at never qualifies, the
highlighted tab is never the active/visible tab — so the bell highlight
never competes with active-tab styling.

## Design rationale

- **Ring/background, not a status icon.** The horizontal tab bar shows a
  single, mutually-exclusive status indicator icon (agent status, shell
  type, error, sharing, …). Encoding the bell as another icon variant would
  *hide* whichever indicator was already there. A ring and/or background
  tint is orthogonal — a belled tab keeps showing its agent/shell icon and
  *also* reads as needing attention.
- **Reuse existing per-tab visual paths.** Both tab surfaces already apply
  per-tab borders and backgrounds (active-tab accent border, user-assigned
  tab color gradients, hover/drag backgrounds). The highlight should slot
  into those existing paths rather than introduce a new visual primitive.

## Related issues

- #8851 — per-tab bell/attention indication. This spec is a candidate
  implementation for that issue and should reference it.

## Open questions (likely need design mocks)

- **Exact visual treatment**: ring only, background tint only, or both?
  Ring thickness, corner treatment, and whether it animates once on the
  bell vs. appears statically. This should be settled with a design mock.
- **Color token**: must be a theme accessor (no hard-coded color). The
  error color (red) is semantically wrong; the accent color risks reading
  like the active tab. A dedicated attention/warning token — or accent at a
  distinguishing opacity — should be chosen by design.
- Should the existing `terminal.show_indicators` setting gate this, or is
  the bell highlight always on (like the bounce)?
- Should it ship behind a temporary rollout feature flag (recommended)?
