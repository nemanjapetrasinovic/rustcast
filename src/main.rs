#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod data_provider;
mod entity;
mod podcasts_model;

use std::{io::BufReader, str::FromStr};

use data_provider::DataProvider;
use eframe::egui::{self, TextStyle};
use egui_extras::{Column, TableBuilder};
use entity::{episode, podcast};
use log::error;
use podcasts_model::{Podcast, PodcastsModel};
use rss::Channel;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use url2audio::Player;
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
    AddPodcast(String, String, String),
    GetPodcasts,
    GetEpisodes(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum AsyncActionResult {
    PodcastsUpdate(Option<Vec<podcast::Model>>),
    EpisodesUpdate(Option<Vec<rss::Item>>),
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let (player_action_tx, mut player_action_rx) = unbounded_channel::<PlayerAction>();
    let (player_state_tx, player_state_rx) = unbounded_channel::<PlayerState>();

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
                Some(AsyncAction::AddPodcast(title, link, description)) => {
                    data_provider.add_podcast(title, link, description)
                        .await
                        .map_err(|e| error!("{}", e));
                },
                Some(AsyncAction::GetPodcasts) => {
                    match data_provider.get_podcasts().await {
                        Ok(res) => {
                            async_action_result_tx.send(AsyncActionResult::PodcastsUpdate(Some(res)));
                        }
                        Err(_) => {
                            async_action_result_tx.send(AsyncActionResult::PodcastsUpdate(None));
                        }
                    }
                }
                Some(AsyncAction::GetEpisodes(link)) => {
                    let start = instant::Instant::now();

                    let mut res = None;
                    if let Ok(episodes) = ureq::get(&link).call() {
                        if let Ok(episodes) = episodes.into_string() {
                            if let Ok(channel) = Channel::from_str(&episodes) {
                                res = Some(channel.items().to_vec());
                            }
                        }
                    }

                    async_action_result_tx.send(AsyncActionResult::EpisodesUpdate(res));

                    let diff = start.elapsed().as_millis();
                    error!("load_all_people: duration: {}", diff);
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
            match player_action_rx.recv().await {
                Some(PlayerAction::Open(src)) => {
                    player_wrapper.inner_player.open(&src);
                    player_state_tx.send(PlayerState::Playing);
                }
                Some(PlayerAction::Play) => {
                    player_wrapper.inner_player.play();
                    player_state_tx.send(PlayerState::Playing);
                }
                Some(PlayerAction::Pause) => {
                    player_wrapper.inner_player.pause();
                    player_state_tx.send(PlayerState::Paused);
                }
                None => {
                    break;
                }
            }
        }
    });

    // puffin::set_scopes_on(true); // tell puffin to collect data

    // match puffin_http::Server::new("127.0.0.1:8585") {
    //     Ok(puffin_server) => {
    //         eprintln!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");
    //
    //         std::process::Command::new("puffin_viewer")
    //             .arg("--url")
    //             .arg("127.0.0.1:8585")
    //             .spawn()
    //             .ok();
    //
    //         // We can store the server if we want, but in this case we just want
    //         // it to keep running. Dropping it closes the server, so let's not drop it!
    //         #[allow(clippy::mem_forget)]
    //         std::mem::forget(puffin_server);
    //     }
    //     Err(err) => {
    //         eprintln!("Failed to start puffin server: {err}");
    //     }
    // };
    //
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rustcast",
        native_options,
        Box::new(move |cc| Box::new(MyEguiApp::new(cc, player_action_tx, player_state_rx, async_action_tx, async_action_result_rx, PlayerState::Open, PodcastsModel::new()))),
    )
    .unwrap_or_else(|e| error!("An error occured {}", e));

    player_thread.await.unwrap();
    async_action_thread.await.unwrap();
}

struct MyEguiApp {
    player_action_tx: UnboundedSender<PlayerAction>,
    player_action_rx: UnboundedReceiver<PlayerState>,
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
        player_action_tx: UnboundedSender<PlayerAction>,
        player_action_rx: UnboundedReceiver<PlayerState>,
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
            player_action_tx,
            player_action_rx,
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
        // puffin::profile_function!();
        // puffin::GlobalProfiler::lock().new_frame();

        ctx.request_repaint();
        match self.player_action_rx.try_recv() {
            Ok(player_state) => self.player_state = player_state,
            Err(_) => {}
        };

        match self.async_action_result_rx.try_recv() {
            Ok(AsyncActionResult::PodcastsUpdate(podcasts)) => {
                self.podcasts_model.podcasts = podcasts;
            }
            Ok(AsyncActionResult::EpisodesUpdate(episodes)) => {
                self.podcasts_model.episodes = episodes;
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
                                    if let Some(title) = &p.title {
                                        if ui.add(egui::Link::new(title)).clicked() {
                                            // puffin::profile_scope!("table render");
                                            self.async_action_tx.send(AsyncAction::GetEpisodes(p.link.clone().unwrap()));
                                        }
                                    }
                                }
                            } else {
                                error!("refreshing");
                                self.async_action_tx.send(AsyncAction::GetPodcasts);
                            }
                        },
                    );
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(70.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Bottom Panel");
                });
                ui.vertical_centered(|ui| {
                    ui.vertical_centered(|ui| {
                        if self.player_state == PlayerState::Paused {
                            if ui.add(egui::Button::new("Play")).clicked() {
                                // self.tx.try_send(PlayerAction::Open(self.src_url.clone()));
                                self.player_action_tx.send(PlayerAction::Play);
                            }
                        }
                        if self.player_state == PlayerState::Playing || self.player_state == PlayerState::Open {
                            if ui.add(egui::Button::new("Pause")).clicked() {
                                self.player_action_tx.send(PlayerAction::Pause);
                            }
                        }
                    })
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // puffin::profile_scope!("Table update");
            ui.vertical(|ui| {
                ui.heading("Episodes");
            });
            if let Some(episodes) = &self.podcasts_model.episodes {
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    let text_height = egui::TextStyle::Body
                        .resolve(ui.style())
                        .size
                        .max(ui.spacing().interact_size.y);

                    let ah = ui.available_height();
                    let table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(false)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::remainder())
                        .min_scrolled_height(0.0)
                        .max_scroll_height(ah);

                    table
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("Ep");
                            });
                            header.col(|ui| {
                                ui.strong("Action");
                            });
                            header.col(|ui| {
                                ui.strong("Title");
                            });
                        })
                        .body(|mut body| {
                            body.rows(text_height, episodes.len(), |mut row| {
                                let row_index = row.index();

                                row.col(|ui| {
                                    ui.label(row_index.to_string());
                                });
                                row.col(|ui| {
                                    if ui.add(egui::Button::new("Play")).clicked() {
                                        self.player_action_tx.send(PlayerAction::Open(episodes[row_index].enclosure.clone().unwrap().url));
                                        error!("{:?}", episodes[row_index].link.clone().unwrap());
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(episodes[row_index].title.clone().unwrap());
                                });
                            });
                        });
                });
            }
        });


        if self.show_add_podcast {
            egui::Window::new("Add podcast")
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.podcast_dialog.link).hint_text("Podcast url"));
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.podcast_dialog.title).hint_text("Podcast title"));
                        ui.add(egui::TextEdit::singleline(&mut self.podcasts_model.podcast_dialog.description).hint_text("Podcast description"));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            if ui.add(egui::Button::new("Close")).clicked() {
                                self.podcasts_model.podcast_dialog.link = String::new();
                                self.podcasts_model.podcast_dialog.title = String::new();
                                self.podcasts_model.podcast_dialog.description = String::new();
                                self.show_add_podcast = false;
                            }
                            if ui.add(egui::Button::new("Add")).clicked() {
                                self.async_action_tx.send(AsyncAction::AddPodcast(
                                    self.podcasts_model.podcast_dialog.title.clone(),
                                    self.podcasts_model.podcast_dialog.link.clone(),
                                    self.podcasts_model.podcast_dialog.description.clone()
                                ))
                                    .unwrap_or_else(|e| error!("{:?}", e.to_string()));

                                self.async_action_tx.send(AsyncAction::GetPodcasts)
                                    .unwrap_or_else(|e| error!("{:?}", e.to_string()));
                                
                                self.podcasts_model.podcast_dialog.link = String::new();
                                self.podcasts_model.podcast_dialog.title = String::new();
                                self.podcasts_model.podcast_dialog.description = String::new();
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
