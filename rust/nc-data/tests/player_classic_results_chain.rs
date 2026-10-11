use nc_data::{ClassicResultsChain, Player};

#[test]
fn classic_results_chain_round_trips() {
    let mut player = Player::default();
    player.set_results_first(i32::from_le_bytes([0xAA; 4]));
    player.set_results_last(i32::from_le_bytes([0xAA; 4]));

    let chain = ClassicResultsChain {
        first_header: 12,
        last_header: 36,
    };
    player.set_classic_results_chain(Some(chain));
    assert_eq!(player.classic_results_first_header_raw(), 12);
    assert_eq!(player.classic_results_last_header_raw(), 36);
    // The records are i32s; the chain replaces them whole.
    assert_eq!(&player.to_bytes()[0x38..0x40], &[12, 0, 0, 0, 36, 0, 0, 0]);
    assert_eq!(player.classic_results_chain(), Some(chain));
    assert!(player.has_classic_results_chain_state());

    player.set_classic_results_chain(None);
    assert_eq!(&player.to_bytes()[0x38..0x40], &[0; 8]);
    assert_eq!(player.classic_results_chain(), None);
    assert!(!player.has_classic_results_chain_state());
}
