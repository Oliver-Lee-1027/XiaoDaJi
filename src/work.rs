//! 本地上下班记录。日期按本机时区，时长是自然经过时间而非考勤。
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, TimeZone};
use serde_json::{json, Value};
use std::{fs, io, path::PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Config {
    pub automatic: bool,
    pub enabled: bool,
    pub end_minutes: u32,
    pub weekdays: u8,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            automatic: true,
            enabled: false,
            end_minutes: 18 * 60 + 30,
            weekdays: 31,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Record {
    pub date: String,
    pub start: i64,
    pub end: Option<i64>,
    pub reminded: bool,
    pub snooze: Option<i64>,
}
#[derive(Clone, Debug)]
pub(crate) struct Work {
    pub config: Config,
    pub records: Vec<Record>,
    pub error: Option<String>,
    path: PathBuf,
}
pub(crate) fn moment(date: &str, clock: &str) -> Option<i64> {
    let clock = if clock.len() == 5 {
        format!("{clock}:00")
    } else {
        clock.to_owned()
    };
    let naive =
        NaiveDateTime::parse_from_str(&format!("{date} {clock}"), "%Y-%m-%d %H:%M:%S").ok()?;
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|time| time.timestamp())
}
pub(crate) fn parse_time(text: &str) -> Option<u32> {
    let bytes = text.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 2 || b.is_ascii_digit())
    {
        return None;
    }
    let h: u32 = text[..2].parse().ok()?;
    let m: u32 = text[3..].parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}
pub(crate) fn time_text(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}
pub(crate) fn clock(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.with_timezone(&Local).format("%H:%M").to_string())
        .unwrap_or_else(|| "—".into())
}
pub(crate) fn duration(seconds: i64) -> String {
    let minutes = seconds.max(0) / 60;
    format!("{} 小时 {} 分钟", minutes / 60, minutes % 60)
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl Work {
    pub fn load() -> Self {
        Self::load_at(crate::model::config_dir().join("work.json"))
    }
    pub fn load_at(path: PathBuf) -> Self {
        let mut work = Self {
            config: Config::default(),
            records: vec![],
            error: None,
            path,
        };
        if let Err(error) = work.read() {
            work.error = Some(format!("上下班记录读取失败，原文件未改动：{error}"));
        }
        work
    }
    fn read(&mut self) -> io::Result<()> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let v: Value = serde_json::from_slice(&bytes)?;
        if v["version"].as_u64() != Some(1) {
            return Err(invalid("不支持的数据版本"));
        }
        let c = &v["config"];
        let end = c["end_minutes"]
            .as_u64()
            .filter(|n| *n < 1440)
            .ok_or_else(|| invalid("下班时间无效"))?;
        let days = c["weekdays"]
            .as_u64()
            .filter(|n| *n > 0 && *n < 128)
            .ok_or_else(|| invalid("提醒日期无效"))?;
        let config = Config {
            automatic: c["automatic"]
                .as_bool()
                .ok_or_else(|| invalid("自动记录设置无效"))?,
            enabled: c["enabled"]
                .as_bool()
                .ok_or_else(|| invalid("提醒设置无效"))?,
            end_minutes: end as u32,
            weekdays: days as u8,
        };
        let mut records = Vec::<Record>::new();
        for row in v["records"]
            .as_array()
            .ok_or_else(|| invalid("记录列表无效"))?
        {
            let date = row["date"]
                .as_str()
                .ok_or_else(|| invalid("记录日期无效"))?;
            let start = row["start"]
                .as_i64()
                .ok_or_else(|| invalid("上班时间无效"))?;
            let day = chrono::DateTime::from_timestamp(start, 0)
                .ok_or_else(|| invalid("上班时间越界"))?
                .with_timezone(&Local)
                .date_naive();
            if day.to_string() != date || records.iter().any(|r| r.date == date) {
                return Err(invalid("记录日期不匹配或重复"));
            }
            let end = if row["end"].is_null() {
                None
            } else {
                Some(
                    row["end"]
                        .as_i64()
                        .filter(|n| *n >= start)
                        .ok_or_else(|| invalid("下班时间无效"))?,
                )
            };
            if let Some(end) = end {
                if chrono::DateTime::from_timestamp(end, 0)
                    .ok_or_else(|| invalid("下班时间越界"))?
                    .with_timezone(&Local)
                    .date_naive()
                    != day
                {
                    return Err(invalid("暂不支持跨天班次"));
                }
            }
            let snooze = if row["snooze"].is_null() {
                None
            } else {
                Some(
                    row["snooze"]
                        .as_i64()
                        .ok_or_else(|| invalid("延后提醒时间无效"))?,
                )
            };
            let reminded = row["reminded"]
                .as_bool()
                .ok_or_else(|| invalid("提醒状态无效"))?;
            records.push(Record {
                date: date.into(),
                start,
                end,
                reminded,
                snooze,
            });
        }
        records.sort_by(|a, b| a.date.cmp(&b.date));
        self.config = config;
        self.records = records;
        Ok(())
    }
    fn save(&self) -> io::Result<()> {
        if let Some(error) = &self.error {
            return Err(io::Error::other(error.clone()));
        }
        let rows:Vec<_>=self.records.iter().map(|r|json!({"date":r.date,"start":r.start,"end":r.end,"reminded":r.reminded,"snooze":r.snooze})).collect();
        let c = &self.config;
        let body = serde_json::to_vec_pretty(
            &json!({"version":1,"config":{"automatic":c.automatic,"enabled":c.enabled,"end_minutes":c.end_minutes,"weekdays":c.weekdays},"records":rows}),
        )?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        crate::model::atomic_write(&self.path, &body)
    }
    fn update(&mut self, change: impl FnOnce(&mut Self) -> io::Result<()>) -> io::Result<()> {
        let mut next = self.clone();
        change(&mut next)?;
        next.save()?;
        *self = next;
        Ok(())
    }
    pub fn record(&self, date: &str) -> Option<&Record> {
        self.records.iter().find(|r| r.date == date)
    }
    pub fn target(&self, date: &str) -> Option<i64> {
        moment(date, &time_text(self.config.end_minutes))
    }
    pub fn active_day(&self, date: &str) -> bool {
        NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .ok()
            .is_some_and(|d| self.config.weekdays & (1 << d.weekday().num_days_from_monday()) != 0)
    }
    pub fn configure(&mut self, config: Config) -> io::Result<()> {
        if config.end_minutes >= 1440 || config.weekdays == 0 || config.weekdays > 127 {
            return Err(invalid("请设置有效时间并至少选择一天"));
        }
        self.update(|w| {
            w.config = config;
            if !w.config.enabled {
                for r in &mut w.records {
                    r.snooze = None;
                }
            }
            Ok(())
        })
    }
    pub fn begin(&mut self, date: &str, now: i64) -> io::Result<()> {
        if self.record(date).is_some() {
            return Ok(());
        }
        if chrono::DateTime::from_timestamp(now, 0)
            .map(|d| d.with_timezone(&Local).date_naive().to_string())
            .as_deref()
            != Some(date)
        {
            return Err(invalid("记录日期无效"));
        }
        self.update(|w| {
            w.records.push(Record {
                date: date.into(),
                start: now,
                end: None,
                reminded: false,
                snooze: None,
            });
            w.records.sort_by(|a, b| a.date.cmp(&b.date));
            Ok(())
        })
    }
    /// 只有当天首次启动或跨日后真实用户活跃时，才允许自动建档。
    /// 提醒状态先持久化，再交给 UI，重启不会重复弹同一次提醒。
    pub fn poll(
        &mut self,
        date: &str,
        now: i64,
        can_begin: bool,
        notifications: bool,
    ) -> io::Result<bool> {
        if can_begin && self.config.automatic {
            self.begin(date, now)?;
        }
        if !notifications || !self.config.enabled || !self.active_day(date) {
            return Ok(false);
        }
        let Some(r) = self.record(date) else {
            return Ok(false);
        };
        let Some(target) = self.target(date) else {
            return Ok(false);
        };
        if r.end.is_some()
            || target < r.start
            || now < r.start
            || (r.snooze.is_none() && r.reminded)
        {
            return Ok(false);
        }
        if now < r.snooze.unwrap_or(target) {
            return Ok(false);
        }
        self.update(|w| {
            let r = w.records.iter_mut().find(|r| r.date == date).unwrap();
            r.reminded = true;
            r.snooze = None;
            Ok(())
        })?;
        Ok(true)
    }
    pub fn edit_start(&mut self, date: &str, text: &str, now: i64) -> io::Result<()> {
        let minutes = parse_time(text).ok_or_else(|| invalid("请输入 HH:MM，例如 09:00"))?;
        let start =
            moment(date, &time_text(minutes)).ok_or_else(|| invalid("该本地时间不存在或有歧义"))?;
        if start > now {
            return Err(invalid("上班时间不能晚于当前时间"));
        }
        self.update(|w| {
            let r = w
                .records
                .iter_mut()
                .find(|r| r.date == date)
                .ok_or_else(|| invalid("今天还没有上班记录"))?;
            if r.end.is_some_and(|end| start > end) {
                return Err(invalid("上班时间不能晚于已记录的下班时间"));
            }
            r.start = start;
            Ok(())
        })
    }
    pub fn finish(&mut self, date: &str, now: i64) -> io::Result<()> {
        self.update(|w| {
            let r = w
                .records
                .iter_mut()
                .find(|r| r.date == date)
                .ok_or_else(|| invalid("今天还没有上班记录"))?;
            if r.end.is_some() {
                return Ok(());
            }
            if now < r.start
                || chrono::DateTime::from_timestamp(now, 0)
                    .map(|d| d.with_timezone(&Local).date_naive().to_string())
                    .as_deref()
                    != Some(date)
            {
                return Err(invalid("时间异常，不能记录下班"));
            }
            r.end = Some(now);
            r.snooze = None;
            Ok(())
        })
    }
    pub fn snooze(&mut self, date: &str, now: i64) -> io::Result<()> {
        self.update(|w| {
            let r = w
                .records
                .iter_mut()
                .find(|r| r.date == date)
                .ok_or_else(|| invalid("今天还没有上班记录"))?;
            if r.end.is_some() {
                return Err(invalid("今天已经下班"));
            }
            let due = now.checked_add(600).ok_or_else(|| invalid("时间越界"))?;
            r.reminded = true;
            r.snooze = Some(due);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Work {
        let dir = std::env::temp_dir().join(format!(
            "daozai-work-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        Work::load_at(dir.join("work.json"))
    }
    fn at(date: &str, time: &str) -> i64 {
        moment(date, time).unwrap()
    }
    #[test]
    fn validates_time() {
        for text in ["9:00", "24:00", "12:60", "ab:cd", "０９:００", ""] {
            assert_eq!(parse_time(text), None);
        }
        assert_eq!(parse_time("09:05"), Some(545));
    }
    #[test]
    fn first_start_survives_restart_and_finish_is_sticky() {
        let mut w = fixture();
        let d = "2026-11-05";
        w.poll(d, at(d, "09:00"), true, true).unwrap();
        let mut w = Work::load_at(w.path.clone());
        w.poll(d, at(d, "11:00"), true, true).unwrap();
        assert_eq!(w.record(d).unwrap().start, at(d, "09:00"));
        w.finish(d, at(d, "18:40")).unwrap();
        w.poll(d, at(d, "19:00"), true, true).unwrap();
        assert_eq!(w.record(d).unwrap().end, Some(at(d, "18:40")));
    }
    #[test]
    fn reminder_snooze_and_restart_are_deduplicated() {
        let mut w = fixture();
        w.config.enabled = true;
        let d = "2026-11-05";
        w.poll(d, at(d, "09:00"), true, true).unwrap();
        assert!(!w.poll(d, at(d, "18:29"), false, true).unwrap());
        assert!(w.poll(d, at(d, "18:30"), false, true).unwrap());
        let mut w = Work::load_at(w.path.clone());
        assert!(!w.poll(d, at(d, "18:31"), false, true).unwrap());
        w.snooze(d, at(d, "18:31")).unwrap();
        let mut w = Work::load_at(w.path.clone());
        assert!(!w.poll(d, at(d, "18:40"), false, true).unwrap());
        assert!(w.poll(d, at(d, "18:41"), false, true).unwrap());
        w.finish(d, at(d, "18:42")).unwrap();
        assert!(!w.poll(d, at(d, "19:00"), false, true).unwrap());
    }
    #[test]
    fn next_day_requires_activity_and_keeps_unfinished_history() {
        let mut w = fixture();
        w.poll("2026-11-05", at("2026-11-05", "09:00"), true, true)
            .unwrap();
        w.poll("2026-11-06", at("2026-11-06", "00:00"), false, true)
            .unwrap();
        assert!(w.record("2026-11-06").is_none());
        w.poll("2026-11-06", at("2026-11-06", "09:00"), true, true)
            .unwrap();
        assert_eq!(w.records.len(), 2);
        assert!(w.records[0].end.is_none());
    }
    #[test]
    fn sleeping_missed_time_is_reminded_once_on_resume() {
        let mut w = fixture();
        w.config.enabled = true;
        let d = "2026-11-05";
        w.poll(d, at(d, "09:00"), true, true).unwrap();
        assert!(w.poll(d, at(d, "19:00"), false, true).unwrap());
        assert!(!w.poll(d, at(d, "19:01"), false, true).unwrap());
    }
    #[test]
    fn days_switches_clock_rollback_and_manual_edit() {
        let mut w = fixture();
        w.config.enabled = true;
        let d = "2026-11-07";
        w.poll(d, at(d, "09:00"), true, true).unwrap();
        assert!(!w.poll(d, at(d, "19:00"), false, true).unwrap());
        w.config.weekdays = 127;
        assert!(!w.poll(d, at(d, "19:00"), false, false).unwrap());
        assert!(!w.poll(d, at(d, "08:00"), false, true).unwrap());
        assert!(w.edit_start(d, "20:00", at(d, "19:00")).is_err());
        w.edit_start(d, "08:30", at(d, "19:00")).unwrap();
        assert_eq!(w.record(d).unwrap().start, at(d, "08:30"));
        assert!(w.poll(d, at(d, "19:00"), false, true).unwrap());
    }
    #[test]
    fn corrupt_data_and_failed_writes_do_not_overwrite_or_advance() {
        let mut w = fixture();
        fs::create_dir_all(w.path.parent().unwrap()).unwrap();
        fs::write(&w.path, b"broken").unwrap();
        let mut loaded = Work::load_at(w.path.clone());
        assert!(loaded.error.is_some());
        assert!(loaded
            .begin("2026-11-05", at("2026-11-05", "09:00"))
            .is_err());
        assert_eq!(fs::read(&w.path).unwrap(), b"broken");
        w.path = w.path.with_file_name("blocked");
        fs::create_dir_all(&w.path).unwrap();
        assert!(w.begin("2026-11-05", at("2026-11-05", "09:00")).is_err());
        assert!(w.records.is_empty());
    }
    #[test]
    fn disabled_automatic_and_changed_schedule_do_not_reopen_finished_record() {
        let mut w = fixture();
        let d = "2026-11-05";
        let mut c = w.config.clone();
        c.automatic = false;
        c.enabled = true;
        w.configure(c).unwrap();
        w.poll(d, at(d, "09:00"), true, true).unwrap();
        assert!(w.record(d).is_none());
        w.begin(d, at(d, "09:00")).unwrap();
        w.snooze(d, at(d, "18:30")).unwrap();
        let mut c = w.config.clone();
        c.enabled = false;
        w.configure(c).unwrap();
        assert!(w.record(d).unwrap().snooze.is_none());
        w.finish(d, at(d, "18:40")).unwrap();
        let mut c = w.config.clone();
        c.enabled = true;
        c.end_minutes = 19 * 60;
        w.configure(c).unwrap();
        assert!(!w.poll(d, at(d, "19:00"), true, true).unwrap());
    }
    #[test]
    fn finished_record_cannot_be_edited_past_end_and_invalid_local_clock_is_rejected() {
        let mut w = fixture();
        let d = "2026-11-05";
        w.begin(d, at(d, "09:00")).unwrap();
        w.finish(d, at(d, "18:00")).unwrap();
        assert!(w.edit_start(d, "18:30", at(d, "19:00")).is_err());
        assert_eq!(w.record(d).unwrap().start, at(d, "09:00"));
        assert!(moment(d, "25:00").is_none());
        assert!(moment("bad", "09:00").is_none());
    }
}
