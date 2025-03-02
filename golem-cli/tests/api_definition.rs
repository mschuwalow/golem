// Copyright 2024 Golem Cloud
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use test_r::{add_test, inherit_test_dep, test_dep, test_gen};

use crate::cli::{Cli, CliLive};
use crate::worker::add_component_from_file;
use crate::Tracing;
use assert2::assert;
use chrono::{DateTime, Utc};
use golem_cli::model::component::ComponentView;
use golem_cli::model::ApiDefinitionFileFormat;
use golem_client::model::{
    GatewayBindingData, GatewayBindingType, GatewayBindingWithTypeInfo, HttpApiDefinitionRequest,
    HttpApiDefinitionWithTypeInfo, MethodPattern, RibInputTypeInfo, RouteData, RouteWithTypeInfo,
    VersionedComponentId,
};
use golem_test_framework::config::{EnvBasedTestDependencies, TestDependencies};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use test_r::core::{DynamicTestRegistration, TestType};
use uuid::Uuid;

inherit_test_dep!(EnvBasedTestDependencies);
inherit_test_dep!(Tracing);

#[test_dep]
fn cli(deps: &EnvBasedTestDependencies) -> CliLive {
    CliLive::make("gateway_api_definition", Arc::new(deps.clone())).unwrap()
}

#[test_gen]
fn generated(r: &mut DynamicTestRegistration) {
    make(r, "_short", "CLI_short", true);
    make(r, "_long", "CLI_long", false);
}

fn make(r: &mut DynamicTestRegistration, suffix: &'static str, name: &'static str, short: bool) {
    add_test!(
        r,
        format!("api_definition_json_import{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_import(
                (deps, name.to_string(), cli.with_args(short)),
                &ApiDefinitionFileFormat::Json,
            )
        }
    );
    add_test!(
        r,
        format!("api_definition_yaml_import{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_import(
                (deps, name.to_string(), cli.with_args(short)),
                &ApiDefinitionFileFormat::Yaml,
            )
        }
    );
    add_test!(
        r,
        format!("api_definition_yaml_add{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_add(
                (deps, name.to_string(), cli.with_args(short)),
                &ApiDefinitionFileFormat::Yaml,
            )
        }
    );
    add_test!(
        r,
        format!("api_definition_json_add{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_add(
                (deps, name.to_string(), cli.with_args(short)),
                &ApiDefinitionFileFormat::Json,
            )
        }
    );
    add_test!(
        r,
        format!("api_definition_yaml_update{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_update(
                (deps, name.to_string(), cli.with_args(short)),
                &ApiDefinitionFileFormat::Yaml,
            )
        }
    );
    add_test!(
        r,
        format!("api_definition_json_update{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_update(
                (deps, name.to_string(), cli.with_args(short)),
                &ApiDefinitionFileFormat::Json,
            )
        }
    );
    add_test!(
        r,
        format!("api_definition_update_immutable{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_update_immutable((deps, name.to_string(), cli.with_args(short)))
        }
    );
    add_test!(
        r,
        format!("api_definition_list{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_list((deps, name.to_string(), cli.with_args(short)))
        }
    );
    add_test!(
        r,
        format!("api_definition_list_versions{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_list_versions((deps, name.to_string(), cli.with_args(short)))
        }
    );
    add_test!(
        r,
        format!("api_definition_get{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_get((deps, name.to_string(), cli.with_args(short)))
        }
    );
    add_test!(
        r,
        format!("api_definition_delete{suffix}"),
        TestType::IntegrationTest,
        move |deps: &EnvBasedTestDependencies, cli: &CliLive, _tracing: &Tracing| {
            api_definition_delete((deps, name.to_string(), cli.with_args(short)))
        }
    );
}

pub fn make_shopping_cart_component(
    deps: &(impl TestDependencies + Send + Sync + 'static),
    component_name: &str,
    cli: &CliLive,
) -> anyhow::Result<ComponentView> {
    add_component_from_file(deps, component_name, cli, "shopping-cart.wasm")
}

pub fn make_json_file<T: Serialize>(id: &str, value: &T) -> anyhow::Result<PathBuf> {
    let text = serde_json::to_string_pretty(value)?;

    let path = PathBuf::from(format!("../target/{id}.json"));

    fs::write(&path, text)?;

    Ok(path)
}

pub fn make_yaml_file<T: Serialize>(id: &str, value: &T) -> anyhow::Result<PathBuf> {
    let text = serde_yaml::to_string(value)?;

    let path = PathBuf::from(format!("../target/{id}.yaml"));

    fs::write(&path, text)?;

    Ok(path)
}

fn golem_def_with_response(
    id: &str,
    component_id: &str,
    response: String,
) -> HttpApiDefinitionRequest {
    HttpApiDefinitionRequest {
        id: id.to_string(),
        version: "0.1.0".to_string(),
        draft: true,
        routes: vec![RouteData {
            method: MethodPattern::Get,
            path: "/{user-id}/get-cart-contents".to_string(),
            binding: GatewayBindingData {
                component_id: Some(VersionedComponentId {
                    component_id: Uuid::parse_str(component_id).unwrap(),
                    version: 0,
                }),
                worker_name: Some("\"foo\"".to_string()),
                idempotency_key: None,
                response: Some(response),
                middleware: None,
                allow_origin: None,
                allow_methods: None,
                allow_headers: None,
                expose_headers: None,
                binding_type: None,
                max_age: None,
                allow_credentials: None,
            },
        }],
    }
}

pub fn native_api_definition_request(id: &str, component_id: &str) -> HttpApiDefinitionRequest {
    golem_def_with_response(
        id,
        component_id,
        "let x = golem:it/api.{checkout}();\nlet status: u64 = 200;\n{headers: {ContentType: \"json\", userid: \"foo\"}, body: \"foo\", status: status}"
            .to_string(),
    )
}

pub fn make_open_api_yaml_file(
    id: &str,
    component_id: &str,
    component_version: u64,
) -> anyhow::Result<PathBuf> {
    // Use format! to interpolate variables into the YAML string.
    let yaml_open_api_str = format!(
        r#"
openapi: "3.0.0"
info:
  title: "Sample API"
  version: "1.0.3"
x-golem-api-definition-id: "{id}"
x-golem-api-definition-version: "0.1.0"
paths:
  "/{{user-id}}/get-cart-contents":
    get:
      x-golem-api-gateway-binding:
        worker-name: "\"foo\""
        component-id: "{component_id}"
        component-version: {component_version}
        response: |
          let x = golem:it/api.{{checkout}}();
          let status: u64 = 200;
          {{headers: {{ContentType: "json", userid: "foo"}}, body: "foo", status: status}}
      summary: "Get Cart Contents"
      description: "Get the contents of a user's cart"
      parameters:
        - name: "user-id"
          in: "path"
          required: true
          schema:
            type: "string"
      responses:
        "404":
          description: "Contents not found"
"#,
        component_id = component_id,
        component_version = component_version,
    );

    // Parse YAML string into a serde_yaml::Value or another type
    let open_api_yaml: serde_json::Value = serde_yaml::from_str(&yaml_open_api_str)?;

    // Implement your `make_file` logic here to save `open_api_yaml` to a file
    make_yaml_file(id, &open_api_yaml)
}

pub fn make_open_api_json_file(
    id: &str,
    component_id: &str,
    component_version: u64,
) -> anyhow::Result<PathBuf> {
    let open_api_json = json!(
      {
        "openapi": "3.0.0",
        "info": {
          "title": "Sample API",
          "version": "1.0.2"
        },
        "x-golem-api-definition-id": id,
        "x-golem-api-definition-version": "0.1.0",
        "paths": {
            "/{user-id}/get-cart-contents": {
              "get": {
                  "x-golem-api-gateway-binding": {
                     "binding-type" : "default",
                     "worker-name": "\"foo\"",
                     "component-id": component_id,
                     "component-version": component_version,
                     "response" : "let x = golem:it/api.{checkout}();\nlet status: u64 = 200; {headers : {ContentType: \"json\", userid: \"foo\"}, body: \"foo\", status: status}"
                  },
                "summary": "Get Cart Contents",
                "description": "Get the contents of a user's cart",
                "parameters": [
                  {
                    "name": "user-id",
                    "in": "path",
                    "required": true,
                    "schema": {
                      "type": "string"
                    }
                  }
                ],
                "responses": {
                  "200": {
                    "description": "OK",
                    "content":{
                      "application/json": {
                        "schema": {
                          "$ref": "#/components/schemas/CartItem"
                        }
                      }

                    }
                  },
                  "404": {
                    "description": "Contents not found"
                  }
                }
              }
            }
          },
          "components": {
            "schemas": {
              "CartItem": {
                "type": "object",
                "properties": {
                  "id": {
                    "type": "string"
                  },
                  "name": {
                    "type": "string"
                  },
                  "price": {
                    "type": "number"
                  }
                }
              }
            }
        }
        }
    );

    make_json_file(id, &open_api_json)
}

pub fn to_api_definition_with_type_info(
    request: HttpApiDefinitionRequest,
    created_at: Option<DateTime<Utc>>,
) -> HttpApiDefinitionWithTypeInfo {
    HttpApiDefinitionWithTypeInfo {
        id: request.id,
        version: request.version,
        draft: request.draft,
        routes: request
            .routes
            .iter()
            .map(|v0| {
                let v = v0.clone();

                RouteWithTypeInfo {
                    method: v.method,
                    path: v.path,
                    binding: GatewayBindingWithTypeInfo {
                        component_id: v.binding.component_id,
                        worker_name: v.binding.worker_name.clone(),
                        idempotency_key: v.binding.idempotency_key.clone(),
                        response: v.binding.response,
                        response_mapping_input: Some(RibInputTypeInfo {
                            types: HashMap::new(),
                        }),
                        worker_name_input: Some(RibInputTypeInfo {
                            types: HashMap::new(),
                        }),
                        idempotency_key_input: None,
                        binding_type: Some(GatewayBindingType::Default),
                        cors_preflight: None,
                    },
                }
            })
            .collect(),
        created_at,
    }
}

fn api_definition_import(
    (deps, name, cli): (
        &(impl TestDependencies + Send + Sync + 'static),
        String,
        CliLive,
    ),
    api_definition_format: &ApiDefinitionFileFormat,
) -> anyhow::Result<()> {
    let component_name = format!("api_definition_{api_definition_format}_import{name}");
    let component = make_shopping_cart_component(deps, &component_name, &cli)?;
    let component_id = component.component_urn.id.0.to_string();
    let component_version = component.component_version;

    let res: HttpApiDefinitionWithTypeInfo = match api_definition_format {
        ApiDefinitionFileFormat::Json => {
            let path = make_open_api_json_file(&component_name, &component_id, component_version)?;
            cli.run(&["api-definition", "import", path.to_str().unwrap()])?
        }
        ApiDefinitionFileFormat::Yaml => {
            let path = make_open_api_yaml_file(&component_name, &component_id, component_version)?;
            cli.run(&[
                "api-definition",
                "import",
                "--def-format",
                "yaml",
                path.to_str().unwrap(),
            ])?
        }
    };

    let expected = to_api_definition_with_type_info(
        native_api_definition_request(&component_name, &component_id),
        res.created_at,
    );

    assert_eq!(res, expected);

    Ok(())
}

fn api_definition_add(
    (deps, name, cli): (
        &(impl TestDependencies + Send + Sync + 'static),
        String,
        CliLive,
    ),
    api_definition_format: &ApiDefinitionFileFormat,
) -> anyhow::Result<()> {
    let component_name = format!("api_definition_{api_definition_format}_add{name}");
    let component = make_shopping_cart_component(deps, &component_name, &cli)?;
    let component_id = component.component_urn.id.0.to_string();
    let def = native_api_definition_request(&component_name, &component_id);

    let res: HttpApiDefinitionWithTypeInfo = match api_definition_format {
        ApiDefinitionFileFormat::Json => {
            let path = make_json_file(&def.id, &def)?;
            cli.run(&["api-definition", "add", path.to_str().unwrap()])?
        }
        ApiDefinitionFileFormat::Yaml => {
            let path = make_yaml_file(&def.id, &def)?;
            cli.run(&[
                "api-definition",
                "add",
                "--def-format",
                "yaml",
                path.to_str().unwrap(),
            ])?
        }
    };

    let expected = to_api_definition_with_type_info(def, res.created_at);

    assert_eq!(res, expected);

    Ok(())
}

fn api_definition_update(
    (deps, name, cli): (
        &(impl TestDependencies + Send + Sync + 'static),
        String,
        CliLive,
    ),
    api_definition_format: &ApiDefinitionFileFormat,
) -> anyhow::Result<()> {
    let component_name = format!("api_definition_{api_definition_format}_update{name}");
    let component = make_shopping_cart_component(deps, &component_name, &cli)?;
    let component_id = component.component_urn.id.0.to_string();

    let def = native_api_definition_request(&component_name, &component_id);
    let path = make_json_file(&def.id, &def)?;
    let _: HttpApiDefinitionWithTypeInfo =
        cli.run(&["api-definition", "add", path.to_str().unwrap()])?;

    let updated = golem_def_with_response(
        &component_name,
        &component_id,
        "let status: u64 = 200;\n{headers: {ContentType: \"json\", userid: \"bar\"}, body: \"baz\", status: status}"
            .to_string(),
    );

    let res: HttpApiDefinitionWithTypeInfo = match api_definition_format {
        ApiDefinitionFileFormat::Json => {
            let path = make_json_file(&updated.id, &updated)?;
            cli.run(&["api-definition", "update", path.to_str().unwrap()])?
        }
        ApiDefinitionFileFormat::Yaml => {
            let path = make_yaml_file(&updated.id, &updated)?;
            cli.run(&[
                "api-definition",
                "update",
                "--def-format",
                "yaml",
                path.to_str().unwrap(),
            ])?
        }
    };

    let expected = to_api_definition_with_type_info(updated, res.created_at);

    assert_eq!(res, expected);

    Ok(())
}

fn api_definition_update_immutable(
    (deps, name, cli): (
        &(impl TestDependencies + Send + Sync + 'static),
        String,
        CliLive,
    ),
) -> anyhow::Result<()> {
    let component_name = format!("api_definition_update_immutable{name}");
    let component = make_shopping_cart_component(deps, &component_name, &cli)?;
    let component_id = component.component_urn.id.0.to_string();

    let mut def = native_api_definition_request(&component_name, &component_id);
    def.draft = false;
    let path = make_json_file(&def.id, &def)?;
    let _: HttpApiDefinitionWithTypeInfo =
        cli.run(&["api-definition", "add", path.to_str().unwrap()])?;

    let updated = golem_def_with_response(&component_name, &component_id, "${let status: u64 = 200; {headers: {ContentType: \"json\", userid: \"bar\"}, body: worker.response, status: status}}".to_string());
    let path = make_json_file(&updated.id, &updated)?;
    let res = cli.run_string(&["api-definition", "update", path.to_str().unwrap()]);

    assert!(res.is_err());

    Ok(())
}

fn api_definition_list(
    (deps, name, cli): (
        &(impl TestDependencies + Send + Sync + 'static),
        String,
        CliLive,
    ),
) -> anyhow::Result<()> {
    let component_name = format!("api_definition_list{name}");
    let component = make_shopping_cart_component(deps, &component_name, &cli)?;
    let component_id = component.component_urn.id.0.to_string();
    let def = native_api_definition_request(&component_name, &component_id);
    let path = make_json_file(&def.id, &def)?;

    let _: HttpApiDefinitionWithTypeInfo =
        cli.run(&["api-definition", "add", path.to_str().unwrap()])?;

    let res: Vec<HttpApiDefinitionWithTypeInfo> = cli.run(&["api-definition", "list"])?;

    let found = res.into_iter().find(|d| {
        let e = to_api_definition_with_type_info(def.clone(), d.created_at);
        d == &e
    });

    assert!(found.is_some());

    Ok(())
}

fn api_definition_list_versions(
    (deps, name, cli): (
        &(impl TestDependencies + Send + Sync + 'static),
        String,
        CliLive,
    ),
) -> anyhow::Result<()> {
    let component_name = format!("api_definition_list_versions{name}");
    let component = make_shopping_cart_component(deps, &component_name, &cli)?;
    let component_id = component.component_urn.id.0.to_string();
    let def = native_api_definition_request(&component_name, &component_id);
    let path = make_json_file(&def.id, &def)?;
    let cfg = &cli.config;

    let _: HttpApiDefinitionWithTypeInfo =
        cli.run(&["api-definition", "add", path.to_str().unwrap()])?;

    let res: Vec<HttpApiDefinitionWithTypeInfo> = cli.run(&[
        "api-definition",
        "list",
        &cfg.arg('i', "id"),
        &component_name,
    ])?;

    assert_eq!(res.len(), 1);

    let res: HttpApiDefinitionWithTypeInfo = res.first().unwrap().clone();
    let expected = to_api_definition_with_type_info(def, res.created_at);

    assert_eq!(res, expected);

    Ok(())
}

fn api_definition_get(
    (deps, name, cli): (
        &(impl TestDependencies + Send + Sync + 'static),
        String,
        CliLive,
    ),
) -> anyhow::Result<()> {
    let component_name = format!("api_definition_get{name}");
    let component = make_shopping_cart_component(deps, &component_name, &cli)?;
    let component_id = component.component_urn.id.0.to_string();
    let def = native_api_definition_request(&component_name, &component_id);
    let path = make_json_file(&def.id, &def)?;

    let _: HttpApiDefinitionWithTypeInfo =
        cli.run(&["api-definition", "add", path.to_str().unwrap()])?;

    let cfg = &cli.config;

    let res: HttpApiDefinitionWithTypeInfo = cli.run(&[
        "api-definition",
        "get",
        &cfg.arg('i', "id"),
        &component_name,
        &cfg.arg('V', "version"),
        "0.1.0",
    ])?;

    let expected = to_api_definition_with_type_info(def, res.created_at);

    assert_eq!(res, expected);

    Ok(())
}

fn api_definition_delete(
    (deps, name, cli): (
        &(impl TestDependencies + Send + Sync + 'static),
        String,
        CliLive,
    ),
) -> anyhow::Result<()> {
    let component_name = format!("api_definition_delete{name}");
    let component = make_shopping_cart_component(deps, &component_name, &cli)?;
    let component_id = component.component_urn.id.0.to_string();
    let def = native_api_definition_request(&component_name, &component_id);
    let path = make_json_file(&def.id, &def)?;

    let _: HttpApiDefinitionWithTypeInfo =
        cli.run(&["api-definition", "add", path.to_str().unwrap()])?;

    let cfg = &cli.config;

    let res: HttpApiDefinitionWithTypeInfo = cli.run(&[
        "api-definition",
        "get",
        &cfg.arg('i', "id"),
        &component_name,
        &cfg.arg('V', "version"),
        "0.1.0",
    ])?;

    let expected = to_api_definition_with_type_info(def, res.created_at);

    assert_eq!(res, expected);

    cli.run_unit(&[
        "api-definition",
        "delete",
        &cfg.arg('i', "id"),
        &component_name,
        &cfg.arg('V', "version"),
        "0.1.0",
    ])?;

    let res_list: Vec<HttpApiDefinitionWithTypeInfo> = cli.run(&[
        "api-definition",
        "list",
        &cfg.arg('i', "id"),
        &component_name,
    ])?;

    assert!(res_list.is_empty());

    Ok(())
}
