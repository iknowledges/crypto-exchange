use std::{net::SocketAddr, sync::Arc};
use axum::{Extension, routing::get};

use crate::{controller, feed::{self, session_manager::SessionManager, ws_handler}, matching::command_producer::MatchingEngineCommandProducer};

#[derive(Clone)]
pub struct AppContext {
    pub db: mongodb::Database,
    pub producer: Arc<MatchingEngineCommandProducer>,
}

impl AppContext {
    pub fn new(db: mongodb::Database, producer: MatchingEngineCommandProducer) -> Self {
        Self {
            db,
            producer: Arc::new(producer)
        }
    }
}

pub async fn start(state: AppContext, port: u16) -> anyhow::Result<()> {
    let router = controller::create_router();

    let session_manager = Arc::new(SessionManager::new());

    feed::listener::start(session_manager.clone()).await?;

    let app = axum::Router::new()
        .route("/ws", get(ws_handler))
        .layer(Extension(session_manager))
        .merge(router)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Server port: {}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service())
        .await?;
    Ok(())
}