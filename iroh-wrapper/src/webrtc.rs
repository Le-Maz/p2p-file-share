use js_sys::{
    Array,
    wasm_bindgen::{JsCast, prelude::Closure},
};
use tokio::sync::{mpsc, oneshot};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    RtcConfiguration, RtcDataChannel, RtcDataChannelEvent, RtcIceCandidate, RtcIceCandidateInit,
    RtcIceGatheringState, RtcIceServer, RtcPeerConnection, RtcPeerConnectionIceEvent,
    RtcSessionDescription, RtcSessionDescriptionInit,
};

use crate::{
    debug,
    js_anyhow::{AnyhowError, JsAnyhow},
};

pub struct WebRtcConnection {
    inner: RtcPeerConnection,
}

impl WebRtcConnection {
    pub fn new() -> Result<Self, AnyhowError> {
        let config = RtcConfiguration::new();
        let servers = Array::new();
        [
            "stun:stun.l.google.com:19302",
            "stun:stun2.l.google.com:19302",
            "stun:stun3.l.google.com:19302",
            "stun:stun4.l.google.com:19302",
        ]
        .into_iter()
        .for_each(|url| {
            let server = RtcIceServer::new();
            let urls = Array::new();
            urls.push(&url.into());
            server.set_urls(&urls);
            servers.push(&server);
        });
        config.set_ice_servers(&servers);
        Ok(Self {
            inner: RtcPeerConnection::new_with_configuration(&config).js_anyhow()?,
        })
    }

    pub fn into_inner(self) -> RtcPeerConnection {
        self.inner
    }

    pub async fn create_offer(&self) -> Result<RtcSessionDescription, AnyhowError> {
        let future = JsFuture::from(self.inner.create_offer());
        let offer: RtcSessionDescription = future.await.js_anyhow()?.into();
        JsFuture::from(
            self.inner
                .set_local_description(&JsValue::from(offer.clone()).into()),
        )
        .await
        .js_anyhow()?;
        Ok(offer)
    }

    pub async fn create_answer(&self) -> Result<RtcSessionDescription, AnyhowError> {
        let future = JsFuture::from(self.inner.create_answer());
        let answer: RtcSessionDescription = future.await.js_anyhow()?.into();
        JsFuture::from(
            self.inner
                .set_local_description(&JsValue::from(answer.clone()).into()),
        )
        .await
        .js_anyhow()?;
        Ok(answer)
    }

    pub async fn set_remote_description(
        &self,
        description: RtcSessionDescriptionInit,
    ) -> Result<(), AnyhowError> {
        let promise = self.inner.set_remote_description(&description);
        JsFuture::from(promise).await.js_anyhow()?;
        Ok(())
    }

    pub fn create_data_channel(&self, label: &str) -> RtcDataChannel {
        self.inner.create_data_channel(label)
    }

    pub fn accept_data_channel(&self) -> oneshot::Receiver<RtcDataChannel> {
        let (send, recv) = oneshot::channel();

        let connection = self.inner.clone();
        let handler = Closure::once_into_js(move |event: RtcDataChannelEvent| {
            let _ = send.send(event.channel());
            connection.set_ondatachannel(None);
        });

        self.inner.set_ondatachannel(Some(handler.unchecked_ref()));

        recv
    }

    pub fn get_ice_candidates(&self) -> Result<IceCandidateStream, AnyhowError> {
        let (candidate_send, candidate_recv) = mpsc::unbounded_channel();

        let connection = self.inner.clone();
        let candidate_send_clone = candidate_send.clone();
        let ice_candidate_handler = Closure::<dyn Fn(RtcPeerConnectionIceEvent)>::new(
            move |event: RtcPeerConnectionIceEvent| {
                debug("ICE candidate");
                let Some(candidate) = event.candidate() else {
                    connection.set_onicecandidate(None);
                    connection.set_onicegatheringstatechange(None);
                    let _ = candidate_send_clone.send(None);
                    return;
                };
                if candidate_send_clone.send(Some(candidate)).is_err() {
                    connection.set_onicecandidate(None);
                    connection.set_onicegatheringstatechange(None);
                }
            },
        );
        let connection = self.inner.clone();
        let ice_gathering_state_change_handler = Closure::<dyn Fn()>::new(move || {
            debug("ICE gathering state changed");
            if connection.ice_gathering_state() == RtcIceGatheringState::Complete {
                connection.set_onicecandidate(None);
                connection.set_onicegatheringstatechange(None);
                let _ = candidate_send.send(None);
            }
        });

        self.inner
            .set_onicecandidate(Some(ice_candidate_handler.as_ref().unchecked_ref()));
        self.inner.set_onicegatheringstatechange(Some(
            ice_gathering_state_change_handler.as_ref().unchecked_ref(),
        ));

        ice_candidate_handler.forget();
        ice_gathering_state_change_handler.forget();

        Ok(IceCandidateStream { candidate_recv })
    }

    pub async fn add_ice_candidate(
        &self,
        candidate: Option<&RtcIceCandidateInit>,
    ) -> Result<(), AnyhowError> {
        let promise = self
            .inner
            .add_ice_candidate_with_opt_rtc_ice_candidate_init(candidate);
        JsFuture::from(promise).await.js_anyhow()?;
        Ok(())
    }
}

pub struct IceCandidateStream {
    candidate_recv: mpsc::UnboundedReceiver<Option<RtcIceCandidate>>,
}

impl IceCandidateStream {
    pub async fn recv(&mut self) -> Option<RtcIceCandidate> {
        self.candidate_recv.recv().await.flatten()
    }
}
