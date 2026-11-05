//! 分区置物架：只保存路径引用，任何操作都不移动或删除真实文件。
use serde_json::{json, Value};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
pub(crate) const MAX_GROUPS: usize = 6;
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub paths: Vec<PathBuf>,
}
#[derive(Clone, Debug)]
pub struct Shelf {
    /// 全部条目的去重视图，保留旧录制器/调用方接口。
    pub items: Vec<PathBuf>,
    pub groups: Vec<Group>,
    pub error: Option<String>,
    next_id: u64,
}
impl Default for Shelf {
    fn default() -> Self {
        Self {
            items: vec![],
            groups: vec![Group {
                id: 0,
                name: "未分区".into(),
                paths: vec![],
            }],
            error: None,
            next_id: 1,
        }
    }
}
fn invalid(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, text)
}
fn checked_name(name: &str) -> io::Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 12 || name.chars().any(char::is_control) {
        return Err(invalid("分区名需为 1–12 个字符，不能包含换行"));
    }
    Ok(name.into())
}
impl Shelf {
    pub fn load() -> Self {
        Self::load_at(&crate::model::config_dir())
    }
    fn load_at(dir: &Path) -> Self {
        match Self::read(dir) {
            Ok(s) => s,
            Err(e) => Self {
                error: Some(format!("置物架读取失败，原数据未改动：{e}")),
                ..Default::default()
            },
        }
    }
    fn read(dir: &Path) -> io::Result<Self> {
        let path = dir.join("shelf-v2.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        if let Some(bytes) = bytes {
            let v: Value = serde_json::from_slice(&bytes)?;
            if v["version"].as_u64() != Some(2) {
                return Err(invalid("不支持的置物架版本"));
            }
            let rows = v["groups"]
                .as_array()
                .filter(|a| !a.is_empty() && a.len() <= MAX_GROUPS)
                .ok_or_else(|| invalid("分区数量无效"))?;
            let mut s = Self {
                groups: vec![],
                ..Default::default()
            };
            for row in rows {
                let id = row["id"].as_u64().ok_or_else(|| invalid("分区 ID 无效"))?;
                let name =
                    checked_name(row["name"].as_str().ok_or_else(|| invalid("分区名无效"))?)?;
                if s.groups.iter().any(|g| g.id == id || g.name == name) {
                    return Err(invalid("分区 ID 或名称重复"));
                }
                if id == 0 && name != "未分区" {
                    return Err(invalid("默认分区名称无效"));
                }
                let mut paths = Vec::new();
                for value in row["paths"]
                    .as_array()
                    .ok_or_else(|| invalid("文件列表无效"))?
                {
                    let value = value
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| invalid("文件路径无效"))?;
                    let p = PathBuf::from(value);
                    if !paths.contains(&p) {
                        paths.push(p);
                    }
                }
                s.groups.push(Group { id, name, paths });
            }
            if s.groups[0].id != 0 {
                return Err(invalid("缺少默认分区"));
            }
            let max = s.groups.iter().map(|g| g.id).max().unwrap_or(0);
            s.next_id = v["next_id"]
                .as_u64()
                .filter(|id| *id > max)
                .ok_or_else(|| invalid("分区序号无效"))?;
            s.refresh();
            return Ok(s);
        }
        let mut s = Self::default();
        match fs::read_to_string(dir.join("shelf.txt")) {
            Ok(text) => {
                for line in text
                    .trim_start_matches('\u{feff}')
                    .lines()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let p = PathBuf::from(line);
                    if !s.items.contains(&p) {
                        s.items.push(p);
                    }
                }
                // 旧清单不改写；备份和新文件都写成功后，才算迁移完成。
                if let Err(error) = s.save_at(dir) {
                    s.error = Some(format!(
                        "旧清单已读取，但分区迁移未完成（当前只读）：{error}"
                    ));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        Ok(s)
    }
    pub fn paths(&self, id: u64) -> Vec<PathBuf> {
        let mut paths = self
            .groups
            .iter()
            .find(|g| g.id == id)
            .map(|g| g.paths.clone())
            .unwrap_or_default();
        if id == 0 {
            for p in &self.items {
                if !self.groups.iter().any(|g| g.paths.contains(p)) && !paths.contains(p) {
                    paths.push(p.clone());
                }
            }
        }
        paths
    }
    fn normalize(&mut self) {
        let unassigned: Vec<_> = self
            .items
            .iter()
            .filter(|p| !self.groups.iter().any(|g| g.paths.contains(p)))
            .cloned()
            .collect();
        self.groups[0].paths.extend(unassigned);
        self.refresh();
    }
    fn refresh(&mut self) {
        self.items.clear();
        for g in &self.groups {
            for p in &g.paths {
                if !self.items.contains(p) {
                    self.items.push(p.clone());
                }
            }
        }
    }
    fn save_at(&self, dir: &Path) -> io::Result<()> {
        if let Some(error) = &self.error {
            return Err(io::Error::other(error.clone()));
        }
        fs::create_dir_all(dir)?;
        let legacy = dir.join("shelf.txt");
        let backup = dir.join("shelf-legacy-backup.txt");
        if legacy.exists() {
            if backup.exists() && !backup.is_file() {
                return Err(invalid("旧清单备份位置不是文件"));
            }
            if !backup.exists() {
                let body = fs::read(&legacy)?;
                crate::model::atomic_write(&backup, &body)?;
            }
        }
        let groups: Vec<_> = self
            .groups
            .iter()
            .map(|g| json!({"id":g.id,"name":g.name,"paths":self.paths(g.id)}))
            .collect();
        let bytes = serde_json::to_vec_pretty(
            &json!({"version":2,"next_id":self.next_id,"groups":groups}),
        )?;
        crate::model::atomic_write(&dir.join("shelf-v2.json"), &bytes)
    }
    pub fn save(&self) -> io::Result<()> {
        self.save_at(&crate::model::config_dir())
    }
    fn update(&mut self, change: impl FnOnce(&mut Self) -> io::Result<()>) -> io::Result<()> {
        let mut next = self.clone();
        next.normalize();
        change(&mut next)?;
        next.refresh();
        next.save()?;
        *self = next;
        Ok(())
    }
    pub fn add(&mut self, src: &Path) -> io::Result<()> {
        self.add_to(0, src)
    }
    pub fn add_to(&mut self, id: u64, src: &Path) -> io::Result<()> {
        if !src.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "文件不存在"));
        }
        self.update(|s| {
            let g = s
                .groups
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or_else(|| invalid("分区不存在"))?;
            if !g.paths.iter().any(|p| p == src) {
                g.paths.push(src.into());
            }
            Ok(())
        })
    }
    pub fn create_group(&mut self, name: &str) -> io::Result<u64> {
        let name = checked_name(name)?;
        let id = self.next_id;
        self.update(|s| {
            if s.groups.len() >= MAX_GROUPS {
                return Err(invalid("最多创建 6 个分区（含未分区）"));
            }
            if s.groups.iter().any(|g| g.name == name) {
                return Err(invalid("分区名已存在"));
            }
            s.next_id = id.checked_add(1).ok_or_else(|| invalid("分区序号越界"))?;
            s.groups.push(Group {
                id,
                name,
                paths: vec![],
            });
            Ok(())
        })?;
        Ok(id)
    }
    pub fn rename_group(&mut self, id: u64, name: &str) -> io::Result<()> {
        let name = checked_name(name)?;
        self.update(|s| {
            if id == 0 {
                return Err(invalid("未分区是保底区域，不能重命名"));
            }
            if s.groups.iter().any(|g| g.id != id && g.name == name) {
                return Err(invalid("分区名已存在"));
            }
            s.groups
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or_else(|| invalid("分区不存在"))?
                .name = name;
            Ok(())
        })
    }
    pub fn reorder_group(&mut self, id: u64, up: bool) -> io::Result<()> {
        self.update(|s| {
            let i = s
                .groups
                .iter()
                .position(|g| g.id == id)
                .ok_or_else(|| invalid("分区不存在"))?;
            let to = if up {
                i.saturating_sub(1)
            } else {
                (i + 1).min(s.groups.len() - 1)
            };
            if i > 0 && to > 0 {
                s.groups.swap(i, to);
            }
            Ok(())
        })
    }
    pub fn delete_group(&mut self, id: u64) -> io::Result<()> {
        self.update(|s| {
            if id == 0 {
                return Err(invalid("不能删除未分区"));
            }
            let i = s
                .groups
                .iter()
                .position(|g| g.id == id)
                .ok_or_else(|| invalid("分区不存在"))?;
            let g = s.groups.remove(i);
            for p in g.paths {
                if !s.groups[0].paths.contains(&p) {
                    s.groups[0].paths.push(p);
                }
            }
            Ok(())
        })
    }
    pub fn move_to(&mut self, from: u64, to: u64, p: &Path) -> io::Result<()> {
        self.update(|s| {
            if !s.groups.iter().any(|g| g.id == to)
                || !s
                    .groups
                    .iter()
                    .any(|g| g.id == from && g.paths.iter().any(|old| old == p))
            {
                return Err(invalid("文件或目标分区不存在"));
            }
            if from != to {
                for g in &mut s.groups {
                    if g.id == from {
                        g.paths.retain(|old| old != p);
                    }
                    if g.id == to && !g.paths.iter().any(|old| old == p) {
                        g.paths.push(p.into());
                    }
                }
            }
            Ok(())
        })
    }
    pub fn remove_from(&mut self, id: u64, p: &Path) -> io::Result<()> {
        self.update(|s| {
            let g = s
                .groups
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or_else(|| invalid("分区不存在"))?;
            g.paths.retain(|old| old != p);
            Ok(())
        })
    }
    pub fn clear_group(&mut self, id: u64) -> io::Result<()> {
        self.update(|s| {
            s.groups
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or_else(|| invalid("分区不存在"))?
                .paths
                .clear();
            Ok(())
        })
    }
    pub fn remove(&mut self, p: &Path) {
        let _ = self.update(|s| {
            for g in &mut s.groups {
                g.paths.retain(|old| old != p);
            }
            Ok(())
        });
    }
    pub fn clear(&mut self) {
        let _ = self.update(|s| {
            for g in &mut s.groups {
                g.paths.clear();
            }
            Ok(())
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn dir() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "daozai-shelf-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn legacy_migrates_with_backup_without_pruning_missing_paths() {
        let d = dir();
        let body = "C:\\旧资料\\工作.txt\nC:\\missing.pdf\n";
        fs::write(d.join("shelf.txt"), body).unwrap();
        let s = Shelf::load_at(&d);
        assert!(s.error.is_none());
        assert_eq!(s.paths(0).len(), 2);
        assert_eq!(
            fs::read_to_string(d.join("shelf-legacy-backup.txt")).unwrap(),
            body
        );
        assert_eq!(Shelf::load_at(&d).paths(0), s.paths(0));
    }
    #[test]
    fn corrupt_new_data_never_falls_back_over_it() {
        let d = dir();
        fs::write(d.join("shelf.txt"), "C:\\old.txt").unwrap();
        fs::write(d.join("shelf-v2.json"), "broken").unwrap();
        let s = Shelf::load_at(&d);
        assert!(s.error.is_some());
        assert!(s.save_at(&d).is_err());
        assert_eq!(
            fs::read_to_string(d.join("shelf-v2.json")).unwrap(),
            "broken"
        );
    }
    #[test]
    fn persistence_keeps_group_identity_order_and_shared_references() {
        let d = dir();
        let mut s = Shelf::default();
        s.groups.push(Group {
            id: 9,
            name: "工作".into(),
            paths: vec![d.join("same.txt")],
        });
        s.groups[0].paths.push(d.join("same.txt"));
        s.next_id = 10;
        s.save_at(&d).unwrap();
        let loaded = Shelf::load_at(&d);
        assert_eq!(loaded.groups, s.groups);
        assert_eq!(loaded.items.len(), 1);
    }
    #[test]
    fn failed_save_preserves_original_memory() {
        let mut s = Shelf::default();
        s.error = Some("损坏".into());
        assert!(s.create_group("工作").is_err());
        assert_eq!(s.groups.len(), 1);
    }
    #[test]
    fn failed_migration_keeps_legacy_entries_visible_and_never_reports_success() {
        let d = dir();
        fs::write(d.join("shelf.txt"), "C:\\legacy\\合同.txt\n").unwrap();
        fs::create_dir_all(d.join("shelf-legacy-backup.txt")).unwrap();
        let s = Shelf::load_at(&d);
        assert_eq!(s.paths(0), vec![PathBuf::from("C:\\legacy\\合同.txt")]);
        assert!(s.error.as_ref().unwrap().contains("只读"));
        assert!(!d.join("shelf-v2.json").exists());
    }
}
