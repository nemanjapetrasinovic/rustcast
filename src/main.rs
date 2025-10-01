#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod data_provider;
mod entity;
mod error;
mod podcasts_model;
mod widgets;
mod utils;
mod traits;

use migrations;

use data_provider::DataProvider;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use entity::{episode, podcast};
use error::{RustcastError, RustcastResult};
use log::{error, warn, info};
use podcasts_model::PodcastsModel;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::prelude::*;
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
    LoadEpisodeState(String),
    GetAllEpisodeStates(i32)
}

#[derive(Debug, PartialEq, Clone)]
pub enum AsyncActionResult {
    PodcastsUpdate(Option<Vec<podcast::Model>>),
    EpisodesUpdate(Option<Vec<episode::Model>>),
    AddPodcastResult(Option<String>),
    UniversalResult(Option<String>),
    EpisodeStateUpdate(f64),
    AllEpisodeStatesUpdate(Option<std::collections::HashMap<String, f64>>),
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
        let db: DatabaseConnection = match Database::connect(&connection).await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to connect to database: {}", e);
                panic!("Database connection failed - application cannot continue");
            }
        };

        // Run migrations automatically
        info!("Running database migrations...");
        if let Err(e) = migrations::Migrator::up(&db, None).await {
            error!("Failed to run migrations: {}", e);
            panic!("Database migration failed - application cannot continue");
        }
        info!("Database migrations completed successfully");

        let data_provider = DataProvider::new(db);
        if let Err(e) = data_provider.get_podcasts().await {
            error!("Failed to initialize podcasts: {}", e);
        }

        loop {
            match async_action_rx.recv().await {
                Some(AsyncAction::AddPodcast(title, link, description)) => {
                    let title_clone = title.clone();
                    match handle_add_podcast(&data_provider, title, link, description).await {
                        Ok(_) => {
                            info!("Successfully added podcast: {}", title_clone);
                            // Send success signal or refresh podcasts
                            let _ = async_action_result_tx.send(AsyncActionResult::AddPodcastResult(None));
                        }
                        Err(e) => {
                            error!("Failed to add podcast '{}': {}", title_clone, e);
                            let _ = async_action_result_tx.send(AsyncActionResult::AddPodcastResult(
                                Some(e.user_friendly_message())
                            ));
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
                    match handle_get_episodes(&data_provider, &link, podcast_id).await {
                        Ok(episodes) => {
                            info!("Successfully loaded {} episodes for podcast {}", episodes.len(), podcast_id);
                            let _ = async_action_result_tx.send(AsyncActionResult::EpisodesUpdate(Some(episodes)));
                        }
                        Err(e) => {
                            error!("Failed to load episodes for podcast {}: {}", podcast_id, e);
                            let _ = async_action_result_tx.send(AsyncActionResult::UniversalResult(
                                Some(e.user_friendly_message())
                            ));
                            // Still try to load cached episodes from database
                            match data_provider.get_all_episodes(podcast_id).await {
                                Ok(cached_episodes) => {
                                    warn!("Using cached episodes for podcast {}", podcast_id);
                                    let _ = async_action_result_tx.send(AsyncActionResult::EpisodesUpdate(Some(cached_episodes)));
                                }
                                Err(db_err) => {
                                    error!("Failed to load cached episodes: {}", db_err);
                                    let _ = async_action_result_tx.send(AsyncActionResult::EpisodesUpdate(None));
                                }
                            }
                        }
                    }
                }
                Some(AsyncAction::SaveEpisodeState(progress, podcast_id, link)) => {
                    match data_provider.upsert_episode_state(progress, podcast_id, &link).await {
                        Ok(_) => {
                            info!("Saved episode state: progress={:.1}s, podcast_id={}", progress, podcast_id);
                        }
                        Err(e) => {
                            error!("Failed to save episode state: {}", e);
                            let _ = async_action_result_tx.send(AsyncActionResult::UniversalResult(Some(e.to_string())));
                        }
                    }
                }
                Some(AsyncAction::LoadEpisodeState(link)) => {
                    match data_provider.get_episode_state(&link).await {
                        Ok(res) => {
                            if let Some(state) = res {
                                info!("Loaded episode state: time={:.1}s for link={}", state.time, link);
                                let _ = async_action_result_tx.send(AsyncActionResult::EpisodeStateUpdate(state.time));
                            } else {
                                info!("No saved state found for episode: {}", link);
                                let _ = async_action_result_tx.send(AsyncActionResult::EpisodeStateUpdate(0.0));
                            }
                        }
                        Err(e) => {
                            error!("Failed to load episode state for {}: {}", link, e);
                            let _ = async_action_result_tx.send(AsyncActionResult::UniversalResult(Some(e.to_string())));
                        }
                    }
                }
                Some(AsyncAction::GetAllEpisodeStates(podcast_id)) => {
                    match data_provider.get_all_episode_states(podcast_id).await {
                        Ok(states) => {
                            info!("Loaded {} episode states for podcast {}", states.len(), podcast_id);
                            let _ = async_action_result_tx.send(AsyncActionResult::AllEpisodeStatesUpdate(Some(states)));
                        }
                        Err(e) => {
                            error!("Failed to load episode states for podcast {}: {}", podcast_id, e);
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
    last_update_time: std::time::Instant,
}

impl MyEguiApp {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        player_wrapper: PlayerWrapper,
        async_action_tx: UnboundedSender<AsyncAction>,
        async_action_result_rx: UnboundedReceiver<AsyncActionResult>,
        _player_state: PlayerState,
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
            last_update_time: std::time::Instant::now(),
        }
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Periodic auto-save for playing episodes to preserve state
        let now = std::time::Instant::now();
        if self.player_wrapper.player_state == PlayerState::Playing {
            // Auto-save episode state every 5 seconds while playing
            if now.duration_since(self.last_update_time).as_secs() >= 5 {
                if let (Some(podcast_id), Some(episode)) = (
                    self.podcasts_model.current_podcast.id,
                    &self.podcasts_model.current_episode
                ) {
                    if let Some(episode_link) = &episode.link {
                        let current_position = self.player_wrapper.inner_player.current_position();

                        // Update local state for immediate UI feedback
                        self.podcasts_model.episode_states.insert(episode_link.clone(), current_position);

                        // Auto-save to database in background
                        if let Err(e) = self.async_action_tx.send(AsyncAction::SaveEpisodeState(
                            current_position,
                            podcast_id,
                            episode_link.clone()
                        )) {
                            error!("Failed to auto-save episode state: {}", e);
                        } else {
                            info!("Auto-saved episode state: {:.1}s for '{}'",
                                current_position,
                                episode.title.as_deref().unwrap_or("Unknown")
                            );
                        }

                        self.last_update_time = now;
                    }
                }
            }
        }

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
                if let Some(episode) = &self.podcasts_model.current_episode {
                    if let Some(link) = &episode.link {
                        // Always open the episode to ensure it's properly loaded
                        self.player_wrapper.inner_player.open(link);

                        // Seek to the saved position (or 0.0 if starting fresh)
                        self.player_wrapper.inner_player.seek(res);
                        self.player_wrapper.inner_player.play();
                        self.player_wrapper.player_state = PlayerState::Playing;

                        if res > 0.0 {
                            info!("Resumed episode '{}' from {:.1}s",
                                episode.title.as_deref().unwrap_or("Unknown"), res);
                        } else {
                            info!("Started episode '{}' from beginning",
                                episode.title.as_deref().unwrap_or("Unknown"));
                        }
                    } else {
                        error!("Episode link is missing");
                        self.error = "Episode link is missing or invalid.".to_string();
                        self.show_error = true;
                    }
                } else {
                    error!("No current episode selected");
                    self.error = "No episode selected for playback.".to_string();
                    self.show_error = true;
                }
            }
            Ok(AsyncActionResult::AllEpisodeStatesUpdate(states)) => {
                if let Some(states) = states {
                    self.podcasts_model.episode_states = states;
                }
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
                                                if let (Some(link), Some(description)) = (&p.link, &p.description) {
                                                    self.podcasts_model.current_podcast = podcasts_model::Podcast {
                                                        id: Some(p.id),
                                                        title: title.clone(),
                                                        link: link.clone(),
                                                        description: description.clone()
                                                    };

                                                    let _ = self.async_action_tx.send(
                                                        AsyncAction::GetEpisodes(link.clone(), p.id),
                                                    );
                                                    let _ = self.async_action_tx.send(
                                                        AsyncAction::GetAllEpisodeStates(p.id),
                                                    );
                                                } else {
                                                    error!("Podcast data is incomplete: id={}, title={:?}, link={:?}", p.id, p.title, p.link);
                                                    self.error = "This podcast has incomplete data and cannot be loaded.".to_string();
                                                    self.show_error = true;
                                                }
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

                            if let (Some(podcast_id), Some(episode)) = (
                                self.podcasts_model.current_podcast.id,
                                &self.podcasts_model.current_episode
                            ) {
                                if let Some(episode_link) = &episode.link {
                                    let current_position = self.player_wrapper.inner_player.current_position();

                                    // Update local state immediately for real-time display
                                    self.podcasts_model.episode_states.insert(episode_link.clone(), current_position);

                                    // Send to async handler to save to database
                                    if let Err(e) = self.async_action_tx.send(AsyncAction::SaveEpisodeState(
                                        current_position,
                                        podcast_id,
                                        episode_link.clone()
                                    )) {
                                        error!("Failed to save episode state: {}", e);
                                    }
                                } else {
                                    warn!("Cannot save episode state: episode link is missing");
                                }
                            } else {
                                warn!("Cannot save episode state: podcast or episode not properly selected");
                            }
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
                        ui.label(current_episode.title.as_deref().unwrap_or("Unknown Episode"));
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
                                ui.strong("Paused at");
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

                                            if let (Some(podcast_id), Some(episode)) = (
                                                self.podcasts_model.current_podcast.id,
                                                &self.podcasts_model.current_episode
                                            ) {
                                                if let Some(episode_link) = &episode.link {
                                                    let current_position = self.player_wrapper.inner_player.current_position();

                                                    // Update local state immediately for real-time display
                                                    self.podcasts_model.episode_states.insert(episode_link.clone(), current_position);

                                                    // Send to async handler to save to database
                                                    if let Err(e) = self.async_action_tx.send(AsyncAction::SaveEpisodeState(
                                                        current_position,
                                                        podcast_id,
                                                        episode_link.clone()
                                                    )) {
                                                        error!("Failed to save episode state: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                    } else if ui.add(egui::Button::new("▶").min_size(eframe::egui::Vec2::new(15.0, 15.0))).clicked() {
                                        let selected_episode = &episodes[row_index];
                                        let is_same_episode = self.podcasts_model.current_episode.as_ref()
                                            .map(|current| current.link == selected_episode.link)
                                            .unwrap_or(false);

                                        if is_same_episode && self.player_wrapper.player_state == PlayerState::Paused {
                                            // Same episode - just resume playback from current position
                                            self.player_wrapper.inner_player.play();
                                            self.player_wrapper.player_state = PlayerState::Playing;
                                            info!("Resumed playback of: {}", selected_episode.title.as_deref().unwrap_or("Unknown"));
                                        } else {
                                            // Different episode or no current episode - save current state and load new episode
                                            if self.podcasts_model.current_episode.is_some() {
                                                if let (Some(podcast_id), Some(episode)) = (
                                                    self.podcasts_model.current_podcast.id,
                                                    &self.podcasts_model.current_episode
                                                ) {
                                                    if let Some(episode_link) = &episode.link {
                                                        let current_position = self.player_wrapper.inner_player.current_position();

                                                        // Update local state immediately for real-time display
                                                        self.podcasts_model.episode_states.insert(episode_link.clone(), current_position);

                                                        // Send to async handler to save to database
                                                        if let Err(e) = self.async_action_tx.send(AsyncAction::SaveEpisodeState(
                                                            current_position,
                                                            podcast_id,
                                                            episode_link.clone()
                                                        )) {
                                                            error!("Failed to save episode state: {}", e);
                                                        }
                                                    }
                                                }
                                            }

                                            // Set new current episode
                                            self.podcasts_model.current_episode = Some(selected_episode.clone());

                                            // Load the episode state and start playback
                                            if let Some(episode_link) = &selected_episode.link {
                                                if let Err(e) = self.async_action_tx.send(AsyncAction::LoadEpisodeState(episode_link.clone())) {
                                                    error!("Failed to load episode state: {}", e);
                                                }
                                            } else {
                                                error!("Episode link is missing for episode at index {}", row_index);
                                            }
                                        }
                                    }
                                });
                                row.col(|ui| {
                                    if let Some(episode_link) = &episodes[row_index].link {
                                        // Check if this is the currently playing episode
                                        let is_current_episode = self.podcasts_model.current_episode
                                            .as_ref()
                                            .map(|ep| ep.link.as_ref() == Some(episode_link))
                                            .unwrap_or(false);

                                        if is_current_episode && self.player_wrapper.player_state == PlayerState::Playing {
                                            // Show real-time position and status for playing episode
                                            let current_time = self.player_wrapper.inner_player.current_position();
                                            ui.colored_label(
                                                egui::Color32::from_rgb(0, 155, 255),
                                                format!("Playing: {}", format_time(current_time))
                                            );
                                        } else if is_current_episode && self.player_wrapper.player_state == PlayerState::Paused {
                                            // Show real-time position for paused current episode
                                            let current_time = self.player_wrapper.inner_player.current_position();
                                            ui.colored_label(
                                                egui::Color32::from_rgb(255, 165, 0),
                                                format_time(current_time)
                                            );
                                        } else {
                                            // Show saved state for other episodes
                                            let pause_time = self.podcasts_model.episode_states.get(episode_link)
                                                .copied()
                                                .unwrap_or(0.0);
                                            if pause_time > 0.0 {
                                                ui.label(format_time(pause_time));
                                            } else {
                                                ui.label("Not started");
                                            }
                                        }
                                    } else {
                                        ui.label("No data");
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(episodes[row_index].title.as_deref().unwrap_or("Unknown Episode"));
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

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save current episode state before closing
        if self.player_wrapper.player_state == PlayerState::Playing ||
           self.player_wrapper.player_state == PlayerState::Paused {
            if let (Some(podcast_id), Some(episode)) = (
                self.podcasts_model.current_podcast.id,
                &self.podcasts_model.current_episode
            ) {
                if let Some(episode_link) = &episode.link {
                    let current_position = self.player_wrapper.inner_player.current_position();

                    // Final save before app closes
                    if let Err(e) = self.async_action_tx.send(AsyncAction::SaveEpisodeState(
                        current_position,
                        podcast_id,
                        episode_link.clone()
                    )) {
                        error!("Failed to save final episode state: {}", e);
                    } else {
                        info!("Final save on app close: {:.1}s for '{}'",
                            current_position,
                            episode.title.as_deref().unwrap_or("Unknown")
                        );
                    }
                }
            }
        }
    }
}

async fn handle_add_podcast(
    data_provider: &DataProvider,
    title: String,
    link: String,
    description: String,
) -> RustcastResult<()> {
    // Validate input data
    utils::validate_podcast_data(&title, &link, &description)?;

    // Validate the RSS feed
    let response = utils::safe_network_request(&link)?;
    let content = response.into_string()
        .map_err(|e| RustcastError::Network(error::NetworkError::InvalidResponse(e.to_string())))?;

    let _channel = utils::safe_rss_parse(&content)?;

    // If we get here, the feed is valid, so add it to the database
    data_provider.add_podcast(title, link, description).await
        .map_err(RustcastError::from)?;

    Ok(())
}

async fn handle_get_episodes(
    data_provider: &DataProvider,
    link: &str,
    podcast_id: i32,
) -> RustcastResult<Vec<episode::Model>> {
    // Fetch and parse RSS feed
    let response = utils::safe_network_request(link)?;
    let content = response.into_string()
        .map_err(|e| RustcastError::Network(error::NetworkError::InvalidResponse(e.to_string())))?;

    let channel = utils::safe_rss_parse(&content)?;

    // Clear old episodes and add new ones
    data_provider.delete_episodes_by_podcast_id(podcast_id).await
        .map_err(RustcastError::from)?;

    data_provider.add_episodes(channel.items().to_vec(), podcast_id).await?;

    // Return the updated episodes
    data_provider.get_all_episodes(podcast_id).await
        .map_err(RustcastError::from)
}

fn format_time(seconds: f64) -> String {
    if seconds <= 0.0 {
        return "Not started".to_string();
    }

    let total_seconds = seconds as i64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}
