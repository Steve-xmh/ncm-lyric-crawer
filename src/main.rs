use std::{path::PathBuf, sync::Arc, time::Duration};

use clap::Parser;
use prodash::{render::tui::Options, Root};
use tokio::sync::Semaphore;

mod audio;
mod lyric;
mod ncm;

/// 一个用于检索音乐文件元数据，识别音乐对应的网易云音乐 ID，并下载其歌词的工具
#[derive(Parser)]
struct Args {
    /// 需要检索音乐文件的文件或文件夹路径
    #[arg(required = true)]
    pub path: Vec<PathBuf>,
}

static PERMITS: Semaphore = Semaphore::const_new(64);

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let progress: Arc<_> = prodash::tree::root::Options {
        ..Default::default()
    }
    .create()
    .into();

    let mut sp = progress.add_child("扫描音乐文件");

    let (scan_sx, mut scan_rx) = tokio::sync::mpsc::channel(64);

    let scan_task = async move {
        for p in args.path {
            let wk = walkdir::WalkDir::new(p);
            for p in wk.into_iter().flatten() {
                if p.path().is_file() {
                    const ALLOWED_EXT: &[&str] = &["mp3", "flac", "wav", "ogg"];
                    if let Some(ext) = p.path().extension().and_then(|x| x.to_str()) {
                        if ALLOWED_EXT.contains(&ext) {
                            let music_path = p.path().to_owned();
                            sp.info(music_path.display().to_string());
                            scan_sx.send(music_path).await.unwrap();
                        }
                    }
                }
            }
        }
        sp.done("歌曲文件扫描完成");
    };

    let mut sp = progress.add_child("读取音乐文件");

    let (lyric_sx, mut lyric_rx) = tokio::sync::mpsc::channel(64);

    let parse_task = async move {
        while let Some(music_path) = scan_rx.recv().await {
            let music_path = music_path.clone();
            let mut sp = sp.add_child(music_path.display().to_string());
            let lyric_sx = lyric_sx.clone();
            tokio::spawn(async move {
                let audio_info = {
                    let music_path = music_path.clone();
                    tokio::task::spawn_blocking(move || audio::read_audio_info(music_path)).await
                };
                match audio_info {
                    Ok(Ok(audio_info)) => {
                        sp.done(format!("读取音乐元数据成功: {:?}", audio_info));
                        match ncm::parse_ncm_key(audio_info.comment) {
                            Ok(key) => {
                                lyric_sx.send((music_path, key.music_id)).await.unwrap();
                            }
                            Err(err) => {
                                sp.fail(format!("无法解析音乐元数据中的网易云音乐元数据: {err}"));
                            }
                        }
                    }
                    Ok(Err(err)) => {
                        sp.fail(format!("读取音乐元数据失败: {err}"));
                    }
                    Err(err) => {
                        sp.fail(format!("读取音乐元数据失败: {err}"));
                    }
                }
            });
        }
    };

    let mut sp = progress.add_child("下载并解析歌词");

    let lyric_task = async move {
        while let Some((music_path, music_id)) = lyric_rx.recv().await {
            let _permit = PERMITS.acquire().await.unwrap();
            let mut sp = sp.add_child(format!("{}: {}", music_id, music_path.display()));
            tokio::spawn(async move {
                match ncm::get_ncm_lyric(music_id).await {
                    Ok(lyrics) => {
                        let ttml = lyric::transform_lyric_to_ttml(lyrics);
                        if ttml.lines.is_empty() {
                            sp.fail("歌曲没有歌词");
                            return;
                        }
                        let ttml = ttml.to_ref();
                        match amll_lyric::ttml::stringify_ttml(&ttml) {
                            Ok(ttml) => {
                                let lyric_path = music_path.with_extension("ttml");
                                if let Err(err) = tokio::fs::write(lyric_path, ttml).await {
                                    sp.fail(format!("写入到歌词文件失败: {err}"));
                                } else {
                                    sp.done("歌词下载成功");
                                }
                            }
                            Err(err) => {
                                sp.fail(format!("转换歌词到 TTML 失败: {err}"));
                            }
                        }
                    }
                    Err(err) => {
                        sp.fail(format!("下载歌词失败: {err}"));
                    }
                }
                drop(_permit);
            });
        }
    };

    let render_task = prodash::render::tui(
        std::io::stdout(),
        progress.downgrade(),
        Options {
            title: "NCM 音乐歌词下载器".into(),
            ..Default::default()
        },
    )
    .unwrap();

    tokio::join!(scan_task, parse_task, render_task, lyric_task);
    // sp.
}
