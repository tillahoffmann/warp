# Dock icon badge for terminals that rang the bell

## Summary

Show a numeric badge on the macOS dock icon indicating how many terminals
have rung the terminal bell and have not yet been viewed by the user. This
gives a persistent, glanceable count to complement the existing dock icon
bounce, which is transient and conveys no count.

## Problem

When a terminal emits the bell character (`\a`, ASCII `0x07`), Warp asks the
OS for user attention, which bounces the dock icon on macOS. This is the
primary way Warp signals "a terminal wants your attention" while the app is
in the background — for example, a long-running command finishing, or an
agent such as Claude Code reaching a permission prompt or completing a task
(Claude Code rings the bell in those cases).

The bounce has two shortcomings:

1. **It is transient.** The icon bounces once and then stops. A user who is
   away from their machine when the bounce happens has no lasting indication
   that something needs attention.
2. **It carries no count.** When several terminals ring the bell across
   multiple tabs and windows, the bounce looks identical to a single bell.
   The user cannot tell, without clicking through every tab, how many
   terminals are waiting.

Users who run several terminals in parallel — increasingly common with
multiple concurrent agent sessions — have asked for a clearer at-a-glance
signal (see Related issues below).

## Goals

- Display a count on the macOS dock icon of terminals that have rung the
  bell and have not yet been viewed.
- Keep the badge perfectly consistent with the existing dock bounce: the
  same event (the terminal bell) drives both, so the badge never disagrees
  with what the bounce already taught users to expect.
- Clear a terminal from the count as soon as the user views it.
- Never leak a stale count (e.g. when a terminal that rang the bell is
  closed before being viewed).

## Non-goals

- **A "needs input" badge.** The terminal bell does not distinguish "agent
  needs input" from "command finished" — it is just a bell. This badge
  counts bells, exactly like the bounce. A semantically richer "agent is
  blocked on input" indicator is a separate feature and is explicitly out
  of scope.
- **Per-tab visual bell indicators.** Surfacing bell state on individual
  tabs (vs. the dock icon) is out of scope; it is tracked separately and
  noted as a future direction in the terminal bell handler.
- **Badging every long-running command.** Commands that complete without
  emitting a bell do not badge — consistent with the bounce. Warp's
  separate long-running-command desktop notifications are unaffected.
- **Windows / Linux taskbar badges.** The behavior is specified for macOS.
  Other platforms get a no-op; a taskbar equivalent can be added later.
- **A settings toggle.** The dock bounce on bell currently has no toggle;
  the badge mirrors it and likewise has none. If a toggle is desired later,
  it should gate the bounce and the badge together (see Open questions).

## Behavior

A terminal is **counted** when it rings the bell while the user is not
looking at it, and **uncounted** when the user views it or it goes away.

| Situation when the bell rings                         | Counted? |
|--------------------------------------------------------|----------|
| Warp is not the frontmost application                  | Yes      |
| Warp is frontmost, but the terminal is in another tab  | Yes      |
| Warp is frontmost and the terminal is visible          | No       |

A counted terminal is **cleared** when:

- the user focuses (views) that terminal, or
- the user re-activates Warp while that terminal is the visible one, or
- the terminal's shell exits (so the count cannot reference a dead terminal).

The badge shows the total number of counted terminals across all windows.
When the count is zero, no badge is shown. The badge updates live as
terminals ring the bell and as the user works through them.

The "Warp frontmost and terminal visible → not counted" rule mirrors macOS,
which suppresses the dock bounce while the app is frontmost: if the user is
already looking at the terminal, neither the bounce nor the badge fires.

## Related issues

- #8851 — "Badge notification on tabs when keyboard input is needed"
  (per-*tab* badge; related but distinct surface)
- #8717 — "Completion queue / priority badges for parallel Cloud Code tabs"
  (per-*tab* ordering badges; related but distinct surface)

This spec covers the **dock icon** badge specifically; neither issue above
proposes a dock badge.

## Open questions

- Should the dock bounce and this badge share a future settings toggle
  (e.g. `terminal.dock_attention_on_bell`)? Today neither is configurable.
- Should a Windows/Linux taskbar equivalent be in scope for a follow-up?
