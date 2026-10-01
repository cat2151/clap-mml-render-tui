/// 音ごとの奏法。METAL-GTX の KS（`sw_last` のラッチ式）1 つに対応する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Articulation {
    /// sfz の `sw_default`。ピッキングした音。
    SusDown,
    /// アップストロークでピッキングした音。
    SusUp,
    HammerOn,
    PullOff,
    /// パームミュートしてダウンストロークでピッキングした音。
    MuteDown,
    /// パームミュートしてアップストロークでピッキングした音。
    MuteUp,
    /// ピッキングハーモニクス（ピンチ）。
    PinchHarmonic,
    /// 低い音から滑り上がって着く音。幅は CC26（[`SLIDE_WIDTH_CC`](crate::SLIDE_WIDTH_CC)）で選ぶ。
    SlideUp,
    /// 高い音から滑り下りて着く音。幅は CC26 で選ぶ。
    SlideDown,
    /// 半音下から持ち上げて着くチョーキング。
    BendHalf,
    /// 1 音下から持ち上げて着くチョーキング。
    BendWhole,
    /// 1 音半下から持ち上げて着くチョーキング。
    BendWholeHalf,
    /// ピックの縁で弦を擦る効果音（ピックスクレイプ）。音高は擦りの速さ（[`PICK_SCRATCH_PITCHES`](crate::PICK_SCRATCH_PITCHES)）。
    PickScratch,
    /// ナチュラルハーモニクス。
    NaturalHarmonics,
    /// ダウンストロークのブラッシング（音程の無いカッティング）。音高で音程は変わらない。
    BrushDown,
    /// アップストロークのブラッシング。
    BrushUp,
    /// 左手で弦を押さえずに消音したダウンストローク（フレットミュート）。
    MuteFretDown,
    /// フレットミュートのアップストローク。
    MuteFretUp,
    /// 伸ばした後に滑り下りて離す音。
    SlideOut,
    /// 前の音から音程を滑らせて着く、ピッキングしない音（擬似レガート）。
    PseudoLegato,
    /// 前の音から滑って着く音（ポルタメント）。
    Portamento,
    /// 下から滑り込んで着く音（スライドイン）。幅は CC27（[`SLIDE_IN_WIDTH_CC`](crate::SLIDE_IN_WIDTH_CC)）で選ぶ。
    SlideIn,
    /// 半音上とのトリル。速さは CC112。
    TrillHalf,
    /// 全音上とのトリル。
    TrillWhole,
    /// 短 3 度上とのトリル。
    TrillMinorThird,
    /// 長 3 度上とのトリル。
    TrillMajorThird,
    /// 2 本の弦で同じ音へ向けて片方を持ち上げるユニゾンチョーキング（自動）。速さは CC28。
    UnisonBendAuto,
    /// ユニゾンチョーキング（手動）。2 層が重なって鳴り、片方を pitch bend で持ち上げる。
    UnisonBendManual,
    /// 半音ずつ駆け下りる効果音のフレーズ（クロマチックラン）。音高ごとに別のフレーズ
    /// （[`CHROMATIC_RUN_PITCHES`](crate::CHROMATIC_RUN_PITCHES)）。
    ChromaticRun,
    /// 弦の上を滑り下りる効果音（スライドエフェクト）。velocity で 3 層を選ぶ。
    SlideFxDown,
    /// 弦の上を滑り上がる効果音。
    SlideFxUp,
    /// 弦の上を上下に滑る効果音。
    SlideFxWow,
}

impl Articulation {
    /// 全 variant。KS 番号から奏法を引く候補。
    pub const ALL: [Articulation; 32] = [
        Articulation::SusDown,
        Articulation::SusUp,
        Articulation::HammerOn,
        Articulation::PullOff,
        Articulation::MuteDown,
        Articulation::MuteUp,
        Articulation::PinchHarmonic,
        Articulation::SlideUp,
        Articulation::SlideDown,
        Articulation::BendHalf,
        Articulation::BendWhole,
        Articulation::BendWholeHalf,
        Articulation::PickScratch,
        Articulation::NaturalHarmonics,
        Articulation::BrushDown,
        Articulation::BrushUp,
        Articulation::MuteFretDown,
        Articulation::MuteFretUp,
        Articulation::SlideOut,
        Articulation::PseudoLegato,
        Articulation::Portamento,
        Articulation::SlideIn,
        Articulation::TrillHalf,
        Articulation::TrillWhole,
        Articulation::TrillMinorThird,
        Articulation::TrillMajorThird,
        Articulation::UnisonBendAuto,
        Articulation::UnisonBendManual,
        Articulation::ChromaticRun,
        Articulation::SlideFxDown,
        Articulation::SlideFxUp,
        Articulation::SlideFxWow,
    ];

    /// この奏法を選ぶ KS の note number。
    pub fn keyswitch(self) -> u8 {
        match self {
            Articulation::SusDown => 17,
            Articulation::SusUp => 18,
            Articulation::HammerOn => 26,
            Articulation::PullOff => 25,
            Articulation::MuteDown => 20,
            Articulation::MuteUp => 21,
            Articulation::PinchHarmonic => 10,
            Articulation::SlideUp => 24,
            Articulation::SlideDown => 23,
            Articulation::BendHalf => 91,
            Articulation::BendWhole => 92,
            Articulation::BendWholeHalf => 93,
            Articulation::PickScratch => 8,
            Articulation::NaturalHarmonics => 9,
            Articulation::BrushDown => 11,
            Articulation::BrushUp => 12,
            Articulation::MuteFretDown => 14,
            Articulation::MuteFretUp => 15,
            Articulation::SlideOut => 28,
            Articulation::PseudoLegato => 29,
            Articulation::Portamento => 96,
            Articulation::SlideIn => 27,
            Articulation::TrillHalf => 99,
            Articulation::TrillWhole => 100,
            Articulation::TrillMinorThird => 101,
            Articulation::TrillMajorThird => 102,
            Articulation::UnisonBendAuto => 94,
            Articulation::UnisonBendManual => 95,
            Articulation::ChromaticRun => 4,
            Articulation::SlideFxDown => 5,
            Articulation::SlideFxUp => 6,
            Articulation::SlideFxWow => 7,
        }
    }

    /// METAL-GTX の KS 一覧（`METAL-GTX_KSMap.txt`）での名前。
    pub fn name(self) -> &'static str {
        match self {
            Articulation::SusDown => "Sus_Down",
            Articulation::SusUp => "Sus_Up",
            Articulation::HammerOn => "Hammer-On",
            Articulation::PullOff => "Pull-Off",
            Articulation::MuteDown => "Mute_Down",
            Articulation::MuteUp => "Mute_Up",
            Articulation::PinchHarmonic => "PH",
            Articulation::SlideUp => "Slide_Up",
            Articulation::SlideDown => "Slide_Down",
            Articulation::BendHalf => "Bending_HT",
            Articulation::BendWhole => "Bending_WH",
            Articulation::BendWholeHalf => "Bending_1HT",
            Articulation::PickScratch => "Pick_Scratch",
            Articulation::NaturalHarmonics => "NH",
            Articulation::BrushDown => "Brush_Down",
            Articulation::BrushUp => "Brush_Up",
            Articulation::MuteFretDown => "Mute_Fret_D",
            Articulation::MuteFretUp => "Mute_Fret_U",
            Articulation::SlideOut => "Slide_Out",
            Articulation::PseudoLegato => "Pseudo_Legato",
            Articulation::Portamento => "Portament",
            Articulation::SlideIn => "Slide_In",
            Articulation::TrillHalf => "Trill_HT",
            Articulation::TrillWhole => "Trill_WT",
            Articulation::TrillMinorThird => "Trill_min3",
            Articulation::TrillMajorThird => "Trill_Maj3",
            Articulation::UnisonBendAuto => "Unison_Bend_Auto",
            Articulation::UnisonBendManual => "Unison_Bend_Manual",
            Articulation::ChromaticRun => "Chromatic_Run",
            Articulation::SlideFxDown => "Slide_FX_D",
            Articulation::SlideFxUp => "Slide_FX_U",
            Articulation::SlideFxWow => "Slide_FX_Wow",
        }
    }

    /// KS の note number から奏法を引く。KS でない音高なら `None`。
    pub fn from_keyswitch(key: u8) -> Option<Self> {
        Articulation::ALL
            .into_iter()
            .find(|articulation| articulation.keyswitch() == key)
    }
}

#[cfg(test)]
mod tests;
