use actix_web::HttpRequest;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub(crate) struct Ip {
    #[serde(default = "default_field")]
    pub organization: String,
    #[serde(default = "default_field")]
    pub isp: String,
    #[serde(default = "default_field")]
    pub city: String,
    #[serde(default = "default_field")]
    pub country: String,
    #[serde(default = "default_field")]
    pub ip: String,
}

fn default_field() -> String {
    "~".to_string()
}

pub(crate) async fn handle_ip(req: HttpRequest) -> Ip {
    let ip_string = req.peer_addr().unwrap().ip().to_string();
    let url = format!("https://api.ip.sb/geoip/{}", ip_string);
    reqwest::get(url.as_str())
        .await
        .unwrap()
        .json::<Ip>()
        .await
        .unwrap()
}
