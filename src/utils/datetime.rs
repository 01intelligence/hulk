use chrono::Utc;

pub type DateTime = chrono::DateTime<Utc>;

pub fn now() -> DateTime {
    Utc::now()
}
