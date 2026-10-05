#[tokio::main]
async fn main() {
    let db = std::env::var("DATABASE").unwrap_or_else(|_| "chess-coach.db".into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await.unwrap();
    println!("API on http://127.0.0.1:3001");
    axum::serve(listener, backend::app(&db, backend::EngineConfig { command: "stockfish".into(), depth: 18 })).await.unwrap();
}
