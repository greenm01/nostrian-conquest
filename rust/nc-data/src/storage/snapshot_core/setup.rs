use rusqlite::{Connection, Transaction, named_params, params};

use super::{CampaignStoreError, insert_row, position_of};
use crate::model::Setup;

/// The SETUP.DAT settings; `snapshot_setup_com_ports` holds the IRQ and
/// flow-control setting of COM1 to COM4, by port number.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_setup (
        snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
        version_tag TEXT NOT NULL,
        snoop INTEGER NOT NULL,
        max_key_gap_minutes INTEGER NOT NULL,
        remote_timeout INTEGER NOT NULL,
        local_timeout INTEGER NOT NULL,
        minimum_time_minutes INTEGER NOT NULL,
        purge_after_turns INTEGER NOT NULL,
        autopilot_after_turns INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS snapshot_setup_com_ports (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        com_port INTEGER NOT NULL,
        irq INTEGER NOT NULL,
        hw_flow_control INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, com_port)
    );";

pub(super) fn write(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    setup: &Setup,
) -> Result<(), CampaignStoreError> {
    // One character per tag byte, as classic text maps them.
    let version_tag: String = setup.version_tag.iter().map(|&b| char::from(b)).collect();
    insert_row(
        tx,
        "snapshot_setup",
        named_params! {
            ":snapshot_id": snapshot_id,
            ":version_tag": version_tag,
            ":snoop": setup.snoop,
            ":max_key_gap_minutes": setup.max_key_gap_minutes,
            ":remote_timeout": setup.remote_timeout,
            ":local_timeout": setup.local_timeout,
            ":minimum_time_minutes": setup.minimum_time_minutes,
            ":purge_after_turns": setup.purge_after_turns,
            ":autopilot_after_turns": setup.autopilot_after_turns,
        },
    )?;
    for (port, (irq, flow_control)) in setup
        .com_irq
        .iter()
        .zip(setup.com_hw_flow_control)
        .enumerate()
    {
        insert_row(
            tx,
            "snapshot_setup_com_ports",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":com_port": port + 1,
                ":irq": irq,
                ":hw_flow_control": flow_control,
            },
        )?;
    }
    Ok(())
}

pub(super) fn load(conn: &Connection, snapshot_id: i64) -> Result<Setup, CampaignStoreError> {
    let (version_tag, mut setup) = conn.query_row(
        "SELECT * FROM snapshot_setup WHERE snapshot_id = ?1",
        params![snapshot_id],
        |row| {
            Ok((
                row.get::<_, String>("version_tag")?,
                Setup {
                    snoop: row.get("snoop")?,
                    max_key_gap_minutes: row.get("max_key_gap_minutes")?,
                    remote_timeout: row.get("remote_timeout")?,
                    local_timeout: row.get("local_timeout")?,
                    minimum_time_minutes: row.get("minimum_time_minutes")?,
                    purge_after_turns: row.get("purge_after_turns")?,
                    autopilot_after_turns: row.get("autopilot_after_turns")?,
                    ..Setup::default()
                },
            ))
        },
    )?;
    let tag = version_tag
        .chars()
        .map(|ch| u8::try_from(u32::from(ch)).ok())
        .collect::<Option<Vec<u8>>>()
        .and_then(|tag| <[u8; 5]>::try_from(tag).ok());
    setup.version_tag = tag.ok_or_else(|| {
        CampaignStoreError::InvalidState(format!(
            "setup version tag {version_tag:?} is not five classic characters"
        ))
    })?;

    let mut stmt = conn.prepare(
        "SELECT com_port, irq, hw_flow_control
         FROM snapshot_setup_com_ports
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, usize>("com_port")?,
            row.get::<_, u8>("irq")?,
            row.get::<_, bool>("hw_flow_control")?,
        ))
    })?;
    for row in rows {
        let (com_port, irq, flow_control) = row?;
        let port = position_of(com_port, setup.com_irq.len(), "COM port")?;
        setup.com_irq[port] = irq;
        setup.com_hw_flow_control[port] = flow_control;
    }
    Ok(setup)
}
