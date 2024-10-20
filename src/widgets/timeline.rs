use eframe::egui::Rect;

#[derive(Debug)]
pub struct Timeline<'a> {
    pub progress: f64,
    pub total: f64,
    pub seek_position: &'a mut f64,
}

impl<'a> Timeline<'a> {
    pub fn new(progress: f64, total: f64, seek_position: &'a mut f64) -> Self {
        Self {
            progress,
            total,
            seek_position,
        }
    }
}

impl<'a> eframe::egui::Widget for &mut Timeline<'a> {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let desired_size_x = (ui.available_size().x as i32 - 120) as f32;
        let desired_size_y = 7.0;
        let desired_size: eframe::egui::Vec2 = eframe::egui::vec2(desired_size_x, desired_size_y);
        let (rect, mut response) =
            ui.allocate_exact_size(desired_size, eframe::egui::Sense::click_and_drag());
        let visuals = ui.style().interact(&response);

        if ui.is_rect_visible(rect) {
            let rect = rect.expand(visuals.expansion);
            let radius = 0.3 * rect.height();
            ui.painter()
                .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);

            if response.hovered() {
                if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let seek_time;
                    if mouse_pos.x < rect.min.x {
                        seek_time = 0.0;
                    } else if mouse_pos.x > rect.max.x {
                        seek_time = self.total;
                    } else {
                        seek_time = self.total * (mouse_pos.x - rect.min.x) as f64 / rect.width() as f64
                    }

                    draw_tooltip(
                        ui,
                        eframe::egui::Pos2::new(mouse_pos.x, rect.min.y - 20.0),
                        time_to_display(seek_time),
                        visuals.text_color(),
                        visuals.bg_fill,
                    );
                }
            }

            let mut fill_rect = rect;
            if response.is_pointer_button_down_on() || response.dragged() {
                if let Some(pt) = response.interact_pointer_pos() {
                    fill_rect.max.x = pt.x;
                    if fill_rect.width() > rect.width() {
                        fill_rect.set_width(rect.width());
                    }

                    ui.painter().rect_filled(
                        fill_rect,
                        radius,
                        eframe::egui::Color32::from_rgb(0, 155, 255),
                    );

                    if pt.x < fill_rect.min.x {
                        *self.seek_position = 0.0;
                    } else if pt.x > rect.max.x {
                        *self.seek_position = self.total - 10.0;
                    } else {
                        *self.seek_position =
                            self.total * fill_rect.width() as f64 / rect.width() as f64;
                    }

                    response.mark_changed();
                }
            } else {
                fill_rect.set_width(fill_rect.width() * self.progress as f32 / self.total as f32);
                ui.painter().rect_filled(
                    fill_rect,
                    radius,
                    eframe::egui::Color32::from_rgb(0, 155, 255),
                );
            }
        }

        ui.painter().text(
            rect.left_top() + eframe::egui::Vec2::new(-60.0, -3.0),
            eframe::egui::Align2::LEFT_TOP,
            time_to_display(self.progress),
            eframe::egui::FontId::proportional(12.0),
            visuals.text_color(),
        );

        ui.painter().text(
            rect.right_top() + eframe::egui::Vec2::new(10.0, -3.0),
            eframe::egui::Align2::LEFT_TOP,
            time_to_display(self.total),
            eframe::egui::FontId::proportional(12.0),
            visuals.text_color(),
        );

        response
    }
}

fn time_to_display(seconds: f64) -> String {
    let is: i64 = seconds.round() as i64;
    let hours = is / (60 * 60);
    let mins = (is % (60 * 60)) / 60;
    let secs = seconds - 60.0 * mins as f64 - 60.0 * 60.0 * hours as f64; // is % 60;

    format!("{}:{:0>2}:{:0>4.1}", hours, mins, secs)
}

fn draw_tooltip(
    ui: &eframe::egui::Ui,
    pos: eframe::egui::Pos2,
    tooltip_text: impl ToString,
    text_color: eframe::egui::Color32,
    tooltip_color: eframe::egui::Color32,
) {
    let layer_id = eframe::egui::LayerId::new(eframe::egui::Order::Foreground, ui.id().with("foreground_layer"));
    let foreground_painter = ui.ctx().layer_painter(layer_id);

    let rect = Rect::from_min_size(pos, eframe::egui::vec2(60.0, 17.0));
    let rounding = eframe::egui::Rounding::same(5.0);

    foreground_painter.rect_filled(rect, rounding, tooltip_color);

    foreground_painter.text(
        rect.center(),
        eframe::egui::Align2::CENTER_CENTER,
        tooltip_text,
        eframe::egui::FontId::proportional(12.0),
        text_color,
    );
}
