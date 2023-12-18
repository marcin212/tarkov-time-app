#![feature(fn_traits)]
#![windows_subsystem = "windows"]

mod data_structure;
mod tarkov_time;
mod game_sense_service;
mod tarkov_detector;

use std::collections::HashMap;
use std::fs;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver};
use std::time::{Duration };
use tray_item::{IconSource, TrayItem};
use crate::data_structure::{BindEventDefinition, CoreProps, DataField, Event, EventData, Game, GameMetadata, ScreenHandler, ScreenHandlerData, ScreenHandlerDataLine};
use crate::game_sense_service::GameSenseService;
use crate::tarkov_detector::TarkovDetector;
use crate::tarkov_time::calculate_tarkov_time;


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
