//! Files and folders: messages 400–453 on the main connection.

use openkdx_proto::Message;
use openkdx_proto::msg::common::FilePath;
use openkdx_proto::msg::files::{
    CreateFolder, EmptyTrash, FileInfo, FileList, FileListEntry, FileSearch, FileSearchEntry,
    FileSearchReply, GetFileFolderInfo, GetFileList, MoveToTrash,
};

use crate::Result;
use crate::session::Session;

impl Session {
    /// 400 → 401; an empty folder is an empty payload.
    pub async fn list_files(&self, path: &FilePath) -> Result<Vec<FileListEntry>> {
        let list: FileList = self
            .connection()
            .request_as(&GetFileList { path: path.clone() })
            .await?;
        Ok(list.entries)
    }

    /// 409 → 410.
    pub async fn file_info(&self, path: &FilePath) -> Result<FileInfo> {
        self.connection()
            .request_as(&GetFileFolderInfo { path: path.clone() })
            .await
    }

    /// 402 → 1 Status 0x192 on success. `access` is a folder access tag
    /// such as `DB`, `owner` the drop box owner's login; both may be empty.
    pub async fn create_folder(
        &self,
        parent: &FilePath,
        name: &[u8],
        access: &[u8],
        owner: &[u8],
    ) -> Result<Message> {
        self.connection()
            .request_ok(&CreateFolder {
                ignored: [0; 19],
                name: name.to_vec(),
                access: access.to_vec(),
                owner: owner.to_vec(),
                path: parent.clone(),
            })
            .await
    }

    /// 405 → 1 Status.
    pub async fn move_to_trash(&self, folder: &FilePath, names: &[&[u8]]) -> Result<Message> {
        let names = FilePath {
            names: names
                .iter()
                .map(|n| openkdx_proto::msg::common::PathName::new(n))
                .collect(),
        };
        self.connection()
            .request_ok(&MoveToTrash {
                folder: folder.clone(),
                names,
            })
            .await
    }

    /// 421 → 1 Status.
    pub async fn empty_trash(&self) -> Result<Message> {
        self.connection().request_ok(&EmptyTrash).await
    }

    /// 452 → 453.
    pub async fn search(&self, query: &[u8]) -> Result<Vec<FileSearchEntry>> {
        let reply: FileSearchReply = self
            .connection()
            .request_as(&FileSearch {
                ignored_00: [0; 84],
                ignored_55: [0; 3],
                query: query.to_vec(),
            })
            .await?;
        Ok(reply.entries)
    }
}
