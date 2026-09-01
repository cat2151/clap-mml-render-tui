use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MmlFragment {
    pub(super) json: Option<Value>,
    pub(super) body: String,
}

impl MmlFragment {
    pub(super) fn empty() -> Self {
        Self {
            json: None,
            body: String::new(),
        }
    }
}

pub(super) fn split_mml_fragment(cell: &str) -> MmlFragment {
    use mmlabc_to_smf::mml_preprocessor;

    let cell = cell.trim();
    if cell.is_empty() {
        return MmlFragment::empty();
    }

    let preprocessed = mml_preprocessor::extract_embedded_json(cell);
    let json = preprocessed
        .embedded_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<Value>(json).ok());

    MmlFragment {
        json,
        body: preprocessed.remaining_mml.trim().to_string(),
    }
}

fn merge_json_object(target: &mut Map<String, Value>, source: Map<String, Value>) {
    for (key, value) in source {
        match target.get_mut(&key) {
            Some(existing) => merge_json_value(existing, value),
            None => {
                target.insert(key, value);
            }
        }
    }
}

fn merge_json_value(target: &mut Value, source: Value) {
    match (target, source) {
        (Value::Object(target), Value::Object(source)) => merge_json_object(target, source),
        (Value::Array(target), Value::Array(source)) => target.extend(source),
        (target, source) => *target = source,
    }
}

pub(super) fn merged_json_prefix(json_values: impl IntoIterator<Item = Value>) -> String {
    let mut merged = None::<Value>;
    for value in json_values {
        match &mut merged {
            Some(current) => merge_json_value(current, value),
            None => merged = Some(value),
        }
    }

    merged
        .and_then(|value| serde_json::to_string(&value).ok())
        .unwrap_or_default()
}

pub(super) fn append_fragment_json_values<'a>(
    json_values: &mut Vec<Value>,
    fragments: impl IntoIterator<Item = &'a MmlFragment>,
) {
    json_values.extend(
        fragments
            .into_iter()
            .filter_map(|fragment| fragment.json.clone()),
    );
}
