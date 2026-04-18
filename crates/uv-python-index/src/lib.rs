use serde::{Deserialize, Deserializer, Serialize};
use uv_redacted::DisplaySafeUrl;

/// A custom Python index for downloading managed Python installations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct PythonIndex {
    /// The name of the index.
    pub name: String,
    /// The URL of the index.
    pub url: DisplaySafeUrl,
    /// Whether this is the default index.
    #[serde(default)]
    pub default: bool,
}

impl<'de> Deserialize<'de> for PythonIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case", deny_unknown_fields)]
        struct Wire {
            name: String,
            url: DisplaySafeUrl,
            #[serde(default)]
            default: bool,
        }

        let Wire { name, url, default } = Wire::deserialize(deserializer)?;

        // `$`-prefixed names are reserved for internally synthesized entries (e.g. from
        // `UV_PYTHON_INDEX` or `--python-index`). Reject at deserialization time so an
        // invalid TOML config fails fast with a clear error.
        if name.starts_with('$') {
            return Err(serde::de::Error::custom(format!(
                "Python index name `{name}` uses the reserved `$` prefix; `$`-prefixed names are \
                synthesized internally (e.g. for `UV_PYTHON_INDEX` or `--python-index`)"
            )));
        }

        Ok(Self { name, url, default })
    }
}
