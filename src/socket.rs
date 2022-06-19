use anyhow::{Result, Ok};
use pastedev::SnippetManager;
use tracing::info;
use std::net::SocketAddr;
use tokio::{net::{TcpListener, TcpStream}, io::{BufReader, AsyncReadExt, AsyncWriteExt}};

pub async fn run_socket(addr: SocketAddr, snippet_manager: SnippetManager) -> Result<()> {
    info!("Listening socket on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    
    loop {
        let (stream, addr) = listener.accept().await?;

        let snippet_manager = snippet_manager.clone();

        tokio::spawn(async move {
            info!("New socket connection from {}", addr);
            process_socket(stream, snippet_manager.clone()).await.unwrap();
        });
    }

    // Ok(())
}

pub async fn process_socket(mut stream: TcpStream, snippet_manager: SnippetManager) -> Result<()> {
    let mut buf_reader = BufReader::new(&mut stream);

    let mut text = String::new();
    buf_reader.read_to_string(&mut text).await?;
    
    if !text.trim().is_empty() {
        let snippet_id = snippet_manager.create_snippet(&text).await?;

        stream.write_all(snippet_id.as_bytes()).await?;
        stream.write_all("\n".as_bytes()).await?;
    } else {
        stream.write_all("Nothing to paste\n".as_bytes()).await?;
    }


    Ok(())
}
