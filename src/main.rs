use axum::extract::Path;
use axum::routing::{get, post, MethodRouter};
use axum::Router;
use state::State;

mod state;
mod v1;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let state = State::new();

    tokio::fs::remove_dir_all("./app/partial").await?;

    tokio::fs::create_dir_all("./app/dumps").await?;
    tokio::fs::create_dir_all("./app/partial").await?;

    let app = Router::new()
        .route(
            "/v1/files/:id/:name",
            MethodRouter::new().get({
                let state = state.clone();
                move |path| v1::get_file(path, state)
            }),
        )
        .route(
            "/v1/new/:name",
            MethodRouter::new().post({
                let state = state.clone();
                move |path, body| v1::create_file(path, body, state)
            }),
        );

    if let Err(err) = axum::Server::bind(&([0, 0, 0, 0], 3030).into())
        .serve(app.into_make_service())
        .await
    {
        tracing::error!("Failed to run server: {}", err);
    }

    Ok(())
}
