use crate::web::WebAppError;

pub async fn index() -> Result<&'static str, WebAppError> {
    Ok("Hello")
}
