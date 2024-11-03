use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::worker_binding::CompiledGolemWorkerBinding;
use golem_service_base::model::VersionedComponentId;
use rib::Expr;
use poem_openapi::Enum;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct GolemWorkerBinding {
    pub component_id: VersionedComponentId,
    pub worker_name: Option<Expr>,
    pub idempotency_key: Option<Expr>,
    pub response: ResponseMapping,
    pub worker_binding_type: WorkerBindingType,
}

// ResponseMapping will consist of actual logic such as invoking worker functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct ResponseMapping(pub Expr);

impl From<CompiledGolemWorkerBinding> for GolemWorkerBinding {
    fn from(value: CompiledGolemWorkerBinding) -> Self {
        let worker_binding = value.clone();

        GolemWorkerBinding {
            component_id: worker_binding.component_id,
            worker_name: worker_binding
                .worker_name_compiled
                .map(|compiled| compiled.worker_name),
            idempotency_key: worker_binding
                .idempotency_key_compiled
                .map(|compiled| compiled.idempotency_key),
            response: ResponseMapping(worker_binding.response_compiled.response_rib_expr),
            worker_binding_type: worker_binding.worker_binding_type,
        }
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Enum)]
#[serde(rename_all = "kebab-case")]
#[oai(rename_all = "kebab-case")]
pub enum WorkerBindingType {
    Default,
    FileServer
}

impl Default for WorkerBindingType {
    fn default() -> Self {
        WorkerBindingType::Default
    }
}

impl TryFrom<String> for WorkerBindingType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "default" => Ok(WorkerBindingType::Default),
            "file-server" => Ok(WorkerBindingType::FileServer),
            _ => Err(format!("Invalid WorkerBindingType: {}", value))
        }
    }
}

impl From<golem_api_grpc::proto::golem::apidefinition::WorkerBindingType> for WorkerBindingType {
    fn from(value: golem_api_grpc::proto::golem::apidefinition::WorkerBindingType) -> Self {
        match value {
            golem_api_grpc::proto::golem::apidefinition::WorkerBindingType::Default => WorkerBindingType::Default,
            golem_api_grpc::proto::golem::apidefinition::WorkerBindingType::FileServer => WorkerBindingType::FileServer
        }
    }
}

impl From<WorkerBindingType> for golem_api_grpc::proto::golem::apidefinition::WorkerBindingType {
    fn from(value: WorkerBindingType) -> Self {
        match value {
            WorkerBindingType::Default => golem_api_grpc::proto::golem::apidefinition::WorkerBindingType::Default,
            WorkerBindingType::FileServer => golem_api_grpc::proto::golem::apidefinition::WorkerBindingType::FileServer
        }
    }
}
