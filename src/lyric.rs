use amll_lyric::{ttml::TTMLLyricOwned, LyricLineOwned};

use crate::ncm::EAPILyrics;

enum PairType {
    Translated,
    Roman,
}

fn pair_lyrics(line: LyricLineOwned, lines: &mut [LyricLineOwned], pair_type: PairType) {
    if line.is_empty() {
        return;
    }
    let mut nearest_line_index: Option<usize> = None;
    for (i, dst_line) in lines.iter().enumerate() {
        if dst_line.is_empty() {
            continue;
        }
        if dst_line.start_time == line.start_time {
            nearest_line_index = Some(i);
            break;
        }
        if let Some(nearest_line_i) = nearest_line_index {
            let nearest_line = &lines[nearest_line_i];
            if nearest_line.start_time.abs_diff(line.start_time)
                > dst_line.start_time.abs_diff(line.start_time)
            {
                nearest_line_index = Some(i);
            }
        } else {
            nearest_line_index = Some(i);
        }
    }
    if let Some(nearest_line_index) = nearest_line_index {
        let target_line = &mut lines[nearest_line_index];
        let joined = line.to_line();
        match pair_type {
            PairType::Translated => {
                target_line.translated_lyric = joined;
            }
            PairType::Roman => {
                target_line.roman_lyric = joined;
            }
        }
    }
}

pub fn transform_lyric_to_ttml(lyric: EAPILyrics) -> TTMLLyricOwned {
    let (mut orig, tran, roman) = if lyric.yrc.is_some() {
        let yrc = lyric.yrc.unwrap();
        let ytlrc = lyric.ytlrc.unwrap_or_default();
        let yromalrc = lyric.yromalrc.unwrap_or_default();
        (yrc, ytlrc, yromalrc)
    } else {
        let lrc = lyric.lrc.unwrap_or_default();
        let tlyric = lyric.tlyric.unwrap_or_default();
        let romalrc = lyric.romalrc.unwrap_or_default();
        (lrc, tlyric, romalrc)
    };

    for line in tran {
        pair_lyrics(line.clone(), &mut orig, PairType::Translated);
    }
    for line in roman {
        pair_lyrics(line.clone(), &mut orig, PairType::Roman);
    }
    if let Some(last) = orig.last_mut() {
        last.end_time = last.words.last().map_or(u64::MAX, |x| x.end_time);
    }
    TTMLLyricOwned {
        lines: orig,
        ..Default::default()
    }
}
