use time::format_description::well_known::Rfc3339;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

pub const DATE: &[BorrowedFormatItem<'static>] = format_description!("[year]-[month]-[day]");

pub const DATE_MINUTES: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]");

pub const DATE_SECONDS_UTC: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]:[second] UTC");

pub fn format_utc(timestamp: OffsetDateTime, format: &[BorrowedFormatItem<'_>]) -> String {
    timestamp
        .to_offset(UtcOffset::UTC)
        .format(format)
        .unwrap_or_default()
}

pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default()
}

pub fn parse_rfc3339(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

pub fn unix_milliseconds(timestamp: OffsetDateTime) -> i64 {
    timestamp.unix_timestamp() * 1000 + i64::from(timestamp.millisecond())
}
