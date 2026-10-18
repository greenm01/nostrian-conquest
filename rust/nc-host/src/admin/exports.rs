use super::*;
use base64::Engine;
use nc_data::{CampaignStore, build_player_starmap_projection_from_snapshots};

pub(super) fn render(root: &Path, game: &str, id: &AccountIdentity, format: &str) -> Result<Value> {
    let access = seats::authorize(root, game, id)?;
    let seat = access["seat"].as_u64().ok_or("missing seat")? as u8;
    let (dir, _) = store(root, game)?;
    let runtime = CampaignStore::open_default_in_dir(&dir)?;
    for _ in 0..3 {
        let snapshot = runtime
            .load_latest_runtime_state()?
            .ok_or("missing state")?;
        let (contents, extension, content_type, prefix) = match format {
            "reports" => {
                let contents = snapshot
                    .report_block_rows
                    .iter()
                    .filter(|row| row.is_visible_to_viewer(seat) && !row.recipient_deleted)
                    .map(|row| row.decoded_text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                (contents, "TXT", "text/plain; charset=utf-8", "ECREPORT")
            }
            "map-txt" | "map-csv" | "map-details" | "map-zip" | "map-ods" | "map-xlsx" => {
                let intel = runtime.load_snapshot_planet_intel_by_viewer(
                    snapshot.snapshot_id,
                    snapshot.game_data.conquest.player_count,
                )?;
                let projection = build_player_starmap_projection_from_snapshots(
                    &snapshot.game_data,
                    &intel[usize::from(seat) - 1],
                    seat,
                );
                if matches!(format, "map-zip" | "map-ods" | "map-xlsx") {
                    let bytes = crate::map_archive::render(&projection, format)?;
                    if runtime.latest_revision()? != snapshot.revision {
                        continue;
                    }
                    let (extension, content_type) = match format {
                        "map-zip" => ("zip", "application/zip"),
                        "map-ods" => ("ods", "application/vnd.oasis.opendocument.spreadsheet"),
                        _ => ("xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
                    };
                    return Ok(json!({"game":game,"seat":seat,"year":snapshot.game_year,
                        "filename":format!("ECMAP-{game}-P{seat}-Y{}.{extension}",snapshot.game_year),
                        "content_type":content_type,
                        "contents_base64":base64::engine::general_purpose::STANDARD.encode(bytes)}));
                }
                if format == "map-txt" {
                    (
                        projection.render_ascii_export(),
                        "TXT",
                        "text/plain; charset=utf-8",
                        "ECMAP",
                    )
                } else if format == "map-csv" {
                    (
                        projection.render_csv_export(),
                        "CSV",
                        "text/csv; charset=utf-8",
                        "ECMAP",
                    )
                } else {
                    (
                        projection.render_csv_details_export(),
                        "CSV",
                        "text/csv; charset=utf-8",
                        "ECMAP-DETAILS",
                    )
                }
            }
            _ => return Err("format must be reports, map-txt, map-csv, map-details, map-zip, map-ods or map-xlsx".into()),
        };
        if runtime.latest_revision()? != snapshot.revision {
            continue;
        }
        return Ok(json!({"game":game,"seat":seat,"year":snapshot.game_year,
            "filename":format!("{prefix}-{game}-P{seat}-Y{}.{extension}",snapshot.game_year),
            "content_type":content_type,"contents":contents}));
    }
    Err("campaign changed during export; try again".into())
}
