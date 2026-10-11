use nc_data::{
    DiplomaticRelation, Player, diplomatic_relation_toward, set_diplomatic_relation_toward,
};

#[test]
fn player_record_diplomacy_slots_map_enemy_and_neutral() {
    let mut player = Player::default();

    assert_eq!(
        diplomatic_relation_toward(&player, 1),
        Some(DiplomaticRelation::Neutral)
    );
    assert_eq!(
        diplomatic_relation_toward(&player, 2),
        Some(DiplomaticRelation::Neutral)
    );
    assert_eq!(
        diplomatic_relation_toward(&player, 4),
        Some(DiplomaticRelation::Neutral)
    );

    set_diplomatic_relation_toward(&mut player, 2, DiplomaticRelation::Enemy);
    set_diplomatic_relation_toward(&mut player, 4, DiplomaticRelation::Enemy);

    assert_eq!(
        diplomatic_relation_toward(&player, 1),
        Some(DiplomaticRelation::Neutral)
    );
    assert_eq!(
        diplomatic_relation_toward(&player, 2),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        diplomatic_relation_toward(&player, 3),
        Some(DiplomaticRelation::Neutral)
    );
    assert_eq!(
        diplomatic_relation_toward(&player, 4),
        Some(DiplomaticRelation::Enemy)
    );

    set_diplomatic_relation_toward(&mut player, 25, DiplomaticRelation::Enemy);
    assert_eq!(
        diplomatic_relation_toward(&player, 25),
        Some(DiplomaticRelation::Enemy)
    );
}

#[test]
fn player_record_diplomacy_out_of_range_is_unknown() {
    let player = Player::default();
    assert_eq!(diplomatic_relation_toward(&player, 0), None);
    assert_eq!(diplomatic_relation_toward(&player, 26), None);
}
