use anyhow::*;
use std::path::Path;
use symphonia::core::{
    io::{MediaSourceStream, MediaSourceStreamOptions},
    meta::{MetadataRevision, StandardTagKey, StandardVisualKey},
    probe::ProbeResult,
};

use serde::*;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInfo {
    pub name: String,
    pub artist: String,
    pub album: String,
    pub lyric: String,
    pub cover_media_type: String,
    pub cover: Option<Vec<u8>>,
    pub comment: String,
}

pub fn read_audio_info(path: impl AsRef<Path>) -> Result<AudioInfo> {
    let file = std::fs::File::open(path)?;

    let probe = symphonia::default::get_probe();

    let source_stream = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

    let mut format_result = probe
        .format(
            &Default::default(),
            source_stream,
            &Default::default(),
            &Default::default(),
        )
        .context("无法解析正在加载的音频数据信息")?;

    let audio_info = parse_audio_info(&mut format_result);

    Ok(audio_info)
}

fn parse_audio_info(format_result: &mut ProbeResult) -> AudioInfo {
    let mut new_audio_info = AudioInfo::default();

    let mut read_rev = |rev: &MetadataRevision| {
        if rev.visuals().len() == 1 {
            let visual = &rev.visuals()[0];
            // trace!("仅有一个视觉图");
            // trace!(" 大小 {}", visual.data.len());
            // trace!(" 媒体类型为 {}", visual.media_type);
            // trace!(" 用途为 {:?}", visual.usage);
            // trace!(" 标签为 {:?}", visual.tags);
            new_audio_info.cover_media_type = visual.media_type.clone();
            new_audio_info.cover = Some(visual.data.to_vec());
        } else {
            for visual in rev.visuals() {
                if visual.usage == Some(StandardVisualKey::FrontCover) {
                    new_audio_info.cover_media_type = visual.media_type.clone();
                    new_audio_info.cover = Some(visual.data.to_vec());
                }
            }
        }
        for tag in rev.tags() {
            // trace!("已读取标签 {}", tag);
            match tag.std_key {
                Some(StandardTagKey::TrackTitle) => {
                    new_audio_info.name = tag.value.to_string();
                }
                Some(StandardTagKey::Artist) => {
                    new_audio_info.artist = tag.value.to_string();
                }
                Some(StandardTagKey::Album) => {
                    new_audio_info.album = tag.value.to_string();
                }
                Some(StandardTagKey::Lyrics) => {
                    new_audio_info.lyric = tag.value.to_string();
                }
                Some(StandardTagKey::Comment) => {
                    new_audio_info.comment = tag.value.to_string();
                }
                Some(_) | None => {}
            }
        }
    };

    if let Some(mut metadata) = format_result.metadata.get() {
        while let Some(rev) = metadata.pop() {
            // trace!("已读取音频内容前元数据");
            read_rev(&rev);
        }
        if let Some(rev) = metadata.current() {
            // trace!("已读取音频内容前元数据");
            read_rev(rev);
        }
    }

    let mut metadata = format_result.format.metadata();
    while let Some(rev) = metadata.pop() {
        // trace!("已读取音频内容后元数据");
        read_rev(&rev);
    }
    if let Some(rev) = metadata.current() {
        // trace!("已读取音频内容后元数据");
        read_rev(rev);
    }

    new_audio_info
}
