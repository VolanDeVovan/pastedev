use anyhow::{Ok, Result};
use pastedev::SnippetManager;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};
use tracing::info;
use url::Url;

pub struct SocketServer {
    addr: SocketAddr,
    app_url: Url,
    snippet_manager: SnippetManager,
}

impl SocketServer {
    pub fn new(addr: SocketAddr, app_url: Url, snippet_manager: SnippetManager) -> SocketServer {
        SocketServer {
            addr,
            app_url,
            snippet_manager,
        }
    }

    pub async fn run_socket(self) -> Result<()> {
        info!("Listening socket on {}", self.addr);

        let listener = TcpListener::bind(self.addr).await?;

        let socket_server = Arc::new(self);

        loop {
            let (stream, addr) = listener.accept().await?;

            let socket_server = Arc::clone(&socket_server);

            tokio::spawn(async move {
                info!("New socket connection from {}", addr);
                process_socket(socket_server, stream).await.unwrap();
            });
        }

        Ok(())
    }

}

async fn process_socket(socket_server: Arc<SocketServer>, mut stream: TcpStream) -> Result<()> {
    let mut buf_reader = BufReader::new(&mut stream);

    let mut text = String::new();
    buf_reader.read_to_string(&mut text).await?;

    if !text.trim().is_empty() {
        let snippet_id = socket_server.snippet_manager.create_snippet(&text).await?;

        let snippet_url = socket_server.app_url.join(&snippet_id)?;

        stream.write_all(snippet_url.to_string().as_bytes()).await?;
        stream.write_all(b"\n").await?;
    } else {
        stream.write_all("Nothing to paste\n".as_bytes()).await?;
    }

    Ok(())
}