pub(crate) const VIDEO_HEADER_LEN: usize = 21;
pub(crate) const VIDEO_HEADER_MIN_LEN: usize = 18;
pub(crate) const VIDEO_HEADER_MAX_SCAN_LEN: usize = 24;
pub(crate) const AUDIO_HEADER_LEN: usize = 17;
pub(crate) const NAL_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];
pub(crate) const FRAME_KIND_SPS: u8 = 0x01;
pub(crate) const FRAME_KIND_PPS: u8 = 0x02;
pub(crate) const FRAME_KIND_IDR: u8 = 0x04;
pub(crate) const AUDIO_KIND_ASC: u8 = 0x01;
pub(crate) const AUDIO_KIND_AAC: u8 = 0x02;

#[derive(Clone, Copy)]
pub(crate) struct VideoFrame<'a> {
    pub(crate) kind: u8,
    pub(crate) timestamp_ns: u64,
    pub(crate) payload: &'a [u8],
}

#[derive(Clone, Copy)]
pub(crate) struct AudioFrame<'a> {
    pub(crate) kind: u8,
    pub(crate) timestamp_ns: u64,
    pub(crate) payload: &'a [u8],
}

#[derive(Clone, Copy)]
pub(crate) struct AacConfig {
    pub(crate) audio_object_type: u8,
    pub(crate) sample_rate_index: u8,
    pub(crate) channel_config: u8,
}

impl Default for AacConfig {
    fn default() -> Self {
        // AAC-LC / 44.1kHz / stereo
        Self {
            audio_object_type: 2,
            sample_rate_index: 4,
            channel_config: 2,
        }
    }
}

impl AacConfig {
    pub(crate) fn from_asc(payload: &[u8]) -> Option<Self> {
        if payload.len() < 2 {
            return None;
        }

        let b0 = payload[0];
        let b1 = payload[1];
        let audio_object_type = (b0 >> 3) & 0x1f;
        let sample_rate_index = ((b0 & 0x07) << 1) | (b1 >> 7);
        let channel_config = (b1 >> 3) & 0x0f;

        if sample_rate_index == 0x0f || channel_config == 0 {
            return None;
        }

        Some(Self {
            audio_object_type,
            sample_rate_index,
            channel_config,
        })
    }

    pub(crate) fn adts_header(self, payload_len: usize) -> [u8; 7] {
        let frame_len = payload_len + 7;
        let profile = self.audio_object_type.saturating_sub(1) & 0x03;

        [
            0xff,
            0xf1,
            (profile << 6)
                | ((self.sample_rate_index & 0x0f) << 2)
                | ((self.channel_config >> 2) & 0x01),
            ((self.channel_config & 0x03) << 6) | ((frame_len >> 11) as u8 & 0x03),
            (frame_len >> 3) as u8,
            ((frame_len as u8 & 0x07) << 5) | 0x1f,
            0xfc,
        ]
    }
}

pub(crate) fn parse_video_packet(data: &[u8]) -> Option<VideoFrame<'_>> {
    if data.len() < VIDEO_HEADER_MIN_LEN {
        return None;
    }
    if data[0] != b'M' || data[1] != b'R' || data[2] != 0x01 {
        return None;
    }

    let payload_offset = detect_video_payload_offset(data)?;
    let kind = data[4];
    let timestamp_ns = data
        .get(9..17)
        .and_then(|raw| raw.try_into().ok())
        .map(u64::from_be_bytes)
        .unwrap_or(0);
    let payload = &data[payload_offset..];

    Some(VideoFrame {
        kind,
        timestamp_ns,
        payload,
    })
}

pub(crate) fn parse_audio_packet(data: &[u8]) -> Option<AudioFrame<'_>> {
    if data.len() < AUDIO_HEADER_LEN {
        return None;
    }
    if data[0] != b'M' || data[1] != b'R' || data[2] != 0x02 {
        return None;
    }

    let kind = data[4];
    let timestamp_ns = data
        .get(9..17)
        .and_then(|raw| raw.try_into().ok())
        .map(u64::from_be_bytes)
        .unwrap_or(0);
    let payload = &data[AUDIO_HEADER_LEN..];

    Some(AudioFrame {
        kind,
        timestamp_ns,
        payload,
    })
}

fn detect_video_payload_offset(data: &[u8]) -> Option<usize> {
    let known_offsets = [VIDEO_HEADER_LEN, VIDEO_HEADER_MIN_LEN];
    for &offset in &known_offsets {
        if offset < data.len() && looks_like_h264_payload(&data[offset..]) {
            return Some(offset);
        }
    }

    let end = (VIDEO_HEADER_MAX_SCAN_LEN + 1).min(data.len());
    for offset in VIDEO_HEADER_MIN_LEN..end {
        if looks_like_h264_payload(&data[offset..]) {
            return Some(offset);
        }
    }

    None
}

fn looks_like_h264_payload(payload: &[u8]) -> bool {
    if payload.is_empty() {
        return false;
    }

    if is_annexb(payload) {
        return true;
    }

    if looks_like_length_prefixed(payload, 4)
        || looks_like_length_prefixed(payload, 3)
        || looks_like_length_prefixed(payload, 2)
    {
        return true;
    }

    is_valid_h264_nal_type(payload[0] & 0x1F)
}

fn looks_like_length_prefixed(payload: &[u8], len_bytes: usize) -> bool {
    if len_bytes == 0 || payload.len() < len_bytes + 1 {
        return false;
    }

    let mut i = 0usize;
    while i + len_bytes <= payload.len() {
        let nal_len = match len_bytes {
            4 => match payload[i..i + 4].try_into() {
                Ok(v) => u32::from_be_bytes(v) as usize,
                Err(_) => return false,
            },
            3 => {
                ((payload[i] as usize) << 16)
                    | ((payload[i + 1] as usize) << 8)
                    | payload[i + 2] as usize
            }
            2 => match payload[i..i + 2].try_into() {
                Ok(v) => u16::from_be_bytes(v) as usize,
                Err(_) => return false,
            },
            _ => return false,
        };

        if nal_len == 0 || i + len_bytes + nal_len > payload.len() {
            return false;
        }

        let nal = &payload[i + len_bytes..i + len_bytes + nal_len];
        if nal.is_empty() || !is_valid_h264_nal_type(nal[0] & 0x1F) {
            return false;
        }

        i += len_bytes + nal_len;
    }

    i == payload.len()
}

pub(crate) fn hex_prefix(data: &[u8], max_len: usize) -> String {
    data.iter()
        .take(max_len)
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn nal_types_preview(data: &[u8], max_nals: usize) -> String {
    let mut out = Vec::new();
    for nal in annexb_nals(data).into_iter().take(max_nals) {
        if nal.is_empty() {
            continue;
        }
        out.push(format!("{}", nal[0] & 0x1F));
    }
    if out.is_empty() {
        "-".to_string()
    } else {
        out.join(",")
    }
}

fn is_annexb(payload: &[u8]) -> bool {
    payload.starts_with(&[0x00, 0x00, 0x01]) || payload.starts_with(&[0x00, 0x00, 0x00, 0x01])
}

fn is_valid_h264_nal_type(ty: u8) -> bool {
    (1..=23).contains(&ty)
}

fn convert_length_prefixed(payload: &[u8], len_bytes: usize) -> Option<Vec<u8>> {
    if len_bytes == 0 || payload.len() < len_bytes + 1 {
        return None;
    }

    let mut i = 0usize;
    let mut out = Vec::with_capacity(payload.len() + 64);

    while i + len_bytes <= payload.len() {
        let nal_len = match len_bytes {
            4 => u32::from_be_bytes(payload[i..i + 4].try_into().ok()?) as usize,
            3 => {
                ((payload[i] as usize) << 16)
                    | ((payload[i + 1] as usize) << 8)
                    | payload[i + 2] as usize
            }
            2 => u16::from_be_bytes(payload[i..i + 2].try_into().ok()?) as usize,
            1 => payload[i] as usize,
            _ => return None,
        };

        if nal_len == 0 || i + len_bytes + nal_len > payload.len() {
            return None;
        }
        let nal = &payload[i + len_bytes..i + len_bytes + nal_len];
        if nal.is_empty() || !is_valid_h264_nal_type(nal[0] & 0x1F) {
            return None;
        }

        out.extend_from_slice(&NAL_START_CODE);
        out.extend_from_slice(nal);
        i += len_bytes + nal_len;
    }

    if i == payload.len() && !out.is_empty() {
        Some(out)
    } else {
        None
    }
}

pub(crate) fn ensure_annexb(payload: &[u8]) -> Vec<u8> {
    if payload.is_empty() {
        return Vec::new();
    }
    if is_annexb(payload) {
        return payload.to_vec();
    }
    if let Some(v) = convert_length_prefixed(payload, 4) {
        return v;
    }
    if let Some(v) = convert_length_prefixed(payload, 3) {
        return v;
    }
    if let Some(v) = convert_length_prefixed(payload, 2) {
        return v;
    }
    if !payload.is_empty() && is_valid_h264_nal_type(payload[0] & 0x1F) {
        let mut out = Vec::with_capacity(payload.len() + 4);
        out.extend_from_slice(&NAL_START_CODE);
        out.extend_from_slice(payload);
        return out;
    }
    Vec::new()
}

fn find_start_code(data: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut i = from;
    while i + 3 <= data.len() {
        if i + 4 <= data.len()
            && data[i] == 0x00
            && data[i + 1] == 0x00
            && data[i + 2] == 0x00
            && data[i + 3] == 0x01
        {
            return Some((i, 4));
        }
        if data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x01 {
            return Some((i, 3));
        }
        i += 1;
    }
    None
}

fn annexb_nals(data: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut pos = 0usize;

    while let Some((sc_pos, sc_len)) = find_start_code(data, pos) {
        let nal_start = sc_pos + sc_len;
        let next = find_start_code(data, nal_start)
            .map(|(p, _)| p)
            .unwrap_or(data.len());
        if nal_start < next {
            out.push(&data[nal_start..next]);
        }
        pos = next;
        if pos >= data.len() {
            break;
        }
    }

    out
}

pub(crate) fn has_nal_type(data: &[u8], target: u8) -> bool {
    annexb_nals(data)
        .into_iter()
        .any(|nal| !nal.is_empty() && (nal[0] & 0x1F) == target)
}

pub(crate) fn extract_parameter_sets(data: &[u8]) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let mut sps: Option<Vec<u8>> = None;
    let mut pps: Option<Vec<u8>> = None;

    for nal in annexb_nals(data) {
        if nal.is_empty() {
            continue;
        }
        match nal[0] & 0x1F {
            7 => {
                let mut v = Vec::with_capacity(nal.len() + 4);
                v.extend_from_slice(&NAL_START_CODE);
                v.extend_from_slice(nal);
                sps = Some(v);
            }
            8 => {
                let mut v = Vec::with_capacity(nal.len() + 4);
                v.extend_from_slice(&NAL_START_CODE);
                v.extend_from_slice(nal);
                pps = Some(v);
            }
            _ => {}
        }
    }

    (sps, pps)
}
