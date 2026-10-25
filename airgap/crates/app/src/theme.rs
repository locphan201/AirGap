use egui::{Color32, Context, CornerRadius, Stroke, Style, Theme, Visuals};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

impl ThemeMode {
    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
            ThemeMode::System => "System",
        }
    }
}

pub fn apply_theme(ctx: &Context, mode: ThemeMode) {
    let mut style = Style::default();

    let is_dark = match mode {
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
        ThemeMode::System => {
            ctx.input(|i| i.raw.system_theme.unwrap_or(Theme::Dark)) == Theme::Dark
        }
    };

    let mut visuals = if is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    if is_dark {
        // Refined Dark Palette
        visuals.panel_fill = Color32::from_rgb(24, 24, 27); // Zinc-900
        visuals.window_fill = Color32::from_rgb(18, 18, 21);
        visuals.faint_bg_color = Color32::from_rgb(30, 30, 34);
        visuals.extreme_bg_color = Color32::from_rgb(15, 15, 18);

        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 24, 27);
        visuals.widgets.noninteractive.fg_stroke =
            Stroke::new(1.0f32, Color32::from_rgb(244, 244, 245));
        visuals.widgets.noninteractive.bg_stroke =
            Stroke::new(1.0f32, Color32::from_rgb(39, 39, 42));
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        visuals.widgets.inactive.bg_fill = Color32::from_rgb(32, 32, 38);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0f32, Color32::from_rgb(212, 212, 216));
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(50, 50, 58));
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 45, 55);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0f32, Color32::WHITE);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(96, 165, 250));
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

        visuals.widgets.active.bg_fill = Color32::from_rgb(59, 130, 246);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0f32, Color32::WHITE);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(59, 130, 246, 80);
        visuals.selection.stroke = Stroke::new(1.0f32, Color32::from_rgb(96, 165, 250));
    } else {
        // Refined Light Palette
        visuals.panel_fill = Color32::from_rgb(250, 250, 250);
        visuals.window_fill = Color32::from_rgb(255, 255, 255);
        visuals.faint_bg_color = Color32::from_rgb(244, 244, 245);
        visuals.extreme_bg_color = Color32::from_rgb(255, 255, 255);

        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(250, 250, 250);
        visuals.widgets.noninteractive.fg_stroke =
            Stroke::new(1.0f32, Color32::from_rgb(24, 24, 27));
        visuals.widgets.noninteractive.bg_stroke =
            Stroke::new(1.0f32, Color32::from_rgb(228, 228, 231));
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        visuals.widgets.inactive.bg_fill = Color32::from_rgb(240, 240, 243);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0f32, Color32::from_rgb(39, 39, 42));
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(212, 212, 216));
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(228, 228, 231);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0f32, Color32::BLACK);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(37, 99, 235));
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

        visuals.widgets.active.bg_fill = Color32::from_rgb(37, 99, 235);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0f32, Color32::WHITE);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(37, 99, 235, 60);
        visuals.selection.stroke = Stroke::new(1.0f32, Color32::from_rgb(37, 99, 235));
    }

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);

    ctx.set_style(style);
}
