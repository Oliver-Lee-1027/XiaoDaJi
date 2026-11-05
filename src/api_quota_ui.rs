use crate::{
    api_quota::{self, Account, Balance},
    app::App,
    extras::{setting_row, settings_card, settings_label},
    paint::{Card, H},
    theme::{self, W},
};
use eframe::egui::{self, Color32};

fn detail(balance: &Balance) -> String {
    let mut text = format!(
        "剩余：{}",
        api_quota::amount(balance.remaining, &balance.unit, false)
    );
    if let Some(used) = balance.used {
        text.push_str(&format!(
            "\n已用：{}",
            api_quota::amount(used, &balance.unit, false)
        ));
    }
    if let Some(total) = balance.total {
        text.push_str(&format!(
            "\n总计：{}（当前剩余 + 累计已用，非固定套餐上限）",
            api_quota::amount(total, &balance.unit, false)
        ));
    }
    if let Some(at) = chrono::DateTime::from_timestamp(balance.at, 0) {
        text.push_str(&format!(
            "\n最近成功更新：{}",
            at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S")
        ));
    }
    text
}
impl App {
    pub(crate) fn api_quota_settings(&mut self, ui: &mut egui::Ui) {
        settings_label(ui, "API 额度查询 · NewAPI");
        let before = self.api_quota.config.clone();
        let mut edit = None;
        let mut refresh = None;
        let mut remove = None;
        settings_card(ui, |ui| {
            setting_row(
                ui,
                "启用 API 额度查询",
                "独立于订阅额度；后台只读查询，不发起模型调用",
                &mut self.api_quota.config.enabled,
            );
            ui.separator();
            setting_row(
                ui,
                "悬浮窗最右侧显示余额",
                "一次显示一个指定账户；失败保留旧值并标记过期",
                &mut self.api_quota.config.show_balance,
            );
            for account in &mut self.api_quota.config.accounts {
                ui.push_id(account.id.clone(), |ui| {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(&account.alias);
                        let enabled = account.enabled;
                        ui.checkbox(&mut account.enabled, "监控");
                        if enabled != account.enabled {
                            account.revision = account.revision.wrapping_add(1);
                            self.api_quota.busy.remove(&account.id);
                        }
                        if ui
                            .selectable_label(
                                self.api_quota.config.selected == account.id,
                                "在悬浮窗显示此账户",
                            )
                            .clicked()
                        {
                            self.api_quota.config.selected = account.id.clone();
                        }
                        if ui.small_button("编辑账户").clicked() {
                            edit = Some(account.clone());
                        }
                        if ui
                            .add_enabled(
                                !self.api_quota.busy.contains_key(&account.id),
                                egui::Button::new("立即刷新").small(),
                            )
                            .clicked()
                        {
                            refresh = Some(account.id.clone());
                        }
                        if ui.small_button("移除账户").clicked() {
                            remove = Some(account.id.clone());
                        }
                    });
                    ui.label(format!(
                        "{} · 用户 ID {} · {}",
                        account.base_url,
                        account.user_id,
                        if account.interval == 0 {
                            "仅手动查询".into()
                        } else {
                            format!("每 {} 分钟查询", account.interval)
                        }
                    ));
                    if let Some(row) = self.api_quota.rows.get(&account.id) {
                        if let Some(balance) = &row.balance {
                            ui.label(detail(balance));
                        }
                        if let Some(error) = &row.error {
                            ui.colored_label(
                                Color32::from_rgb(210, 120, 40),
                                format!("{error}（上次成功余额仅供参考）"),
                            );
                        }
                    }
                    if self.api_quota.busy.contains_key(&account.id) {
                        ui.label("正在查询…");
                    }
                });
            }
            if self.api_quota.config.accounts.is_empty() {
                ui.label("尚未添加 API 账户，请在下方填写站点资料。");
            }
        });
        if before.enabled != self.api_quota.config.enabled {
            self.api_quota.invalidate_requests();
        }
        if before != self.api_quota.config {
            if let Err(e) = self.api_quota.config.save() {
                self.api_quota.config = before;
                self.api_quota.note = e;
            }
        }
        if let Some(account) = edit {
            self.api_quota.reset_draft(Some(account));
        }
        if let Some(id) = refresh {
            if let Err(e) = self.api_quota.refresh(&id) {
                self.api_quota.note = e;
            }
        }
        if let Some(id) = remove {
            self.api_quota.remove = Some(id);
        }
        if let Some(id) = self.api_quota.remove.clone() {
            settings_card(ui, |ui| {
                ui.label("确认移除该 API 账户及安全存储中的令牌？不会删除站点上的账户。");
                ui.horizontal(|ui| {
                    if ui.button("确认移除 API 账户").clicked() {
                        self.api_quota.note = match self.api_quota.delete(&id) {
                            Ok(()) => "已移除账户与本机令牌".into(),
                            Err(e) => e,
                        };
                        self.api_quota.remove = None;
                    }
                    if ui.button("取消移除").clicked() {
                        self.api_quota.remove = None;
                    }
                });
            });
        }
        settings_label(
            ui,
            if self.api_quota.draft.account.id.is_empty() {
                "添加 API 账户"
            } else {
                "编辑 API 账户"
            },
        );
        settings_card(ui, |ui| {
            let draft = &mut self.api_quota.draft;
            let previous = draft.account.clone();
            let mut secret_changed = false;
            ui.label("查询模板：NewAPI（GET /api/user/self）");
            let input_width = (ui.available_width() - 175.0).max(120.0);
            egui::Grid::new("newapi-credentials")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("备注");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.account.alias)
                            .hint_text("例如：个人 API 账户"),
                    );
                    ui.end_row();
                    ui.label("服务根地址");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.account.base_url)
                            .desired_width(input_width)
                            .hint_text("https://example.com，不含 /v1"),
                    );
                    ui.end_row();
                    ui.label("访问令牌");
                    ui.horizontal(|ui| {
                        secret_changed = ui
                            .add(
                                egui::TextEdit::singleline(&mut draft.secret)
                                    .desired_width((input_width - 55.0).max(80.0))
                                    .password(!draft.reveal)
                                    .hint_text(if draft.account.id.is_empty() {
                                        "站点个人安全设置中的访问令牌"
                                    } else {
                                        "留空保留已保存令牌"
                                    }),
                            )
                            .changed();
                        ui.checkbox(&mut draft.reveal, "显示");
                    });
                    ui.end_row();
                    ui.label("用户 ID");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.account.user_id)
                            .desired_width(input_width),
                    );
                    ui.end_row();
                    ui.label("请求超时（秒）");
                    ui.add(egui::DragValue::new(&mut draft.account.timeout).range(1..=120));
                    ui.end_row();
                    ui.label("自动查询间隔（分钟）");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut draft.account.interval).range(0..=1440));
                        ui.label("0 = 仅手动查询");
                    });
                    ui.end_row();
                    ui.label("额度换算系数");
                    ui.add(
                        egui::DragValue::new(&mut draft.account.divisor)
                            .speed(100.0)
                            .range(0.000001..=1e15),
                    );
                    ui.end_row();
                    ui.label("显示单位");
                    ui.add(egui::TextEdit::singleline(&mut draft.account.unit).desired_width(90.0));
                    ui.end_row();
                });
            // Invalidate a preview if its request inputs change while the worker is running.
            if previous != draft.account || secret_changed {
                draft.generation = draft.generation.wrapping_add(1);
                self.api_quota.testing = false;
                self.api_quota.preview = None;
            }
            ui.label("换算：剩余 = quota ÷ 系数；已用 = used_quota ÷ 系数。默认 500000，请以站点规则为准。");
            ui.label("令牌保存在 Windows 凭据管理器，不写入普通配置；测试成功不会自动保存。");
            ui.horizontal(|ui| {
                if ui.button("保存 API 账户").clicked() {
                    self.api_quota.note = match self.api_quota.save_draft() {
                        Ok(()) => "API 账户已安全保存".into(),
                        Err(e) => e,
                    };
                }
                if ui
                    .add_enabled(!self.api_quota.testing, egui::Button::new("测试查询"))
                    .clicked()
                {
                    if let Err(e) = self.api_quota.test() {
                        self.api_quota.note = e;
                    }
                }
                if ui.button("清空 / 新建").clicked() {
                    self.api_quota.reset_draft(None);
                }
            });
            if self.api_quota.testing {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("测试查询中…");
                });
            }
            if let Some(balance) = &self.api_quota.preview {
                ui.label(detail(balance));
            }
            if !self.api_quota.note.is_empty() {
                ui.label(&self.api_quota.note);
            }
        });
    }
    pub(crate) fn api_tail_visible(&self) -> bool {
        !self.effective_minimal() && self.t >= self.notification_bell_until
    }
    pub(crate) fn model_tail_visible(&self) -> bool {
        self.api_tail_visible() && self.model_icons.has(&self.settings.current_model)
    }
    pub(crate) fn api_balance_text(&self) -> Option<String> {
        if !self.api_tail_visible()
            || !self.settings.visible_pages[6]
            || !self.api_quota.config.enabled
            || !self.api_quota.config.show_balance
        {
            return None;
        }
        let account = self.api_quota.config.selected()?;
        let mut text = self
            .api_quota
            .rows
            .get(&account.id)
            .and_then(|r| r.balance.as_ref())
            .map(|b| api_quota::amount(b.remaining, &b.unit, true))
            .unwrap_or_else(|| "—".into());
        if self.api_quota.stale(&account.id) {
            text.push('*');
        }
        Some(text)
    }
    pub(crate) fn api_tail_width(&self, ctx: &egui::Context) -> f32 {
        self.api_balance_text()
            .map(|text| {
                ctx.fonts(|f| {
                    f.layout_no_wrap(text, theme::font(11.0, W::Semibold), theme::WHITE)
                        .size()
                        .x
                        + 6.0
                })
            })
            .unwrap_or(0.0)
    }
    pub(crate) fn draw_api_tail(&mut self, c: &Card, cx: f32, layout: &crate::bar_layout::Layout) {
        use crate::bar_layout::{BALANCE, MODEL};
        if let Some(x) = layout.x[MODEL] {
            let x = cx + x;
            self.model_icons
                .draw(c, &self.settings.current_model, x, 15.0, 20.0);
            crate::quota_ui::bar_tooltip(
                c.hit(x - 12.0, 2.0, 24.0, 26.0, "current-model-badge"),
                format!(
                    "当前模型标识：{}\n手动选择，仅作展示",
                    self.model_icons.label(&self.settings.current_model)
                ),
            );
        }
        if let (Some(x), Some(text)) = (layout.x[BALANCE], self.api_balance_text()) {
            let x = cx + x;
            let Some(account) = self.api_quota.config.selected() else {
                return;
            };
            let stale = self.api_quota.stale(&account.id);
            c.centered_text(
                x,
                15.0,
                &text,
                11.0,
                W::Semibold,
                if stale {
                    Color32::from_rgb(255, 188, 90)
                } else {
                    theme::WHITE
                },
                H::Center,
            );
            let mut summary = account.alias.clone();
            if let Some(row) = self.api_quota.rows.get(&account.id) {
                if let Some(balance) = &row.balance {
                    summary.push('\n');
                    summary.push_str(&detail(balance));
                }
                if let Some(error) = &row.error {
                    summary.push_str(&format!("\n{error}"));
                }
            }
            if stale {
                summary.push_str("\n* 旧数据，仅供参考");
            }
            if self.api_quota.busy.contains_key(&account.id) {
                summary.push_str("\n正在更新…");
            }
            let width = layout.balance_width;
            if crate::quota_ui::bar_tooltip(
                c.hit(x - width / 2.0, 1.0, width, 28.0, "api-balance"),
                summary,
            )
            .clicked()
            {
                self.api_quota.show_detail = true;
                self.show_page(6);
            }
        }
    }
    pub(crate) fn page_api_quota(&mut self, c: &Card) {
        if c.hit(303.0, 8.0, 40.0, 21.0, "api-detail-refresh")
            .clicked()
        {
            if let Some(a) = self.api_quota.config.selected() {
                let id = a.id.clone();
                if let Err(e) = self.api_quota.refresh(&id) {
                    self.say(e);
                }
            }
        }
        c.text(
            319.0,
            23.0,
            "刷新",
            10.5,
            W::Regular,
            theme::GRAY,
            H::Center,
        );
        let items: Vec<Account> = self
            .api_quota
            .config
            .accounts
            .iter()
            .filter(|a| a.enabled)
            .cloned()
            .collect();
        let selected = items
            .iter()
            .position(|a| a.id == self.api_quota.config.selected)
            .unwrap_or(0);
        if items.is_empty() {
            c.text(
                43.0,
                64.0,
                "还没有启用的 API 账户",
                13.0,
                W::Bold,
                theme::WHITE,
                H::Left,
            );
            c.text(
                43.0,
                85.0,
                "前往设置 → AI 额度，添加 NewAPI 账户",
                11.0,
                W::Regular,
                theme::GRAY,
                H::Left,
            );
        } else {
            let account = &items[selected];
            let name = c.fit(&account.alias, 12.0, W::Semibold, 240.0);
            c.text(52.0, 56.0, &name, 12.0, W::Semibold, theme::WHITE, H::Left);
            for (x, offset, symbol) in [(40.0, -1isize, "‹"), (354.0, 1, "›")] {
                if c.hit(x - 7.0, 36.0, 18.0, 22.0, ("api-account-switch", offset))
                    .clicked()
                {
                    self.api_quota.config.selected = items
                        [(selected as isize + offset).rem_euclid(items.len() as isize) as usize]
                        .id
                        .clone();
                    if let Err(e) = self.api_quota.config.save() {
                        self.say(e);
                    }
                }
                c.text(x, 55.0, symbol, 14.0, W::Bold, theme::GRAY, H::Center);
            }
            let row = self.api_quota.rows.get(&account.id);
            if let Some(balance) = row.and_then(|r| r.balance.as_ref()) {
                c.text(
                    43.0,
                    68.0,
                    "剩余额度",
                    10.0,
                    W::Regular,
                    theme::GRAY,
                    H::Left,
                );
                let amount = api_quota::amount(balance.remaining, &balance.unit, false);
                c.text(
                    43.0,
                    97.0,
                    &c.fit(&amount, 23.0, W::Bold, 170.0),
                    23.0,
                    W::Bold,
                    theme::WHITE,
                    H::Left,
                );
                let used = balance
                    .used
                    .map(|v| api_quota::amount(v, &balance.unit, false))
                    .unwrap_or_else(|| "未知".into());
                c.text(
                    230.0,
                    69.0,
                    &c.fit(&format!("已用 {used}"), 10.5, W::Regular, 125.0),
                    10.5,
                    W::Regular,
                    theme::GRAY,
                    H::Left,
                );
                let total = balance
                    .total
                    .map(|v| api_quota::amount(v, &balance.unit, false))
                    .unwrap_or_else(|| "未知".into());
                c.text(
                    230.0,
                    85.0,
                    &c.fit(&format!("合计 {total}"), 10.5, W::Regular, 125.0),
                    10.5,
                    W::Regular,
                    theme::GRAY,
                    H::Left,
                );
                let at = chrono::DateTime::from_timestamp(balance.at, 0)
                    .map(|d| {
                        d.with_timezone(&chrono::Local)
                            .format("%H:%M:%S")
                            .to_string()
                    })
                    .unwrap_or_default();
                c.text(
                    230.0,
                    101.0,
                    &format!("更新于 {at}"),
                    10.0,
                    W::Regular,
                    theme::GRAY,
                    H::Left,
                );
                crate::quota_ui::bar_tooltip(
                    c.hit(43.0, 58.0, 312.0, 46.0, "api-detail-summary"),
                    format!("{}\n{}", balance.plan, detail(balance)),
                );
            }
            let stale = self.api_quota.stale(&account.id);
            let status = if self.api_quota.busy.contains_key(&account.id) {
                "正在查询…"
            } else if let Some(error) = row.and_then(|r| r.error.as_deref()) {
                error
            } else if row.and_then(|r| r.balance.as_ref()).is_none() {
                "等待查询；可点击右上角刷新"
            } else if stale {
                "数据已过期，可点击刷新"
            } else {
                "只读余额 · 点击左上角标题切换订阅额度"
            };
            c.text(
                43.0,
                119.0,
                &c.fit(status, 10.0, W::Regular, 310.0),
                10.0,
                W::Regular,
                if stale {
                    Color32::from_rgb(255, 188, 90)
                } else {
                    theme::GRAY
                },
                H::Left,
            );
        }
    }
}
