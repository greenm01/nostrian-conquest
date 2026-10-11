use crate::dashboard::app::state::{DashApp, OwnedPlanetPopupMode};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::{self, MapWidgetFrame, dashboard};
use crate::dashboard::modal::{Rect, compact_content_width, max_content_width};
use crate::dashboard::overlays::frame::{
    OverlayFrame, OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent,
};
use crate::dashboard::planet_view::planet_detail_for_record;
use crate::dashboard::table::{TableFooter, with_command_line_toast};
use crate::dashboard::theme;
use nc_data::ProductionItemKind;
use nc_engine::{
    ArmyTransportMode, build_unit_spec_by_kind, transport_fleet_candidates_for_planet,
};

pub fn draw(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    _map_frame: MapWidgetFrame,
    planet_record_index_1_based: usize,
) {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let compact_body_width = compact_content_width(parent);
    let label_value_body_width = max_content_width(parent);
    let popup = popup_layout(app, compact_body_width, label_value_body_width);
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        &popup.title,
        popup.body_width,
        popup.body_height,
        OverlaySizePolicy::default(),
        popup.footer,
        app.popup_position_for(crate::dashboard::app::state::ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        }),
    );
    draw_popup_body(buf, frame, popup.body);
}

pub fn popup_rect(
    app: &DashApp,
    _map_frame: MapWidgetFrame,
    planet_record_index_1_based: usize,
) -> Rect {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let compact_body_width = compact_content_width(parent);
    let label_value_body_width = max_content_width(parent);
    let popup = popup_layout(app, compact_body_width, label_value_body_width);
    overlay_popup_rect_for_body_in_parent(
        parent,
        &popup.title,
        popup.body_width,
        popup.body_height,
        OverlaySizePolicy::default(),
        popup.footer,
        app.popup_position_for(crate::dashboard::app::state::ActivePopup::OwnedPlanet {
            planet_record_index_1_based,
        }),
    )
}

struct PopupLayout<'a> {
    title: String,
    body_width: usize,
    body_height: usize,
    footer: TableFooter<'a>,
    body: PopupBody,
}

enum PopupBody {
    Plain(Vec<String>),
}

fn popup_layout<'a>(
    app: &'a DashApp,
    compact_body_width: usize,
    label_value_body_width: usize,
) -> PopupLayout<'a> {
    match app.owned_planet_popup.mode {
        OwnedPlanetPopupMode::Browse => {
            let lines = browse_lines(app, label_value_body_width);
            plain_popup_layout(
                String::from("PLANET STATUS"),
                lines,
                command_line_toast_footer(
                    app,
                    TableFooter::CommandPrompt {
                        label: "COMMAND",
                        prompt: "? B C A L U X <ESC> ->",
                    },
                ),
            )
        }
        OwnedPlanetPopupMode::CommissionSelect => {
            let default = if app.owned_planet_popup.default.is_empty() {
                "01"
            } else {
                app.owned_planet_popup.default.as_str()
            };
            plain_popup_layout(
                String::from("PLANET STATUS"),
                wrap_plain_lines(&commission_lines(app), compact_body_width),
                command_line_toast_footer(
                    app,
                    TableFooter::CommandInput {
                        label: "COMMAND",
                        prompt: "Commission slot #",
                        default,
                        input: &app.owned_planet_popup.input,
                    },
                ),
            )
        }
        OwnedPlanetPopupMode::CommissionResult => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&app.owned_planet_popup.report_lines, compact_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "<ESC> ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::MassCommissionConfirm => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(
                &[String::from(
                    "Automatically commission all ships and starbases in stardock?",
                )],
                compact_body_width,
            ),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "Mass commission? Y/[N] ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::MassCommissionReport => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&app.owned_planet_popup.report_lines, compact_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "<ESC> ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::TransportFleetSelect { mode } => {
            let prompt = match mode {
                ArmyTransportMode::Load => "Load Fleet #",
                ArmyTransportMode::Unload => "Unload Fleet #",
            };
            let default = if app.owned_planet_popup.default.is_empty() {
                "1"
            } else {
                app.owned_planet_popup.default.as_str()
            };
            plain_popup_layout(
                String::from("PLANET STATUS"),
                wrap_plain_lines(&transport_fleet_lines(app, mode), compact_body_width),
                command_line_toast_footer(
                    app,
                    TableFooter::CommandInput {
                        label: "COMMAND",
                        prompt,
                        default,
                        input: &app.owned_planet_popup.input,
                    },
                ),
            )
        }
        OwnedPlanetPopupMode::TransportQuantity { mode } => {
            let prompt = match mode {
                ArmyTransportMode::Load => "How many armies to load?",
                ArmyTransportMode::Unload => "How many armies to unload?",
            };
            let default = if app.owned_planet_popup.default.is_empty() {
                "0"
            } else {
                app.owned_planet_popup.default.as_str()
            };
            plain_popup_layout(
                String::from("PLANET STATUS"),
                wrap_plain_lines(&transport_quantity_lines(app, mode), compact_body_width),
                command_line_toast_footer(
                    app,
                    TableFooter::CommandInput {
                        label: "COMMAND",
                        prompt,
                        default,
                        input: &app.owned_planet_popup.input,
                    },
                ),
            )
        }
        OwnedPlanetPopupMode::ScorchConfirm1 => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&scorch_lines(app), compact_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: if scorch_in_force(app) {
                        "Rescind Scorch-Earth? Y/[N] ->"
                    } else {
                        "Are you sure? Y/[N] ->"
                    },
                },
            ),
        ),
        OwnedPlanetPopupMode::ScorchConfirm2 => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&scorch_lines(app), compact_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "Are you really sure? Y/[N] ->",
                },
            ),
        ),
        OwnedPlanetPopupMode::ScorchConfirm3 => plain_popup_layout(
            String::from("PLANET STATUS"),
            wrap_plain_lines(&scorch_lines(app), compact_body_width),
            command_line_toast_footer(
                app,
                TableFooter::CommandPrompt {
                    label: "COMMAND",
                    prompt: "Are you sure-sure? Last chance to bail! Y/[N] ->",
                },
            ),
        ),
    }
}

fn command_line_toast_footer<'a>(app: &'a DashApp, footer: TableFooter<'a>) -> TableFooter<'a> {
    with_command_line_toast(footer, app.active_command_line_toast())
}

fn plain_popup_layout<'a>(
    title: String,
    lines: Vec<String>,
    footer: TableFooter<'a>,
) -> PopupLayout<'a> {
    PopupLayout {
        title,
        body_width: max_line_width(&lines).max(1),
        body_height: lines.len().max(1),
        footer,
        body: PopupBody::Plain(lines),
    }
}

fn draw_popup_body(buf: &mut PlayfieldBuffer, frame: OverlayFrame, body: PopupBody) {
    let PopupBody::Plain(lines) = body;
    for (idx, line) in lines.into_iter().enumerate().take(frame.body_height) {
        layout::write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            &line,
            theme::value_style(),
        );
    }
}

fn max_line_width(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
}

fn browse_lines(app: &DashApp, max_body_width: usize) -> Vec<String> {
    app.owned_planet_popup_record_index_1_based()
        .and_then(|record| planet_detail_for_record(app, record))
        .map(|detail| {
            crate::dashboard::popups::planet_detail::popup_lines(
                &detail.popup_lines,
                max_body_width,
            )
        })
        .unwrap_or_else(|| vec![String::from("No planet selected.")])
}

fn commission_lines(app: &DashApp) -> Vec<String> {
    let mut lines = vec![String::from("Stardock slots ready to commission:")];
    for row in app.owned_planet_commission_entries() {
        lines.push(format!(
            "{:02}  {:<16} qty {:>3}",
            row.slot_0_based + 1,
            build_item_label(row.kind),
            row.qty
        ));
    }
    lines
}

fn transport_fleet_lines(app: &DashApp, mode: ArmyTransportMode) -> Vec<String> {
    let Some(planet) = app.owned_planet_row() else {
        return vec![String::from("Planet unavailable.")];
    };
    let mut lines = vec![format!(
        "{} ({:02},{:02})",
        planet.planet_name, planet.coords[0], planet.coords[1]
    )];
    for fleet in transport_fleet_candidates_for_planet(
        &app.game_data,
        app.player_record_index_1_based as u8,
        mode,
        &planet,
    )
    .into_iter()
    .filter(|fleet| fleet.available_qty > 0)
    {
        lines.push(format!(
            "Fleet {:02}  TTs {:>2}  loaded {:>2}  available {:>2}",
            fleet.fleet_number, fleet.troop_transports, fleet.loaded_armies, fleet.available_qty
        ));
    }
    lines
}

fn transport_quantity_lines(app: &DashApp, mode: ArmyTransportMode) -> Vec<String> {
    let action = match mode {
        ArmyTransportMode::Load => "load",
        ArmyTransportMode::Unload => "unload",
    };
    let fleet_number = app
        .owned_planet_popup
        .transport_selected_fleet_number
        .unwrap_or_default();
    let lines = vec![
        format!("Fleet {:02} selected.", fleet_number),
        format!(
            "Available armies to {action}: {}",
            app.owned_planet_popup.transport_available_qty
        ),
    ];
    lines
}

fn scorch_lines(app: &DashApp) -> Vec<String> {
    let planet = app
        .owned_planet_record()
        .map(|planet| {
            format!(
                "Planet \"{}\" at ({:02},{:02}).",
                planet.name.trim().to_string(),
                planet.sector.to_array()[0],
                planet.sector.to_array()[1]
            )
        })
        .unwrap_or_else(|| String::from("Selected planet unavailable."));
    if scorch_in_force(app) {
        return vec![
            planet,
            String::new(),
            String::from(
                "Scorch-Earth is in force here. Rescinding it ends the yearly decay at the next maintenance.",
            ),
            String::from("Production and stored points already lost are not restored."),
        ];
    }
    vec![
        planet,
        String::new(),
        String::from("Scorch-Earth destroys anything useful to an invading force."),
        String::from(
            "From the next maintenance, and every year until you rescind it, production falls to about a third and stored points to a tenth or less. The planet neither grows nor earns tax. Potential is not lost.",
        ),
    ]
}

fn scorch_in_force(app: &DashApp) -> bool {
    app.owned_planet_record()
        .is_some_and(|planet| planet.scorch_earth)
}

fn wrap_plain_lines(lines: &[String], width: usize) -> Vec<String> {
    let mut wrapped = Vec::new();
    for line in lines {
        if line.is_empty() {
            wrapped.push(String::new());
            continue;
        }
        wrapped.extend(wrap_plain_line(line, width.max(1)));
    }
    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

fn wrap_plain_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    if line.chars().count() <= width {
        return vec![line.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let word_width = word.chars().count();
        if current.is_empty() {
            if word_width <= width {
                current.push_str(word);
            } else {
                lines.extend(chunk_word(word, width));
            }
            continue;
        }
        if current.chars().count() + 1 + word_width <= width {
            current.push(' ');
            current.push_str(word);
            continue;
        }
        lines.push(current);
        current = String::new();
        if word_width <= width {
            current.push_str(word);
        } else {
            let mut chunks = chunk_word(word, width);
            current = chunks.pop().unwrap_or_default();
            lines.extend(chunks);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn chunk_word(word: &str, width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        if current.chars().count() == width {
            chunks.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn build_item_label(kind: ProductionItemKind) -> &'static str {
    build_unit_spec_by_kind(kind)
        .map(|unit| unit.label)
        .unwrap_or_else(|| match kind {
            ProductionItemKind::Starbase => "Starbase",
            ProductionItemKind::GroundBattery => "Ground Battery",
            ProductionItemKind::Army => "Army",
            _ => "?",
        })
}
