use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

pub fn make_spinner(message: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&[
                "\u{2800}", "\u{2801}", "\u{2809}", "\u{2819}", "\u{281b}", "\u{283b}", "\u{2839}",
                "\u{2838}", "\u{2830}", "\u{2820}", "\u{2800}", "\u{2713}",
            ]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(message.into());
    pb
}
