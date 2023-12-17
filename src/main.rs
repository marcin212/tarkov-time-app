#![feature(fn_traits)]
#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};
use reqwest;
use reqwest::blocking::Response;
use serde::{Deserialize, Serialize};
use tray_item::{IconSource, TrayItem};

use process_list::for_each_process;

#[derive(Debug, Deserialize)]
struct CoreProps {
    address: String,
}


#[derive(Debug, Serialize)]
struct GameMetadata {
    game: String,
    game_display_name: String,
    developer: String,
    deinitialize_timer_length_ms: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
struct ScreenHandler {
    device_type: String,
    zone: String,
    mode: String,
    datas: Vec<ScreenHandlerData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
struct ScreenHandlerDataLine {
    has_text: bool,
    context_frame_key: String,
    wrap: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
struct ScreenHandlerData {
    lines: Vec<ScreenHandlerDataLine>,
    icon_id: i32,
}

#[derive(Debug, Serialize)]
struct BindEventDefinition {
    game: String,
    event: String,
    icon_id: i32,
    value_optional: bool,
    handlers: Vec<ScreenHandler>,
    data_fields: Vec<DataField>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "kebab-case"))]
struct DataField {
    context_frame_key: String,
    label: String,
}

#[derive(Debug, Serialize)]
struct Event {
    game: String,
    event: String,
    data: EventData,
}

#[derive(Debug, Serialize)]
struct Game {
    game: String,
}

#[derive(Debug, Serialize)]
struct EventData {
    value: i32,
    frame: HashMap<String, String>,
}

struct GameSenseService {
    url: String,
    client: reqwest::blocking::Client,
}


impl GameSenseService {
    fn new(props: CoreProps) -> Self {
        return Self {
            url: format!("http://{}", props.address),
            client: reqwest::blocking::Client::new(),
        };
    }

    fn post(&self, path: String, body: String) -> reqwest::Result<Response> {
        //println!("{}", format!("{}/{}", self.url, path));
        //println!("{}", body);
        return self.client.post(format!("{}/{}", self.url, path)).body(body).header("Content-Type", "application/json").send();
    }

    fn register_game(&self, game_metadata: GameMetadata) -> reqwest::Result<Response> {
        return self.post("game_metadata".to_owned(), serde_json::to_string(&game_metadata).unwrap());
    }

    fn remove_game(&self, game: Game) -> reqwest::Result<Response> {
        return self.post("remove_game".to_owned(), serde_json::to_string(&game).unwrap());
    }

    fn bind_game_event(&self, bind_event_definition: BindEventDefinition) -> reqwest::Result<Response> {
        return self.post("bind_game_event".to_owned(), serde_json::to_string(&bind_event_definition).unwrap());
    }

    fn send_event(&self, event: Event) -> reqwest::Result<Response> {
        return self.post("game_event".to_owned(), serde_json::to_string(&event).unwrap());
    }
}

struct TarkovTime {
    right_time: String,
    left_time: String,
}

enum Message {
    Quit,
    QuitWithRemove,
    ModeChangeEnable,
    ModeChangeDisable,
    ModeChangeGameDetection,
}

#[derive(Debug, Copy, Clone)]
enum Mode {
    Enable,
    Disable,
    GameDetection,
}


fn hrs(num: u64) -> u64 {
    return 60 * 60 * num;
}

fn real_time_to_tarkov_time(time: u64, left: bool) -> u64 {
    let tarkov_ratio = 7;
    let one_day = hrs(24);
    let russia = hrs(3);
    let offset = if left { 0 } else { hrs(12) } + russia;

    return (offset + (time * tarkov_ratio)) % one_day;
}


fn calculate_tarkov_time() -> TarkovTime {
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let right_time = real_time_to_tarkov_time(now, false);
    let left_time = real_time_to_tarkov_time(now, true);

    let right_time = UNIX_EPOCH + Duration::from_secs(right_time);
    let left_time = UNIX_EPOCH + Duration::from_secs(left_time);


    return TarkovTime {
        left_time: DateTime::<Utc>::from(left_time).format("%H:%M:%S").to_string(),
        right_time: DateTime::<Utc>::from(right_time).format("%H:%M:%S").to_string(),
    };
}


struct App<APP: FnMut(Mode), QUIT: Fn()> {
    tray: TrayItem,
    mode: Mode,
    menu_item_game_detection: u32,
    menu_item_game_enable: u32,
    menu_item_game_disable: u32,
    tick: u64,
    app_loop: APP,
    quit_remove: QUIT,
    channel_rx: Receiver<Message>,
}

impl<APP, QUIT> App<APP, QUIT> where APP: FnMut(Mode), QUIT: Fn()
{
    fn new(tick: u64, app_loop: APP, quit_remove: QUIT) -> Self {
        let mut tray = TrayItem::new(
            "Tarkov Time",
            IconSource::Resource("tarkov-time-icon"),
        ).unwrap();
        tray.add_label("Tarkov Time").unwrap();

        let (tx, rx) = mpsc::sync_channel(1);

        let menu_item_game_detection_tx = tx.clone();
        let menu_item_game_detection = tray.inner_mut().add_menu_item_with_id("* Game Detection *", move || {
            menu_item_game_detection_tx.send(Message::ModeChangeGameDetection).unwrap();
        }).unwrap();

        let menu_item_game_enable_tx = tx.clone();
        let menu_item_game_enable = tray.inner_mut().add_menu_item_with_id("Enable", move || {
            menu_item_game_enable_tx.send(Message::ModeChangeEnable).unwrap();
        }).unwrap();

        let menu_item_game_disable_tx = tx.clone();
        let menu_item_game_disable = tray.inner_mut().add_menu_item_with_id("Disable", move || {
            menu_item_game_disable_tx.send(Message::ModeChangeDisable).unwrap();
        }).unwrap();

        tray.inner_mut().add_separator().unwrap();

        let quit_remove_tx = tx.clone();
        tray.add_menu_item("Quit and Remove", move || {
            quit_remove_tx.send(Message::QuitWithRemove).unwrap();
        }).unwrap();


        let quit_tx = tx.clone();
        tray.add_menu_item("Quit", move || {
            quit_tx.send(Message::Quit).unwrap();
        }).unwrap();

        return Self {
            tray,
            mode: Mode::GameDetection,
            menu_item_game_detection,
            menu_item_game_enable,
            menu_item_game_disable,
            tick,
            app_loop,
            quit_remove,
            channel_rx: rx
        };
    }

    fn on_game_detection_mode(&mut self) {
        self.mode = Mode::GameDetection;
        self.tray.inner_mut().set_menu_item_label("* Game Detection *", self.menu_item_game_detection).unwrap();
        self.tray.inner_mut().set_menu_item_label("Enable", self.menu_item_game_enable).unwrap();
        self.tray.inner_mut().set_menu_item_label("Disable", self.menu_item_game_disable).unwrap();
    }

    fn on_game_enable_mode(&mut self) {
        self.mode = Mode::Enable;
        self.tray.inner_mut().set_menu_item_label("Game Detection", self.menu_item_game_detection).unwrap();
        self.tray.inner_mut().set_menu_item_label("* Enable *", self.menu_item_game_enable).unwrap();
        self.tray.inner_mut().set_menu_item_label("Disable", self.menu_item_game_disable).unwrap();
    }

    fn on_game_disable_mode(&mut self) {
        self.mode = Mode::Disable;
        self.tray.inner_mut().set_menu_item_label("Game Detection", self.menu_item_game_detection).unwrap();
        self.tray.inner_mut().set_menu_item_label("Enable", self.menu_item_game_enable).unwrap();
        self.tray.inner_mut().set_menu_item_label("* Disable *", self.menu_item_game_disable).unwrap();
    }

    fn start(&mut self) {
        loop {
            match self.channel_rx.recv_timeout(Duration::from_secs(self.tick)) {
                Ok(Message::Quit) => {
                    break;
                }
                Ok(Message::QuitWithRemove) => {
                    self.quit_remove.call(());
                    break;
                }
                Ok(Message::ModeChangeDisable) => {
                    self.on_game_disable_mode();
                }
                Ok(Message::ModeChangeEnable) => {
                    self.on_game_enable_mode();
                }
                Ok(Message::ModeChangeGameDetection) => {
                    self.on_game_detection_mode();
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.app_loop.call_mut((self.mode, ));
                }
                _ => {}
            }
        }
    }
}

struct TarkovDetector {
    last_time: Instant,
    is_running: bool,
}

impl TarkovDetector {

    fn new() -> Self {
        return Self {
            last_time: Instant::now(),
            is_running: false
        }
    }

    fn tarkov_running(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_time).as_secs() > 10 {
           self.last_time = now;
            let mut is_running = false;
            for_each_process(|_id: u32, name: &Path| {
                if name.display().to_string().contains("EscapeFromTarkov.exe") {
                    is_running = true;
                }
            }).unwrap();
            self.is_running = is_running;
        }
        return self.is_running;
    }
}


fn main() {
    let program_data_dir = std::env::var("ProgramData")
        .expect("ProgramData env not found");

    let config = fs::read_to_string(format!("{program_data_dir}/SteelSeries/SteelSeries Engine 3/coreProps.json"))
        .expect("Unable to find SteelSeries config");

    let core_props: CoreProps = serde_json::from_str(&config).unwrap();
    let game_key = "TARKOV_TIME";
    let game_sense_service = GameSenseService::new(core_props);


    game_sense_service.register_game(GameMetadata {
        game: game_key.to_owned(),
        developer: "marcin212".to_owned(),
        deinitialize_timer_length_ms: 5000,
        game_display_name: "Escape From Tarkov Time".to_owned(),
    }).unwrap().error_for_status().expect("Unable to register Game");


    game_sense_service.bind_game_event(BindEventDefinition {
        game: game_key.to_owned(),
        event: "TIME".to_owned(),
        icon_id: 15,
        value_optional: true,
        handlers: vec![
            ScreenHandler {
                zone: "one".to_owned(),
                device_type: "screened".to_owned(),
                mode: "screen".to_owned(),
                datas: vec![
                    ScreenHandlerData {
                        icon_id: 15,
                        lines: vec![
                            ScreenHandlerDataLine {
                                context_frame_key: "time-left".to_owned(),
                                has_text: true,
                                wrap: 0,
                            },
                            ScreenHandlerDataLine {
                                context_frame_key: "time-right".to_owned(),
                                has_text: true,
                                wrap: 0,
                            },
                        ],
                    }
                ],
            }
        ],
        data_fields: vec![DataField {
            label: "Time".to_owned(),
            context_frame_key: "time".to_owned(),
        }],
    }).unwrap().error_for_status().expect("Unable to register Bind Game Event");
    println!("Running ....");

    let mut tarkov_detector = TarkovDetector::new();
    let main_fn = |mode: Mode| {
        if (matches!(mode, Mode::GameDetection) && tarkov_detector.tarkov_running()) || matches!(mode, Mode::Enable) {
            let mut frame_data = HashMap::new();
            let time = calculate_tarkov_time();
            frame_data.insert("time-left".to_owned(), time.left_time);
            frame_data.insert("time-right".to_owned(), time.right_time);

            let _ = game_sense_service.send_event(Event {
                event: "TIME".to_owned(),
                game: game_key.to_owned(),
                data: EventData {
                    value: 1,
                    frame: frame_data,
                },
            });
        }
    };

    let remove_app_fn = || {
        game_sense_service.remove_game(Game {
            game: game_key.to_owned()
        }).unwrap().error_for_status().expect("Unable to Remove Game");
    };

    let mut app = App::new(1, main_fn, remove_app_fn);
    app.start();
}
