use reqwest::blocking::Response;
use crate::data_structure::{BindEventDefinition, CoreProps, Event, Game, GameMetadata};

pub struct GameSenseService {
    url: String,
    client: reqwest::blocking::Client,
}

impl GameSenseService {
    pub fn new(props: CoreProps) -> Self {
        Self {
            url: format!("http://{}", props.address),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn post(&self, path: String, body: String) -> reqwest::Result<Response> {
        self.client.post(format!("{}/{}", self.url, path)).body(body).header("Content-Type", "application/json").send()
    }

    pub fn register_game(&self, game_metadata: GameMetadata) -> reqwest::Result<Response> {
        self.post("game_metadata".to_owned(), serde_json::to_string(&game_metadata).unwrap())
    }

    pub fn remove_game(&self, game: Game) -> reqwest::Result<Response> {
        self.post("remove_game".to_owned(), serde_json::to_string(&game).unwrap())
    }

    pub fn bind_game_event(&self, bind_event_definition: BindEventDefinition) -> reqwest::Result<Response> {
        self.post("bind_game_event".to_owned(), serde_json::to_string(&bind_event_definition).unwrap())
    }

    pub fn send_event(&self, event: Event) -> reqwest::Result<Response> {
        self.post("game_event".to_owned(), serde_json::to_string(&event).unwrap())
    }
}
