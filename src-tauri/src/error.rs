use serde::{Serialize, Serializer};

/// The one error type every command returns. The UI receives `{ kind, message }`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    /// A local AI service (Ollama, ComfyUI) is unreachable or returned an error.
    #[error("{service}: {message}")]
    Service { service: &'static str, message: String },
    #[error("Đã huỷ")]
    Cancelled,
    #[error("Lỗi đọc/ghi file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Dữ liệu JSON không hợp lệ: {0}")]
    Json(#[from] serde_json::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::Invalid(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn service(service: &'static str, msg: impl Into<String>) -> Self {
        Self::Service { service, message: msg.into() }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "notFound",
            Self::Invalid(_) => "invalid",
            Self::Service { .. } => "service",
            Self::Cancelled => "cancelled",
            Self::Io(_) => "io",
            Self::Json(_) => "json",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}
