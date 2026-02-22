const TS_PACKET_SIZE: usize = 188;
const PID_PAT: u16 = 0x0000;
const PID_PMT: u16 = 0x0100;
const PID_VIDEO: u16 = 0x0101;
const PID_AUDIO: u16 = 0x0102;

pub(crate) fn ns_to_90k(ts_ns: u64) -> u64 {
    ts_ns.saturating_mul(90_000) / 1_000_000_000
}

pub(crate) struct MpegTsMuxer {
    cc_pat: u8,
    cc_pmt: u8,
    cc_video: u8,
    frame_index: u64,
}

impl MpegTsMuxer {
    pub(crate) fn new() -> Self {
        Self {
            cc_pat: 0,
            cc_pmt: 0,
            cc_video: 0,
            frame_index: 0,
        }
    }

    pub(crate) fn push_video_access_unit(&mut self, annexb: &[u8], pts_90k: u64) -> Vec<u8> {
        let mut out = Vec::new();

        if self.frame_index == 0 || self.frame_index % 30 == 0 {
            out.extend_from_slice(&packetize_psi(
                &build_pat_section(PID_PMT),
                PID_PAT,
                &mut self.cc_pat,
            ));
            out.extend_from_slice(&packetize_psi(
                &build_pmt_section(PID_VIDEO, PID_VIDEO),
                PID_PMT,
                &mut self.cc_pmt,
            ));
        }

        let pes = build_h264_pes(annexb, pts_90k);
        out.extend_from_slice(&packetize_pes(
            &pes,
            PID_VIDEO,
            &mut self.cc_video,
            Some(pts_90k),
        ));
        self.frame_index += 1;

        out
    }
}

pub(crate) struct AvMpegTsMuxer {
    cc_pat: u8,
    cc_pmt: u8,
    cc_video: u8,
    cc_audio: u8,
    origin_ns: Option<u64>,
    pts_offset_90k: u64,
    last_pts_90k: Option<u64>,
    video_count: u64,
    audio_count: u64,
}

impl AvMpegTsMuxer {
    pub(crate) fn new() -> Self {
        Self {
            cc_pat: 0,
            cc_pmt: 0,
            cc_video: 0,
            cc_audio: 0,
            origin_ns: None,
            pts_offset_90k: 0,
            last_pts_90k: None,
            video_count: 0,
            audio_count: 0,
        }
    }

    fn pts_90k(&mut self, timestamp_ns: u64) -> u64 {
        let base = self.origin_ns.get_or_insert(timestamp_ns);
        let raw_pts = ns_to_90k(timestamp_ns.saturating_sub(*base));
        let mut pts = raw_pts.saturating_add(self.pts_offset_90k);

        // Timestamp reset (or major clock jump) can happen after WS reconnect.
        // Keep PTS monotonic by shifting the timebase forward.
        if let Some(last) = self.last_pts_90k {
            // 900 ticks = 10ms at 90kHz.
            if pts.saturating_add(900) < last {
                self.pts_offset_90k = last.saturating_add(900).saturating_sub(raw_pts);
                pts = raw_pts.saturating_add(self.pts_offset_90k);
            }
            if pts <= last {
                pts = last.saturating_add(1);
            }
        }

        self.last_pts_90k = Some(pts);
        pts
    }

    fn write_tables(&mut self, out: &mut Vec<u8>) {
        out.extend_from_slice(&packetize_psi(
            &build_pat_section(PID_PMT),
            PID_PAT,
            &mut self.cc_pat,
        ));
        out.extend_from_slice(&packetize_psi(
            &build_pmt_section_av(PID_VIDEO, PID_VIDEO, PID_AUDIO),
            PID_PMT,
            &mut self.cc_pmt,
        ));
    }

    pub(crate) fn push_video_access_unit(&mut self, annexb: &[u8], timestamp_ns: u64) -> Vec<u8> {
        let mut out = Vec::new();

        if self.video_count == 0 || self.video_count % 30 == 0 {
            self.write_tables(&mut out);
        }

        let pts_90k = self.pts_90k(timestamp_ns);
        let pes = build_h264_pes(annexb, pts_90k);
        out.extend_from_slice(&packetize_pes(
            &pes,
            PID_VIDEO,
            &mut self.cc_video,
            Some(pts_90k),
        ));
        self.video_count += 1;

        out
    }

    pub(crate) fn push_audio_adts_frame(
        &mut self,
        adts_frame: &[u8],
        timestamp_ns: u64,
    ) -> Vec<u8> {
        let mut out = Vec::new();

        // Audio only periods can happen on some lives, so refresh PMT periodically from audio too.
        if self.audio_count == 0 || self.audio_count % 120 == 0 {
            self.write_tables(&mut out);
        }

        let pts_90k = self.pts_90k(timestamp_ns);
        let pes = build_aac_pes(adts_frame, pts_90k);
        out.extend_from_slice(&packetize_pes(&pes, PID_AUDIO, &mut self.cc_audio, None));
        self.audio_count += 1;

        out
    }
}

pub(crate) fn build_bootstrap_tables() -> Vec<u8> {
    let mut cc_pat = 0u8;
    let mut cc_pmt = 0u8;
    let mut out = Vec::new();
    out.extend_from_slice(&packetize_psi(
        &build_pat_section(PID_PMT),
        PID_PAT,
        &mut cc_pat,
    ));
    out.extend_from_slice(&packetize_psi(
        &build_pmt_section(PID_VIDEO, PID_VIDEO),
        PID_PMT,
        &mut cc_pmt,
    ));
    out
}

pub(crate) fn build_bootstrap_tables_av() -> Vec<u8> {
    let mut cc_pat = 0u8;
    let mut cc_pmt = 0u8;
    let mut out = Vec::new();
    out.extend_from_slice(&packetize_psi(
        &build_pat_section(PID_PMT),
        PID_PAT,
        &mut cc_pat,
    ));
    out.extend_from_slice(&packetize_psi(
        &build_pmt_section_av(PID_VIDEO, PID_VIDEO, PID_AUDIO),
        PID_PMT,
        &mut cc_pmt,
    ));
    out
}

fn packetize_psi(section: &[u8], pid: u16, cc: &mut u8) -> Vec<u8> {
    let mut packet = [0xFFu8; TS_PACKET_SIZE];
    packet[0] = 0x47;
    packet[1] = 0x40 | ((pid >> 8) as u8 & 0x1F);
    packet[2] = (pid & 0xFF) as u8;
    packet[3] = 0x10 | (*cc & 0x0F);
    packet[4] = 0x00; // pointer_field

    let max_len = TS_PACKET_SIZE - 5;
    let copy_len = section.len().min(max_len);
    packet[5..5 + copy_len].copy_from_slice(&section[..copy_len]);

    *cc = (*cc + 1) & 0x0F;
    packet.to_vec()
}

fn packetize_pes(pes: &[u8], pid: u16, cc: &mut u8, pcr_90k: Option<u64>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut first = true;

    while offset < pes.len() {
        let remain = pes.len() - offset;
        let mut packet = [0xFFu8; TS_PACKET_SIZE];
        packet[0] = 0x47;
        packet[1] = (if first { 0x40 } else { 0x00 }) | ((pid >> 8) as u8 & 0x1F);
        packet[2] = (pid & 0xFF) as u8;

        if first && pcr_90k.is_some() {
            // Put PCR on the first TS packet of each video PES.
            // adaptation_field_length includes flags(1) + PCR(6) + optional stuffing.
            let min_adaptation_len = 7usize;
            let max_payload_with_min_adaptation = 184 - (1 + min_adaptation_len); // 176 bytes
            let payload_len = remain.min(max_payload_with_min_adaptation);
            let adaptation_len = 184 - payload_len - 1;

            packet[3] = 0x30 | (*cc & 0x0F); // adaptation + payload
            packet[4] = adaptation_len as u8;
            packet[5] = 0x10; // PCR_flag

            let pcr = pcr_90k.unwrap_or(0);
            write_pcr(&mut packet[6..12], pcr);

            if adaptation_len > min_adaptation_len {
                let stuffing_start = 12usize;
                let stuffing_len = adaptation_len - min_adaptation_len;
                packet[stuffing_start..stuffing_start + stuffing_len].fill(0xFF);
            }

            let payload_start = 5 + adaptation_len;
            packet[payload_start..payload_start + payload_len]
                .copy_from_slice(&pes[offset..offset + payload_len]);
            offset += payload_len;
        } else if remain >= 184 {
            packet[3] = 0x10 | (*cc & 0x0F); // payload only
            packet[4..TS_PACKET_SIZE].copy_from_slice(&pes[offset..offset + 184]);
            offset += 184;
        } else {
            let stuffing = 184 - remain;
            let adaptation_field_len = stuffing.saturating_sub(1);
            packet[3] = 0x30 | (*cc & 0x0F); // adaptation + payload
            packet[4] = adaptation_field_len as u8;

            let mut payload_start = 5usize;
            if adaptation_field_len > 0 {
                packet[5] = 0x00; // no adaptation flags
                payload_start += 1;
                let extra = adaptation_field_len - 1;
                if extra > 0 {
                    let end = payload_start + extra;
                    packet[payload_start..end].fill(0xFF);
                    payload_start = end;
                }
            }

            packet[payload_start..payload_start + remain].copy_from_slice(&pes[offset..]);
            offset = pes.len();
        }

        *cc = (*cc + 1) & 0x0F;
        out.extend_from_slice(&packet);
        first = false;
    }

    out
}

fn write_pcr(out: &mut [u8], pcr_90k: u64) {
    // PCR is a 42-bit value: base(33 bits, 90kHz) + extension(9 bits, 27MHz remainder).
    // We use extension=0 and drive base from our 90k timeline.
    let base = pcr_90k & 0x1FFFF_FFFFF; // 33 bits
    let ext = 0u16;
    out[0] = ((base >> 25) & 0xFF) as u8;
    out[1] = ((base >> 17) & 0xFF) as u8;
    out[2] = ((base >> 9) & 0xFF) as u8;
    out[3] = ((base >> 1) & 0xFF) as u8;
    out[4] = (((base & 0x01) as u8) << 7) | 0x7E | (((ext >> 8) & 0x01) as u8);
    out[5] = (ext & 0xFF) as u8;
}

fn build_h264_pes(annexb: &[u8], pts_90k: u64) -> Vec<u8> {
    let mut pes = Vec::with_capacity(14 + annexb.len());
    pes.extend_from_slice(&[0x00, 0x00, 0x01, 0xE0, 0x00, 0x00, 0x80, 0x80, 0x05]);
    pes.extend_from_slice(&encode_pts(pts_90k));
    pes.extend_from_slice(annexb);
    pes
}

fn build_aac_pes(adts_frame: &[u8], pts_90k: u64) -> Vec<u8> {
    let mut pes = Vec::with_capacity(14 + adts_frame.len());
    pes.extend_from_slice(&[0x00, 0x00, 0x01, 0xC0, 0x00, 0x00, 0x80, 0x80, 0x05]);
    pes.extend_from_slice(&encode_pts(pts_90k));
    pes.extend_from_slice(adts_frame);
    pes
}

fn encode_pts(pts_90k: u64) -> [u8; 5] {
    let pts = pts_90k & 0x1FFFFFFFF;
    [
        (0x2 << 4) | (((pts >> 30) as u8 & 0x07) << 1) | 0x01,
        ((pts >> 22) & 0xFF) as u8,
        (((pts >> 15) as u8 & 0x7F) << 1) | 0x01,
        ((pts >> 7) & 0xFF) as u8,
        (((pts as u8) & 0x7F) << 1) | 0x01,
    ]
}

fn build_pat_section(pmt_pid: u16) -> Vec<u8> {
    let mut section = vec![
        0x00, 0xB0, 0x00, // table id + section length placeholder
        0x00, 0x01, // transport stream id
        0xC1, // version 0, current_next_indicator=1
        0x00, // section number
        0x00, // last section number
        0x00, 0x01, // program number
        0xE0 | ((pmt_pid >> 8) as u8 & 0x1F),
        (pmt_pid & 0xFF) as u8,
        0x00, 0x00, 0x00, 0x00, // crc placeholder
    ];

    set_section_length(&mut section);
    let crc = mpeg_crc32(&section[..section.len() - 4]).to_be_bytes();
    let n = section.len();
    section[n - 4..n].copy_from_slice(&crc);
    section
}

fn build_pmt_section(pcr_pid: u16, video_pid: u16) -> Vec<u8> {
    let mut section = vec![
        0x02, 0xB0, 0x00, // table id + section length placeholder
        0x00, 0x01, // program number
        0xC1, // version 0, current_next_indicator=1
        0x00, // section number
        0x00, // last section number
        0xE0 | ((pcr_pid >> 8) as u8 & 0x1F),
        (pcr_pid & 0xFF) as u8,
        0xF0, 0x00, // program info length
        0x1B, // stream_type H.264
        0xE0 | ((video_pid >> 8) as u8 & 0x1F),
        (video_pid & 0xFF) as u8,
        0xF0, 0x00, // ES info length
        0x00, 0x00, 0x00, 0x00, // crc placeholder
    ];

    set_section_length(&mut section);
    let crc = mpeg_crc32(&section[..section.len() - 4]).to_be_bytes();
    let n = section.len();
    section[n - 4..n].copy_from_slice(&crc);
    section
}

fn build_pmt_section_av(pcr_pid: u16, video_pid: u16, audio_pid: u16) -> Vec<u8> {
    let mut section = vec![
        0x02, 0xB0, 0x00, // table id + section length placeholder
        0x00, 0x01, // program number
        0xC1, // version 0, current_next_indicator=1
        0x00, // section number
        0x00, // last section number
        0xE0 | ((pcr_pid >> 8) as u8 & 0x1F),
        (pcr_pid & 0xFF) as u8,
        0xF0, 0x00, // program info length
        0x1B, // stream_type H.264
        0xE0 | ((video_pid >> 8) as u8 & 0x1F),
        (video_pid & 0xFF) as u8,
        0xF0, 0x00, // ES info length
        0x0F, // stream_type AAC (ADTS)
        0xE0 | ((audio_pid >> 8) as u8 & 0x1F),
        (audio_pid & 0xFF) as u8,
        0xF0, 0x00, // ES info length
        0x00, 0x00, 0x00, 0x00, // crc placeholder
    ];

    set_section_length(&mut section);
    let crc = mpeg_crc32(&section[..section.len() - 4]).to_be_bytes();
    let n = section.len();
    section[n - 4..n].copy_from_slice(&crc);
    section
}

fn set_section_length(section: &mut [u8]) {
    let section_length = (section.len() - 3) as u16;
    section[1] = (section[1] & 0xF0) | ((section_length >> 8) as u8 & 0x0F);
    section[2] = (section_length & 0xFF) as u8;
}

fn mpeg_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= (b as u32) << 24;
        for _ in 0..8 {
            if (crc & 0x8000_0000) != 0 {
                crc = (crc << 1) ^ 0x04C1_1DB7;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
