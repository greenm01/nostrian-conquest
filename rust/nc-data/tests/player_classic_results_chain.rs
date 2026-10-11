use nc_compat::classic::encode_player;
use nc_data::{ClassicResultsChain, Player, has_results_chain, results_chain, set_results_chain};

#[test]
fn classic_results_chain_round_trips() {
    let mut player = Player {
        results_first: i32::from_le_bytes([0xAA; 4]),
        results_last: i32::from_le_bytes([0xAA; 4]),
        ..Player::default()
    };

    let chain = ClassicResultsChain {
        first_header: 12,
        last_header: 36,
    };
    set_results_chain(&mut player, Some(chain));
    assert_eq!(player.results_first, 12);
    assert_eq!(player.results_last, 36);
    // The records are i32s; the chain replaces them whole.
    assert_eq!(
        &encode_player(&player)[0x38..0x40],
        &[12, 0, 0, 0, 36, 0, 0, 0]
    );
    assert_eq!(results_chain(&player), Some(chain));
    assert!(has_results_chain(&player));

    set_results_chain(&mut player, None);
    assert_eq!(&encode_player(&player)[0x38..0x40], &[0; 8]);
    assert_eq!(results_chain(&player), None);
    assert!(!has_results_chain(&player));
}
