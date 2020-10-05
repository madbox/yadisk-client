//
// Disk
//

use serde::{Deserialize, Serialize};
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Debug)]
pub struct YaUser {
    pub country: String,
    pub login: String,
    pub display_name: String,
    pub uid: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YaDisk {
    pub unlimited_autoupload_enabled: bool,
    pub max_file_size: u64,
    pub total_space: u64,
    pub trash_size: u64,
    pub is_paid: bool,
    pub used_space: u64,
    pub system_folders: HashMap<String, String>,
    pub user: YaUser,
    pub revision: u64
}

//
// Resource
//

#[derive(Serialize, Deserialize, Debug)]
pub struct Resource {
    #[serde(default)]
    pub antivirus_status: String, // (undefined, optional): <Статус проверки антивирусом>,
    #[serde(default)]
    pub resource_id: String, // (string, optional): <Идентификатор ресурса>,
    #[serde(default)]
    pub share: serde_json::Value, // (ShareInfo, optional): <Информация об общей папке>,
    #[serde(default)]
    pub file: String, // (string, optional): <URL для скачивания файла>,
    #[serde(default)]
    pub size: u64, // (integer, optional): <Размер файла>,
    #[serde(default)]
    pub photoslice_time: String, // (string, optional): <Дата создания фото или видео файла>,
    #[serde(default)]
    pub _embedded: ResourceList, // (ResourceList, optional): <Список вложенных ресурсов>,
    pub exif: Exif, // (Exif, optional): <Метаданные медиафайла (EXIF)>,
    #[serde(default)]
    pub custom_properties: serde_json::Value, // (object, optional): <Пользовательские атрибуты ресурса>,
    #[serde(default)]
    pub media_type: String, // (string, optional): <Определённый Диском тип файла>,
    #[serde(default)]
    pub preview: String, // (string, optional): <URL превью файла>,
    pub r#type: String, // (string): <Тип>,
    #[serde(default)]
    pub mime_type: String, // (string, optional): <MIME-тип файла>,
    #[serde(default)]
    pub revision: u64, // (integer, optional): <Ревизия Диска в которой этот ресурс был изменён последний раз>,
    #[serde(default)]
    pub public_url: String, // (string, optional): <Публичный URL>,
    pub path: String, // (string): <Путь к ресурсу>,
    #[serde(default)]
    pub md5: String, // (string, optional): <MD5-хэш>,
    #[serde(default)]
    pub public_key: String, // (string, optional): <Ключ опубликованного ресурса>,
    #[serde(default)]
    pub sha256: String, // (string, optional): <SHA256-хэш>,
    pub name: String, // (string): <Имя>,
    pub created: String, // (string): <Дата создания>,
    pub modified: String, // (string): <Дата изменения>,
    #[serde(default)]
    pub comment_ids: serde_json::Value // (CommentIds, optional): <Идентификаторы комментариев>
}

impl Default for ResourceList {
    fn default() -> Self {
        ResourceList {
            sort: String::from("_Uninitialized"),
            items: Vec::new(),
            limit: 0,
            offset: 0,
            path: String::from("_Uninitialized"),
            total: 0
        }
    }
}

/*
#[derive(Serialize, Deserialize, Debug)]
pub struct ShareInfo {
    is_root: bool, // (boolean, optional): <Признак того, что папка является корневой в группе>,
    is_owned: bool, // (boolean, optional): <Признак, что текущий пользователь является владельцем общей папки>,
    rights: String // (string): <Права доступа>
}
*/

#[derive(Serialize, Deserialize, Debug)]
pub struct ResourceList {
    #[serde(default)]
    pub sort: String, // (string, optional): <Поле, по которому отсортирован список>,
    pub items: Vec<Resource>, // (array[Resource]): <Элементы списка>,
    #[serde(default)]
    pub limit: u64, // (integer, optional): <Количество элементов на странице>,
    #[serde(default)]
    pub offset: u64, // (integer, optional): <Смещение от начала списка>,
    #[serde(default)]
    pub path: String, // (string): <Путь к ресурсу, для которого построен список>,
    #[serde(default)]
    pub total: u64, // (integer, optional): <Общее количество элементов в списке>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Exif {
    #[serde(default)]
    pub date_time: String, // (string, optional): <Дата съёмки.>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommentIds {
    #[serde(default)]
    pub private_resource: String, // (string, optional): <Идентификатор комментариев для приватных ресурсов.>,
    #[serde(default)]
    pub public_resource: String // (string, optional): <Идентификатор комментариев для публичных ресурсов.>
}

//
// DownloadInfo
//

#[derive(Serialize, Deserialize, Debug)]
pub struct DownloadInfo {
    pub href: String,
    pub method: String,
    pub templated: bool,
}

//
// UploadInfo
//

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadInfo {
    pub operation_id: String,
    pub href: String,
    pub method: String,
    pub templated: bool,
}
