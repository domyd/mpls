use super::types;
use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    combinator::map,
    cond, count, do_parse,
    error::ErrorKind,
    map,
    multi::{count, length_value},
    number::complete::{be_u16, be_u32, be_u64, be_u8},
    sequence::tuple,
    take, Err, IResult,
};
use std::convert::TryInto;
use types::{
    AngleInfo, AppInfoPlayList, AudioFormat, CharacterCode, Clip, ColorSpace, DynamicRange,
    ExtensionDataEntry, FrameRate, FrameRateFraction, LanguageCode, MarkType, Mpls, PlayItem,
    PlayItemRef, PlayList, PlayListMark, PlaybackType, Ref, SampleRate, Stream, StreamAttributes,
    StreamEntry, StreamEntryRef, StreamNumberTable, StreamRef, StreamType, SubClipRef, SubPath,
    SubPathRef, SubPlayItem, TimeStamp, VideoFormat,
};

fn str_len(len: usize, input: &[u8]) -> IResult<&[u8], &str> {
    let value = take(len);
    map(value, |v| std::str::from_utf8(v).unwrap())(input)
}

fn str_len_owned(len: usize, input: &[u8]) -> IResult<&[u8], String> {
    let (input, s) = str_len(len, input)?;
    Ok((input, s.into()))
}

// matches the ASCII/UTF-8 string "MPLS"
fn header_tag(input: &[u8]) -> IResult<&[u8], &str> {
    let (rest, s) = str_len(4, input)?;
    if s == "MPLS" {
        Ok((rest, s))
    } else {
        Err(Err::Error((input, ErrorKind::Tag)))
    }
}

fn version(input: &[u8]) -> IResult<&[u8], &str> {
    str_len(4, input)
}

fn addr(input: &[u8]) -> IResult<&[u8], u32> {
    let offset = take(4usize);
    map(offset, |o: &[u8]| u32::from_be_bytes(o.try_into().unwrap()))(input)
}

fn clip_file_name(input: &[u8]) -> IResult<&[u8], &str> {
    str_len(5, input)
}

fn clip_codec_id(input: &[u8]) -> IResult<&[u8], &str> {
    str_len(4, input)
}

fn is_multi_angle(input: &[u8]) -> IResult<&[u8], bool> {
    let (input, b) = be_u16(input)?;
    // 0000 0000 000X .... <-- connection_condition
    // |-reserved -|^---- the bit we want
    let is_multi_angle = ((b & 0x1F) >> 4) == 1;
    Ok((input, is_multi_angle))
}

fn time_stamp(input: &[u8]) -> IResult<&[u8], TimeStamp> {
    map(be_u32, |t| TimeStamp(t))(input)
}

fn clip_with_clock_ref(input: &[u8]) -> IResult<&[u8], Clip> {
    clip(input, true)
}

fn play_item_clip(input: &[u8]) -> IResult<&[u8], Clip> {
    clip(input, false)
}

fn clip(input: &[u8], with_ref_to_stcid: bool) -> IResult<&[u8], Clip> {
    do_parse!(
        input,
        f: clip_file_name
            >> c: clip_codec_id
            >> cond!(with_ref_to_stcid, take!(1usize))
            >> (Clip {
                file_name: f.into(),
                codec_id: c.into()
            })
    )
}

fn stream_entry(input: &[u8]) -> IResult<&[u8], StreamEntry> {
    fn stream_pid(input: &[u8]) -> IResult<&[u8], Ref> {
        map(be_u16, |n| Ref::Stream(StreamRef(n)))(input)
    }

    fn sub_clip_id(input: &[u8]) -> IResult<&[u8], Ref> {
        map(be_u8, |n| Ref::SubClip(SubClipRef(n)))(input)
    }

    fn sub_path_id(input: &[u8]) -> IResult<&[u8], Ref> {
        map(be_u8, |n| Ref::SubPath(SubPathRef(n)))(input)
    }

    fn stream_type(input: &[u8]) -> IResult<&[u8], u8> {
        map(
            alt((tag("\x01"), tag("\x02"), tag("\x03"), tag("\x04"))),
            |s: &[u8]| u8::from_be_bytes(s.try_into().unwrap()),
        )(input)
    }

    fn parser(input: &[u8]) -> IResult<&[u8], StreamEntry> {
        let (input, stream_type) = stream_type(input)?;
        let (input, refs) = match stream_type {
            0x1 => map(stream_pid, |s| StreamEntryRef::PlayItem(s))(input),
            0x2 => {
                let (input, sub_path_ref) = sub_path_id(input)?;
                let (input, sub_clip_ref) = sub_clip_id(input)?;
                let (input, stream_ref) = stream_pid(input)?;
                Ok((
                    input,
                    StreamEntryRef::SubPathKind1(sub_path_ref, sub_clip_ref, stream_ref),
                ))
            }
            0x3 | 0x4 => {
                let (input, sub_path_ref) = sub_path_id(input)?;
                let (input, stream_ref) = stream_pid(input)?;
                Ok((
                    input,
                    StreamEntryRef::SubPathKind2(sub_path_ref, stream_ref),
                ))
            }
            _ => panic!("can't happen, oh no"),
        }?;

        Ok((input, StreamEntry { stream_type, refs }))
    }

    length_value(be_u8, parser)(input)
}

fn stream_attrs(input: &[u8]) -> IResult<&[u8], StreamAttributes> {
    fn video_format(input: &[u8]) -> IResult<&[u8], (VideoFormat, FrameRate)> {
        map(be_u8, |n| {
            let video_format = match (n & 0xF0) >> 4 {
                0x1 => VideoFormat::Interlaced480,
                0x2 => VideoFormat::Interlaced576,
                0x3 => VideoFormat::Progressive480,
                0x4 => VideoFormat::Interlaced1080,
                0x5 => VideoFormat::Progressive720,
                0x6 => VideoFormat::Progressive1080,
                0x7 => VideoFormat::Progressive576,
                0x8 => VideoFormat::Progressive2160,
                _ => VideoFormat::Unknown,
            };
            let frame_rate = match n & 0x0F {
                0x1 => Some(FrameRateFraction {
                    numerator: 24_000,
                    denominator: 1_001,
                }),
                0x2 => Some(FrameRateFraction {
                    numerator: 24,
                    denominator: 1,
                }),
                0x3 => Some(FrameRateFraction {
                    numerator: 25,
                    denominator: 1,
                }),
                0x4 => Some(FrameRateFraction {
                    numerator: 30_000,
                    denominator: 1_001,
                }),
                0x6 => Some(FrameRateFraction {
                    numerator: 50,
                    denominator: 1,
                }),
                0x7 => Some(FrameRateFraction {
                    numerator: 60_000,
                    denominator: 1_001,
                }),
                _ => None,
            };
            (video_format, frame_rate)
        })(input)
    }
    fn dyn_range_col_space(input: &[u8]) -> IResult<&[u8], (DynamicRange, ColorSpace)> {
        map(be_u8, |n| {
            let dyn_range = match (n & 0xF0) >> 4 {
                0x0 => DynamicRange::Sdr,
                0x1 => DynamicRange::Hdr10,
                0x2 => DynamicRange::DolbyVision,
                _ => DynamicRange::Unknown,
            };
            let color_space = match n & 0x0F {
                0x1 => ColorSpace::BT709,
                0x2 => ColorSpace::BT2020,
                _ => ColorSpace::Unknown,
            };
            (dyn_range, color_space)
        })(input)
    }
    fn audio_format(input: &[u8]) -> IResult<&[u8], (AudioFormat, SampleRate)> {
        map(be_u8, |n| {
            let audio_format = match (n & 0xF0) >> 4 {
                0x1 => AudioFormat::Mono,
                0x3 => AudioFormat::Stereo,
                0x6 => AudioFormat::Multichannel,
                0xC => AudioFormat::StereoAndMultichannel,
                _ => AudioFormat::Unknown,
            };
            let sample_rate = match n & 0x0F {
                0x1 => SampleRate::One(48_000),
                0x4 => SampleRate::One(96_000),
                0x5 => SampleRate::One(192_000),
                0xC => SampleRate::Two(48_000, 192_000),
                0xE => SampleRate::Two(48_000, 96_000),
                _ => SampleRate::Unknown,
            };
            (audio_format, sample_rate)
        })(input)
    }
    fn lang_code(input: &[u8]) -> IResult<&[u8], LanguageCode> {
        str_len_owned(3, input)
    }
    fn char_code(input: &[u8]) -> IResult<&[u8], CharacterCode> {
        map(be_u8, |n| match n {
            0x1 => CharacterCode::Utf8,
            0x2 => CharacterCode::Utf16BE,
            0x3 => CharacterCode::ShiftJIS,
            0x4 => CharacterCode::EucKr,
            0x5 => CharacterCode::Gb18030,
            0x6 => CharacterCode::EucCn,
            0x7 => CharacterCode::Big5,
            _ => CharacterCode::Unknown,
        })(input)
    }

    fn parser(input: &[u8]) -> IResult<&[u8], StreamAttributes> {
        let (input, coding_type) = be_u8(input)?;
        let (input, stream_type) = match coding_type {
            0x01 | 0x02 | 0x1B | 0x20 | 0xEA => {
                // SDR video
                map(video_format, |(v, f)| StreamType::SdrVideo(v, f))(input)
            }
            0x24 => {
                // HDR video
                map(
                    tuple((video_format, dyn_range_col_space)),
                    |((v, f), (d, c))| StreamType::HdrVideo(v, f, d, c),
                )(input)
            }
            0x03 | 0x04 | 0x80 | 0x81 | 0x82 | 0x83 | 0x84 | 0x85 | 0x86 | 0xA1 | 0xA2 => {
                // Audio
                map(tuple((audio_format, lang_code)), |((a, s), l)| {
                    StreamType::Audio(a, s, l)
                })(input)
            }
            0x90 | 0x91 => {
                // Graphics (PGS)
                map(lang_code, |l| StreamType::Graphics(l))(input)
            }
            0x92 => {
                // Text
                map(tuple((char_code, lang_code)), |(c, l)| {
                    StreamType::Text(l, c)
                })(input)
            }
            _ => Ok((input, StreamType::Unknown)),
        }?;

        Ok((
            input,
            StreamAttributes {
                coding_type,
                stream_type,
            },
        ))
    }

    length_value(be_u8, parser)(input)
}

fn stream(input: &[u8]) -> IResult<&[u8], Stream> {
    let (input, (entry, attrs)) = tuple((stream_entry, stream_attrs))(input)?;
    Ok((input, Stream { entry, attrs }))
}

fn stream_number_table(input: &[u8]) -> IResult<&[u8], StreamNumberTable> {
    fn parser(input: &[u8]) -> IResult<&[u8], StreamNumberTable> {
        let (input, (_, p_video, p_audio, p_pgs, p_igs, s_audio, s_video, s_pgs, dv, _)) =
            tuple((
                take(2usize),
                be_u8,
                be_u8,
                be_u8,
                be_u8,
                be_u8,
                be_u8,
                be_u8,
                be_u8,
                take(4usize),
            ))(input)?;

        do_parse!(
            input,
            primary_video_streams: count!(stream, p_video as usize)
                >> primary_audio_streams: count!(stream, p_audio as usize)
                >> primary_pgs_streams: count!(stream, p_pgs as usize)
                >> primary_igs_streams: count!(stream, p_igs as usize)
                >> secondary_video_streams: count!(stream, s_video as usize)
                >> secondary_audio_streams: count!(stream, s_audio as usize)
                >> secondary_pgs_streams: count!(stream, s_pgs as usize)
                >> dolby_vision_streams: count!(stream, dv as usize)
                >> (StreamNumberTable {
                    primary_video_streams,
                    primary_audio_streams,
                    primary_pgs_streams,
                    primary_igs_streams,
                    secondary_video_streams,
                    secondary_audio_streams,
                    secondary_pgs_streams,
                    dolby_vision_streams,
                })
        )
    }

    length_value(be_u16, parser)(input)
}

fn sub_play_item(input: &[u8]) -> IResult<&[u8], SubPlayItem> {
    fn multi_clip_entries(input: &[u8]) -> IResult<&[u8], Vec<Clip>> {
        let (input, num_entries) = be_u8(input)?;
        let (input, _) = take(1usize)(input)?;
        count(clip_with_clock_ref, num_entries as usize)(input)
    }
    fn parser(input: &[u8]) -> IResult<&[u8], SubPlayItem> {
        do_parse!(
            input,
            clip: play_item_clip >>
            is_multi_clip: map!(be_u32, |n| (n & 0x1) == 1) >>
            // reftostcid
            take!(1usize) >>
            in_time: time_stamp >>
            out_time: time_stamp >>
            sync_play_item_id: be_u16 >>
            sync_start_pts: be_u32 >>
            multi_clip_entries: map!(cond!(is_multi_clip, multi_clip_entries), |c| c.unwrap_or(Vec::new())) >>
            (SubPlayItem {
                clip,
                in_time,
                out_time,
                sync_play_item_id,
                sync_start_pts,
                multi_clip_entries,
            })
        )
    }

    length_value(be_u16, parser)(input)
}

fn sub_path(input: &[u8]) -> IResult<&[u8], SubPath> {
    fn parser(input: &[u8]) -> IResult<&[u8], SubPath> {
        do_parse!(
            input,
            take!(1usize)
                >> sub_path_type: be_u8
                >> is_repeat: map!(be_u16, |n| (n & 0x1) == 1)
                >> take!(1usize)
                >> num_items: be_u8
                >> play_items: count!(sub_play_item, num_items as usize)
                >> (SubPath {
                    sub_path_type,
                    is_repeat,
                    play_items
                })
        )
    }

    length_value(be_u32, parser)(input)
}

fn play_item_angles(input: &[u8]) -> IResult<&[u8], (AngleInfo, Vec<Clip>)> {
    // main clip counts as an angle, too, so we want to read (n - 1) angle clips
    let (input, additional_angles) = map(be_u8, |n| n.saturating_sub(1))(input)?;
    let (input, angle_info) = map(be_u8, |b| {
        let is_seamless_angle_change = (b & 0x1) == 1;
        let is_different_audios = ((b & 0x2) >> 1) == 1;
        AngleInfo {
            is_seamless_angle_change,
            is_different_audios,
        }
    })(input)?;

    let (input, clips) = count(clip_with_clock_ref, additional_angles as usize)(input)?;
    Ok((input, (angle_info, clips)))
}

fn play_item(input: &[u8]) -> IResult<&[u8], PlayItem> {
    fn parser(input: &[u8]) -> IResult<&[u8], PlayItem> {
        do_parse!(
            input,
            clip: play_item_clip >>
            is_multi_angle: is_multi_angle >>
            // RefToSTCID
            take!(1usize) >>
            in_time: time_stamp >>
            out_time: time_stamp >>
            user_opt_mask: be_u64 >>
            // PlayItemRandomAccessFlag
            take!(1usize) >>
            // StillMode/StillTime
            take!(3usize) >>
            angle_data: map!(cond!(is_multi_angle, play_item_angles),
                |o| o.map(|(a, b)| (Some(a), b)).unwrap_or((None, Vec::new()))) >>
            stream_number_table: stream_number_table >>
            (PlayItem {
                clip,
                in_time,
                out_time,
                user_opt_mask,
                angle_info: angle_data.0,
                angles: angle_data.1,
                stream_number_table,
            })
        )
    }

    length_value(be_u16, parser)(input)
}

fn play_list(input: &[u8]) -> IResult<&[u8], PlayList> {
    fn parser(input: &[u8]) -> IResult<&[u8], PlayList> {
        do_parse!(
            input,
            take!(2usize)
                >> n_play_items: be_u16
                >> n_sub_paths: be_u16
                >> play_items: count!(play_item, n_play_items as usize)
                >> sub_paths: count!(sub_path, n_sub_paths as usize)
                >> (PlayList {
                    play_items,
                    sub_paths,
                })
        )
    }

    length_value(be_u32, parser)(input)
}

fn app_info_play_list(input: &[u8]) -> IResult<&[u8], AppInfoPlayList> {
    fn playback_count(input: &[u8]) -> IResult<&[u8], Option<u16>> {
        let (input, v) = be_u16(input)?;
        let res = match v {
            0x2 | 0x3 => Some(v),
            _ => None,
        };
        Ok((input, res))
    }
    fn playback_type(input: &[u8]) -> IResult<&[u8], PlaybackType> {
        map(be_u8, |n| match n {
            0x1 => PlaybackType::Standard,
            0x2 => PlaybackType::Random,
            0x3 => PlaybackType::Shuffle,
            _ => PlaybackType::Unknown,
        })(input)
    }
    fn parser(input: &[u8]) -> IResult<&[u8], AppInfoPlayList> {
        do_parse!(
            input,
            take!(1usize)
                >> playback_type: playback_type
                >> playback_count: playback_count
                >> user_opt_mask: be_u64
                >> flags: be_u16
                >> (AppInfoPlayList {
                    playback_type,
                    playback_count,
                    user_opt_mask,
                    flags
                })
        )
    }

    length_value(be_u32, parser)(input)
}

fn play_list_mark(input: &[u8]) -> IResult<&[u8], Vec<PlayListMark>> {
    fn mark_type(input: &[u8]) -> IResult<&[u8], MarkType> {
        map(be_u8, |n| match n {
            0x1 => MarkType::EntryPoint,
            0x2 => MarkType::LinkPoint,
            _ => MarkType::Unknown,
        })(input)
    }
    fn mark(input: &[u8]) -> IResult<&[u8], PlayListMark> {
        do_parse!(
            input,
            be_u8 >>
            mark_type: mark_type >>
            play_item: map!(be_u16, |n| PlayItemRef(n)) >>
            ts: time_stamp >>
            // EntryESPID, meaning unknown
            take!(2usize) >>
            duration: map!(time_stamp, |t| if t.0 == 0 { None } else { Some(t) }) >>
            (PlayListMark {
                mark_type,
                play_item,
                time_stamp: ts,
                duration
            })
        )
    }
    fn parser(input: &[u8]) -> IResult<&[u8], Vec<PlayListMark>> {
        let (input, n_marks) = be_u16(input)?;
        count(mark, n_marks as usize)(input)
    }

    length_value(be_u32, parser)(input)
}

struct ExtEntryHeader {
    data_type: u16,
    data_version: u16,
    data_len: u32,
}

fn extension_data(input: &[u8]) -> IResult<&[u8], Vec<ExtensionDataEntry>> {
    fn ext_data_entry(input: &[u8]) -> IResult<&[u8], ExtEntryHeader> {
        do_parse!(
            input,
            data_type: be_u16 >>
            data_version: be_u16 >>
            // data addr, shouldn't need that
            take!(4usize) >>
            data_len: be_u32 >>
            (ExtEntryHeader {
                data_type,
                data_version,
                data_len
            })
        )
    }
    fn parser(input: &[u8]) -> IResult<&[u8], Vec<ExtensionDataEntry>> {
        let (input, _) = be_u32(input)?;
        let (input, num_entries) = map(be_u32, |n| n & 0xF)(input)?;
        let (mut input, entries) = count(ext_data_entry, num_entries as usize)(input)?;
        let mut v: Vec<ExtensionDataEntry> = Vec::with_capacity(num_entries as usize);
        for entry in entries.iter() {
            let (rest, data) = take(entry.data_len as usize)(input)?;
            input = rest;
            v.push(ExtensionDataEntry {
                data_type: entry.data_type,
                data_version: entry.data_version,
                data: Vec::from(data),
            });
        }

        Ok((input, v))
    }

    let (input, len) = be_u32(input)?;
    if len == 0 {
        Ok((input, Vec::new()))
    } else {
        parser(input)
    }
}

pub fn parse_mpls(input: &[u8]) -> IResult<&[u8], Mpls> {
    do_parse!(
        input,
        header_tag
            >> version
            >> count!(addr, 2)
            >> has_ext_data: map!(addr, |a| a != 0)
            >> take!(20usize) // reserved
            >> app_info_play_list: app_info_play_list
            >> play_list: play_list
            >> marks: play_list_mark
            >> ext: map!(cond!(has_ext_data, extension_data), |e| e
                .unwrap_or(Vec::new()))
            >> (Mpls {
                app_info_play_list,
                play_list,
                marks,
                ext
            })
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn header_tag() {
        let data = [0x4d, 0x50, 0x4c, 0x53, 0x30];
        let sl = &data[..];
        assert_eq!(super::header_tag(sl), Ok((&sl[4..], "MPLS")));
    }

    #[test]
    fn header_tag_wrong() {
        let data = [0x4e, 0x51, 0x4c, 0x53, 0x30];
        let sl = &data[..];
        assert_eq!(
            super::header_tag(sl),
            Err(nom::Err::Error((&sl[..], nom::error::ErrorKind::Tag)))
        );
    }

    #[test]
    fn version() {
        let data = [0x30, 0x33, 0x30, 0x30, 0x01, 0x02];
        let sl = &data[..];

        assert_eq!(super::version(sl), Ok((&sl[4..], "0300")));
    }

    #[test]
    fn addr() {
        let data = [0x00, 0x00, 0x66, 0x92, 0x00, 0x01];
        let sl = &data[..];

        assert_eq!(super::addr(sl), Ok((&sl[4..], 26_258_u32)));
    }

    #[test]
    fn str_len() {
        let data = [
            0x30, 0x30, 0x30, 0x38, 0x36, 0x4D, 0x32, 0x54, 0x53, 0x00, 0x01,
        ];
        let sl = &data[..];

        assert_eq!(super::str_len(9, sl), Ok((&sl[9..], "00086M2TS")));
    }

    #[test]
    fn clip_file_name() {
        let data = [0x30, 0x30, 0x30, 0x35, 0x35, 0x4D, 0x32];
        let sl = &data[..];

        assert_eq!(super::clip_file_name(sl), Ok((&sl[5..], "00055")));
    }

    #[test]
    fn clip_codec_id() {
        let data = [0x4D, 0x32, 0x54, 0x53, 0x00, 0x01];
        let sl = &data[..];

        assert_eq!(super::clip_codec_id(sl), Ok((&sl[4..], "M2TS")));
    }
}
