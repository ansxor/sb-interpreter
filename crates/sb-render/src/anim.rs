//! Keyframe animation engine shared by sprites (`SPANIM`) and BG layers (`BGANIM`).
//!
//! SmileBASIC's `SPANIM`/`BGANIM` drive a target channel of an object across up to 32
//! keyframes. Each keyframe is a `Time` plus 1-2 `Item`s: a POSITIVE time HOLDS the item
//! value for that many frames, a NEGATIVE time LINEARLY INTERPOLATES toward it over
//! `|time|` frames (the smooth form). Adding 8 to the numeric target (or suffixing the
//! string target with `"+"`) makes the items RELATIVE to the object's current runtime value.
//!
//! The advancement is object-agnostic: it operates purely on the keyframe list and a vector
//! of channel values (`cur`). The owning state (sprite slot / BG layer) supplies the
//! per-channel read/write back, then ticks each running animation per displayed frame. This
//! module is the structural engine sprites and BG share; the exact interpolation rounding is
//! oracle-pending (no framebuffer harvest yet — see `HARVEST_QUEUE.md`).

/// Maximum keyframes accepted per animation target (`cmp r0,#0x20` → errnum 39 past 32).
pub const ANIM_MAX_KEYFRAMES: usize = 32;

/// Why a keyframe list was rejected. The lifecycle/argument errnums (4/8/10) are decided by
/// the builtin; these are the data-build errnums raised while validating the keyframe data
/// (`FUN_001ee360`/`0x163a00` for sprites, the equivalent BGANIM builder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimError {
    /// Fewer than one keyframe's worth of data for the channel — errnum 4 ("Illegal
    /// function call").
    TooFew,
    /// More than 32 keyframes — errnum 39 ("Animation is too long").
    TooLong,
    /// A keyframe time/item is outside the ±32768 fixed-point range — errnum 10 ("Out of
    /// range"; the items are `sxth`-truncated to 16 bits).
    OutOfRange,
    /// A keyframe has a zero duration — errnum 40 ("Illegal animation data").
    ZeroTime,
}

/// One keyframe: a `time` (frames) and 1-2 `items`. A positive time HOLDS the item value
/// for that many frames; a negative time LINEARLY INTERPOLATES toward it over `|time|`
/// frames (the smooth form).
#[derive(Debug, Clone, PartialEq)]
pub struct AnimKeyframe {
    /// Per-keyframe duration in frames; sign selects hold (≥0) vs interpolate (<0).
    pub time: i32,
    /// 1 or 2 target values (per the channel's items-per-keyframe).
    pub items: Vec<f64>,
}

/// A running per-channel animation. [`KeyframeAnim::step`] advances it one frame; the owner
/// reads [`KeyframeAnim::cur`] and writes it back into the object's channel.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeAnim {
    /// Target channel 0..7 (`target & 7`).
    pub channel: usize,
    /// Relative flag (`+8` / trailing `"+"`): items are offsets from `base`.
    pub relative: bool,
    /// The object's channel value captured at definition time (the relative base / the start
    /// of the first interpolation segment).
    pub base: Vec<f64>,
    /// The keyframe sequence (1..=32).
    pub keyframes: Vec<AnimKeyframe>,
    /// Loop count: run the sequence this many times, or endlessly when 0.
    pub loop_count: i32,
    /// Current keyframe index.
    pub kf: usize,
    /// Frames already applied within the current keyframe.
    pub frame: i32,
    /// Value at the start of the current segment (the interpolation source).
    pub seg_start: Vec<f64>,
    /// The current applied channel value.
    pub cur: Vec<f64>,
    /// Completed loops.
    pub loops_done: i32,
    /// Whether the animation has finished (a non-endless loop ran out).
    pub done: bool,
}

impl KeyframeAnim {
    /// Build + validate an animation from an already-flattened `data` list
    /// (`Time, Item[, Item], …`). `items_per_kf` is the channel's item count (1 or 2);
    /// `base` is the object's current channel value (the relative/interpolation base). The
    /// handler floors the keyframe count to whole keyframes (fewer than one is
    /// [`AnimError::TooFew`]), caps at 32 ([`AnimError::TooLong`]), range-checks each
    /// time/item to ±32768 ([`AnimError::OutOfRange`]), and rejects a zero duration
    /// ([`AnimError::ZeroTime`]).
    pub fn build(
        channel: usize,
        relative: bool,
        items_per_kf: usize,
        base: Vec<f64>,
        data: &[f64],
        loop_count: i32,
    ) -> Result<Self, AnimError> {
        let stride = 1 + items_per_kf; // Time + items
        let frames = data.len() / stride;
        if frames == 0 {
            return Err(AnimError::TooFew);
        }
        if frames > ANIM_MAX_KEYFRAMES {
            return Err(AnimError::TooLong);
        }
        let used = &data[..frames * stride];
        // Every time/item is a 16-bit fixed-point value (±32768) in the handler.
        if used.iter().any(|&v| !(-32768.0..32768.0).contains(&v)) {
            return Err(AnimError::OutOfRange);
        }
        let mut keyframes = Vec::with_capacity(frames);
        for chunk in used.chunks_exact(stride) {
            let time = chunk[0] as i32;
            // A zero-duration keyframe is illegal animation data (errnum 40).
            if time == 0 {
                return Err(AnimError::ZeroTime);
            }
            keyframes.push(AnimKeyframe {
                time,
                items: chunk[1..].to_vec(),
            });
        }
        Ok(Self {
            channel,
            relative,
            base: base.clone(),
            keyframes,
            loop_count,
            kf: 0,
            frame: 0,
            seg_start: base.clone(),
            cur: base,
            loops_done: 0,
            done: false,
        })
    }

    /// Absolute target value of keyframe `i` (adding `base` when relative).
    fn target(&self, i: usize) -> Vec<f64> {
        let items = &self.keyframes[i].items;
        if self.relative {
            items
                .iter()
                .enumerate()
                .map(|(k, v)| self.base.get(k).copied().unwrap_or(0.0) + v)
                .collect()
        } else {
            items.clone()
        }
    }

    /// Advance one frame, updating `cur` (and the keyframe/loop state).
    pub fn step(&mut self) {
        if self.done || self.keyframes.is_empty() {
            return;
        }
        let i = self.kf;
        let kf_time = self.keyframes[i].time;
        let dur = kf_time.abs();
        let target = self.target(i);
        self.frame += 1;
        if self.frame >= dur {
            // Segment complete: snap to the keyframe target and advance.
            self.cur = target;
            self.advance_keyframe();
        } else if kf_time < 0 {
            // Mid-interpolation: linear from the segment start toward the target.
            let t = self.frame as f64 / dur as f64;
            self.cur = self
                .seg_start
                .iter()
                .zip(target.iter())
                .map(|(s, e)| s + (e - s) * t)
                .collect();
        } else {
            // Hold: the value is the keyframe target for the whole segment.
            self.cur = target;
        }
    }

    /// Move to the next keyframe, wrapping + counting loops at the end of the sequence.
    fn advance_keyframe(&mut self) {
        self.frame = 0;
        self.seg_start = self.cur.clone();
        self.kf += 1;
        if self.kf >= self.keyframes.len() {
            self.kf = 0;
            if self.loop_count != 0 {
                self.loops_done += 1;
                if self.loops_done >= self.loop_count {
                    self.done = true;
                }
            }
        }
    }
}
