#![feature(slice_as_array)]

mod js_anyhow;
mod webrtc;

use anyhow::anyhow;
use iroh::{
    NodeAddr, Watcher,
    endpoint::{Incoming, ReadExactError, RecvStream, SendStream},
};
use js_sys::JSON;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys::Function, spawn_local};
use web_sys::{
    RtcDataChannel, RtcIceCandidate, RtcIceCandidateInit, RtcPeerConnection, RtcSessionDescription,
    RtcSessionDescriptionInit,
};

use crate::{
    js_anyhow::{AnyhowError, JsAnyhow},
    webrtc::WebRtcConnection,
};

const ALPN: &[u8] = b"iroh+webrtc";

#[wasm_bindgen]
pub struct Endpoint(iroh::Endpoint);

#[wasm_bindgen(getter_with_clone)]
pub struct Connection {
    pub peer_connection: RtcPeerConnection,
    pub data_channel: RtcDataChannel,
}

#[wasm_bindgen]
impl Endpoint {
    pub async fn new() -> Result<Endpoint, AnyhowError> {
        let endpoint = iroh::Endpoint::builder()
            .discovery_n0()
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await?;
        Ok(Endpoint(endpoint))
    }

    pub fn node_id(&self) -> String {
        self.0.node_id().to_string()
    }

    pub async fn initialized(&self) {
        self.0.node_addr().initialized().await;
    }

    async fn handle(incoming: Incoming, on_connect: Function) -> Result<(), AnyhowError> {
        let conn = incoming.accept()?.await?;
        let (send, recv) = conn.accept_bi().await?;
        let mut channel = Channel { send, recv };

        let rtc = WebRtcConnection::new()?;
        let mut ice_candidate_stream = rtc.get_ice_candidates()?;
        let data_channel = rtc.accept_data_channel();

        let offer = channel.receive_session_description().await?;
        rtc.set_remote_description(offer).await?;

        let answer = rtc.create_answer().await?;
        channel.send_session_description(&answer).await?;

        while let Some(ice_candidate) = channel.receive_ice_candidate().await? {
            rtc.add_ice_candidate(Some(&ice_candidate)).await?;
        }
        rtc.add_ice_candidate(None).await?;

        while let Some(ice_candidate) = ice_candidate_stream.recv().await {
            channel.send_ice_candidate(&ice_candidate).await?;
        }
        channel.stop_writing().await?;

        let data_channel = data_channel.await?;

        on_connect
            .call1(
                &on_connect,
                &Connection {
                    peer_connection: rtc.into_inner(),
                    data_channel,
                }
                .into(),
            )
            .js_anyhow()?;
        Ok(())
    }

    fn spawn_handle(incoming: Incoming, on_connect: Function) {
        spawn_local(async move {
            let _ = Self::handle(incoming, on_connect).await;
        });
    }

    pub async fn listen(&self, on_connect: Function) {
        while let Some(incoming) = self.0.accept().await {
            Self::spawn_handle(incoming, on_connect.clone());
        }
    }

    pub async fn connect(&self, node_id: String) -> Result<Connection, AnyhowError> {
        let conn = self
            .0
            .connect(NodeAddr::new(node_id.parse()?), ALPN)
            .await?;
        let (send, recv) = conn.open_bi().await?;
        let mut channel = Channel { send, recv };

        let rtc = WebRtcConnection::new()?;
        let mut ice_candidate_stream = rtc.get_ice_candidates()?;

        let data_channel = rtc.create_data_channel("data");

        let offer = rtc.create_offer().await?;
        channel.send_session_description(&offer).await?;

        let answer = channel.receive_session_description().await?;
        rtc.set_remote_description(answer).await?;

        while let Some(ice_candidate) = ice_candidate_stream.recv().await {
            channel.send_ice_candidate(&ice_candidate).await?;
        }
        channel.stop_writing().await?;

        while let Some(ice_candidate) = channel.receive_ice_candidate().await? {
            rtc.add_ice_candidate(Some(&ice_candidate)).await?;
        }
        rtc.add_ice_candidate(None).await?;

        Ok(Connection {
            peer_connection: rtc.into_inner(),
            data_channel,
        })
    }
}

pub struct Channel {
    send: SendStream,
    recv: RecvStream,
}

impl Channel {
    pub async fn send_message(&mut self, message: String) -> Result<(), AnyhowError> {
        self.send.write_u16(message.len().try_into()?).await?;
        self.send.write_all(message.as_bytes()).await?;
        self.send.flush().await?;
        Ok(())
    }

    pub async fn send_session_description(
        &mut self,
        session_description: &RtcSessionDescription,
    ) -> Result<(), AnyhowError> {
        let json_js = JSON::stringify(&session_description).js_anyhow()?;
        let json_rs = json_js
            .as_string()
            .ok_or(anyhow!("Session description is not valid UTF-8"))?;
        self.send_message(json_rs).await
    }

    pub async fn send_ice_candidate(
        &mut self,
        ice_candidate: &RtcIceCandidate,
    ) -> Result<(), AnyhowError> {
        let json_js = JSON::stringify(&ice_candidate).js_anyhow()?;
        let json_rs = json_js
            .as_string()
            .ok_or(anyhow!("Ice candidate is not valid UTF-8"))?;
        self.send_message(json_rs).await
    }

    pub async fn stop_writing(&mut self) -> Result<(), AnyhowError> {
        self.send.write_u16(0).await?;
        self.send.finish()?;
        self.send.stopped().await?;
        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<Option<String>, AnyhowError> {
        let size = self.recv.read_u16().await?;
        if size == 0 {
            return Ok(None);
        }
        let mut buf = vec![0; size.try_into()?];
        match self.recv.read_exact(&mut buf).await {
            Err(ReadExactError::FinishedEarly(_)) => Ok(None),
            Err(other) => Err(other)?,
            Ok(()) => Ok(Some(String::from_utf8_lossy(&buf).to_string())),
        }
    }

    pub async fn receive_session_description(
        &mut self,
    ) -> Result<RtcSessionDescriptionInit, AnyhowError> {
        let offer_json = self
            .receive_message()
            .await?
            .ok_or(anyhow!("No session description received"))?;
        Ok(JSON::parse(&offer_json).js_anyhow()?.into())
    }

    pub async fn receive_ice_candidate(
        &mut self,
    ) -> Result<Option<RtcIceCandidateInit>, AnyhowError> {
        let Some(offer_json) = self.receive_message().await? else {
            return Ok(None);
        };
        Ok(Some(JSON::parse(&offer_json).js_anyhow()?.into()))
    }
}
