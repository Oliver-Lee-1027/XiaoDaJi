//! 下班页、设置与提醒的 UI 接线。所有计时依据可持久化的本地时间戳。
use crate::{
    app::{App, Env, WORK_PAGE},
    extras::{setting_row, settings_card, settings_label},
    paint::{self, Card, H},
    theme::{self, W},
    work::{self, Config},
};
use eframe::egui::{self, Color32};
#[derive(Default)]
pub(crate) struct State {
    pub time: String,
    pub start: String,
    pub message: String,
    pub now: Option<i64>,
    pub date: String,
    pub retry_at: f64,
    pub last_tick: Option<i64>,
}
impl State {
    pub fn new(config: &Config) -> Self {
        Self {
            time: work::time_text(config.end_minutes),
            ..Default::default()
        }
    }
}
impl App {
    pub(crate) fn clear_work_alerts(&mut self) {
        self.pending
            .retain(|cmd| !matches!(cmd,crate::app::Cmd::Notify(title,_) if title=="下班时间到了"));
        self.alerts.retain(|a| a.work_date.is_none());
        if self.notice.as_ref().is_some_and(|a| a.work_date.is_some()) {
            self.notice = None;
        }
    }
    pub(crate) fn work_tick(&mut self, env: &Env, first: bool, active: bool) {
        let Some(now) = work::moment(&env.today, &env.clock) else {
            return;
        };
        let day_changed = self.work_ui.date != env.today;
        // 总开关关闭后，已排队但尚未展示的下班提醒也必须撤回。
        if !self.settings.notifications {
            self.clear_work_alerts();
        }
        self.work_ui.now = Some(now);
        let can_begin = first || active;
        if day_changed {
            self.clear_work_alerts();
            self.work_ui.date = env.today.clone();
            self.work_ui.start.clear();
        }
        if self.work_ui.retry_at > self.t {
            return;
        }
        if self.work_ui.last_tick == Some(now)
            && !(can_begin && self.work.record(&env.today).is_none())
        {
            return;
        }
        self.work_ui.last_tick = Some(now);
        match self
            .work
            .poll(&env.today, now, can_begin, self.settings.notifications)
        {
            Ok(true) => {
                let elapsed = self.work.record(&env.today).map_or(0, |r| now - r.start);
                self.work_alert(
                    env.today.clone(),
                    format!("已上班 {}，辛苦了", work::duration(elapsed)),
                );
            }
            Ok(false) => {}
            Err(error) => {
                self.work_ui.message = format!("保存失败：{error}");
                self.work_ui.retry_at = self.t + 30.0;
            }
        }
        if self.work_ui.start.is_empty() {
            if let Some(record) = self.work.record(&env.today) {
                self.work_ui.start = work::clock(record.start);
            }
        }
    }
    pub(crate) fn finish_work(&mut self, date: &str) {
        if date != self.work_ui.date {
            self.say("这条提醒已过期，请查看今天的上下班记录");
            self.clear_work_alerts();
            return;
        }
        let Some(now) = self.work_ui.now else { return };
        match self.work.finish(date, now) {
            Ok(()) => {
                self.clear_work_alerts();
                self.work_ui.message = "已记录下班时间".into();
                self.say("今天辛苦了，下班愉快");
            }
            Err(error) => {
                self.work_ui.message = error.to_string();
                self.say(format!("未能记录下班：{error}"));
            }
        }
    }
    pub(crate) fn snooze_work(&mut self, date: &str) {
        if date != self.work_ui.date {
            self.say("这条提醒已过期");
            self.clear_work_alerts();
            return;
        }
        let Some(now) = self.work_ui.now else { return };
        match self.work.snooze(date, now) {
            Ok(()) => {
                self.clear_work_alerts();
                self.say("将在 10 分钟后再次提醒");
            }
            Err(error) => self.say(format!("延后失败：{error}")),
        }
    }
    fn save_work_config(&mut self, config: Config) {
        match self.work.configure(config) {
            Ok(()) => {
                self.work_ui.message = "设置已保存".into();
                self.clear_work_alerts();
                self.work_ui.last_tick = None;
            }
            Err(error) => self.work_ui.message = format!("未保存：{error}"),
        }
    }
    pub(crate) fn work_settings(&mut self, ui: &mut egui::Ui) {
        let mut config = self.work.config.clone();
        let before = config.clone();
        settings_label(ui, "下班提醒");
        settings_card(ui, |ui| {
            setting_row(
                ui,
                "到点提醒",
                "使用小妲己弹窗提醒，不会自动结束计时",
                &mut config.enabled,
            );
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("下班时间");
                ui.add(
                    egui::TextEdit::singleline(&mut self.work_ui.time)
                        .id_salt("work-end-time")
                        .desired_width(65.0)
                        .char_limit(5)
                        .hint_text("18:30"),
                );
                if ui.button("保存时间").clicked() {
                    if let Some(minutes) = work::parse_time(&self.work_ui.time) {
                        let conflicts = self
                            .work
                            .record(&self.work_ui.date)
                            .filter(|r| r.end.is_none())
                            .is_some_and(|r| {
                                work::moment(&self.work_ui.date, &work::time_text(minutes))
                                    .is_none_or(|end| end < r.start)
                            });
                        if conflicts {
                            self.work_ui.message =
                                "下班时间早于今日上班时间；请先修正上班时间。暂不支持跨天班次。"
                                    .into();
                        } else {
                            config.end_minutes = minutes;
                            self.save_work_config(config.clone());
                        }
                    } else {
                        self.work_ui.message = "请输入有效的 HH:MM，例如 18:30".into();
                    }
                }
            });
            ui.small(format!(
                "当前生效：{} · 本机时间",
                work::time_text(self.work.config.end_minutes)
            ));
            ui.small("提醒日期（周一至周日）");
            ui.horizontal_wrapped(|ui| {
                for (i, name) in ["一", "二", "三", "四", "五", "六", "日"]
                    .iter()
                    .enumerate()
                {
                    let on = config.weekdays & (1 << i) != 0;
                    if ui.selectable_label(on, *name).clicked() {
                        let days = config.weekdays ^ (1 << i);
                        if days == 0 {
                            self.work_ui.message = "至少选择一个提醒日期".into();
                        } else {
                            config.weekdays = days;
                        }
                    }
                }
            });
            if !self.settings.notifications {
                ui.colored_label(
                    Color32::from_rgb(220, 135, 60),
                    "通知总开关已关闭，下班提醒暂不会弹出。",
                );
            }
            ui.small("退出或关机期间无法提醒；唤醒恢复后会检查当天错过的提醒。");
        });
        settings_label(ui, "上班记录");
        settings_card(ui, |ui| {
            setting_row(
                ui,
                "自动记录上班",
                "每天首次启动记录；持续运行跨天后，首次用户操作起记",
                &mut config.automatic,
            );
            ui.separator();
            if let Some(record) = self.work.record(&self.work_ui.date).cloned() {
                ui.label(format!(
                    "今日上班 {} · {}",
                    work::clock(record.start),
                    if record.end.is_some() {
                        "已下班"
                    } else {
                        "计时中"
                    }
                ));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.work_ui.start)
                            .id_salt("work-start-time")
                            .desired_width(65.0)
                            .char_limit(5)
                            .hint_text("09:00"),
                    );
                    if ui.button("修正上班时间").clicked() {
                        if let Some(now) = self.work_ui.now {
                            match self
                                .work
                                .edit_start(&self.work_ui.date, &self.work_ui.start, now)
                            {
                                Ok(()) => self.work_ui.message = "上班时间已修正".into(),
                                Err(error) => self.work_ui.message = error.to_string(),
                            }
                        }
                    }
                });
                if record.end.is_none() && ui.button("我下班了").clicked() {
                    let date = self.work_ui.date.clone();
                    self.finish_work(&date);
                }
            } else {
                ui.label("今天尚未记录上班");
                if ui.button("现在开始上班").clicked() {
                    self.begin_work();
                }
            }
            ui.small("按自然经过时间计算，包含午休、锁屏和中途退出；不是正式考勤。");
            if ui.button("打开下班页").clicked() {
                self.settings_open = false;
                self.show_page(WORK_PAGE);
            }
        });
        if config != before && config != self.work.config {
            self.save_work_config(config);
        }
        if let Some(error) = &self.work.error {
            ui.colored_label(theme::RED_TOP, error);
        }
        if !self.work_ui.message.is_empty() {
            ui.add(egui::Label::new(&self.work_ui.message).wrap());
        }
        settings_label(ui, "近期记录");
        settings_card(ui, |ui| {
            if self.work.records.is_empty() {
                ui.small("记录会保存在本机；确认下班后显示结束时间。");
            }
            for r in self.work.records.iter().rev().take(30) {
                let end = r.end.map(work::clock).unwrap_or_else(|| {
                    if r.date == self.work_ui.date {
                        "计时中".into()
                    } else {
                        "未确认下班".into()
                    }
                });
                ui.label(format!("{}  {} — {}", r.date, work::clock(r.start), end));
                if let Some(end) = r.end {
                    ui.small(work::duration(end - r.start));
                }
                ui.separator();
            }
            if self.work.records.len() > 30 {
                ui.small("这里展示最近 30 条，历史记录仍保存在本机。");
            }
        });
    }
    fn begin_work(&mut self) {
        if let Some(now) = self.work_ui.now {
            match self.work.begin(&self.work_ui.date, now) {
                Ok(()) => {
                    self.work_ui.start = work::clock(now);
                    self.work_ui.message = "已记录上班时间".into();
                }
                Err(error) => {
                    self.work_ui.message = error.to_string();
                    self.say(format!("记录失败：{error}"));
                }
            }
        }
    }
    pub(crate) fn page_work(&mut self, c: &Card) {
        let date = self.work_ui.date.clone();
        if let Some(error) = &self.work.error {
            c.centered_text(
                209.5,
                75.0,
                "上下班记录暂不可用",
                16.0,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            c.centered_text(
                209.5,
                101.0,
                &c.fit(error, 10.0, W::Regular, 315.0),
                10.0,
                W::Regular,
                theme::GRAY,
                H::Center,
            );
            return;
        }
        let now = self.work_ui.now.unwrap_or(0);
        if let Some(record) = self.work.record(&date).cloned() {
            let elapsed = record.end.unwrap_or(now).saturating_sub(record.start);
            c.centered_text(
                209.5,
                50.0,
                if record.end.is_some() {
                    "今天已下班"
                } else {
                    "今日已上班"
                },
                11.0,
                W::Regular,
                theme::GRAY,
                H::Center,
            );
            c.centered_text(
                209.5,
                77.0,
                &work::duration(elapsed),
                24.0,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            c.centered_text(
                209.5,
                102.0,
                &format!(
                    "上班 {}    ·    {} {}",
                    work::clock(record.start),
                    if record.end.is_some() {
                        "下班"
                    } else {
                        "计划下班"
                    },
                    record
                        .end
                        .map(work::clock)
                        .unwrap_or_else(|| work::time_text(self.work.config.end_minutes))
                ),
                11.0,
                W::Regular,
                theme::GRAY,
                H::Center,
            );
            let (note, col) = if record.end.is_some() {
                ("今天辛苦了，好好休息".into(), theme::GREEN)
            } else if now < record.start {
                (
                    "系统时间早于上班时间，请检查时间设置".into(),
                    theme::RED_TOP,
                )
            } else if let Some(target) = self.work.target(&date) {
                if target < record.start {
                    ("请修正下班时间，暂不支持跨天班次".into(), theme::RED_TOP)
                } else if let Some(due) = record.snooze {
                    (format!("已延后提醒 · {}", work::clock(due)), theme::CYAN)
                } else if now >= target {
                    (
                        format!("已到下班时间 · 超过 {} 分钟", (now - target) / 60),
                        theme::GREEN,
                    )
                } else {
                    (
                        format!("距离下班 {}", work::duration(target - now)),
                        theme::CYAN,
                    )
                }
            } else {
                ("请设置有效下班时间".into(), theme::GRAY)
            };
            c.centered_text(209.5, 125.0, &note, 11.0, W::Regular, col, H::Center);
            if button(c, 78.0, 144.0, 128.0, "修改上班时间", "work-edit") {
                self.settings_tab = 12;
                self.open_settings();
            }
            if record.end.is_none() && button(c, 218.0, 144.0, 123.0, "我下班了", "work-finish")
            {
                self.finish_work(&date);
            }
        } else {
            c.centered_text(
                209.5,
                71.0,
                "今天尚未记录上班",
                18.0,
                W::Semibold,
                theme::WHITE,
                H::Center,
            );
            c.centered_text(
                209.5,
                102.0,
                "首次启动自动记录，也可以手动开始",
                11.0,
                W::Regular,
                theme::GRAY,
                H::Center,
            );
            if button(c, 141.0, 135.0, 137.0, "现在开始上班", "work-begin") {
                self.begin_work();
            }
        }
    }
}
fn button(c: &Card, x: f32, y: f32, width: f32, label: &str, id: &str) -> bool {
    let (r, b) = paint::capsule(c, x, y, width, 23.0, id, theme::BTN, theme::BTN_HOVER);
    b.centered_text(
        x + width / 2.0,
        y + 11.5,
        label,
        11.0,
        W::Semibold,
        theme::WHITE,
        H::Center,
    );
    r.clicked()
}
