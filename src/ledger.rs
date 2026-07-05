use crate::cursor::ByteCursor;
use crate::dictionary::{name_similarity, NameLease, StringDictionary};
use crate::error::{ParseError, ParseResult};
use crate::model::{Finding, FindingSeverity, VesselId};

#[derive(Debug, Clone)]
pub struct LedgerRecord {
    pub vessel: VesselId,
    pub name_code: u16,
    pub berth_code: u16,
    pub flags: u16,
    pub retained_index: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct VesselLedger {
    dictionary: StringDictionary,
    records: Vec<LedgerRecord>,
    retained: Vec<NameLease>,
    harbor_retained: Vec<HarborLease>,
    materialized: Vec<String>,
    audit_score: u32,
}

#[derive(Debug, Clone, Copy)]
struct HarborLease {
    vessel: VesselId,
    name_code: u16,
    berth_code: u16,
    flags: u16,
    lease: NameLease,
    revision_tag: u32,
    route_hash: u32,
}

impl VesselLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dictionary(&self) -> &StringDictionary {
        &self.dictionary
    }

    pub fn records(&self) -> &[LedgerRecord] {
        &self.records
    }

    pub fn materialized(&self) -> &[String] {
        &self.materialized
    }

    pub fn apply_program(&mut self, data: &[u8], base: usize) -> ParseResult<Vec<Finding>> {
        self.apply_program_with_harbor_dictionary(data, base, None)
    }

    pub fn apply_program_with_harbor_dictionary(
        &mut self,
        data: &[u8],
        base: usize,
        harbor_dictionary: Option<&StringDictionary>,
    ) -> ParseResult<Vec<Finding>> {
        let mut cursor = ByteCursor::with_base(data, base);
        let mut findings = Vec::new();
        let count = cursor.read_u16()? as usize;
        if count > 32_768 {
            return Err(ParseError::limit(cursor.absolute_position(), "ledger op count"));
        }
        for _ in 0..count {
            let op_offset = cursor.absolute_position();
            let op = cursor.read_u8()?;
            match op {
                0x01 => {
                    let len = cursor.read_u16()? as usize;
                    let section_base = cursor.absolute_position();
                    let section = cursor.read_slice(len, "ledger dictionary delta")?;
                    self.dictionary.apply_delta(section, section_base)?;
                }
                0x02 => {
                    let len = cursor.read_u16()? as usize;
                    let section_base = cursor.absolute_position();
                    let section = cursor.read_slice(len, "ledger dictionary replace")?;
                    self.dictionary.replace_from(section, section_base)?;
                }
                0x03 => {
                    let vessel = VesselId(cursor.read_u32()?);
                    let name_code = cursor.read_u16()?;
                    let berth_code = cursor.read_u16()?;
                    let flags = cursor.read_u16()?;
                    let retained_index = if flags & 0x0002 != 0 {
                        let lease = self
                            .dictionary
                            .lease(name_code)
                            .unwrap_or_else(|| NameLease::empty(name_code, self.dictionary.generation()));
                        self.retained.push(lease);
                        Some(self.retained.len() - 1)
                    } else {
                        None
                    };
                    self.records.push(LedgerRecord {
                        vessel,
                        name_code,
                        berth_code,
                        flags,
                        retained_index,
                    });
                }
                0x04 => {
                    let keep_flags = cursor.read_u16()?;
                    self.dictionary.compact(keep_flags);
                }
                0x05 => {
                    self.materialize();
                    self.audit_score = self.audit_score.saturating_add(self.score_materialized());
                }
                0x06 => {
                    let target = cursor.read_u16()?;
                    findings.extend(self.find_similar_names(target));
                }
                0x07 => {
                    let trim = cursor.read_u16()? as usize;
                    if trim >= self.records.len() {
                        self.records.clear();
                    } else {
                        self.records.drain(0..trim);
                    }
                }
                0x08 => {
                    let vessel = VesselId(cursor.read_u32()?);
                    let name_code = cursor.read_u16()?;
                    let berth_code = cursor.read_u16()?;
                    let flags = cursor.read_u16()?;
                    let route_hash = cursor.read_u32()?;
                    if let Some(dictionary) = harbor_dictionary {
                        if let Some(lease) = dictionary.lease(name_code) {
                            self.harbor_retained.push(HarborLease {
                                vessel,
                                name_code,
                                berth_code,
                                flags,
                                lease,
                                revision_tag: dictionary.revision_tag(),
                                route_hash,
                            });
                        }
                    }
                }
                0x09 => {
                    let minimum_weight = cursor.read_u16()?;
                    let revision = harbor_dictionary
                        .map(|dictionary| dictionary.revision_tag())
                        .unwrap_or_default();
                    findings.extend(self.reconcile_harbor_leases(minimum_weight, revision));
                }
                _ => {
                    return Err(ParseError::invalid_tag(op_offset, "ledger op"));
                }
            }
        }
        Ok(findings)
    }

    pub fn materialize(&mut self) {
        for record in &self.records {
            if let Some(index) = record.retained_index {
                if let Some(lease) = self.retained.get(index).copied() {
                    if lease.generation() == self.dictionary.generation() {
                        let text = lease.text_lossy();
                        self.materialized.push(text);
                        continue;
                    }
                }
            }
            if let Some(text) = self.dictionary.get(record.name_code) {
                self.materialized.push(text.to_owned());
            }
        }
    }

    pub fn score_materialized(&self) -> u32 {
        let mut score = 0u32;
        for text in &self.materialized {
            score = score.saturating_add(text.len() as u32);
            if text.contains("HAZ") || text.contains("TANK") {
                score = score.saturating_add(12);
            }
        }
        score
    }

    pub fn audit_score(&self) -> u32 {
        self.audit_score
    }

    fn find_similar_names(&self, target_code: u16) -> Vec<Finding> {
        let mut findings = Vec::new();
        let target = self.dictionary.get(target_code).unwrap_or("");
        for record in &self.records {
            if let Some(name) = self.dictionary.get(record.name_code) {
                let similarity = name_similarity(target, name);
                if similarity > 640 && target_code != record.name_code {
                    findings.push(Finding::new(
                        FindingSeverity::Low,
                        "similar-name",
                        Some(record.vessel),
                        format!("name code {} resembles {}", record.name_code, target_code),
                    ));
                }
            }
        }
        findings
    }

    fn reconcile_harbor_leases(
        &mut self,
        minimum_weight: u16,
        current_revision: u32,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        for cached in &self.harbor_retained {
            let weighted = cached
                .route_hash
                .rotate_left((cached.berth_code as u32) & 31)
                ^ cached.revision_tag
                ^ current_revision
                ^ cached.lease.checksum();
            let priority = ((weighted >> 7) as u16) ^ cached.flags ^ cached.name_code;
            if priority < minimum_weight {
                continue;
            }
            if cached.revision_tag == current_revision || cached.flags & 0x0040 != 0 {
                let text = cached.lease.text_lossy();
                self.materialized.push(text.clone());
                if text.contains("TANK") || text.contains("GAS") || text.contains("CHEM") {
                    findings.push(Finding::new(
                        FindingSeverity::Medium,
                        "harbor-name-risk",
                        Some(cached.vessel),
                        format!("cached harbor name {}", cached.name_code),
                    ));
                }
            }
        }
        findings
    }
}

pub fn decode_ledger_program(data: &[u8]) -> ParseResult<Vec<Finding>> {
    let mut ledger = VesselLedger::new();
    ledger.apply_program(data, 0)
}
