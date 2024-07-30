#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod data_provider;
mod entity;
mod podcasts_model;

use data_provider::DataProvider;
use eframe::egui;
use log::error;
use podcasts_model::PodcastsModel;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use url2audio::Player;
use crate::podcasts_model::Podcast;
use sea_orm::{Database, DatabaseConnection};

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
    AddPodcast(Podcast),
    GetPodcasts,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AsyncActionResult {
    PodcastsUpdate(Vec<Podcast>)
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let (tx, mut rx) = unbounded_channel::<PlayerAction>();
    let (tx1, rx1) = unbounded_channel::<PlayerState>();

    let (async_action_tx, mut async_action_rx) = unbounded_channel::<AsyncAction>();
    let (async_action_result_tx, async_action_result_rx) = unbounded_channel::<AsyncActionResult>();

    let async_action_thread = tokio::spawn(async move {
        let home = std::env::var("HOME").unwrap();
        let connection = std::env::var("DATABASE_URL").unwrap_or(format!("sqlite://{}/.rustcast.db?mode=rwc", home));
        let db: DatabaseConnection = Database::connect(connection)
            .await
            .unwrap();

        let data_provider = DataProvider::new(db);
        data_provider.get_podcasts().await.unwrap();

        loop {
            match async_action_rx.recv().await {
                Some(AsyncAction::AddPodcast(podcast)) => {
                    data_provider.add_podcast(podcast)
                        .await
                        .map_err(|e| error!("{}", e));
                },
                Some(AsyncAction::GetPodcasts) => {
                    if let Ok(res) = data_provider.get_podcasts().await {
                        async_action_result_tx.send(AsyncActionResult::PodcastsUpdate(res));
                    }
                }
                None => break
            }
        }
    });

    let player_thread = tokio::spawn(async move {
        let player = Player::new();
        let mut player_wrapper = PlayerWrapper {
            inner_player: player,
            player_state: PlayerState::Paused,
        };

        // let src = "https://podcast.daskoimladja.com/media/2024-05-27-PONEDELJAK_27.05.2024.mp3";
        // let src = "https://stream.daskoimladja.com:9000/stream";

        loop {
            match rx.recv().await {
                Some(PlayerAction::Open(src)) => {
                    player_wrapper.inner_player.open(&src);
                    tx1.send(PlayerState::Playing);
                }
                Some(PlayerAction::Play) => {
                    player_wrapper.inner_player.play();
                    tx1.send(PlayerState::Playing);
                }
                Some(PlayerAction::Pause) => {
                    player_wrapper.inner_player.pause();
                    tx1.send(PlayerState::Paused);
                }
                None => {
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
        Box::new(move |cc| Box::new(MyEguiApp::new(cc, tx, rx1, async_action_tx, async_action_result_rx, PlayerState::Open, PodcastsModel::new()))),
    )
    .unwrap_or_else(|e| error!("An error occured {}", e));

    player_thread.await.unwrap();
    async_action_thread.await.unwrap();
}

struct MyEguiApp {
    tx: UnboundedSender<PlayerAction>,
    rx: UnboundedReceiver<PlayerState>,
    async_action_tx: UnboundedSender<AsyncAction>,
    async_action_result_rx: UnboundedReceiver<AsyncActionResult>,
    player_state: PlayerState,
    show_add_podcast: bool,
    podcasts_model: PodcastsModel,
    show_error: bool,
    error: String,
}

impl MyEguiApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        tx: UnboundedSender<PlayerAction>,
        rx: UnboundedReceiver<PlayerState>,
        async_action_tx: UnboundedSender<AsyncAction>,
        async_action_result_rx: UnboundedReceiver<AsyncActionResult>,
        player_state: PlayerState,
        podcasts_model: PodcastsModel
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        MyEguiApp {
            tx,
            rx,
            async_action_tx,
            async_action_result_rx,
            player_state: PlayerState::Paused,
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
        };

        match self.async_action_result_rx.try_recv() {
            Ok(AsyncActionResult::PodcastsUpdate(podcasts)) => {
                self.podcasts_model.podcasts = Some(podcasts);
            }
            Err(_) => {}
        };

        egui::SidePanel::left("podcasts_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(150.0..=300.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Podcasts");
                        if ui.add(egui::Button::new("+")).on_hover_text("Add podcast").clicked() {
                            self.show_add_podcast = true;
                        }
                    });
                });
                egui::ScrollArea::vertical().max_width(600.0).show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                        |ui| {
                            if let Some(podcasts) = &self.podcasts_model.podcasts {
                                for p in podcasts {
                                    ui.add(egui::Link::new(&p.title));
                                }
                            } else {
                                error!("refreshing");
                                self.async_action_tx.send(AsyncAction::GetPodcasts);
                            }
                        },
                    );
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
            //     ui.add(egui::TextEdit::singleline(&mut self.src_url).hint_text("Stream url")).highlight();
            // });
            // if ui.add(egui::Button::new("Add +")).clicked() {
            //     self.show_add_podcast = true;
            // }
            ui.with_layout(
                egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical_centered(|ui| {
                            if self.player_state == PlayerState::Paused {
                                if ui.add(egui::Button::new("Play")).clicked() {
                                    // self.tx.try_send(PlayerAction::Open(self.src_url.clone()));
                                    self.tx.send(PlayerAction::Play);
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
        });

        if self.show_add_podcast {
            egui::Window::new("Add podcast")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.new_podcast.link).hint_text("Podcast url"));
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.new_podcast.title).hint_text("Podcast title"));
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.new_podcast.description).hint_text("Podcast description"));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            if ui.add(egui::Button::new("Close")).clicked() {
                                self.podcasts_model.new_podcast.link = String::new();
                                self.podcasts_model.new_podcast.title = String::new();
                                self.podcasts_model.new_podcast.description = String::new();
                                self.show_add_podcast = false;
                            }
                            if ui.add(egui::Button::new("Add")).clicked() {
                                self.async_action_tx.send(AsyncAction::AddPodcast(self.podcasts_model.new_podcast.clone()))
                                    .unwrap_or_else(|e| error!("{:?}", e.to_string()));
                                self.async_action_tx.send(AsyncAction::GetPodcasts)
                                    .unwrap_or_else(|e| error!("{:?}", e.to_string()));
                                self.show_add_podcast = false;
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
