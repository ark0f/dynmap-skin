mod api;
mod error;
mod ratelimit;

use crate::{api::ApiClient, error::Error, ratelimit::RateLimit};
use actix_web::{
    get,
    middleware::{Compress, Logger},
    web,
    web::{Data, Path},
    App, Error as ActixError, HttpResponse, HttpServer,
};
use std::{io, net::SocketAddr, sync::Arc};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    #[structopt(short, default_value = "0.0.0.0:7653")]
    address: SocketAddr,
    #[structopt(short, default_value = "/skin")]
    path: String,
    #[structopt(short, default_value = "30")]
    rate_limit: u64,
}

struct State {
    client: ApiClient,
    rate_limit: Arc<RateLimit>,
}

#[get("/{playername}")]
async fn get_skin(
    state: Data<State>,
    playername: Path<String>,
) -> Result<HttpResponse, ActixError> {
    let client = &state.client;
    state.rate_limit.wait().await;

    let uuids = client
        .uuids_by_playernames(&[playername.into_inner()])
        .await?;
    let uuid = uuids
        .into_iter()
        .next()
        .map(|get_uuid| get_uuid.id)
        .ok_or(Error::NoPlayer)?;

    let profile = client.profile(uuid).await?;
    let value = profile
        .properties
        .into_iter()
        .find(|prop| prop.name == "textures")
        .ok_or(Error::NoSkin)?
        .value;
    let value = base64::decode(value).map_err(Error::Base64)?;
    let property: api::PropertyTextures = serde_json::from_slice(&value).map_err(Error::Json)?;
    let skin = property.textures.skin.ok_or(Error::NoSkin)?;

    let image = client.skin(skin).await?;
    let resp = HttpResponse::Ok().streaming(image);

    Ok(resp)
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    let Args {
        address,
        path,
        rate_limit,
    } = Args::from_args();

    let rate_limit = Arc::new(RateLimit::new(rate_limit));

    HttpServer::new(move || {
        App::new()
            .data(State {
                client: ApiClient::default(),
                rate_limit: rate_limit.clone(),
            })
            .wrap(Logger::default())
            .wrap(Compress::default())
            .service(web::scope(&path).service(get_skin))
    })
    .bind(address)?
    .run()
    .await
}
