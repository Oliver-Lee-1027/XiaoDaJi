//! 小岛分区条与分区管理；所有文件操作仅改变路径引用。
use crate::{
    app::App,
    extras::{settings_card, settings_label},
    paint::{self, Card, H},
    shelf::MAX_GROUPS,
    theme::{self, W},
};
use eframe::egui::{self, Color32, Pos2, Rect};
use std::collections::HashMap;
#[derive(Default)]
pub(crate) struct State {
    pub active: u64,
    pub new_name: String,
    pub edits: HashMap<u64, String>,
    pub confirm_delete: Option<u64>,
    pub message: String,
}
impl App {
    pub(crate) fn shelf_paths(&self) -> Vec<std::path::PathBuf> {
        self.shelf.paths(self.shelf_ui.active)
    }
    pub(crate) fn select_shelf_group(&mut self, id: u64) {
        if !self.shelf.groups.iter().any(|g| g.id == id) {
            return;
        }
        if self.shelf_ui.active != id {
            self.shelf_ui.active = id;
            self.shelf_first = 0;
            self.shelf_scroll.snap(0.0);
            self.confirm_clear = None;
        }
    }
    /// 分区和文件区的落点独立命中；空白/收起态由调用方回退到未分区。
    pub(crate) fn shelf_target_at(&self, p: Pos2) -> u64 {
        for (i, g) in self.shelf.groups.iter().enumerate() {
            if group_rect(i).contains(p) {
                return g.id;
            }
        }
        if Rect::from_min_max(Pos2::new(36.0, 84.0), Pos2::new(383.0, 187.0)).contains(p) {
            self.shelf_ui.active
        } else {
            0
        }
    }
    pub(crate) fn shelf_tabs(&mut self, c: &Card) {
        let mut selected = None;
        let hover = self.cursor_local.map(|p| self.shelf_target_at(p));
        for (i, g) in self.shelf.groups.clone().iter().enumerate() {
            let rect = group_rect(i);
            let on = g.id == self.shelf_ui.active;
            let dropping = self.drag_hot && hover == Some(g.id);
            let (r, b) = paint::capsule(
                c,
                rect.left(),
                rect.top(),
                rect.width(),
                rect.height(),
                &format!("shelf-group-{}", g.id),
                if dropping {
                    Color32::from_rgb(34, 85, 116)
                } else if on {
                    Color32::from_gray(57)
                } else {
                    Color32::from_gray(26)
                },
                theme::BTN_HOVER,
            );
            let count = self.shelf.paths(g.id).len();
            let label = if dropping {
                format!("放入「{}」", g.name)
            } else {
                format!("{}  {count}", g.name)
            };
            b.centered_text(
                rect.center().x,
                rect.center().y,
                &b.fit(&label, 10.5, W::Semibold, rect.width() - 12.0),
                10.5,
                W::Semibold,
                if dropping {
                    theme::CYAN
                } else if on {
                    theme::WHITE
                } else {
                    theme::GRAY
                },
                H::Center,
            );
            if r.on_hover_text(format!("{} · {} 个文件，可将文件拖到此分区", g.name, count))
                .clicked()
            {
                selected = Some(g.id);
            }
        }
        if let Some(id) = selected {
            self.select_shelf_group(id);
        }
    }
    pub(crate) fn shelf_drop_hint(&self, c: &Card) {
        if !self.drag_hot {
            return;
        }
        let id = self
            .cursor_local
            .map(|p| self.shelf_target_at(p))
            .unwrap_or(0);
        let name = self
            .shelf
            .groups
            .iter()
            .find(|g| g.id == id)
            .map(|g| g.name.as_str())
            .unwrap_or("未分区");
        c.stroke(38.0, 85.0, 343.0, 92.0, 9.0, 1.2, theme::CYAN);
        c.fill(92.0, 155.0, 235.0, 19.0, 9.0, Color32::from_gray(30));
        c.centered_text(
            209.5,
            164.5,
            &format!("松开放入「{name}」 · 不移动原文件"),
            10.0,
            W::Semibold,
            theme::CYAN,
            H::Center,
        );
    }
    pub(crate) fn shelf_settings(&mut self, ui: &mut egui::Ui) {
        settings_label(ui, "置物架分区");
        settings_card(ui, |ui| {
            ui.label("只保存文件位置，移除和归类都不会改动原文件。");
            ui.small(format!(
                "{} / {MAX_GROUPS} 个分区 · {} 个不同文件",
                self.shelf.groups.len(),
                self.shelf.items.len()
            ));
            ui.small("分区名最多 12 字；拖到分区标签即可归类。同一文件可被不同分区引用。");
            if let Some(error) = &self.shelf.error {
                ui.colored_label(theme::RED_TOP, error);
            }
            ui.separator();
            let mut action = None;
            for g in self.shelf.groups.clone() {
                ui.push_id(("shelf-manage", g.id), |ui| {
                    if g.id == 0 {
                        ui.label(format!("未分区 · {} 项（保留）", self.shelf.paths(0).len()));
                    } else {
                        let name = self.shelf_ui.edits.entry(g.id).or_insert(g.name.clone());
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(name)
                                    .desired_width(115.0)
                                    .char_limit(12),
                            );
                            if ui.button("保存名称").clicked() {
                                action = Some(Action::Rename(g.id, name.clone()));
                            }
                            ui.small(format!("{} 项", g.paths.len()));
                        });
                        ui.horizontal(|ui| {
                            if ui.small_button("上移").clicked() {
                                action = Some(Action::Reorder(g.id, true));
                            }
                            if ui.small_button("下移").clicked() {
                                action = Some(Action::Reorder(g.id, false));
                            }
                            if ui.small_button("删除分区").clicked() {
                                self.shelf_ui.confirm_delete = Some(g.id);
                            }
                        });
                        if self.shelf_ui.confirm_delete == Some(g.id) {
                            ui.label("删除后，条目移回「未分区」，原文件不动。");
                            ui.horizontal(|ui| {
                                if ui.button("确认删除分区").clicked() {
                                    action = Some(Action::Delete(g.id));
                                }
                                if ui.button("取消").clicked() {
                                    self.shelf_ui.confirm_delete = None;
                                }
                            });
                        }
                    }
                    ui.separator();
                });
            }
            if self.shelf.groups.len() < MAX_GROUPS {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.shelf_ui.new_name)
                            .desired_width(130.0)
                            .char_limit(12)
                            .hint_text("输入新分区名"),
                    );
                    if ui.button("新建分区").clicked() {
                        action = Some(Action::Create(self.shelf_ui.new_name.clone()));
                    }
                });
            } else {
                ui.small("已达到 6 个分区上限（含未分区）。");
            }
            if let Some(action) = action {
                let result = match action {
                    Action::Create(name) => self.shelf.create_group(&name).map(|id| {
                        self.select_shelf_group(id);
                        self.shelf_ui.new_name.clear();
                    }),
                    Action::Rename(id, name) => self.shelf.rename_group(id, &name),
                    Action::Delete(id) => self.shelf.delete_group(id).map(|_| {
                        if self.shelf_ui.active == id {
                            self.select_shelf_group(0);
                        }
                        self.shelf_ui.confirm_delete = None;
                    }),
                    Action::Reorder(id, up) => self.shelf.reorder_group(id, up),
                };
                self.shelf_ui.message = match result {
                    Ok(()) => "分区已保存".into(),
                    Err(error) => format!("未保存：{error}"),
                };
            }
            if !self.shelf_ui.message.is_empty() {
                ui.add(egui::Label::new(&self.shelf_ui.message).wrap());
            }
            if ui.button("打开置物架").clicked() {
                self.settings_open = false;
                self.show_page(3);
            }
        });
    }
    pub(crate) fn shelf_move_item(&mut self, from: u64, to: u64, path: &std::path::Path) {
        match self.shelf.move_to(from, to, path) {
            Ok(()) => {
                self.shelf_first = self
                    .shelf_first
                    .min(self.shelf_paths().len().saturating_sub(5));
                self.say("已移动到目标分区，原文件位置不变");
            }
            Err(error) => self.say(format!("移动失败：{error}")),
        }
    }
    pub(crate) fn on_drop_to(&mut self, files: Vec<std::path::PathBuf>, target: u64) {
        if !self.settings.visible_pages[3] || files.is_empty() {
            return;
        }
        self.drag_hot = false;
        self.drag_saved_clear();
        self.select_shelf_group(target);
        if self.page != 3 {
            self.go(3, 1.0);
        }
        let (mut added, mut duplicate, mut failed) = (0, 0, 0);
        let mut last_error = String::new();
        for p in files {
            if !p.exists() {
                failed += 1;
                last_error = "文件不存在或无法访问".into();
                continue;
            }
            if self.shelf.paths(target).contains(&p) {
                duplicate += 1;
                continue;
            }
            match self.shelf.add_to(target, &p) {
                Ok(()) => {
                    self.shelf_born.insert(p, self.t);
                    added += 1;
                }
                Err(error) => {
                    failed += 1;
                    last_error = error.to_string();
                }
            }
        }
        let message = if added == 0 && failed == 0 {
            if duplicate == 1 {
                "该文件已在本区".into()
            } else {
                "这些文件已在本区".into()
            }
        } else if added == 0 && duplicate == 0 {
            "添加失败，请在分区设置查看原因".into()
        } else {
            format!("新增 {added} · 重复 {duplicate} · 失败 {failed}")
        };
        if failed > 0 {
            self.shelf_ui.message = format!(
                "上次投放：{added} 个新增、{duplicate} 个重复、{failed} 个失败。原因：{last_error}"
            );
        }
        self.say(message);
        if added > 0 {
            self.pet.react(crate::pet::PetState::Happy);
            self.shelf_first = self.shelf_paths().len().saturating_sub(5);
        }
    }
}
fn group_rect(i: usize) -> Rect {
    Rect::from_min_size(
        Pos2::new(36.0 + (i % 3) as f32 * 117.0, 36.0 + (i / 3) as f32 * 24.0),
        egui::vec2(112.0, 21.0),
    )
}
enum Action {
    Create(String),
    Rename(u64, String),
    Delete(u64),
    Reorder(u64, bool),
}
