use anyhow::{anyhow, Context, Result};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

const NB_NO_FTL: &str = include_str!("../locales/nb_NO.ftl");
const EN_UK_FTL: &str = include_str!("../locales/en_UK.ftl");

pub const DEFAULT_LOCALE: &str = "nb_NO";

pub struct Localizer {
    bundle: FluentBundle<FluentResource>,
    locale: String,
}

impl Localizer {
    pub fn new(config_locale: Option<&str>) -> Result<Self> {
        let selected_locale = normalize_locale(config_locale);
        let lang_id: LanguageIdentifier = selected_locale.langid
            .parse()
            .with_context(|| format!("invalid locale tag: {}", selected_locale.langid))?;

        let mut bundle = FluentBundle::new(vec![lang_id]);
        let resource = FluentResource::try_new(selected_locale.ftl.to_string())
            .map_err(|error| anyhow!("failed to parse {} fluent resource: {error:?}", selected_locale.key))?;
        bundle
            .add_resource(resource)
            .map_err(|_| anyhow!("failed to add {} fluent resource", selected_locale.key))?;

        Ok(Self {
            bundle,
            locale: selected_locale.key.to_string(),
        })
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn world_online(&self, did: &str, endpoint: &str, services: &str) -> String {
        self.format(
            "world-online",
            [
                ("did", did),
                ("endpoint", endpoint),
                ("services", services),
            ],
        )
    }

    pub fn inbox_received(&self, from: &str, to: &str, content_type: &str, id: &str) -> String {
        self.format(
            "inbox-received",
            [
                ("from", from),
                ("to", to),
                ("content_type", content_type),
                ("id", id),
            ],
        )
    }

    pub fn ipfs_received(&self, from: &str, to: &str, content_type: &str, id: &str) -> String {
        self.format(
            "ipfs-received",
            [
                ("from", from),
                ("to", to),
                ("content_type", content_type),
                ("id", id),
            ],
        )
    }

    pub fn ipfs_reply(&self, to: &str, status: u16, code: &str, id: &str, content_type: &str) -> String {
        self.format(
            "ipfs-reply",
            [
                ("to", to),
                ("status", &status.to_string()),
                ("code", code),
                ("id", id),
                ("content_type", content_type),
            ],
        )
    }

    pub fn cli_usage(&self) -> String {
        self.format("cli-usage", [])
    }

    pub fn cli_missing_value(&self, flag: &str) -> String {
        self.format("cli-missing-value", [("flag", flag)])
    }

    pub fn cli_unknown_argument(&self, arg: &str) -> String {
        self.format("cli-unknown-argument", [("arg", arg)])
    }

    pub fn generated_headless_config(&self, path: &str) -> String {
        self.format("generated-headless-config", [("path", path)])
    }

    fn format<const N: usize>(&self, message_id: &str, pairs: [(&str, &str); N]) -> String {
        let mut args = FluentArgs::new();
        for (key, value) in pairs {
            args.set(key, value);
        }

        let Some(message) = self.bundle.get_message(message_id) else {
            return message_id.to_string();
        };
        let Some(pattern) = message.value() else {
            return message_id.to_string();
        };

        let mut errors = Vec::new();
        let value = self
            .bundle
            .format_pattern(pattern, Some(&args), &mut errors)
            .to_string();

        if errors.is_empty() {
            value
        } else {
            message_id.to_string()
        }
    }
}

struct LanguageChoice {
    key: &'static str,
    langid: &'static str,
    ftl: &'static str,
}

fn normalize_locale(config_locale: Option<&str>) -> LanguageChoice {
    let Some(locale) = config_locale else {
        return LanguageChoice {
            key: DEFAULT_LOCALE,
            langid: "nb-NO",
            ftl: NB_NO_FTL,
        };
    };

    let normalized = locale.trim().replace('-', "_").to_ascii_lowercase();
    match normalized.as_str() {
        "en" | "en_uk" | "en_gb" => LanguageChoice {
            key: "en_UK",
            langid: "en-GB",
            ftl: EN_UK_FTL,
        },
        "nb" | "nb_no" => LanguageChoice {
            key: "nb_NO",
            langid: "nb-NO",
            ftl: NB_NO_FTL,
        },
        _ => LanguageChoice {
            key: DEFAULT_LOCALE,
            langid: "nb-NO",
            ftl: NB_NO_FTL,
        },
    }
}
