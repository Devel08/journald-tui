use serde::{Deserialize, Deserializer};

/// The data structure that we'll use for a single journald entry
#[derive(Debug, Deserialize, Clone)]
pub struct JournalEntry {
    // Time in unix epoch
    #[serde(
        rename = "__REALTIME_TIMESTAMP",
        deserialize_with = "deserialize_string_lossy"
    )]
    pub timestamp: String,

    // Log message
    #[serde(rename = "MESSAGE", deserialize_with = "deserialize_string_lossy")]
    pub message: String,

    // Systemd unit
    #[serde(rename = "_SYSTEMD_UNIT")]
    pub systemd_unit: Option<String>,

    // Name of process
    #[serde(rename = "_COMM")]
    pub process_name: Option<String>,

    // Priority of a given log
    #[serde(
        rename = "PRIORITY",
        deserialize_with = "deserialize_option_string_lossy"
    )]
    pub priority: Option<String>,

    // PID of the process
    #[serde(rename = "_PID")]
    pub pid: Option<String>,

    // The hostname of the machine
    #[serde(rename = "_HOSTNAME")]
    pub hostname: Option<String>,
}

/// Helper function to handle strings, integers, byte arrays from journalctl JSON
fn deserialize_string_lossy<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrBytes {
        String(String),
        Number(serde_json::Value),
        Bytes(Vec<u8>),
    }

    match StringOrBytes::deserialize(deserializer)? {
        StringOrBytes::String(s) => Ok(s),
        StringOrBytes::Number(n) => Ok(n.to_string()),
        StringOrBytes::Bytes(b) => Ok(String::from_utf8_lossy(&b).into_owned()),
    }
}

/// Helper function to handle optional fields that can be strings, integers, or byte arrays
fn deserialize_option_string_lossy<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionStringOrBytes {
        None,
        String(String),
        Number(serde_json::Value),
        Bytes(Vec<u8>),
    }

    match OptionStringOrBytes::deserialize(deserializer)? {
        OptionStringOrBytes::None => Ok(None),
        OptionStringOrBytes::String(s) => Ok(Some(s)),
        OptionStringOrBytes::Number(n) => Ok(Some(n.to_string())),
        OptionStringOrBytes::Bytes(b) => Ok(Some(String::from_utf8_lossy(&b).into_owned())),
    }
}
