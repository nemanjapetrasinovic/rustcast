#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod data_provider;
mod entity;
mod podcasts_model;

use crossbeam::channel::{unbounded, Receiver, Sender};
use eframe::egui;
use log::error;
use podcasts_model::PodcastsModel;
use url2audio::Player;
use crate::podcasts_model::Podcast;
use std::sync::{Arc, RwLock};

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

#[derive(Debug, PartialEq, Clone)]
pub enum AsyncAction {
    AddPodcast
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let (tx, rx) = unbounded::<PlayerAction>();
    let (tx1, rx1) = unbounded::<PlayerState>();

    let podcasts_model = Arc::new(RwLock::new(PodcastsModel::new()));
    let podcasts_model_1 = podcasts_model.clone();
    let podcasts_model_2 = podcasts_model.clone();

    let player_thread = tokio::spawn(async move {
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
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rustcast",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc, tx, rx1, PlayerState::Open, podcasts_model_2))),
    )
    .unwrap_or_else(|e| error!("An error occured {}", e));

    player_thread.await.unwrap();
}

struct MyEguiApp {
    tx: Sender<PlayerAction>,
    rx: Receiver<PlayerState>,
    player_state: PlayerState,
    podcast_to_add: Podcast,
    show_add_podcast: bool,
    podcasts_model: Arc<RwLock<PodcastsModel>>,
    show_error: bool,
    error: String,
}

impl MyEguiApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        tx: Sender<PlayerAction>,
        rx: Receiver<PlayerState>,
        player_state: PlayerState,
        podcasts_model: Arc<RwLock<PodcastsModel>>
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        MyEguiApp {
            tx,
            rx,
            player_state: PlayerState::Paused,
            podcast_to_add: Podcast::default(),
            show_add_podcast: false,
            podcasts_model,
            show_error: false,
            error: String::new()
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.rx.try_recv() {
            Ok(player_state) => self.player_state = player_state,
            Err(_) => {}
        }

        egui::SidePanel::left("podcasts_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(150.0..=600.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Podcasts");
                        if ui.add(egui::Button::new("+")).on_hover_text("Add podcast").clicked() {
                            self.show_add_podcast = true;
                        }
                    });
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
            //     ui.add(egui::TextEdit::singleline(&mut self.src_url).hint_text("Stream url")).highlight();
            // });
            // if ui.add(egui::Button::new("Add +")).clicked() {
            //     self.show_add_podcast = true;
            // }
            ui.horizontal(|ui| {
                ui.vertical_centered(|ui| {
                    if self.player_state == PlayerState::Paused {
                        if ui.add(egui::Button::new("Play")).clicked() {
                            // self.tx.try_send(PlayerAction::Open(self.src_url.clone()));
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

        if self.show_add_podcast {
            egui::Window::new("Add podcast")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.write().unwrap().new_podcast.link).hint_text("Podcast url"));
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.write().unwrap().new_podcast.title).hint_text("Podcast title"));
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.write().unwrap().new_podcast.description).hint_text("Podcast description"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            if ui.add(egui::Button::new("Close")).clicked() {
                                let mut p = self.podcasts_model.write().unwrap();
                                p.new_podcast.link = String::new();
                                p.new_podcast.title = String::new();
                                p.new_podcast.description = String::new();
                                drop(p);
                                self.show_add_podcast = false;
                            }
                            if ui.add(egui::Button::new("Add")).clicked() {
                            }
                        });
                    });
                });
        }

        if self.show_error {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.label(self.error.clone());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.add(egui::Button::new("Ok")).clicked() {
                            self.show_error = false;
                            self.error = String::new();
                        }
                    })
                });
        }
    }
}
