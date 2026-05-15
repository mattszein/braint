//! Output formatting — human-readable vs NDJSON.

use serde::Serialize;

pub enum OutputMode {
    Human,
    Ndjson,
}

impl OutputMode {
    pub fn from_flag(json: bool) -> Self {
        if json { Self::Ndjson } else { Self::Human }
    }
}

pub fn print_id(label: &str, id: &str, mode: &OutputMode) {
    match mode {
        OutputMode::Human => println!("{id}"),
        OutputMode::Ndjson => {
            let v = serde_json::json!({ "type": label, "id": id });
            println!("{v}");
        }
    }
}

pub fn print_json<T: Serialize>(value: &T) {
    if let Ok(s) = serde_json::to_string(value) {
        println!("{s}");
    }
}
