pub mod binary;
pub mod combat;
pub mod compose;
pub mod entries;
pub mod format;
pub mod join;
pub mod mod_constants;
pub mod rankings;
pub mod render;
pub mod structured;
pub mod validation;

pub use entries::{apply_results_reviewable_flags, build_results_dat, build_results_report_blocks};
pub use rankings::build_rankings_text;

#[cfg(test)]
mod tests {
    use super::binary::classic_results_lines;
    use super::format::ordinal_number;
    use super::structured::{StructuredBodyItem, structured_combat_body, structured_report_text};

    #[test]
    fn ordinal_number_formats_st_nd_rd_and_teen_exceptions() {
        assert_eq!(ordinal_number(1), "1st");
        assert_eq!(ordinal_number(2), "2nd");
        assert_eq!(ordinal_number(3), "3rd");
        assert_eq!(ordinal_number(4), "4th");
        assert_eq!(ordinal_number(11), "11th");
        assert_eq!(ordinal_number(12), "12th");
        assert_eq!(ordinal_number(13), "13th");
        assert_eq!(ordinal_number(21), "21st");
        assert_eq!(ordinal_number(22), "22nd");
        assert_eq!(ordinal_number(23), "23rd");
    }

    #[test]
    fn classic_results_lines_wrap_body_without_leading_indent() {
        let text = "From your 13th Fleet, located in System(24,14)         Stardate: 52/3011 Sensor contact -- detected and identified an alien fleet in System(24,14). It is the 5th Fleet of \"Enemy\", (Empire #2). Their fleet contains 2 small vessel(s) of unknown type.";
        let lines = classic_results_lines(text);
        assert_eq!(
            lines[0],
            "From your 13th Fleet, located in System(24,14)         Stardate: 52/3011"
        );
        assert_eq!(
            lines[1],
            "Sensor contact -- detected and identified an alien fleet in"
        );
        assert_eq!(
            lines[2],
            "System(24,14). It is the 5th Fleet of \"Enemy\", (Empire #2). Their fleet"
        );
        assert!(lines.iter().all(|line| line.chars().count() <= 72));
        assert!(lines[1].starts_with("Sensor"));
    }

    #[test]
    fn classic_results_lines_preserve_explicit_blank_lines() {
        let header = "From your Fleet Command Center:                        Stardate: 03/3031";
        let text = structured_report_text(
            header,
            vec![
                StructuredBodyItem::Title("ALERT: Fleet contact lost!".to_string()),
                StructuredBodyItem::Blank,
                StructuredBodyItem::Label {
                    label: "Fleet lost:".to_string(),
                    value: "15th Fleet".to_string(),
                },
            ],
        );
        let lines = classic_results_lines(&text);
        assert_eq!(lines[1], "ALERT: Fleet contact lost!");
        assert_eq!(lines[2], "");
        assert!(lines.iter().all(|line| line.chars().count() <= 72));
    }

    #[test]
    fn structured_combat_body_inserts_blank_lines_between_sections() {
        let header = "From your Fleet Command Center:                        Stardate: 03/3031";
        let text = structured_report_text(
            header,
            structured_combat_body(
                "ALERT: Fleet contact lost!",
                vec![
                    StructuredBodyItem::Label {
                        label: "Fleet lost:".to_string(),
                        value: "13th Fleet".to_string(),
                    },
                    StructuredBodyItem::Label {
                        label: "Last contact:".to_string(),
                        value:
                            "destroyed by the 29th Fleet of \"Player1\", (Empire #1) in System(6,7)"
                                .to_string(),
                    },
                ],
                vec![
                    StructuredBodyItem::Label {
                        label: "Our forces:".to_string(),
                        value: "BB1".to_string(),
                    },
                    StructuredBodyItem::Label {
                        label: "Alien forces:".to_string(),
                        value: "BB9 CA1 TT*12 TT1".to_string(),
                    },
                ],
                vec![StructuredBodyItem::Label {
                    label: "Enemy losses:".to_string(),
                    value: "none".to_string(),
                }],
            ),
        );
        let lines = classic_results_lines(&text);
        let forces_idx = lines
            .iter()
            .position(|line| line.starts_with("Our forces:"))
            .expect("expected Our forces line");
        assert_eq!(lines[forces_idx - 1], "");
        assert!(lines.iter().all(|line| line.chars().count() <= 72));
    }
}
