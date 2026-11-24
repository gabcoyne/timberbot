use futures_util::{SinkExt, StreamExt};
use timberbot_protocol::ws::{ClientFrame, HeartbeatPayload, ServerFrame, WsEnvelope};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{HeaderValue, AUTHORIZATION};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use crate::error::Result;

pub struct TimberbotWsConnection {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl TimberbotWsConnection {
    pub async fn connect(ws_url: &str, auth_token: Option<&str>) -> Result<Self> {
        let mut req = ws_url.into_client_request()?;
        if let Some(token) = auth_token {
            let trimmed = token.trim();
            if !trimmed.is_empty() {
                if let Ok(val) = HeaderValue::from_str(&format!("Bearer {trimmed}")) {
                    req.headers_mut().insert(AUTHORIZATION, val);
                }
            }
        }
        let (stream, _resp) = connect_async(req).await?;
        Ok(Self { stream })
    }

    pub async fn send_frame(&mut self, frame: &ClientFrame) -> Result<()> {
        let env = frame.to_envelope()?;
        let text = serde_json::to_string(&env)?;
        self.stream.send(Message::Text(text)).await?;
        Ok(())
    }

    pub async fn send_ping(&mut self) -> Result<()> {
        self.send_frame(&ClientFrame::Ping).await
    }

    pub async fn send_heartbeat(&mut self, payload: HeartbeatPayload) -> Result<()> {
        self.send_frame(&ClientFrame::Heartbeat(payload)).await
    }

    pub async fn next_frame(&mut self) -> Result<Option<ServerFrame>> {
        while let Some(msg) = self.stream.next().await {
            let msg = msg?;
            match msg {
                Message::Text(txt) => {
                    let env: WsEnvelope = serde_json::from_str(&txt)?;
                    return Ok(Some(ServerFrame::from_envelope(env)?));
                }
                Message::Ping(bytes) => {
                    self.stream.send(Message::Pong(bytes)).await?;
                }
                Message::Pong(_) => {}
                Message::Close(_) => return Ok(None),
                Message::Binary(_) | Message::Frame(_) => {}
            }
        }
        Ok(None)
    }

    pub async fn close(&mut self) -> Result<()> {
        self.stream.close(None).await?;
        Ok(())
    }
}
