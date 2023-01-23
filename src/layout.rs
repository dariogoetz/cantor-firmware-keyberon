use keyberon::action::{k, m, Action::*, HoldTapAction, HoldTapConfig};
use keyberon::key_code::KeyCode::*;

type Action = keyberon::action::Action<()>;

static DLAYER: Action = Action::DefaultLayer(0);
static QWERTZLAYER: Action = Action::DefaultLayer(4);

const TIMEOUT: u16 = 200;

const SHIFT_SP: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: k(LShift),
    tap: k(Space),
});

const CTRL_TAB: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: k(LCtrl),
    tap: k(Tab),
});

const ALT_ENT: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: k(LAlt),
    tap: k(Enter),
});

const PPN: Action = HoldTap(&HoldTapAction {
    timeout: TIMEOUT,
    tap_hold_interval: 200,
    config: HoldTapConfig::Default,
    hold: k(MediaNextSong),
    tap: k(MediaPlayPause),
});

macro_rules! s {
    ($k:ident) => {
        m(&[LShift, $k].as_slice())
    };
}
macro_rules! a {
    ($k:ident) => {
        m(&&[RAlt, $k].as_slice())
    };
}

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<12, 4, 5, ()> = keyberon::layout::layout! {
    {
        [ J     Y     Z     U     A     Q     P     B     M     L     F       -  ],
        [(1)    C     S     I     E     O     D     T     N     R     H      (1) ],
        [LGui   V     X LBracket Quote  ;     W     G     ,     .     K      LGui],
        [ t     t t (2) LShift {CTRL_TAB} {ALT_ENT} {SHIFT_SP} (2) t  t      t   ],
    }{
        [ t {a!(E)}     {s!(Slash)} {a!(Kb8)}         {a!(Kb9)}      Grave          {s!(Kb1)}   NonUsBslash {s!(NonUsBslash)} {s!(Kb0)}       {s!(Kb6)}   {a!(Q)} ],
        [ t {a!(Minus)} {s!(Kb7)}   {a!(Kb7)}         {a!(Kb0)}      {s!(RBracket)} {s!(Minus)} {s!(Kb8)}   {s!(Kb9)}         Slash           {s!(Dot)}   t],
        [ t NonUsHash   {s!(Kb4)}   {a!(NonUsBslash)} {a!(RBracket)} {s!(Equal)}    RBracket    {s!(Kb5)}   {s!(Kb2)}         {s!(NonUsHash)} {s!(Comma)} t],
        [ t t           t           (3)               t              t              t           t           (3)               t               t           t],
    }{
        [ t  PgUp   BSpace Up   Delete PgDown n      Kb7 Kb8 Kb9 RBracket Slash],
        [(3) Home   Left   Down Right  End    n      Kb4 Kb5 Kb6 Dot      {s!(RBracket)}],
        [ t  Escape Tab    n    Enter  n      Kb0    Kb1 Kb2 Kb3 Comma    {s!(Kb7)}],
        [ t  t      t      t    t      t      t      t   t   t   t        t],
    }{
        [{Custom(())}  n    n     n     VolUp    n   F12  F7  F8  F9  n  {Custom(())}],
        [t             n    n     n     {PPN}    n   F11  F4  F5  F6  n  t],
        [n             n    n     n     VolDown  n   F10  F1  F2  F3  n  n],
        [t             t    t     t     t        t   t    t   {QWERTZLAYER} t   t   t],
    }{
         [ Tab    Q   W   E   R   T     Y   U   I   O   P   BSpace ]
         [ LCtrl  A   S   D   F   G     H   J   K   L   ;   Quote  ]
         [ LShift Z   X   C   V   B     N   M   ,   .   /   Escape ]
         [ n n n LGui LCtrl LAlt Enter Space {DLAYER} n n   n ]
     }
};
