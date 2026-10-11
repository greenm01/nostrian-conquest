use crate::app::helpers::{
    is_coordinate_input_char, resolve_default_coords_input, sync_scroll_to_cursor,
};
use crate::app::state::App;
use crate::domains::planet::state::PlanetCommandContext;
use crate::domains::planet::{KnownOwnerLabelStyle, PlanetAction, known_owner_label};
use crate::screen::{
    CommandMenu, PlanetDatabaseFilter, PlanetDatabaseFilterMode, PlanetDatabasePromptMode,
    PlanetDatabaseRow, PlanetDatabaseSort, PlanetDatabaseSortMode, PlanetListFilter,
    PlanetListFilterMode, PlanetListFilterPromptMode, PlanetListMode, PlanetListSort, ScreenId,
    SortDirection,
};
use nc_data::build_player_starmap_projection_from_snapshots;
use nc_engine::planet_build_view;
use nc_ui::table_filter::{
    ColumnCodeParseError, FilterKind, TableFilterClause, TableFilterColumn, TableFilterPredicate,
    format_column_code_error, is_filter_column_char, parse_column_code, parse_filter_clause,
};

const PLANET_LIST_FILTER_COLUMNS: &[TableFilterColumn] = &[
    TableFilterColumn {
        code: "coo",
        label: "Coord",
        aliases: &["coordinates", "location"],
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "pla",
        label: "Planet",
        aliases: &["name"],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "max",
        label: "Max",
        aliases: &["maximum", "potential"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "cur",
        label: "Current",
        aliases: &["currentprod", "production", "current production"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "trs",
        label: "Treasury",
        aliases: &["points", "treasury points"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "bdg",
        label: "Budget",
        aliases: &["bdgt", "bgdt"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "rev",
        label: "Revenue",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "gro",
        label: "Growth",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "bui",
        label: "Build",
        aliases: &["queue"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "sta",
        label: "Dock",
        aliases: &["stardock"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "sbs",
        label: "Starbase",
        aliases: &["starbases"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "ars",
        label: "Armies",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "gbs",
        label: "Batteries",
        aliases: &["groundbatteries"],
        kind: FilterKind::Number,
    },
];

const PLANET_DATABASE_FILTER_COLUMNS: &[TableFilterColumn] = &[
    TableFilterColumn {
        code: "coo",
        label: "Coord",
        aliases: &["coordinates", "location"],
        kind: FilterKind::Coord,
    },
    TableFilterColumn {
        code: "pla",
        label: "Planet",
        aliases: &["name"],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "own",
        label: "Owner",
        aliases: &["empire"],
        kind: FilterKind::Text,
    },
    TableFilterColumn {
        code: "max",
        label: "Max",
        aliases: &["maximum", "potential"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "see",
        label: "Seen",
        aliases: &["year", "yearseen", "seenyear"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "ars",
        label: "Armies",
        aliases: &[],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "gbs",
        label: "Batteries",
        aliases: &["groundbatteries"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "sbs",
        label: "Starbase",
        aliases: &["starbases"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "cur",
        label: "Current",
        aliases: &["currentprod", "production", "current production"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "trs",
        label: "Treasury",
        aliases: &["points", "treasury points"],
        kind: FilterKind::Number,
    },
    TableFilterColumn {
        code: "sco",
        label: "Scout",
        aliases: &["scoutyear"],
        kind: FilterKind::Number,
    },
];

const fn planet_list_sort_code(sort: PlanetListSort) -> &'static str {
    match sort {
        PlanetListSort::Location => "coo",
        PlanetListSort::PlanetName => "pla",
        PlanetListSort::PotentialProduction => "max",
        PlanetListSort::CurrentProduction => "cur",
        PlanetListSort::Treasury => "trs",
        PlanetListSort::Budget => "bdg",
        PlanetListSort::Revenue => "rev",
        PlanetListSort::Growth => "gro",
        PlanetListSort::BuildQueue => "bui",
        PlanetListSort::Stardock => "sta",
        PlanetListSort::Starbase => "sbs",
        PlanetListSort::Armies => "ars",
        PlanetListSort::Batteries => "gbs",
    }
}

fn planet_list_sort_from_column(column: TableFilterColumn) -> Option<PlanetListSort> {
    match column.code {
        "coo" => Some(PlanetListSort::Location),
        "pla" => Some(PlanetListSort::PlanetName),
        "max" => Some(PlanetListSort::PotentialProduction),
        "cur" => Some(PlanetListSort::CurrentProduction),
        "trs" => Some(PlanetListSort::Treasury),
        "bdg" => Some(PlanetListSort::Budget),
        "rev" => Some(PlanetListSort::Revenue),
        "gro" => Some(PlanetListSort::Growth),
        "bui" => Some(PlanetListSort::BuildQueue),
        "sta" => Some(PlanetListSort::Stardock),
        "sbs" => Some(PlanetListSort::Starbase),
        "ars" => Some(PlanetListSort::Armies),
        "gbs" => Some(PlanetListSort::Batteries),
        _ => None,
    }
}

const fn planet_database_sort_code(sort: PlanetDatabaseSort) -> &'static str {
    match sort {
        PlanetDatabaseSort::Location => "coo",
        PlanetDatabaseSort::Range(_) => "rng",
        PlanetDatabaseSort::PlanetName => "pla",
        PlanetDatabaseSort::Owner => "own",
        PlanetDatabaseSort::MaxProduction => "max",
        PlanetDatabaseSort::YearSeen => "see",
        PlanetDatabaseSort::Armies => "ars",
        PlanetDatabaseSort::Batteries => "gbs",
        PlanetDatabaseSort::Starbases => "sbs",
        PlanetDatabaseSort::CurrentProduction => "cur",
        PlanetDatabaseSort::Treasury => "trs",
        PlanetDatabaseSort::ScoutYear => "sco",
    }
}

const fn default_planet_list_sort_direction(sort: PlanetListSort) -> SortDirection {
    match sort {
        PlanetListSort::Location => SortDirection::Asc,
        PlanetListSort::PlanetName => SortDirection::Asc,
        PlanetListSort::PotentialProduction => SortDirection::Desc,
        PlanetListSort::CurrentProduction => SortDirection::Desc,
        PlanetListSort::Treasury => SortDirection::Desc,
        PlanetListSort::Budget => SortDirection::Desc,
        PlanetListSort::Revenue => SortDirection::Desc,
        PlanetListSort::Growth => SortDirection::Desc,
        PlanetListSort::BuildQueue => SortDirection::Desc,
        PlanetListSort::Stardock => SortDirection::Desc,
        PlanetListSort::Starbase => SortDirection::Desc,
        PlanetListSort::Armies => SortDirection::Desc,
        PlanetListSort::Batteries => SortDirection::Desc,
    }
}

const fn default_planet_database_sort_direction(sort: PlanetDatabaseSort) -> SortDirection {
    match sort {
        PlanetDatabaseSort::Location => SortDirection::Asc,
        PlanetDatabaseSort::Range(_) => SortDirection::Asc,
        PlanetDatabaseSort::PlanetName => SortDirection::Asc,
        PlanetDatabaseSort::Owner => SortDirection::Asc,
        PlanetDatabaseSort::MaxProduction => SortDirection::Desc,
        PlanetDatabaseSort::YearSeen => SortDirection::Desc,
        PlanetDatabaseSort::Armies => SortDirection::Desc,
        PlanetDatabaseSort::Batteries => SortDirection::Desc,
        PlanetDatabaseSort::Starbases => SortDirection::Desc,
        PlanetDatabaseSort::CurrentProduction => SortDirection::Desc,
        PlanetDatabaseSort::Treasury => SortDirection::Desc,
        PlanetDatabaseSort::ScoutYear => SortDirection::Desc,
    }
}

fn planet_row_sort_metrics(
    app: &App,
    row: &nc_data::EmpirePlanetEconomyRow,
) -> (u32, u32, u32, u32, u8, u8) {
    let planet = app
        .game_data
        .planets
        .get(row.planet_record_index_1_based.saturating_sub(1));
    let (treasury, budget) = planet_build_view(&app.game_data, row)
        .map(|view| (view.treasury_left, view.points_left))
        .unwrap_or_else(|_| {
            (
                row.stored_production_points,
                u32::from(row.build_capacity).min(row.stored_production_points),
            )
        });
    let build_queue = planet
        .map(|planet| {
            (0..10)
                .map(|slot| {
                    let points = planet.build_queue[slot].points_owed;
                    let kind = nc_data::ProductionItemKind::from_code(
                        planet.build_queue[slot].kind.code(),
                    );
                    kind.build_cost().map(|cost| points / cost).unwrap_or(0)
                })
                .sum()
        })
        .unwrap_or(0);
    let docked = app.planet_list_docked_units(row);
    let armies = planet
        .map(|planet| planet.armies.min(u16::from(u8::MAX)) as u8)
        .unwrap_or(0);
    let batteries = planet
        .map(|planet| planet.ground_batteries.min(u16::from(u8::MAX)) as u8)
        .unwrap_or(0);
    (treasury, budget, build_queue, docked, armies, batteries)
}

fn apply_sort_direction(
    direction: SortDirection,
    ordering: std::cmp::Ordering,
) -> std::cmp::Ordering {
    match direction {
        SortDirection::Asc => ordering,
        SortDirection::Desc => ordering.reverse(),
    }
}

fn planet_list_clause_matches(
    app: &App,
    row: &nc_data::EmpirePlanetEconomyRow,
    clause: &TableFilterClause,
) -> bool {
    let planet = app
        .game_data
        .planets
        .get(row.planet_record_index_1_based.saturating_sub(1));
    let (treasury, budget) = planet_build_view(&app.game_data, row)
        .map(|view| (i64::from(view.treasury_left), i64::from(view.points_left)))
        .unwrap_or_else(|_| {
            (
                i64::from(row.stored_production_points),
                i64::from(u32::from(row.build_capacity).min(row.stored_production_points)),
            )
        });
    match clause.column.code {
        "coo" => clause.predicate.matches_coord(row.coords),
        "pla" => clause.predicate.matches_text(Some(&row.planet_name)),
        "max" => clause
            .predicate
            .matches_number(Some(i64::from(row.potential_production))),
        "cur" => clause
            .predicate
            .matches_number(Some(i64::from(row.present_production))),
        "trs" => clause.predicate.matches_number(Some(treasury)),
        "bdg" => clause.predicate.matches_number(Some(budget)),
        "rev" => clause
            .predicate
            .matches_number(Some(i64::from(row.yearly_tax_revenue))),
        "gro" => clause
            .predicate
            .matches_number(Some(i64::from(row.yearly_growth_delta))),
        "bui" => clause.predicate.matches_number(Some(
            planet
                .map(|planet| {
                    (0..10)
                        .map(|slot| {
                            let points = planet.build_queue[slot].points_owed;
                            let kind = nc_data::ProductionItemKind::from_code(
                                planet.build_queue[slot].kind.code(),
                            );
                            kind.build_cost().map(|cost| points / cost).unwrap_or(0)
                        })
                        .sum::<u32>() as i64
                })
                .unwrap_or(0),
        )),
        "sta" => clause
            .predicate
            .matches_number(Some(i64::from(app.planet_list_docked_units(row)))),
        "sbs" => clause
            .predicate
            .matches_number(Some(i64::from(u8::from(row.has_friendly_starbase)))),
        "ars" => clause.predicate.matches_number(
            planet.map(|planet| i64::from(planet.armies.min(u16::from(u8::MAX)) as u8)),
        ),
        "gbs" => clause.predicate.matches_number(
            planet.map(|planet| i64::from(planet.ground_batteries.min(u16::from(u8::MAX)) as u8)),
        ),
        _ => true,
    }
}

fn parse_unknown_i64(label: &str) -> Option<i64> {
    if label.trim() == "?" {
        None
    } else {
        label.trim().parse::<i64>().ok()
    }
}

fn planet_database_clause_matches(row: &PlanetDatabaseRow, clause: &TableFilterClause) -> bool {
    match clause.column.code {
        "coo" => clause.predicate.matches_coord(row.coords),
        "pla" => match &clause.predicate {
            TableFilterPredicate::Unknown => row.name_label.trim() == "?",
            predicate => predicate.matches_text(Some(&row.name_label)),
        },
        "own" => clause
            .predicate
            .matches_text(if row.known_owner_empire_id.is_some() {
                Some(&row.owner_label)
            } else {
                None
            }),
        "max" => clause
            .predicate
            .matches_number(row.known_max_production.map(i64::from)),
        "see" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.year_seen_label)),
        "ars" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.armies_label)),
        "gbs" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.batteries_label)),
        "sbs" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.starbase_count_label)),
        "cur" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.current_prod_label)),
        "trs" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.stored_points_label)),
        "sco" => clause
            .predicate
            .matches_number(parse_unknown_i64(&row.year_scout_label)),
        _ => true,
    }
}

impl App {
    pub(crate) fn planet_database_visible_rows(&self) -> usize {
        crate::screen::layout::stacked_table_visible_rows_for(self.screen_geometry, 1)
    }

    fn planet_brief_visible_rows(&self) -> usize {
        crate::screen::layout::stacked_table_visible_rows_for(self.screen_geometry, 1)
    }

    fn command_menu_for_planet_list_mode(mode: PlanetListMode) -> CommandMenu {
        match mode {
            PlanetListMode::Brief | PlanetListMode::Stub(_) => CommandMenu::Planet,
            PlanetListMode::BuildSelect => CommandMenu::PlanetBuild,
        }
    }

    pub(crate) fn planet_context_screen(&self) -> ScreenId {
        match self.planet.command_context {
            PlanetCommandContext::Menu => ScreenId::PlanetMenu,
            PlanetCommandContext::List => {
                ScreenId::PlanetList(PlanetListMode::Brief, self.planet.list_sort)
            }
        }
    }

    pub(crate) fn clear_planet_list_status(&mut self) {
        self.planet.list_prompt_status = None;
    }

    pub(crate) fn show_planet_context_notice(&mut self, message: impl Into<String>) {
        match self.planet.command_context {
            PlanetCommandContext::Menu => {
                self.show_command_menu_notice(CommandMenu::Planet, message)
            }
            PlanetCommandContext::List => {
                self.clear_command_menu_notice();
                self.planet.list_prompt_status = Some(message.into());
                self.current_screen = self.planet_context_screen();
            }
        }
    }

    pub fn open_planet_menu(&mut self) {
        self.clear_command_menu_notice();
        self.clear_planet_list_status();
        self.close_planet_auto_commission_prompt();
        self.clear_planet_auto_commission_report();
        self.close_planet_tax_prompt();
        self.close_planet_build_abort_prompt();
        self.clear_planet_scorch_prompt();
        self.clear_planet_transport_prompt();
        self.planet.command_context = PlanetCommandContext::Menu;
        self.planet.build_return_to_list = false;
        self.command_return_menu = CommandMenu::Planet;
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub fn open_planet_tax_prompt(&mut self) {
        self.clear_command_menu_notice();
        self.close_planet_auto_commission_prompt();
        self.close_planet_info_prompt();
        self.planet.tax_prompt_active = true;
        self.planet.tax_input = String::new();
        self.planet.tax_error = None;
        self.planet.tax_notice = None;
        self.current_screen = ScreenId::PlanetMenu;
    }

    pub fn close_planet_tax_prompt(&mut self) {
        self.planet.tax_prompt_active = false;
        self.planet.tax_input.clear();
        self.planet.tax_error = None;
        self.planet.tax_notice = None;
    }

    pub fn open_planet_auto_commission_prompt(&mut self) {
        if self.commission_planet_rows().is_empty() {
            self.show_planet_context_notice("No ships or starbases are waiting in stardock.");
            return;
        }
        if matches!(
            self.current_screen,
            ScreenId::PlanetList(PlanetListMode::Brief, _)
        ) {
            self.planet.command_context = PlanetCommandContext::List;
            self.clear_planet_list_status();
        }
        self.clear_command_menu_notice();
        self.close_planet_tax_prompt();
        self.close_planet_info_prompt();
        self.clear_planet_auto_commission_report();
        self.planet.auto_commission_prompt_active = true;
        self.current_screen = self.planet_context_screen();
    }

    pub fn close_planet_auto_commission_prompt(&mut self) {
        self.planet.auto_commission_prompt_active = false;
    }

    pub fn clear_planet_auto_commission_report(&mut self) {
        self.planet.auto_commission_report_rows.clear();
        self.planet.auto_commission_report_revealed_rows = 0;
    }

    pub fn open_planet_database(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::PlanetDatabaseList
                | ScreenId::PlanetDatabaseFilterPrompt
                | ScreenId::PlanetDatabaseSortPrompt
        ) {
            self.command_return_menu = self.origin_command_menu();
            let default_index = 0usize;
            self.planet.database_cursor = default_index;
            self.planet.database_scroll_offset =
                default_index.saturating_sub(self.planet_database_visible_rows() / 2);
            self.planet.database_input.clear();
            self.planet.database_prompt_default_value.clear();
            self.planet.database_pending_range_anchor = None;
            self.planet.database_status = None;
            self.planet.database_filter = PlanetDatabaseFilter::All;
            self.planet.database_filter_clause = None;
            self.planet.database_pending_column = None;
            self.planet.database_sort = PlanetDatabaseSort::Location;
        }
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.planet.database_status = None;
        self.current_screen = ScreenId::PlanetDatabaseList;
    }

    pub fn open_planet_database_filter_prompt(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value = "all".to_string();
        self.planet.database_pending_range_anchor = None;
        self.planet.database_status = None;
        self.planet.database_prompt_dismiss_message = None;
        self.planet.database_pending_column = None;
        self.current_screen = ScreenId::PlanetDatabaseFilterPrompt;
    }

    pub fn open_planet_database_sort_prompt(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::SortMenu;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value =
            planet_database_sort_code(self.planet.database_sort).to_string();
        self.planet.database_pending_range_anchor = None;
        self.planet.database_status = None;
        self.planet.database_prompt_dismiss_message = None;
        self.current_screen = ScreenId::PlanetDatabaseSortPrompt;
    }

    pub fn open_planet_database_detail(&mut self) {
        let rows = self.planet_database_rows();
        let total = rows.len();
        if total == 0 {
            self.current_screen = ScreenId::PlanetDatabaseList;
            return;
        }
        let coords = rows[self.planet.database_cursor.min(total - 1)].coords;
        let _ = self.open_planet_info_detail_at_coords(coords, Some(ScreenId::PlanetDatabaseList));
    }

    pub(crate) fn enforce_valid_planet_list_filter(&mut self) {
        if self.planet.list_filter == PlanetListFilter::All
            && self.planet.list_filter_clause.is_none()
        {
            return;
        }
        if !self
            .planet_list_rows(PlanetListMode::Brief, self.planet.list_sort)
            .is_empty()
        {
            return;
        }

        let previous_filter = self.planet.list_filter;
        let previous_clause = self.planet.list_filter_clause.clone();
        self.planet.list_filter = PlanetListFilter::All;
        self.planet.list_filter_clause = None;
        if self
            .planet_list_rows(PlanetListMode::Brief, self.planet.list_sort)
            .is_empty()
        {
            self.planet.list_filter = previous_filter;
            self.planet.list_filter_clause = previous_clause;
            return;
        }

        self.planet.brief_cursor = 0;
        self.planet.brief_scroll_offset = 0;
    }

    pub(crate) fn enforce_valid_planet_database_filter(&mut self) {
        if self.planet.database_filter == PlanetDatabaseFilter::All
            && self.planet.database_filter_clause.is_none()
        {
            return;
        }
        if !self.planet_database_rows().is_empty() {
            return;
        }

        let previous_filter = self.planet.database_filter;
        let previous_clause = self.planet.database_filter_clause.clone();
        self.planet.database_filter = PlanetDatabaseFilter::All;
        self.planet.database_filter_clause = None;
        if self.planet_database_rows().is_empty() {
            self.planet.database_filter = previous_filter;
            self.planet.database_filter_clause = previous_clause;
            return;
        }

        self.planet.database_cursor = 0;
        self.planet.database_scroll_offset = 0;
    }

    pub fn open_planet_list_sort_prompt(&mut self, mode: PlanetListMode) {
        self.enforce_valid_planet_list_filter();
        if self
            .planet_list_rows(mode, PlanetListSort::Location)
            .is_empty()
        {
            self.show_command_menu_notice(
                Self::command_menu_for_planet_list_mode(mode),
                "You do not currently control any planets.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.clear_planet_list_status();
        self.planet.list_prompt_input.clear();
        self.planet.list_prompt_default_value =
            planet_list_sort_code(self.planet.list_sort).to_string();
        self.planet.list_prompt_dismiss_message = None;
        self.current_screen = ScreenId::PlanetListSortPrompt(mode);
    }

    pub fn open_planet_list_filter_prompt(&mut self, mode: PlanetListMode) {
        if self
            .game_data
            .empire_planet_economy_rows(self.player.record_index_1_based)
            .is_empty()
        {
            self.show_command_menu_notice(
                Self::command_menu_for_planet_list_mode(mode),
                "You do not currently control any planets.",
            );
            return;
        }
        self.clear_command_menu_notice();
        self.clear_planet_list_status();
        self.planet.list_prompt_input.clear();
        self.planet.list_prompt_default_value = "all".to_string();
        self.planet.list_pending_range_anchor = None;
        self.planet.list_prompt_dismiss_message = None;
        self.planet.list_filter_pending_column = None;
        self.planet.list_filter_prompt_mode = PlanetListFilterPromptMode::FilterMenu;
        self.current_screen = ScreenId::PlanetListFilterPrompt(mode);
    }

    pub fn dismiss_planet_list_filter_prompt_notice(&mut self) {
        self.planet.list_prompt_dismiss_message = None;
    }

    pub fn dismiss_planet_database_filter_prompt_notice(&mut self) {
        self.planet.database_prompt_dismiss_message = None;
    }

    pub fn submit_planet_list_sort(&mut self, mode: PlanetListMode, sort: PlanetListSort) {
        let total = self.planet_list_rows(mode, sort).len();
        if total == 0 {
            self.show_command_menu_notice(
                Self::command_menu_for_planet_list_mode(mode),
                "You do not currently control any planets.",
            );
            return;
        }
        self.clear_command_menu_notice();
        if self.current_screen == ScreenId::PlanetListSortPrompt(mode)
            && self.planet.list_sort == sort
        {
            self.planet.list_sort_direction = self.planet.list_sort_direction.toggle();
        } else {
            self.planet.list_sort = sort;
            self.planet.list_sort_direction = default_planet_list_sort_direction(sort);
        }
        self.clear_planet_list_status();
        self.planet.brief_scroll_offset = 0;
        self.planet.brief_cursor = 0;
        self.planet.brief_input.clear();
        self.planet.list_prompt_input.clear();
        self.planet.list_prompt_default_value.clear();
        self.planet.list_pending_range_anchor = None;
        self.planet.list_prompt_dismiss_message = None;
        self.planet.list_filter_pending_column = None;
        self.current_screen = match mode {
            PlanetListMode::Brief | PlanetListMode::BuildSelect => {
                if mode == PlanetListMode::Brief {
                    self.planet.command_context = PlanetCommandContext::List;
                }
                self.select_planet_brief_origin_row(mode, sort);
                ScreenId::PlanetList(mode, sort)
            }
            PlanetListMode::Stub(_) => ScreenId::PlanetMenu,
        };
    }

    pub fn submit_planet_list_sort_prompt(&mut self, mode: PlanetListMode) {
        if self.current_screen != ScreenId::PlanetListSortPrompt(mode) {
            return;
        }
        let raw = if self.planet.list_prompt_input.trim().is_empty() {
            self.planet.list_prompt_default_value.trim()
        } else {
            self.planet.list_prompt_input.trim()
        };
        match parse_column_code(PLANET_LIST_FILTER_COLUMNS, raw) {
            Ok(column) => match planet_list_sort_from_column(column) {
                Some(sort) => self.submit_planet_list_sort(mode, sort),
                None => {
                    self.planet.list_prompt_input.clear();
                    self.planet.list_prompt_status = None;
                    self.planet.list_prompt_dismiss_message =
                        Some("Enter a valid sort column.".to_string());
                }
            },
            Err(ColumnCodeParseError::Ambiguous(codes)) => {
                self.planet.list_prompt_input.clear();
                self.planet.list_prompt_status = Some(format!(
                    " {}",
                    format_column_code_error(&ColumnCodeParseError::Ambiguous(codes))
                ));
                self.planet.list_prompt_dismiss_message = None;
            }
            Err(ColumnCodeParseError::Unknown) => {
                self.planet.list_prompt_input.clear();
                self.planet.list_prompt_status = None;
                self.planet.list_prompt_dismiss_message =
                    Some("Enter a valid sort column.".to_string());
            }
        }
    }

    pub fn submit_planet_list_filter(
        &mut self,
        mode: PlanetListMode,
        filter_mode: PlanetListFilterMode,
    ) {
        if self.current_screen != ScreenId::PlanetListFilterPrompt(mode) {
            return;
        }
        self.planet.list_filter_clause = None;
        match self.planet.list_filter_prompt_mode {
            PlanetListFilterPromptMode::FilterMenu => match filter_mode {
                PlanetListFilterMode::All => {
                    self.apply_planet_list_filter(mode, PlanetListFilter::All)
                }
                PlanetListFilterMode::Range => {
                    let column = PLANET_LIST_FILTER_COLUMNS
                        .iter()
                        .copied()
                        .find(|column| column.code == "coo")
                        .expect("coo filter column");
                    self.planet.list_filter_pending_column = Some(column);
                    self.planet.list_filter_prompt_mode = PlanetListFilterPromptMode::ValueInput;
                    self.planet.list_prompt_input.clear();
                    self.planet.list_prompt_default_value = format!(
                        "{},{}",
                        self.default_planet_prompt_coords()[0],
                        self.default_planet_prompt_coords()[1]
                    );
                    self.planet.list_prompt_status = None;
                }
                PlanetListFilterMode::Starbase => {
                    self.apply_planet_list_filter(mode, PlanetListFilter::Starbase)
                }
                PlanetListFilterMode::Stardock => {
                    self.apply_planet_list_filter(mode, PlanetListFilter::Stardock)
                }
            },
            PlanetListFilterPromptMode::ValueInput => {
                self.submit_planet_list_filter_prompt(mode);
            }
        }
    }

    pub fn submit_planet_list_filter_prompt(&mut self, mode: PlanetListMode) {
        if self.current_screen != ScreenId::PlanetListFilterPrompt(mode) {
            return;
        }
        match self.planet.list_filter_prompt_mode {
            PlanetListFilterPromptMode::FilterMenu => {
                let raw = if self.planet.list_prompt_input.trim().is_empty() {
                    self.planet.list_prompt_default_value.trim()
                } else {
                    self.planet.list_prompt_input.trim()
                };
                if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                    self.apply_planet_list_filter_clause(mode, None);
                    return;
                }
                match parse_column_code(PLANET_LIST_FILTER_COLUMNS, raw) {
                    Ok(column) => {
                        self.planet.list_filter_pending_column = Some(column);
                        self.planet.list_filter_prompt_mode =
                            PlanetListFilterPromptMode::ValueInput;
                        self.planet.list_prompt_input.clear();
                        self.planet.list_prompt_default_value =
                            self.planet_list_filter_default_value(mode, column);
                        self.planet.list_prompt_status = None;
                        self.planet.list_prompt_dismiss_message = None;
                    }
                    Err(ColumnCodeParseError::Ambiguous(codes)) => {
                        self.planet.list_prompt_input.clear();
                        self.planet.list_prompt_status = Some(format!(
                            " {}",
                            format_column_code_error(&ColumnCodeParseError::Ambiguous(codes))
                        ));
                        self.planet.list_prompt_dismiss_message = None;
                    }
                    Err(ColumnCodeParseError::Unknown) => {
                        self.planet.list_prompt_input.clear();
                        self.planet.list_prompt_status = None;
                        self.planet.list_prompt_dismiss_message =
                            Some("Enter a valid column name/code or ALL".to_string());
                    }
                }
            }
            PlanetListFilterPromptMode::ValueInput => {
                let Some(column) = self.planet.list_filter_pending_column else {
                    self.planet.list_prompt_status = Some("Enter a column code first.".to_string());
                    self.planet.list_filter_prompt_mode = PlanetListFilterPromptMode::FilterMenu;
                    return;
                };
                let raw = if self.planet.list_prompt_input.trim().is_empty() {
                    self.planet.list_prompt_default_value.trim()
                } else {
                    self.planet.list_prompt_input.trim()
                };
                match parse_filter_clause(column, raw) {
                    Ok(clause) => self.apply_planet_list_filter_clause(mode, Some(clause)),
                    Err(err) => {
                        self.planet.list_prompt_status = Some(err);
                        self.planet.list_prompt_dismiss_message = None;
                    }
                }
            }
        }
    }

    pub fn close_planet_list_sort_prompt(&mut self, mode: PlanetListMode) {
        self.clear_planet_list_status();
        self.planet.list_prompt_input.clear();
        self.planet.list_prompt_default_value.clear();
        self.planet.list_prompt_dismiss_message = None;
        if mode == PlanetListMode::BuildSelect {
            self.select_planet_brief_origin_row(mode, self.planet.list_sort);
        }
        self.current_screen = match mode {
            PlanetListMode::Brief | PlanetListMode::BuildSelect => {
                ScreenId::PlanetList(mode, self.planet.list_sort)
            }
            PlanetListMode::Stub(_) => ScreenId::PlanetMenu,
        };
    }

    pub fn close_planet_list_filter_prompt(&mut self, mode: PlanetListMode) {
        self.clear_planet_list_status();
        self.planet.list_filter_prompt_mode = PlanetListFilterPromptMode::FilterMenu;
        self.planet.list_prompt_input.clear();
        self.planet.list_prompt_default_value.clear();
        self.planet.list_pending_range_anchor = None;
        self.planet.list_prompt_dismiss_message = None;
        self.planet.list_filter_pending_column = None;
        self.current_screen = ScreenId::PlanetList(mode, self.planet.list_sort);
    }

    pub fn scroll_planet_brief(&mut self, delta: i8) {
        let ScreenId::PlanetList(mode, sort) = self.current_screen else {
            return;
        };
        let total = self.planet_list_rows(mode, sort).len();
        let max_offset = total.saturating_sub(self.planet_brief_visible_rows());
        self.planet.brief_scroll_offset = self
            .planet
            .brief_scroll_offset
            .saturating_add_signed(delta as isize)
            .min(max_offset);
    }

    pub fn move_planet_brief_cursor(&mut self, delta: i8) {
        let ScreenId::PlanetList(mode, sort) = self.current_screen else {
            return;
        };
        let total = self.planet_list_rows(mode, sort).len();
        if total == 0 {
            self.planet.brief_cursor = 0;
            return;
        }
        let visible_rows = self.planet_brief_visible_rows();
        let next = self.planet.brief_cursor as isize + delta as isize;
        self.planet.brief_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
        self.clear_planet_list_status();
    }

    pub fn append_planet_brief_char(&mut self, ch: char) {
        let ScreenId::PlanetList(_, sort) = self.current_screen else {
            return;
        };
        if self.planet.brief_input.len() < 16 && is_coordinate_input_char(ch) {
            self.planet.brief_input.push(ch);
            if self.sync_planet_brief_cursor_to_input(sort) {
                self.planet.brief_input.clear();
            }
            self.clear_planet_list_status();
        }
    }

    pub fn backspace_planet_brief_input(&mut self) {
        let ScreenId::PlanetList(_, sort) = self.current_screen else {
            return;
        };
        self.planet.brief_input.pop();
        let _ = self.sync_planet_brief_cursor_to_input(sort);
        self.clear_planet_list_status();
    }

    pub fn append_planet_list_prompt_char(&mut self, ch: char) {
        if !matches!(
            self.current_screen,
            ScreenId::PlanetListFilterPrompt(_) | ScreenId::PlanetListSortPrompt(_)
        ) {
            return;
        }
        if self.planet.list_prompt_input.len() >= 16 {
            return;
        }
        self.planet.list_prompt_input.push(ch);
        self.clear_planet_list_status();
        self.planet.list_prompt_dismiss_message = None;
    }

    pub fn backspace_planet_list_prompt_input(&mut self) {
        if !matches!(
            self.current_screen,
            ScreenId::PlanetListFilterPrompt(_) | ScreenId::PlanetListSortPrompt(_)
        ) {
            return;
        }
        self.planet.list_prompt_input.pop();
        self.clear_planet_list_status();
        self.planet.list_prompt_dismiss_message = None;
    }

    pub fn submit_planet_brief_input(&mut self) {
        let ScreenId::PlanetList(mode, sort) = self.current_screen else {
            return;
        };
        let rows = self.planet_list_rows(mode, sort);
        if rows.is_empty() {
            return;
        }

        let default_coords = rows
            .get(self.planet.brief_cursor)
            .map(|row| row.coords)
            .unwrap_or([0, 0]);

        if self.planet.brief_input.trim().is_empty() {
            let coords = rows[self.planet.brief_cursor.min(rows.len() - 1)].coords;
            match mode {
                PlanetListMode::Brief => {
                    let _ = self.open_planet_info_detail_at_coords(
                        coords,
                        Some(ScreenId::PlanetList(mode, sort)),
                    );
                }
                PlanetListMode::BuildSelect => {
                    let _ = self.open_planet_build_menu_at_coords(coords);
                }
                PlanetListMode::Stub(_) => {}
            }
            return;
        }

        let Some(coords) = resolve_default_coords_input(&self.planet.brief_input, default_coords)
        else {
            self.planet.list_prompt_status = Some("Enter coordinates like 5,2".to_string());
            return;
        };

        let Some(index) = rows.iter().position(|row| row.coords == coords) else {
            self.planet.list_prompt_status = Some(format!(
                "No world found at ({:02},{:02})",
                coords[0], coords[1]
            ));
            return;
        };

        self.planet.brief_cursor = index;
        let visible_rows = self.planet_brief_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
        self.planet.brief_input.clear();
        self.planet.list_prompt_status = None;
        match mode {
            PlanetListMode::Brief => {
                let _ = self.open_planet_info_detail_at_coords(
                    coords,
                    Some(ScreenId::PlanetList(mode, sort)),
                );
            }
            PlanetListMode::BuildSelect => {
                let _ = self.open_planet_build_menu_at_coords(coords);
            }
            PlanetListMode::Stub(_) => {}
        }
    }

    pub fn move_planet_database_list(&mut self, delta: i8) {
        self.move_planet_database_list_by(delta as isize);
    }

    pub fn move_planet_database_list_by(&mut self, delta: isize) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        let total = self.planet_database_rows().len();
        if total == 0 {
            self.planet.database_cursor = 0;
            return;
        }
        let visible_rows = self.planet_database_visible_rows();
        let next = self.planet.database_cursor as isize + delta;
        self.planet.database_cursor = next.rem_euclid(total as isize) as usize;
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
    }

    pub fn append_planet_database_char(&mut self, ch: char) {
        let accepts_input = match self.current_screen {
            ScreenId::PlanetDatabaseList => true,
            ScreenId::PlanetDatabaseFilterPrompt | ScreenId::PlanetDatabaseSortPrompt => matches!(
                self.planet.database_prompt_mode,
                PlanetDatabasePromptMode::FilterMenu
                    | PlanetDatabasePromptMode::FilterValueInput
                    | PlanetDatabasePromptMode::SortMenu
                    | PlanetDatabasePromptMode::SortRangeInput
            ),
            _ => false,
        };
        let allow_char = match self.current_screen {
            ScreenId::PlanetDatabaseList => is_coordinate_input_char(ch),
            ScreenId::PlanetDatabaseFilterPrompt | ScreenId::PlanetDatabaseSortPrompt => {
                match self.planet.database_prompt_mode {
                    PlanetDatabasePromptMode::SortRangeInput => is_coordinate_input_char(ch),
                    PlanetDatabasePromptMode::FilterMenu | PlanetDatabasePromptMode::SortMenu => {
                        is_filter_column_char(ch)
                    }
                    PlanetDatabasePromptMode::FilterValueInput => {
                        matches!(
                            ch,
                            ' ' | '-' | '#' | '*' | '/' | '?' | '=' | '!' | '>' | '<' | '+' | ','
                        ) || ch.is_ascii_alphanumeric()
                    }
                }
            }
            _ => false,
        };
        if accepts_input && self.planet.database_input.len() < 16 && allow_char {
            self.planet.database_input.push(ch);
            if self.current_screen == ScreenId::PlanetDatabaseList {
                if self.sync_planet_database_cursor_to_input() {
                    self.planet.database_input.clear();
                }
            }
            self.planet.database_status = None;
            self.planet.database_prompt_dismiss_message = None;
        }
    }

    pub fn backspace_planet_database_input(&mut self) {
        let accepts_input = match self.current_screen {
            ScreenId::PlanetDatabaseList => true,
            ScreenId::PlanetDatabaseFilterPrompt | ScreenId::PlanetDatabaseSortPrompt => matches!(
                self.planet.database_prompt_mode,
                PlanetDatabasePromptMode::FilterMenu
                    | PlanetDatabasePromptMode::FilterValueInput
                    | PlanetDatabasePromptMode::SortMenu
                    | PlanetDatabasePromptMode::SortRangeInput
            ),
            _ => false,
        };
        if !accepts_input {
            return;
        }
        self.planet.database_input.pop();
        if self.current_screen == ScreenId::PlanetDatabaseList {
            let _ = self.sync_planet_database_cursor_to_input();
        }
        self.planet.database_status = None;
        self.planet.database_prompt_dismiss_message = None;
    }

    pub fn submit_planet_database_lookup(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseList {
            return;
        }
        let rows = self.planet_database_rows();
        if self.planet.database_input.trim().is_empty() {
            self.open_planet_database_detail();
            return;
        }
        let Some(coords) = resolve_default_coords_input(
            &self.planet.database_input,
            self.default_planet_database_coords(),
        ) else {
            self.planet.database_status = Some("Enter coordinates like 5,2".to_string());
            return;
        };
        let Some(index) = rows.iter().position(|row| row.coords == coords) else {
            self.planet.database_status =
                Some(format!("No world found at [{},{}]", coords[0], coords[1]));
            return;
        };
        self.planet.database_cursor = index;
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
        self.planet.database_status = None;
        self.planet.database_input.clear();
        self.open_planet_database_detail();
    }

    pub fn submit_planet_database_filter(&mut self, mode: PlanetDatabaseFilterMode) {
        if self.current_screen != ScreenId::PlanetDatabaseFilterPrompt {
            return;
        }
        self.planet.database_filter_clause = None;
        match self.planet.database_prompt_mode {
            PlanetDatabasePromptMode::FilterMenu => match mode {
                PlanetDatabaseFilterMode::All => {
                    self.apply_planet_database_filter(PlanetDatabaseFilter::All);
                }
                PlanetDatabaseFilterMode::Range => {
                    let column = PLANET_DATABASE_FILTER_COLUMNS
                        .iter()
                        .copied()
                        .find(|column| column.code == "coo")
                        .expect("coo filter column");
                    self.planet.database_pending_column = Some(column);
                    self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterValueInput;
                    self.planet.database_input.clear();
                    self.planet.database_prompt_default_value =
                        self.planet_database_filter_default_value(column);
                    self.planet.database_status = None;
                }
                PlanetDatabaseFilterMode::Empire => {
                    let column = PLANET_DATABASE_FILTER_COLUMNS
                        .iter()
                        .copied()
                        .find(|column| column.code == "own")
                        .expect("own filter column");
                    self.planet.database_pending_column = Some(column);
                    self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterValueInput;
                    self.planet.database_input.clear();
                    self.planet.database_prompt_default_value =
                        self.planet_database_filter_default_value(column);
                    self.planet.database_status = None;
                }
                PlanetDatabaseFilterMode::MaxProduction => {
                    let column = PLANET_DATABASE_FILTER_COLUMNS
                        .iter()
                        .copied()
                        .find(|column| column.code == "max")
                        .expect("max filter column");
                    self.planet.database_pending_column = Some(column);
                    self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterValueInput;
                    self.planet.database_input.clear();
                    self.planet.database_prompt_default_value =
                        self.planet_database_filter_default_value(column);
                    self.planet.database_status = None;
                }
            },
            PlanetDatabasePromptMode::FilterValueInput => {
                self.submit_planet_database_filter_prompt();
            }
            PlanetDatabasePromptMode::SortMenu | PlanetDatabasePromptMode::SortRangeInput => {}
        }
    }

    pub fn submit_planet_database_filter_prompt(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseFilterPrompt {
            return;
        }
        match self.planet.database_prompt_mode {
            PlanetDatabasePromptMode::FilterMenu => {
                let raw = if self.planet.database_input.trim().is_empty() {
                    self.planet.database_prompt_default_value.trim()
                } else {
                    self.planet.database_input.trim()
                };
                if raw.eq_ignore_ascii_case("a") || raw.eq_ignore_ascii_case("all") {
                    self.apply_planet_database_filter_clause(None);
                    return;
                }
                match parse_column_code(PLANET_DATABASE_FILTER_COLUMNS, raw) {
                    Ok(column) => {
                        self.planet.database_pending_column = Some(column);
                        self.planet.database_prompt_mode =
                            PlanetDatabasePromptMode::FilterValueInput;
                        self.planet.database_input.clear();
                        self.planet.database_prompt_default_value =
                            self.planet_database_filter_default_value(column);
                        self.planet.database_status = None;
                        self.planet.database_prompt_dismiss_message = None;
                    }
                    Err(ColumnCodeParseError::Ambiguous(codes)) => {
                        self.planet.database_input.clear();
                        self.planet.database_status = Some(format!(
                            " {}",
                            format_column_code_error(&ColumnCodeParseError::Ambiguous(codes))
                        ));
                        self.planet.database_prompt_dismiss_message = None;
                    }
                    Err(ColumnCodeParseError::Unknown) => {
                        self.planet.database_input.clear();
                        self.planet.database_status = None;
                        self.planet.database_prompt_dismiss_message =
                            Some("Enter a valid column name/code or ALL".to_string());
                    }
                }
            }
            PlanetDatabasePromptMode::FilterValueInput => {
                let Some(column) = self.planet.database_pending_column else {
                    self.planet.database_status = Some("Enter a column code first.".to_string());
                    self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
                    return;
                };
                let raw = if self.planet.database_input.trim().is_empty() {
                    self.planet.database_prompt_default_value.trim()
                } else {
                    self.planet.database_input.trim()
                };
                match parse_filter_clause(column, raw) {
                    Ok(clause) => self.apply_planet_database_filter_clause(Some(clause)),
                    Err(err) => {
                        self.planet.database_status = Some(err);
                        self.planet.database_prompt_dismiss_message = None;
                    }
                }
            }
            PlanetDatabasePromptMode::SortMenu | PlanetDatabasePromptMode::SortRangeInput => {}
        }
    }

    pub fn submit_planet_database_sort(&mut self, mode: PlanetDatabaseSortMode) {
        if self.current_screen != ScreenId::PlanetDatabaseSortPrompt {
            return;
        }
        match self.planet.database_prompt_mode {
            PlanetDatabasePromptMode::SortMenu => match mode {
                PlanetDatabaseSortMode::Location => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::Location);
                }
                PlanetDatabaseSortMode::Range => {
                    self.planet.database_prompt_mode = PlanetDatabasePromptMode::SortRangeInput;
                    self.planet.database_input.clear();
                    let default = match self.planet.database_sort {
                        PlanetDatabaseSort::Range(anchor) => anchor,
                        _ => self.default_planet_database_coords(),
                    };
                    self.planet.database_prompt_default_value =
                        format!("{:02},{:02}", default[0], default[1]);
                    self.planet.database_status = None;
                    self.planet.database_prompt_dismiss_message = None;
                }
                PlanetDatabaseSortMode::PlanetName => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::PlanetName);
                }
                PlanetDatabaseSortMode::Owner => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::Owner);
                }
                PlanetDatabaseSortMode::MaxProduction => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::MaxProduction);
                }
                PlanetDatabaseSortMode::YearSeen => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::YearSeen);
                }
                PlanetDatabaseSortMode::Armies => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::Armies);
                }
                PlanetDatabaseSortMode::Batteries => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::Batteries);
                }
                PlanetDatabaseSortMode::Starbases => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::Starbases);
                }
                PlanetDatabaseSortMode::CurrentProduction => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::CurrentProduction);
                }
                PlanetDatabaseSortMode::Treasury => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::Treasury);
                }
                PlanetDatabaseSortMode::ScoutYear => {
                    self.apply_planet_database_sort(PlanetDatabaseSort::ScoutYear);
                }
            },
            PlanetDatabasePromptMode::SortRangeInput => {
                let default_coords = resolve_default_coords_input(
                    &self.planet.database_prompt_default_value,
                    self.default_planet_database_coords(),
                )
                .unwrap_or_else(|| self.default_planet_database_coords());
                let Some(coords) =
                    resolve_default_coords_input(self.planet.database_input.trim(), default_coords)
                else {
                    self.planet.database_status = Some("Enter coordinates like 5,2".to_string());
                    return;
                };
                self.apply_planet_database_sort(PlanetDatabaseSort::Range(coords));
            }
            _ => {}
        }
    }

    pub fn submit_planet_database_sort_prompt(&mut self) {
        if self.current_screen != ScreenId::PlanetDatabaseSortPrompt
            || self.planet.database_prompt_mode != PlanetDatabasePromptMode::SortMenu
        {
            return;
        }
        let raw = if self.planet.database_input.trim().is_empty() {
            self.planet.database_prompt_default_value.trim()
        } else {
            self.planet.database_input.trim()
        };
        if raw.eq_ignore_ascii_case("rng") || raw.eq_ignore_ascii_case("range") {
            self.submit_planet_database_sort(PlanetDatabaseSortMode::Range);
            return;
        }
        match parse_column_code(PLANET_DATABASE_FILTER_COLUMNS, raw) {
            Ok(column) => {
                let mode = match column.code {
                    "coo" => Some(PlanetDatabaseSortMode::Location),
                    "pla" => Some(PlanetDatabaseSortMode::PlanetName),
                    "own" => Some(PlanetDatabaseSortMode::Owner),
                    "max" => Some(PlanetDatabaseSortMode::MaxProduction),
                    "see" => Some(PlanetDatabaseSortMode::YearSeen),
                    "ars" => Some(PlanetDatabaseSortMode::Armies),
                    "gbs" => Some(PlanetDatabaseSortMode::Batteries),
                    "sbs" => Some(PlanetDatabaseSortMode::Starbases),
                    "cur" => Some(PlanetDatabaseSortMode::CurrentProduction),
                    "trs" => Some(PlanetDatabaseSortMode::Treasury),
                    "sco" => Some(PlanetDatabaseSortMode::ScoutYear),
                    _ => None,
                };
                if let Some(mode) = mode {
                    self.submit_planet_database_sort(mode);
                } else {
                    self.planet.database_input.clear();
                    self.planet.database_status = None;
                    self.planet.database_prompt_dismiss_message =
                        Some("Enter a valid sort column or RNG.".to_string());
                }
            }
            Err(ColumnCodeParseError::Ambiguous(codes)) => {
                self.planet.database_input.clear();
                self.planet.database_status = Some(format!(
                    " {}",
                    format_column_code_error(&ColumnCodeParseError::Ambiguous(codes))
                ));
                self.planet.database_prompt_dismiss_message = None;
            }
            Err(ColumnCodeParseError::Unknown) => {
                self.planet.database_input.clear();
                self.planet.database_status = None;
                self.planet.database_prompt_dismiss_message =
                    Some("Enter a valid sort column or RNG.".to_string());
            }
        }
    }

    pub fn append_planet_tax_char(&mut self, ch: char) {
        if self.inline_planet_tax_active_on_current_screen() && self.planet.tax_input.len() < 3 {
            self.planet.tax_input.push(ch);
            self.planet.tax_error = None;
            self.planet.tax_notice = None;
        }
    }

    pub fn backspace_planet_tax_input(&mut self) {
        if self.inline_planet_tax_active_on_current_screen() {
            self.planet.tax_input.pop();
            self.planet.tax_error = None;
            self.planet.tax_notice = None;
        }
    }

    pub fn submit_planet_tax(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.inline_planet_tax_active_on_current_screen() {
            return Ok(());
        }
        let raw = self.planet.tax_input.trim();
        let parsed = if raw.is_empty() {
            self.game_data.players[self.player.record_index_1_based - 1].tax_rate
        } else {
            match raw.parse::<u8>() {
                Ok(value) => value,
                Err(_) => {
                    self.planet.tax_error =
                        Some("Enter an integer tax rate from 0 to 100.".to_string());
                    return Ok(());
                }
            }
        };
        if parsed > 100 {
            self.planet.tax_error = Some("Enter an integer tax rate from 0 to 100.".to_string());
            return Ok(());
        }
        self.game_data
            .set_player_tax_rate(self.player.record_index_1_based, parsed)?;
        self.save_game_data()?;
        self.close_planet_tax_prompt();
        self.show_command_menu_notice(
            CommandMenu::Planet,
            format!("Empire tax rate set to {parsed}%."),
        );
        Ok(())
    }

    pub fn open_planet_info_prompt(&mut self, menu: CommandMenu) {
        self.close_planet_tax_prompt();
        self.close_planet_auto_commission_prompt();
        self.close_planet_build_abort_prompt();
        self.messaging.delete_reviewables_prompt_active = false;
        if menu == CommandMenu::PlanetBuild {
            self.planet.build_status = None;
        }
        self.command_return_menu = menu;
        self.return_screen = None;
        self.clear_command_menu_notice();
        self.planet.info_prompt_active = true;
        self.planet.info_input.clear();
        self.planet.info_error = None;
        self.planet.info_selected = None;
        self.current_screen = match menu {
            CommandMenu::Main => ScreenId::MainMenu,
            CommandMenu::General => ScreenId::GeneralMenu,
            CommandMenu::Fleet => ScreenId::FleetMenu,
            CommandMenu::Starbase => ScreenId::StarbaseMenu,
            CommandMenu::Planet => ScreenId::PlanetMenu,
            CommandMenu::PlanetBuild => ScreenId::PlanetBuildMenu,
        };
    }

    pub fn close_planet_info_prompt(&mut self) {
        self.planet.info_prompt_active = false;
        self.planet.info_input.clear();
        self.planet.info_error = None;
    }

    pub fn append_planet_info_char(&mut self, ch: char) {
        if self.planet.info_input.len() < 16 {
            self.planet.info_input.push(ch);
            self.planet.info_error = None;
        }
    }

    fn sync_planet_brief_cursor_to_input(&mut self, sort: PlanetListSort) -> bool {
        let mode = match self.current_screen {
            ScreenId::PlanetList(mode, _) => mode,
            _ => PlanetListMode::Brief,
        };
        let rows = self.planet_list_rows(mode, sort);
        let match_rows = rows
            .iter()
            .map(|row| vec![crate::screen::format_sector_coords_table(row.coords)])
            .collect::<Vec<_>>();
        let Some(matched) = crate::screen::table_selection::find_typed_jump(
            &match_rows,
            0,
            &self.planet.brief_input,
        ) else {
            return false;
        };
        self.planet.brief_cursor = matched.index;
        let visible_rows = self.planet_brief_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
        matched.is_terminal_exact_match
    }

    fn sync_planet_database_cursor_to_input(&mut self) -> bool {
        let rows = self.planet_database_rows();
        let match_rows = rows
            .iter()
            .map(|row| vec![crate::screen::format_sector_coords_table(row.coords)])
            .collect::<Vec<_>>();
        let Some(matched) = crate::screen::table_selection::find_typed_jump(
            &match_rows,
            0,
            &self.planet.database_input,
        ) else {
            return false;
        };
        self.planet.database_cursor = matched.index;
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
        matched.is_terminal_exact_match
    }

    pub fn backspace_planet_info_input(&mut self) {
        self.planet.info_input.pop();
        self.planet.info_error = None;
    }

    pub fn submit_planet_info_prompt(&mut self) {
        let Some(coords) = resolve_default_coords_input(
            &self.planet.info_input,
            self.default_planet_prompt_coords(),
        ) else {
            self.planet.info_error = Some("Enter coordinates like 5,2".to_string());
            return;
        };

        if let Err(message) = self.open_planet_info_detail_at_coords(coords, None) {
            self.planet.info_error = Some(message);
        }
    }

    pub fn open_planet_info_detail_at_coords(
        &mut self,
        coords: [u8; 2],
        return_screen: Option<ScreenId>,
    ) -> Result<(), String> {
        let Some(planet_idx) = self.game_data.planet_record_index_at_coords(coords) else {
            return Err(format!(
                "No world found at [{:02},{:02}]",
                coords[0], coords[1]
            ));
        };

        self.return_screen = return_screen;
        self.planet.info_prompt_active = false;
        self.planet.info_selected = Some(planet_idx);
        self.planet.info_error = None;
        self.current_screen = ScreenId::PlanetInfoDetail;
        Ok(())
    }

    pub(crate) fn current_planet_list_row(
        &self,
    ) -> Result<nc_data::EmpirePlanetEconomyRow, String> {
        let ScreenId::PlanetList(mode, sort) = self.current_screen else {
            return Err("Planet list is not active.".to_string());
        };
        self.planet_list_rows(mode, sort)
            .get(self.planet.brief_cursor)
            .cloned()
            .ok_or_else(|| "You do not currently control any planets.".to_string())
    }

    pub(crate) fn inline_planet_tax_active_on_current_screen(&self) -> bool {
        self.planet.tax_prompt_active && self.current_screen == ScreenId::PlanetMenu
    }

    pub(crate) fn inline_planet_transport_prompt_active_on_current_screen(&self) -> bool {
        matches!(
            self.current_screen,
            ScreenId::PlanetMenu | ScreenId::PlanetList(PlanetListMode::Brief, _)
        ) && self.planet.transport_prompt_mode.is_some()
    }

    pub(crate) fn inline_planet_auto_commission_active_on_current_screen(&self) -> bool {
        self.planet.auto_commission_prompt_active
            && matches!(
                self.current_screen,
                ScreenId::PlanetMenu | ScreenId::PlanetList(PlanetListMode::Brief, _)
            )
    }

    pub(crate) fn inline_planet_build_abort_active_on_current_screen(&self) -> bool {
        self.planet.build_abort_prompt_active
            && matches!(
                self.current_screen,
                ScreenId::PlanetBuildMenu | ScreenId::PlanetList(PlanetListMode::Brief, _)
            )
    }

    pub(crate) fn inline_planet_info_active_on_current_screen(&self) -> bool {
        self.planet.info_prompt_active
            && matches!(
                self.current_screen,
                ScreenId::MainMenu
                    | ScreenId::GeneralMenu
                    | ScreenId::FleetMenu
                    | ScreenId::StarbaseMenu
                    | ScreenId::PlanetMenu
                    | ScreenId::PlanetBuildMenu
            )
    }

    pub fn planet_info_input(&self) -> &str {
        &self.planet.info_input
    }

    pub fn selected_planet_info(&self) -> Option<usize> {
        self.planet.info_selected
    }

    pub(crate) fn sorted_planet_rows(
        &self,
        sort: PlanetListSort,
    ) -> Vec<nc_data::EmpirePlanetEconomyRow> {
        let mut rows = self
            .game_data
            .empire_planet_economy_rows(self.player.record_index_1_based);
        rows.sort_by(|left, right| match sort {
            PlanetListSort::Location => apply_sort_direction(
                self.planet.list_sort_direction,
                left.coords.cmp(&right.coords),
            ),
            PlanetListSort::PlanetName => apply_sort_direction(
                self.planet.list_sort_direction,
                left.planet_name.cmp(&right.planet_name),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::PotentialProduction => apply_sort_direction(
                self.planet.list_sort_direction,
                left.potential_production.cmp(&right.potential_production),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::CurrentProduction => apply_sort_direction(
                self.planet.list_sort_direction,
                left.present_production.cmp(&right.present_production),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Treasury => apply_sort_direction(
                self.planet.list_sort_direction,
                planet_row_sort_metrics(self, left)
                    .0
                    .cmp(&planet_row_sort_metrics(self, right).0),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Budget => apply_sort_direction(
                self.planet.list_sort_direction,
                planet_row_sort_metrics(self, left)
                    .1
                    .cmp(&planet_row_sort_metrics(self, right).1),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Revenue => apply_sort_direction(
                self.planet.list_sort_direction,
                left.yearly_tax_revenue.cmp(&right.yearly_tax_revenue),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Growth => apply_sort_direction(
                self.planet.list_sort_direction,
                left.yearly_growth_delta.cmp(&right.yearly_growth_delta),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::BuildQueue => apply_sort_direction(
                self.planet.list_sort_direction,
                planet_row_sort_metrics(self, left)
                    .2
                    .cmp(&planet_row_sort_metrics(self, right).2),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Stardock => apply_sort_direction(
                self.planet.list_sort_direction,
                planet_row_sort_metrics(self, left)
                    .3
                    .cmp(&planet_row_sort_metrics(self, right).3),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Starbase => apply_sort_direction(
                self.planet.list_sort_direction,
                left.has_friendly_starbase.cmp(&right.has_friendly_starbase),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Armies => apply_sort_direction(
                self.planet.list_sort_direction,
                planet_row_sort_metrics(self, left)
                    .4
                    .cmp(&planet_row_sort_metrics(self, right).4),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetListSort::Batteries => apply_sort_direction(
                self.planet.list_sort_direction,
                planet_row_sort_metrics(self, left)
                    .5
                    .cmp(&planet_row_sort_metrics(self, right).5),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
        });
        rows
    }

    pub(crate) fn planet_list_rows(
        &self,
        mode: PlanetListMode,
        sort: PlanetListSort,
    ) -> Vec<nc_data::EmpirePlanetEconomyRow> {
        let rows = self.sorted_planet_rows(sort);
        if mode != PlanetListMode::Brief {
            return rows;
        }
        let mut rows = rows
            .into_iter()
            .filter(|row| self.planet_list_filter_matches(row))
            .collect::<Vec<_>>();
        if let Some(clause) = &self.planet.list_filter_clause {
            rows.retain(|row| planet_list_clause_matches(self, row, clause));
        }
        rows
    }

    fn planet_list_filter_matches(&self, row: &nc_data::EmpirePlanetEconomyRow) -> bool {
        match self.planet.list_filter {
            PlanetListFilter::All => true,
            PlanetListFilter::Range { anchor, radius } => {
                planet_database_distance_sq(anchor, row.coords)
                    <= u32::from(radius) * u32::from(radius)
            }
            PlanetListFilter::Starbase => row.has_friendly_starbase,
            PlanetListFilter::Stardock => self.planet_list_docked_units(row) > 0,
        }
    }

    fn planet_list_docked_units(&self, row: &nc_data::EmpirePlanetEconomyRow) -> u32 {
        self.game_data
            .planets
            .get(row.planet_record_index_1_based.saturating_sub(1))
            .map(|planet| {
                (0..nc_data::STARDOCK_SLOT_COUNT)
                    .map(|slot| u32::from(planet.stardock[slot].min(u32::from(u16::MAX)) as u16))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub(crate) fn planet_database_rows(&self) -> Vec<PlanetDatabaseRow> {
        let mut rows = build_player_starmap_projection_from_snapshots(
            &self.game_data,
            &self.planet_intel_snapshots,
            self.player.record_index_1_based as u8,
        )
        .worlds
        .into_iter()
        .map(|world| {
            let intel_snapshot = self
                .planet_intel_snapshots
                .get(&world.planet_record_index_1_based);
            let owner_label = known_owner_label(
                world.known_owner_empire_id,
                world.known_owner_empire_name.as_deref(),
                KnownOwnerLabelStyle::Database,
                world
                    .known_owner_empire_id
                    .and_then(|id| self.game_data.empire_campaign_state(id)),
            );
            let year_label = intel_snapshot
                .and_then(|snapshot| snapshot.last_intel_year)
                .map(|year| year.to_string())
                .unwrap_or_else(|| "?".to_string());
            PlanetDatabaseRow {
                planet_record_index_1_based: world.planet_record_index_1_based,
                coords: world.coords,
                known_name: world.known_name.clone(),
                known_owner_empire_id: world.known_owner_empire_id,
                known_owner_name: world.known_owner_empire_name.clone(),
                known_max_production: world.known_potential_production,
                known_year_seen: intel_snapshot.and_then(|snapshot| snapshot.last_intel_year),
                known_armies: world.known_armies,
                known_batteries: world.known_ground_batteries,
                known_starbase_count: world.known_starbase_count,
                known_current_production: world.known_current_production,
                known_stored_points: world.known_stored_points,
                known_scout_year: intel_snapshot.and_then(|snapshot| snapshot.last_intel_year),
                name_label: world.known_name.unwrap_or_else(|| "?".to_string()),
                owner_label,
                max_prod_label: world
                    .known_potential_production
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                year_seen_label: year_label.clone(),
                armies_label: world
                    .known_armies
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                batteries_label: world
                    .known_ground_batteries
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                starbase_count_label: world
                    .known_starbase_count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                current_prod_label: world
                    .known_current_production
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                stored_points_label: world
                    .known_stored_points
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                year_scout_label: year_label,
            }
        })
        .collect::<Vec<_>>();
        rows.retain(|row| match self.planet.database_filter {
            PlanetDatabaseFilter::All => true,
            PlanetDatabaseFilter::Range { anchor, radius } => {
                planet_database_distance_sq(anchor, row.coords)
                    <= u32::from(radius) * u32::from(radius)
            }
            PlanetDatabaseFilter::Empire(empire_id) => row.known_owner_empire_id == Some(empire_id),
            PlanetDatabaseFilter::MaxProduction(min_prod) => row
                .known_max_production
                .is_some_and(|value| value >= min_prod),
        });
        if let Some(clause) = &self.planet.database_filter_clause {
            rows.retain(|row| planet_database_clause_matches(row, clause));
        }

        rows.sort_by(|left, right| match self.planet.database_sort {
            PlanetDatabaseSort::Location => apply_sort_direction(
                self.planet.database_sort_direction,
                left.coords.cmp(&right.coords),
            ),
            PlanetDatabaseSort::Range(anchor) => apply_sort_direction(
                self.planet.database_sort_direction,
                planet_database_distance_sq(anchor, left.coords)
                    .cmp(&planet_database_distance_sq(anchor, right.coords)),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::PlanetName => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_name.as_deref().cmp(&right.known_name.as_deref()),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::Owner => apply_sort_direction(
                self.planet.database_sort_direction,
                (
                    left.known_owner_empire_id.is_none(),
                    left.known_owner_empire_id.unwrap_or(0),
                )
                    .cmp(&(
                        right.known_owner_empire_id.is_none(),
                        right.known_owner_empire_id.unwrap_or(0),
                    )),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::MaxProduction => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_max_production.cmp(&right.known_max_production),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::YearSeen => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_year_seen.cmp(&right.known_year_seen),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::Armies => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_armies.cmp(&right.known_armies),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::Batteries => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_batteries.cmp(&right.known_batteries),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::Starbases => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_starbase_count.cmp(&right.known_starbase_count),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::CurrentProduction => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_current_production
                    .cmp(&right.known_current_production),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::Treasury => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_stored_points.cmp(&right.known_stored_points),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
            PlanetDatabaseSort::ScoutYear => apply_sort_direction(
                self.planet.database_sort_direction,
                left.known_scout_year.cmp(&right.known_scout_year),
            )
            .then_with(|| left.coords.cmp(&right.coords)),
        });
        rows
    }

    pub(crate) fn default_planet_database_coords(&self) -> [u8; 2] {
        self.planet_database_rows()
            .get(self.planet.database_cursor)
            .map(|row| row.coords)
            .unwrap_or([0, 0])
    }

    fn planet_list_filter_default_value(
        &self,
        mode: PlanetListMode,
        column: TableFilterColumn,
    ) -> String {
        let row = self
            .planet_list_rows(mode, self.planet.list_sort)
            .get(self.planet.brief_cursor)
            .cloned();
        let Some(row) = row else {
            return String::new();
        };
        let planet = self
            .game_data
            .planets
            .get(row.planet_record_index_1_based.saturating_sub(1));
        let (treasury, budget) = planet_build_view(&self.game_data, &row)
            .map(|view| (view.treasury_left.to_string(), view.points_left.to_string()))
            .unwrap_or_else(|_| {
                (
                    row.stored_production_points.to_string(),
                    u32::from(row.build_capacity)
                        .min(row.stored_production_points)
                        .to_string(),
                )
            });
        match column.code {
            "coo" => format!("{},{}", row.coords[0], row.coords[1]),
            "pla" => row.planet_name,
            "max" => row.potential_production.to_string(),
            "cur" => row.present_production.to_string(),
            "trs" => treasury,
            "bdg" => budget,
            "rev" => row.yearly_tax_revenue.to_string(),
            "gro" => row.yearly_growth_delta.to_string(),
            "bui" => planet
                .map(|planet| {
                    (0..10)
                        .map(|slot| {
                            let points = planet.build_queue[slot].points_owed;
                            let kind = nc_data::ProductionItemKind::from_code(
                                planet.build_queue[slot].kind.code(),
                            );
                            kind.build_cost().map(|cost| points / cost).unwrap_or(0)
                        })
                        .sum::<u32>()
                        .to_string()
                })
                .unwrap_or_default(),
            "sta" => self.planet_list_docked_units(&row).to_string(),
            "sbs" => u8::from(row.has_friendly_starbase).to_string(),
            "ars" => planet
                .map(|planet| (planet.armies.min(u16::from(u8::MAX)) as u8).to_string())
                .unwrap_or_default(),
            "gbs" => planet
                .map(|planet| (planet.ground_batteries.min(u16::from(u8::MAX)) as u8).to_string())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn planet_database_filter_default_value(&self, column: TableFilterColumn) -> String {
        let row = self
            .planet_database_rows()
            .get(self.planet.database_cursor)
            .cloned();
        let Some(row) = row else {
            return String::new();
        };
        match column.code {
            "coo" => format!("{},{}", row.coords[0], row.coords[1]),
            "pla" => row.name_label,
            "own" => {
                if row.known_owner_empire_id.is_some() {
                    row.owner_label
                } else {
                    "?".to_string()
                }
            }
            "max" => row.max_prod_label,
            "see" => row.year_seen_label,
            "ars" => row.armies_label,
            "gbs" => row.batteries_label,
            "sbs" => row.starbase_count_label,
            "cur" => row.current_prod_label,
            "trs" => row.stored_points_label,
            "sco" => row.year_scout_label,
            _ => String::new(),
        }
    }

    fn apply_planet_database_filter(&mut self, filter: PlanetDatabaseFilter) {
        self.planet.database_filter_clause = None;
        let selected_record = self
            .planet_database_rows()
            .get(self.planet.database_cursor)
            .map(|row| row.planet_record_index_1_based);
        let previous_filter = self.planet.database_filter;
        self.planet.database_filter = filter;
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_status = None;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.planet.database_pending_column = None;
        self.current_screen = ScreenId::PlanetDatabaseList;

        let rows = self.planet_database_rows();
        if rows.is_empty() {
            self.planet.database_filter = PlanetDatabaseFilter::All;
            if previous_filter == PlanetDatabaseFilter::All {
                self.planet.database_cursor = 0;
                self.planet.database_scroll_offset = 0;
            } else {
                let full_rows = self.planet_database_rows();
                self.planet.database_cursor = selected_record
                    .and_then(|record| {
                        full_rows
                            .iter()
                            .position(|row| row.planet_record_index_1_based == record)
                    })
                    .unwrap_or(0);
                let visible_rows = self.planet_database_visible_rows();
                sync_scroll_to_cursor(
                    &mut self.planet.database_scroll_offset,
                    self.planet.database_cursor,
                    visible_rows,
                );
            }
            return;
        }

        self.planet.database_cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
    }

    fn apply_planet_database_filter_clause(&mut self, clause: Option<TableFilterClause>) {
        let selected_record = self
            .planet_database_rows()
            .get(self.planet.database_cursor)
            .map(|row| row.planet_record_index_1_based);
        self.planet.database_filter = PlanetDatabaseFilter::All;
        self.planet.database_filter_clause = clause;
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_status = None;
        self.planet.database_prompt_dismiss_message = None;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.planet.database_pending_column = None;
        self.current_screen = ScreenId::PlanetDatabaseList;
        let rows = self.planet_database_rows();
        if rows.is_empty() {
            self.planet.database_filter_clause = None;
        }
        let rows = self.planet_database_rows();
        self.planet.database_cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
    }

    fn apply_planet_database_sort(&mut self, sort: PlanetDatabaseSort) {
        let selected_record = self
            .planet_database_rows()
            .get(self.planet.database_cursor)
            .map(|row| row.planet_record_index_1_based);
        if self.planet.database_sort == sort {
            self.planet.database_sort_direction = self.planet.database_sort_direction.toggle();
        } else {
            self.planet.database_sort = sort;
            self.planet.database_sort_direction = default_planet_database_sort_direction(sort);
        }
        self.planet.database_prompt_mode = PlanetDatabasePromptMode::FilterMenu;
        self.planet.database_status = None;
        self.planet.database_prompt_dismiss_message = None;
        self.planet.database_input.clear();
        self.planet.database_prompt_default_value.clear();
        self.planet.database_pending_range_anchor = None;
        self.current_screen = ScreenId::PlanetDatabaseList;

        let rows = self.planet_database_rows();
        if rows.is_empty() {
            self.planet.database_cursor = 0;
            self.planet.database_scroll_offset = 0;
            return;
        }

        self.planet.database_cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.planet_database_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.database_scroll_offset,
            self.planet.database_cursor,
            visible_rows,
        );
    }

    fn apply_planet_list_filter(&mut self, mode: PlanetListMode, filter: PlanetListFilter) {
        self.planet.list_filter_clause = None;
        let selected_record = self
            .planet_list_rows(mode, self.planet.list_sort)
            .get(self.planet.brief_cursor)
            .map(|row| row.planet_record_index_1_based);
        let previous_filter = self.planet.list_filter;
        self.planet.list_filter = filter;
        self.planet.list_filter_prompt_mode = PlanetListFilterPromptMode::FilterMenu;
        self.planet.list_prompt_status = None;
        self.planet.list_prompt_dismiss_message = None;
        self.planet.list_prompt_input.clear();
        self.planet.list_prompt_default_value.clear();
        self.planet.list_pending_range_anchor = None;
        self.planet.list_filter_pending_column = None;
        self.current_screen = ScreenId::PlanetList(mode, self.planet.list_sort);

        let rows = self.planet_list_rows(mode, self.planet.list_sort);
        if rows.is_empty() {
            self.planet.list_filter = PlanetListFilter::All;
            if previous_filter == PlanetListFilter::All {
                self.planet.brief_cursor = 0;
                self.planet.brief_scroll_offset = 0;
            } else {
                let full_rows = self.planet_list_rows(mode, self.planet.list_sort);
                self.planet.brief_cursor = selected_record
                    .and_then(|record| {
                        full_rows
                            .iter()
                            .position(|row| row.planet_record_index_1_based == record)
                    })
                    .unwrap_or(0);
                let visible_rows = self.planet_brief_visible_rows();
                sync_scroll_to_cursor(
                    &mut self.planet.brief_scroll_offset,
                    self.planet.brief_cursor,
                    visible_rows,
                );
            }
            return;
        }

        self.planet.brief_cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.planet_brief_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
    }

    fn apply_planet_list_filter_clause(
        &mut self,
        mode: PlanetListMode,
        clause: Option<TableFilterClause>,
    ) {
        let selected_record = self
            .planet_list_rows(mode, self.planet.list_sort)
            .get(self.planet.brief_cursor)
            .map(|row| row.planet_record_index_1_based);
        self.planet.list_filter = PlanetListFilter::All;
        self.planet.list_filter_clause = clause;
        self.planet.list_filter_prompt_mode = PlanetListFilterPromptMode::FilterMenu;
        self.planet.list_prompt_status = None;
        self.planet.list_prompt_dismiss_message = None;
        self.planet.list_prompt_input.clear();
        self.planet.list_prompt_default_value.clear();
        self.planet.list_pending_range_anchor = None;
        self.planet.list_filter_pending_column = None;
        self.current_screen = ScreenId::PlanetList(mode, self.planet.list_sort);
        let rows = self.planet_list_rows(mode, self.planet.list_sort);
        if rows.is_empty() {
            self.planet.list_filter_clause = None;
        }
        let rows = self.planet_list_rows(mode, self.planet.list_sort);
        self.planet.brief_cursor = selected_record
            .and_then(|record| {
                rows.iter()
                    .position(|row| row.planet_record_index_1_based == record)
            })
            .unwrap_or(0);
        let visible_rows = self.planet_brief_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
    }

    pub(crate) fn handle_planet_info_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                crate::app::Action::Planet(PlanetAction::CloseInfoPrompt)
            }
            KeyCode::Enter => crate::app::Action::Planet(PlanetAction::SubmitInfoPrompt),
            KeyCode::Backspace => crate::app::Action::Planet(PlanetAction::BackspaceInfoInput),
            KeyCode::Char(ch)
                if ch.is_ascii_digit()
                    || matches!(ch, ',' | ' ' | ':' | '/' | '-' | '(' | ')' | '[' | ']') =>
            {
                crate::app::Action::Planet(PlanetAction::AppendInfoChar(ch))
            }
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_planet_auto_commission_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                crate::app::Action::Planet(PlanetAction::ConfirmAutoCommission)
            }
            KeyCode::Char('q')
            | KeyCode::Char('Q')
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Esc => crate::app::Action::Planet(PlanetAction::CloseAutoCommissionPrompt),
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn handle_planet_build_abort_prompt_key(
        &self,
        key: crossterm::event::KeyEvent,
    ) -> crate::app::Action {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                crate::app::Action::Planet(PlanetAction::ConfirmBuildAbort)
            }
            KeyCode::Char('q')
            | KeyCode::Char('Q')
            | KeyCode::Char('n')
            | KeyCode::Char('N')
            | KeyCode::Enter
            | KeyCode::Esc => crate::app::Action::Planet(PlanetAction::CloseBuildAbortPrompt),
            _ => crate::app::Action::Noop,
        }
    }

    pub(crate) fn default_planet_prompt_coords(&self) -> [u8; 2] {
        let homeworld_index = self
            .game_data
            .players
            .get(self.player.record_index_1_based - 1)
            .map(|player| player.homeworld as usize)
            .unwrap_or(0);
        if homeworld_index != 0 {
            if let Some(planet) = self.game_data.planets.get(homeworld_index - 1) {
                return planet.sector.to_array();
            }
        }
        self.game_data
            .planets
            .iter()
            .find(|planet| {
                planet.owner.0 as usize == self.player.record_index_1_based
                    && nc_data::is_homeworld_seed_ignoring_name(&planet)
            })
            .map(|planet| planet.sector.to_array())
            .unwrap_or([8, 2])
    }

    fn select_planet_brief_origin_row(&mut self, mode: PlanetListMode, sort: PlanetListSort) {
        if mode != PlanetListMode::BuildSelect {
            return;
        }
        let Some(selected_record) = self
            .build_planet_rows()
            .get(self.planet.build_index)
            .map(|row| row.planet_record_index_1_based)
        else {
            return;
        };
        let rows = self.sorted_planet_rows(sort);
        let Some(index) = rows
            .iter()
            .position(|row| row.planet_record_index_1_based == selected_record)
        else {
            return;
        };
        self.planet.brief_cursor = index;
        let visible_rows = self.planet_brief_visible_rows();
        sync_scroll_to_cursor(
            &mut self.planet.brief_scroll_offset,
            self.planet.brief_cursor,
            visible_rows,
        );
    }

    fn open_planet_build_menu_at_coords(
        &mut self,
        coords: [u8; 2],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rows = self.build_planet_rows();
        let Some(index) = rows.iter().position(|row| row.coords == coords) else {
            self.planet.list_prompt_status = Some(format!(
                "No build target found at ({:02},{:02})",
                coords[0], coords[1]
            ));
            return Ok(());
        };
        self.planet.build_index = index;
        self.planet.list_prompt_status = None;
        self.open_planet_build_menu();
        Ok(())
    }
}

fn planet_database_distance_sq(a: [u8; 2], b: [u8; 2]) -> u32 {
    let dx = i32::from(a[0]) - i32::from(b[0]);
    let dy = i32::from(a[1]) - i32::from(b[1]);
    (dx * dx + dy * dy) as u32
}
