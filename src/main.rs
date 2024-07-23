#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use crossbeam::channel::{unbounded, Receiver, Sender};
use eframe::egui;
use log::error;
use std::thread;
use url2audio::Player;

#[derive(Debug, PartialEq, Clone)]
pub enum PlayerAction {
    Play,
    Pause,
    Open(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum PlayerState {
    Open,
    Playing,
    Paused,
}

pub struct PlayerWrapper {
    pub inner_player: Player,
    pub player_state: PlayerState,
}

fn main() {
    env_logger::init();
    let (tx, rx) = unbounded::<PlayerAction>();
    let (tx1, rx1) = unbounded::<PlayerState>();
    let mut p = Player::new();

    let player_thread = thread::spawn(move || {
        let player = Player::new();
        let mut player_wrapper = PlayerWrapper {
            inner_player: player,
            player_state: PlayerState::Paused,
        };
        // let src = "https://podcast.daskoimladja.com/media/2024-05-27-PONEDELJAK_27.05.2024.mp3";
        let src = "https://stream.daskoimladja.com:9000/stream";

        loop {
            match rx.recv() {
                Ok(PlayerAction::Open(src)) => {
                    player_wrapper.inner_player.open(&src);
                    tx1.try_send(PlayerState::Playing);
                }
                Ok(PlayerAction::Play) => {
                    player_wrapper.inner_player.play();
                    tx1.try_send(PlayerState::Playing);
                }
                Ok(PlayerAction::Pause) => {
                    player_wrapper.inner_player.pause();
                    tx1.try_send(PlayerState::Paused);
                }
                Err(e) => {
                    error!("{}", e);
                    break;
                }
            }
        }
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rustcast",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc, tx, rx1, PlayerState::Open))),
    )
    .unwrap_or_else(|e| error!("An error occured {}", e));

    player_thread.join().unwrap();
}

struct MyEguiApp {
    tx: Sender<PlayerAction>,
    rx: Receiver<PlayerState>,
    player_state: PlayerState,
    src_url: String,
    show_add_stram: bool
}

impl MyEguiApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        tx: Sender<PlayerAction>,
        rx: Receiver<PlayerState>,
        player_state: PlayerState,
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        MyEguiApp {
            tx,
            rx,
            player_state: PlayerState::Paused,
            src_url: String::new(),
            show_add_stram: false,
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.rx.try_recv() {
            Ok(player_state) => self.player_state = player_state,
            Err(_) => {}
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.src_url).hint_text("Stream url")).highlight();
            });
            if ui.add(egui::Button::new("Add +")).clicked() {
                self.show_add_stram = true;
            }
            ui.horizontal(|ui| {
                ui.vertical_centered(|ui| {
                    if self.player_state == PlayerState::Paused {
                        if ui.add(egui::Button::new("Play")).clicked() {
                            self.tx.try_send(PlayerAction::Open(self.src_url.clone()));
                            self.tx.try_send(PlayerAction::Play);
                        }
                    }
                    if self.player_state == PlayerState::Playing || self.player_state == PlayerState::Open {
                        if ui.add(egui::Button::new("Pause")).clicked() {
                            self.tx.send(PlayerAction::Pause);
                        }
                    }
                })
            });
        });
        if self.show_add_stram {
            egui::Window::new("Add Stream")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.src_url).hint_text("Stream url")).highlight();
            });
            });
        }
    }
}
