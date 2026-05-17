//! Integration tests for terminal bell handling.

use warp::integration_testing::{
    step::new_step_with_default_assertions,
    terminal::{
        execute_command_for_single_terminal_in_tab, util::ExpectedExitStatus,
        wait_until_bootstrapped_single_pane_for_tab,
    },
};

use super::{new_builder, Builder};

/// Ringing the terminal bell in the *focused* terminal must not crash the app.
///
/// Regression test: bell-attention tracking recorded the bell from inside the
/// focused terminal view's own update and then resolved visibility by reading
/// that same view back, re-entering it and panicking with
/// "circular view reference for view type TerminalView".
pub fn test_bell_in_focused_terminal_does_not_crash() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        // `\a` is the bell byte; it fires while this terminal is the focused one.
        // The trailing newline avoids the shell's missing-newline (`%`) marker.
        .with_step(execute_command_for_single_terminal_in_tab(
            0,
            "printf 'rang\\a\\n'".to_string(),
            ExpectedExitStatus::Success,
            "rang".to_string(),
        ))
        .with_step(new_step_with_default_assertions(
            "App is still responsive after a bell in the focused terminal",
        ))
}
