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

            // timeline_fill(&rect, &visuals.bg_stroke, ui, eframe::egui::Pos2::new(rect.max.x * 0.5, 0.0));


            if response.clicked() || response.dragged() {
                if let Some(pt) = response.interact_pointer_pos() {
                    println!("{:?}", pt);
                    timeline_fill(&rect, &visuals.bg_stroke, ui, pt);
                    response.mark_changed();
                    return response;
                }
            }
        }

        response
    }
}

fn timeline_fill(timeline_rect: &eframe::egui::Rect, stroke: &eframe::egui::Stroke, ui: &eframe::egui::Ui, pt: eframe::egui::Pos2) {
    let mut fill_rect = eframe::egui::Rect::ZERO;
    fill_rect.min = timeline_rect.min;
    fill_rect.max = eframe::egui::Pos2::new(pt.x, timeline_rect.max.y);
    fill_rect.set_height(timeline_rect.height());
    ui.painter().rect(fill_rect, 0.3 * timeline_rect.height(), eframe::egui::Color32::from_rgb(0, 155, 255), *stroke);

    let center = eframe::egui::pos2(pt.x, timeline_rect.center().y);
    ui.painter()
        .circle(center, 6.0, ui.style().noninteractive().bg_fill, ui.style().noninteractive().fg_stroke);
}
