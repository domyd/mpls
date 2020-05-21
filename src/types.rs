use std::{fmt::{Display, Debug}, ops::Deref};
use super::parser::parse_mpls;

/// The parser error of the [`Mpls::parse`] method.
/// 
/// [`Mpls::parse`]: struct.Mpls.html#method.parse
#[derive(Debug)]
pub enum MplsParseError {
    /// Unexpectedly encountered the end of the byte stream.
    UnexpectedEof,
    /// Failed to parse as an MPLS.
    Invalid,
}

/// The movie playlist.
///
/// See the [crate-level docs] for high-level documentation about how to use this type.
/// 
/// [crate-level docs]: ../index.html
#[derive(Debug)]
pub struct Mpls {
    pub app_info_play_list: AppInfoPlayList,
    pub play_list: PlayList,
    pub marks: Vec<PlayListMark>,
    pub ext: Vec<ExtensionDataEntry>,
}

/// Represents a playlist's angle.
///
/// "Angles", as they are called, are just a variation of a playlist where one
/// or more segments are swapped out for different ones. The overall number of
/// segments, however, is always the same for all angles.
///
/// You can use the [`segments`] method to retrieve the playlist segments
/// associated with this angle.
/// 
/// [`segments`]: #method.segments
#[derive(Copy, Clone, Debug)]
pub struct Angle<'mpls> {
    /// The angle index in this playlist.
    pub index: u8,
    mpls: &'mpls Mpls,
}

impl<'a> Display for Angle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.index)
    }
}

impl Mpls {
    /// Parses the given `bytes` and returns an `Mpls` struct.
    ///
    /// # Examples
    /// ```no_run
    /// # fn main() -> std::io::Result<()> {
    /// use std::fs::File;
    /// use std::io::Read;
    /// use mpls::Mpls;
    ///
    /// let bytes = {
    ///     let mut f = File::open("00800.mpls")?;
    ///     let mut buffer = Vec::new();
    ///     f.read_to_end(&mut buffer)?;
    ///     buffer
    /// };
    /// let mpls = Mpls::parse(&bytes).unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse(bytes: &[u8]) -> Result<Mpls, MplsParseError> {
        let (_, mpls) = parse_mpls(bytes).map_err(|e| match e {
            nom::Err::Incomplete(_) => MplsParseError::UnexpectedEof,
            _ => MplsParseError::Invalid,
        })?;
        Ok(mpls)
    }

    /// Gets all of the movie's angles.
    ///
    /// This method will always return at least one element, since it counts the
    /// main feature as an angle regardless of whether the movie contains any
    /// additional angles.
    ///
    /// # Examples
    /// A playlist with four angles in total (one of them being the main feature):
    /// ```
    /// use mpls::Mpls;
    /// 
    /// // let mpls = Mpls::parse(...)?;
    /// # let mpls = {
    /// #     let bytes = include_bytes!("../assets/multi-angle.mpls");
    /// #     let mpls = Mpls::parse(&bytes[..]).unwrap();
    /// #     mpls
    /// # };
    /// let angles = mpls.angles();
    /// assert_eq!(angles.len(), 4);
    /// ```
    ///
    /// A playlist that has no additional angles, only the main feature:
    /// ```
    /// use mpls::Mpls;
    /// 
    /// // let mpls = Mpls::parse(...)?;
    /// # let mpls = {
    /// #     let bytes = include_bytes!("../assets/simple.mpls");
    /// #     let mpls = Mpls::parse(&bytes[..]).unwrap();
    /// #     mpls
    /// # };
    /// # let angles = mpls.angles();
    /// assert_eq!(angles.len(), 1);
    /// ```
    pub fn angles(&self) -> Vec<Angle> {
        self.play_list
            .play_items
            .iter()
            .map(|p| p.angles.len() + 1)
            .max()
            .map(|n| {
                (0..n)
                    .map(|i| Angle {
                        index: i as u8,
                        mpls: &self,
                    })
                    .collect()
            })
            .unwrap_or(Vec::new())
    }
}

impl Angle<'_> {
    /// Gets all segments for this angle.
    /// 
    /// # Examples
    /// Get all clip file names (without their extension) for an angle:
    /// ```
    /// use mpls::Mpls;
    /// 
    /// # let mpls = {
    /// #     let bytes = include_bytes!("../assets/simple.mpls");
    /// #     let mpls = Mpls::parse(&bytes[..]).unwrap();
    /// #     mpls
    /// # };
    /// # let angle = mpls.angles()[0];
    /// let segments: Vec<&str> = angle
    ///     .segments()
    ///     .iter()
    ///     .map(|s| s.file_name.as_ref())
    ///     .collect();
    /// 
    /// assert_eq!(segments, &["00055", "00059", "00061"])
    /// ```
    /// 
    /// Multi-angle:
    /// ```
    /// use mpls::Mpls;
    /// 
    /// // let mpls = Mpls::parse(...)?;
    /// # let mpls = {
    /// #     let bytes = include_bytes!("../assets/multi-angle.mpls");
    /// #     let mpls = Mpls::parse(&bytes[..]).unwrap();
    /// #     mpls
    /// # };
    /// let angles = mpls.angles();
    /// let segments: (Vec<&str>, Vec<&str>) = (
    ///     angles[0].segments().iter().map(|s| s.file_name.as_ref()).collect(),
    ///     angles[1].segments().iter().map(|s| s.file_name.as_ref()).collect());
    /// 
    /// assert_eq!(&segments.0[..5], &["00081", "00082", "00086", "00087", "00091"]);
    /// assert_eq!(&segments.1[..5], &["00081", "00083", "00086", "00088", "00091"]);
    /// ```
    pub fn segments(&self) -> Vec<&Clip> {
        let play_items = &self.mpls.play_list.play_items;
        let mut clips: Vec<&Clip> = Vec::with_capacity(play_items.len());
        for play_item in play_items.iter() {
            let clip = play_item.clip_for_angle(self);
            clips.push(clip);
        }
        clips
    }
}

#[derive(Debug)]
pub struct PlayItem {
    pub clip: Clip,
    pub in_time: TimeStamp,
    pub out_time: TimeStamp,
    pub user_opt_mask: u64,
    pub angles: Vec<Clip>,
    pub angle_info: Option<AngleInfo>,
    pub stream_number_table: StreamNumberTable,
}

impl PlayItem {
    pub fn clip_for_angle(&self, angle: &Angle) -> &Clip {
        match angle.index {
            0 => &self.clip,
            i => {
                let idx = i.saturating_sub(1) as usize;
                match (&self.angles).get(idx) {
                    Some(c) => c,
                    None => &self.clip
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SubPlayItem {
    pub clip: Clip,
    pub in_time: TimeStamp,
    pub out_time: TimeStamp,
    pub sync_play_item_id: u16,
    pub sync_start_pts: u32,
    pub multi_clip_entries: Vec<Clip>,
}

#[derive(Debug)]
pub struct ExtensionDataEntry {
    pub data_type: u16,
    pub data_version: u16,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct SubPath {
    pub sub_path_type: u8,
    pub is_repeat: bool,
    pub play_items: Vec<SubPlayItem>,
}

#[derive(Debug)]
pub struct AngleInfo {
    pub is_different_audios: bool,
    pub is_seamless_angle_change: bool,
}

/// A clip file, also known as a segment.
///
/// This identifies the playable stream file. `file_name` consists of 5 numbers
/// (e.g. "00055"), and `codec_id` of 4 letters which will usually be "M2TS" on
/// blu-rays.
#[derive(Debug)]
pub struct Clip {
    pub file_name: String,
    pub codec_id: String,
}

#[derive(Debug)]
pub struct PlayList {
    pub play_items: Vec<PlayItem>,
    pub sub_paths: Vec<SubPath>,
}

#[derive(Debug)]
pub struct PlayListMark {
    pub mark_type: MarkType,
    pub play_item: PlayItemRef,
    pub time_stamp: TimeStamp,
    pub duration: Option<TimeStamp>,
}

#[derive(Debug)]
pub enum MarkType {
    EntryPoint,
    LinkPoint,
    Unknown,
}

#[derive(Debug)]
pub enum PlaybackType {
    Standard,
    Random,
    Shuffle,
    Unknown,
}

#[derive(Debug)]
pub struct AppInfoPlayList {
    pub playback_type: PlaybackType,
    pub playback_count: Option<u16>,
    pub user_opt_mask: u64,
    pub flags: u16,
}

#[derive(Debug)]
pub struct StreamNumberTable {
    pub primary_video_streams: Vec<Stream>,
    pub primary_audio_streams: Vec<Stream>,
    pub primary_pgs_streams: Vec<Stream>,
    pub primary_igs_streams: Vec<Stream>,
    pub secondary_audio_streams: Vec<Stream>,
    pub secondary_video_streams: Vec<Stream>,
    pub secondary_pgs_streams: Vec<Stream>,
    pub dolby_vision_streams: Vec<Stream>,
}

/// A media stream within a [`Clip`].
/// 
/// [`Clip`]: struct.Clip.html
#[derive(Debug)]
pub struct Stream {
    pub entry: StreamEntry,
    pub attrs: StreamAttributes,
}

#[derive(Debug)]
pub struct StreamEntry {
    pub stream_type: u8,
    pub refs: StreamEntryRef,
}

#[derive(Debug)]
pub enum StreamEntryRef {
    PlayItem(Ref),
    SubPathKind1(Ref, Ref, Ref),
    SubPathKind2(Ref, Ref),
}

#[derive(Debug)]
pub struct SubPathRef(pub u8);

#[derive(Debug)]
pub struct SubClipRef(pub u8);

#[derive(Debug)]
pub struct PlayItemRef(pub u16);

#[derive(Debug)]
pub struct StreamRef(pub u16);

#[derive(Debug)]
pub enum Ref {
    SubPath(SubPathRef),
    SubClip(SubClipRef),
    PlayItem(PlayItemRef),
    Stream(StreamRef),
}

#[derive(Debug)]
pub enum SubPathStream {
    Type2(u8, u8, u16),
    Type3(),
}

#[derive(Debug)]
pub struct StreamAttributes {
    pub coding_type: u8,
    pub stream_type: StreamType,
}

#[derive(Debug)]
pub enum StreamType {
    SdrVideo(VideoFormat, FrameRate),
    HdrVideo(VideoFormat, FrameRate, DynamicRange, ColorSpace),
    Audio(AudioFormat, SampleRate, LanguageCode),
    Graphics(LanguageCode),
    Text(LanguageCode, CharacterCode),
    Unknown,
}

pub type LanguageCode = String;

#[derive(Debug)]
pub enum CharacterCode {
    Utf8,
    Utf16BE,
    ShiftJIS,
    EucKr,
    Gb18030,
    EucCn,
    Big5,
    Unknown,
}

#[derive(Debug)]
pub enum AudioFormat {
    Mono,
    Stereo,
    Multichannel,
    StereoAndMultichannel,
    Unknown,
}

#[derive(Debug)]
pub enum SampleRate {
    One(u32),
    Two(u32, u32),
    Unknown,
}

#[derive(Debug)]
pub enum VideoFormat {
    Interlaced480,
    Interlaced576,
    Interlaced1080,
    Progressive480,
    Progressive576,
    Progressive720,
    Progressive1080,
    Progressive2160,
    Unknown,
}

#[derive(Debug)]
pub enum DynamicRange {
    Sdr,
    Hdr10,
    DolbyVision,
    Unknown,
}

#[derive(Debug)]
pub enum ColorSpace {
    BT709,
    BT2020,
    Unknown,
}

pub type FrameRate = Option<FrameRateFraction>;

/// A video frame rate, represented as a fraction.
#[derive(Debug)]
pub struct FrameRateFraction {
    pub numerator: i32,
    pub denominator: i32,
}

impl FrameRateFraction {
    /// Returns the fraction's value as an `f64`.
    pub fn fps(&self) -> f64 {
        (self.numerator as f64) / (self.denominator as f64)
    }

    /// Returns the fraction's value as an `f32`.
    pub fn fps_single(&self) -> f32 {
        (self.numerator as f32) / (self.denominator as f32)
    }
}

/// A time stamp, relative to some System Time Clock sequence, expressed in 45 KHz.
/// 
/// To get a floating-point value in seconds, you can use the [`seconds`] method.
/// 
/// [`seconds`]: #method.seconds
#[derive(Copy, Clone)]
pub struct TimeStamp(pub u32);

impl TimeStamp {
    /// Returns this time stamp in units of seconds.
    pub fn seconds(&self) -> f64 {
        (self.0 as f64) / 45_000f64
    }
}

impl Deref for TimeStamp {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for TimeStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimeStamp")
            .field("raw", &self.0)
            .field("secs", &self.seconds())
            .finish()
    }
}
