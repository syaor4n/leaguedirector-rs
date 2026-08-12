use super::error::{ApiError, Result};
use super::types::{Game, Particles, Playback, Recording, Render, Sequence};
use reqwest::blocking::Client;
use reqwest::Certificate;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

const HOST: &str = "https://127.0.0.1:2999";
const PEM: &str = include_str!("../../assets/riotgames.pem");

#[derive(Clone)]
pub struct ReplayClient {
    http: Client,
}

impl ReplayClient {
    pub fn new() -> Result<Self> {
        let cert = Certificate::from_pem(PEM.as_bytes())?;
        let http = Client::builder()
            .add_root_certificate(cert)
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(3))
            .connect_timeout(Duration::from_millis(400))
            .no_proxy()
            .build()?;
        Ok(Self { http })
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self.http.get(format!("{HOST}{path}")).send()?;
        if !response.status().is_success() {
            return Err(ApiError::Disconnected);
        }
        Ok(response.json()?)
    }

    fn post<T: Serialize>(&self, path: &str, body: &T) -> Result<()> {
        let response = self.http.post(format!("{HOST}{path}")).json(body).send()?;
        if response.status().is_success() || response.status().as_u16() == 204 {
            Ok(())
        } else {
            Err(ApiError::Message(format!(
                "{} {}",
                path,
                response.status()
            )))
        }
    }

    pub fn game(&self) -> Result<Game> {
        self.get("/replay/game")
    }

    pub fn playback(&self) -> Result<Playback> {
        self.get("/replay/playback")
    }

    pub fn set_playback(&self, body: &serde_json::Value) -> Result<()> {
        self.post("/replay/playback", body)
    }

    pub fn render(&self) -> Result<Render> {
        self.get("/replay/render")
    }

    pub fn set_render(&self, body: &serde_json::Value) -> Result<()> {
        self.post("/replay/render", body)
    }

    pub fn recording(&self) -> Result<Recording> {
        self.get("/replay/recording")
    }

    pub fn set_recording(&self, body: &serde_json::Value) -> Result<()> {
        let long = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(8))
            .connect_timeout(Duration::from_secs(2))
            .no_proxy()
            .build()?;
        let response = long
            .post(format!("{HOST}/replay/recording"))
            .json(body)
            .send()?;
        if response.status().is_success() || response.status().as_u16() == 204 {
            Ok(())
        } else {
            Err(ApiError::Message(format!(
                "/replay/recording {}",
                response.status()
            )))
        }
    }

    pub fn particles(&self) -> Result<Particles> {
        self.get("/replay/particles")
    }

    pub fn set_particle(&self, name: &str, enabled: bool) -> Result<()> {
        let mut map = serde_json::Map::new();
        map.insert(name.to_string(), serde_json::Value::Bool(enabled));
        self.post("/replay/particles", &serde_json::Value::Object(map))
    }

    #[allow(dead_code)]
    pub fn sequence(&self) -> Result<Sequence> {
        self.get("/replay/sequence")
    }

    pub fn set_sequence(&self, sequence: &Sequence) -> Result<()> {
        self.post("/replay/sequence", sequence)
    }

    pub fn clear_sequence(&self) -> Result<()> {
        self.post("/replay/sequence", &serde_json::json!({}))
    }
}
