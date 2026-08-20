use codex_tools::AdditionalProperties;
use codex_tools::JsonSchema;
use codex_tools::JsonSchemaPrimitiveType;
use codex_tools::JsonSchemaType;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SchemaViolation {
    pub(super) path: String,
    pub(super) reason: String,
}

pub(super) fn validate_json_schema(
    value: &Value,
    schema: &JsonSchema,
) -> Result<(), SchemaViolation> {
    validate_at(value, schema, schema, "$")
}

fn validate_at(
    value: &Value,
    schema: &JsonSchema,
    root: &JsonSchema,
    path: &str,
) -> Result<(), SchemaViolation> {
    if let Some(reference) = schema.schema_ref.as_deref() {
        let target = resolve_local_reference(root, reference).ok_or_else(|| SchemaViolation {
            path: path.to_string(),
            reason: format!("unresolved schema reference `{reference}`"),
        })?;
        return validate_at(value, target, root, path);
    }
    if let Some(variants) = schema.any_of.as_ref() {
        let discriminated = discriminated_variants(value, variants);
        let candidates = if discriminated.is_empty() {
            variants.iter().collect::<Vec<_>>()
        } else {
            discriminated
        };
        let mut violations = Vec::new();
        for variant in candidates {
            match validate_at(value, variant, root, path) {
                Ok(()) => return Ok(()),
                Err(violation) => violations.push(violation),
            }
        }
        return Err(violations
            .into_iter()
            .max_by_key(|violation| violation.path.len())
            .unwrap_or_else(|| SchemaViolation {
                path: path.to_string(),
                reason: "value does not match any allowed schema variant".into(),
            }));
    }
    if let Some(schema_type) = schema.schema_type.as_ref()
        && !matches_type(value, schema_type)
    {
        return Err(SchemaViolation {
            path: path.to_string(),
            reason: "value has the wrong JSON type".into(),
        });
    }
    if let Some(allowed) = schema.enum_values.as_ref()
        && !allowed.contains(value)
    {
        return Err(SchemaViolation {
            path: path.to_string(),
            reason: "value is not in the allowed enum".into(),
        });
    }
    if let Some(object) = value.as_object() {
        validate_object(object, schema, root, path)?;
    }
    if let Some(array) = value.as_array() {
        if let Some(min_items) = schema.min_items
            && array.len() < min_items
        {
            return Err(SchemaViolation {
                path: path.to_string(),
                reason: format!("array requires at least {min_items} item(s)"),
            });
        }
        if let Some(items) = schema.items.as_deref() {
            for (index, item) in array.iter().enumerate() {
                validate_at(item, items, root, &format!("{path}[{index}]"))?;
            }
        }
    }
    if let Some(minimum) = schema.minimum.as_ref()
        && value
            .as_number()
            .is_some_and(|number| number_is_below_minimum(number, minimum))
    {
        return Err(SchemaViolation {
            path: path.to_string(),
            reason: format!("number is below minimum {minimum}"),
        });
    }
    Ok(())
}

fn number_is_below_minimum(number: &serde_json::Number, minimum: &serde_json::Number) -> bool {
    match (number.as_i64(), minimum.as_i64()) {
        (Some(number), Some(minimum)) => number < minimum,
        _ => match (number.as_u64(), minimum.as_u64()) {
            (Some(number), Some(minimum)) => number < minimum,
            _ => match (number.as_f64(), minimum.as_f64()) {
                (Some(number), Some(minimum)) => number < minimum,
                _ => false,
            },
        },
    }
}

fn discriminated_variants<'a>(value: &Value, variants: &'a [JsonSchema]) -> Vec<&'a JsonSchema> {
    ["type", "tool"]
        .into_iter()
        .find_map(|field| {
            let actual = value.get(field)?;
            let matching = variants
                .iter()
                .filter(|variant| {
                    variant
                        .properties
                        .as_ref()
                        .and_then(|properties| properties.get(field))
                        .and_then(|schema| schema.enum_values.as_ref())
                        .is_some_and(|allowed| allowed.contains(actual))
                })
                .collect::<Vec<_>>();
            (!matching.is_empty()).then_some(matching)
        })
        .unwrap_or_default()
}

fn validate_object(
    object: &serde_json::Map<String, Value>,
    schema: &JsonSchema,
    root: &JsonSchema,
    path: &str,
) -> Result<(), SchemaViolation> {
    if let Some(required) = schema.required.as_ref() {
        for name in required {
            if !object.contains_key(name) {
                return Err(SchemaViolation {
                    path: path.to_string(),
                    reason: format!("required property `{name}` is missing"),
                });
            }
        }
    }
    let properties = schema.properties.as_ref();
    for (name, value) in object {
        if let Some(property_schema) = properties.and_then(|properties| properties.get(name)) {
            validate_at(value, property_schema, root, &format!("{path}.{name}"))?;
            continue;
        }
        match schema.additional_properties.as_ref() {
            Some(AdditionalProperties::Boolean(false)) => {
                return Err(SchemaViolation {
                    path: format!("{path}.{name}"),
                    reason: format!("unknown field `{name}`"),
                });
            }
            Some(AdditionalProperties::Schema(additional_schema)) => {
                validate_at(value, additional_schema, root, &format!("{path}.{name}"))?;
            }
            Some(AdditionalProperties::Boolean(true)) | None => {}
        }
    }
    Ok(())
}

fn resolve_local_reference<'a>(root: &'a JsonSchema, reference: &str) -> Option<&'a JsonSchema> {
    let name = reference.strip_prefix("#/$defs/")?;
    root.defs.as_ref().or(root.definitions.as_ref())?.get(name)
}

fn matches_type(value: &Value, schema_type: &JsonSchemaType) -> bool {
    match schema_type {
        JsonSchemaType::Single(expected) => matches_primitive(value, *expected),
        JsonSchemaType::Multiple(expected) => expected
            .iter()
            .any(|expected| matches_primitive(value, *expected)),
    }
}

fn matches_primitive(value: &Value, expected: JsonSchemaPrimitiveType) -> bool {
    match expected {
        JsonSchemaPrimitiveType::String => value.is_string(),
        JsonSchemaPrimitiveType::Number => value.is_number(),
        JsonSchemaPrimitiveType::Boolean => value.is_boolean(),
        JsonSchemaPrimitiveType::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
        JsonSchemaPrimitiveType::Object => value.is_object(),
        JsonSchemaPrimitiveType::Array => value.is_array(),
        JsonSchemaPrimitiveType::Null => value.is_null(),
    }
}
