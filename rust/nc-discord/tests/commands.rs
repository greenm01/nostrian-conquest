#[test]
fn discovery_help_manuals_and_map_formats_are_registered_privately() {
    let commands = nc_discord::commands();
    let names: Vec<_> = commands.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "games", "join", "invite", "play", "status", "map", "report", "guide", "help", "about",
            "manuals"
        ]
    );
    assert!(commands.iter().all(|c| c.ephemeral));
    let join = commands.iter().find(|c| c.name == "join").unwrap();
    assert!(
        !join
            .parameters
            .iter()
            .find(|p| p.name == "code")
            .unwrap()
            .required
    );
    let map = commands.iter().find(|c| c.name == "map").unwrap();
    let format = map.parameters.iter().find(|p| p.name == "format").unwrap();
    assert!(!format.required);
    let choices: Vec<_> = format.choices.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(choices, ["Zip", "Text", "Csv", "Ods", "Excel"]);
}
