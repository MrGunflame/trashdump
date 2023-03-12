use axum::routing::{get, post};
use axum::Router;
use state::State;

mod state;
mod v1;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let state = State::new();

    let app = Router::new()
        .route(
            "/v1/files",
            post({
                let state = state.clone();
                move |body| v1::create_file(body, state)
            }),
        )
        .route(
            "/v1/files/:id",
            get({
                let state = state.clone();
                move |path| v1::get_file(path, state)
            }),
        );

    if let Err(err) = axum::Server::bind(&([0, 0, 0, 0], 3030).into())
        .serve(app.into_make_service())
        .await
    {
        tracing::error!("Failed to run server: {}", err);
    }
}
