use futures::future::BoxFuture;
use futures::task::Context;
use hyper::body;
use hyper::client::HttpConnector;
use hyper::service::{make_service_fn, Service};
use hyper::{Body, Client, Request, Response, Server, StatusCode};
use hyper_tls::HttpsConnector;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use structopt::StructOpt;
use tokio::macros::support::Poll;

const PATH: &str = "/skin/";

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("{0}")]
    Json(
        #[from]
        #[source]
        serde_json::Error,
    ),
    #[error("{0}")]
    Http(
        #[from]
        #[source]
        hyper::http::Error,
    ),
    #[error("{0}")]
    Hyper(
        #[from]
        #[source]
        hyper::Error,
    ),
    #[error("{0}")]
    Base64Decode(
        #[from]
        #[source]
        base64::DecodeError,
    ),
}

#[derive(StructOpt)]
struct Args {
    #[structopt(short, default_value = "0.0.0.0:7653")]
    address: SocketAddr,
    #[structopt(short, default_value = "/skin/")]
    path: String,
}

#[derive(Deserialize)]
struct GetUuid {
    id: String,
}

#[derive(Deserialize)]
struct GetSkin {
    properties: Vec<Property>,
}

#[derive(Deserialize)]
struct Property {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct PropertyTextures {
    textures: Textures,
}

#[derive(Deserialize)]
struct Textures {
    #[serde(rename = "SKIN")]
    skin: Option<Skin>,
}

#[derive(Deserialize)]
struct Skin {
    url: String,
}

#[derive(Clone)]
struct SkinService {
    client: Client<HttpsConnector<HttpConnector>>,
    path: String,
}

impl SkinService {
    async fn call_async(&self, req: Request<Body>) -> Result<Response<Body>, Error> {
        let path = req.uri().path();
        if !path.starts_with(PATH) {
            return Ok(Self::not_found());
        }

        let nickname = &path[PATH.len()..];
        let uuid = self.get_uuid(nickname).await?;

        let skin = if let Some(uuid) = uuid {
            self.get_skin(uuid).await?
        } else {
            return Ok(Self::not_found());
        };

        let skin = if let Some(skin) = skin {
            skin
        } else {
            return Ok(Self::not_found());
        };

        let req = Request::builder().uri(skin).body(Body::empty())?;
        let resp = self.client.request(req).await?;
        Ok(resp)
    }

    fn not_found() -> Response<Body> {
        let mut resp = Response::new(Body::empty());
        *resp.status_mut() = StatusCode::NOT_FOUND;
        resp
    }

    async fn get_json<T: DeserializeOwned>(&self, req: Request<Body>) -> Result<T, Error> {
        let resp = self.client.request(req).await?;
        let bytes = body::to_bytes(resp).await?;
        Ok(serde_json::from_slice(bytes.as_ref())?)
    }

    async fn get_uuid(&self, nickname: &str) -> Result<Option<String>, Error> {
        let body = serde_json::to_string(&[nickname])?;
        let req = Request::builder()
            .method("POST")
            .uri("https://api.mojang.com/profiles/minecraft")
            .body(Body::from(body))?;
        let uuids: Vec<GetUuid> = self.get_json(req).await?;
        Ok(uuids.get(0).map(|resp| resp.id.clone()))
    }

    async fn get_skin(&self, uuid: String) -> Result<Option<String>, Error> {
        let uri = format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{}",
            uuid
        );
        let req = Request::builder().uri(uri).body(Body::empty())?;
        let properties = self.get_json::<GetSkin>(req).await?.properties;
        for property in properties {
            if property.name == "textures" {
                let value = base64::decode(property.value)?;
                let skin = serde_json::from_slice::<PropertyTextures>(value.as_ref())?
                    .textures
                    .skin
                    .map(|skin| skin.url);
                return Ok(skin);
            }
        }
        Ok(None)
    }
}

impl Service<Request<Body>> for SkinService {
    type Response = Response<Body>;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move { this.call_async(req).await })
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let Args { address, mut path } = Args::from_args();

    if !path.ends_with('/') {
        path.push('/');
    }

    let make_svc = make_service_fn(|_conn| {
        let path = path.clone();
        async move {
            Ok::<_, Infallible>(SkinService {
                client: Client::builder().build(HttpsConnector::new()),
                path,
            })
        }
    });

    let server = Server::bind(&address).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
