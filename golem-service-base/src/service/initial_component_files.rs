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

use std::{path::PathBuf, sync::Arc};

use bytes::Bytes;
use sha2::{Digest, Sha256};
use tracing::debug;

use crate::storage::blob::{BlobStorage, BlobStorageNamespace};
use golem_common::model::{AccountId, InitialComponentFileKey};

const INITIAL_COMPONENT_FILES_LABEL: &str = "initial_component_files";

/// Service for storing initial component files.
pub struct InitialComponentFilesService {
    blob_storage: Arc<dyn BlobStorage + Send + Sync>,
}

impl InitialComponentFilesService {
    pub fn new(blob_storage: Arc<dyn BlobStorage + Send + Sync>) -> Self {
        Self { blob_storage }
    }

    pub async fn exists(
        &self,
        account_id: &AccountId,
        key: &InitialComponentFileKey,
    ) -> Result<bool, String> {
        let path = PathBuf::from(key.0.clone());

        let metadata = self
            .blob_storage
            .get_metadata(
                INITIAL_COMPONENT_FILES_LABEL,
                "exists",
                BlobStorageNamespace::InitialComponentFiles {
                    account_id: account_id.clone(),
                },
                &path,
            )
            .await
            .map_err(|err| format!("Failed to get metadata: {}", err))?;

        Ok(metadata.is_some())
    }

    pub async fn get(
        &self,
        account_id: &AccountId,
        key: &InitialComponentFileKey,
    ) -> Result<Option<Bytes>, String> {
        self.blob_storage
            .get_raw(
                INITIAL_COMPONENT_FILES_LABEL,
                "get",
                BlobStorageNamespace::InitialComponentFiles {
                    account_id: account_id.clone(),
                },
                &PathBuf::from(key.0.clone()),
            )
            .await
    }

    pub async fn put_if_not_exists(
        &self,
        account_id: &AccountId,
        bytes: &Bytes,
    ) -> Result<InitialComponentFileKey, String> {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = hex::encode(hasher.finalize());

        let key = PathBuf::from(hash.clone());

        let metadata = self
            .blob_storage
            .get_metadata(
                INITIAL_COMPONENT_FILES_LABEL,
                "put",
                BlobStorageNamespace::InitialComponentFiles {
                    account_id: account_id.clone(),
                },
                &key,
            )
            .await
            .map_err(|err| format!("Failed to get metadata: {}", err))?;

        if metadata.is_none() {
            debug!("Storing initial component file with hash: {}", hash);

            self.blob_storage
                .put_raw(
                    INITIAL_COMPONENT_FILES_LABEL,
                    "put",
                    BlobStorageNamespace::InitialComponentFiles {
                        account_id: account_id.clone(),
                    },
                    &key,
                    bytes,
                )
                .await?;
        };
        Ok(InitialComponentFileKey(hash))
    }
}
