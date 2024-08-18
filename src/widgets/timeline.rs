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
        let desired_size_x = ui.available_size().x;
        let desired_size_y = 7.0;
        let desired_size : eframe::egui::Vec2 = eframe::egui::vec2(desired_size_x, desired_size_y);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, eframe::egui::Sense::click_and_drag());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            let rect = rect.expand(visuals.expansion);
            let radius = 0.3 * rect.height();
            ui.painter()
                .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);

            let mut fill_rect = rect;
            if response.is_pointer_button_down_on() || response.dragged() {
                if let Some(pt) = response.interact_pointer_pos() {
                    fill_rect.max.x = pt.x;
                    if fill_rect.width() > rect.width() {
                        fill_rect.set_width(rect.width());
                    }

                    ui.painter()
                        .rect_filled(fill_rect, radius, eframe::egui::Color32::from_rgb(0, 155, 255));

                    if pt.x < fill_rect.min.x {
                        *self.seek_position = 0.0;
                    } else if pt.x > rect.max.x {
                        *self.seek_position = self.total - 10.0;
                    } else {
                        *self.seek_position = self.total * fill_rect.width() as f64 / rect.width() as f64;
                    }
                    response.mark_changed();
                }
            } else {
                fill_rect.set_width(fill_rect.width() * self.progress as f32 / self.total as f32);
                ui.painter()
                    .rect_filled(fill_rect, radius, eframe::egui::Color32::from_rgb(0, 155, 255));
            }
        }

        response
    }
}
