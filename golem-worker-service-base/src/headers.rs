use std::collections::HashMap;
use std::str::FromStr;

use golem_wasm_rpc::protobuf::TypedRecord;
use golem_wasm_rpc::protobuf::type_annotated_value::TypeAnnotatedValue;
use golem_wasm_rpc::json::TypeAnnotatedValueJsonExtensions;
use poem::web::headers::ContentType;
use rib::GetLiteralValue;

#[derive(Default, Debug, PartialEq)]
pub struct ResolvedResponseHeaders {
    headers: HeaderMap,
}

impl ResolvedResponseHeaders {
    pub fn from_typed_value(
        header_map: &TypeAnnotatedValue,
    ) -> Result<ResolvedResponseHeaders, String> {
        match header_map {
            TypeAnnotatedValue::Record(TypedRecord { value, .. }) => {
                let mut resolved_headers: HashMap<String, String> = HashMap::new();

                for name_value_pair in value {
                    let value_str = name_value_pair
                        .value
                        .as_ref()
                        .and_then(|v| v.type_annotated_value.clone())
                        .ok_or("Unable to resolve header value".to_string())?
                        .get_literal()
                        .map(|primitive| primitive.to_string())
                        .unwrap_or_else(|| "Unable to resolve header".to_string());

                    resolved_headers.insert(name_value_pair.name.clone(), value_str);
                }

                let headers: Result<HeaderMap, String> = (&resolved_headers)
                    .try_into()
                    .map_err(|e: hyper::http::Error| e.to_string())
                    .map_err(|e| format!("Unable to resolve valid headers. Error: {e}"))?;


                Ok(ResolvedResponseHeaders { headers })
            }

            _ => Err(format!(
                "Header expression is not a record. It is resolved to {}",
                header_map.to_json_value()
            )),
        }
    }

    pub fn get_content_type(&self) -> Option<ContentType> {
        self.headers
            .get(http::header::CONTENT_TYPE.to_string())
            .and_then(|header_value| {
                header_value
                    .to_str()
                    .ok()
                    .and_then(|header_str| ContentType::from_str(header_str).ok())
            })
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_get_response_headers_from_typed_value() {
        let header_map = create_record(vec![
            (
                "header1".to_string(),
                TypeAnnotatedValue::Str("value1".to_string()),
            ),
            ("header2".to_string(), TypeAnnotatedValue::F32(1.0)),
        ]);

        let resolved_headers = ResolvedResponseHeaders::from_typed_value(&header_map).unwrap();

        let mut map = HashMap::new();

        map.insert("header1".to_string(), "value1".to_string());
        map.insert("header2".to_string(), "1".to_string());

        let header_map: HeaderMap = map.try_into().unwrap();

        let expected = ResolvedResponseHeaders { headers: header_map };

        assert_eq!(resolved_headers, expected)
    }
}
