use braint_proto::{JsonRpcRequest, JsonRpcResponse};
use interprocess::local_socket::tokio::prelude::*;
use serde::{Serialize, de::DeserializeOwned};

pub struct Client {
    stream: LocalSocketStream,
}

impl Client {
    pub async fn connect(path: &str) -> crate::error::Result<Self> {
        use interprocess::local_socket::GenericFilePath;
        let name = path
            .to_fs_name::<GenericFilePath>()
            .map_err(|e| crate::error::ClientError::DaemonUnreachable(e.to_string()))?;
        let stream = LocalSocketStream::connect(name)
            .await
            .map_err(|e| crate::error::ClientError::DaemonUnreachable(e.to_string()))?;
        Ok(Self { stream })
    }

    pub async fn send<Req, Resp>(
        &mut self,
        request: &JsonRpcRequest<Req>,
    ) -> crate::error::Result<JsonRpcResponse<Resp>>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        let payload = serde_json::to_vec(request)?;
        crate::framing::write_frame(&mut self.stream, &payload).await?;

        let response_bytes = crate::framing::read_frame(&mut self.stream).await?;
        let response: JsonRpcResponse<Resp> = serde_json::from_slice(&response_bytes)?;
        Ok(response)
    }
}
