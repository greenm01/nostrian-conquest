use nc_data::{ClassicResultsChain, PlayerRecord};

#[test]
fn classic_results_chain_round_trips() {
    let mut player = PlayerRecord::new_zeroed();
    player.raw[0x38..0x40].fill(0xAA);

    let chain = ClassicResultsChain {
        first_header: 12,
        last_header: 36,
    };
    player.set_classic_results_chain(Some(chain));
    assert_eq!(player.classic_results_first_header_raw(), 12);
    assert_eq!(player.classic_results_last_header_raw(), 36);
    assert_eq!(&player.raw[0x3A..0x3C], &[0, 0]);
    assert_eq!(&player.raw[0x3E..0x40], &[0, 0]);
    assert_eq!(player.classic_results_chain(), Some(chain));
    assert!(player.has_classic_results_chain_state());

    player.set_classic_results_chain(None);
    assert_eq!(&player.raw[0x38..0x40], &[0; 8]);
    assert_eq!(player.classic_results_chain(), None);
    assert!(!player.has_classic_results_chain_state());
}
