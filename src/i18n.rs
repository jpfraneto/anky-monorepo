//! Minimal i18n: load `locales/*.json` at startup, look up strings by key,
//! fall back to English when a key or language is missing.
//!
//! Registered into Tera as a function: `{{ t(key="nav.login_cta") }}`
//! The template handler supplies the current `lang` via a context var that the
//! function closure reads. We also ship a per-request JSON bundle to the browser
//! as `window.__I18N__` so inline JS can call `window.t('chat.anky_thinking')`.
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct I18n {
    /// lang code (e.g. "en", "es") -> key -> string
    locales: HashMap<String, HashMap<String, String>>,
    /// Fallback language (always "en")
    fallback: String,
}

impl I18n {
    pub fn load_from_dir(dir: &str) -> anyhow::Result<Self> {
        let mut locales = HashMap::new();
        let entries = std::fs::read_dir(dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let lang = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_lowercase(),
                None => continue,
            };
            let bytes = std::fs::read(&path)?;
            let value: Value = serde_json::from_slice(&bytes)?;
            let mut map = HashMap::new();
            if let Some(obj) = value.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        map.insert(k.clone(), s.to_string());
                    }
                }
            }
            locales.insert(lang, map);
        }
        if !locales.contains_key("en") {
            anyhow::bail!("locales/en.json missing — required as fallback");
        }
        Ok(I18n {
            locales,
            fallback: "en".to_string(),
        })
    }

    pub fn has_language(&self, lang: &str) -> bool {
        self.locales.contains_key(lang)
    }

    /// Resolve a supported language code. Falls back to `en`.
    pub fn resolve(&self, candidate: &str) -> String {
        let c = candidate.to_lowercase();
        if self.has_language(&c) {
            c
        } else {
            self.fallback.clone()
        }
    }

    /// Look up a key in `lang`, falling back to the fallback language, then to the key itself.
    pub fn t(&self, lang: &str, key: &str) -> String {
        if let Some(map) = self.locales.get(lang) {
            if let Some(v) = map.get(key) {
                return v.clone();
            }
        }
        if lang != self.fallback {
            if let Some(map) = self.locales.get(&self.fallback) {
                if let Some(v) = map.get(key) {
                    return v.clone();
                }
            }
        }
        key.to_string()
    }

    /// JSON bundle for one language, merged over English so missing keys fall back.
    /// Returned as a serde_json::Value for safe inlining into `<script>`.
    pub fn client_bundle(&self, lang: &str) -> Value {
        let mut out: Map<String, Value> = Map::new();
        if let Some(en) = self.locales.get(&self.fallback) {
            for (k, v) in en {
                out.insert(k.clone(), Value::String(v.clone()));
            }
        }
        if lang != self.fallback {
            if let Some(m) = self.locales.get(lang) {
                for (k, v) in m {
                    out.insert(k.clone(), Value::String(v.clone()));
                }
            }
        }
        Value::Object(out)
    }
}

/// Register the `t` Tera function. The current language is read from the
/// `lang` context var that page handlers insert; if missing, English is used.
pub fn register_tera_function(tera: &mut tera::Tera, i18n: Arc<I18n>) {
    tera.register_function(
        "t",
        move |args: &HashMap<String, Value>| -> tera::Result<Value> {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| tera::Error::msg("t(key=...) requires a string key"))?;
            // The template can pass lang explicitly, or rely on the caller
            // having inserted a `lang` context var (pulled in via `lang=lang`).
            let lang = args.get("lang").and_then(|v| v.as_str()).unwrap_or("en");
            Ok(Value::String(i18n.t(lang, key)))
        },
    );
}

/// Convenience: given an Axum HeaderMap + CookieJar + optional `?lang=`, resolve the
/// language and inject `lang` and `i18n_js` into a Tera context. Returns the chosen lang.
/// Call this from every page handler before `tera.render`.
pub fn inject_into_context(
    i18n: &I18n,
    ctx: &mut tera::Context,
    headers: &axum::http::HeaderMap,
    jar: &axum_extra::extract::cookie::CookieJar,
    query_lang: Option<&str>,
) -> String {
    let cookie_lang = jar.get("anky_lang").map(|c| c.value().to_string());
    let accept_language = headers
        .get("accept-language")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let lang = resolve_request_lang(
        i18n,
        query_lang,
        cookie_lang.as_deref(),
        accept_language.as_deref(),
    );
    let bundle = i18n.client_bundle(&lang);
    ctx.insert("lang", &lang);
    ctx.insert("i18n_js", &bundle);
    lang
}

/// Resolve the request's language: ?lang=xx query → cookie → Accept-Language → en.
/// Returns a supported language code (falls back to "en" if no file exists).
pub fn resolve_request_lang(
    i18n: &I18n,
    query_lang: Option<&str>,
    cookie_lang: Option<&str>,
    accept_language: Option<&str>,
) -> String {
    if let Some(q) = query_lang {
        let primary = q.split('-').next().unwrap_or(q).trim();
        if !primary.is_empty() {
            return i18n.resolve(primary);
        }
    }
    if let Some(c) = cookie_lang {
        let primary = c.split('-').next().unwrap_or(c).trim();
        if !primary.is_empty() {
            return i18n.resolve(primary);
        }
    }
    if let Some(al) = accept_language {
        // e.g. "es-MX,es;q=0.9,en;q=0.8" → try each in order
        for part in al.split(',') {
            let code = part.split(';').next().unwrap_or("").trim();
            let primary = code.split('-').next().unwrap_or("").trim().to_lowercase();
            if !primary.is_empty() && i18n.has_language(&primary) {
                return primary;
            }
        }
    }
    "en".to_string()
}
