//! NewAPI account balances. Tokens never enter the JSON configuration or diagnostics.
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub id: String,
    pub alias: String,
    pub base_url: String,
    pub user_id: String,
    pub timeout: u64,
    pub interval: u64,
    pub divisor: f64,
    pub unit: String,
    pub enabled: bool,
    pub revision: u64,
}
impl Default for Account {
    fn default() -> Self {
        Self {
            id: String::new(),
            alias: String::new(),
            base_url: String::new(),
            user_id: String::new(),
            timeout: 10,
            interval: 5,
            divisor: 500_000.0,
            unit: "USD".into(),
            enabled: true,
            revision: 0,
        }
    }
}
impl Account {
    pub fn validate(&mut self) -> Result<(), String> {
        self.base_url = validated_base(&self.base_url)?;
        self.alias = self.alias.trim().to_owned();
        self.user_id = self.user_id.trim().to_owned();
        self.unit = self.unit.trim().to_uppercase();
        if self.alias.is_empty() || self.alias.chars().count() > 40 {
            return Err("备注须为 1–40 个字符".into());
        }
        if self.user_id.is_empty()
            || self.user_id.len() > 20
            || !self.user_id.bytes().all(|b| b.is_ascii_digit())
            || self.user_id.bytes().all(|b| b == b'0')
        {
            return Err("用户 ID 须为正整数".into());
        }
        if !(1..=120).contains(&self.timeout) || self.interval > 1440 {
            return Err("超时须为 1–120 秒，间隔须为 0–1440 分钟".into());
        }
        if !self.divisor.is_finite() || self.divisor <= 0.0 {
            return Err("额度换算系数必须大于零".into());
        }
        if self.unit.is_empty()
            || self.unit.len() > 8
            || !self.unit.bytes().all(|b| b.is_ascii_alphabetic())
        {
            return Err("单位须为 1–8 个英文字母，例如 USD".into());
        }
        Ok(())
    }
    fn json(&self) -> Value {
        json!({"id":self.id,"alias":self.alias,"base_url":self.base_url,"user_id":self.user_id,"timeout":self.timeout,"interval":self.interval,"divisor":self.divisor,"unit":self.unit,"enabled":self.enabled,"revision":self.revision})
    }
}
pub fn validated_base(raw: &str) -> Result<String, String> {
    let mut url =
        url::Url::parse(raw.trim()).map_err(|_| "请输入完整的 HTTPS 服务地址".to_string())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("服务地址必须为 HTTPS，且不能包含账号密码、查询参数或片段".into());
    }
    let path = url.path().trim_end_matches('/').to_owned();
    if path.ends_with("/api/user/self") || path.ends_with("/v1") {
        return Err("请填写站点根地址（可含部署前缀），不要填写 /v1 或 /api/user/self".into());
    }
    url.set_path(&path);
    Ok(url.as_str().trim_end_matches('/').to_owned())
}
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub show_balance: bool,
    pub selected: String,
    pub accounts: Vec<Account>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            show_balance: true,
            selected: String::new(),
            accounts: vec![],
        }
    }
}
impl Config {
    pub fn load() -> Self {
        fs::read(crate::model::config_dir().join("api-quota.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map(|v| Self::from_json(&v))
            .unwrap_or_default()
    }
    fn from_json(v: &Value) -> Self {
        let accounts = v["accounts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| {
                let id = a["id"].as_str()?.to_owned();
                if !valid_id(&id) {
                    return None;
                }
                let mut account = Account {
                    id,
                    alias: a["alias"].as_str()?.into(),
                    base_url: a["base_url"].as_str()?.into(),
                    user_id: a["user_id"].as_str()?.into(),
                    timeout: a["timeout"].as_u64().unwrap_or(10),
                    interval: a["interval"].as_u64().unwrap_or(5),
                    divisor: a["divisor"].as_f64().unwrap_or(500_000.0),
                    unit: a["unit"].as_str().unwrap_or("USD").into(),
                    enabled: a["enabled"].as_bool().unwrap_or(true),
                    revision: a["revision"].as_u64().unwrap_or(0),
                };
                account.validate().ok()?;
                Some(account)
            })
            .collect();
        Self {
            enabled: v["enabled"].as_bool().unwrap_or(false),
            show_balance: v["show_balance"].as_bool().unwrap_or(true),
            selected: v["selected"].as_str().unwrap_or_default().into(),
            accounts,
        }
    }
    fn json(&self) -> Value {
        json!({"enabled":self.enabled,"show_balance":self.show_balance,"selected":self.selected,"accounts":self.accounts.iter().map(Account::json).collect::<Vec<_>>()})
    }
    pub fn save(&self) -> Result<(), String> {
        let dir = crate::model::config_dir();
        fs::create_dir_all(&dir).map_err(|_| "无法创建配置目录")?;
        let bytes = serde_json::to_vec_pretty(&self.json()).map_err(|_| "无法编码余额配置")?;
        crate::model::atomic_write(&dir.join("api-quota.json"), &bytes)
            .map_err(|_| "无法保存余额配置".into())
    }
    pub fn selected(&self) -> Option<&Account> {
        self.accounts
            .iter()
            .find(|a| a.id == self.selected && a.enabled)
    }
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() < 100 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
pub(crate) fn new_id() -> String {
    format!(
        "api-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct Balance {
    pub remaining: f64,
    pub used: Option<f64>,
    pub total: Option<f64>,
    pub plan: String,
    pub unit: String,
    pub at: i64,
}
fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str()?.parse().ok())
        .filter(|n| n.is_finite())
}
pub fn parse(v: &Value, divisor: f64, unit: &str) -> Result<Balance, String> {
    if !divisor.is_finite() || divisor <= 0.0 {
        return Err("额度换算系数无效".into());
    }
    // Do not echo server messages: they may contain credentials, HTML or private data.
    if v["success"].as_bool() != Some(true) {
        return Err("站点返回查询失败，请检查账户权限与查询配置".into());
    }
    let remaining = number(&v["data"]["quota"]).ok_or("响应缺少有效的 data.quota")? / divisor;
    let used = if v["data"]["used_quota"].is_null() {
        None
    } else {
        Some(number(&v["data"]["used_quota"]).ok_or("data.used_quota 不是有效数值")? / divisor)
    };
    let total = used.map(|n| n + remaining);
    if !remaining.is_finite()
        || used.is_some_and(|n| !n.is_finite() || n < 0.0)
        || total.is_some_and(|n| !n.is_finite())
    {
        return Err("余额数值超出有效范围".into());
    }
    Ok(Balance {
        remaining,
        used,
        total,
        plan: v["data"]["group"]
            .as_str()
            .unwrap_or("默认分组")
            .chars()
            .take(80)
            .collect(),
        unit: unit.into(),
        at: chrono::Utc::now().timestamp(),
    })
}
pub fn amount(value: f64, unit: &str, compact: bool) -> String {
    let magnitude = value.abs();
    let text = if compact && magnitude >= 1e12 {
        format!("{value:.2e}")
    } else if compact && magnitude >= 1e9 {
        format!("{:.2}B", value / 1e9)
    } else if compact && magnitude >= 1e6 {
        format!("{:.2}M", value / 1e6)
    } else if compact && magnitude >= 1e3 {
        format!("{:.2}K", value / 1e3)
    } else if compact && value != 0.0 && magnitude < 0.01 {
        if value > 0.0 {
            "<0.01".into()
        } else {
            "-<0.01".into()
        }
    } else {
        format!("{value:.2}")
    };
    match unit {
        "USD" => format!("${text}"),
        "CNY" => format!("¥{text}"),
        "EUR" => format!("€{text}"),
        _ => format!("{text} {unit}"),
    }
}
#[cfg(windows)]
fn credential(id: &str) -> Result<keyring::Entry, String> {
    if !valid_id(id) {
        return Err("凭据引用无效".into());
    }
    keyring::Entry::new("Daozai.ApiQuota", id).map_err(|_| "无法打开 Windows 凭据管理器".into())
}
pub(crate) fn store_token(id: &str, token: &str) -> Result<(), String> {
    validate_token(token)?;
    #[cfg(windows)]
    {
        credential(id)?
            .set_password(token.trim())
            .map_err(|_| "无法安全保存令牌，请重试".into())
    }
    #[cfg(not(windows))]
    {
        let _ = id;
        Err("此功能需要 Windows 安全存储".into())
    }
}
fn token(id: &str) -> Result<String, String> {
    #[cfg(windows)]
    {
        credential(id)?
            .get_password()
            .map_err(|_| "未找到访问令牌，请编辑账户重新输入".into())
    }
    #[cfg(not(windows))]
    {
        let _ = id;
        Err("此功能需要 Windows 安全存储".into())
    }
}
pub(crate) fn forget_token(id: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        match credential(id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("无法删除安全存储中的令牌".into()),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = id;
        Ok(())
    }
}
fn validate_token(token: &str) -> Result<(), String> {
    let token = token.trim();
    if token.is_empty() || token.len() > 8192 || !token.bytes().all(|b| (33..=126).contains(&b)) {
        Err("访问令牌为空、过长或包含非法字符".into())
    } else {
        Ok(())
    }
}
struct Job {
    account: Account,
    token: Option<String>,
    preview: bool,
    generation: u64,
}
struct Event {
    id: String,
    revision: u64,
    preview: bool,
    generation: u64,
    result: Result<Balance, String>,
    wait: u64,
}
struct Service {
    tx: mpsc::SyncSender<Job>,
    rx: mpsc::Receiver<Event>,
}
impl Service {
    fn start() -> Self {
        let (tx, jobs) = mpsc::sync_channel::<Job>(16);
        let (events, rx) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(job) = jobs.recv() {
                let (result, wait) = match job.token.or_else(|| token(&job.account.id).ok()) {
                    Some(secret) => fetch(&job.account, &secret),
                    None => (Err("未找到访问令牌，请编辑账户重新输入".into()), 60),
                };
                let _ = events.send(Event {
                    id: job.account.id,
                    revision: job.account.revision,
                    preview: job.preview,
                    generation: job.generation,
                    result,
                    wait,
                });
            }
        });
        Self { tx, rx }
    }
}
fn fetch(account: &Account, secret: &str) -> (Result<Balance, String>, u64) {
    let run = || -> Result<Balance, (String, u64)> {
        let mut account = account.clone();
        account.validate().map_err(|e| (e, 60))?;
        validate_token(secret).map_err(|e| (e, 60))?;
        let agent = crate::quota::http_agent_timeout(account.timeout);
        let response = agent
            .get(&format!("{}/api/user/self", account.base_url))
            .set("Authorization", &format!("Bearer {}", secret.trim()))
            .set("New-Api-User", &account.user_id)
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set("User-Agent", "Daozai/ApiQuota")
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(401 | 403, _) => {
                    ("令牌失效或权限不足，请编辑账户更新令牌".into(), 300)
                }
                ureq::Error::Status(429, r) => (
                    "查询过于频繁，已暂停自动重试".into(),
                    r.header("Retry-After")
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(180)
                        .clamp(30, 3600),
                ),
                ureq::Error::Status(code, _) => {
                    (format!("额度服务返回 HTTP {code}，请检查站点与接口"), 60)
                }
                _ => ("无法连接额度服务，请检查网络、代理或超时设置".into(), 60),
            })?;
        if !(200..300).contains(&response.status()) {
            return Err((
                "站点发生重定向，已阻止转发访问令牌；请填写最终 HTTPS 地址".into(),
                60,
            ));
        }
        let mut bytes = Vec::new();
        response
            .into_reader()
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|_| ("读取额度响应失败".into(), 60))?;
        if bytes.len() > 1_048_576 {
            return Err(("额度响应过大，已拒绝解析".into(), 60));
        }
        let value = serde_json::from_slice(&bytes)
            .map_err(|_| ("站点未返回有效 JSON，请确认服务根地址".into(), 60))?;
        parse(&value, account.divisor, &account.unit).map_err(|e| (e, 60))
    };
    match run() {
        Ok(balance) => (Ok(balance), 0),
        Err((error, wait)) => (Err(error), wait),
    }
}
#[derive(Default)]
pub(crate) struct Row {
    pub balance: Option<Balance>,
    pub error: Option<String>,
}
#[derive(Default)]
pub(crate) struct Draft {
    pub account: Account,
    pub secret: String,
    pub reveal: bool,
    pub generation: u64,
}
pub(crate) struct State {
    pub config: Config,
    pub rows: HashMap<String, Row>,
    pub busy: HashMap<String, u64>,
    due: HashMap<String, Instant>,
    last_attempt: HashMap<String, Instant>,
    service: Option<Service>,
    pub draft: Draft,
    pub note: String,
    pub preview: Option<Balance>,
    pub testing: bool,
    pub remove: Option<String>,
    pub show_detail: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            config: Config::default(),
            rows: HashMap::new(),
            busy: HashMap::new(),
            due: HashMap::new(),
            last_attempt: HashMap::new(),
            service: None,
            draft: Draft::default(),
            note: String::new(),
            preview: None,
            testing: false,
            remove: None,
            show_detail: false,
        }
    }
}
impl State {
    pub fn load() -> Self {
        Self {
            config: if cfg!(test) {
                Config::default()
            } else {
                Config::load()
            },
            ..Default::default()
        }
    }
    fn service(&mut self) -> &Service {
        self.service.get_or_insert_with(Service::start)
    }
    pub fn invalidate_requests(&mut self) {
        for account in &mut self.config.accounts {
            account.revision = account.revision.wrapping_add(1);
        }
        self.busy.clear();
        self.due.clear();
    }
    pub fn reset_draft(&mut self, account: Option<Account>) {
        let generation = self.draft.generation.wrapping_add(1);
        self.draft = Draft {
            account: account.unwrap_or_default(),
            generation,
            ..Default::default()
        };
        self.preview = None;
        self.testing = false;
    }
    pub fn save_draft(&mut self) -> Result<(), String> {
        let mut a = self.draft.account.clone();
        a.validate()?;
        let exists = self.config.accounts.iter().any(|old| old.id == a.id);
        if !exists {
            a.id = new_id();
        }
        let old_secret = if exists { token(&a.id).ok() } else { None };
        let supplied = !self.draft.secret.trim().is_empty();
        if !supplied && !exists {
            return Err("请输入访问令牌".into());
        }
        if supplied {
            store_token(&a.id, &self.draft.secret)?;
        } else {
            token(&a.id)?;
        }
        a.revision = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let old = self.config.clone();
        if let Some(target) = self.config.accounts.iter_mut().find(|old| old.id == a.id) {
            *target = a.clone();
        } else {
            self.config.accounts.push(a.clone());
        }
        if self.config.selected.is_empty() {
            self.config.selected = a.id.clone();
        }
        if let Err(e) = self.config.save() {
            self.config = old;
            if !exists {
                let _ = forget_token(&a.id);
            } else if supplied {
                if let Some(secret) = old_secret {
                    let _ = store_token(&a.id, &secret);
                }
            }
            return Err(e);
        }
        self.rows.remove(&a.id);
        self.busy.remove(&a.id);
        self.due.remove(&a.id);
        self.reset_draft(None);
        Ok(())
    }
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let old = self.config.clone();
        self.config.accounts.retain(|a| a.id != id);
        if self.config.selected == id {
            self.config.selected = self
                .config
                .accounts
                .first()
                .map(|a| a.id.clone())
                .unwrap_or_default();
        }
        if let Err(e) = self.config.save() {
            self.config = old;
            return Err(e);
        }
        self.rows.remove(id);
        self.busy.remove(id);
        self.due.remove(id);
        if self.draft.account.id == id {
            self.reset_draft(None);
        }
        forget_token(id)
    }
    pub fn test(&mut self) -> Result<(), String> {
        let mut account = self.draft.account.clone();
        account.validate()?;
        let secret = if self.draft.secret.trim().is_empty() {
            token(&account.id)?
        } else {
            validate_token(&self.draft.secret)?;
            self.draft.secret.trim().to_owned()
        };
        let generation = self.draft.generation.wrapping_add(1);
        self.draft.generation = generation;
        let job = Job {
            account,
            token: Some(secret),
            preview: true,
            generation,
        };
        self.service()
            .tx
            .try_send(job)
            .map_err(|_| "查询队列繁忙，请稍后重试".to_string())?;
        self.testing = true;
        self.preview = None;
        Ok(())
    }
    pub fn refresh(&mut self, id: &str) -> Result<(), String> {
        if !self.config.enabled {
            return Err("请先启用 API 额度查询".into());
        }
        let account = self
            .config
            .accounts
            .iter()
            .find(|a| a.id == id && a.enabled)
            .cloned()
            .ok_or("账户未启用")?;
        if self.busy.contains_key(id) {
            return Err("该账户正在查询中".into());
        }
        if self
            .last_attempt
            .get(id)
            .is_some_and(|at| at.elapsed() < Duration::from_secs(15))
        {
            return Err("请至少间隔 15 秒再手动刷新".into());
        }
        if self.due.get(id).is_some_and(|at| *at > Instant::now())
            && self.rows.get(id).is_some_and(|r| r.error.is_some())
        {
            return Err("查询失败后的退避期内，请稍后重试".into());
        }
        let revision = account.revision;
        self.service()
            .tx
            .try_send(Job {
                account,
                token: None,
                preview: false,
                generation: 0,
            })
            .map_err(|_| "查询队列繁忙，请稍后重试".to_string())?;
        self.last_attempt.insert(id.into(), Instant::now());
        self.busy.insert(id.into(), revision);
        self.due
            .insert(id.into(), Instant::now() + Duration::from_secs(15));
        Ok(())
    }
    fn receive(&mut self, event: Event) {
        if event.preview {
            if event.generation == self.draft.generation {
                self.testing = false;
                match event.result {
                    Ok(b) => {
                        self.preview = Some(b);
                        self.note = "测试成功（未保存配置）".into();
                    }
                    Err(e) => self.note = e,
                }
            }
            return;
        }
        if self.busy.get(&event.id) == Some(&event.revision) {
            self.busy.remove(&event.id);
        }
        let Some(account) = self
            .config
            .accounts
            .iter()
            .find(|a| a.id == event.id && a.revision == event.revision && a.enabled)
        else {
            return;
        };
        if !self.config.enabled {
            return;
        }
        let wait = event.wait.max(if account.interval == 0 {
            0
        } else {
            account.interval * 60
        });
        self.due
            .insert(event.id.clone(), Instant::now() + Duration::from_secs(wait));
        let row = self.rows.entry(event.id).or_default();
        match event.result {
            Ok(b) => {
                row.balance = Some(b);
                row.error = None;
            }
            Err(e) => row.error = Some(e),
        }
    }
    pub fn tick(&mut self) {
        loop {
            let event = self.service.as_ref().and_then(|s| s.rx.try_recv().ok());
            match event {
                Some(e) => self.receive(e),
                None => break,
            }
        }
        if cfg!(test) || !self.config.enabled {
            return;
        }
        for id in self.due_accounts(Instant::now()) {
            let _ = self.refresh(&id);
        }
    }
    fn due_accounts(&self, now: Instant) -> Vec<String> {
        if !self.config.enabled {
            return vec![];
        }
        self.config
            .accounts
            .iter()
            .filter(|a| {
                a.enabled
                    && a.interval > 0
                    && !self.busy.contains_key(&a.id)
                    && self.due.get(&a.id).is_none_or(|due| *due <= now)
            })
            .map(|a| a.id.clone())
            .collect()
    }
    pub fn stale(&self, id: &str) -> bool {
        self.rows.get(id).is_some_and(|r| {
            r.error.is_some()
                || r.balance.as_ref().is_some_and(|b| {
                    let interval = self
                        .config
                        .accounts
                        .iter()
                        .find(|a| a.id == id)
                        .map(|a| a.interval)
                        .unwrap_or(5);
                    chrono::Utc::now().timestamp() - b.at > (interval.max(1) * 60 + 60) as i64
                })
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newapi_amounts_are_not_percentages() {
        let b=parse(&json!({"success":true,"data":{"quota":6_170_000,"used_quota":2_835_000,"group":"vip"}}),500_000.0,"USD").unwrap();
        assert_eq!(b.remaining, 12.34);
        assert_eq!(b.used, Some(5.67));
        assert!((b.total.unwrap() - 18.01).abs() < 1e-10);
    }
    #[test]
    fn zero_negative_and_numeric_strings() {
        assert_eq!(
            parse(&json!({"success":true,"data":{"quota":"0"}}), 1.0, "USD")
                .unwrap()
                .remaining,
            0.0
        );
        assert_eq!(
            parse(&json!({"success":true,"data":{"quota":-1}}), 1.0, "USD")
                .unwrap()
                .remaining,
            -1.0
        );
    }
    #[test]
    fn rejects_invalid_payloads_without_leaking_messages() {
        for v in [
            json!({"success":false,"message":"secret-token"}),
            json!({"success":true,"data":{}}),
            json!({"success":true,"data":{"quota":"NaN"}}),
            json!({"success":true,"data":{"quota":2,"used_quota":-1}}),
        ] {
            let error = parse(&v, 1.0, "USD").unwrap_err();
            assert!(!error.contains("secret-token"));
        }
        assert!(parse(&json!({}), 0.0, "USD").is_err());
    }
    #[test]
    fn rejects_unsafe_urls() {
        for url in [
            "http://example.com",
            "https://u:p@example.com",
            "https://example.com?token=x",
            "https://example.com/#x",
            "https://example.com/v1",
            "https://example.com/api/user/self",
        ] {
            assert!(validated_base(url).is_err(), "{url}");
        }
        assert_eq!(
            validated_base("https://example.com/prefix/").unwrap(),
            "https://example.com/prefix"
        );
    }
    #[test]
    fn config_roundtrip_has_no_secret() {
        let c = Config {
            accounts: vec![Account {
                id: "api-1".into(),
                alias: "测试".into(),
                base_url: "https://example.com".into(),
                user_id: "7".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let v = c.json();
        assert!(!v.to_string().contains("token"));
        assert_eq!(Config::from_json(&v), c);
        assert!(!Config::from_json(&json!({})).enabled);
    }
    #[test]
    fn ignores_stale_revision_and_keeps_last_good_balance() {
        let mut s = State::default();
        let a = Account {
            id: "api-1".into(),
            revision: 2,
            enabled: true,
            ..Default::default()
        };
        s.config.enabled = true;
        s.config.accounts.push(a);
        let b = parse(&json!({"success":true,"data":{"quota":0}}), 1.0, "USD").unwrap();
        s.rows.insert(
            "api-1".into(),
            Row {
                balance: Some(b.clone()),
                error: None,
            },
        );
        s.receive(Event {
            id: "api-1".into(),
            revision: 1,
            preview: false,
            generation: 0,
            result: Err("旧请求".into()),
            wait: 1,
        });
        assert!(s.rows["api-1"].error.is_none());
        s.receive(Event {
            id: "api-1".into(),
            revision: 2,
            preview: false,
            generation: 0,
            result: Err("离线".into()),
            wait: 1,
        });
        assert_eq!(s.rows["api-1"].balance, Some(b));
        assert!(s.stale("api-1"));
    }
    #[test]
    fn compact_currency_and_tiny_balance() {
        assert_eq!(amount(0.0, "USD", true), "$0.00");
        assert_eq!(amount(12345.0, "USD", true), "$12.35K");
        assert_eq!(amount(0.001, "USD", true), "$<0.01");
        assert_eq!(amount(1.2, "CNY", false), "¥1.20");
        assert!(amount(1e300, "USD", true).len() < 24);
    }
    #[test]
    fn preview_generation_rejects_replaced_draft() {
        let mut s = State::default();
        s.reset_draft(None);
        s.receive(Event {
            id: String::new(),
            revision: 0,
            preview: true,
            generation: 0,
            result: Ok(Balance {
                remaining: 1.0,
                used: None,
                total: None,
                plan: String::new(),
                unit: "USD".into(),
                at: 0,
            }),
            wait: 0,
        });
        assert!(s.preview.is_none());
    }
    #[test]
    fn schedule_respects_per_account_due_enabled_and_busy() {
        let mut s = State::default();
        s.config.enabled = true;
        for (id, interval, enabled) in [
            ("a", 1, true),
            ("b", 0, true),
            ("c", 5, false),
            ("d", 5, true),
        ] {
            s.config.accounts.push(Account {
                id: id.into(),
                interval,
                enabled,
                ..Default::default()
            });
        }
        s.due
            .insert("d".into(), Instant::now() + Duration::from_secs(60));
        assert_eq!(s.due_accounts(Instant::now()), vec!["a"]);
        s.busy.insert("a".into(), 0);
        assert!(s.due_accounts(Instant::now()).is_empty());
        s.config.enabled = false;
        assert!(s
            .due_accounts(Instant::now() + Duration::from_secs(3600))
            .is_empty());
    }
    #[test]
    fn zero_interval_never_schedules_automatically() {
        let mut s = State::default();
        s.config.enabled = true;
        s.config.accounts.push(Account {
            interval: 0,
            ..Default::default()
        });
        assert!(s.due_accounts(Instant::now()).is_empty());
        s.tick();
        assert!(s.service.is_none());
        assert!(s.busy.is_empty());
    }
}
