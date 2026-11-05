//! Manual model labels are independent of quota providers and actual model routing.
use crate::{
    app::{App, Cmd},
    extras::{settings_card, settings_label},
    paint::{Card, H},
    theme::{self, W},
};
use eframe::egui::{self, Color32};
use serde_json::{json, Value};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

pub(crate) const BUILTINS: [(&str, &str); 6] = [
    ("codex", "Codex"),
    ("claude", "Claude"),
    ("kimi", "Kimi"),
    ("grokbot", "Grok"),
    ("gemini", "Gemini"),
    ("sv", "SV"),
];
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Custom {
    pub id: String,
    pub name: String,
    pub file: String,
}
#[derive(Default)]
pub(crate) struct State {
    pub custom: Vec<Custom>,
    pub name: String,
    pub source: String,
    pub editing: Option<String>,
    pub note: String,
    pub remove: Option<String>,
}
fn safe_file(file: &str) -> bool {
    file.ends_with(".png")
        && file.len() < 120
        && file
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        && !file.contains("..")
}
fn directory() -> PathBuf {
    crate::model::config_dir().join("model-icons")
}
impl State {
    pub fn load() -> Self {
        if cfg!(test) {
            return Self::default();
        }
        let custom = fs::read(crate::model::config_dir().join("model-icons.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map(|v| {
                v.as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| {
                        let file = c["file"].as_str()?;
                        let id = c["id"].as_str()?;
                        let name = c["name"].as_str()?;
                        if !safe_file(file)
                            || !id.starts_with("custom-")
                            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                            || name.trim().is_empty()
                            || name.chars().count() > 20
                        {
                            return None;
                        }
                        Some(Custom {
                            id: id.into(),
                            name: name.into(),
                            file: file.into(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            custom,
            ..Default::default()
        }
    }
    fn save(&self) -> Result<(), String> {
        let dir = crate::model::config_dir();
        fs::create_dir_all(&dir).map_err(|_| "无法创建配置目录")?;
        let bytes = serde_json::to_vec_pretty(
            &self
                .custom
                .iter()
                .map(|c| json!({"id":c.id,"name":c.name,"file":c.file}))
                .collect::<Vec<_>>(),
        )
        .map_err(|_| "无法编码图标配置")?;
        crate::model::atomic_write(&dir.join("model-icons.json"), &bytes)
            .map_err(|_| "无法保存自定义标识".into())
    }
    pub fn label(&self, key: &str) -> String {
        BUILTINS
            .iter()
            .find(|(id, _)| *id == key)
            .map(|(_, name)| name.to_string())
            .or_else(|| {
                self.custom
                    .iter()
                    .find(|c| c.id == key)
                    .map(|c| c.name.clone())
            })
            .unwrap_or_else(|| "不显示".into())
    }
    pub fn has(&self, key: &str) -> bool {
        BUILTINS.iter().any(|(id, _)| *id == key) || self.custom.iter().any(|c| c.id == key)
    }
    fn validate_name(&self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 20 || name.chars().any(char::is_control) {
            return Err("名称须为 1–20 个非控制字符".into());
        }
        if name == "不显示"
            || BUILTINS.iter().any(|(_, n)| n.eq_ignore_ascii_case(name))
            || self.custom.iter().any(|c| {
                Some(&c.id) != self.editing.as_ref() && c.name.to_lowercase() == name.to_lowercase()
            })
        {
            return Err("名称已存在，请换一个名称".into());
        }
        Ok(())
    }
    pub fn commit(&mut self) -> Result<String, String> {
        let name = self.name.trim().to_owned();
        self.validate_name(&name)?;
        let existing = self
            .editing
            .as_ref()
            .and_then(|id| self.custom.iter().find(|c| &c.id == id))
            .cloned();
        let bytes = if self.source.trim().is_empty() {
            if existing.is_none() {
                return Err("请选择本地 PNG、JPG 或 WebP 图片".into());
            }
            None
        } else {
            Some(normalize_image(Path::new(self.source.trim()))?)
        };
        let unique = crate::api_quota::new_id().replace("api-", "custom-");
        let id = existing
            .as_ref()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| unique.clone());
        let file = if let Some(bytes) = &bytes {
            fs::create_dir_all(directory()).map_err(|_| "无法创建图标目录")?;
            let file = format!("{unique}.png");
            crate::model::atomic_write(&directory().join(&file), bytes)
                .map_err(|_| "无法保存图标副本")?;
            file
        } else {
            existing.as_ref().unwrap().file.clone()
        };
        let old = self.custom.clone();
        let entry = Custom {
            id: id.clone(),
            name,
            file: file.clone(),
        };
        if let Some(target) = self.custom.iter_mut().find(|c| c.id == id) {
            *target = entry;
        } else {
            self.custom.push(entry);
        }
        if let Err(e) = self.save() {
            self.custom = old;
            if bytes.is_some() {
                remove_image(&file);
            }
            return Err(e);
        }
        if let Some(old) = existing {
            if old.file != file {
                remove_image(&old.file);
            }
        }
        self.name.clear();
        self.source.clear();
        self.editing = None;
        Ok(id)
    }
    pub(crate) fn delete(&mut self, id: &str) -> Result<(), String> {
        let old = self.custom.clone();
        let removed = self.custom.iter().find(|c| c.id == id).cloned();
        self.custom.retain(|c| c.id != id);
        if let Err(e) = self.save() {
            self.custom = old;
            return Err(e);
        }
        if let Some(c) = removed {
            remove_image(&c.file);
        }
        if self.editing.as_deref() == Some(id) {
            self.editing = None;
            self.source.clear();
            self.name.clear();
        }
        Ok(())
    }
    fn texture(&self, ctx: &egui::Context, key: &str) -> Option<egui::TextureHandle> {
        let custom = self.custom.iter().find(|c| c.id == key);
        let cache = egui::Id::new((
            "model-badge",
            key,
            custom.map(|c| c.file.as_str()).unwrap_or("builtin"),
        ));
        if let Some(t) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(cache)) {
            return Some(t);
        }
        let image = match key {
            "codex" | "claude" | "kimi" | "grokbot" | "gemini" => {
                let provider = match key {
                    "codex" => crate::quota::Provider::Codex,
                    "claude" => crate::quota::Provider::Claude,
                    "kimi" => crate::quota::Provider::Kimi,
                    "grokbot" => crate::quota::Provider::GrokBot,
                    _ => crate::quota::Provider::Gemini,
                };
                return crate::quota_ui::logo_texture(ctx, provider, 64);
            }
            "sv" => image::load_from_memory(include_bytes!("../assets/ai-icons/sv.png")).ok()?,
            _ => {
                let c = custom?;
                if !safe_file(&c.file) {
                    return None;
                }
                let bytes = normalize_image(&directory().join(&c.file)).ok()?;
                image::load_from_memory(&bytes).ok()?
            }
        }
        .to_rgba8();
        let texture = ctx.load_texture(
            format!("badge-{key}"),
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        ctx.data_mut(|d| d.insert_temp(cache, texture.clone()));
        Some(texture)
    }
    pub fn draw(&self, c: &Card, key: &str, x: f32, y: f32, size: f32) {
        if let Some(texture) = self.texture(c.ui.ctx(), key) {
            let dimensions = texture.size_vec2();
            let scale = size / dimensions.x.max(dimensions.y);
            let w = dimensions.x * scale;
            let h = dimensions.y * scale;
            c.painter.image(
                texture.id(),
                c.rect(x - w / 2.0, y - h / 2.0, w, h),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            c.centered_text(x, y, "?", 12.0, W::Bold, theme::GRAY, H::Center);
        }
    }
}
fn preview_image(ui: &mut egui::Ui, texture: &egui::TextureHandle, size: f32) {
    egui::Frame::new()
        .fill(Color32::BLACK)
        .corner_radius(5.0)
        .inner_margin(4)
        .show(ui, |ui| {
            ui.add(
                egui::Image::new((texture.id(), texture.size_vec2()))
                    .max_size(egui::vec2(size, size)),
            );
        });
}
fn remove_image(file: &str) {
    if safe_file(file) {
        let _ = fs::remove_file(directory().join(file));
    }
}
fn normalize_image(path: &Path) -> Result<Vec<u8>, String> {
    if !path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| {
            ["png", "jpg", "jpeg", "webp"]
                .iter()
                .any(|v| ext.eq_ignore_ascii_case(v))
        })
    {
        return Err("只支持 PNG、JPG、WebP 图片".into());
    }
    if fs::metadata(path).map_err(|_| "无法读取所选图片")?.len() > 2 * 1024 * 1024 {
        return Err("图片不能超过 2 MB".into());
    }
    let bytes = fs::read(path).map_err(|_| "无法读取所选图片")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("图片不能超过 2 MB".into());
    }
    let reader = image::ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|_| "无法识别图片")?;
    let (w, h) = reader
        .into_dimensions()
        .map_err(|_| "图片已损坏或格式不支持")?;
    if w == 0 || h == 0 || w > 2048 || h > 2048 {
        return Err("图片尺寸须在 1×1 至 2048×2048 之间".into());
    }
    let image = image::load_from_memory(&bytes)
        .map_err(|_| "图片解码失败")?
        .thumbnail(128, 128);
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|_| "图标转换失败")?;
    Ok(out.into_inner())
}
impl App {
    pub fn model_icon_source(&mut self, path: PathBuf) {
        self.model_icons.source = path.to_string_lossy().into_owned();
    }
    pub(crate) fn model_badge_settings(&mut self, ui: &mut egui::Ui) {
        settings_label(ui, "当前使用模型");
        settings_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("模型标识");
                egui::ComboBox::from_id_salt("current-model")
                    .selected_text(self.model_icons.label(&self.settings.current_model))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.settings.current_model,
                            String::new(),
                            "不显示",
                        );
                        for (key, name) in BUILTINS {
                            ui.selectable_value(
                                &mut self.settings.current_model,
                                key.to_string(),
                                name,
                            );
                        }
                        for custom in &self.model_icons.custom {
                            ui.selectable_value(
                                &mut self.settings.current_model,
                                custom.id.clone(),
                                &custom.name,
                            );
                        }
                    });
                if !self.settings.current_model.is_empty() {
                    if let Some(texture) = self
                        .model_icons
                        .texture(ui.ctx(), &self.settings.current_model)
                    {
                        preview_image(ui, &texture, 24.0);
                    }
                }
            });
            ui.label("仅手动展示标识，不会切换其他软件的实际模型；与 API 余额账户独立。");
            ui.collapsing("添加 / 管理自定义标识", |ui| {
                let items = self.model_icons.custom.clone();
                for item in items {
                    ui.horizontal(|ui| {
                        ui.label(&item.name);
                        if ui.small_button("使用").clicked() {
                            self.settings.current_model = item.id.clone();
                        }
                        if ui.small_button("编辑").clicked() {
                            self.model_icons.editing = Some(item.id.clone());
                            self.model_icons.name = item.name.clone();
                            self.model_icons.source.clear();
                            self.model_icons.note = "未选择新图片时保留原图".into();
                            if let Some(texture) = self.model_icons.texture(ui.ctx(), &item.id) {
                                preview_image(ui, &texture, 24.0);
                            }
                        }
                        if ui.small_button("删除").clicked() {
                            self.model_icons.remove = Some(item.id.clone());
                        }
                    });
                }
                if let Some(id) = self.model_icons.remove.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "确认删除 {}？若正在使用，将改为不显示。",
                            self.model_icons.label(&id)
                        ));
                        if ui.button("确认删除标识").clicked() {
                            match self.model_icons.delete(&id) {
                                Ok(()) => {
                                    if self.settings.current_model == id {
                                        self.settings.current_model.clear();
                                    }
                                    self.model_icons.note = "已删除标识，原始图片不受影响".into();
                                }
                                Err(e) => self.model_icons.note = e,
                            }
                            self.model_icons.remove = None;
                        }
                        if ui.button("取消删除").clicked() {
                            self.model_icons.remove = None;
                        }
                    });
                }
                ui.separator();
                ui.label(if self.model_icons.editing.is_some() {
                    "编辑自定义标识"
                } else {
                    "添加自定义标识"
                });
                ui.horizontal(|ui| {
                    ui.label("名称");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.model_icons.name)
                            .hint_text("1–20 个字符"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("图标");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.model_icons.source)
                            .desired_width(300.0)
                            .hint_text("PNG / JPG / WebP，≤2 MB，≤2048×2048"),
                    );
                    if ui.button("选择图片…").clicked() {
                        self.pending.push(Cmd::ModelChooseIcon);
                    }
                });
                if !self.model_icons.source.trim().is_empty() {
                    match normalize_image(Path::new(self.model_icons.source.trim())) {
                        Ok(bytes) => {
                            if let Ok(image) = image::load_from_memory(&bytes) {
                                let rgba = image.to_rgba8();
                                let texture = ui.ctx().load_texture(
                                    "model-import-preview",
                                    egui::ColorImage::from_rgba_unmultiplied(
                                        [rgba.width() as usize, rgba.height() as usize],
                                        rgba.as_raw(),
                                    ),
                                    egui::TextureOptions::LINEAR,
                                );
                                ui.horizontal(|ui| {
                                    preview_image(ui, &texture, 36.0);
                                    ui.label(&self.model_icons.name);
                                });
                            }
                        }
                        Err(e) => {
                            ui.label(e);
                        }
                    }
                }
                ui.horizontal(|ui| {
                    if ui.button("保存并使用").clicked() {
                        match self.model_icons.commit() {
                            Ok(id) => {
                                self.settings.current_model = id;
                                self.model_icons.note =
                                    "已保存并使用，图标副本位于应用配置目录".into();
                            }
                            Err(e) => self.model_icons.note = e,
                        }
                    }
                    if self.model_icons.editing.is_some() && ui.button("取消编辑").clicked() {
                        self.model_icons.editing = None;
                        self.model_icons.name.clear();
                        self.model_icons.source.clear();
                    }
                });
                if !self.model_icons.note.is_empty() {
                    ui.label(&self.model_icons.note);
                }
            });
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn six_builtins_include_sv() {
        assert_eq!(BUILTINS.len(), 6);
        assert_eq!(State::default().label("sv"), "SV");
        assert!(!State::default().has("unknown"));
    }
    #[test]
    fn names_and_paths_are_validated() {
        let mut state = State::default();
        for name in ["", "SV", "codex", "不显示", "line\nbreak"] {
            assert!(state.validate_name(name).is_err());
        }
        assert!(state.validate_name("我的模型").is_ok());
        state.custom.push(Custom {
            id: "custom-1".into(),
            name: "我的模型".into(),
            file: "custom-1.png".into(),
        });
        assert!(state.validate_name("我的模型").is_err());
        state.editing = Some("custom-1".into());
        assert!(state.validate_name("我的模型").is_ok());
        for file in ["../file.png", "C:/file.png", "..\\x.png", "file.svg"] {
            assert!(!safe_file(file));
        }
        assert!(safe_file("custom-123.png"));
    }
    #[test]
    fn sv_decodes_and_keeps_its_blue_color() {
        let rgba = image::load_from_memory(include_bytes!("../assets/ai-icons/sv.png"))
            .unwrap()
            .to_rgba8();
        assert!(rgba.pixels().any(|p| p[2] > p[0] && p[3] > 0));
    }
}
