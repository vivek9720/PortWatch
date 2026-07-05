use crate::cursor::ByteCursor;
use crate::error::{ParseError, ParseResult};
use crate::model::{ChannelNotice, Timestamp};

pub fn parse_notices(data: &[u8], base: usize) -> ParseResult<Vec<ChannelNotice>> {
    let mut cursor = ByteCursor::with_base(data, base);
    let count = cursor.read_u16()? as usize;
    if count > 8192 {
        return Err(ParseError::limit(cursor.absolute_position(), "notice count"));
    }
    let mut notices = Vec::with_capacity(count.min(128));
    for _ in 0..count {
        notices.push(parse_notice(&mut cursor)?);
    }
    Ok(notices)
}

pub fn parse_notice(cursor: &mut ByteCursor<'_>) -> ParseResult<ChannelNotice> {
    let notice_id = cursor.read_u32()?;
    let basin_code = cursor.read_u16()?;
    let starts_at = Timestamp(cursor.read_u64()?);
    let duration = cursor.read_u32()? as u64;
    let severity = cursor.read_u8()?;
    let subject_code = cursor.read_u16()?;
    let message_code = cursor.read_u16()?;
    let flags = cursor.read_u16()?;
    Ok(ChannelNotice {
        notice_id,
        basin_code,
        starts_at,
        expires_at: starts_at.saturating_add(duration),
        severity,
        subject_code,
        message_code,
        flags,
    })
}

#[derive(Debug, Clone, Default)]
pub struct NoticeIndex {
    notices: Vec<ChannelNotice>,
}

impl NoticeIndex {
    pub fn new(mut notices: Vec<ChannelNotice>) -> Self {
        notices.sort_by_key(|notice| (notice.basin_code, notice.starts_at.0, notice.notice_id));
        Self { notices }
    }

    pub fn active_for_basin(
        &self,
        basin_code: u16,
        time: Timestamp,
    ) -> impl Iterator<Item = &ChannelNotice> {
        self.notices
            .iter()
            .filter(move |notice| notice.basin_code == basin_code && notice.active_at(time))
    }

    pub fn closures_for_basin(
        &self,
        basin_code: u16,
    ) -> impl Iterator<Item = &ChannelNotice> {
        self.notices
            .iter()
            .filter(move |notice| notice.basin_code == basin_code && notice.is_closure())
    }

    pub fn expired_before(&self, time: Timestamp) -> Vec<u32> {
        self.notices
            .iter()
            .filter(|notice| notice.expires_at < time)
            .map(|notice| notice.notice_id)
            .collect()
    }

    pub fn severe_count(&self) -> usize {
        self.notices
            .iter()
            .filter(|notice| notice.severity >= 7)
            .count()
    }

    pub fn len(&self) -> usize {
        self.notices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notices.is_empty()
    }
}

pub fn coalesce_notices(mut notices: Vec<ChannelNotice>) -> Vec<ChannelNotice> {
    notices.sort_by_key(|notice| {
        (
            notice.basin_code,
            notice.subject_code,
            notice.message_code,
            notice.starts_at.0,
        )
    });
    let mut out: Vec<ChannelNotice> = Vec::new();
    for notice in notices {
        if let Some(last) = out.last_mut() {
            if last.basin_code == notice.basin_code
                && last.subject_code == notice.subject_code
                && last.message_code == notice.message_code
                && last.expires_at.0 >= notice.starts_at.0
            {
                last.expires_at = Timestamp(last.expires_at.0.max(notice.expires_at.0));
                last.severity = last.severity.max(notice.severity);
                last.flags |= notice.flags;
                continue;
            }
        }
        out.push(notice);
    }
    out
}

pub fn notice_pressure(notices: &[ChannelNotice], basin_code: u16, time: Timestamp) -> u32 {
    let mut pressure = 0u32;
    for notice in notices {
        if notice.basin_code != basin_code || !notice.active_at(time) {
            continue;
        }
        pressure = pressure.saturating_add(notice.severity as u32);
        if notice.is_closure() {
            pressure = pressure.saturating_add(20);
        }
        if notice.flags & 0x0004 != 0 {
            pressure = pressure.saturating_add(5);
        }
    }
    pressure
}
