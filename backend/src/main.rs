#[tokio::main]
async fn main() {
    let db = std::env::var("DATABASE").unwrap_or_else(|_| "chess-coach.db".into());
    // Leave a couple of cores for the OS and the browser.
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(2).max(1)) as u32;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await.unwrap();
    println!("API on http://127.0.0.1:3001");
    axum::serve(listener, backend::app(&db, backend::EngineConfig { command: "stockfish".into(), depth: 18, threads, hash_mb: 1024 })).await.unwrap();
}
