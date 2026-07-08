use axum::{
    Json,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
use jsonwebtoken::{DecodingKey, Validation, decode};

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub status: &'static str,
    pub message: String,
}

pub async fn auth<B>(
    cookie_jar: CookieJar,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let token = cookie_jar
        .get("token")
        .map(|cookie| cookie.value().to_string())
        .or_else(|| {
            req.headers()
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|auth_header| auth_header.to_str().ok())
                .and_then(|auth_value| auth_value.strip_prefix("Bearer ").map(String::from))
        });

    let token = token.ok_or_else(|| {
        let json_error = ErrorResponse {
            status: "fail",
            message: "You are not logged in, please provide token".to_string(),
        };
        (StatusCode::UNAUTHORIZED, Json(json_error))
    })?;

    let secret_key = icarus_envy::environment::get_secret_main_key().value;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_audience(&["icarus"]); // Must match exactly what's in the token
    let _claims = decode::<icarus_models::token::UserClaims>(
        &token,
        &DecodingKey::from_secret(secret_key.as_ref()),
        &validation,
    )
    .map_err(|err| {
        eprintln!("Error: {err:?}");
        let json_error = ErrorResponse {
            status: "fail",
            message: "Invalid token - Error decoding claims".to_string(),
        };
        (StatusCode::UNAUTHORIZED, Json(json_error))
    })?
    .claims;

    Ok(next.run(req).await)
}
