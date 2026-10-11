use nc_ui::modal::{ModalTheme, render_modal_box, wrap_formatted_help_lines};
use nc_ui::theme::classic;

use crate::screen::PlayfieldBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuHelpTopic {
    Main,
    General,
    Fleet,
    Starbase,
    Planet,
    Build,
    FirstTime,
}

pub struct HelpSpec {
    pub title: &'static str,
    pub lines: &'static [&'static str],
}

const MAIN_LOCAL_LINES: [&str; 12] = [
    "<C> - open the color theme picker",
    "<B> - display all empire information in brief format",
    "<D> - display all empire information in detailed format",
    "<F> - bring up the Fleet Command Center menu",
    "<G> - bring up the General Command Center menu",
    "<?> - show Main Menu command help",
    "<I> - show Intelligence on what you know about any planet",
    "<P> - bring up the Planet Command Center menu",
    "<Q> - quit Nostrian Conquest and returns you back to Jump Start",
    "<T> - list database information about planets",
    "<V> - display a portion of the map (goto GENERAL MENU for entire map)",
    "<X> - hide/show command menus",
];

const MAIN_DOOR_LINES: [&str; 12] = [
    "<A> - turn ANSI color on or off",
    "<B> - display all empire information in brief format",
    "<D> - display all empire information in detailed format",
    "<F> - bring up the Fleet Command Center menu",
    "<G> - bring up the General Command Center menu",
    "<?> - show Main Menu command help",
    "<I> - show Intelligence on what you know about any planet",
    "<P> - bring up the Planet Command Center menu",
    "<Q> - quit Nostrian Conquest and returns you back to Jump Start",
    "<T> - list database information about planets",
    "<V> - display a portion of the map (goto GENERAL MENU for entire map)",
    "<X> - hide/show command menus",
];

const GENERAL_LINES: [&str; 14] = [
    "<A> - allow maintenance to issue orders and builds automatically",
    "<C> - type and send messages to other players in the game",
    "<D> - clear all reports",
    "<E> - list and declare your enemies",
    "<?> - show General Command Center help",
    "<I> - show intelligence on what you know about any planet",
    "<M> - display the entire game map for capture or export",
    "<O> - list all empires in the order you specify",
    "<P> - display the profile of your empire",
    "<Q> - quit the General Command Center and return to the Main Menu",
    "<R> - review your inbox of recent messages and reports",
    "<S> - display time left to play and other status information",
    "<V> - display a portion of the map; use M for the whole map",
    "<X> - hide/show command menus",
];

const FLEET_LINES: [&str; 17] = [
    "<B> - display a brief list of your fleets with locations, destinations, etc.",
    "<C> - change a fleet's ROE, ID Number, or Speed of Travel.",
    "<D> - detach one or more starships from a fleet to form a new fleet",
    "<E> - calculate any fleet's travel time to any potential destination",
    "<F> - display a longer but detailed list of your fleets",
    "Fleet List - use SPACE on the Sel column to check fleets for O/C/M/T bulk work",
    "<?> - show Fleet Command Center help",
    "<I> - show Intelligence on what you know about any planet",
    "<L> - load one or more armies from a planet onto the transports of a fleet",
    "<M> - merge two fleets into a single fleet - also see merge mission under <O>",
    "<O> - allow you to assign orders to any fleet (ie, send it onto a mission)",
    "<Q> - quit the Fleet Command Center menu & returns you to the Main Menu",
    "<S> - bring up the Starbase Control menu",
    "<T> - transfer one or more starships from one fleet to another",
    "<U> - unload one or more armies from a fleet to a planet",
    "<V> - display a partial map",
    "<X> - hide or show menus",
];

const STARBASE_LINES: [&str; 8] = [
    "<?> - show Starbase Control help",
    "<I> - show Intelligence on what you know about any planet",
    "<M> - order a starbase to move to a new location",
    "<Q> - quit the Starbase Control menu & returns you to the Fleet Command Center",
    "<R> - display all game information regarding a specified starbase",
    "<S> - display all of your starbases with their locations, destinations etc.",
    "<V> - display a portion of the map (goto GENERAL MENU for entire map)",
    "<X> - hide/show menus",
];

const PLANET_LINES: [&str; 14] = [
    "<M> - mass commission ships and starbases waiting in stardock",
    "<B> - open the build menu to spend production on local construction",
    "<C> - open the commission menu for fine-grained ground-defense control",
    "<D> - display a detailed list of your planets and their economies",
    "<?> - show Planet Command help",
    "<I> - display information you know about any planet",
    "<L> - load transport fleets with armies from the selected planet",
    "<P> - display a brief list of your planets",
    "<Q> - quit Planet Command and return to the Main Menu",
    "<S> - set or rescind a planet's scorch-earth denial order",
    "<T> - set the empire-wide tax rate used for yearly revenue",
    "<U> - unload transport fleets with armies at a planet",
    "<V> - display a partial starmap centered where you choose",
    "<X> - hide/show command menus",
];

const BUILD_LINES: [&str; 13] = [
    "<S> - specify build orders using this planet's build budget",
    "<D> - display units currently queued for construction",
    "<R> - review the current build planet through planet information",
    "<C> - change to another owned planet for local build orders",
    "<N> - move to the next owned planet in the build cycle",
    "<A> - abort queued build orders on the current planet",
    "<Q> - return to the Build Command menu",
    "<X> - hide/show command menus",
    "Build queue - work still in progress; those PP are already committed",
    "Stardock - completed ships and starbases waiting for commission",
    "Armies / batteries - complete immediately and do not enter stardock",
    "Full stardock - ships and starbases wait in queue until space opens",
    "Commission - lift completed ships and starbases out of stardock",
];

const FIRST_TIME_LOCAL_LINES: [&str; 6] = [
    "<C> - open the color theme picker",
    "<?> - show First Time Menu help",
    "<J> - join the game and control an unowned empire",
    "<L> - list all empires in the order you specify",
    "<Q> - quit Nostrian Conquest and return to the BBS",
    "<V> - view the introduction to this game",
];

const FIRST_TIME_DOOR_LINES: [&str; 6] = [
    "<A> - turn ANSI color on or off",
    "<?> - show First Time Menu help",
    "<J> - join the game and control an unowned empire",
    "<L> - list all empires in the order you specify",
    "<Q> - quit Nostrian Conquest and return to the BBS",
    "<V> - view the introduction to this game",
];

pub fn menu_help_spec(topic: MenuHelpTopic, door_mode: bool) -> HelpSpec {
    match topic {
        MenuHelpTopic::Main => HelpSpec {
            title: "HELP WITH COMMANDS",
            lines: if door_mode {
                &MAIN_DOOR_LINES
            } else {
                &MAIN_LOCAL_LINES
            },
        },
        MenuHelpTopic::General => HelpSpec {
            title: "GENERAL COMMAND HELP",
            lines: &GENERAL_LINES,
        },
        MenuHelpTopic::Fleet => HelpSpec {
            title: "FLEET COMMAND HELP",
            lines: &FLEET_LINES,
        },
        MenuHelpTopic::Starbase => HelpSpec {
            title: "STARBASE HELP",
            lines: &STARBASE_LINES,
        },
        MenuHelpTopic::Planet => HelpSpec {
            title: "PLANET COMMAND HELP",
            lines: &PLANET_LINES,
        },
        MenuHelpTopic::Build => HelpSpec {
            title: "BUILD COMMAND HELP",
            lines: &BUILD_LINES,
        },
        MenuHelpTopic::FirstTime => HelpSpec {
            title: "FIRST TIME HELP",
            lines: if door_mode {
                &FIRST_TIME_DOOR_LINES
            } else {
                &FIRST_TIME_LOCAL_LINES
            },
        },
    }
}

pub fn help_lines(lines: &[&str]) -> Vec<String> {
    let mut rendered = Vec::new();
    let mut block = Vec::new();

    let flush_block = |rendered: &mut Vec<String>, block: &mut Vec<(&str, &str)>| {
        if block.is_empty() {
            return;
        }
        rendered.extend(nc_ui::modal::format_help_rows(block.iter().copied()));
        block.clear();
    };

    for line in lines {
        if line.is_empty() {
            flush_block(&mut rendered, &mut block);
            rendered.push(String::new());
            continue;
        }
        if let Some((command, description)) = line.split_once(" - ") {
            block.push((command, description));
        } else {
            flush_block(&mut rendered, &mut block);
            rendered.push((*line).to_string());
        }
    }
    flush_block(&mut rendered, &mut block);

    rendered
}

pub fn render_help_popup(buffer: &mut PlayfieldBuffer, title: &str, lines: &[String]) {
    let wrapped = wrap_formatted_help_lines(lines, buffer.width().saturating_sub(12));
    render_modal_box(
        buffer,
        title,
        &wrapped.lines,
        ModalTheme {
            body_style: classic::help_panel_style(),
            pad_style: classic::help_panel_style(),
            chrome_style: classic::table_chrome_style(),
            title_style: classic::table_header_style(),
        },
    );
    buffer.clear_cursor();
}
