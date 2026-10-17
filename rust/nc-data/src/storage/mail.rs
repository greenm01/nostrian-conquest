use rusqlite::{Connection, params};

use super::{CampaignStore, CampaignStoreError};
use crate::QueuedPlayerMail;

impl CampaignStore {
    pub fn load_snapshot_queued_mail(
        &self,
        snapshot_id: i64,
    ) -> Result<Vec<QueuedPlayerMail>, CampaignStoreError> {
        let mut conn = self.connection()?;
        load_queued_mail_rows(&mut conn, snapshot_id)
    }
}

pub(super) fn write_queued_mail_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    queued_mail: &[QueuedPlayerMail],
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO queued_mail(
             snapshot_id, queue_index, sender_empire_id, recipient_empire_id, year, subject, body,
             recipient_deleted
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    for (idx, mail) in queued_mail.iter().enumerate() {
        stmt.execute(params![
            snapshot_id,
            idx as i64,
            i64::from(mail.sender_empire_id),
            i64::from(mail.recipient_empire_id),
            i64::from(mail.year),
            &mail.subject,
            &mail.body,
            i64::from(u8::from(mail.recipient_deleted)),
        ])?;
        if mail.sender_deleted || mail.cancelled {
            tx.execute(
                "INSERT INTO mail_sender_state(snapshot_id, queue_index, sender_deleted, cancelled) VALUES (?1, ?2, ?3, ?4)",
                params![snapshot_id, idx as i64, mail.sender_deleted, mail.cancelled],
            )?;
        }
    }
    Ok(())
}

pub(super) fn load_queued_mail_rows(
    conn: &Connection,
    snapshot_id: i64,
) -> Result<Vec<QueuedPlayerMail>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT m.sender_empire_id, m.recipient_empire_id, m.year, m.subject, m.body, m.recipient_deleted,
                COALESCE(s.sender_deleted, 0), COALESCE(s.cancelled, 0)
         FROM queued_mail m LEFT JOIN mail_sender_state s USING(snapshot_id, queue_index)
         WHERE m.snapshot_id = ?1
         ORDER BY m.queue_index",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok(QueuedPlayerMail {
            sender_empire_id: row.get::<_, i64>(0)? as u8,
            recipient_empire_id: row.get::<_, i64>(1)? as u8,
            year: row.get::<_, i64>(2)? as u16,
            subject: row.get(3)?,
            body: row.get(4)?,
            recipient_deleted: row.get::<_, i64>(5)? != 0,
            sender_deleted: row.get::<_, i64>(6)? != 0,
            cancelled: row.get::<_, i64>(7)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
