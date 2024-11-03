use std::{pin::Pin, str::FromStr, sync::Arc};

use bytes::Bytes;
use futures::Stream;
use golem_common::model::{component_metadata, InitialComponentFilePath};
use golem_service_base::{auth::EmptyAuthCtx, service::initial_component_files::InitialComponentFilesService};
use mime_guess::Mime;
use poem::web::{headers::ContentType, Path};
use rib::RibResult;
use async_trait::async_trait;
use testcontainers_modules::postgres;
use crate::service::component::ComponentService;
use super::WorkerDetail;
use golem_wasm_rpc::protobuf::type_annotated_value::TypeAnnotatedValue;
use crate::getter::{Getter, GetterExt};

pub struct FileServerBindingResult {
    pub original_result: RibResult,
    pub data: Pin<Box<dyn Stream<Item = Bytes>>>
}

pub struct FileServerBindingDetails {
    mime_type: Mime,
    status_code: u16,
    file_path: InitialComponentFilePath,
}

impl FileServerBindingDetails {
    pub fn from_rib(result: RibResult) -> Result<FileServerBindingDetails, String> {
        // Three supported formats:
        // 1. A string path. Mime type is guessed from the path. Status code is 200.
        // 2. A record with a 'file-path' field. Mime type and status are optionally taken from the record, otherwise guessed.
        // 3. A result of either of the above, with the same rules applied.
        // match result {

        // }

        todo!()


        // let (path_value, response_details) = match value {
        //     // Allow evaluating to a single string as a shortcut...
        //     path @ TypeAnnotatedValue::Str(_) => (path, None),
        //     // ...Or a Result<String, String>
        //     TypeAnnotatedValue::Result(res) =>
        //         match res.result_value.ok_or("result not set")? {
        //             ResultValue::OkValue(ok) => (ok.type_annotated_value.ok_or("ok unset")?, None),
        //             ResultValue::ErrorValue(err) => {
        //                 let err = err.type_annotated_value.ok_or("err unset")?;
        //                 let TypeAnnotatedValue::Str(err) = err else { Err("'file-server' result error must be a string")? };
        //                 return Err(err);
        //             }
        //         },
        //     // Otherwise use 'file-path'
        //     rec @ TypeAnnotatedValue::Record(_) => {
        //         let Some(path) = rec.get_optional(&Path::from_key("file-path")) else {
        //             // If there is no 'file-path', assume this is a standard error response
        //             return Ok(FileServerResult::Err(rec));
        //         };

        //         (path, Some(rec))
        //     }
        //     _ => Err("Response value expected")?,
        // };

        // let TypeAnnotatedValue::Str(content) = path_value else {
        //     return Err(format!("'file-server' must provide a string path, but evaluated to '{}'", path_value.to_json_value()));
        // };

        // let mime_hint = mime_guess::from_path(&content);
    }

    /// Like the above, just without the result case.
    fn from_rib_happy(result: TypeAnnotatedValue) -> Result<FileServerBindingDetails, String> {
        match value {
            TypeAnnotatedValue::Str(raw_path) => {
                Self::make_from(raw_path, None, None)
            }
            record @ TypeAnnotatedValue::Record(_) => {
                let status = match record.get_optional(&Path::from_key("status")) {
                    Some(typed_value) => get_status_code(&typed_value),
                    None => Ok(StatusCode::OK),
                }?;

                let headers = match record.get_optional(&Path::from_key("headers")) {
                    None => Ok(ResolvedResponseHeaders::default()),
                    Some(header) => ResolvedResponseHeaders::from_typed_value(&header),
                }?;
                todo!();
            }
        }
    }


    fn make_from(
        path: String,
        mime_type: Option<String>,
        status_code: Option<u16>,
    ) -> Result<FileServerBindingDetails, String> {
        let file_path = InitialComponentFilePath::from_either_str(&path)?;

        let mime_type = mime_type
            .map(|s| Mime::from_str(&s).map_err(|e| format!("Invalid mime type: {}", e)))
            .transpose()?;

        let mime_type = mime_type.ok_or(|| mime_guess::from_path(&path).first().ok_or("Could not determine mime type".to_string()))?;

        let status_code = status_code.unwrap_or(200);

        Ok(FileServerBindingDetails {
            status_code,
            mime_type,
            file_path,
        })
    }
}

#[async_trait]
pub trait FileServerBindingHandler {
    async fn handle_file_server_binding(
        &self,
        worker_detail: WorkerDetail,
        original_result: RibResult,
    ) -> FileServerBindingResult;
}

pub struct DefaultFileServerBindingHandler {
    component_service: Arc<dyn ComponentService<EmptyAuthCtx> + Sync + Send>,
    initial_component_files_service: Arc<InitialComponentFilesService>,
}

impl DefaultFileServerBindingHandler {
    pub fn new(
        component_service: Arc<dyn ComponentService<EmptyAuthCtx> + Sync + Send>,
        initial_component_files_service: Arc<InitialComponentFilesService>,
    ) -> Self {
        DefaultFileServerBindingHandler {
            component_service,
            initial_component_files_service,
        }
    }
}

#[async_trait]
impl FileServerBindingHandler for DefaultFileServerBindingHandler {
    async fn handle_file_server_binding(
        &self,
        worker_detail: WorkerDetail,
        original_result: RibResult,
    ) -> FileServerBindingResult {
        let component_metadata = self
            .component_service
            .get_by_version(&worker_detail.component_id.component_id, worker_detail.component_id.version, &EmptyAuthCtx())
            .await
            .unwrap();

        // if we are serving a read_only file, we can just go straight to the blob storage.
    }
}

fn get_file_server_result_internal(worker_response: RibResult) -> Result<FileServerResult<String>, String> {
    let RibResult::Val(value) = worker_response else {
        return Err(format!("Response value expected"));
    };

    let (path_value, response_details) = match value {
        // Allow evaluating to a single string as a shortcut...
        path @ TypeAnnotatedValue::Str(_) => (path, None),
        // ...Or a Result<String, String>
        TypeAnnotatedValue::Result(res) =>
            match res.result_value.ok_or("result not set")? {
                ResultValue::OkValue(ok) => (ok.type_annotated_value.ok_or("ok unset")?, None),
                ResultValue::ErrorValue(err) => {
                    let err = err.type_annotated_value.ok_or("err unset")?;
                    let TypeAnnotatedValue::Str(err) = err else { Err("'file-server' result error must be a string")? };
                    return Err(err);
                }
            },
        // Otherwise use 'file-path'
        rec @ TypeAnnotatedValue::Record(_) => {
            let Some(path) = rec.get_optional(&Path::from_key("file-path")) else {
                // If there is no 'file-path', assume this is a standard error response
                return Ok(FileServerResult::Err(rec));
            };

            (path, Some(rec))
        }
        _ => Err("Response value expected")?,
    };

    let TypeAnnotatedValue::Str(content) = path_value else {
        return Err(format!("'file-server' must provide a string path, but evaluated to '{}'", path_value.to_json_value()));
    };

    let mime_hint = mime_guess::from_path(&content);

    Ok(FileServerResult::Ok { content, response_details, mime_hint })
}
