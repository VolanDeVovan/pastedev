use anyhow::{Ok, Result};
use crate::snippet::Snippet;
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    select,
    time::sleep,
};
use tracing::info;
use url::Url;

pub struct SocketServer {
    addr: SocketAddr,
    app_url: Url,
    db: PgPool,
}

impl SocketServer {
    pub fn new(
        addr: SocketAddr,
        app_url: Url,
        db: PgPool,
    ) -> SocketServer {
        SocketServer {
            addr,
            app_url,
            db,
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

        // Ok(())
    }
}

async fn process_socket(socket_server: Arc<SocketServer>, mut stream: TcpStream) -> Result<()> {
    let mut buf_reader = BufReader::new(&mut stream);

    let mut buff: Vec<u8> = Vec::new();

    loop {
        select! {
            res = buf_reader.read_until(0xA, &mut buff) => {
                if res? == 0 { break }
            }

            () = sleep(Duration::from_secs(3)) => break
        }
    }

    let text = String::from_utf8(buff)?;

    if !text.trim().is_empty() {
        let snippet = Snippet::create_snippet(&socket_server.db, text.trim().to_string(), true).await?;

        let snippet_url = socket_server.app_url.join(&snippet.alias)?;

        stream.write_all(snippet_url.to_string().as_bytes()).await?;
        stream.write_all(b"\n").await?;
    } else {
        stream.write_all("Nothing to paste\n".as_bytes()).await?;
    }

    Ok(())
}
