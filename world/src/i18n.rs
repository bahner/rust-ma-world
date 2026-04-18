use anyhow::{anyhow, Context, Result};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

const NB_NO_FTL: &str = r#"
world-online = world online: did={$did} endpoint={$endpoint} services={$services}
publish-ok-source = publisert did={$did} kilde={$source}
publish-ok-alias = publisert did={$did} cid={$cid} alias={$alias}
inbox-received = mottok inbox: {$from} -> {$to} type={$content_type} id={$id}
ipfs-received = mottok ipfs: {$from} -> {$to} type={$content_type} id={$id}
ipfs-reply = ipfs svar: til={$to} status={$status} code={$code} id={$id} type={$content_type}
"#;

pub struct Localizer {
    bundle: FluentBundle<FluentResource>,
    language: String,
}

impl Localizer {
    pub fn new(config_language: Option<&str>) -> Result<Self> {
        let selected_language = normalize_language(config_language);
        let lang_id: LanguageIdentifier = selected_language
            .parse()
            .with_context(|| format!("invalid language tag: {selected_language}"))?;

        let mut bundle = FluentBundle::new(vec![lang_id]);
        let resource = FluentResource::try_new(NB_NO_FTL.to_string())
            .map_err(|error| anyhow!("failed to parse nb_NO fluent resource: {error:?}"))?;
        bundle
            .add_resource(resource)
            .map_err(|_| anyhow!("failed to add nb_NO fluent resource"))?;

        Ok(Self {
            bundle,
            language: selected_language.to_string(),
        })
    }

    pub fn language(&self) -> &str {
        &self.language
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

    pub fn publish_ok_source(&self, did: &str, source: &str) -> String {
        self.format("publish-ok-source", [("did", did), ("source", source)])
    }

    pub fn publish_ok_alias(&self, did: &str, cid: &str, alias: &str) -> String {
        self.format(
            "publish-ok-alias",
            [("did", did), ("cid", cid), ("alias", alias)],
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

fn normalize_language(config_language: Option<&str>) -> &'static str {
    let Some(language) = config_language else {
        return "nb-NO";
    };

    let normalized = language.trim().replace('_', "-").to_ascii_lowercase();
    if normalized == "nb" || normalized == "nb-no" {
        "nb-NO"
    } else {
        "nb-NO"
    }
}
