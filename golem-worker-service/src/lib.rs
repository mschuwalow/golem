use golem_worker_service_base::service::worker::WorkerRequestMetadata;

pub mod api;
pub mod config;
pub mod grpcapi;
pub mod service;

#[cfg(test)]
test_r::enable!();
