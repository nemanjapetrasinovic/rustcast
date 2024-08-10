
#[derive(Debug, Default)]
pub struct Timeline {
    pub progress: f64,
    pub progress_display: String,
    pub total: f64,
    pub total_display: String,
    pub playing: bool,
    pub error: Option<String>
}

impl eframe::egui::Widget for Timeline {
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

            let mut fill_rect = eframe::egui::Rect::ZERO;
            fill_rect.min = rect.min;
            fill_rect.max = eframe::egui::Pos2::new(rect.min.x + ui.available_size().x * 0.5, rect.max.y);
            fill_rect.set_height(rect.height());

            ui.painter().rect(fill_rect, radius, eframe::egui::Color32::from_rgb(0, 155, 255), visuals.bg_stroke);

            let circle_x = rect.left() + 50.0;
            let center = eframe::egui::pos2(circle_x, rect.center().y);
            ui.painter()
                .circle(center, 6.0, visuals.bg_fill, visuals.fg_stroke);
        }

        response
    }
}
