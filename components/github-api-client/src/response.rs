use crate::error::{HubError, HubResult};
use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;

async fn read_status_and_body(response: Response) -> HubResult<(StatusCode, String)> {
    let status = response.status();
    let body = response.text().await?;
    debug!("GitHub response body, {}", body);
    Ok((status, body))
}

pub async fn read_json<T>(response: Response) -> HubResult<T>
where
    T: DeserializeOwned,
{
    let (status, body) = read_status_and_body(response).await?;
    if status != StatusCode::OK {
        return Err(HubError::api_response(status, &body));
    }

    serde_json::from_str(&body).map_err(HubError::from)
}

pub async fn read_optional_json<T>(response: Response) -> HubResult<Option<T>>
where
    T: DeserializeOwned,
{
    let (status, body) = read_status_and_body(response).await?;
    match status {
        StatusCode::OK => serde_json::from_str(&body)
            .map(Some)
            .map_err(HubError::from),
        StatusCode::NOT_FOUND => Ok(None),
        status => Err(HubError::api_response(status, &body)),
    }
}

pub async fn expect_ok(response: Response) -> HubResult<()> {
    let (status, body) = read_status_and_body(response).await?;
    if status == StatusCode::OK {
        Ok(())
    } else {
        Err(HubError::api_response(status, &body))
    }
}
