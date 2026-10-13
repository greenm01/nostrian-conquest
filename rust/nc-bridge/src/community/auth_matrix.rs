//! Matrix sign-in: `nc-matrix` sends a code and link to the verified sender of a
//! request, and this route redeems either one.
use super::*;
use nc_data::matrix_login::{self, Redeem};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginBody {
    code: String,
}

pub(super) async fn login(
    State(state): State<Arc<Community>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if let Err(response) = check_origin(&state, &headers) {
        return response;
    }
    let request = match serde_json::from_slice::<LoginBody>(&body) {
        Ok(request) if request.code.len() <= 128 => request,
        _ => return error(StatusCode::BAD_REQUEST, "Invalid login request"),
    };
    let result = auth::connection(&state).and_then(|mut conn| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let outcome = matrix_login::redeem(&tx, &request.code, chrono::Utc::now().timestamp())
            .map_err(|e| e.to_string())?;
        let token = match &outcome {
            Redeem::SignedIn {
                account,
                display_name,
            } => Some(auth::save_session(&tx, account, Some(display_name))?),
            Redeem::Invalid | Redeem::Throttled => None,
        };
        // Commit failures too, so they count toward the guessing limit.
        tx.commit().map_err(|e| e.to_string())?;
        Ok((outcome, token))
    });
    match result {
        Ok((_, Some(token))) => {
            let mut response = Json(json!({"ok":true})).into_response();
            auth::session_cookie(&state, &mut response, &token);
            response
        }
        Ok((Redeem::Throttled, _)) => error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many incorrect sign-in codes; try again in a few minutes",
        ),
        Ok(_) => error(
            StatusCode::UNAUTHORIZED,
            "Sign-in code is incorrect, used or expired; ask the bot for a new one",
        ),
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Session storage unavailable",
        ),
    }
}
