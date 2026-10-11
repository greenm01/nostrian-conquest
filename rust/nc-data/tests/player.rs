use nc_data::{DiplomaticRelation, Player};

#[test]
fn player_record_diplomacy_slots_map_enemy_and_neutral() {
    let mut player = Player::default();

    assert_eq!(
        player.diplomatic_relation_toward(1),
        Some(DiplomaticRelation::Neutral)
    );
    assert_eq!(
        player.diplomatic_relation_toward(2),
        Some(DiplomaticRelation::Neutral)
    );
    assert_eq!(
        player.diplomatic_relation_toward(4),
        Some(DiplomaticRelation::Neutral)
    );

    player.set_diplomatic_relation_byte_raw(2, 0x01);
    player.set_diplomatic_relation_byte_raw(4, 0x01);

    assert_eq!(
        player.diplomatic_relation_toward(1),
        Some(DiplomaticRelation::Neutral)
    );
    assert_eq!(
        player.diplomatic_relation_toward(2),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        player.diplomatic_relation_toward(3),
        Some(DiplomaticRelation::Neutral)
    );
    assert_eq!(
        player.diplomatic_relation_toward(4),
        Some(DiplomaticRelation::Enemy)
    );

    player.set_diplomatic_relation_byte_raw(25, 0x01);
    assert_eq!(
        player.diplomatic_relation_toward(25),
        Some(DiplomaticRelation::Enemy)
    );
}

#[test]
fn player_record_diplomacy_out_of_range_is_unknown() {
    let player = Player::default();
    assert_eq!(player.diplomatic_relation_toward(0), None);
    assert_eq!(player.diplomatic_relation_toward(26), None);
}
