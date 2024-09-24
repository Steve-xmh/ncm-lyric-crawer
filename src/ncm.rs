use amll_lyric::{LyricLine, LyricLineOwned};
use anyhow::*;
use base64::prelude::*;
use serde::*;
use soft_aes::aes::aes_dec_ecb;

const NCM_KEY: &[u8] = b"#14ljk_!\\]&0U<'(";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NCMMusicInfo {
    pub album: String,
    pub album_id: u128,
    pub album_pic: String,
    pub album_pix_doc_id: String,
    pub bitrate: u128,
    pub duration: u128,
    pub format: String,
    pub mp3_doc_id: String,
    pub music_id: u128,
    pub music_name: String,
}

pub fn parse_ncm_key(data: impl AsRef<str>) -> Result<NCMMusicInfo> {
    let data = data.as_ref();
    let key = data
        .strip_prefix("163 key(Don't modify):")
        .context("无法读取到密钥头部")?;

    let encrypted = BASE64_STANDARD.decode(key)?;

    let dec =
        aes_dec_ecb(&encrypted, NCM_KEY, Some("PKCS7")).map_err(|e| anyhow!("无法解密密钥 {e}"))?;

    let data = dec
        .strip_prefix(b"music:")
        .context("无法识别到解密后的 music: 字符串")?;

    let data: NCMMusicInfo = serde_json::from_slice(data)?;

    Ok(data)
}

#[test]
fn test_key() {
    dbg!(parse_ncm_key("163 key(Don't modify):L64FU3W4YxX3ZFTmbZ+8/RRHwdZew2VwDePdDQC3VGRqmDCdbpLMQzF+I5wkI7WH93/xNa4COjW9oLy00/Vp9vd7uiWMV0UBER4xn0CFVGRF1OzvZGOhbEOex7yMwm749fMfSK5qJt56FFxr3KUaVMd8TD1I2WcL51PMFPrH+8raIJLt/ZOLKeUhlvYGxTtNh8zWkQQo3WRe4hl949KJGlGDqBu9VZ7ZPKo2ofJ0cLb7vUStxPqtMW2EGaODC4szWokp0pe+8AWUoMrxRyomuXNeXQTRIqVbbUu/8DNXAG9dB3OV74oJqXkz0tKk35aC2L12na0AeVuxkhHpKAIYo0/eOOrDfcOqh+d2xkdrWEPOgeixeOonupg34xec1p9s5ErdEwwfzJV7Vd3l1V8n/su2DSg2/RgXG1eFXXkyu+Wlvdly0awk1q0s0MVaMtzakKWCjGpnaamEDkQVZYVnJ2m+/FGuy/x+sYwNv8d38R2ssIuEsqccEDnPUv/kFIvb")).unwrap();
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct EAPILyric {
    pub version: usize,
    pub lyric: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct EAPILyricResponse {
    pub code: usize,
    pub lrc: Option<EAPILyric>,
    pub tlyric: Option<EAPILyric>,
    pub romalrc: Option<EAPILyric>,
    pub yrc: Option<EAPILyric>,
    pub ytlrc: Option<EAPILyric>,
    pub yromalrc: Option<EAPILyric>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EAPILyrics {
    pub lrc: Option<Vec<LyricLineOwned>>,
    pub tlyric: Option<Vec<LyricLineOwned>>,
    pub romalrc: Option<Vec<LyricLineOwned>>,
    pub yrc: Option<Vec<LyricLineOwned>>,
    pub ytlrc: Option<Vec<LyricLineOwned>>,
    pub yromalrc: Option<Vec<LyricLineOwned>>,
}

pub async fn get_ncm_lyric(music_id: u128) -> Result<EAPILyrics> {
    let url = format!("https://music.163.com/api/song/lyric/v1?tv=0&lv=0&rv=0&kv=0&yv=0&ytv=0&yrv=0&cp=false&id={}", music_id);
    let resp = reqwest::get(&url).await?.text().await?;
    let data: EAPILyricResponse = serde_json::from_str(&resp)?;

    if data.code != 200 {
        bail!("请求失败: {}", data.code);
    }

    let parse_lrc = |lrc: EAPILyric| -> Vec<LyricLineOwned> {
        let lrc = lrc.lyric.as_str();
        amll_lyric::lrc::parse_lrc(lrc)
            .into_iter()
            .map(|x| x.into())
            .collect()
    };
    let parse_yrc = |lrc: EAPILyric| -> Vec<LyricLineOwned> {
        let lrc = lrc.lyric.as_str();
        amll_lyric::yrc::parse_yrc(lrc)
            .into_iter()
            .map(|x| x.into())
            .collect()
    };

    Ok(EAPILyrics {
        lrc: data.lrc.map(parse_lrc),
        tlyric: data.tlyric.map(parse_lrc),
        romalrc: data.romalrc.map(parse_lrc),
        yrc: data.yrc.map(parse_yrc),
        ytlrc: data.ytlrc.map(parse_lrc),
        yromalrc: data.yromalrc.map(parse_lrc),
    })
}

#[tokio::test]
async fn test_lyric() {
    dbg!(get_ncm_lyric(402073643).await.unwrap());
}
