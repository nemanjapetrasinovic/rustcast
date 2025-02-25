#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod data_provider;
mod entity;
mod podcasts_model;
mod widgets;
mod utils;
mod traits;

use std::str::FromStr;
use data_provider::DataProvider;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use entity::{episode, podcast};
use log::error;
use podcasts_model::PodcastsModel;
use rss::Channel;
use sea_orm::{Database, DatabaseConnection};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use url2audio::Player;
use widgets::timeline::Timeline;

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
    pub seek_position: f64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AsyncAction {
    AddPodcast(String, String, String),
    GetPodcasts,
    GetEpisodes(String, i32),
    SaveEpisodeState(f64, i32, String),
    LoadEpisodeState(String)
}

#[derive(Debug, PartialEq, Clone)]
pub enum AsyncActionResult {
    PodcastsUpdate(Option<Vec<podcast::Model>>),
    EpisodesUpdate(Option<Vec<episode::Model>>),
    AddPodcastResult(Option<String>),
    UniversalResult(Option<String>),
    EpisodeStateUpdate(f64),
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let (async_action_tx, mut async_action_rx) = unbounded_channel::<AsyncAction>();
    let (async_action_result_tx, async_action_result_rx) = unbounded_channel::<AsyncActionResult>();

    let async_action_thread = tokio::spawn(async move {
        let home = std::env::var("HOME").unwrap();
        let connection = std::env::var("DATABASE_URL")
            .unwrap_or(format!("sqlite://{}/.rustcast.db?mode=rwc", home));
        let db: DatabaseConnection = Database::connect(connection).await.unwrap();

        let data_provider = DataProvider::new(db);
        data_provider.get_podcasts().await.unwrap();

        loop {
            match async_action_rx.recv().await {
                Some(AsyncAction::AddPodcast(title, link, description)) => {
                    match ureq::get(&link).call() {
                        Ok(episodes) => {
                            match episodes.into_string() {
                                Ok(episodes) => {
                                    if let Err(e) = Channel::from_str(&episodes) {
                                        let _ = async_action_result_tx.send(AsyncActionResult::AddPodcastResult(Some(e.to_string())));
                                    } else if let Err(e) = data_provider
                                        .add_podcast(title, link, description)
                                        .await {
                                        let _ = async_action_result_tx.send(AsyncActionResult::AddPodcastResult(Some(e.to_string())));
                                    }
                                },
                                Err(e) => {
                                    let _ = async_action_result_tx.send(AsyncActionResult::AddPodcastResult(Some(e.to_string())));
                                }
                            }
                        },
                        Err(e) => {
                            let _ = async_action_result_tx.send(AsyncActionResult::AddPodcastResult(Some(e.to_string())));
                        }
                    }
                }
                Some(AsyncAction::GetPodcasts) => match data_provider.get_podcasts().await {
                    Ok(res) => {
                        let _ = async_action_result_tx.send(AsyncActionResult::PodcastsUpdate(Some(res)));
                    }
                    Err(_) => {
                        let _ = async_action_result_tx.send(AsyncActionResult::PodcastsUpdate(None));
                    }
                },
                Some(AsyncAction::GetEpisodes(link, podcast_id)) => {
                    if let Ok(episodes) = ureq::get(&link).call() {
                        if let Ok(episodes) = episodes.into_string() {
                            if let Ok(channel) = Channel::from_str(&episodes) {
                                if let Err(e) = data_provider.delete_episodes_by_podcast_id(podcast_id).await {
                                    let _ = async_action_result_tx.send(AsyncActionResult::UniversalResult(Some(e.to_string())));
                                }
                                if let Err(e) = data_provider.add_episodes(channel.items().to_vec(), podcast_id).await {
                                    let _ = async_action_result_tx.send(AsyncActionResult::UniversalResult(Some(e.to_string())));
                                }
                            }
                        }
                    }

                    let res = Some(data_provider.get_all_episodes(podcast_id).await.unwrap());
                    let _ = async_action_result_tx.send(AsyncActionResult::EpisodesUpdate(res));
                }
                Some(AsyncAction::SaveEpisodeState(progress, podcast_id, link)) => {
                    if let Err(e) = data_provider.upsert_episode_state(progress, podcast_id, &link).await {
                        let _ = async_action_result_tx.send(AsyncActionResult::UniversalResult(Some(e.to_string())));
                    }
                }
                Some(AsyncAction::LoadEpisodeState(link)) => {
                    match data_provider.get_episode_state(&link).await {
                        Ok(res) => {
                            if res.is_some() {
                                let _ = async_action_result_tx.send(AsyncActionResult::EpisodeStateUpdate(res.unwrap().time));
                            } else {
                                let _ = async_action_result_tx.send(AsyncActionResult::EpisodeStateUpdate(0.0));
                            }
                        }
                        Err(e) => {
                            let _ = async_action_result_tx.send(AsyncActionResult::UniversalResult(Some(e.to_string())));
                        }
                    }
                }
                None => break,
            }
        }
    });

    let player = Player::new();
    let player_wrapper = PlayerWrapper {
        inner_player: player,
        player_state: PlayerState::Paused,
        seek_position: 0.0
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Rustcast",
        native_options,
        Box::new(move |cc| {
            Box::new(MyEguiApp::new(
                cc,
                player_wrapper,
                async_action_tx,
                async_action_result_rx,
                PlayerState::Open,
                PodcastsModel::new(),
            ))
        }),
    )
    .unwrap_or_else(|e| error!("An error occured {}", e));

    async_action_thread.await.unwrap();
}

struct MyEguiApp {
    player_wrapper: PlayerWrapper,
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
        player_wrapper: PlayerWrapper,
        async_action_tx: UnboundedSender<AsyncAction>,
        async_action_result_rx: UnboundedReceiver<AsyncActionResult>,
        player_state: PlayerState,
        podcasts_model: PodcastsModel,
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        MyEguiApp {
            player_wrapper,
            async_action_tx,
            async_action_result_rx,
            player_state: PlayerState::Paused,
            show_add_podcast: false,
            podcasts_model,
            show_error: false,
            error: String::new(),
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.async_action_result_rx.try_recv() {
            Ok(AsyncActionResult::PodcastsUpdate(podcasts)) => {
                self.podcasts_model.podcasts = podcasts;
            }
            Ok(AsyncActionResult::EpisodesUpdate(episodes)) => {
                self.podcasts_model.episodes = episodes;
            },
            Ok(AsyncActionResult::AddPodcastResult(res)) => {
                if res.is_some() {
                    self.error = res.unwrap();
                    self.show_error = true;
                }
            }
            Ok(AsyncActionResult::UniversalResult(res)) => {
                if res.is_some() {
                    self.error = res.unwrap();
                    self.show_error = true;
                }
            }
            Ok(AsyncActionResult::EpisodeStateUpdate(res)) => {
                self.player_wrapper.inner_player.open(self.podcasts_model.current_episode.as_ref().unwrap().link.as_ref().unwrap());
                self.player_wrapper.inner_player.seek(res);
                self.player_wrapper.inner_player.play();
                self.player_wrapper.player_state = PlayerState::Playing;
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
                        if ui
                            .add(egui::Button::new("+"))
                            .on_hover_text("Add podcast")
                            .clicked()
                        {
                            self.show_add_podcast = true;
                        }
                    });
                });
                egui::ScrollArea::vertical()
                    .max_width(600.0)
                    .show(ui, |ui| {
                        ui.with_layout(
                            egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                            |ui| {
                                if let Some(podcasts) = &self.podcasts_model.podcasts {
                                    for p in podcasts {
                                        if let Some(title) = &p.title {
                                            if ui.add(egui::Link::new(title)).clicked() {
                                                self.podcasts_model.current_podcast = podcasts_model::Podcast {
                                                    id: Some(p.id),
                                                    title: p.title.clone().unwrap(),
                                                    link: p.link.clone().unwrap(),
                                                    description: p.description.clone().unwrap()
                                                };


                                                let _ = self.async_action_tx.send(
                                                    AsyncAction::GetEpisodes(
                                                        p.link.clone().unwrap(),
                                                        self.podcasts_model.current_podcast.id.unwrap()
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    let _ = self.async_action_tx.send(AsyncAction::GetPodcasts);
                                }
                            },
                        );
                    });
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(70.0)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);

                    if self.player_wrapper.player_state == PlayerState::Paused && ui.add(egui::Button::new("▶")).clicked() {
                            self.player_wrapper.inner_player.play();
                            self.player_wrapper.player_state = PlayerState::Playing;
                        }

                    if (self.player_wrapper.player_state == PlayerState::Playing
                        || self.player_state == PlayerState::Open)
                        && ui.add(egui::Button::new("⏸")).clicked() {
                            self.player_wrapper.inner_player.pause();
                            self.player_wrapper.player_state = PlayerState::Paused;

                            self.async_action_tx.send(AsyncAction::SaveEpisodeState(
                                self.player_wrapper.inner_player.current_position(),
                                self.podcasts_model.current_podcast.id.unwrap(),
                                self.podcasts_model.current_episode.clone().unwrap().link.unwrap())
                            ).unwrap();
                    }

                    ui.add_space(5.0);

                    let timeline_add = ui.add(&mut Timeline::new(
                        self.player_wrapper.inner_player.current_position(),
                        self.player_wrapper.inner_player.duration(),
                        &mut self.player_wrapper.seek_position
                    ));
                    if timeline_add.clicked() || timeline_add.drag_stopped() {
                        self.player_wrapper.inner_player.seek(self.player_wrapper.seek_position);
                    }

                    if let Some(current_episode) = &self.podcasts_model.current_episode {
                        ui.label(current_episode.title.clone().unwrap());
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
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
                        .body(|body| {
                            body.rows(text_height + 5.0, episodes.len(), |mut row| {
                                let row_index = row.index();
                                row.col(|ui| {
                                    ui.label(row_index.to_string());
                                });
                                row.col(|ui| {
                                    if self.podcasts_model.current_episode == Some(episodes[row_index].clone())
                                        && self.player_wrapper.player_state == PlayerState::Playing {
                                        if ui.add(egui::Button::new("⏸").min_size(eframe::egui::Vec2::new(15.0, 15.0)).fill(eframe::egui::Color32::from_rgb(0, 155, 255))).clicked() {
                                            self.player_wrapper.inner_player.pause();
                                            self.player_wrapper.player_state = PlayerState::Paused;

                                            self.async_action_tx.send(AsyncAction::SaveEpisodeState(
                                                self.player_wrapper.inner_player.current_position(),
                                                self.podcasts_model.current_podcast.id.unwrap(),
                                                self.podcasts_model.current_episode.clone().unwrap().link.unwrap())
                                            ).unwrap();
                                        }
                                    } else if ui.add(egui::Button::new("▶").min_size(eframe::egui::Vec2::new(15.0, 15.0))).clicked() {
                                        if self.podcasts_model.current_episode.is_some() {
                                            self.async_action_tx.send(AsyncAction::SaveEpisodeState(
                                                self.player_wrapper.inner_player.current_position(),
                                                self.podcasts_model.current_podcast.id.unwrap(),
                                                self.podcasts_model.current_episode.clone().unwrap().link.unwrap())
                                            ).unwrap();
                                        }

                                        self.async_action_tx
                                            .send(AsyncAction::LoadEpisodeState(episodes[row_index].clone().link.unwrap()))
                                            .unwrap_or_else(|e| error!("{:?}", e.to_string()));
                                        self.podcasts_model.current_episode = Some(episodes[row_index].clone());
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
                    ui.with_layout(
                        egui::Layout::top_down_justified(egui::Align::Center),
                        |ui| {
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut self.podcasts_model.podcast_dialog.link,
                                )
                                .hint_text("Podcast url"),
                            );
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut self.podcasts_model.podcast_dialog.title,
                                )
                                .hint_text("Podcast title"),
                            );
                            ui.add(
                                egui::TextEdit::singleline(
                                    &mut self.podcasts_model.podcast_dialog.description,
                                )
                                .hint_text("Podcast description"),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                if ui.add(egui::Button::new("Close")).clicked() {
                                    self.podcasts_model.podcast_dialog.link = String::new();
                                    self.podcasts_model.podcast_dialog.title = String::new();
                                    self.podcasts_model.podcast_dialog.description = String::new();
                                    self.show_add_podcast = false;
                                }
                                if ui.add(egui::Button::new("Add")).clicked() {
                                    self.async_action_tx
                                        .send(AsyncAction::AddPodcast(
                                            self.podcasts_model.podcast_dialog.title.clone(),
                                            self.podcasts_model.podcast_dialog.link.clone(),
                                            self.podcasts_model.podcast_dialog.description.clone(),
                                        ))
                                        .unwrap_or_else(|e| error!("{:?}", e.to_string()));

                                    self.async_action_tx
                                        .send(AsyncAction::GetPodcasts)
                                        .unwrap_or_else(|e| error!("{:?}", e.to_string()));

                                    self.podcasts_model.podcast_dialog.link = String::new();
                                    self.podcasts_model.podcast_dialog.title = String::new();
                                    self.podcasts_model.podcast_dialog.description = String::new();
                                    self.show_add_podcast = false;
                                }
                            });
                        },
                    );
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

        ctx.request_repaint();
    }
}
