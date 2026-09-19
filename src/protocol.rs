use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Ping,
    Spawn { name: String, command: Vec<String> },
    List,
    Send { name: String, text: String },
    Read { name: String, lines: Option<usize> },
    Kill { name: String },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PaneStatus {
    Running,
    Exited { code: u32 },
}

#[derive(Serialize, Deserialize)]
pub struct PaneInfo {
    pub name: String,
    pub command: Vec<String>,
    #[serde(flatten)]
    pub status: PaneStatus,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Response {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panes: Vec<PaneInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// The pane's process's current working directory, best-effort (macOS
    /// `lsof`-derived — absent if the lookup fails or the process exited).
    /// Only populated on `Read` responses, alongside `output`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

impl Response {
    pub fn ok() -> Self {
        Response {
            ok: true,
            ..Default::default()
        }
    }

    pub fn ok_with_panes(panes: Vec<PaneInfo>) -> Self {
        Response {
            ok: true,
            panes,
            ..Default::default()
        }
    }

    pub fn ok_with_output(output: String, cwd: Option<String>) -> Self {
        Response {
            ok: true,
            output: Some(output),
            cwd,
            ..Default::default()
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Response {
            ok: false,
            error: Some(message.into()),
            ..Default::default()
        }
    }
}
