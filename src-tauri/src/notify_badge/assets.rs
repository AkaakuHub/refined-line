#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BadgeToken {
  One,
  Two,
  Three,
  Four,
  Five,
  Six,
  Seven,
  Eight,
  Nine,
  NinePlus,
}

impl BadgeToken {
  const fn as_index(self) -> usize {
    match self {
      BadgeToken::One => 0,
      BadgeToken::Two => 1,
      BadgeToken::Three => 2,
      BadgeToken::Four => 3,
      BadgeToken::Five => 4,
      BadgeToken::Six => 5,
      BadgeToken::Seven => 6,
      BadgeToken::Eight => 7,
      BadgeToken::Nine => 8,
      BadgeToken::NinePlus => 9,
    }
  }
}

pub(crate) const BADGE_SIZES: [u32; 7] = [16, 20, 24, 32, 40, 48, 64];

type BadgeAssetSet = [&'static [u8]; 10];

macro_rules! badge_asset_set {
  ($size:literal) => {
    [
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_1_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_2_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_3_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_4_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_5_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_6_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_7_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_8_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_9_",
        stringify!($size),
        ".png"
      )),
      include_bytes!(concat!(
        "../../icons/notify-badge/",
        stringify!($size),
        "/badge_9plus_",
        stringify!($size),
        ".png"
      )),
    ]
  };
}

static BADGE_ASSETS_16: BadgeAssetSet = badge_asset_set!(16);
static BADGE_ASSETS_20: BadgeAssetSet = badge_asset_set!(20);
static BADGE_ASSETS_24: BadgeAssetSet = badge_asset_set!(24);
static BADGE_ASSETS_32: BadgeAssetSet = badge_asset_set!(32);
static BADGE_ASSETS_40: BadgeAssetSet = badge_asset_set!(40);
static BADGE_ASSETS_48: BadgeAssetSet = badge_asset_set!(48);
static BADGE_ASSETS_64: BadgeAssetSet = badge_asset_set!(64);

pub(crate) fn parse_badge_token(text: Option<&str>) -> Option<BadgeToken> {
  let raw = text?.trim();
  if raw.is_empty() || raw == "0" {
    return None;
  }
  if raw.ends_with('+') {
    return Some(BadgeToken::NinePlus);
  }

  let digits: String = raw.chars().filter(|ch| ch.is_ascii_digit()).collect();
  if digits.is_empty() {
    return None;
  }

  let count = digits.parse::<u32>().ok()?;
  match count {
    0 => None,
    1 => Some(BadgeToken::One),
    2 => Some(BadgeToken::Two),
    3 => Some(BadgeToken::Three),
    4 => Some(BadgeToken::Four),
    5 => Some(BadgeToken::Five),
    6 => Some(BadgeToken::Six),
    7 => Some(BadgeToken::Seven),
    8 => Some(BadgeToken::Eight),
    9 => Some(BadgeToken::Nine),
    _ => Some(BadgeToken::NinePlus),
  }
}

fn badge_assets_for_size(size: u32) -> Option<&'static BadgeAssetSet> {
  match size {
    16 => Some(&BADGE_ASSETS_16),
    20 => Some(&BADGE_ASSETS_20),
    24 => Some(&BADGE_ASSETS_24),
    32 => Some(&BADGE_ASSETS_32),
    40 => Some(&BADGE_ASSETS_40),
    48 => Some(&BADGE_ASSETS_48),
    64 => Some(&BADGE_ASSETS_64),
    _ => None,
  }
}

pub(crate) fn badge_png_bytes(token: BadgeToken, size: u32) -> Option<&'static [u8]> {
  let assets = badge_assets_for_size(size)?;
  Some(assets[token.as_index()])
}
