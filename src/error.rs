#[derive(Debug, thiserror::Error)]
pub enum ReporterError {
    #[error("IO ошибка: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON ошибка: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Дата не найдена")]
    DateNotFound,

    #[error("Строка не найдена")]
    RowNotFound,
}
