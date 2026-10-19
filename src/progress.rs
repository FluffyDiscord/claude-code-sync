use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use std::borrow::Cow;
use std::time::Duration;

const SPINNER_TICK_INTERVAL: Duration = Duration::from_millis(100);

pub fn bar(length: usize, message: impl Into<Cow<'static, str>>) -> ProgressBar {
    let style = ProgressStyle::with_template("  {msg} [{bar:30}] {pos}/{len} ({elapsed})")
        .expect("valid progress template")
        .progress_chars("=> ");

    ProgressBar::new(length as u64)
        .with_style(style)
        .with_message(message)
        .with_finish(ProgressFinish::AndClear)
}

pub fn while_spinning<T>(message: impl Into<Cow<'static, str>>, step: impl FnOnce() -> T) -> T {
    let style = ProgressStyle::with_template("  {spinner} {msg} ({elapsed})")
        .expect("valid spinner template");

    let spinner = ProgressBar::new_spinner()
        .with_style(style)
        .with_message(message)
        .with_finish(ProgressFinish::AndClear);
    spinner.enable_steady_tick(SPINNER_TICK_INTERVAL);

    step()
}
