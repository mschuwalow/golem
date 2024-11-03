use std::{pin::Pin, str::FromStr, sync::Arc};

use bytes::Bytes;
use futures::Stream;
use golem_common::model::{component_metadata, InitialComponentFilePath};
use golem_service_base::model::validate_worker_name;
use golem_service_base::{auth::EmptyAuthCtx, service::initial_component_files::InitialComponentFilesService};
use golem_wasm_rpc::protobuf::typed_result::ResultValue;
use http::StatusCode;
use mime_guess::Mime;
use poem::web::{headers::ContentType};
use rib::RibResult;
use async_trait::async_trait;
use crate::service::component::ComponentService;
use crate::service::worker::{self, WorkerService};
use super::WorkerDetail;
use golem_wasm_rpc::protobuf::type_annotated_value::TypeAnnotatedValue;
use crate::getter::{get_response_headers, get_response_headers_or_default, get_status_code, Getter, GetterExt};
use crate::path::Path;
use golem_wasm_rpc::json::TypeAnnotatedValueJsonExtensions;

pub struct FileServerBindingResult {
    pub original_result: RibResult,
    pub data: Pin<Box<dyn Stream<Item = Bytes>>>
}

pub struct FileServerBindingDetails {
    content_type: ContentType,
    status_code: StatusCode,
    file_path: InitialComponentFilePath,
}

impl FileServerBindingDetails {
    pub fn from_rib(result: RibResult) -> Result<FileServerBindingDetails, String> {
        // Three supported formats:
        // 1. A string path. Mime type is guessed from the path. Status code is 200.
        // 2. A record with a 'file-path' field. Mime type and status are optionally taken from the record, otherwise guessed.
        // 3. A result of either of the above, with the same rules applied.
        match result {
            RibResult::Val(value) => {
                match value {
                    TypeAnnotatedValue::Result(inner) => {
                        let value = inner.result_value.ok_or("Expected a result value".to_string())?;
                        match value {
                            ResultValue::OkValue(ok) => {
                                Self::from_rib_happy(ok.type_annotated_value.ok_or("ok unset".to_string())?)
                            }
                            ResultValue::ErrorValue(err) => {
                                let value = err.type_annotated_value.ok_or("err unset".to_string())?;
                                Err(format!("Error result: {}", value.to_json_value().to_string()))
                            }
                        }
                    },
                    other => Self::from_rib_happy(other)
                }
            }
            RibResult::Unit => {
                Err("Expected a value".to_string())
            }
        }
    }

    /// Like the above, just without the result case.
    fn from_rib_happy(value: TypeAnnotatedValue) -> Result<FileServerBindingDetails, String> {
        match value {
            TypeAnnotatedValue::Str(raw_path) => {
                Self::make_from(raw_path, None, None)
            }
            record @ TypeAnnotatedValue::Record(_) => {
                let path = record.get_optional(&Path::from_key("file-path"))
                    .ok_or("Record must contain 'file-path' field")?;
                let status = get_status_code(&record)?;
                let headers = get_response_headers_or_default(&record)?;
                let content_type  = headers.get_content_type();

                Self::make_from(path, content_type, status)
            }
            _ => Err("Response value expected".to_string()),
        }
    }

    fn make_from(
        path: String,
        content_type: Option<ContentType>,
        status_code: Option<StatusCode>,
    ) -> Result<FileServerBindingDetails, String> {
        let file_path = InitialComponentFilePath::from_either_str(&path)?;

        let content_type = match content_type {
            Some(content_type) => content_type,
            None => {
                let mime_type = mime_guess::from_path(&path).first().ok_or("Could not determine mime type")?;
                ContentType::from_str(mime_type.as_ref()).map_err(|e| format!("Invalid mime type: {}", e))?
            }
        };

        let status_code = status_code.unwrap_or(StatusCode::OK);

        Ok(FileServerBindingDetails {
            status_code,
            content_type,
            file_path,
        })
    }
}

#[async_trait]
pub trait FileServerBindingHandler {
    async fn handle_file_server_binding(
        &self,
        worker_detail: WorkerDetail,
        binding_details: FileServerBindingDetails,
        original_result: RibResult,
    ) -> FileServerBindingResult;
}

pub struct DefaultFileServerBindingHandler {
    component_service: Arc<dyn ComponentService<EmptyAuthCtx> + Sync + Send>,
    initial_component_files_service: Arc<InitialComponentFilesService>,
    worker_service: Arc<dyn WorkerService<EmptyAuthCtx> + Sync + Send>,
}

impl DefaultFileServerBindingHandler {
    pub fn new(
        component_service: Arc<dyn ComponentService<EmptyAuthCtx> + Sync + Send>,
        initial_component_files_service: Arc<InitialComponentFilesService>,
        worker_service: Arc<dyn WorkerService<EmptyAuthCtx> + Sync + Send>,
    ) -> Self {
        DefaultFileServerBindingHandler {
            component_service,
            initial_component_files_service,
            worker_service,
        }
    }
}

#[async_trait]
impl FileServerBindingHandler for DefaultFileServerBindingHandler {
    async fn handle_file_server_binding(
        &self,
        worker_detail: WorkerDetail,
        binding_details: FileServerBindingDetails,
        original_result: RibResult,
    ) -> FileServerBindingResult {
        let component_metadata = self
            .component_service
            .get_by_version(&worker_detail.component_id.component_id, worker_detail.component_id.version, &EmptyAuthCtx())
            .await
            .unwrap();

        // if we are serving a read_only file, we can just go straight to the blob storage.
        let matching_file = component_metadata
            .files
            .iter()
            .find(|file| file.path == binding_details.file_path && file.is_read_only());

        if let Some(file) = matching_file {
            let data = self
                .initial_component_files_service
                .get(&file.key)
                .await
                .unwrap()
                .ok_or("File not found")
                .unwrap();

            return FileServerBindingResult {
                original_result,
                data: Box::pin(futures::stream::once(async move { data })),
            };
        } else {
            // Read write files need to be fetched from a running worker. If no worker id is provided,
            // just create an ephemeral worker. Using the same worker for all requests ensures that we
            // are don't spawn too many workers.
            let worker_name_opt_validated = worker_detail
                .worker_name
                .map(|w| validate_worker_name(w.as_str()).map(|_| w))
                .transpose();

            let component_id = worker_request_params.component_id;

            let worker_id = TargetWorkerId {
                component_id: component_id.clone(),
                worker_name: worker_name_opt_validated.clone(),
            };

            todo!()
            // // if we are serving a read_write file, we need to get it from the worker service.
            // let data = self
            //     .worker_service
            //     .get_file_contents(&worker_detail, &binding_details.file_path)
            //     .await
            //     .unwrap();
        }
    }
}
