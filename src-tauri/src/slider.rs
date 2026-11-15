use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context};
use base64::Engine;
use image::DynamicImage;
use serde::Deserialize;

/// `GET /index.php?ctl=slider&act=create` 返回的题目
#[derive(Debug, Clone, Deserialize)]
pub struct SliderChallenge {
    pub token: String,
    pub bg: String,
    pub piece: String,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub w: u32,
    #[serde(default)]
    pub h: u32,
    #[serde(default)]
    pub s: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SliderVerifyResp {
    #[serde(default)]
    pub pass: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// 识别缺口位置。
///
/// 站点返回的 `bg` 是「已经挖掉一块（并压暗）」的背景图，`piece` 是挖下来的拼图块。
/// 因为缺口区域被整体压暗，直接做像素差（SSD）会被其它暗部骗到，实测会偏几十像素；
/// 这里改用「掩码内的归一化互相关（NCC）」，对整体亮度缩放不敏感，缺口处相关性接近 1。
pub fn locate_gap(challenge: &SliderChallenge) -> anyhow::Result<u32> {
    let bg = decode_data_url(&challenge.bg)
        .context("解析滑块背景图失败")?
        .to_luma8();
    let piece = decode_data_url(&challenge.piece)
        .context("解析滑块拼图块失败")?
        .to_rgba8();

    let (piece_w, piece_h) = piece.dimensions();
    let (bg_w, bg_h) = bg.dimensions();
    if piece_w == 0 || piece_h == 0 || bg_w == 0 || bg_h == 0 {
        return Err(anyhow!("滑块图片尺寸异常"));
    }

    let y = challenge.y.max(0.0).round() as u32;
    if y + piece_h > bg_h {
        return Err(anyhow!(
            "滑块 y 坐标超出背景图范围: y={y} piece_h={piece_h} bg_h={bg_h}"
        ));
    }
    let last_x = bg_w.saturating_sub(piece_w);
    if last_x == 0 {
        return Err(anyhow!("滑块背景图太窄"));
    }

    // 拼图块的灰度值（只取不透明像素）与其均值/标准差
    let mut piece_values: Vec<f64> = Vec::with_capacity((piece_w * piece_h) as usize);
    for py in 0..piece_h {
        for px in 0..piece_w {
            let pixel = piece.get_pixel(px, py);
            if pixel[3] >= 128 {
                piece_values.push(f64::from(pixel[0]));
            }
        }
    }
    let count = piece_values.len() as f64;
    if count < 16.0 {
        return Err(anyhow!("拼图块有效像素太少"));
    }
    let piece_mean = piece_values.iter().sum::<f64>() / count;
    let piece_var = piece_values
        .iter()
        .map(|v| (v - piece_mean) * (v - piece_mean))
        .sum::<f64>()
        / count;
    let piece_std = piece_var.sqrt();
    if piece_std < 1.0 {
        return Err(anyhow!("拼图块对比度太低，无法识别"));
    }
    let piece_norm: Vec<f64> = piece_values
        .iter()
        .map(|v| (v - piece_mean) / piece_std)
        .collect();

    let mut best_score = f64::MIN;
    let mut best_x = 0u32;
    let mut region_values: Vec<f64> = Vec::with_capacity(piece_values.len());

    for x in 0..=last_x {
        region_values.clear();
        let mut sum = 0f64;
        for py in 0..piece_h {
            for px in 0..piece_w {
                let pixel = piece.get_pixel(px, py);
                if pixel[3] < 128 {
                    continue;
                }
                let value = f64::from(bg.get_pixel(x + px, y + py)[0]);
                sum += value;
                region_values.push(value);
            }
        }
        let region_mean = sum / count;
        let region_var = region_values
            .iter()
            .map(|v| (v - region_mean) * (v - region_mean))
            .sum::<f64>()
            / count;
        let region_std = region_var.sqrt();
        if region_std < 1.0 {
            continue;
        }
        let mut dot = 0f64;
        for (value, piece_norm_value) in region_values.iter().zip(piece_norm.iter()) {
            dot += (value - region_mean) / region_std * piece_norm_value;
        }
        let score = dot / count;
        if score > best_score {
            best_score = score;
            best_x = x;
        }
    }

    if best_score == f64::MIN {
        return Err(anyhow!("没有找到可用的缺口位置"));
    }
    Ok(best_x)
}

/// 生成带加速度、抖动和轻微回退的人类拖动轨迹：`[[t, x, y], ...]`
///
/// 后端会校验「位置 + 轨迹」：点位不能太少（<20 步容易判失败）、必须单调推进、
/// 时间间隔要接近真人节奏，所以这里固定 24~32 步、每步 10~24ms。
pub fn build_track(target_x: f64, seed: u64) -> Vec<Vec<f64>> {
    let mut rng = Rng::new(seed);
    let start_y = rng.range(380.0, 430.0);
    let steps = rng.range(24.0, 32.0) as u32;
    let mut track: Vec<Vec<f64>> = Vec::with_capacity(steps as usize + 2);
    let mut t = 0.0;
    let mut last_x = 0.0f64;
    track.push(vec![0.0, 0.0, start_y.round()]);

    for i in 1..=steps {
        let progress = f64::from(i) / f64::from(steps);
        // 先快后慢，接近缺口时减速
        let eased = 1.0 - (1.0 - progress).powf(2.3);
        let mut x = target_x * eased;
        if progress > 0.85 {
            // 真人会在末端回退一点点
            x -= rng.range(0.5, 2.0);
        }
        x = x.max(last_x);
        last_x = x;
        t += rng.range(10.0, 24.0);
        let y = start_y + rng.range(-2.0, 2.0);
        track.push(vec![t.round(), round1(x), y.round()]);
    }

    // 松手：最后一点必须精确落在缺口坐标上
    t += rng.range(60.0, 140.0);
    track.push(vec![t.round(), round1(target_x), start_y.round()]);
    track
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn decode_data_url(value: &str) -> anyhow::Result<DynamicImage> {
    let base64_part = match value.find("base64,") {
        Some(pos) => &value[pos + "base64,".len()..],
        None => value,
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_part.trim())
        .context("base64 解码失败")?;
    image::load_from_memory(&bytes).context("图片解码失败")
}

/// 极小的伪随机数发生器，避免为一个随机数引入额外依赖
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Rng {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| u64::from(d.subsec_nanos()))
            .unwrap_or(0);
        let state = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(nanos)
            .wrapping_add(0x9E37_79B9_7F4A_7C15)
            | 1;
        Rng { state }
    }

    fn next_f64(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        let v = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (v >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, low: f64, high: f64) -> f64 {
        low + self.next_f64() * (high - low)
    }
}

pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// 构造一张「已经挖掉一块并压暗」的假题目，缺口在 `hole_x`
    fn fake_challenge(hole_x: u32) -> SliderChallenge {
        let (bg_w, bg_h) = (160u32, 60u32);
        let (piece_size, piece_y) = (20u32, 12u32);
        let mut bg = image::RgbImage::new(bg_w, bg_h);
        for (x, y, pixel) in bg.enumerate_pixels_mut() {
            let v = ((x * 37 + y * 17) % 251) as u8;
            *pixel = image::Rgb([v, v / 2, 255 - v]);
        }

        let mut piece = image::RgbaImage::new(piece_size, piece_size);
        for py in 0..piece_size {
            for px in 0..piece_size {
                let source = *bg.get_pixel(hole_x + px, piece_y + py);
                piece.put_pixel(px, py, image::Rgba([source[0], source[1], source[2], 255]));
            }
        }
        // 站点返回的背景图里，缺口区域是被压暗的
        for py in 0..piece_size {
            for px in 0..piece_size {
                let source = *bg.get_pixel(hole_x + px, piece_y + py);
                bg.put_pixel(
                    hole_x + px,
                    piece_y + py,
                    image::Rgb([source[0] / 3, source[1] / 3, source[2] / 3]),
                );
            }
        }

        SliderChallenge {
            token: "test-token".to_string(),
            bg: encode_png(&image::DynamicImage::ImageRgb8(bg)),
            piece: encode_png(&image::DynamicImage::ImageRgba8(piece)),
            y: f64::from(piece_y),
            w: bg_w,
            h: bg_h,
            s: piece_size,
        }
    }

    fn encode_png(image: &DynamicImage) -> String {
        let mut buf = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode png");
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(buf)
        )
    }

    #[test]
    fn locate_gap_finds_darkened_hole() {
        for hole_x in [0u32, 37, 61, 140] {
            let challenge = fake_challenge(hole_x);
            let found = locate_gap(&challenge).expect("locate gap");
            assert_eq!(found, hole_x, "缺口 x 识别错误");
        }
    }

    #[test]
    fn track_is_monotonic_and_ends_at_target() {
        for seed in [1u64, 7, 42, 99_991] {
            let target = 137.0f64;
            let track = build_track(target, seed);
            assert!(
                (24..=36).contains(&track.len()),
                "步数不合理: {}",
                track.len()
            );
            assert_eq!(track[0][1], 0.0);
            let last = track.last().expect("track not empty");
            assert!(
                (last[1] - target).abs() < 0.05,
                "最后一点必须落在缺口上: {} vs {target}",
                last[1]
            );

            let mut prev_t = f64::MIN;
            let mut prev_x = f64::MIN;
            for point in &track {
                assert!(point[0] > prev_t, "时间戳必须递增");
                assert!(point[1] >= prev_x, "x 不能倒退");
                assert!(point[1] <= target + 0.01, "x 不能超过缺口");
                prev_t = point[0];
                prev_x = point[1];
            }
        }
    }

    #[test]
    fn track_duration_looks_human() {
        for seed in [3u64, 17, 128] {
            let track = build_track(120.0, seed);
            let total = track.last().expect("track not empty")[0];
            assert!(
                (250.0..=1200.0).contains(&total),
                "拖动总时长不像真人: {total}ms"
            );
        }
    }
}
