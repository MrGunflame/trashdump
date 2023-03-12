use std::io;

use axum::body::{Bytes, Full, StreamBody};
use axum::extract::{BodyStream, Path};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use futures::{Stream, StreamExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::state::State;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    size: u64,
    id: String,
    name: String,
}

pub async fn get_file(
    Path((path, name)): Path<(String, String)>,
    state: State,
) -> Response<Full<Bytes>> {
    let Ok(mut dump) = state.dumps.get(&path,&name).await else {
        return Response::builder().status(StatusCode::NOT_FOUND).body(Full::new(Bytes::new())).unwrap();
    };

    let mut buf = Vec::new();
    if let Err(err) = dump.read_to_end(&mut buf).await {
        tracing::error!("Failed to read file: {}", err);
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::new()))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .body(Full::new(buf.into()))
        .unwrap()
}

pub async fn create_file(
    Path(name): Path<String>,
    mut body: BodyStream,
    state: State,
) -> Response<Full<Bytes>> {
    let mut dump = state.dumps.insert(&name).await.unwrap();
    let mut size: u64 = 0;

    while let Some(chunk) = body.next().await {
        let chunk = match chunk {
            Err(err) => {
                tracing::debug!("read chunk error: {}", err);

                if let Err(err) = dump.abort().await {
                    tracing::error!("Failed to abort upload: {}", err);
                }

                return Response::builder().body(Full::from("abort")).unwrap();
            }
            Ok(c) => c,
        };

        size += chunk.len() as u64;
        if let Err(err) = dump.write(&chunk).await {
            tracing::error!("Failed to write to file: {}", err);
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::new()))
                .unwrap();
        }

        if size >= state.max_size {}
    }

    let hash = match dump.finish().await {
        Err(err) => {
            tracing::error!("Failed to finish upload: {}", err);
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::new()))
                .unwrap();
        }
        Ok(e) => e,
    };

    let buf = serde_json::to_vec(&File {
        id: hash,
        size,
        name,
    })
    .unwrap();

    Response::builder()
        .header("Content-Type", "application/json")
        .body(Full::from(buf))
        .unwrap()
}
