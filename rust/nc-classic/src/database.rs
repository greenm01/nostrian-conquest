use crate::support::{ParseError, copy_array};

pub const DATABASE_RECORD_SIZE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseRecord {
    pub raw: [u8; DATABASE_RECORD_SIZE],
}

impl DatabaseRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; DATABASE_RECORD_SIZE],
        }
    }

    pub fn from_raw(raw: [u8; DATABASE_RECORD_SIZE]) -> Self {
        Self { raw }
    }

    pub fn planet_name_bytes(&self) -> &[u8] {
        let len = self.raw[0x00] as usize;
        if len > 0 && len <= 14 {
            &self.raw[0x01..0x01 + len]
        } else {
            &self.raw[0x01..0x01]
        }
    }

    pub fn set_planet_name(&mut self, name: &str) {
        self.raw[0x00..0x0F].fill(0);

        let bytes = name.as_bytes();
        let len = bytes.len().min(14);
        self.raw[0x00] = len as u8;
        self.raw[0x01..0x01 + len].copy_from_slice(&bytes[..len]);
    }

    pub fn name_area_raw(&self) -> [u8; 15] {
        copy_array(&self.raw[0x00..0x0F])
    }

    pub fn set_name_area_raw(&mut self, area: [u8; 15]) {
        self.raw[0x00..0x0F].copy_from_slice(&area);
    }

    pub fn word_at(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.raw[offset], self.raw[offset + 1]])
    }

    pub fn set_word_at(&mut self, offset: usize, value: u16) {
        self.raw[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn year_word(&self) -> u16 {
        self.word_at(0x08)
    }

    pub fn set_year_word(&mut self, year: u16) {
        self.set_word_at(0x08, year);
    }

    pub fn copy_from(&mut self, source: &DatabaseRecord) {
        self.raw.copy_from_slice(&source.raw);
    }

    pub fn set_unknown_planet(&mut self) {
        self.raw.fill(0);
        self.set_planet_name("UNKNOWN");
        self.raw[0x15] = 0xff;
        self.raw[0x1c] = 0xff;
        self.raw[0x1d] = 0xff;
        self.raw[0x1e] = 0xff;
        self.raw[0x1f] = 0xff;
        self.raw[0x20] = 0xff;
        self.raw[0x23] = 0xff;
        self.raw[0x24] = 0xff;
        self.raw[0x25] = 0xff;
        self.raw[0x26] = 0xff;
    }

    pub fn set_blank_unknown_planet(&mut self) {
        self.raw.fill(0);
        self.raw[0x01..0x08].copy_from_slice(b"UNKNOWN");
        self.raw[0x15] = 0xff;
        self.raw[0x1c] = 0xff;
        self.raw[0x1d] = 0xff;
        self.raw[0x1e] = 0xff;
        self.raw[0x1f] = 0xff;
        self.raw[0x20] = 0xff;
        self.raw[0x23] = 0xff;
        self.raw[0x24] = 0xff;
        self.raw[0x25] = 0xff;
        self.raw[0x26] = 0xff;
    }

    pub fn has_blank_unknown_name_area(&self) -> bool {
        self.raw[0x00] == 0 && &self.raw[0x01..0x08] == b"UNKNOWN"
    }

    pub fn is_compat_orbit_seed(&self) -> bool {
        (self.has_blank_unknown_name_area() || !self.planet_name_bytes().is_empty())
            && (1..=4).contains(&self.raw[0x15])
            && self.word_at(0x16) == 0
            && self.word_at(0x18) == 0
            && self.word_at(0x27) == 0
            && self.raw[0x1c] == 100
            && self.raw[0x1d] == 100
            && self.word_at(0x1e) == 0x23
            && self.word_at(0x23) == 10
            && self.word_at(0x25) == 4
    }

    pub fn is_compat_orbit_seed_for_viewer(&self, viewer_empire: u8) -> bool {
        self.is_compat_orbit_seed() && self.raw[0x15] == viewer_empire
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseDat {
    pub records: Vec<DatabaseRecord>,
}

impl DatabaseDat {
    pub fn new_zeroed(record_count: usize) -> Self {
        Self {
            records: (0..record_count)
                .map(|_| DatabaseRecord::new_zeroed())
                .collect(),
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % DATABASE_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "DATABASE.DAT",
                record_size: DATABASE_RECORD_SIZE,
                actual: data.len(),
            });
        }

        let records: Vec<DatabaseRecord> = data
            .chunks_exact(DATABASE_RECORD_SIZE)
            .map(|chunk| DatabaseRecord {
                raw: copy_array(chunk),
            })
            .collect();

        Ok(Self { records })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }

    pub fn record_index(planet_index: usize, player_index: usize, planet_count: usize) -> usize {
        player_index * planet_count + planet_index
    }

    pub fn record_mut(
        &mut self,
        planet_index: usize,
        player_index: usize,
        planet_count: usize,
    ) -> &mut DatabaseRecord {
        let idx = Self::record_index(planet_index, player_index, planet_count);
        &mut self.records[idx]
    }

    pub fn record(
        &self,
        planet_index: usize,
        player_index: usize,
        planet_count: usize,
    ) -> &DatabaseRecord {
        let idx = Self::record_index(planet_index, player_index, planet_count);
        &self.records[idx]
    }

    pub fn generate_from_planets_and_year(
        planet_names: &[String],
        _game_year: u16,
        player_count: usize,
        template: Option<&DatabaseDat>,
    ) -> Self {
        let expected_record_count = player_count * planet_names.len();
        let result = if let Some(t) = template.filter(|t| t.records.len() == expected_record_count)
        {
            t.clone()
        } else {
            let mut default = Self::new_zeroed(expected_record_count);
            for player in 0..player_count {
                for planet in 0..planet_names.len() {
                    let record = default.record_mut(planet, player, planet_names.len());
                    record.set_unknown_planet();
                }
            }
            default
        };

        let _ = planet_names;
        result
    }
}
