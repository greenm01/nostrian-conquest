use crate::screen::help::{MenuHelpTopic, help_lines, menu_help_spec, render_help_popup};
use crate::screen::{PlanetListMode, PlayfieldBuffer, ScreenId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupHelp {
    pub title: String,
    pub lines: Vec<String>,
}

pub fn render_popup(buffer: &mut PlayfieldBuffer, popup: &PopupHelp) {
    render_help_popup(buffer, &popup.title, &popup.lines);
}

pub fn popup_for_screen(screen: ScreenId, door_mode: bool) -> Option<PopupHelp> {
    match screen {
        ScreenId::MainMenu => Some(from_menu_topic(MenuHelpTopic::Main, door_mode)),
        ScreenId::GeneralMenu => Some(from_menu_topic(MenuHelpTopic::General, door_mode)),
        ScreenId::FleetMenu => Some(from_menu_topic(MenuHelpTopic::Fleet, door_mode)),
        ScreenId::StarbaseMenu => Some(from_menu_topic(MenuHelpTopic::Starbase, door_mode)),
        ScreenId::PlanetMenu => Some(from_menu_topic(MenuHelpTopic::Planet, door_mode)),
        ScreenId::PlanetBuildMenu => Some(from_menu_topic(MenuHelpTopic::Build, door_mode)),
        ScreenId::FirstTimeMenu => Some(from_menu_topic(MenuHelpTopic::FirstTime, door_mode)),

        ScreenId::ThemePicker => Some(table_help(
            "COLOR THEMES",
            &[
                ("Type", "filter themes by name"),
                ("Enter", "apply selected theme"),
                ("Q", "close picker"),
                ("Esc", "close picker"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::FleetList => Some(table_help(
            "FLEET COMMANDS",
            &[
                ("Digits", "jump to a fleet number"),
                ("SPACE", "toggle the checked state of the current fleet row"),
                ("Enter", "review highlighted fleet"),
                ("F", "Filter List"),
                ("S", "Sort List"),
                (
                    "O",
                    "assign orders to checked fleets, or the selected fleet",
                ),
                (
                    "C",
                    "change checked fleets: ROE or speed; selected fleet also allows ID",
                ),
                ("E", "calculate travel time (ETA) for selected fleet"),
                ("D", "detach ships from selected fleet"),
                ("M", "merge checked fleets, or merge the selected fleet"),
                (
                    "T",
                    "transfer ships between checked fleets, or from the selected fleet",
                ),
                ("L", "load armies from selected fleet"),
                ("U", "unload armies from selected fleet"),
                ("Ships", nc_data::NOTATION_LEGEND),
                ("+", "more ships; Review shows the full composition"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::FleetListFilterPrompt => Some(table_help(
            "FILTER COMMANDS",
            &[
                ("Type", "enter a column code or unique prefix, then Enter"),
                ("Codes", "id sel loc ord tar spd eta roe ars shi"),
                (
                    "Prefix",
                    "ambiguous prefixes stay open and show matching codes",
                ),
                ("Order", "ord also accepts holding, moving, and combat"),
                ("Selected", "sel accepts yes/no, selected, unselected, or x"),
                (
                    "Value",
                    "enter text, a number test, or coords at the next prompt",
                ),
                ("all", "clear the current filter"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::FleetListSortPrompt => Some(table_help(
            "SORT COMMANDS",
            &[
                ("Type", "enter a column code or unique prefix, then Enter"),
                ("Codes", "id sel loc ord tar spd eta roe ars shi"),
                (
                    "Prefix",
                    "ambiguous prefixes stay open and show matching codes",
                ),
                ("Repeat", "same sort flips ASC/DESC"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::FleetDetach => Some(table_help(
            "DETACH SHIPS",
            &[
                ("Type", "enter a ship class, then a quantity"),
                ("3", "stage three ships, healthy ones first"),
                ("3!", "stage exactly three crippled ships"),
                ("Ships", nc_data::NOTATION_LEGEND),
                ("D", "finish staging, then confirm the detach"),
                ("Enter", "accept the input or confirm the detach"),
                ("N", "return from confirmation to editing"),
                ("X", "clear staged ships while editing"),
                ("Q", "cancel; at a quantity prompt, return to classes"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::FleetTransfer => Some(table_help(
            "TRANSFER SHIPS",
            &[
                ("Type", "enter a ship class, then a quantity"),
                ("3", "stage three ships, healthy ones first"),
                ("3!", "stage exactly three crippled ships"),
                ("Ships", nc_data::NOTATION_LEGEND),
                ("T", "finish staging, then confirm the transfer"),
                ("Enter", "accept the input or confirm the transfer"),
                ("N", "return from confirmation to editing"),
                ("X", "clear staged ships while editing"),
                ("Q", "cancel; at a quantity prompt, return to classes"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::FleetMissionPicker | ScreenId::FleetEta => Some(table_help(
            "FLEET COMMANDS",
            &[
                ("Enter", "accept highlighted item"),
                ("Type", "use the prompt value shown on the command line"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::StarbaseList | ScreenId::StarbaseReviewSelect => Some(table_help(
            "STARBASE COMMANDS",
            &[
                ("Digits", "jump to a starbase number"),
                ("Enter", "review highlighted starbase"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetBuildList => Some(table_help(
            "PLANET COMMANDS",
            &[
                ("D", "delete highlighted build order"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetList(PlanetListMode::Brief, _) => Some(table_help(
            "PLANET COMMANDS",
            &[
                ("Coords", "jump to a planet by coordinates"),
                ("F", "Filter List"),
                ("I", "review highlighted planet"),
                ("Enter", "review highlighted planet"),
                ("B", "specify new build orders for selected planet"),
                ("D", "display queued build orders for selected planet"),
                ("A", "abort queued build orders for selected planet"),
                ("C", "manually commission ships into a fleet"),
                ("M", "mass commission ships from stardock to fleets"),
                ("L", "load armies from planet onto transports"),
                ("U", "unload armies from transports to planet"),
                ("X", "scorch earth (set or rescind)"),
                ("S", "Sort List"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetDatabaseList => Some(table_help(
            "DATABASE COMMANDS",
            &[
                ("Coords", "jump to a world by coordinates"),
                ("F", "Filter List"),
                ("S", "Sort List"),
                ("Enter", "review highlighted world"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetList(_, _)
        | ScreenId::PlanetCommissionPicker
        | ScreenId::PlanetCommissionMenu
        | ScreenId::PlanetCommissionDraft
        | ScreenId::PlanetAutoCommissionReport
        | ScreenId::PlanetTransportPlanetSelect(_)
        | ScreenId::PlanetTransportFleetSelect(_)
        | ScreenId::PlanetBuildChange => Some(table_help(
            "PLANET COMMANDS",
            &[
                ("Enter", "accept highlighted item"),
                ("Type", "use the prompt value shown on the command line"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetListSortPrompt(_) => Some(table_help(
            "SORT COMMANDS",
            &[
                ("Type", "enter a column code or unique prefix, then Enter"),
                (
                    "Codes",
                    "coo pla max cur trs bdg rev gro bui sta sbs ars gbs",
                ),
                (
                    "Prefix",
                    "ambiguous prefixes stay open and show matching codes",
                ),
                ("Repeat", "same sort flips ASC/DESC"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetListFilterPrompt(_) => Some(table_help(
            "FILTER COMMANDS",
            &[
                ("Type", "enter a column code or unique prefix, then Enter"),
                (
                    "Codes",
                    "coo pla max cur trs bdg rev gro bui sta sbs ars gbs",
                ),
                (
                    "Prefix",
                    "ambiguous prefixes stay open and show matching codes",
                ),
                ("Coords", "coo accepts xx,yy or xx,yy/r"),
                (
                    "Value",
                    "text matches contain; numbers accept > >= < <= = !=",
                ),
                ("all", "clear the current filter"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetDatabaseFilterPrompt => Some(table_help(
            "FILTER COMMANDS",
            &[
                ("Type", "enter a column code or unique prefix, then Enter"),
                ("Codes", "coo pla own max see ars gbs sbs cur trs sco"),
                (
                    "Prefix",
                    "ambiguous prefixes stay open and show matching codes",
                ),
                ("Coords", "coo accepts xx,yy or xx,yy/r"),
                ("Unknown", "use ? for unknown database values"),
                (
                    "Value",
                    "text matches contain; numbers accept > >= < <= = !=",
                ),
                ("all", "clear the current filter"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetDatabaseSortPrompt => Some(table_help(
            "SORT COMMANDS",
            &[
                ("Type", "enter a column code or unique prefix, then Enter"),
                ("Codes", "coo pla own max see ars gbs sbs cur trs sco"),
                ("rng", "sort by range from a sector"),
                (
                    "Prefix",
                    "ambiguous prefixes stay open and show matching codes",
                ),
                ("Repeat", "same sort flips ASC/DESC"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PlanetTransportQuantityPrompt(_)
        | ScreenId::PlanetBuildSpecify
        | ScreenId::PlanetBuildQuantity
        | ScreenId::PlanetCommissionResult
        | ScreenId::PlanetTransportDone(_)
        | ScreenId::ComposeMessageDiscardConfirm
        | ScreenId::ComposeMessageSendConfirm
        | ScreenId::ComposeMessageSent => Some(prompt_help(
            "COMMAND HELP",
            &[
                ("Type", "enter the value shown on the command line"),
                ("Enter", "accept the current prompt"),
                ("Q", "cancel or return when available"),
                ("Esc", "cancel or return when available"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::ComposeMessageSubject => Some(prompt_help(
            "COMMAND HELP",
            &[
                ("Type", "enter the message subject"),
                ("Enter", "accept the current subject"),
                ("Esc", "return to recipient selection"),
                ("^G", "open this helper"),
            ],
        )),
        ScreenId::ComposeMessageBody => Some(prompt_help(
            "MESSAGE EDITOR HELP",
            &[
                ("^E", "confirm sending the message"),
                ("^X", "confirm canceling the message"),
                ("^G", "open this helper"),
            ],
        )),
        ScreenId::ComposeMessageRecipient => Some(table_help(
            "MESSAGE COMMANDS",
            &[
                ("D", "open queued outgoing messages"),
                ("Enter", "accept highlighted item"),
                ("Type", "use the prompt value shown on the command line"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::ComposeMessageOutbox => Some(table_help(
            "MESSAGE COMMANDS",
            &[
                ("Enter", "read the highlighted queued message"),
                ("D", "delete (cancel) the highlighted queued message"),
                ("Type", "use the prompt value shown on the command line"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::Enemies | ScreenId::Rankings(_) => Some(table_help(
            "EMPIRE COMMANDS",
            &[
                ("Enter", "accept highlighted item when available"),
                ("Type", "use the prompt value shown on the command line"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::Reports => Some(prompt_help(
            "INBOX COMMANDS",
            &[
                ("M", "filter to messages"),
                ("R", "filter to reports"),
                ("A", "filter to all items"),
                ("Y", "set or clear the year filter"),
                ("Tab", "switch focus between inbox and preview"),
                ("D", "delete the selected inbox item when offered"),
                ("Digits", "jump to the visible inbox ID on the command line"),
                ("Enter", "toggle preview focus or accept the current prompt"),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::PartialStarmapView | ScreenId::Starmap => Some(prompt_help(
            "MAP COMMANDS",
            &[
                (
                    "Enter",
                    "open info for the planet at the current map cursor",
                ),
                ("Q", "return"),
                ("Esc", "return"),
                ("?", "open this helper"),
            ],
        )),
        ScreenId::FirstTimeIntro | ScreenId::Startup(_) => Some(prompt_help(
            "INTRO HELP",
            &[
                ("Enter", "advance the current page"),
                ("Q", "return when offered"),
                ("Esc", "return when offered"),
                ("?", "open this helper"),
            ],
        )),
        _ => None,
    }
}

fn from_menu_topic(topic: MenuHelpTopic, door_mode: bool) -> PopupHelp {
    let spec = menu_help_spec(topic, door_mode);
    PopupHelp {
        title: spec.title.to_string(),
        lines: help_lines(spec.lines),
    }
}

fn table_help(title: &str, rows: &[(&str, &str)]) -> PopupHelp {
    PopupHelp {
        title: title.to_string(),
        lines: nc_ui::modal::format_help_rows(rows.iter().copied()),
    }
}

fn prompt_help(title: &str, rows: &[(&str, &str)]) -> PopupHelp {
    table_help(title, rows)
}
