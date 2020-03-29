use actix_http::encoding::Decoder;
use actix_web::{
    client::{Client, ClientResponse, Connector},
    dev::{Payload, PayloadStream},
    Error as ActixError, ResponseError,
};
use rustls::ClientConfig;
use serde::Deserialize;
use std::sync::Arc;
use thiserror::Error;
use webpki_roots::TLS_SERVER_ROOTS;

type Result<T> = std::result::Result<T, ActixError>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{}: {}", .0.error, .0.error_message)]
    Api(ApiError),
}

impl ResponseError for Error {}

pub struct ApiClient(Client);

impl ApiClient {
    pub async fn uuids_by_playernames(&self, playernames: &[String]) -> Result<Vec<GetUuid>> {
        Ok(self
            .0
            .post("https://api.mojang.com/profiles/minecraft")
            .send_json(&playernames)
            .await?
            .json::<SuccessOrError<_>>()
            .await?
            .into_success()?)
    }

    pub async fn profile(&self, uuid: String) -> Result<Profile> {
        Ok(self
            .0
            .get(format!(
                "https://sessionserver.mojang.com/session/minecraft/profile/{}",
                uuid
            ))
            .send()
            .await?
            .json::<SuccessOrError<_>>()
            .await?
            .into_success()?)
    }

    pub async fn skin(
        &self,
        skin: Skin,
    ) -> Result<ClientResponse<Decoder<Payload<PayloadStream>>>> {
        Ok(self.0.get(skin.url).send().await?)
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        let mut config = ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&TLS_SERVER_ROOTS);
        let connector = Connector::new().rustls(Arc::new(config)).finish();
        Self(Client::build().connector(connector).finish())
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SuccessOrError<T> {
    Success(T),
    Error(ApiError),
}

impl<T> SuccessOrError<T> {
    pub fn into_success(self) -> Result<T> {
        match self {
            Self::Success(inner) => Ok(inner),
            Self::Error(err) => Err(Error::Api(err).into()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: String,
    #[serde(rename = "errorMessage")]
    pub error_message: String,
}

#[derive(Deserialize)]
pub struct GetUuid {
    pub id: String,
}

#[derive(Deserialize)]
pub struct Profile {
    pub properties: Vec<Property>,
}

#[derive(Deserialize)]
pub struct Property {
    pub name: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct PropertyTextures {
    pub textures: Textures,
}

#[derive(Deserialize)]
pub struct Textures {
    #[serde(rename = "SKIN")]
    pub skin: Option<Skin>,
}

#[derive(Deserialize)]
pub struct Skin {
    pub url: String,
}
