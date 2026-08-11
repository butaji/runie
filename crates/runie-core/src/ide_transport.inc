impl IdeActor {
    /// Own one bounded async reader until EOF, reducing each complete JSON-RPC
    /// notification through the actor and publishing disconnect as an event.
    pub async fn ingest_reader<R>(&self, reader: R) -> Result<usize, String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(reader);
        let mut buffer = IdeWireBuffer::default();
        let mut line = Vec::new();
        let mut frames = 0;
        loop {
            line.clear();
            let read = reader
                .read_until(b'\n', &mut line)
                .await
                .map_err(|error| format!("IDE reader failed: {error}"))?;
            if read == 0 {
                break;
            }
            if line.len() > IDE_MAX_FRAME_BYTES {
                return Err("IDE JSON-RPC frame exceeds the bounded byte limit".into());
            }
            let text = std::str::from_utf8(&line)
                .map_err(|error| format!("IDE frame is not UTF-8: {error}"))?;
            frames += self.apply_wire_frames(&mut buffer, text).await?;
        }
        self.apply(IdeEvent::ConnectionLost).await?;
        Ok(frames)
    }
}

enum IdeTransportCommand {
    Send {
        notification: IdeRpcNotification,
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    Close {
        reply: tokio::sync::oneshot::Sender<()>,
    },
}

#[derive(Clone)]
pub struct IdeTransport {
    tx: tokio::sync::mpsc::Sender<IdeTransportCommand>,
    _owner: std::sync::Arc<crate::task_owner::TaskOwner>,
}

impl IdeTransport {
    pub async fn connect(endpoint: IdeEndpoint, actor: IdeActor) -> Result<Self, String> {
        match endpoint {
            IdeEndpoint::Tcp { host, port } => tokio::net::TcpStream::connect((host, port))
                .await
                .map(|stream| Self::from_stream(stream, actor))
                .map_err(|error| format!("IDE TCP connect failed: {error}")),
            IdeEndpoint::Unix { path } => connect_unix(path, actor).await,
        }
    }

    pub async fn connect_with_retry<F, Fut>(
        endpoint: IdeEndpoint,
        actor: IdeActor,
        policy: IdeReconnectPolicy,
        mut connector: F,
    ) -> Result<Self, String>
    where
        F: FnMut(IdeEndpoint, IdeActor) -> Fut,
        Fut: std::future::Future<Output = Result<Self, String>>,
    {
        let mut state = IdeReconnectState::default();
        loop {
            match connector(endpoint.clone(), actor.clone()).await {
                Ok(transport) => return Ok(transport),
                Err(error) => match state.disconnected(policy) {
                    (next, IdeReconnectDecision::RetryAfter { delay_ms }) => {
                        state = next;
                        if delay_ms > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        }
                    }
                    (_, IdeReconnectDecision::Exhausted) => {
                        return Err(format!("IDE reconnect exhausted: {error}"));
                    }
                },
            }
        }
    }

    pub fn from_stream<S>(stream: S, actor: IdeActor) -> Self
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static,
    {
        let (tx, mut commands) = tokio::sync::mpsc::channel(16);
        let owner = crate::spawn_owned_worker!(async move {
            use tokio::io::AsyncBufReadExt;
            let (read_half, mut write_half) = tokio::io::split(stream);
            let mut reader = tokio::io::BufReader::new(read_half);
            let mut buffer = IdeWireBuffer::default();
            let mut line = Vec::new();
            loop {
                tokio::select! {
                    incoming = reader.read_until(b'\n', &mut line) => {
                        let Ok(bytes) = incoming else { break; };
                        if bytes == 0 { break; }
                        if line.len() > IDE_MAX_FRAME_BYTES { break; }
                        let Ok(text) = std::str::from_utf8(&line) else { break; };
                        if actor.apply_wire_frames(&mut buffer, text).await.is_err() { break; }
                        line.clear();
                    }
                    command = commands.recv() => match command {
                        Some(IdeTransportCommand::Send { notification, reply }) => {
                            let result = match serde_json::to_string(&notification) {
                                Ok(value) => async_write(&mut write_half, format!("{value}\n").as_bytes()).await,
                                Err(error) => Err(format!("encode IDE notification: {error}")),
                            };
                            let _ = reply.send(result);
                        }
                        Some(IdeTransportCommand::Close { reply }) => {
                            let _ = actor.apply(IdeEvent::ConnectionLost).await;
                            let _ = reply.send(());
                            break;
                        }
                        None => break,
                    }
                }
                line.clear();
            }
            let _ = actor.apply(IdeEvent::ConnectionLost).await;
        });
        Self { tx, _owner: owner }
    }

    pub async fn send(&self, notification: IdeRpcNotification) -> Result<(), String> {
        let (reply, result) = tokio::sync::oneshot::channel();
        self.tx.send(IdeTransportCommand::Send { notification, reply }).await
            .map_err(|_| "IDE transport is closed".to_owned())?;
        result.await.map_err(|_| "IDE write acknowledgement was dropped".to_owned())?
    }

    pub async fn close(&self) {
        let (reply, result) = tokio::sync::oneshot::channel();
        if self.tx.send(IdeTransportCommand::Close { reply }).await.is_ok() { let _ = result.await; }
    }
}

#[cfg(unix)]
async fn connect_unix(path: String, actor: IdeActor) -> Result<IdeTransport, String> {
    tokio::net::UnixStream::connect(path)
        .await
        .map(|stream| IdeTransport::from_stream(stream, actor))
        .map_err(|error| format!("IDE Unix connect failed: {error}"))
}

#[cfg(not(unix))]
async fn connect_unix(_path: String, _actor: IdeActor) -> Result<IdeTransport, String> {
    Err("IDE Unix endpoints are unsupported on this platform".into())
}

async fn async_write<W: tokio::io::AsyncWrite + Unpin>(writer: &mut W, bytes: &[u8]) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    writer.write_all(bytes).await.map_err(|error| format!("IDE writer failed: {error}"))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IdeEndpoint {
    Tcp { host: String, port: u16 },
    Unix { path: String },
}

impl IdeEndpoint {
    pub fn parse(value: &str) -> Result<Self, String> {
        if let Some(address) = value.strip_prefix("tcp://") {
            let (host, port) = address.rsplit_once(':').ok_or("IDE TCP endpoint needs host:port")?;
            let port = port.parse().map_err(|_| "IDE TCP endpoint has an invalid port")?;
            if host.trim().is_empty() { return Err("IDE TCP endpoint needs a host".into()); }
            return Ok(Self::Tcp { host: host.into(), port });
        }
        if let Some(path) = value.strip_prefix("unix://").filter(|path| !path.trim().is_empty()) {
            return Ok(Self::Unix { path: path.into() });
        }
        Err("IDE endpoint must use tcp:// or unix://".into())
    }

    pub fn discover<I, S>(candidates: I) -> Option<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        candidates
            .into_iter()
            .find_map(|candidate| Self::parse(candidate.as_ref()).ok())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeReconnectPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl IdeReconnectPolicy {
    pub const fn bounded() -> Self { Self { max_attempts: 3, base_delay_ms: 250, max_delay_ms: 4_000 } }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeReconnectState { pub attempts: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IdeReconnectDecision { RetryAfter { delay_ms: u64 }, Exhausted }

impl IdeReconnectState {
    pub const fn new() -> Self { Self { attempts: 0 } }
    pub fn disconnected(mut self, policy: IdeReconnectPolicy) -> (Self, IdeReconnectDecision) {
        if self.attempts >= policy.max_attempts { return (self, IdeReconnectDecision::Exhausted); }
        self.attempts += 1;
        let exponent = self.attempts.saturating_sub(1).min(31);
        let delay_ms = policy.base_delay_ms.saturating_mul(1u64 << exponent).min(policy.max_delay_ms);
        (self, IdeReconnectDecision::RetryAfter { delay_ms })
    }
    pub const fn reconnected() -> Self { Self::new() }
}
