pub(super) fn coerce_json_schema(
    schema: &serde_json::Value,
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if let Some(value) = coerce_combinator(schema, value.clone())? {
        return Ok(value);
    }
    let Some(kind) = schema.get("type").and_then(serde_json::Value::as_str) else {
        return Ok(value);
    };
    match kind {
        "object" => coerce_object(schema, value),
        "array" => {
            let serde_json::Value::Array(items) = value else {
                return Err("root: expected array".into());
            };
            let item_schema = schema.get("items").unwrap_or(&serde_json::Value::Null);
            items
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    coerce_json_schema(item_schema, item)
                        .map_err(|error| format!("{index}.{error}"))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array)
        }
        "string" | "number" | "integer" | "boolean" | "null" => coerce_scalar(kind, value),
        _ => Ok(value),
    }
}

pub(super) fn coerce_combinator(
    schema: &serde_json::Value,
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if let Some(schemas) = schema.get("allOf").and_then(serde_json::Value::as_array) {
        return schemas
            .iter()
            .try_fold(value, |value, schema| coerce_json_schema(schema, value))
            .map(Some);
    }
    for keyword in ["anyOf", "oneOf"] {
        let Some(schemas) = schema.get(keyword).and_then(serde_json::Value::as_array) else {
            continue;
        };
        let matches = schemas
            .iter()
            .filter_map(|branch| coerce_json_schema(branch, value.clone()).ok())
            .collect::<Vec<_>>();
        if (keyword == "anyOf" && !matches.is_empty()) || (keyword == "oneOf" && matches.len() == 1)
        {
            return Ok(Some(
                matches.into_iter().next().expect("schema match exists"),
            ));
        }
        return Err(format!("root: does not match {keyword}"));
    }
    Ok(None)
}

pub(super) fn coerce_scalar(
    kind: &str,
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match kind {
        "string" => Ok(serde_json::Value::String(match value {
            serde_json::Value::String(text) => text,
            serde_json::Value::Null => "null".into(),
            other => other.to_string(),
        })),
        "number" | "integer" => coerce_number(kind, value),
        "boolean" => coerce_boolean(value),
        "null"
            if matches!(
                value,
                serde_json::Value::Null | serde_json::Value::Bool(false)
            ) || value.as_i64() == Some(0)
                || value.as_str() == Some("") =>
        {
            Ok(serde_json::Value::Null)
        }
        "null" => Err("root: expected null".into()),
        _ => Ok(value),
    }
}

pub(super) fn coerce_object(
    schema: &serde_json::Value,
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let serde_json::Value::Object(mut object) = value else {
        return Err("root: expected object".into());
    };
    let properties = schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (name, property_schema) in properties {
        if let Some(property) = object.remove(&name) {
            object.insert(name, coerce_json_schema(&property_schema, property)?);
        }
    }
    if let Some(required) = schema.get("required").and_then(serde_json::Value::as_array) {
        for name in required.iter().filter_map(serde_json::Value::as_str) {
            if !object.contains_key(name) {
                return Err(format!("{name}: is required"));
            }
        }
    }
    Ok(serde_json::Value::Object(object))
}

pub(super) fn coerce_number(
    kind: &str,
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let number = match value {
        serde_json::Value::Number(number) => number,
        serde_json::Value::Bool(value) => serde_json::Number::from(if value { 1 } else { 0 }),
        serde_json::Value::Null => serde_json::Number::from(0),
        serde_json::Value::String(value) if kind == "integer" => value
            .parse::<i64>()
            .map(serde_json::Number::from)
            .map_err(|_| "root: expected integer".to_string())?,
        serde_json::Value::String(value) => value
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .ok_or_else(|| format!("root: expected {kind}"))?,
        _ => return Err(format!("root: expected {kind}")),
    };
    if kind == "integer" && number.as_i64().is_none() {
        return Err("root: expected integer".into());
    }
    Ok(serde_json::Value::Number(number))
}

pub(super) fn coerce_boolean(value: serde_json::Value) -> Result<serde_json::Value, String> {
    let value = match value {
        serde_json::Value::Bool(value) => value,
        serde_json::Value::Null => false,
        serde_json::Value::Number(value) => value != serde_json::Number::from(0),
        serde_json::Value::String(value) if value.eq_ignore_ascii_case("true") => true,
        serde_json::Value::String(value) if value.eq_ignore_ascii_case("false") => false,
        _ => return Err("root: expected boolean".into()),
    };
    Ok(serde_json::Value::Bool(value))
}
