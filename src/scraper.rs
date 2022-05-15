use anyhow::{Context, Result};
use macroquad::texture::Image;
use reqwest::blocking::{Client, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct IgdbClient {
    pub client: Client,
    pub client_id: String,
    pub access_token: String,
}

impl IgdbClient {
    pub fn request_raw(&self, endpoint: &str, body: String) -> Result<Response, reqwest::Error> {
        self.client
            .post(format!("https://api.igdb.com/v4/{}", endpoint))
            .header("Client-ID", &self.client_id)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .body(body)
            .send()
    }

    pub fn request<T: DeserializeOwned>(&self, endpoint: &str, body: &str) -> Result<T> {
        let res = self.request_raw(endpoint, body.to_string())?;
        let body = res.bytes()?;
        dbg!(&body);
        serde_json::from_slice(&body).context("Malformed response body")
    }

    pub fn request_cover(&self, id: IgdbGameId) -> Result<Image> {
        let req = format!("fields game, url, width, height; where id = {};", id.0);
        let images: Vec<IgdbCover> = self.request("covers", &req)?;
        todo!()
    }

    pub fn request_game_search(&self, title: &str) -> Result<Vec<IgdbGame>> {
        let req = format!("fields id, name, cover; search \"{}\"; where version_parent = null;", title);
        let games: Vec<IgdbGame> = self.request("games", &req)?;
        Ok(games)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct IgdbGameId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct IgdbCoverId(pub u32);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct IgdbGame {
    pub id: IgdbGameId,
    pub name: String,
    pub cover: IgdbCoverId,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct IgdbCover {
    pub id: IgdbCoverId,
    pub game: IgdbGameId,
    pub url: String,
    pub width: u32,
    pub height: u32,
}
